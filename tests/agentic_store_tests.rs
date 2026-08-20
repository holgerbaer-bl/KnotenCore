// Sprint 321: Distributed CRDT Key-Value Storage & State Sync
// Validates knc_store_put, knc_store_get, knc_store_sync, CRDT LWW conflict resolution,
// auth token guards, and multi-node peer state synchronization.

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{JsonRpcResponse, MeshKvStore, RpcServer};
use serde_json::Value;

fn make_server(node_id: &str) -> RpcServer {
    RpcServer::with_mesh(AgentPermissions::default(), node_id, "127.0.0.1:0", None)
}

fn make_authed_server(node_id: &str, token: &str) -> RpcServer {
    RpcServer::with_mesh(
        AgentPermissions::default(),
        node_id,
        "127.0.0.1:0",
        Some(token.to_string()),
    )
}

fn parse_response(raw: &str) -> JsonRpcResponse {
    serde_json::from_str(raw).expect("response must be valid JSON-RPC 2.0")
}

fn result_field<'a>(resp: &'a JsonRpcResponse, key: &str) -> &'a Value {
    resp.result
        .as_ref()
        .unwrap_or_else(|| panic!("expected result, got error: {:?}", resp.error))
        .get(key)
        .unwrap_or_else(|| panic!("key '{}' missing in result", key))
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Basic Store Operations (knc_store_put / knc_store_get)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_store_put_and_get() {
    let server = make_server("store-node-1");

    let put_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": {
            "key": "leader_id",
            "value": "node-alpha",
            "timestamp": 1000,
            "writer_id": "node-alpha"
        },
        "id": 1
    })
    .to_string();

    let put_raw = server.dispatch_request(&put_req);
    let put_resp = parse_response(&put_raw);
    assert!(put_resp.error.is_none());
    assert!(result_field(&put_resp, "updated").as_bool().unwrap());

    let get_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_get",
        "params": { "key": "leader_id" },
        "id": 2
    })
    .to_string();

    let get_raw = server.dispatch_request(&get_req);
    let get_resp = parse_response(&get_raw);
    assert!(get_resp.error.is_none());

    let entry = result_field(&get_resp, "entry");
    assert_eq!(entry["key"].as_str().unwrap(), "leader_id");
    assert_eq!(entry["value"].as_str().unwrap(), "node-alpha");
    assert_eq!(entry["timestamp"].as_u64().unwrap(), 1000);
}

#[test]
fn test_store_get_unknown_key_returns_null_entry() {
    let server = make_server("store-node-2");

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_get",
        "params": { "key": "missing_key" },
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    assert!(result_field(&resp, "entry").is_null());
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Auth Guards on Write & Sync Operations
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_store_put_and_sync_require_auth_on_authed_server() {
    let server = make_authed_server("store-node-authed", "secret-token");

    // Put without auth → must be rejected
    let put_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": { "key": "test", "value": 123 },
        "id": 1
    })
    .to_string();

    let put_resp = parse_response(&server.dispatch_request(&put_req));
    assert!(put_resp.error.is_some());
    assert_eq!(put_resp.error.unwrap().code, -32001);

    // Sync without auth → must be rejected
    let sync_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_sync",
        "params": { "entries": [] },
        "id": 2
    })
    .to_string();

    let sync_resp = parse_response(&server.dispatch_request(&sync_req));
    assert!(sync_resp.error.is_some());
    assert_eq!(sync_resp.error.unwrap().code, -32001);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. CRDT Last-Write-Wins (LWW) Conflict Resolution
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_crdt_lww_newer_timestamp_overwrites_older() {
    let store = MeshKvStore::new();

    // Initial write at t=100
    assert!(store.put("temperature", serde_json::json!(20.5), 100, "sensor-1"));

    // Older write at t=90 → must be ignored
    assert!(!store.put("temperature", serde_json::json!(18.0), 90, "sensor-2"));
    assert_eq!(
        store.get("temperature").unwrap().value,
        serde_json::json!(20.5)
    );

    // Newer write at t=110 → must overwrite
    assert!(store.put("temperature", serde_json::json!(22.1), 110, "sensor-3"));
    assert_eq!(
        store.get("temperature").unwrap().value,
        serde_json::json!(22.1)
    );
}

