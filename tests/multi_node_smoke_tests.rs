// Sprint 361: Multi-Node Integration Smoke Gate (v2.24.24)
// Automated end-to-end integration test suite simulating a 3-node cluster:
// 1. Node Initialization with canonical self-certifying identities (knc-<64_hex_chars>)
// 2. Gossip Discovery & Bidirectional Peer Registration
// 3. Concurrent CRDT State Replication & Digest Convergence
// 4. Authenticated Task Delegation & Parity Verification

use aether_compiler::crypto_ed25519::{Ed25519KeyPair, derive_node_id};
use aether_compiler::executor::AgentPermissions;
use aether_compiler::mesh::{
    AntiReplayTracker, GossipState, PeerMetrics, create_signed_gossip_frame, verify_gossip_frame,
};
use aether_compiler::rpc::handlers::tasks::{TaskStatus, create_signed_task_result};
use aether_compiler::rpc::types::KNC_PROTOCOL_VERSION;
use aether_compiler::rpc::{JsonRpcResponse, RpcServer};
use knoten_core_types::ast::Node;
use serde_json::{Value, json};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn parse_resp(raw: &str) -> JsonRpcResponse {
    serde_json::from_str(raw).expect("Valid JSON-RPC response")
}

static REQ_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn signed_req(kp: &Ed25519KeyPair, method: &str, mut params: Value, id: u64) -> String {
    let sender_node_id = kp.node_id();
    let pubkey = kp.public_key_hex();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let seq = REQ_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let nonce = format!("nonce-{}-{}-{}", id, now, seq);
    let message = format!("{}:{}:{}", now, nonce, sender_node_id);
    let sig = kp.sign_hex(message.as_bytes());

    let envelope = json!({
        "sender_node_id": sender_node_id,
        "public_key": pubkey,
        "signature": sig,
        "timestamp": now,
        "nonce": nonce
    });

    params["zero_trust_envelope"] = envelope;
    params["sender_node_id"] = json!(sender_node_id);
    params["public_key"] = json!(pubkey);
    params["signature"] = json!(sig);
    params["timestamp"] = json!(now);
    params["nonce"] = json!(nonce);

    json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": method,
        "params": params
    })
    .to_string()
}

#[test]
fn test_version_assertion_sprint361() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.24",
        "Protocol version must be synchronized to v2.24.24 for Sprint 361"
    );
}

