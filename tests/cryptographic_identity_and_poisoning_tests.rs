// Sprint 359: Cryptographic Identity Binding, Exploit Negative Tests & Mutex Poisoning Resilience Tests

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use aether_compiler::crypto_ed25519::Ed25519KeyPair;
use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::types::KNC_PROTOCOL_VERSION;
use aether_compiler::rpc::{NodeRole, RpcServer};
use serde_json::{Value, json};

fn current_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn parse_response(json_str: &str) -> Value {
    serde_json::from_str(json_str).expect("Valid JSON-RPC response")
}

#[test]
fn test_version_assertion_sprint360_identity() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.24");
}

#[test]
fn test_valid_key_and_matching_node_id_accepted() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9001",
        None,
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let client_node_id = client_keypair.node_id();
    let client_pubkey = client_keypair.public_key_hex();
    let now = current_ts();
    let nonce = "nonce-valid-1";

    let msg = format!("{}:{}:{}", now, nonce, client_node_id);
    let sig = client_keypair.sign_hex(msg.as_bytes());

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": client_pubkey,
                "signature": sig,
                "timestamp": now,
                "nonce": nonce,
                "sender_node_id": client_node_id
            }
        },
        "id": 1
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["result"]["status"], "ok");
}

#[test]
fn test_spoofed_sender_node_id_with_valid_third_party_signature_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9002",
        None,
    );
    server.enable_zero_trust();

    let alice_keypair = Ed25519KeyPair::generate();
    let alice_node_id = alice_keypair.node_id();
    let alice_pubkey = alice_keypair.public_key_hex();
    let now = current_ts();
    let nonce = "nonce-alice-valid";

    // Alice generates a valid signature over her canonical node_id
    let alice_msg = format!("{}:{}:{}", now, nonce, alice_node_id);
    let alice_sig = alice_keypair.sign_hex(alice_msg.as_bytes());

    // Exploit attempt 1: Attacker sends Alice's signature claiming sender is "node-attacker"
    let req_spoof_sender = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": alice_pubkey,
                "signature": alice_sig,
                "timestamp": now,
                "nonce": nonce,
                "sender_node_id": "node-attacker"
            }
        },
        "id": 2
    });

    let resp1 = parse_response(&server.dispatch_request(&req_spoof_sender.to_string()));
    assert_eq!(resp1["error"]["code"], -32001);
    assert!(
        resp1["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Sender node ID does not match canonical derived public key identity")
    );

    // Exploit attempt 2: Attacker generates their own keypair, signs their own envelope,
    // but claims sender_node_id: "ed25519:<alice_pubkey>"
    let eve_keypair = Ed25519KeyPair::generate();
    let eve_pubkey = eve_keypair.public_key_hex();
    let eve_nonce = "nonce-eve-1";
    let spoofed_sender = format!("ed25519:{}", alice_pubkey);
    let eve_msg = format!("{}:{}:{}", now, eve_nonce, spoofed_sender);
    let eve_sig = eve_keypair.sign_hex(eve_msg.as_bytes());

    let req_spoof_key = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": eve_pubkey,
                "signature": eve_sig,
                "timestamp": now,
                "nonce": eve_nonce,
                "sender_node_id": spoofed_sender
            }
        },
        "id": 3
    });

    let resp2 = parse_response(&server.dispatch_request(&req_spoof_key.to_string()));
    assert_eq!(resp2["error"]["code"], -32001);
    assert!(
        resp2["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Sender node ID does not match canonical derived public key identity")
    );
}

