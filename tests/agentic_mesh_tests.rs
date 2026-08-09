use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{JsonRpcResponse, MeshPeer, RpcServer};
use knoten_core_types::ast::Node;

fn spawn_test_rpc_server(node_id: &str, token: Option<&str>) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("Failed to bind ephemeral test port");
    let addr = listener.local_addr().expect("Failed to get local addr");
    let addr_str = addr.to_string();

    let server = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        node_id,
        &addr_str,
        token.map(String::from),
    ));

    let handle = thread::spawn(move || {
        listener.set_nonblocking(false).ok();
        for _ in 0..20 {
            if let Ok((mut stream, _)) = listener.accept() {
                let server_clone = server.clone();
                thread::spawn(move || {
                    let mut reader = BufReader::new(stream.try_clone().unwrap());
                    let mut line = String::new();
                    if reader.read_line(&mut line).is_ok() && !line.trim().is_empty() {
                        let response_str = server_clone.dispatch_request(&line);
                        let mut resp_line = response_str;
                        resp_line.push('\n');
                        stream.write_all(resp_line.as_bytes()).ok();
                        stream.flush().ok();
                    }
                });
            } else {
                break;
            }
        }
    });

    (addr_str, handle)
}

#[test]
fn test_mesh_discover_and_peers() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-alpha",
        "127.0.0.1:9090",
        None,
    );

    // 1. Discover
    let req_discover = r#"{"jsonrpc":"2.0","method":"knc_mesh_discover","params":{},"id":1}"#;
    let resp_str = server.dispatch_request(req_discover);
    let resp: JsonRpcResponse = serde_json::from_str(&resp_str).expect("Valid JSON-RPC response");
    assert!(resp.error.is_none());
    let res = resp.result.expect("Result present");
    assert_eq!(res.get("status").unwrap(), "ok");
    assert_eq!(res.get("node_id").unwrap(), "node-alpha");
    assert_eq!(res.get("address").unwrap(), "127.0.0.1:9090");
    assert_eq!(res.get("protocol_version").unwrap(), "v2.18.0");

    // 2. Register Peer
    let req_reg = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_peers",
        "params": {
            "action": "register",
            "peer": {
                "node_id": "node-beta",
                "address": "127.0.0.1:9091",
                "capabilities": ["agent_teleport"]
            }
        },
        "id": 2
    })
    .to_string();
    let resp_reg_str = server.dispatch_request(&req_reg);
    let resp_reg: JsonRpcResponse =
        serde_json::from_str(&resp_reg_str).expect("Valid JSON-RPC response");
    assert!(resp_reg.error.is_none());

    // 3. List Peers
    let req_list =
        r#"{"jsonrpc":"2.0","method":"knc_mesh_peers","params":{"action":"list"},"id":3}"#;
    let resp_list_str = server.dispatch_request(req_list);
    let resp_list: JsonRpcResponse =
        serde_json::from_str(&resp_list_str).expect("Valid JSON-RPC response");
    let res_list = resp_list.result.expect("Result present");
    let peers: Vec<MeshPeer> =
        serde_json::from_value(res_list.get("peers").unwrap().clone()).unwrap();
    assert_eq!(peers.len(), 1);
    assert_eq!(peers[0].node_id, "node-beta");
}

#[test]
fn test_mesh_auth_token_enforcement() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-secure",
        "127.0.0.1:9092",
        Some("secret-token-123".to_string()),
    );

    // Unauthorized request
    let req_unauth = r#"{"jsonrpc":"2.0","method":"knc_mesh_discover","params":{},"id":1}"#;
    let resp_unauth_str = server.dispatch_request(req_unauth);
    let resp_unauth: JsonRpcResponse = serde_json::from_str(&resp_unauth_str).unwrap();
    assert!(resp_unauth.error.is_some());
    assert_eq!(resp_unauth.error.unwrap().code, -32001);

    // Authorized request
    let req_auth = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_discover",
        "params": {
            "mesh_auth_token": "secret-token-123"
        },
        "id": 2
    })
    .to_string();
    let resp_auth_str = server.dispatch_request(&req_auth);
    let resp_auth: JsonRpcResponse = serde_json::from_str(&resp_auth_str).unwrap();
    assert!(resp_auth.error.is_none());
}

#[test]
fn test_inter_node_state_teleportation() {
    let (addr_b, _handle_b) = spawn_test_rpc_server("node-beta", Some("mesh-auth-xyz"));

    let server_a = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-alpha",
        "127.0.0.1:0",
        Some("mesh-auth-xyz".to_string()),
    );

    // Step 1: Execute AST on Node A to build an execution state
    let ast_add = Node::Add(
        Box::new(Node::IntLiteral(40)),
        Box::new(Node::IntLiteral(2)),
    );
    let req_exec = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "session-origin",
            "ast": ast_add
        },
        "id": 1
    })
    .to_string();

    let resp_exec_str = server_a.dispatch_request(&req_exec);
    let resp_exec: JsonRpcResponse = serde_json::from_str(&resp_exec_str).unwrap();
    assert!(resp_exec.error.is_none());

    // Step 2: Capture Snapshot on Node A
    let req_snap = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_snapshot",
        "params": {
            "session_id": "session-origin"
        },
        "id": 2
    })
    .to_string();

    let resp_snap_str = server_a.dispatch_request(&req_snap);
    let resp_snap: JsonRpcResponse = serde_json::from_str(&resp_snap_str).unwrap();
    let snap_data = resp_snap.result.unwrap().get("snapshot").unwrap().clone();

    // Step 3: Teleport session from Node A to Node B via network dispatch
    let req_teleport = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_teleport",
        "params": {
            "target_node_address": addr_b,
            "target_session_id": "session-teleported-target",
            "mesh_auth_token": "mesh-auth-xyz",
            "snapshot": snap_data
        },
        "id": 3
    })
    .to_string();

    let resp_teleport_str = server_a.dispatch_request(&req_teleport);
    let resp_teleport: JsonRpcResponse = serde_json::from_str(&resp_teleport_str).unwrap();
    assert!(
        resp_teleport.error.is_none(),
        "Teleport error: {:?}",
        resp_teleport.error
    );
    let res_teleport = resp_teleport.result.unwrap();
    assert_eq!(res_teleport.get("status").unwrap(), "ok");
    assert_eq!(res_teleport.get("teleported").unwrap(), true);
    assert_eq!(
        res_teleport.get("session_id").unwrap(),
        "session-teleported-target"
    );
}
