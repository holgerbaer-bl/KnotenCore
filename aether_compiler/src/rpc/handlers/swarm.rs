use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::rpc::types::{JsonRpcResponse, validate_param_string_len};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    Leader,
    Worker,
    Candidate,
    Storage,
    Observer,
}

/// Distributed Raft Voting & Swarm Governance.
pub struct SwarmGovernance {
    pub current_role: Mutex<NodeRole>,
    pub leader_node_id: Mutex<Option<String>>,
    pub current_term: Mutex<u64>,
    pub voted_for: Mutex<Option<String>>,
    pub votes_received: Mutex<HashSet<String>>,
    pub last_heartbeat_timestamp: Mutex<u64>,
}

impl Default for SwarmGovernance {
    fn default() -> Self {
        Self::new()
    }
}

fn current_now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl SwarmGovernance {
    pub fn new() -> Self {
        Self {
            current_role: Mutex::new(NodeRole::Worker),
            leader_node_id: Mutex::new(None),
            current_term: Mutex::new(1),
            voted_for: Mutex::new(None),
            votes_received: Mutex::new(HashSet::new()),
            last_heartbeat_timestamp: Mutex::new(current_now_ms()),
        }
    }

    pub fn role(&self) -> NodeRole {
        match self.current_role.lock() {
            Ok(r) => r.clone(),
            Err(_) => NodeRole::Observer, // Fail-safe
        }
    }

    pub fn set_role(&self, role: NodeRole) {
        if let Ok(mut r) = self.current_role.lock() {
            *r = role;
        }
    }

    pub fn leader_id(&self) -> Option<String> {
        match self.leader_node_id.lock() {
            Ok(l) => l.clone(),
            Err(_) => None,
        }
    }

    pub fn term(&self) -> u64 {
        match self.current_term.lock() {
            Ok(t) => *t,
            Err(_) => 0,
        }
    }

    pub fn process_heartbeat(&self, leader_term: u64, leader_id: &str) -> (u64, bool) {
        let mut term = match self.current_term.lock() {
            Ok(t) => t,
            Err(_) => return (0, false), // Fail-safe rejection
        };
        let mut role = match self.current_role.lock() {
            Ok(r) => r,
            Err(_) => return (*term, false),
        };
        let mut leader = match self.leader_node_id.lock() {
            Ok(l) => l,
            Err(_) => return (*term, false),
        };
        let mut last_hb = match self.last_heartbeat_timestamp.lock() {
            Ok(h) => h,
            Err(_) => return (*term, false),
        };

        if leader_term < *term {
            return (*term, false);
        }

        *term = leader_term;
        *leader = Some(leader_id.to_string());
        if *role == NodeRole::Candidate {
            *role = NodeRole::Worker;
        }
        *last_hb = current_now_ms();

        (*term, true)
    }

    pub fn last_heartbeat_elapsed_ms(&self) -> u64 {
        let last = match self.last_heartbeat_timestamp.lock() {
            Ok(l) => *l,
            Err(_) => 0,
        };
        current_now_ms().saturating_sub(last)
    }

    pub fn touch_heartbeat(&self) {
        if let Ok(mut last) = self.last_heartbeat_timestamp.lock() {
            *last = current_now_ms();
        }
    }

    pub fn request_vote(&self, candidate_term: u64, candidate_id: &str) -> (u64, bool) {
        let mut term = match self.current_term.lock() {
            Ok(t) => t,
            Err(_) => return (0, false),
        };
        let mut voted = match self.voted_for.lock() {
            Ok(v) => v,
            Err(_) => return (*term, false),
        };
        let mut role = match self.current_role.lock() {
            Ok(r) => r,
            Err(_) => return (*term, false),
        };
        let mut leader = match self.leader_node_id.lock() {
            Ok(l) => l,
            Err(_) => return (*term, false),
        };

        if candidate_term < *term {
            return (*term, false);
        }

        if candidate_term > *term {
            *term = candidate_term;
            *voted = None;
            *role = NodeRole::Worker;
            *leader = None;
        }

        let grant = match voted.as_deref() {
            None => true,
            Some(v) if v == candidate_id => true,
            _ => false,
        };

        if grant {
            *voted = Some(candidate_id.to_string());
        }

        (*term, grant)
    }

    pub fn elect(
        &self,
        local_node_id: &str,
        candidate_node_id: Option<&str>,
        requested_term: Option<u64>,
        _force: bool,
    ) -> Result<(String, u64, NodeRole), String> {
        if self.current_role.is_poisoned()
            || self.leader_node_id.is_poisoned()
            || self.current_term.is_poisoned()
            || self.voted_for.is_poisoned()
            || self.votes_received.is_poisoned()
        {
            return Err(
                "InternalSecurityError: Swarm governance mutex is poisoned; state mutations rejected"
                    .to_string(),
            );
        }

        let term_to_request = requested_term.unwrap_or_else(|| self.term());
        let target_candidate = candidate_node_id.unwrap_or(local_node_id);
        let (granted_term, granted) = self.request_vote(term_to_request, target_candidate);
        if granted {
            if target_candidate == local_node_id {
                let mut role = self.current_role.lock().map_err(|_| {
                    "InternalSecurityError: current_role mutex is poisoned".to_string()
                })?;
                *role = NodeRole::Leader;
                let mut leader = self.leader_node_id.lock().map_err(|_| {
                    "InternalSecurityError: leader_node_id mutex is poisoned".to_string()
                })?;
                *leader = Some(local_node_id.to_string());
            }
            let role = self.role();
            let leader = self
                .leader_id()
                .unwrap_or_else(|| target_candidate.to_string());
            Ok((leader, granted_term, role))
        } else {
            Err(
                "Unauthorized: Leader is already elected. Unilateral re-election requires swarm consensus."
                    .to_string(),
            )
        }
    }

    #[cfg(test)]
    pub fn reset_for_testing(&self) {
        if let Ok(mut current_leader) = self.leader_node_id.lock() {
            *current_leader = None;
        }
        if let Ok(mut voted) = self.voted_for.lock() {
            *voted = None;
        }
        if let Ok(mut role) = self.current_role.lock() {
            *role = NodeRole::Worker;
        }
        if let Ok(mut term) = self.current_term.lock() {
            *term = 1;
        }
        if let Ok(mut votes) = self.votes_received.lock() {
            votes.clear();
        }
    }
}

