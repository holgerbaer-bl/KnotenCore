// =============================================================================
// KnotenCore — Integration Test Suite: Sprint 335 (v2.22.0)
// Swarm Phase 2: Distributed Raft Voting & Multi-Node Cluster Consensus
// =============================================================================

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use serde_json::json;
use std::sync::Arc;

#[test]
fn test_version_assertion_sprint335() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.5",
        "Protocol version must be synchronized to v2.24.5"
    );
}

#[test]
fn test_request_vote_unauthenticated_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-1",
        "127.0.0.1:0",
        Some("valid-cluster-token".to_string()),
    );
    server.set_revoked_keys_path(None);

    // Unauthenticated request (no mesh_auth_token)
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_swarm_request_vote",
        "params": {
            "term": 2,
            "candidate_id": "candidate-node-1"
        }
    });

    let resp = server.dispatch_request(&req.to_string());
    let resp_val: serde_json::Value = serde_json::from_str(&resp).unwrap();

    assert!(
        resp_val.get("error").is_some(),
        "Unauthenticated vote request must be rejected"
    );
    let err_code = resp_val["error"]["code"].as_i64().unwrap();
    assert_eq!(err_code, -32001);
}

#[test]
fn test_request_vote_lower_term_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-1",
        "127.0.0.1:0",
        Some("token123".to_string()),
    );
    server.set_revoked_keys_path(None);

    // Set current term to 5
    {
        let mut term = server
            .swarm_governance
            .current_term
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *term = 5;
    }

    // Candidate requests vote with term 3 (< 5)
    let req = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_swarm_request_vote",
        "params": {
            "term": 3,
            "candidate_id": "stale-candidate",
            "mesh_auth_token": "token123"
        }
    });

    let resp = server.dispatch_request(&req.to_string());
    let resp_val: serde_json::Value = serde_json::from_str(&resp).unwrap();

    assert_eq!(resp_val["result"]["status"], "ok");
    assert_eq!(resp_val["result"]["vote_granted"], false);
    assert_eq!(resp_val["result"]["term"], 5);
}

#[test]
fn test_request_vote_grants_single_vote_per_term() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-1",
        "127.0.0.1:0",
        Some("token123".to_string()),
    );
    server.set_revoked_keys_path(None);

    // Vote 1 for Candidate A in term 2
    let req1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_swarm_request_vote",
        "params": {
            "term": 2,
            "candidate_id": "candidate-A",
            "mesh_auth_token": "token123"
        }
    });

    let resp1 = server.dispatch_request(&req1.to_string());
    let resp1_val: serde_json::Value = serde_json::from_str(&resp1).unwrap();
    assert_eq!(resp1_val["result"]["vote_granted"], true);

    // Vote 2 for Candidate B in SAME term 2
    let req2 = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_swarm_request_vote",
        "params": {
            "term": 2,
            "candidate_id": "candidate-B",
            "mesh_auth_token": "token123"
        }
    });

    let resp2 = server.dispatch_request(&req2.to_string());
    let resp2_val: serde_json::Value = serde_json::from_str(&resp2).unwrap();
    assert_eq!(
        resp2_val["result"]["vote_granted"], false,
        "Node must not grant more than one vote per term"
    );
}

