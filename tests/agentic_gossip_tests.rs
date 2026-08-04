// Sprint 317: Mesh Peer Gossip Protocol, Heartbeats & Auto-Healing Eviction Tests

use std::sync::Arc;

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{MeshGossipConfig, MeshGossipWorker, MeshPeer, RpcServer};

#[test]
fn test_mesh_ping_rpc_endpoint() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-ping-alpha",
        "127.0.0.1:9091",
        None,
    );

    let ping_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "sender_node_id": "node-ping-beta",
            "sender_address": "127.0.0.1:9092",
            "latency_ms": 12
        },
        "id": 1
    });

    let resp_str = server.dispatch_request(&ping_req.to_string());
    let resp_val: serde_json::Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp_val["jsonrpc"], "2.0");
    assert_eq!(resp_val["result"]["status"], "ok");
    assert_eq!(resp_val["result"]["pong"], true);
    assert_eq!(resp_val["result"]["responder_node_id"], "node-ping-alpha");
    assert_eq!(resp_val["result"]["responder_address"], "127.0.0.1:9091");

    // Verify sender node beta was auto-registered in topology
    let peers = server.peers.lock().unwrap();
    let beta_peer = peers
        .get("node-ping-beta")
        .expect("Beta peer auto-registered");
    assert_eq!(beta_peer.address, "127.0.0.1:9092");
    assert_eq!(beta_peer.status, "Active");
    assert_eq!(beta_peer.latency_ms, 12);
}

#[test]
fn test_gossip_timeout_evaluation_and_auto_healing_eviction() {
    let server = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-master",
        "127.0.0.1:9100",
        None,
    ));

    let now_base = 1000000u64;

    // Register 3 peers at timestamp now_base
    let peer_active = MeshPeer {
        node_id: "peer-active".to_string(),
        address: "127.0.0.1:9101".to_string(),
        capabilities: vec!["mesh_ping".to_string()],
        last_seen: now_base,
        latency_ms: 5,
        status: "Active".to_string(),
    };
    let peer_stale = MeshPeer {
        node_id: "peer-stale".to_string(),
        address: "127.0.0.1:9102".to_string(),
        capabilities: vec!["mesh_ping".to_string()],
        last_seen: now_base,
        latency_ms: 10,
        status: "Active".to_string(),
    };
    let peer_dead = MeshPeer {
        node_id: "peer-dead".to_string(),
        address: "127.0.0.1:9103".to_string(),
        capabilities: vec!["mesh_ping".to_string()],
        last_seen: now_base,
        latency_ms: 0,
        status: "Active".to_string(),
    };

    {
        let mut peers = server.peers.lock().unwrap();
        peers.insert(peer_active.node_id.clone(), peer_active);
        peers.insert(peer_stale.node_id.clone(), peer_stale);
        peers.insert(peer_dead.node_id.clone(), peer_dead);
    }

    let config = MeshGossipConfig {
        gossip_interval_secs: 2,
        stale_timeout_secs: 5,
        eviction_timeout_secs: 15,
        ping_timeout_ms: 500,
    };

    let worker = MeshGossipWorker::new(server.clone(), config);

    // Update peer_active's last_seen to now_base + 12s
    {
        let mut peers = server.peers.lock().unwrap();
        if let Some(p) = peers.get_mut("peer-active") {
            p.last_seen = now_base + 12;
        }
        if let Some(p) = peers.get_mut("peer-stale") {
            p.last_seen = now_base + 8; // 6s ago at now_base + 14 -> Stale (>5s, <15s)
        }
        // peer_dead remains at now_base -> 14s ago at now_base + 16 -> Evicted (>=15s)
    }

    let (active_cnt, stale_cnt, evicted_cnt) = worker.evaluate_timeouts(now_base + 16);
    assert_eq!(active_cnt, 1); // peer-active
    assert_eq!(stale_cnt, 1); // peer-stale
    assert_eq!(evicted_cnt, 1); // peer-dead

    let peers_after = server.peers.lock().unwrap();
    assert_eq!(peers_after.get("peer-active").unwrap().status, "Active");
    assert_eq!(peers_after.get("peer-stale").unwrap().status, "Stale");
    assert_eq!(peers_after.get("peer-dead").unwrap().status, "Evicted");
    drop(peers_after);

    // Prune evicted peers
    let pruned = worker.prune_evicted();
    assert_eq!(pruned, 1);

    let peers_final = server.peers.lock().unwrap();
    assert_eq!(peers_final.len(), 2);
    assert!(peers_final.contains_key("peer-active"));
    assert!(peers_final.contains_key("peer-stale"));
    assert!(!peers_final.contains_key("peer-dead"));
}

#[test]
fn test_mesh_peers_prune_rpc_action() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-prune-master",
        "127.0.0.1:9200",
        None,
    );

    // Register active peer and evicted peer
    let reg_req1 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_peers",
        "params": {
            "action": "register",
            "peer": {
                "node_id": "good-node",
                "address": "127.0.0.1:9201",
                "capabilities": ["mesh_ping"],
                "last_seen": 100,
                "latency_ms": 4,
                "status": "Active"
            }
        },
        "id": 1
    });

    let reg_req2 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_peers",
        "params": {
            "action": "register",
            "peer": {
                "node_id": "failed-node",
                "address": "127.0.0.1:9202",
                "capabilities": ["mesh_ping"],
                "last_seen": 10,
                "latency_ms": 0,
                "status": "Evicted"
            }
        },
        "id": 2
    });

    server.dispatch_request(&reg_req1.to_string());
    server.dispatch_request(&reg_req2.to_string());

    // Execute prune action via RPC
    let prune_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_peers",
        "params": {
            "action": "prune"
        },
        "id": 3
    });

    let prune_resp_str = server.dispatch_request(&prune_req.to_string());
    let prune_resp: serde_json::Value = serde_json::from_str(&prune_resp_str).unwrap();

    assert_eq!(prune_resp["result"]["status"], "ok");
    assert_eq!(prune_resp["result"]["pruned_count"], 1);

    let remaining_peers = prune_resp["result"]["peers"].as_array().unwrap();
    assert_eq!(remaining_peers.len(), 1);
    assert_eq!(remaining_peers[0]["node_id"], "good-node");
}
