use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer, start_raft_governance_worker};
use serde_json::json;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread::sleep;
use std::time::Duration;

#[test]
fn test_version_assertion_sprint336() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.5");
}

#[test]
fn test_worker_rejects_stale_leader_heartbeat() {
    let server = RpcServer::new(AgentPermissions::default());
    server.set_revoked_keys_path(None);

    // Initial heartbeat to set term 5
    let req1 = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_swarm_heartbeat",
        "params": {
            "term": 5,
            "leader_id": "leader-node"
        }
    });
    let resp1_str = server.dispatch_request(&req1.to_string());
    let resp1: serde_json::Value = serde_json::from_str(&resp1_str).unwrap();
    assert_eq!(resp1["result"]["status"], "ok");
    assert_eq!(resp1["result"]["success"], true);

    // Stale heartbeat with lower term (term 3 < term 5)
    let req_stale = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_swarm_heartbeat",
        "params": {
            "term": 3,
            "leader_id": "stale-leader"
        }
    });
    let resp_stale_str = server.dispatch_request(&req_stale.to_string());
    let resp_stale: serde_json::Value = serde_json::from_str(&resp_stale_str).unwrap();
    assert_eq!(resp_stale["result"]["status"], "ok");
    assert_eq!(resp_stale["result"]["success"], false);
    assert_eq!(resp_stale["result"]["term"], 5);
}

#[test]
fn test_leader_sends_periodic_heartbeats() {
    let token = "heartbeat-secret".to_string();

    let server1 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "hb-node-1",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server1.set_revoked_keys_path(None);

    let server2 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "hb-node-2",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server2.set_revoked_keys_path(None);

    let server3 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "hb-node-3",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server3.set_revoked_keys_path(None);

    // Spawn TCP servers
    let (port1, _h1) = RpcServer::spawn_background_tcp_server(server1.clone(), 0).unwrap();
    let (port2, _h2) = RpcServer::spawn_background_tcp_server(server2.clone(), 0).unwrap();
    let (port3, _h3) = RpcServer::spawn_background_tcp_server(server3.clone(), 0).unwrap();

    let addr1 = format!("127.0.0.1:{}", port1);
    let addr2 = format!("127.0.0.1:{}", port2);
    let addr3 = format!("127.0.0.1:{}", port3);

    // Register peers
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

    reg_peer(&server1, "hb-node-2", &addr2);
    reg_peer(&server1, "hb-node-3", &addr3);

    reg_peer(&server2, "hb-node-1", &addr1);
    reg_peer(&server2, "hb-node-3", &addr3);

    reg_peer(&server3, "hb-node-1", &addr1);
    reg_peer(&server3, "hb-node-2", &addr2);

    // Start background governance workers
    let shutdown1 = Arc::new(AtomicBool::new(false));
    let shutdown2 = Arc::new(AtomicBool::new(false));
    let shutdown3 = Arc::new(AtomicBool::new(false));

    let _w1 = start_raft_governance_worker(server1.clone(), shutdown1.clone());
    let _w2 = start_raft_governance_worker(server2.clone(), shutdown2.clone());
    let _w3 = start_raft_governance_worker(server3.clone(), shutdown3.clone());

    // Elect server1
    let elect_req = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "knc_swarm_elect",
        "params": { "mesh_auth_token": token }
    });
    let elect_resp = server1.dispatch_request(&elect_req.to_string());
    assert!(elect_resp.contains("\"status\":\"ok\""));
    assert_eq!(
        server1.swarm_governance.role(),
        aether_compiler::rpc::NodeRole::Leader
    );

    // Wait for periodic heartbeats
    sleep(Duration::from_millis(350));

    // Workers should have received heartbeats from hb-node-1
    assert_eq!(
        server2.swarm_governance.leader_id(),
        Some("hb-node-1".to_string())
    );
    assert_eq!(
        server3.swarm_governance.leader_id(),
        Some("hb-node-1".to_string())
    );
    assert!(server2.swarm_governance.last_heartbeat_elapsed_ms() < 350);

    shutdown1.store(true, std::sync::atomic::Ordering::Relaxed);
    shutdown2.store(true, std::sync::atomic::Ordering::Relaxed);
    shutdown3.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[test]
fn test_leader_failover_triggers_reelection() {
    let token = "failover-secret".to_string();

    let server1 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "fo-node-1",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server1.set_revoked_keys_path(None);

    let server2 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "fo-node-2",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server2.set_revoked_keys_path(None);

    let server3 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "fo-node-3",
        "127.0.0.1:0",
        Some(token.clone()),
    ));
    server3.set_revoked_keys_path(None);

    let (port1, _h1) = RpcServer::spawn_background_tcp_server(server1.clone(), 0).unwrap();
    let (port2, _h2) = RpcServer::spawn_background_tcp_server(server2.clone(), 0).unwrap();
    let (port3, _h3) = RpcServer::spawn_background_tcp_server(server3.clone(), 0).unwrap();

    let addr1 = format!("127.0.0.1:{}", port1);
    let addr2 = format!("127.0.0.1:{}", port2);
    let addr3 = format!("127.0.0.1:{}", port3);

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

    reg_peer(&server1, "fo-node-2", &addr2);
    reg_peer(&server1, "fo-node-3", &addr3);

    reg_peer(&server2, "fo-node-1", &addr1);
    reg_peer(&server2, "fo-node-3", &addr3);

    reg_peer(&server3, "fo-node-1", &addr1);
    reg_peer(&server3, "fo-node-2", &addr2);

    let shutdown1 = Arc::new(AtomicBool::new(false));
    let shutdown2 = Arc::new(AtomicBool::new(false));
    let shutdown3 = Arc::new(AtomicBool::new(false));

    let _w1 = start_raft_governance_worker(server1.clone(), shutdown1.clone());
    let _w2 = start_raft_governance_worker(server2.clone(), shutdown2.clone());
    let _w3 = start_raft_governance_worker(server3.clone(), shutdown3.clone());

    // Elect server 1
    let elect_req = json!({
        "jsonrpc": "2.0",
        "id": 100,
        "method": "knc_swarm_elect",
        "params": { "mesh_auth_token": token }
    });
    let _ = server1.dispatch_request(&elect_req.to_string());
    assert_eq!(
        server1.swarm_governance.role(),
        aether_compiler::rpc::NodeRole::Leader
    );

    sleep(Duration::from_millis(250));
    assert_eq!(
        server2.swarm_governance.leader_id(),
        Some("fo-node-1".to_string())
    );

    // Stop leader (server 1) to simulate failure
    shutdown1.store(true, std::sync::atomic::Ordering::Relaxed);
    server1
        .swarm_governance
        .set_role(aether_compiler::rpc::NodeRole::Worker);

    // Wait for failover timeout and re-election among server 2 and server 3
    sleep(Duration::from_millis(750));

    let new_leader = server2.swarm_governance.leader_id();
    assert!(
        new_leader.is_some(),
        "Failover should have elected a new leader"
    );
    let leader_name = new_leader.unwrap();
    assert!(
        leader_name == "fo-node-2" || leader_name == "fo-node-3",
        "New leader should be fo-node-2 or fo-node-3, got: {}",
        leader_name
    );

    shutdown2.store(true, std::sync::atomic::Ordering::Relaxed);
    shutdown3.store(true, std::sync::atomic::Ordering::Relaxed);
}
