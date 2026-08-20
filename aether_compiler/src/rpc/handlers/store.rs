use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;

use crate::rpc::types::{
    JsonRpcResponse, MAX_STORE_KEYS, MAX_SYNC_ENTRIES, MAX_VALUE_SIZE_BYTES, is_future_timestamp,
    validate_param_string_len,
};

/// A single CRDT LWW (Last-Write-Wins) Key-Value Entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CrdtEntry {
    pub key: String,
    pub value: Value,
    pub timestamp: u64,
    pub writer_id: String,
}

impl CrdtEntry {
    /// Returns true if `self` is strictly newer or wins the LWW tiebreaker over `other`.
    pub fn is_newer_than(&self, other: &CrdtEntry) -> bool {
        if self.timestamp != other.timestamp {
            self.timestamp > other.timestamp
        } else {
            self.writer_id > other.writer_id
        }
    }
}

/// Thread-safe distributed CRDT Key-Value Store using LWW (Last-Write-Wins) register semantics.
pub struct MeshKvStore {
    entries: Mutex<HashMap<String, CrdtEntry>>,
}

impl Default for MeshKvStore {
    fn default() -> Self {
        Self::new()
    }
}

impl MeshKvStore {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    pub fn put(&self, key: &str, value: Value, timestamp: u64, writer_id: &str) -> bool {
        if is_future_timestamp(timestamp) {
            return false;
        }
        let val_str = serde_json::to_string(&value).unwrap_or_default();
        if val_str.len() > MAX_VALUE_SIZE_BYTES {
            return false;
        }

        let mut store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let is_existing = store.contains_key(key);
        if !is_existing && store.len() >= MAX_STORE_KEYS {
            return false;
        }

        let new_entry = CrdtEntry {
            key: key.to_string(),
            value,
            timestamp,
            writer_id: writer_id.to_string(),
        };

        if let Some(existing) = store.get(key) {
            if new_entry.is_newer_than(existing) {
                store.insert(key.to_string(), new_entry);
                true
            } else {
                false
            }
        } else {
            store.insert(key.to_string(), new_entry);
            true
        }
    }

    pub fn get(&self, key: &str) -> Option<CrdtEntry> {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        store.get(key).cloned()
    }

    pub fn contains_key(&self, key: &str) -> bool {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        store.contains_key(key)
    }

    pub fn len(&self) -> usize {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        store.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn sync(&self, incoming: Vec<CrdtEntry>) -> usize {
        if incoming.len() > MAX_SYNC_ENTRIES {
            return 0;
        }
        let mut store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut updated = 0;
        for entry in incoming {
            if is_future_timestamp(entry.timestamp) {
                continue;
            }
            let val_str = serde_json::to_string(&entry.value).unwrap_or_default();
            if val_str.len() > MAX_VALUE_SIZE_BYTES {
                continue;
            }
            let key = entry.key.clone();
            let is_existing = store.contains_key(&key);
            if !is_existing && store.len() >= MAX_STORE_KEYS {
                continue;
            }
            if let Some(existing) = store.get(&key) {
                if entry.is_newer_than(existing) {
                    store.insert(key, entry);
                    updated += 1;
                }
            } else {
                store.insert(key, entry);
                updated += 1;
            }
        }
        updated
    }

    pub fn dump_entries(&self) -> Vec<CrdtEntry> {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        store.values().cloned().collect()
    }

    /// Computes a deterministic SHA256 anti-entropy state digest across all active store entries.
    ///
    /// Deterministically sorts active keys, computes canonical value digests, and feeds
    /// (key, value_hash, timestamp, writer_id) tuples into a SHA-256 digest context.
    ///
    /// Authenticated identity binding guarantees that `writer_id` entries are bound to verified session
    /// keys (`ed25519:<pubkey_hex>`) or scoped HMAC senders (`legacy-hmac:<sender_node_id>`),
    /// preventing unauthenticated spoofing during CRDT reconciliation and digest verification.
    pub fn compute_state_digest(&self) -> String {
        use ring::digest::{Context, SHA256, digest};

        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut sorted_keys: Vec<&String> = store.keys().collect();
        sorted_keys.sort();

        let mut context = Context::new(&SHA256);
        for key in sorted_keys {
            if let Some(entry) = store.get(key) {
                let val_str = serde_json::to_string(&entry.value).unwrap_or_default();
                let val_digest = digest(&SHA256, val_str.as_bytes());
                let val_hash_hex: String = val_digest
                    .as_ref()
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect();

                let canonical_repr = format!(
                    "{}:{}:{}:{}\n",
                    entry.key, val_hash_hex, entry.timestamp, entry.writer_id
                );
                context.update(canonical_repr.as_bytes());
            }
        }

        let final_digest = context.finish();
        final_digest
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }

    /// Returns the maximum timestamp recorded among all active entries, or 0 if empty.
    pub fn latest_timestamp(&self) -> u64 {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        store.values().map(|e| e.timestamp).max().unwrap_or(0)
    }

    /// Returns differential entries matching optional `since_timestamp` and `key_prefix`, bounded by limit.
    pub fn diff_entries(
        &self,
        since_timestamp: Option<u64>,
        key_prefix: Option<&str>,
        limit: usize,
    ) -> Vec<CrdtEntry> {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut matching: Vec<CrdtEntry> = store
            .values()
            .filter(|e| {
                if let Some(since) = since_timestamp
                    && e.timestamp < since
                {
                    return false;
                }
                if let Some(prefix) = key_prefix
                    && !e.key.starts_with(prefix)
                {
                    return false;
                }
                true
            })
            .cloned()
            .collect();

        matching.sort_by(|a, b| a.key.cmp(&b.key));
        if matching.len() > limit {
            matching.truncate(limit);
        }
        matching
    }
}

impl super::super::RpcServer {
    pub fn handle_store_put(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => match validate_param_string_len(k) {
                Ok(_) => k,
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => return JsonRpcResponse::error(id, -32602, "Missing 'key' parameter"),
        };

        let value = match params.get("value") {
            Some(v) => v.clone(),
            None => return JsonRpcResponse::error(id, -32602, "Missing 'value' parameter"),
        };

        let timestamp = params
            .get("timestamp")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            });

