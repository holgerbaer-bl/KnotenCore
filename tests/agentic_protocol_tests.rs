// Sprint 313: Agentic Execution Protocol & State Snapshots Integration Tests
//
// Tests verify:
//   1. knc_agent_handshake returns v2.11.0-agent metadata, capabilities and default quotas
//   2. knc_agent_snapshot captures suspended (Yielded) VM state & session instructions
//   3. knc_agent_restore restores snapshot into a fresh session boundary
//   4. knc_yield_resume seamlessly resumes execution on the restored session to final completion

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::RpcServer;
use knoten_core_types::ast::Node;
use serde_json::{Value, json};

fn test_perms() -> AgentPermissions {
    AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    }
}

#[test]
fn test_agentic_handshake() {
    let server = RpcServer::new(test_perms());

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {},
        "id": 1
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["id"], 1);
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["protocol_version"], "v2.21.1-security");
    assert_eq!(resp["result"]["engine"], "KnotenCore");
    assert_eq!(resp["result"]["capabilities"]["state_snapshots"], true);
    assert_eq!(resp["result"]["capabilities"]["async_yield"], true);
    assert!(
        resp["result"]["default_quota"]["max_instructions"]
            .as_u64()
            .unwrap()
            > 0
    );
}

#[test]
fn test_agentic_snapshot_and_restore_resume_flow() {
    let server_a = RpcServer::new(test_perms());

    // AST: Yield, then add 10 + 20
    let ast = Node::Block(vec![
        Node::Yield,
        Node::Add(
            Box::new(Node::IntLiteral(10)),
            Box::new(Node::IntLiteral(20)),
        ),
    ]);

    // 1. Execute on Server A (session_orig) -> should Yield
    let exec_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_execute",
        "params": {
            "session_id": "session_orig",
            "ast": ast
        },
        "id": 100
    });

    let resp_a: Value =
        serde_json::from_str(&server_a.dispatch_request(&exec_req.to_string())).unwrap();
    assert_eq!(resp_a["result"]["status"], "ok");
    assert_eq!(resp_a["result"]["is_yielded"], true);

    // 2. Capture Snapshot of session_orig
    let snap_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_snapshot",
        "params": {
            "session_id": "session_orig"
        },
        "id": 101
    });

    let snap_resp: Value =
        serde_json::from_str(&server_a.dispatch_request(&snap_req.to_string())).unwrap();
    assert_eq!(snap_resp["result"]["status"], "ok");
    let snapshot_payload = snap_resp["result"]["snapshot"].clone();
    assert!(!snapshot_payload.is_null());

    // 3. Create independent Server B and restore snapshot as session_restored
    let server_b = RpcServer::new(test_perms());

    let restore_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_restore",
        "params": {
            "session_id": "session_restored",
            "snapshot": snapshot_payload
        },
        "id": 200
    });

    let restore_resp: Value =
        serde_json::from_str(&server_b.dispatch_request(&restore_req.to_string())).unwrap();
    assert_eq!(restore_resp["result"]["status"], "ok");
    assert_eq!(restore_resp["result"]["is_yielded"], true);

    // 4. Resume execution on Server B (session_restored) -> should finish with 30
    let resume_req = json!({
        "jsonrpc": "2.0",
        "method": "knc_yield_resume",
        "params": {
            "session_id": "session_restored"
        },
        "id": 201
    });

    let resume_resp: Value =
        serde_json::from_str(&server_b.dispatch_request(&resume_req.to_string())).unwrap();
    assert_eq!(resume_resp["result"]["status"], "ok");
    assert_eq!(resume_resp["result"]["result"]["Int"], 30);
}
