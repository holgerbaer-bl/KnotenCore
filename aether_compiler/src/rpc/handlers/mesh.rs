use serde_json::Value;
use std::sync::Arc;

use crate::rpc::types::{JsonRpcResponse, KNC_PROTOCOL_VERSION, MeshPeer};

#[derive(Debug, Clone)]
pub struct MeshGossipConfig {
    pub gossip_interval_secs: u64,
    pub stale_timeout_secs: u64,
    pub eviction_timeout_secs: u64,
    pub ping_timeout_ms: u64,
}

impl Default for MeshGossipConfig {
    fn default() -> Self {
        Self {
            gossip_interval_secs: 5,
            stale_timeout_secs: 60,
            eviction_timeout_secs: 180,
            ping_timeout_ms: 1000,
        }
    }
}

pub struct MeshGossipWorker {
    pub server: Arc<super::super::RpcServer>,
    pub config: MeshGossipConfig,
}

impl MeshGossipWorker {
    pub fn new(server: Arc<super::super::RpcServer>, config: MeshGossipConfig) -> Self {
        Self { server, config }
    }

    pub fn evaluate_timeouts(&self, current_timestamp: u64) -> (usize, usize, usize) {
        let mut peers = self.server.peers.lock().unwrap_or_else(|e| e.into_inner());
        let mut active = 0;
        let mut stale = 0;
        let mut evicted = 0;

        for peer in peers.values_mut() {
            let elapsed = current_timestamp.saturating_sub(peer.last_seen);
            if elapsed >= self.config.eviction_timeout_secs {
                peer.status = "Evicted".to_string();
                evicted += 1;
            } else if elapsed >= self.config.stale_timeout_secs {
                peer.status = "Stale".to_string();
                stale += 1;
            } else {
                peer.status = "Active".to_string();
                active += 1;
            }
        }
        (active, stale, evicted)
    }

    pub fn prune_evicted(&self) -> usize {
        let mut peers = self.server.peers.lock().unwrap_or_else(|e| e.into_inner());
        let initial = peers.len();
        peers.retain(|_, peer| peer.status != "Evicted");
        initial - peers.len()
    }
}