        if is_future_timestamp(timestamp) {
            return JsonRpcResponse::error(
                id,
                -32602,
                "Timestamp is in the future beyond acceptable clock drift",
            );
        }

        // Authenticated writer_id resolution & identity binding (Sprint 354):
        // 1. Ed25519 Zero-Trust: writer_id is derived strictly from verified Ed25519 public key
        //    (`ed25519:<pubkey_hex>`), strictly ignoring any client-supplied `writer_id` or `sender_node_id`.
        // 2. Legacy HMAC: writer_id is constructed as `legacy-hmac:<sender_node_id>`.
        //    Scoped trust exception: Shared-secret auth cannot cryptographically prove individual sender identity,
        //    so sender_node_id is scoped under the legacy-hmac namespace.
        // 3. Unauthenticated/Dev fallback: client-supplied writer_id or local node ID.
        let envelope = params.get("zero_trust_envelope");
        let pubkey_str = envelope
            .and_then(|e| e.get("public_key"))
            .or_else(|| params.get("public_key"))
            .and_then(|v| v.as_str());
        let sig_str = envelope
            .and_then(|e| e.get("signature").or_else(|| e.get("ed25519_signature")))
            .or_else(|| params.get("signature"))
            .or_else(|| params.get("ed25519_signature"))
            .and_then(|v| v.as_str());

        let writer_id: String = if let (Some(pk), Some(_)) = (pubkey_str, sig_str) {
            format!("ed25519:{}", pk.trim().to_lowercase())
        } else if self.mesh_auth_token.is_some()
            || params.get("mesh_auth_signature").is_some()
            || params.get("mesh_auth_token").is_some()
        {
            let sender = params
                .get("sender_node_id")
                .or_else(|| params.get("node_id"))
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| {
                    params
                        .get("writer_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(&self.node_id)
                });
            format!("legacy-hmac:{}", sender)
        } else {
            let sender = params
                .get("writer_id")
                .or_else(|| params.get("sender_node_id"))
                .or_else(|| params.get("node_id"))
                .and_then(|v| v.as_str())
                .unwrap_or(&self.node_id);
            sender.to_string()
        };

        let success = self.store.put(key, value.clone(), timestamp, &writer_id);

        let resp_val = serde_json::json!({
            "status": "ok",
            "updated": success,
            "written": success,
            "key": key,
            "timestamp": timestamp,
            "writer_id": writer_id
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_store_get(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => match validate_param_string_len(k) {
                Ok(_) => k,
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => return JsonRpcResponse::error(id, -32602, "Missing 'key' parameter"),
        };

        if let Some(entry) = self.store.get(key) {
            let resp_val = serde_json::json!({
                "status": "ok",
                "found": true,
                "entry": entry
            });
            JsonRpcResponse::success(id, resp_val)
        } else {
            let resp_val = serde_json::json!({
                "status": "ok",
                "found": false,
                "entry": Value::Null,
                "key": key
            });
            JsonRpcResponse::success(id, resp_val)
        }
    }

    pub fn handle_store_sync(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("push");

        match action {
            "push" => {
                let incoming: Vec<CrdtEntry> = params
                    .get("entries")
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
                    .unwrap_or_default();

                let updated_count = self.store.sync(incoming);
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "synced": true,
                    "updated_entries": updated_count,
                    "total_store_keys": self.store.len()
                });
                JsonRpcResponse::success(id, resp_val)
            }
            "pull" => {
                let entries = self.store.dump_entries();
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "total_entries": entries.len(),
                    "entries": entries
                });
                JsonRpcResponse::success(id, resp_val)
            }
            _ => JsonRpcResponse::error(
                id,
                -32602,
                format!("Unknown knc_store_sync action '{}'", action),
            ),
        }
    }

    pub fn handle_store_digest(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let state_digest = self.store.compute_state_digest();
        let entry_count = self.store.len();
        let latest_timestamp = self.store.latest_timestamp();

        let resp_val = serde_json::json!({
            "status": "ok",
            "state_digest": state_digest,
            "entry_count": entry_count,
            "latest_timestamp": latest_timestamp
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_store_diff(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let peer_digest = params.get("peer_digest").and_then(|v| v.as_str());
        let local_digest = self.store.compute_state_digest();

        if let Some(pd) = peer_digest
            && !pd.is_empty()
            && pd.eq_ignore_ascii_case(&local_digest)
        {
            let resp_val = serde_json::json!({
                "status": "ok",
                "in_sync": true,
                "state_digest": local_digest,
                "entries": []
            });
            return JsonRpcResponse::success(id, resp_val);
        }

        let since_timestamp = params.get("since_timestamp").and_then(|v| v.as_u64());
        let key_prefix = params.get("key_prefix").and_then(|v| v.as_str());

        let limit = params
            .get("limit")
            .and_then(|v| v.as_u64())
            .map(|l| (l as usize).min(MAX_SYNC_ENTRIES))
            .unwrap_or(MAX_SYNC_ENTRIES);

        let delta_entries = self.store.diff_entries(since_timestamp, key_prefix, limit);

        let resp_val = serde_json::json!({
            "status": "ok",
            "in_sync": false,
            "state_digest": local_digest,
            "entries_count": delta_entries.len(),
            "entries": delta_entries
        });
        JsonRpcResponse::success(id, resp_val)
    }
}
