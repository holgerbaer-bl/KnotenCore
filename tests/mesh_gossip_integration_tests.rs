use aether_compiler::crypto_ed25519::Ed25519KeyPair;
use aether_compiler::executor::AgentPermissions;
use aether_compiler::mesh::{
    AntiReplayTracker, GossipState, PeerMetrics, create_signed_gossip_frame, verify_gossip_frame,
};
use aether_compiler::rpc::RpcServer;
use aether_compiler::rpc::handlers::tasks::{
    MAX_PER_PEER_TASK_RATE, PeerRateLimiter, create_signed_task_result,
};
use aether_compiler::rpc::types::KNC_PROTOCOL_VERSION;
use knoten_core_types::ast::Node;
use serde_json::json;

#[test]
fn test_version_assertion_sprint345() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.13");
}

#[test]
fn test_version_assertion_sprint346() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.13");
    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_metrics",
        "params": {}
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"protocol_version\":\"v2.24.13\""));
}

#[test]
fn test_mesh_gossip_peer_discovery_and_selection() {
    let state = GossipState::new();

    let peer1 = PeerMetrics {
        node_id: "node-fast".to_string(),
        address: "127.0.0.1:9001".to_string(),
        public_key: "pubkey1".to_string(),
        cpu_load_percent: 10.0,
        memory_used_bytes: 100,
        memory_total_bytes: 1000,
        memory_usage_percent: 10.0,
        task_queue_depth: 1,
        latency_ms: 10,
        is_overloaded: false,
        status: "Active".to_string(),
        last_seen: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        sequence_number: 1,
    };

    let peer2 = PeerMetrics {
        node_id: "node-slow".to_string(),
        address: "127.0.0.1:9002".to_string(),
        public_key: "pubkey2".to_string(),
        cpu_load_percent: 80.0,
        memory_used_bytes: 800,
        memory_total_bytes: 1000,
        memory_usage_percent: 80.0,
        task_queue_depth: 20,
        latency_ms: 250,
        is_overloaded: false,
        status: "Active".to_string(),
        last_seen: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        sequence_number: 1,
    };

    state.update_peer_metrics(peer1);
    state.update_peer_metrics(peer2);

    let optimal = state.select_optimal_peer().expect("Expected optimal peer");
    assert_eq!(optimal.node_id, "node-fast");

    // Test decay logic
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let (active, stale, evicted) = state.decay_unresponsive_peers(now + 100, 60, 180);
    assert_eq!(active, 0);
    assert_eq!(stale, 2);
    assert_eq!(evicted, 0);
}

#[test]
fn test_signed_task_delegation_and_result_verification() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "delegator",
        "127.0.0.1:9000",
        Some("secret-token".to_string()),
    );

    let worker_key = Ed25519KeyPair::generate();

    // 1. Submit task
    let task_id = server
        .task_dispatcher
        .submit(Node::IntLiteral(100), 128)
        .expect("Submission failed");

    // 2. Worker executes task and constructs signed result
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let signed_res = create_signed_task_result(
        &worker_key,
        task_id.clone(),
        "worker-1".to_string(),
        json!(100),
        now,
    );

    // 3. Delegator verifies signature & completes task
    let complete_res = server
        .task_dispatcher
        .complete_signed(signed_res.clone(), |pk| server.is_peer_key_revoked(pk));
    assert!(complete_res.is_ok());

    let entry = server.task_dispatcher.status(&task_id).unwrap();
    assert_eq!(
        entry.status,
        aether_compiler::rpc::handlers::tasks::TaskStatus::Completed
    );

    // 4. Test tampered result signature rejection
    let mut tampered_res = signed_res;
    tampered_res.result = json!(999);
    let tampered_eval = server
        .task_dispatcher
        .complete_signed(tampered_res, |pk| server.is_peer_key_revoked(pk));
    assert!(tampered_eval.is_err());
}

#[test]
fn test_task_queue_depth_and_rate_limiting() {
    let limiter = PeerRateLimiter::new();
    let peer = "peer-spammer";

    for _ in 0..MAX_PER_PEER_TASK_RATE {
        assert!(
            limiter
                .check_and_record(peer, 60, MAX_PER_PEER_TASK_RATE)
                .is_ok()
        );
    }

    let flood_att = limiter.check_and_record(peer, 60, MAX_PER_PEER_TASK_RATE);
    assert!(flood_att.is_err());
    assert!(flood_att.unwrap_err().contains("rate limit exceeded"));
}

#[test]
fn test_gossip_message_integrity_and_anti_replay() {
    let sender_key = Ed25519KeyPair::generate();
    let tracker = AntiReplayTracker::new();

    let frame = create_signed_gossip_frame(
        &sender_key,
        "sender-node",
        1,
        1000,
        json!({ "cpu_load_percent": 15.0 }),
    );

    // Valid frame check
    assert!(verify_gossip_frame(&frame, Some(&tracker)).is_ok());

    // Replayed frame check (same sequence number)
    let replay_res = verify_gossip_frame(&frame, Some(&tracker));
    assert!(replay_res.is_err());
    assert!(replay_res.unwrap_err().contains("Replayed"));
}
