// =============================================================================
// Sprint 328: Zero-Trust Mesh Phase 2 — Key Rotation & Peer Revocation Tests
// =============================================================================

use aether_compiler::crypto_ed25519::Ed25519KeyPair;
use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{MAX_NONCE_CACHE_CAPACITY, NonceCache, RpcServer};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};

fn current_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[test]
fn test_zero_trust_key_rotation_handshake() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-key-rotation-test",
        "127.0.0.1:9091",
        None,
    );
    server.enable_zero_trust();

    let initial_pubkey = server.public_key_hex();
    let now = current_ts();

    // 1. Dispatch knc_mesh_rotate_key
    let (pubkey_hex, sig_hex) = server.sign_envelope("nonce-rot-1", now);
    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_rotate_key",
        "params": {
            "zero_trust_envelope": {
                "public_key": pubkey_hex,
                "signature": sig_hex,
                "timestamp": now,
                "nonce": "nonce-rot-1",
                "sender_node_id": "node-key-rotation-test"
            }
        },
        "id": 1
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["old_public_key"], initial_pubkey);

    let new_pubkey = resp["result"]["new_public_key"].as_str().unwrap();
    assert_ne!(new_pubkey, initial_pubkey);
    assert_eq!(server.public_key_hex(), new_pubkey);

    // 2. Sign envelope using new key and verify request succeeds
    let (new_pub_envelope, new_sig) = server.sign_envelope("nonce-rot-2", now);
    assert_eq!(new_pub_envelope, new_pubkey);

    let req2 = json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {
            "zero_trust_envelope": {
                "public_key": new_pub_envelope,
                "signature": new_sig,
                "timestamp": now,
                "nonce": "nonce-rot-2",
                "sender_node_id": "node-key-rotation-test"
            }
        },
        "id": 2
    });

    let resp2_str = server.dispatch_request(&req2.to_string());
    let resp2: serde_json::Value = serde_json::from_str(&resp2_str).unwrap();
    assert_eq!(resp2["result"]["status"], "ok");
    assert_eq!(resp2["result"]["local_public_key"], new_pubkey);
    assert!(
        resp2["result"]["capabilities"]["key_rotation"]
            .as_bool()
            .unwrap()
    );
    assert!(
        resp2["result"]["capabilities"]["peer_revocation"]
            .as_bool()
            .unwrap()
    );
}

#[test]
fn test_zero_trust_peer_revocation_list_crl() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-crl-test",
        "127.0.0.1:9092",
        None,
    );
    server.enable_zero_trust();

    let peer_keypair = Ed25519KeyPair::generate();
    let peer_pubkey = peer_keypair.public_key_hex();
    let now = current_ts();

    // 1. Verify handshake from valid peer succeeds
    let msg = format!("{}:{}:{}", now, "nonce-peer-1", "peer-compromised");
    let sig_hex = peer_keypair.sign_hex(msg.as_bytes());

    let req_hs = json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {
            "zero_trust_envelope": {
                "public_key": peer_pubkey,
                "signature": sig_hex,
                "timestamp": now,
                "nonce": "nonce-peer-1",
                "sender_node_id": "peer-compromised"
            }
        },
        "id": 10
    });

    let resp_hs_str = server.dispatch_request(&req_hs.to_string());
    let resp_hs: serde_json::Value = serde_json::from_str(&resp_hs_str).unwrap();
    assert_eq!(resp_hs["result"]["status"], "ok");

    // 2. Revoke peer public key via knc_mesh_revoke_peer
    let (self_pub, self_sig) = server.sign_envelope("nonce-revoke-1", now);
    let req_revoke = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_revoke_peer",
        "params": {
            "peer_pubkey": peer_pubkey,
            "zero_trust_envelope": {
                "public_key": self_pub,
                "signature": self_sig,
                "timestamp": now,
                "nonce": "nonce-revoke-1",
                "sender_node_id": "node-crl-test"
            }
        },
        "id": 11
    });

    let resp_rev_str = server.dispatch_request(&req_revoke.to_string());
    let resp_rev: serde_json::Value = serde_json::from_str(&resp_rev_str).unwrap();
    assert_eq!(resp_rev["result"]["status"], "ok");
    assert_eq!(resp_rev["result"]["revoked"], true);
    assert_eq!(
        resp_rev["result"]["revoked_peer_key"],
        peer_pubkey.to_lowercase()
    );

    assert!(server.is_peer_key_revoked(&peer_pubkey));

    // 3. Subsequent request from revoked peer must be rejected
    let msg2 = format!("{}:{}:{}", now, "nonce-peer-2", "peer-compromised");
    let sig2_hex = peer_keypair.sign_hex(msg2.as_bytes());

    let req_blocked = json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {
            "zero_trust_envelope": {
                "public_key": peer_pubkey,
                "signature": sig2_hex,
                "timestamp": now,
                "nonce": "nonce-peer-2",
                "sender_node_id": "peer-compromised"
            }
        },
        "id": 12
    });

    let resp_blocked_str = server.dispatch_request(&req_blocked.to_string());
    let resp_blocked: serde_json::Value = serde_json::from_str(&resp_blocked_str).unwrap();
    assert_eq!(resp_blocked["error"]["code"], -32001);
    assert!(
        resp_blocked["error"]["message"]
            .as_str()
            .unwrap()
            .contains("revoked")
    );
}