#[test]
fn test_cluster_quorum_election_success() {
    let token = "cluster-secret-token".to_string();

    // 1. Instantiate 3 real RpcServer instances with distinct node IDs
    let server1 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "cluster-node-1",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server1.set_revoked_keys_path(None);

    let server2 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "cluster-node-2",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server2.set_revoked_keys_path(None);

    let server3 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "cluster-node-3",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server3.set_revoked_keys_path(None);

    // 2. Start real TCP listeners on 3 distinct OS-assigned ports
    let (port1, _h1) = RpcServer::spawn_background_tcp_server(server1.clone(), 0).unwrap();
    let (port2, _h2) = RpcServer::spawn_background_tcp_server(server2.clone(), 0).unwrap();
    let (port3, _h3) = RpcServer::spawn_background_tcp_server(server3.clone(), 0).unwrap();

    let addr1 = format!("127.0.0.1:{}", port1);
    let addr2 = format!("127.0.0.1:{}", port2);
    let addr3 = format!("127.0.0.1:{}", port3);

    // 3. Interconnect nodes in peer tables
    let reg_peer = |srv: &Arc<RpcServer>, id: &str, addr: &str| {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "knc_mesh_peers",
            "params": {
                "action": "register",
                "peer": {
                    "node_id": id,
                    "address": addr,
                    "status": "Active",
                    "capabilities": ["worker"]
                },
                "mesh_auth_token": token
            }
        });
        let resp = srv.dispatch_request(&req.to_string());
        assert!(resp.contains("\"status\":\"ok\""));
    };

    reg_peer(&server1, "cluster-node-2", &addr2);
    reg_peer(&server1, "cluster-node-3", &addr3);

    reg_peer(&server2, "cluster-node-1", &addr1);
    reg_peer(&server2, "cluster-node-3", &addr3);

    reg_peer(&server3, "cluster-node-1", &addr1);
    reg_peer(&server3, "cluster-node-2", &addr2);

    // 4. Trigger election on node 1 over TCP network mesh
    let elect_req = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "knc_swarm_elect",
        "params": {
            "mesh_auth_token": token
        }
    });

    let elect_resp = server1.dispatch_request(&elect_req.to_string());
    let resp_val: serde_json::Value = serde_json::from_str(&elect_resp).unwrap();

    assert_eq!(resp_val["result"]["status"], "ok");
    assert_eq!(resp_val["result"]["leader_node_id"], "cluster-node-1");
    assert_eq!(resp_val["result"]["role"], "Leader");
    assert_eq!(resp_val["result"]["votes_granted"], 3);
    assert_eq!(resp_val["result"]["active_nodes"], 3);
}

#[test]
fn test_split_vote_triggers_backoff_and_prevents_leader() {
    let token = "split-secret-token".to_string();

    let server1 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "split-node-1",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server1.set_revoked_keys_path(None);

    let server2 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "split-node-2",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server2.set_revoked_keys_path(None);

    let server3 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "split-node-3",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server3.set_revoked_keys_path(None);

    let (port1, _h1) = RpcServer::spawn_background_tcp_server(server1.clone(), 0).unwrap();
    let (port2, _h2) = RpcServer::spawn_background_tcp_server(server2.clone(), 0).unwrap();
    let (port3, _h3) = RpcServer::spawn_background_tcp_server(server3.clone(), 0).unwrap();

    let _addr1 = format!("127.0.0.1:{}", port1);
    let addr2 = format!("127.0.0.1:{}", port2);
    let addr3 = format!("127.0.0.1:{}", port3);

    // Register peers on server 1
    let reg_peer = |srv: &Arc<RpcServer>, id: &str, addr: &str| {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "knc_mesh_peers",
            "params": {
                "action": "register",
                "peer": {
                    "node_id": id,
                    "address": addr,
                    "status": "Active",
                    "capabilities": ["worker"]
                },
                "mesh_auth_token": token
            }
        });
        srv.dispatch_request(&req.to_string());
    };

    reg_peer(&server1, "split-node-2", &addr2);
    reg_peer(&server1, "split-node-3", &addr3);

    // Pre-vote server 2 and server 3 for term 2 for competitor node "competitor-node"
    let vote_comp = |srv: &Arc<RpcServer>| {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "knc_swarm_request_vote",
            "params": {
                "term": 2,
                "candidate_id": "competitor-node",
                "mesh_auth_token": token
            }
        });
        srv.dispatch_request(&req.to_string());
    };

    vote_comp(&server2);
    vote_comp(&server3);

    // Now trigger election on server 1 (which increments term to 2)
    let start_time = std::time::Instant::now();
    let elect_req = json!({
        "jsonrpc": "2.0",
        "id": 200,
        "method": "knc_swarm_elect",
        "params": {
            "mesh_auth_token": token
        }
    });

    let elect_resp = server1.dispatch_request(&elect_req.to_string());
    let elapsed = start_time.elapsed();
    let resp_val: serde_json::Value = serde_json::from_str(&elect_resp).unwrap();

    // Node 1 receives votes: self (1), peer 2 (denied - voted competitor), peer 3 (denied - voted competitor) => 1/3 votes < 2 quorum
    assert!(
        resp_val.get("error").is_some(),
        "Election without majority must fail"
    );
    let err_code = resp_val["error"]["code"].as_i64().unwrap();
    assert_eq!(err_code, -32001);

    assert!(
        elapsed >= std::time::Duration::from_millis(140),
        "Randomized backoff sleep must be applied on election failure"
    );
}