pub fn start_raft_governance_worker(
    server: Arc<super::super::RpcServer>,
    shutdown_signal: Arc<AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut seed: u64 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;

        let mut pseudo_rand = move |modulus: u64| -> u64 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            if modulus == 0 {
                0
            } else {
                (seed >> 16) % modulus
            }
        };

        while !shutdown_signal.load(std::sync::atomic::Ordering::Relaxed) {
            let role = server.swarm_governance.role();
            match role {
                NodeRole::Leader => {
                    let term = server.swarm_governance.term();
                    let active_peers: Vec<(String, String)> = {
                        let peers = server.peers.lock().unwrap_or_else(|e| e.into_inner());
                        peers
                            .values()
                            .filter(|p| p.status == "Active")
                            .map(|p| (p.node_id.clone(), p.address.clone()))
                            .collect()
                    };

                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();

                    let is_legacy_hmac = !server.is_zero_trust()
                        && !server.node_id.starts_with("knc-")
                        && server.mesh_auth_token.is_some();

                    let (sender_id, pubkey_hex_opt, sig_hex_opt, token_opt) = if is_legacy_hmac {
                        (
                            server.node_id.clone(),
                            None,
                            None,
                            server.mesh_auth_token.clone(),
                        )
                    } else {
                        let kp = server
                            .ed25519_keypair
                            .lock()
                            .unwrap_or_else(|e| e.into_inner());
                        let canonical_id = kp.node_id();
                        let canonical_msg =
                            format!("{}:{}:{}:{}", canonical_id, term, canonical_id, now);
                        let sig = kp.sign_hex(canonical_msg.as_bytes());
                        let pk = kp.public_key_hex();
                        (canonical_id, Some(pk), Some(sig), None)
                    };

                    for (_peer_id, peer_addr) in active_peers {
                        let mut params = serde_json::json!({
                            "term": term,
                            "leader_id": sender_id,
                            "sender_node_id": sender_id,
                            "timestamp": now,
                        });
                        if let (Some(pk), Some(sig)) = (&pubkey_hex_opt, &sig_hex_opt) {
                            params["public_key"] = serde_json::json!(pk);
                            params["signature"] = serde_json::json!(sig);
                        }
                        if let Some(tok) = &token_opt {
                            params["mesh_auth_token"] = serde_json::json!(tok);
                        }
                        let req = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 9999,
                            "method": "knc_swarm_heartbeat",
                            "params": params
                        });
                        let _ = server.dispatch_request_over_network(&peer_addr, &req.to_string());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                _ => {
                    let elapsed = server.swarm_governance.last_heartbeat_elapsed_ms();
                    let randomized_timeout = 300 + pseudo_rand(200);

                    if elapsed > randomized_timeout && server.swarm_governance.leader_id().is_some()
                    {
                        {
                            let mut leader = server
                                .swarm_governance
                                .leader_node_id
                                .lock()
                                .unwrap_or_else(|e| e.into_inner());
                            *leader = None;
                        }

                        let elect_req = serde_json::json!({
                            "jsonrpc": "2.0",
                            "id": 8888,
                            "method": "knc_swarm_elect",
                            "params": {
                                "mesh_auth_token": server.mesh_auth_token
                            }
                        });
                        let _ = server.dispatch_request(&elect_req.to_string());
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }
            }
        }
    })
}