#[test]
fn test_crdt_lww_tiebreaker_on_equal_timestamps() {
    let store = MeshKvStore::new();

    // Write from "node-a" at t=500
    assert!(store.put("cluster_state", serde_json::json!("init"), 500, "node-a"));

    // Write from "node-b" (lexicographically greater than "node-a") at t=500 → wins
    assert!(store.put("cluster_state", serde_json::json!("active"), 500, "node-b"));
    assert_eq!(
        store.get("cluster_state").unwrap().value,
        serde_json::json!("active")
    );

    // Write from "node-a" at t=500 → loses tiebreaker
    assert!(!store.put("cluster_state", serde_json::json!("stale"), 500, "node-a"));
    assert_eq!(
        store.get("cluster_state").unwrap().value,
        serde_json::json!("active")
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Peer State Sync (knc_store_sync)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_peer_state_sync_merges_two_node_stores() {
    let server_a = make_server("node-a");
    let server_b = make_server("node-b");

    // Node A writes k1=10 at t=100, k2="old" at t=100
    let put1 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": { "key": "k1", "value": 10, "timestamp": 100, "writer_id": "node-a" },
        "id": 1
    })
    .to_string();
    server_a.dispatch_request(&put1);

    let put2 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": { "key": "k2", "value": "old", "timestamp": 100, "writer_id": "node-a" },
        "id": 2
    })
    .to_string();
    server_a.dispatch_request(&put2);

    // Node B writes k2="new" at t=200, k3=true at t=150
    let put3 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": { "key": "k2", "value": "new", "timestamp": 200, "writer_id": "node-b" },
        "id": 3
    })
    .to_string();
    server_b.dispatch_request(&put3);

    let put4 = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": { "key": "k3", "value": true, "timestamp": 150, "writer_id": "node-b" },
        "id": 4
    })
    .to_string();
    server_b.dispatch_request(&put4);

    // Node A syncs with Node B's entries
    let b_entries = server_b.store.dump_entries();
    let sync_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_sync",
        "params": { "entries": b_entries },
        "id": 5
    })
    .to_string();

    let sync_raw = server_a.dispatch_request(&sync_req);
    let sync_resp = parse_response(&sync_raw);
    assert!(sync_resp.error.is_none());

    // Node A should now contain k1=10, k2="new", k3=true
    assert_eq!(
        server_a.store.get("k1").unwrap().value,
        serde_json::json!(10)
    );
    assert_eq!(
        server_a.store.get("k2").unwrap().value,
        serde_json::json!("new")
    );
    assert_eq!(
        server_a.store.get("k3").unwrap().value,
        serde_json::json!(true)
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Handshake Advertises Storage Capabilities
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_handshake_advertises_crdt_store_capabilities() {
    let server = make_server("capability-node");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {},
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let caps = result_field(&resp, "capabilities");
    assert!(caps["crdt_store"].as_bool().unwrap());
    assert!(caps["peer_state_sync"].as_bool().unwrap());
    assert_eq!(
        result_field(&resp, "protocol_version").as_str().unwrap(),
        "v2.24.17"
    );
}

#[test]
fn test_store_rejects_future_u64_max_timestamp() {
    let server = make_server("timestamp-node");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": {
            "key": "poison_key",
            "value": "poison_val",
            "timestamp": u64::MAX,
            "writer_id": "attacker"
        },
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32602);
}

#[test]
fn test_store_rejects_unauth_get() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "authed-node",
        "127.0.0.1:0",
        Some("secret-key".to_string()),
    );
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_get",
        "params": { "key": "secret" },
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);
}