#[test]
fn test_zero_trust_nonce_lru_eviction() {
    let mut cache = NonceCache::new();
    assert!(cache.is_empty());

    let now_ts = current_ts();

    // 1. Normal insertion and replay detection
    assert!(cache.insert("key1:nonce1".to_string(), now_ts));
    assert!(!cache.insert("key1:nonce1".to_string(), now_ts));
    assert_eq!(cache.len(), 1);

    // 2. Capacity eviction test
    for i in 2..=(MAX_NONCE_CACHE_CAPACITY + 50) {
        let entry = format!("key1:nonce{}", i);
        assert!(cache.insert(entry, now_ts));
    }

    assert_eq!(cache.len(), MAX_NONCE_CACHE_CAPACITY);
    // Oldest nonce key1:nonce1 was evicted when capacity reached MAX_NONCE_CACHE_CAPACITY
    assert!(!cache.contains("key1:nonce1"));
    // Re-inserting key1:nonce1 works now because it was LRU evicted
    assert!(cache.insert("key1:nonce1".to_string(), now_ts));
}

#[test]
fn test_server_enforced_quorum_rejects_client_manipulation() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "quorum-server-test",
        "127.0.0.1:0",
        None,
    );

    // Register 2 active peers -> active_nodes = 3 -> server_threshold = (3/2) + 1 = 2
    {
        let mut peers = server.peers.lock().unwrap();
        peers.insert(
            "peer-1".to_string(),
            aether_compiler::rpc::MeshPeer {
                node_id: "peer-1".to_string(),
                address: "127.0.0.1:9001".to_string(),
                capabilities: vec![],
                last_seen: 100,
                latency_ms: 5,
                status: "Active".to_string(),
            },
        );
        peers.insert(
            "peer-2".to_string(),
            aether_compiler::rpc::MeshPeer {
                node_id: "peer-2".to_string(),
                address: "127.0.0.1:9002".to_string(),
                capabilities: vec![],
                last_seen: 100,
                latency_ms: 10,
                status: "Active".to_string(),
            },
        );
    }

    // Client attempts to pass required_quorum: 1
    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_swarm_quorum",
        "params": {
            "operation": "critical_op",
            "required_quorum": 1
        },
        "id": 1
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["result"]["status"], "ok");
    assert_eq!(resp["result"]["active_nodes"], 3);
    // Server-enforced quorum threshold MUST remain 2 (max(2, 1) = 2)
    assert_eq!(resp["result"]["quorum_threshold"], 2);
}

#[test]
fn test_zero_trust_blocks_forced_self_election() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "election-node",
        "127.0.0.1:0",
        Some("secret".to_string()),
    );
    server.enable_zero_trust();

    // Establish initial bootstrap leader
    let _ = server
        .swarm_governance
        .elect("election-node", Some("election-node"), None, false);

    let now = current_ts();
    let (pubkey, sig) = server.sign_envelope("nonce-elect-force", now);

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_swarm_elect",
        "params": {
            "candidate_node_id": "election-node",
            "force": true,
            "zero_trust_envelope": {
                "public_key": pubkey,
                "signature": sig,
                "timestamp": now,
                "nonce": "nonce-elect-force",
                "sender_node_id": "election-node"
            }
        },
        "id": 1
    });

    let resp_str = server.dispatch_request(&req.to_string());
    let resp: serde_json::Value = serde_json::from_str(&resp_str).unwrap();

    assert_eq!(resp["error"]["code"], -32001);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Unauthorized")
    );
}

#[test]
fn test_quorum_gated_peer_revocation() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "revocation-quorum-node",
        "127.0.0.1:0",
        None,
    );

    // Add 2 stale peers (active_nodes = 1, server_threshold = 2 -> quorum_reached = false)
    {
        let mut peers = server.peers.lock().unwrap();
        peers.insert(
            "stale-peer-1".to_string(),
            aether_compiler::rpc::MeshPeer {
                node_id: "stale-peer-1".to_string(),
                address: "127.0.0.1:9001".to_string(),
                capabilities: vec![],
                last_seen: 100,
                latency_ms: 5,
                status: "Stale".to_string(),
            },
        );
        peers.insert(
            "stale-peer-2".to_string(),
            aether_compiler::rpc::MeshPeer {
                node_id: "stale-peer-2".to_string(),
                address: "127.0.0.1:9002".to_string(),
                capabilities: vec![],
                last_seen: 100,
                latency_ms: 10,
                status: "Stale".to_string(),
            },
        );
    }

    // Attempt peer revocation when quorum is NOT reached
    let req_unauth = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_revoke_peer",
        "params": {
            "peer_pubkey": "abc123revokekey",
            "required_quorum": 2
        },
        "id": 1
    });

    let resp_unauth_str = server.dispatch_request(&req_unauth.to_string());
    let resp_unauth: serde_json::Value = serde_json::from_str(&resp_unauth_str).unwrap();

    assert_eq!(resp_unauth["error"]["code"], -32001);
    assert!(
        resp_unauth["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Quorum consensus not reached")
    );

    // Activate peers so active_nodes = 3 >= 2 (quorum_reached = true)
    {
        let mut peers = server.peers.lock().unwrap();
        if let Some(p) = peers.get_mut("stale-peer-1") {
            p.status = "Active".to_string();
        }
        if let Some(p) = peers.get_mut("stale-peer-2") {
            p.status = "Active".to_string();
        }
    }

    let req_auth = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_revoke_peer",
        "params": {
            "peer_pubkey": "abc123revokekey"
        },
        "id": 2
    });

    let resp_auth_str = server.dispatch_request(&req_auth.to_string());
    let resp_auth: serde_json::Value = serde_json::from_str(&resp_auth_str).unwrap();

    assert_eq!(resp_auth["result"]["status"], "ok");
    assert_eq!(resp_auth["result"]["revoked"], true);
}