impl super::super::RpcServer {
    pub fn handle_swarm_elect(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let requested_term = params.get("term").and_then(|v| v.as_u64());

        let candidate_id = match params.get("candidate_node_id").and_then(|v| v.as_str()) {
            Some(c) => match validate_param_string_len(c) {
                Ok(_) => Some(c),
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => None,
        };

        let is_zt = self.is_zero_trust()
            || params
                .get("zero_trust")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            || params.get("zero_trust_envelope").is_some();

        let requested_force = params
            .get("force")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_zt && requested_force {
            return JsonRpcResponse::error(
                id,
                -32001,
                "Unauthorized: Forced self-election ('force: true') is prohibited in Zero-Trust mode. Swarm consensus is required.",
            );
        }

        let force = if is_zt { false } else { requested_force };

        if candidate_id.is_none() && requested_term.is_none() && !force {
            let active_peers: Vec<(String, String)> = {
                let peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
                peers
                    .values()
                    .filter(|p| p.status == "Active")
                    .map(|p| (p.node_id.clone(), p.address.clone()))
                    .collect()
            };

            let current_term = self.swarm_governance.term();
            let election_term = current_term + 1;
            {
                let mut term_guard = self
                    .swarm_governance
                    .current_term
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                *term_guard = election_term;

                let mut role_guard = self
                    .swarm_governance
                    .current_role
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                *role_guard = NodeRole::Candidate;

                let mut voted_guard = self
                    .swarm_governance
                    .voted_for
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                *voted_guard = Some(self.node_id.clone());

                let mut votes_guard = self
                    .swarm_governance
                    .votes_received
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                votes_guard.clear();
                votes_guard.insert(self.node_id.clone());
            }

            let mut votes_count = 1usize;
            for (_peer_id, peer_addr) in &active_peers {
                let vote_req = serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "knc_swarm_request_vote",
                    "params": {
                        "term": election_term,
                        "candidate_id": self.node_id,
                        "mesh_auth_token": self.mesh_auth_token
                    }
                });

                if let Ok(resp_str) =
                    self.dispatch_request_over_network(peer_addr, &vote_req.to_string())
                    && let Ok(resp_val) = serde_json::from_str::<Value>(&resp_str)
                    && resp_val
                        .get("result")
                        .and_then(|r| r.get("vote_granted"))
                        .and_then(|v| v.as_bool())
                        == Some(true)
                {
                    votes_count += 1;
                }
            }

            let active_nodes = 1 + active_peers.len();
            let quorum_threshold = (active_nodes / 2) + 1;

            if votes_count >= quorum_threshold {
                self.swarm_governance.set_role(NodeRole::Leader);
                let mut leader_guard = self
                    .swarm_governance
                    .leader_node_id
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                *leader_guard = Some(self.node_id.clone());

                let resp_val = serde_json::json!({
                    "status": "ok",
                    "leader_id": self.node_id,
                    "leader_node_id": self.node_id,
                    "term": election_term,
                    "role": NodeRole::Leader,
                    "votes_count": votes_count,
                    "votes_granted": votes_count,
                    "active_nodes": active_nodes,
                    "quorum_reached": true
                });
                return JsonRpcResponse::success(id, resp_val);
            } else {
                self.swarm_governance.set_role(NodeRole::Worker);
                let mut seed = election_term.wrapping_mul(1000003);
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let backoff_ms = 150 + ((seed >> 16) % 150);
                std::thread::sleep(std::time::Duration::from_millis(backoff_ms));

                return JsonRpcResponse::error(
                    id,
                    -32001,
                    format!(
                        "Quorum consensus not reached for election: votes {} < threshold {}",
                        votes_count, quorum_threshold
                    ),
                );
            }
        }