#[test]
fn test_mesh_auth_replay_attack_protection() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "replay-node",
        "127.0.0.1:0",
        Some("secret-token".to_string()),
    );

    let old_ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 120;

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_get",
        "params": {
            "key": "secret",
            "timestamp": old_ts,
            "signature": "dummy_sig"
        },
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);
}

#[test]
fn test_store_digest_deterministic_across_identical_nodes() {
    let server_a = make_server("node-digest-a");
    let server_b = make_server("node-digest-b");

    // Insert keys in forward order on node A
    server_a.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "k1", "value": "val1", "timestamp": 100, "writer_id": "w1" },
            "id": 1
        })
        .to_string(),
    );
    server_a.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "k2", "value": {"nested": 42}, "timestamp": 200, "writer_id": "w2" },
            "id": 2
        })
        .to_string(),
    );
    server_a.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "k3", "value": [1, 2, 3], "timestamp": 300, "writer_id": "w3" },
            "id": 3
        })
        .to_string(),
    );

    // Insert keys in reverse order on node B
    server_b.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "k3", "value": [1, 2, 3], "timestamp": 300, "writer_id": "w3" },
            "id": 10
        })
        .to_string(),
    );
    server_b.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "k1", "value": "val1", "timestamp": 100, "writer_id": "w1" },
            "id": 11
        })
        .to_string(),
    );
    server_b.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "k2", "value": {"nested": 42}, "timestamp": 200, "writer_id": "w2" },
            "id": 12
        })
        .to_string(),
    );

    // Query digests on both nodes
    let req_digest = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_digest",
        "params": {},
        "id": 100
    })
    .to_string();

    let resp_a = parse_response(&server_a.dispatch_request(&req_digest));
    let resp_b = parse_response(&server_b.dispatch_request(&req_digest));

    assert!(resp_a.error.is_none());
    assert!(resp_b.error.is_none());

    let digest_a = result_field(&resp_a, "state_digest").as_str().unwrap();
    let digest_b = result_field(&resp_b, "state_digest").as_str().unwrap();

    assert_eq!(
        digest_a, digest_b,
        "Digests must match regardless of insertion order"
    );
    assert_eq!(result_field(&resp_a, "entry_count").as_u64().unwrap(), 3);
    assert_eq!(
        result_field(&resp_a, "latest_timestamp").as_u64().unwrap(),
        300
    );
}