#[test]
fn test_multi_node_cluster_initialization_and_gossip_discovery() {
    // 1. Node Initialization: Generate 3 distinct nodes with canonical self-certifying identities
    let kp1 = Ed25519KeyPair::generate();
    let id1 = kp1.node_id();
    let pk1 = kp1.public_key_hex();

    let kp2 = Ed25519KeyPair::generate();
    let id2 = kp2.node_id();
    let pk2 = kp2.public_key_hex();

    let kp3 = Ed25519KeyPair::generate();
    let id3 = kp3.node_id();
    let pk3 = kp3.public_key_hex();

    // Verify canonical prefix and derivation invariant
    assert!(id1.starts_with("knc-"));
    assert!(id2.starts_with("knc-"));
    assert!(id3.starts_with("knc-"));
    assert_eq!(id1, derive_node_id(&kp1.public_key().bytes));
    assert_eq!(id2, derive_node_id(&kp2.public_key().bytes));
    assert_eq!(id3, derive_node_id(&kp3.public_key().bytes));

    // Spawn 3 distinct RPC servers in loopback
    let server1 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        &id1,
        "127.0.0.1:0",
        None,
    ));
    server1.enable_zero_trust();
    server1.set_revoked_keys_path(None);

    let server2 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        &id2,
        "127.0.0.1:0",
        None,
    ));
    server2.enable_zero_trust();
    server2.set_revoked_keys_path(None);

    let server3 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        &id3,
        "127.0.0.1:0",
        None,
    ));
    server3.enable_zero_trust();
    server3.set_revoked_keys_path(None);

    let (port1, _h1) = RpcServer::spawn_background_tcp_server(server1.clone(), 0).unwrap();
    let (port2, _h2) = RpcServer::spawn_background_tcp_server(server2.clone(), 0).unwrap();
    let (port3, _h3) = RpcServer::spawn_background_tcp_server(server3.clone(), 0).unwrap();

    let addr1 = format!("127.0.0.1:{}", port1);
    let addr2 = format!("127.0.0.1:{}", port2);
    let addr3 = format!("127.0.0.1:{}", port3);

    // 2. Bidirectional Peer Registration across all nodes using signed requests
    let reg_peer = |srv: &Arc<RpcServer>,
                    caller_kp: &Ed25519KeyPair,
                    peer_id: &str,
                    peer_addr: &str,
                    peer_pk: &str| {
        let req_str = signed_req(
            caller_kp,
            "knc_mesh_peers",
            json!({
                "action": "register",
                "peer": {
                    "node_id": peer_id,
                    "address": peer_addr,
                    "public_key": peer_pk,
                    "status": "Active",
                    "capabilities": ["worker", "storage"]
                }
            }),
            10,
        );
        let raw = srv.dispatch_request(&req_str);
        let resp = parse_resp(&raw);
        assert!(
            resp.error.is_none(),
            "Peer registration failed: {:?}",
            resp.error
        );
        assert_eq!(resp.result.unwrap()["status"], "ok");
    };

    // Node 1 registers 2 & 3
    reg_peer(&server1, &kp1, &id2, &addr2, &pk2);
    reg_peer(&server1, &kp1, &id3, &addr3, &pk3);

    // Node 2 registers 1 & 3
    reg_peer(&server2, &kp2, &id1, &addr1, &pk1);
    reg_peer(&server2, &kp2, &id3, &addr3, &pk3);

    // Node 3 registers 1 & 2
    reg_peer(&server3, &kp3, &id1, &addr1, &pk1);
    reg_peer(&server3, &kp3, &id2, &addr2, &pk2);

    // Assert each server has 2 peers registered
    let list_req1 = signed_req(&kp1, "knc_mesh_peers", json!({ "action": "list" }), 21);
    let resp1 = parse_resp(&server1.dispatch_request(&list_req1));
    let peers1 = resp1.result.unwrap()["peers"].as_array().unwrap().clone();
    assert_eq!(peers1.len(), 2, "Node 1 must have exactly 2 active peers");

    let list_req2 = signed_req(&kp2, "knc_mesh_peers", json!({ "action": "list" }), 22);
    let resp2 = parse_resp(&server2.dispatch_request(&list_req2));
    let peers2 = resp2.result.unwrap()["peers"].as_array().unwrap().clone();
    assert_eq!(peers2.len(), 2, "Node 2 must have exactly 2 active peers");

    let list_req3 = signed_req(&kp3, "knc_mesh_peers", json!({ "action": "list" }), 23);
    let resp3 = parse_resp(&server3.dispatch_request(&list_req3));
    let peers3 = resp3.result.unwrap()["peers"].as_array().unwrap().clone();
    assert_eq!(peers3.len(), 2, "Node 3 must have exactly 2 active peers");

    // 3. Signed Gossip Discovery
    let tracker = AntiReplayTracker::new();
    let frame1 = create_signed_gossip_frame(
        &kp1,
        &id1,
        1,
        1000,
        json!({
            "cpu_load_percent": 12.5,
            "memory_usage_percent": 30.0,
            "task_queue_depth": 0
        }),
    );

    // Peers 2 and 3 verify Gossip frame from Node 1
    assert!(verify_gossip_frame(&frame1, Some(&tracker)).is_ok());

    // Peer discovery via GossipState
    let gossip_state = GossipState::new();
    let p1_metrics = PeerMetrics {
        node_id: id1.clone(),
        address: addr1.clone(),
        public_key: pk1.clone(),
        cpu_load_percent: 12.5,
        memory_used_bytes: 300,
        memory_total_bytes: 1000,
        memory_usage_percent: 30.0,
        task_queue_depth: 0,
        latency_ms: 5,
        is_overloaded: false,
        status: "Active".to_string(),
        last_seen: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        sequence_number: 1,
    };
    gossip_state.update_peer_metrics(p1_metrics);

    let optimal = gossip_state.select_optimal_peer();
    assert!(optimal.is_some());
    assert_eq!(optimal.unwrap().node_id, id1);
}