#[test]
fn test_foreign_public_key_claiming_known_node_id_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9003",
        None,
    );
    server.enable_zero_trust();

    let alice_keypair = Ed25519KeyPair::generate();
    let alice_node_id = alice_keypair.node_id();
    let alice_pubkey = alice_keypair.public_key_hex();
    let now = current_ts();

    // 1. Register Alice with canonical node_id
    let msg = format!("{}:{}:{}", now, "nonce-reg-alice", alice_node_id);
    let sig = alice_keypair.sign_hex(msg.as_bytes());
    let req_reg = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_verify_peer",
        "params": {
            "peer_node_id": alice_node_id,
            "public_key": alice_pubkey,
            "zero_trust_envelope": {
                "public_key": alice_pubkey,
                "signature": sig,
                "timestamp": now,
                "nonce": "nonce-reg-alice",
                "sender_node_id": alice_node_id
            }
        },
        "id": 10
    });
    let resp_reg = parse_response(&server.dispatch_request(&req_reg.to_string()));
    assert_eq!(resp_reg["result"]["status"], "ok");

    // 2. Mallory generates KM and attempts to sign claiming Alice's canonical node_id
    let mallory_keypair = Ed25519KeyPair::generate();
    let mallory_pubkey = mallory_keypair.public_key_hex();
    let m_nonce = "nonce-mallory-impersonate";
    let m_msg = format!("{}:{}:{}", now, m_nonce, alice_node_id);
    let m_sig = mallory_keypair.sign_hex(m_msg.as_bytes());

    let req_mallory = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": mallory_pubkey,
                "signature": m_sig,
                "timestamp": now,
                "nonce": m_nonce,
                "sender_node_id": alice_node_id
            }
        },
        "id": 11
    });

    let resp_mallory = parse_response(&server.dispatch_request(&req_mallory.to_string()));
    assert_eq!(resp_mallory["error"]["code"], -32001);
    assert!(
        resp_mallory["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Sender node ID does not match canonical derived public key identity")
    );

    // 3. Mallory attempts to claim server's own canonical node_id
    let server_id = server.canonical_node_id();
    let s_nonce = "nonce-mallory-server";
    let s_msg = format!("{}:{}:{}", now, s_nonce, server_id);
    let s_sig = mallory_keypair.sign_hex(s_msg.as_bytes());

    let req_server_impersonate = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": mallory_pubkey,
                "signature": s_sig,
                "timestamp": now,
                "nonce": s_nonce,
                "sender_node_id": server_id
            }
        },
        "id": 12
    });

    let resp_srv = parse_response(&server.dispatch_request(&req_server_impersonate.to_string()));
    assert_eq!(resp_srv["error"]["code"], -32001);
    assert!(
        resp_srv["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Sender node ID does not match canonical derived public key identity")
    );
}

#[test]
fn test_altered_envelope_payload_with_valid_signature_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9004",
        None,
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let client_node_id = client_keypair.node_id();
    let client_pubkey = client_keypair.public_key_hex();
    let now = current_ts();
    let nonce = "nonce-payload-1";

    let msg = format!("{}:{}:{}", now, nonce, client_node_id);
    let sig = client_keypair.sign_hex(msg.as_bytes());

    // Exploit 1: Tampered outer params.sender_node_id diverging from envelope sender
    let req_divergent = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "sender_node_id": "node-divergent-attacker",
            "zero_trust_envelope": {
                "public_key": client_pubkey,
                "signature": sig,
                "timestamp": now,
                "nonce": nonce,
                "sender_node_id": client_node_id
            }
        },
        "id": 20
    });

    let resp_div = parse_response(&server.dispatch_request(&req_divergent.to_string()));
    assert_eq!(resp_div["error"]["code"], -32001);
    assert!(
        resp_div["error"]["message"]
            .as_str()
            .unwrap()
            .contains("altered payload")
    );

    // Exploit 2: Tampered timestamp inside envelope after signing
    let nonce2 = "nonce-payload-2";
    let msg2 = format!("{}:{}:{}", now, nonce2, client_node_id);
    let sig2 = client_keypair.sign_hex(msg2.as_bytes());

    let req_tampered_ts = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": client_pubkey,
                "signature": sig2,
                "timestamp": now + 5,
                "nonce": nonce2,
                "sender_node_id": client_node_id
            }
        },
        "id": 21
    });

    let resp_ts = parse_response(&server.dispatch_request(&req_tampered_ts.to_string()));
    assert_eq!(resp_ts["error"]["code"], -32001);
    assert!(
        resp_ts["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Invalid Ed25519 signature")
    );
}