#[test]
fn test_store_differential_sync_only_transfers_deltas() {
    let server_a = make_server("node-sync-a");
    let server_b = make_server("node-sync-b");

    // Populate common base data
    for i in 1..=5 {
        server_a.dispatch_request(
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "knc_store_put",
                "params": { "key": format!("base_{}", i), "value": i, "timestamp": 100, "writer_id": "init" },
                "id": i
            })
            .to_string(),
        );
        server_b.dispatch_request(
            &serde_json::json!({
                "jsonrpc": "2.0",
                "method": "knc_store_put",
                "params": { "key": format!("base_{}", i), "value": i, "timestamp": 100, "writer_id": "init" },
                "id": i
            })
            .to_string(),
        );
    }

    // Add new updates only on Node A with timestamp 500
    server_a.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "delta_key_1", "value": "new_val_1", "timestamp": 500, "writer_id": "updater" },
            "id": 99
        })
        .to_string(),
    );
    server_a.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": { "key": "delta_key_2", "value": "new_val_2", "timestamp": 500, "writer_id": "updater" },
            "id": 100
        })
        .to_string(),
    );

    // Node B gets its own digest
    let req_b_digest = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_digest",
        "params": {},
        "id": 101
    })
    .to_string();
    let resp_b_digest = parse_response(&server_b.dispatch_request(&req_b_digest));
    let digest_b = result_field(&resp_b_digest, "state_digest")
        .as_str()
        .unwrap();

    // Node B requests differential updates from Node A since timestamp 200
    let diff_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_diff",
        "params": {
            "peer_digest": digest_b,
            "since_timestamp": 200
        },
        "id": 102
    })
    .to_string();

    let diff_raw = server_a.dispatch_request(&diff_req);
    let diff_resp = parse_response(&diff_raw);
    assert!(diff_resp.error.is_none());

    assert!(!result_field(&diff_resp, "in_sync").as_bool().unwrap());
    let entries = result_field(&diff_resp, "entries").as_array().unwrap();
    assert_eq!(
        entries.len(),
        2,
        "Only the 2 delta keys updated after ts 200 should be returned"
    );

    // Apply differential sync on Node B
    let sync_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_sync",
        "params": {
            "action": "push",
            "entries": entries
        },
        "id": 103
    })
    .to_string();
    let sync_resp = parse_response(&server_b.dispatch_request(&sync_req));
    assert!(sync_resp.error.is_none());
    assert_eq!(
        result_field(&sync_resp, "updated_entries")
            .as_u64()
            .unwrap(),
        2
    );

    // After sync, digests must match and knc_store_diff returns in_sync: true
    let resp_b_digest_after = parse_response(&server_b.dispatch_request(&req_b_digest));
    let digest_b_after = result_field(&resp_b_digest_after, "state_digest")
        .as_str()
        .unwrap();

    let resp_a_digest = parse_response(&server_a.dispatch_request(&req_b_digest));
    let digest_a = result_field(&resp_a_digest, "state_digest")
        .as_str()
        .unwrap();
    assert_eq!(digest_a, digest_b_after);

    let verify_diff_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_diff",
        "params": { "peer_digest": digest_b_after },
        "id": 104
    })
    .to_string();
    let verify_diff_resp = parse_response(&server_a.dispatch_request(&verify_diff_req));
    assert!(
        result_field(&verify_diff_resp, "in_sync")
            .as_bool()
            .unwrap()
    );
    assert_eq!(
        result_field(&verify_diff_resp, "entries")
            .as_array()
            .unwrap()
            .len(),
        0
    );
}

#[test]
fn test_store_digest_revoked_node_rejected() {
    let server = RpcServer::new(AgentPermissions::default());
    server.enable_zero_trust();
    server.set_revoked_keys_path(None);

    let evil_kp = aether_compiler::crypto_ed25519::Ed25519KeyPair::generate();
    let evil_pubkey = evil_kp.public_key_hex();
    server.revoke_peer_key(&evil_pubkey);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let canonical_msg = format!("{}:{}", now, "nonce-evil");
    let sig_hex = evil_kp.sign_hex(canonical_msg.as_bytes());

    // Call knc_store_digest from revoked node
    let digest_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_digest",
        "params": {
            "timestamp": now,
            "nonce": "nonce-evil",
            "public_key": evil_pubkey,
            "signature": sig_hex
        },
        "id": 1
    })
    .to_string();

    let resp = parse_response(&server.dispatch_request(&digest_req));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);

    // Call knc_store_diff from revoked node
    let diff_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_diff",
        "params": {
            "peer_digest": "abc",
            "timestamp": now,
            "nonce": "nonce-evil",
            "public_key": evil_pubkey,
            "signature": sig_hex
        },
        "id": 2
    })
    .to_string();

    let resp_diff = parse_response(&server.dispatch_request(&diff_req));
    assert!(resp_diff.error.is_some());
    assert_eq!(resp_diff.error.unwrap().code, -32001);
}

#[test]
fn test_store_put_ed25519_forces_public_key_writer_id() {
    let server = RpcServer::new(AgentPermissions::default());
    server.enable_zero_trust();

    let kp = aether_compiler::crypto_ed25519::Ed25519KeyPair::generate();
    let pubkey = kp.public_key_hex();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let canonical_msg = format!("{}:{}:{}", now, "nonce-store-put", "spoofed_sender");
    let sig_hex = kp.sign_hex(canonical_msg.as_bytes());

    // Call knc_store_put with spoofed writer_id and sender_node_id
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": {
            "key": "secure_config",
            "value": "locked_value",
            "timestamp": now,
            "nonce": "nonce-store-put",
            "public_key": pubkey,
            "signature": sig_hex,
            "writer_id": "spoofed_admin_node",
            "sender_node_id": "spoofed_sender"
        },
        "id": 1
    })
    .to_string();

    let resp = parse_response(&server.dispatch_request(&req));
    assert!(resp.error.is_none());
    assert!(result_field(&resp, "written").as_bool().unwrap());

    // Response and internal entry writer_id MUST be ed25519:<pubkey_hex>
    let expected_writer = format!("ed25519:{}", pubkey.to_lowercase());
    assert_eq!(
        result_field(&resp, "writer_id").as_str().unwrap(),
        expected_writer
    );

    let stored_entry = server.store.get("secure_config").unwrap();
    assert_eq!(stored_entry.writer_id, expected_writer);
}