        match self
            .swarm_governance
            .elect(&self.node_id, candidate_id, requested_term, force)
        {
            Ok((leader, term, role)) => {
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "leader_id": leader,
                    "leader_node_id": leader,
                    "term": term,
                    "role": role
                });
                JsonRpcResponse::success(id, resp_val)
            }
            Err(err) => JsonRpcResponse::error(id, -32001, err),
        }
    }

    pub fn handle_swarm_roles(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let local_role = self.swarm_governance.role();
        let peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());

        let mut roles_map = std::collections::HashMap::new();
        roles_map.insert(self.node_id.clone(), format!("{:?}", local_role));

        for (peer_id, peer) in peers.iter() {
            let role_str = if peer.capabilities.contains(&"storage".to_string()) {
                "Storage"
            } else if peer.status == "Stale" {
                "Observer"
            } else {
                "Worker"
            };
            roles_map.insert(peer_id.clone(), role_str.to_string());
        }

        let resp_val = serde_json::json!({
            "status": "ok",
            "local_node_id": self.node_id,
            "local_role": format!("{:?}", local_role),
            "roles": roles_map
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_swarm_quorum(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let operation = params
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("cluster_op")
            .to_string();

        let active_nodes = {
            let peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            let active_peers_count = peers.values().filter(|p| p.status == "Active").count();
            1 + active_peers_count
        };
        let server_threshold = (active_nodes / 2) + 1;

        let requested_quorum = params
            .get("required_quorum")
            .and_then(|v| v.as_u64())
            .map(|q| q as usize);

        let quorum_threshold = match requested_quorum {
            Some(req_q) => std::cmp::max(server_threshold, req_q),
            None => server_threshold,
        };

        let quorum_reached = active_nodes >= quorum_threshold;

        let resp_val = serde_json::json!({
            "status": "ok",
            "operation": operation,
            "quorum_reached": quorum_reached,
            "active_nodes": active_nodes,
            "quorum_threshold": quorum_threshold
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_swarm_request_vote(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let candidate_term = match params.get("term").and_then(|v| v.as_u64()) {
            Some(t) => t,
            None => return JsonRpcResponse::error(id, -32602, "Missing 'term' parameter"),
        };

        let candidate_id = match params.get("candidate_id").and_then(|v| v.as_str()) {
            Some(c) => match validate_param_string_len(c) {
                Ok(_) => c,
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => return JsonRpcResponse::error(id, -32602, "Missing 'candidate_id' parameter"),
        };

        let (current_term, vote_granted) = self
            .swarm_governance
            .request_vote(candidate_term, candidate_id);

        let resp_val = serde_json::json!({
            "status": "ok",
            "term": current_term,
            "vote_granted": vote_granted,
            "node_id": self.node_id
        });

        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_swarm_heartbeat(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let pubkey_str = params
            .get("zero_trust_envelope")
            .and_then(|e| e.get("public_key"))
            .or_else(|| params.get("public_key"))
            .and_then(|v| v.as_str());

        let sig_str = params
            .get("zero_trust_envelope")
            .and_then(|e| e.get("signature").or_else(|| e.get("ed25519_signature")))
            .or_else(|| params.get("signature"))
            .or_else(|| params.get("ed25519_signature"))
            .and_then(|v| v.as_str());

        let is_signed = pubkey_str.is_some() && sig_str.is_some();

        // STRICT ANTI-DOWNGRADE INVARIANT:
        // If the server is in Zero-Trust mode, incoming requests with plaintext HMAC tokens or
        // client-side backward_compat flags MUST be rejected immediately.
        if self.is_zero_trust() && !is_signed {
            return JsonRpcResponse::error(
                id,
                -32001,
                "Unauthorized: Unsigned payload or legacy HMAC token rejected in zero-trust mode",
            );
        }

        let leader_term = match params.get("term").and_then(|v| v.as_u64()) {
            Some(t) => t,
            None => return JsonRpcResponse::error(id, -32602, "Missing 'term' parameter"),
        };

        let leader_id = match params.get("leader_id").and_then(|v| v.as_str()) {
            Some(l) => match validate_param_string_len(l) {
                Ok(_) => l,
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => return JsonRpcResponse::error(id, -32602, "Missing 'leader_id' parameter"),
        };

        let sender_node_id = params
            .get("sender_node_id")
            .and_then(|v| v.as_str())
            .unwrap_or(leader_id);

        if let (Some(pubkey_hex), Some(sig_hex)) = (pubkey_str, sig_str) {
            let normalized_pubkey = pubkey_hex.trim().to_lowercase();
            if self.is_peer_key_revoked(&normalized_pubkey)
                || self.is_peer_key_revoked(sender_node_id)
                || self.is_peer_key_revoked(leader_id)
            {
                return JsonRpcResponse::error(
                    id,
                    -32001,
                    "Unauthorized: Leader or sender public key has been revoked",
                );
            }

            let ts = params
                .get("zero_trust_envelope")
                .and_then(|e| e.get("timestamp"))
                .or_else(|| params.get("timestamp"))
                .and_then(|v| v.as_u64());

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            if let Some(ts_val) = ts {
                let ts_secs = if ts_val > 10_000_000_000 {
                    ts_val / 1000
                } else {
                    ts_val
                };
                if now.abs_diff(ts_secs) > crate::rpc::types::MAX_ZERO_TRUST_WINDOW_SECS {
                    return JsonRpcResponse::error(
                        id,
                        -32001,
                        "Unauthorized: Request timestamp expired or invalid (replay protection window: 30s)",
                    );
                }

                // Canonical message: sender_node_id:term:leader_id:timestamp
                let canonical_msg = format!(
                    "{}:{}:{}:{}",
                    sender_node_id, leader_term, leader_id, ts_secs
                );
                let alt_msg = format!(
                    "{}:{}:{}",
                    ts_secs,
                    params
                        .get("nonce")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default(),
                    sender_node_id
                );

                let pubkey = match crate::crypto_ed25519::Ed25519PublicKey::from_hex(pubkey_hex) {
                    Ok(pk) => pk,
                    Err(e) => {
                        return JsonRpcResponse::error(
                            id,
                            -32001,
                            format!("Unauthorized: Bad public key: {}", e),
                        );
                    }
                };

                let expected_node_id = pubkey.node_id();
                if sender_node_id != expected_node_id {
                    return JsonRpcResponse::error(
                        id,
                        -32001,
                        format!(
                            "Unauthorized: Sender node ID does not match canonical derived public key identity (expected: '{}', claimed: '{}')",
                            expected_node_id, sender_node_id
                        ),
                    );
                }

                if !pubkey.verify_hex(canonical_msg.as_bytes(), sig_hex)
                    && !pubkey.verify_hex(alt_msg.as_bytes(), sig_hex)
                {
                    return JsonRpcResponse::error(
                        id,
                        -32001,
                        "Unauthorized: Invalid Ed25519 signature on Raft heartbeat",
                    );
                }

                let mut verified = match self.verified_peer_keys.lock() {
                    Ok(v) => v,
                    Err(_) => {
                        return JsonRpcResponse::error(
                            id,
                            -32000,
                            "InternalSecurityError: verified_peer_keys mutex is poisoned",
                        );
                    }
                };
                verified.insert(expected_node_id, normalized_pubkey);
            } else {
                return JsonRpcResponse::error(
                    id,
                    -32001,
                    "Unauthorized: Missing timestamp in heartbeat zero-trust envelope",
                );
            }
        } else {
            // Legacy HMAC check when server is NOT in zero-trust mode
            if let Err(err) = self.check_mesh_auth(&params) {
                return JsonRpcResponse::error(id, -32001, err);
            }
            if sender_node_id.starts_with("knc-") || leader_id.starts_with("knc-") {
                return JsonRpcResponse::error(
                    id,
                    -32001,
                    "Unauthorized: Legacy-HMAC requests cannot claim canonical self-certifying 'knc-*' identity",
                );
            }
        }

        let (current_term, success) = self
            .swarm_governance
            .process_heartbeat(leader_term, leader_id);

        let resp_val = serde_json::json!({
            "status": "ok",
            "success": success,
            "term": current_term,
            "node_id": self.node_id
        });

        JsonRpcResponse::success(id, resp_val)
    }
}