#[test]
fn test_revoked_public_key_attempting_replay_or_invocation_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9005",
        None,
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let client_node_id = client_keypair.node_id();
    let client_pubkey = client_keypair.public_key_hex();
    let now = current_ts();

    // Revoke client key
    server.revoke_peer_key(&client_pubkey);
    assert!(server.is_peer_key_revoked(&client_pubkey));

    // Client signs a fresh valid envelope
    let nonce = "nonce-revoked-fresh";
    let msg = format!("{}:{}:{}", now, nonce, client_node_id);
    let sig = client_keypair.sign_hex(msg.as_bytes());

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": client_pubkey,
                "signature": sig,
                "timestamp": now,
                "nonce": nonce,
                "sender_node_id": client_node_id
            }
        },
        "id": 30
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert_eq!(resp["error"]["code"], -32001);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Peer public key has been revoked")
    );
}

#[test]
fn test_mutex_poisoning_resilience_verified_peer_keys() {
    let server = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-poison-test",
        "127.0.0.1:9006",
        None,
    ));

    // Deliberately poison verified_peer_keys mutex via panic in spawned thread
    let server_clone = Arc::clone(&server);
    let handle = std::thread::spawn(move || {
        let _guard = server_clone.verified_peer_keys.lock().unwrap();
        panic!("Simulated worker panic while holding verified_peer_keys mutex");
    });
    let _ = handle.join();

    assert!(server.verified_peer_keys.is_poisoned());

    // Now attempt knc_mesh_verify_peer: must reject state mutation and fail safely with InternalSecurityError (-32000)
    let client_keypair = Ed25519KeyPair::generate();
    let client_node_id = client_keypair.node_id();
    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_verify_peer",
        "params": {
            "peer_node_id": client_node_id,
            "public_key": client_keypair.public_key_hex()
        },
        "id": 40
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert_eq!(resp["error"]["code"], -32000);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("InternalSecurityError")
    );

    // Also verify that check_mesh_auth fails safely (-32001) when verified_peer_keys is poisoned
    server.enable_zero_trust();
    let now = current_ts();
    let nonce_zt = "nonce-zt-poison";
    let msg_zt = format!("{}:{}:{}", now, nonce_zt, client_node_id);
    let sig_zt = client_keypair.sign_hex(msg_zt.as_bytes());
    let req_zt = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": client_keypair.public_key_hex(),
                "signature": sig_zt,
                "timestamp": now,
                "nonce": nonce_zt,
                "sender_node_id": client_node_id
            }
        },
        "id": 41
    });
    let resp_zt = parse_response(&server.dispatch_request(&req_zt.to_string()));
    assert_eq!(resp_zt["error"]["code"], -32001);
    assert!(
        resp_zt["error"]["message"]
            .as_str()
            .unwrap()
            .contains("InternalSecurityError")
    );
}

#[test]
fn test_mutex_poisoning_resilience_revoked_keys_fails_closed() {
    let server = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-crl-poison-test",
        "127.0.0.1:9007",
        None,
    ));

    // Deliberately poison revoked_peer_keys mutex
    let server_clone = Arc::clone(&server);
    let handle = std::thread::spawn(move || {
        let _guard = server_clone.revoked_peer_keys.lock().unwrap();
        panic!("Simulated panic while holding revoked_peer_keys lock");
    });
    let _ = handle.join();

    assert!(server.revoked_peer_keys.is_poisoned());

    // Fail-closed invariant: is_peer_key_revoked MUST return true on poisoned lock
    assert!(
        server.is_peer_key_revoked("any-unverified-key"),
        "Fail-closed invariant: poisoned revocation list must treat keys as revoked"
    );
}

#[test]
fn test_mutex_poisoning_resilience_swarm_governance() {
    let server = Arc::new(RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-gov-poison-test",
        "127.0.0.1:9008",
        None,
    ));

    // Deliberately poison current_role mutex in swarm governance
    let server_clone = Arc::clone(&server);
    let handle = std::thread::spawn(move || {
        let _guard = server_clone.swarm_governance.current_role.lock().unwrap();
        panic!("Simulated panic while holding current_role lock");
    });
    let _ = handle.join();

    assert!(server.swarm_governance.current_role.is_poisoned());

    // Fail-safe read: role() must return Observer rather than panicking or crashing
    assert_eq!(server.swarm_governance.role(), NodeRole::Observer);

    // Fail-safe mutation: elect() must reject state mutations and return InternalSecurityError
    let elect_res = server
        .swarm_governance
        .elect("node-gov-poison-test", None, Some(2), false);
    assert!(elect_res.is_err());
    assert!(elect_res.unwrap_err().contains("InternalSecurityError"));
}

