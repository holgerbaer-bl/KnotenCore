// Sprint 334: Audit Completion & State Persistence Tests (v2.21.4-security)
//
// Verifies:
// 1. C4: Peer Revocation Persistence (survives restart, graceful fallback) & Registration Gate (action=register blocked for revoked keys).
// 2. C3: Quorum Denominator Hardening in knc_swarm_quorum & knc_mesh_revoke_peer (excludes Evicted peers).
// 3. A4: Memory Estimator Stack Traversal Fix (traverses entire stack > 64 items).
// 4. A5: Isolate Custom Quota Support (VMIsolate applies custom quota).

use std::fs;
use std::path::PathBuf;

use aether_compiler::executor::{AgentPermissions, RelType};
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, MeshPeer, RpcServer};
use aether_compiler::vm::isolate::VMIsolate;
use knoten_core_types::ast::IsolateQuota;
use knoten_core_types::opcode::OpCode;

fn temp_file_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "kc_test_{}_{}.json",
        name,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    path
}

#[test]
fn test_revocation_survives_restart() {
    let test_path = temp_file_path("rev_restart");

    // Server A: Revoke a key and persist
    {
        let server_a = RpcServer::new(AgentPermissions::default());
        server_a.set_revoked_keys_path(Some(test_path.clone()));
        server_a.revoke_peer_key("revoked_pubkey_123");
        assert!(server_a.is_peer_key_revoked("revoked_pubkey_123"));
    }

    // Verify file exists on disk
    assert!(test_path.exists());

    // Server B: Restart and load from same file
    {
        let server_b = RpcServer::new(AgentPermissions::default());
        server_b.set_revoked_keys_path(Some(test_path.clone()));
        server_b.load_revoked_keys();
        assert!(
            server_b.is_peer_key_revoked("revoked_pubkey_123"),
            "Revoked key did not survive restart"
        );
    }

    // Graceful Fallback Test: Write corrupted JSON
    fs::write(&test_path, "{ invalid json }").unwrap();

    // Server C: Restart with corrupted file (must not panic)
    {
        let server_c = RpcServer::new(AgentPermissions::default());
        server_c.set_revoked_keys_path(Some(test_path.clone()));
        server_c.load_revoked_keys();
        // Server starts safely without crashing
    }

    let _ = fs::remove_file(test_path);
}

#[test]
fn test_register_revoked_peer_rejected() {
    let server = RpcServer::new(AgentPermissions::default());
    server.set_revoked_keys_path(None);

    server.revoke_peer_key("rogue_peer_key");

    let peer = MeshPeer {
        node_id: "rogue_peer_node".to_string(),
        address: "127.0.0.1:9099".to_string(),
        capabilities: vec!["rogue_peer_key".to_string()],
        last_seen: 100,
        latency_ms: 5,
        status: "Active".to_string(),
    };

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_peers",
        "params": {
            "action": "register",
            "peer": peer
        }
    });

    let resp = server.dispatch_request(&req.to_string());
    assert!(
        resp.contains("-32001"),
        "Revoked peer registration was not rejected: {}",
        resp
    );
    assert!(resp.contains("Peer key is revoked"));
}

#[test]
fn test_quorum_and_revoke_exclude_evicted_peers() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-main",
        "127.0.0.1:0",
        None,
    );
    server.set_revoked_keys_path(None);

    // Add 1 Active peer and 4 Evicted peers
    {
        let mut peers = server.peers.lock().unwrap();
        peers.insert(
            "peer-active".to_string(),
            MeshPeer {
                node_id: "peer-active".to_string(),
                address: "127.0.0.1:9001".to_string(),
                capabilities: vec![],
                last_seen: 100,
                latency_ms: 1,
                status: "Active".to_string(),
            },
        );
        for i in 1..=4 {
            let id = format!("peer-evicted-{}", i);
            peers.insert(
                id.clone(),
                MeshPeer {
                    node_id: id,
                    address: "127.0.0.1:9000".to_string(),
                    capabilities: vec![],
                    last_seen: 100,
                    latency_ms: 1,
                    status: "Evicted".to_string(),
                },
            );
        }
    }

    // Active nodes = 1 (local) + 1 (peer-active) = 2.
    // Correct server_threshold = (2 / 2) + 1 = 2.
    // If Evicted peers were incorrectly counted, total_nodes would be 6, and threshold 4 (failing quorum).

    let req_quorum = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_swarm_quorum",
        "params": {}
    });

    let resp_q = server.dispatch_request(&req_quorum.to_string());
    let val_q: serde_json::Value = serde_json::from_str(&resp_q).unwrap();
    assert_eq!(val_q["result"]["active_nodes"], 2);
    assert_eq!(val_q["result"]["quorum_threshold"], 2);
    assert_eq!(val_q["result"]["quorum_reached"], true);

    // knc_mesh_revoke_peer must also succeed because active quorum is reached (2 >= 2)
    let req_revoke = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "knc_mesh_revoke_peer",
        "params": { "peer_pubkey": "some_pubkey" }
    });

    let resp_r = server.dispatch_request(&req_revoke.to_string());
    assert!(
        resp_r.contains("\"result\""),
        "knc_mesh_revoke_peer failed due to Evicted peers in denominator: {}",
        resp_r
    );
}

#[test]
fn test_stack_deep_large_array_memory_cap() {
    let mut vm = aether_compiler::vm::machine::VM::new();
    let custom_quota = IsolateQuota {
        max_memory_bytes: 100_000, // 100 KB limit
        ..Default::default()
    };
    let max_mem = custom_quota.max_memory_bytes;
    vm.set_quota(custom_quota);

    // Push 70 dummy integers to push stack depth > 64
    for _ in 0..70 {
        vm.stack.push(RelType::Int(42));
    }

    // Push a large array (50,000 ints ~ 400 KB heap) at stack position 71 (> 64)
    let large_array = RelType::Array(vec![RelType::Int(1); 50_000]);
    vm.stack.push(large_array);

    // Memory estimation must catch this item > depth 64 and exceed 100 KB limit
    assert!(
        vm.estimate_memory_bytes() > max_mem,
        "Stack traversal > 64 failed to detect large heap item"
    );
}

#[test]
fn test_isolate_custom_quota_applied() {
    let custom_quota = IsolateQuota {
        max_instructions: 10, // Extremely low instruction limit
        ..Default::default()
    };

    // Create an isolate with 20 Constant instructions
    let instructions = vec![OpCode::Constant(0); 20];
    let constants = vec![RelType::Int(42)];

    let isolate = VMIsolate::new(instructions, constants).with_quota(custom_quota);
    let res = isolate.run();

    assert!(res.is_err());
    let err = res.err().unwrap();
    assert!(
        err.contains("ERR_QUOTA_EXCEEDED") || err.to_lowercase().contains("quota"),
        "Custom quota was not passed to VM: {}",
        err
    );
}

#[test]
fn test_version_assertion_sprint334() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.3");
}