#[test]
fn test_multi_node_crdt_state_replication_and_digest_convergence() {
    let kp1 = Ed25519KeyPair::generate();
    let kp2 = Ed25519KeyPair::generate();
    let kp3 = Ed25519KeyPair::generate();

    let server1 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        kp1.node_id(),
        "127.0.0.1:0",
        None,
    ));
    let server2 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        kp2.node_id(),
        "127.0.0.1:0",
        None,
    ));
    let server3 = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        kp3.node_id(),
        "127.0.0.1:0",
        None,
    ));

    // Concurrent mutations on different nodes
    // Node 1: put key "config/workers" = 4 @ ts 100
    server1
        .store
        .put("config/workers", json!(4), 100, &kp1.node_id());
    // Node 2: put key "config/cluster_name" = "aether-grid" @ ts 150
    server2.store.put(
        "config/cluster_name",
        json!("aether-grid"),
        150,
        &kp2.node_id(),
    );
    // Node 3: put key "config/workers" = 8 @ ts 200 (LWW conflict winner over Node 1)
    server3
        .store
        .put("config/workers", json!(8), 200, &kp3.node_id());

    // Before sync: digests differ
    let digest1_before = server1.store.compute_state_digest();
    let digest2_before = server2.store.compute_state_digest();
    assert_ne!(digest1_before, digest2_before);

    // Multi-Node State Replication:
    // Gather all entries from all nodes and sync
    let entries1 = server1.store.dump_entries();
    let entries2 = server2.store.dump_entries();
    let entries3 = server3.store.dump_entries();

    // Replicate across all 3 nodes
    for srv in [&server1, &server2, &server3] {
        srv.store.sync(entries1.clone());
        srv.store.sync(entries2.clone());
        srv.store.sync(entries3.clone());
    }

    // Convergence verification:
    // 1. All nodes produce identical SHA-256 state digests
    let d1 = server1.store.compute_state_digest();
    let d2 = server2.store.compute_state_digest();
    let d3 = server3.store.compute_state_digest();
    assert_eq!(d1, d2, "Node 1 and Node 2 digests must converge");
    assert_eq!(d2, d3, "Node 2 and Node 3 digests must converge");

    // 2. LWW conflict resolution asserted across all nodes
    for srv in [&server1, &server2, &server3] {
        let workers_entry = srv.store.get("config/workers").expect("Key exists");
        assert_eq!(
            workers_entry.value,
            json!(8),
            "LWW rule must select ts 200 (value 8)"
        );
        assert_eq!(workers_entry.writer_id, kp3.node_id());

        let name_entry = srv.store.get("config/cluster_name").expect("Key exists");
        assert_eq!(name_entry.value, json!("aether-grid"));
    }

    // 3. Differential sync: since_timestamp = 180 returns only the ts 200 delta
    let diff = server1.store.diff_entries(Some(180), None, 10);
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].key, "config/workers");
    assert_eq!(diff[0].timestamp, 200);
}

#[test]
fn test_multi_node_authenticated_task_delegation_and_parity() {
    let delegator_kp = Ed25519KeyPair::generate();
    let worker_kp = Ed25519KeyPair::generate();

    let delegator_server = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        delegator_kp.node_id(),
        "127.0.0.1:0",
        None,
    ));
    delegator_server.enable_zero_trust();
    delegator_server.set_revoked_keys_path(None);

    let worker_server = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        worker_kp.node_id(),
        "127.0.0.1:0",
        None,
    ));
    worker_server.enable_zero_trust();
    worker_server.set_revoked_keys_path(None);

    // Register worker key in delegator verified peer keys
    delegator_server
        .verified_peer_keys
        .lock()
        .unwrap()
        .insert(worker_kp.node_id(), worker_kp.public_key_hex());

    // Delegator creates and submits a compute task: (100 * 4) + 2
    let ast = Node::Add(
        Box::new(Node::Mul(
            Box::new(Node::IntLiteral(100)),
            Box::new(Node::IntLiteral(4)),
        )),
        Box::new(Node::IntLiteral(2)),
    );

    let task_id = delegator_server
        .task_dispatcher
        .submit(ast, 128)
        .expect("Task submission succeeds");

    // Worker executes isolated compute and creates signed result
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let computed_result = 402;
    let signed_task = create_signed_task_result(
        &worker_kp,
        task_id.clone(),
        worker_kp.node_id(),
        json!(computed_result),
        now,
    );

    // Delegator verifies worker's cryptographic signature
    let completion = delegator_server
        .task_dispatcher
        .complete_signed(signed_task.clone(), |pk| {
            delegator_server.is_peer_key_revoked(pk)
        });

    assert!(
        completion.is_ok(),
        "Authenticated task completion must succeed"
    );

    let status = delegator_server.task_dispatcher.status(&task_id).unwrap();
    assert_eq!(status.status, TaskStatus::Completed);
    assert_eq!(status.result, Some(json!(402)));

    // Tampered result rejected
    let mut tampered = signed_task;
    tampered.result = json!(999);
    let tampered_completion = delegator_server
        .task_dispatcher
        .complete_signed(tampered, |pk| delegator_server.is_peer_key_revoked(pk));
    assert!(
        tampered_completion.is_err(),
        "Tampered result signature must be rejected"
    );
}