#[test]
fn test_exploit_arbitrary_node_id_squatting_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9010",
        None,
    );
    server.enable_zero_trust();

    let attacker_keypair = Ed25519KeyPair::generate();
    let attacker_pubkey = attacker_keypair.public_key_hex();
    let now = current_ts();
    let nonce = "nonce-squat-1";

    // Attacker signs payload claiming arbitrary victim node_id "node-victim-treasury"
    let msg = format!("{}:{}:{}", now, nonce, "node-victim-treasury");
    let sig = attacker_keypair.sign_hex(msg.as_bytes());

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": attacker_pubkey,
                "signature": sig,
                "timestamp": now,
                "nonce": nonce,
                "sender_node_id": "node-victim-treasury"
            }
        },
        "id": 101
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert_eq!(resp["error"]["code"], -32001);
    assert!(
        resp["error"]["message"]
            .as_str()
            .unwrap()
            .contains("Sender node ID does not match canonical derived public key identity")
    );
}

#[test]
fn test_exploit_legacy_hmac_claiming_knc_identity_rejected() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9011",
        Some("shared-mesh-secret".to_string()),
    );
    // Server is NOT in zero-trust mode, so Legacy-HMAC is accepted for normal nodes
    assert!(!server.is_zero_trust());

    // Exploit attempt 1: Legacy token claiming knc-* identity directly in params
    let req_token = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "mesh_auth_token": "shared-mesh-secret",
            "sender_node_id": "knc-deadbeef0123456789abcdef0123456789abcdef0123456789abcdef01234567"
        },
        "id": 102
    });

    let resp1 = parse_response(&server.dispatch_request(&req_token.to_string()));
    assert_eq!(resp1["error"]["code"], -32001);
    assert!(
        resp1["error"]["message"].as_str().unwrap().contains(
            "Legacy-HMAC requests cannot claim canonical self-certifying 'knc-*' identity"
        )
    );

    // Exploit attempt 2: HMAC signature claiming knc-* identity
    let now = current_ts();
    let claimed_id = "knc-cafebabe0123456789abcdef0123456789abcdef0123456789abcdef01234567";
    let nonce = "nonce-hmac-knc-1";
    let message = format!("{}:{}", now, claimed_id);
    let sig = aether_compiler::rpc::hmac_sha256(b"shared-mesh-secret", message.as_bytes());

    let req_sig = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "timestamp": now,
            "nonce": nonce,
            "sender_node_id": claimed_id,
            "mesh_auth_signature": sig
        },
        "id": 103
    });

    let resp2 = parse_response(&server.dispatch_request(&req_sig.to_string()));
    assert_eq!(resp2["error"]["code"], -32001);
    assert!(
        resp2["error"]["message"].as_str().unwrap().contains(
            "Legacy-HMAC requests cannot claim canonical self-certifying 'knc-*' identity"
        )
    );
}

#[test]
fn test_canonical_self_certifying_identity_accepted() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-server",
        "127.0.0.1:9012",
        None,
    );
    server.enable_zero_trust();

    let client_keypair = Ed25519KeyPair::generate();
    let canonical_id = client_keypair.node_id();
    let client_pubkey = client_keypair.public_key_hex();
    let now = current_ts();
    let nonce = "nonce-canonical-accepted-1";

    let msg = format!("{}:{}:{}", now, nonce, canonical_id);
    let sig = client_keypair.sign_hex(msg.as_bytes());

    let req = json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_ping",
        "params": {
            "zero_trust_envelope": {
                "public_key": client_pubkey,
                "signature": sig,
                "timestamp": now,
                "nonce": nonce,
                "sender_node_id": canonical_id
            }
        },
        "id": 104
    });

    let resp = parse_response(&server.dispatch_request(&req.to_string()));
    assert_eq!(resp["jsonrpc"], "2.0");
    assert_eq!(resp["result"]["status"], "ok");
}