#[test]
fn test_store_put_legacy_hmac_marks_scoped_writer_id() {
    let server = make_authed_server("hmac-server", "shared-mesh-secret");

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_store_put",
        "params": {
            "key": "shared_setting",
            "value": "active",
            "timestamp": now,
            "mesh_auth_token": "shared-mesh-secret",
            "sender_node_id": "worker-node-42"
        },
        "id": 1
    })
    .to_string();

    let resp = parse_response(&server.dispatch_request(&req));
    assert!(resp.error.is_none());
    assert!(result_field(&resp, "written").as_bool().unwrap());

    // Response and internal entry writer_id MUST be legacy-hmac:<sender_node_id>
    assert_eq!(
        result_field(&resp, "writer_id").as_str().unwrap(),
        "legacy-hmac:worker-node-42"
    );

    let stored_entry = server.store.get("shared_setting").unwrap();
    assert_eq!(stored_entry.writer_id, "legacy-hmac:worker-node-42");
}

#[test]
fn test_store_digest_incorporates_authenticated_writer_identities() {
    let server_zt = RpcServer::new(AgentPermissions::default());
    server_zt.enable_zero_trust();

    let kp1 = aether_compiler::crypto_ed25519::Ed25519KeyPair::generate();
    let pubkey1 = kp1.public_key_hex();

    let kp2 = aether_compiler::crypto_ed25519::Ed25519KeyPair::generate();
    let pubkey2 = kp2.public_key_hex();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let msg1 = format!("{}:{}:", now, "nonce-d1");
    let sig1 = kp1.sign_hex(msg1.as_bytes());

    let resp1 = parse_response(&server_zt.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": {
                "key": "same_key",
                "value": "same_val",
                "timestamp": now,
                "nonce": "nonce-d1",
                "public_key": pubkey1,
                "signature": sig1
            },
            "id": 1
        })
        .to_string(),
    ));
    assert!(resp1.error.is_none());
    assert!(result_field(&resp1, "written").as_bool().unwrap());

    let digest1 = server_zt.store.compute_state_digest();

    // Re-create server and insert identical key/value/timestamp but from different authenticated key
    let server_zt2 = RpcServer::new(AgentPermissions::default());
    server_zt2.enable_zero_trust();

    let msg2 = format!("{}:{}:", now, "nonce-d2");
    let sig2 = kp2.sign_hex(msg2.as_bytes());

    let resp2 = parse_response(&server_zt2.dispatch_request(
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_store_put",
            "params": {
                "key": "same_key",
                "value": "same_val",
                "timestamp": now,
                "nonce": "nonce-d2",
                "public_key": pubkey2,
                "signature": sig2
            },
            "id": 2
        })
        .to_string(),
    ));
    assert!(resp2.error.is_none());
    assert!(result_field(&resp2, "written").as_bool().unwrap());

    let digest2 = server_zt2.store.compute_state_digest();

    // Digests must differ because writer_id is bound to the distinct authenticated public keys
    assert_ne!(
        digest1, digest2,
        "State digests must incorporate authenticated writer identities"
    );
}

#[test]
fn test_version_assertion_sprint354() {
    assert_eq!(aether_compiler::rpc::KNC_PROTOCOL_VERSION, "v2.24.17");
}
