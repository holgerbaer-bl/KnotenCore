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
        "v2.24.12"
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
