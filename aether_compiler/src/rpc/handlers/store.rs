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

        let writer_id = params
            .get("writer_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.node_id);

        let success = self.store.put(key, value.clone(), timestamp, writer_id);

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
}