impl super::super::RpcServer {
    pub fn handle_mesh_discover(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let resp_val = serde_json::json!({
            "status": "ok",
            "protocol_version": KNC_PROTOCOL_VERSION,
            "node_id": self.node_id,
            "address": self.node_address,
            "capabilities": ["mesh_discover", "mesh_peers", "mesh_ping", "mesh_gossip", "agent_teleport", "mesh_metrics", "task_queue", "crdt_store"],
            "auth_required": self.mesh_auth_token.is_some()
        });

        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_mesh_peers(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("list");

        match action {
            "register" => {
                let peer_val = match params.get("peer") {
                    Some(v) => v,
                    None => {
                        return JsonRpcResponse::error(
                            id,
                            -32602,
                            "Missing 'peer' object in register payload",
                        );
                    }
                };

                let mut peer: MeshPeer = match serde_json::from_value(peer_val.clone()) {
                    Ok(p) => p,
                    Err(e) => {
                        return JsonRpcResponse::error(
                            id,
                            -32602,
                            format!("Invalid MeshPeer payload: {}", e),
                        );
                    }
                };

                let is_revoked = self.is_peer_key_revoked(&peer.node_id)
                    || peer
                        .capabilities
                        .iter()
                        .any(|cap| self.is_peer_key_revoked(cap))
                    || self
                        .verified_peer_keys
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .get(&peer.node_id)
                        .is_some_and(|pk| self.is_peer_key_revoked(pk));

                if is_revoked {
                    return JsonRpcResponse::error(id, -32001, "Unauthorized: Peer key is revoked");
                }

                peer.last_seen = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
                peers.insert(peer.node_id.clone(), peer.clone());

                let resp_val = serde_json::json!({
                    "status": "ok",
                    "registered_peer": peer,
                    "total_peers": peers.len()
                });
                JsonRpcResponse::success(id, resp_val)
            }
            "gossip" => {
                let incoming_peers: Vec<MeshPeer> = params
                    .get("peers")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let mut added = 0;
                let mut updated = 0;

                for mut peer in incoming_peers {
                    if peer.node_id == self.node_id {
                        continue;
                    }
                    if let Some(existing) = peers.get_mut(&peer.node_id) {
                        existing.last_seen = now;
                        existing.status = "Active".to_string();
                        updated += 1;
                    } else {
                        peer.last_seen = now;
                        peer.status = "Active".to_string();
                        peers.insert(peer.node_id.clone(), peer);
                        added += 1;
                    }
                }

                let peer_list: Vec<MeshPeer> = peers.values().cloned().collect();

                let resp_val = serde_json::json!({
                    "status": "ok",
                    "gossip_summary": {
                        "added": added,
                        "updated": updated,
                        "total_known_peers": peers.len()
                    },
                    "peers": peer_list
                });
                JsonRpcResponse::success(id, resp_val)
            }
            "list" => {
                let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                const STALE_TIMEOUT_SECS: u64 = 60;
                const EVICT_TIMEOUT_SECS: u64 = 180;

                let mut to_evict = Vec::new();
                for (id_key, peer) in peers.iter_mut() {
                    let elapsed = now.saturating_sub(peer.last_seen);
                    if elapsed > EVICT_TIMEOUT_SECS {
                        to_evict.push(id_key.clone());
                    } else if elapsed > STALE_TIMEOUT_SECS {
                        peer.status = "Stale".to_string();
                    }
                }

                for evict_id in to_evict {
                    peers.remove(&evict_id);
                }

                let peer_list: Vec<MeshPeer> = peers.values().cloned().collect();
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "total_peers": peer_list.len(),
                    "peers": peer_list
                });
                JsonRpcResponse::success(id, resp_val)
            }
            "prune" => {
                let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
                let initial = peers.len();
                peers.retain(|_, peer| peer.status != "Evicted");
                let pruned = initial - peers.len();
                let peer_list: Vec<MeshPeer> = peers.values().cloned().collect();

                let resp_val = serde_json::json!({
                    "status": "ok",
                    "pruned_count": pruned,
                    "remaining_peers": peers.len(),
                    "peers": peer_list
                });
                JsonRpcResponse::success(id, resp_val)
            }
            _ => JsonRpcResponse::error(
                id,
                -32602,
                format!("Unknown knc_mesh_peers action '{}'", action),
            ),
        }
    }

    pub fn handle_mesh_ping(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let sender_node_id = params
            .get("sender_node_id")
            .or_else(|| params.get("node_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let sender_address = params
            .get("sender_address")
            .or_else(|| params.get("address"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let latency_ms = params
            .get("latency_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if !sender_node_id.is_empty() {
            let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(peer) = peers.get_mut(sender_node_id) {
                peer.last_seen = now;
                peer.status = "Active".to_string();
                if latency_ms > 0 {
                    peer.latency_ms = latency_ms;
                }
            } else if !sender_address.is_empty() {
                peers.insert(
                    sender_node_id.to_string(),
                    MeshPeer {
                        node_id: sender_node_id.to_string(),
                        address: sender_address.to_string(),
                        capabilities: vec!["mesh_ping".to_string()],
                        last_seen: now,
                        latency_ms,
                        status: "Active".to_string(),
                    },
                );
            }
        }

        let resp_val = serde_json::json!({
            "status": "ok",
            "pong": true,
            "responder_node_id": self.node_id,
            "responder_address": self.node_address,
            "timestamp": now
        });

        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_mesh_metrics(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let metrics = self.metrics_collector.collect(self.task_dispatcher.stats());
        let resp_val = serde_json::json!({
            "status": "ok",
            "node_id": self.node_id,
            "protocol_version": KNC_PROTOCOL_VERSION,
            "metrics": metrics,
            "capabilities": {
                "adaptive_work_stealing": true,
                "load_balancing": true
            }
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_mesh_rotate_key(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let (old_key, new_key) = self.rotate_key();
        let resp_val = serde_json::json!({
            "status": "ok",
            "node_id": self.node_id,
            "old_public_key": old_key,
            "previous_public_key": old_key,
            "new_public_key": new_key
        });

        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_mesh_revoke_peer(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let peer_pubkey = params
            .get("peer_pubkey")
            .or_else(|| params.get("peer_public_key"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim()
            .to_lowercase();

        if peer_pubkey.is_empty() {
            return JsonRpcResponse::error(id, -32602, "Missing parameter 'peer_pubkey'");
        }

        let (active_nodes, quorum_threshold) = {
            let peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            let active_peers_count = peers.values().filter(|p| p.status == "Active").count();
            let active = 1 + active_peers_count;
            let server_threshold = (active / 2) + 1;
            let requested_quorum = params
                .get("required_quorum")
                .and_then(|v| v.as_u64())
                .map(|q| q as usize);
            let threshold = match requested_quorum {
                Some(req_q) => std::cmp::max(server_threshold, req_q),
                None => server_threshold,
            };
            (active, threshold)
        };
        let quorum_reached = active_nodes >= quorum_threshold;

        if !quorum_reached {
            return JsonRpcResponse::error(
                id,
                -32001,
                format!(
                    "Quorum consensus not reached for peer revocation: active nodes {} < threshold {}",
                    active_nodes, quorum_threshold
                ),
            );
        }

        self.revoke_peer_key(&peer_pubkey);

        let resp_val = serde_json::json!({
            "status": "ok",
            "revoked": true,
            "quorum_reached": quorum_reached,
            "active_nodes": active_nodes,
            "quorum_threshold": quorum_threshold,
            "revoked_peer_key": peer_pubkey,
            "node_id": self.node_id
        });

        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_mesh_verify_peer(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let peer_id = params
            .get("peer_node_id")
            .or_else(|| params.get("sender_node_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let peer_pubkey = params
            .get("public_key")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if peer_id.is_empty() || peer_pubkey.is_empty() {
            return JsonRpcResponse::error(
                id,
                -32602,
                "Missing 'peer_node_id' or 'public_key' parameter",
            );
        }

        if self.is_peer_key_revoked(peer_pubkey) {
            return JsonRpcResponse::error(
                id,
                -32001,
                "Unauthorized: Peer public key has been revoked",
            );
        }

        let mut verified_keys = self
            .verified_peer_keys
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        verified_keys.insert(peer_id.to_string(), peer_pubkey.to_string());

        let resp_val = serde_json::json!({
            "status": "ok",
            "verified": true,
            "peer_node_id": peer_id,
            "peer_public_key": peer_pubkey,
            "local_node_id": self.node_id,
            "local_public_key": self.public_key_hex()
        });

        JsonRpcResponse::success(id, resp_val)
    }
}
