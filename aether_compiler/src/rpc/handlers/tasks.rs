use knoten_core_types::ast::Node;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use crate::crypto_ed25519::{Ed25519KeyPair, Ed25519PublicKey};
use crate::rpc::types::{JsonRpcResponse, MAX_TASK_QUEUE_DEPTH, validate_param_string_len};

pub const MAX_PER_PEER_TASK_RATE: usize = 50;

/// Per-peer sliding window task submission rate limiter.
#[derive(Debug, Default)]
pub struct PeerRateLimiter {
    counts: Mutex<HashMap<String, Vec<u64>>>,
}

impl PeerRateLimiter {
    pub fn new() -> Self {
        Self {
            counts: Mutex::new(HashMap::new()),
        }
    }

    pub fn check_and_record(
        &self,
        peer_id: &str,
        window_secs: u64,
        max_allowed: usize,
    ) -> Result<(), String> {
        let mut map = self.counts.lock().unwrap_or_else(|e| e.into_inner());
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let timestamps = map.entry(peer_id.to_string()).or_default();
        timestamps.retain(|&ts| now.saturating_sub(ts) <= window_secs);

        if timestamps.len() >= max_allowed {
            return Err(format!(
                "Task queue flood protection: Per-peer rate limit exceeded for peer '{}' (max {} tasks / {}s)",
                peer_id, max_allowed, window_secs
            ));
        }

        timestamps.push(now);
        Ok(())
    }
}

/// Cryptographically signed worker execution result.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignedTaskResult {
    pub task_id: String,
    pub worker_node_id: String,
    pub worker_public_key: String,
    pub result: Value,
    pub timestamp: u64,
    pub worker_signature: String,
}

impl SignedTaskResult {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let res_str = serde_json::to_string(&self.result).unwrap_or_default();
        format!(
            "{}:{}:{}:{}",
            self.task_id, self.worker_node_id, self.timestamp, res_str
        )
        .into_bytes()
    }

    pub fn verify(&self) -> Result<(), String> {
        if self.worker_public_key.is_empty() || self.worker_signature.is_empty() {
            return Err("Missing worker public key or signature in task result".to_string());
        }
        let pubkey = Ed25519PublicKey::from_hex(&self.worker_public_key)
            .map_err(|e| format!("Invalid worker public key: {}", e))?;
        let msg = self.canonical_bytes();
        if !pubkey.verify_hex(&msg, &self.worker_signature) {
            return Err("Worker result cryptographic signature verification failed".to_string());
        }
        Ok(())
    }
}

pub fn create_signed_task_result(
    keypair: &Ed25519KeyPair,
    task_id: String,
    worker_node_id: String,
    result: Value,
    timestamp: u64,
) -> SignedTaskResult {
    let worker_public_key = keypair.public_key_hex();
    let mut signed_res = SignedTaskResult {
        task_id,
        worker_node_id,
        worker_public_key,
        result,
        timestamp,
        worker_signature: String::new(),
    };
    let msg = signed_res.canonical_bytes();
    signed_res.worker_signature = keypair.sign_hex(&msg);
    signed_res
}

/// Lifecycle state of a dispatched task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    Completed,
    Cancelled,
    Failed,
}

/// A single entry in the distributed work pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    pub task_id: String,
    pub ast: Node,
    pub priority: u8,
    pub status: TaskStatus,
    pub worker_node_id: Option<String>,
    pub worker_public_key: Option<String>,
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signed_result: Option<SignedTaskResult>,
}

/// Thread-safe distributed task queue with cooperative work-stealing and signature verification.
pub struct TaskDispatcher {
    tasks: Mutex<HashMap<String, TaskEntry>>,
    counter: AtomicU64,
}

impl Default for TaskDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl TaskDispatcher {
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(1),
        }
    }

    pub fn submit(&self, ast: Node, priority: u8) -> Result<String, String> {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());

        if tasks.len() >= MAX_TASK_QUEUE_DEPTH {
            tasks.retain(|_, entry| {
                matches!(entry.status, TaskStatus::Queued | TaskStatus::Running)
            });
        }

        if tasks.len() >= MAX_TASK_QUEUE_DEPTH {
            return Err(format!(
                "Task queue capacity limit exceeded (max {})",
                MAX_TASK_QUEUE_DEPTH
            ));
        }

        let id_num = self.counter.fetch_add(1, AtomicOrdering::Relaxed);
        let task_id = id_num.to_string();
        let entry = TaskEntry {
            task_id: task_id.clone(),
            ast,
            priority,
            status: TaskStatus::Queued,
            worker_node_id: None,
            worker_public_key: None,
            result: None,
            signed_result: None,
        };
        tasks.insert(task_id.clone(), entry);
        Ok(task_id)
    }

    pub fn gc_completed(&self) -> usize {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let initial_len = tasks.len();
        tasks.retain(|_, entry| matches!(entry.status, TaskStatus::Queued | TaskStatus::Running));
        initial_len - tasks.len()
    }

    pub fn status(&self, task_id: &str) -> Option<TaskEntry> {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        tasks.get(task_id).cloned()
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = tasks.get_mut(task_id)
            && entry.status == TaskStatus::Queued
        {
            entry.status = TaskStatus::Cancelled;
            return true;
        }
        false
    }

    pub fn mark_running(&self, task_id: &str, worker_node_id: &str) -> bool {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = tasks.get_mut(task_id)
            && entry.status == TaskStatus::Queued
        {
            entry.status = TaskStatus::Running;
            entry.worker_node_id = Some(worker_node_id.to_string());
            return true;
        }
        false
    }

    pub fn complete(&self, task_id: &str, result: Value) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = tasks.get_mut(task_id) {
            entry.status = TaskStatus::Completed;
            entry.result = Some(result);
        }
    }

    pub fn complete_signed<F>(
        &self,
        signed_result: SignedTaskResult,
        is_revoked: F,
    ) -> Result<(), String>
    where
        F: Fn(&str) -> bool,
    {
        signed_result.verify()?;
        if is_revoked(&signed_result.worker_public_key) {
            return Err("Unauthorized: Worker public key has been revoked".to_string());
        }
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = tasks.get_mut(&signed_result.task_id) {
            entry.status = TaskStatus::Completed;
            entry.result = Some(signed_result.result.clone());
            entry.worker_node_id = Some(signed_result.worker_node_id.clone());
            entry.worker_public_key = Some(signed_result.worker_public_key.clone());
            entry.signed_result = Some(signed_result);
            Ok(())
        } else {
            Err(format!("Task '{}' not found", signed_result.task_id))
        }
    }

    pub fn fail(&self, task_id: &str, error_msg: &str) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = tasks.get_mut(task_id) {
            entry.status = TaskStatus::Failed;
            entry.result = Some(Value::String(error_msg.to_string()));
        }
    }

    pub fn steal(&self, max_tasks: usize, worker_node_id: &str) -> Vec<TaskEntry> {
        if max_tasks == 0 {
            return Vec::new();
        }
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());

        let mut queued_ids: Vec<(u8, String)> = tasks
            .values()
            .filter(|e| e.status == TaskStatus::Queued)
            .map(|e| (e.priority, e.task_id.clone()))
            .collect();
        queued_ids.sort_by_key(|(p, _)| *p);
        queued_ids.truncate(max_tasks);

        let mut stolen = Vec::with_capacity(queued_ids.len());
        for (_, id) in &queued_ids {
            if let Some(entry) = tasks.get_mut(id) {
                entry.status = TaskStatus::Running;
                entry.worker_node_id = Some(worker_node_id.to_string());
                stolen.push(entry.clone());
            }
        }
        stolen
    }

    pub fn stats(&self) -> Value {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        let mut queued = 0u64;
        let mut running = 0u64;
        let mut completed = 0u64;
        let mut cancelled = 0u64;
        let mut failed = 0u64;
        for e in tasks.values() {
            match e.status {
                TaskStatus::Queued => queued += 1,
                TaskStatus::Running => running += 1,
                TaskStatus::Completed => completed += 1,
                TaskStatus::Cancelled => cancelled += 1,
                TaskStatus::Failed => failed += 1,
            }
        }
        serde_json::json!({
            "queued": queued,
            "running": running,
            "completed": completed,
            "cancelled": cancelled,
            "failed": failed,
            "total": tasks.len()
        })
    }
}

/// Node performance metrics including CPU, memory, and task queue depth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub cpu_load_percent: f64,
    pub memory_used_bytes: u64,
    pub memory_total_bytes: u64,
    pub memory_usage_percent: f64,
    pub task_queue_depth: Value,
    pub is_overloaded: bool,
}

/// Thread-safe collector for node metrics.
pub struct MetricsCollector {
    simulated_cpu_load: Mutex<Option<f64>>,
    simulated_memory_used: Mutex<Option<u64>>,
    simulated_memory_total: Mutex<Option<u64>>,
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            simulated_cpu_load: Mutex::new(None),
            simulated_memory_used: Mutex::new(None),
            simulated_memory_total: Mutex::new(None),
        }
    }

    #[cfg(any(test, debug_assertions))]
    pub fn set_simulated_cpu_load(&self, load: Option<f64>) {
        let mut guard = self
            .simulated_cpu_load
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = load;
    }

    #[cfg(any(test, debug_assertions))]
    pub fn set_simulated_memory(&self, used: Option<u64>, total: Option<u64>) {
        let mut u_guard = self
            .simulated_memory_used
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *u_guard = used;
        let mut t_guard = self
            .simulated_memory_total
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *t_guard = total;
    }

    pub fn collect(&self, task_queue_stats: Value) -> NodeMetrics {
        let cpu_load = self
            .simulated_cpu_load
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(15.0);

        let mem_used = self
            .simulated_memory_used
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(134_217_728);

        let mem_total = self
            .simulated_memory_total
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(8_589_934_592);

        let memory_usage_percent = if mem_total > 0 {
            (mem_used as f64 / mem_total as f64) * 100.0
        } else {
            0.0
        };

        let is_overloaded = cpu_load > 80.0 || memory_usage_percent > 85.0;

        NodeMetrics {
            cpu_load_percent: cpu_load,
            memory_used_bytes: mem_used,
            memory_total_bytes: mem_total,
            memory_usage_percent,
            task_queue_depth: task_queue_stats,
            is_overloaded,
        }
    }
}

impl super::super::RpcServer {
    pub fn handle_task_submit(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let sender_id = params
            .get("sender_node_id")
            .or_else(|| params.get("node_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("remote_peer");

        if let Err(err) =
            self.task_rate_limiter
                .check_and_record(sender_id, 60, MAX_PER_PEER_TASK_RATE)
        {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let sender_pubkey = params.get("sender_public_key").and_then(|v| v.as_str());
        let sender_sig = params.get("sender_signature").and_then(|v| v.as_str());
        let timestamp = params
            .get("timestamp")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        if let (Some(pubkey_hex), Some(sig_hex)) = (sender_pubkey, sender_sig) {
            if self.is_peer_key_revoked(pubkey_hex) {
                return JsonRpcResponse::error(
                    id,
                    -32001,
                    "Unauthorized: Sender peer key is revoked",
                );
            }
            let pubkey = match Ed25519PublicKey::from_hex(pubkey_hex) {
                Ok(pk) => pk,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("Invalid sender_public_key: {}", e),
                    );
                }
            };
            let node = match self.extract_ast_node(&params) {
                Ok(n) => n,
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            };
            let priority = params
                .get("priority")
                .and_then(|v| v.as_u64())
                .map(|v| (v & 0xFF) as u8)
                .unwrap_or(128);
            let ast_str = serde_json::to_string(&node).unwrap_or_default();
            let msg = format!("{}:{}:{}", timestamp, priority, ast_str);
            if !pubkey.verify_hex(msg.as_bytes(), sig_hex) {
                return JsonRpcResponse::error(
                    id,
                    -32001,
                    "Cryptographic signature verification failed for task payload",
                );
            }
        } else if self.is_zero_trust() {
            return JsonRpcResponse::error(
                id,
                -32001,
                "Zero-Trust Policy: Missing required cryptographic task signature parameters ('sender_public_key', 'sender_signature')",
            );
        }

        let node = match self.extract_ast_node(&params) {
            Ok(n) => n,
            Err(err) => return JsonRpcResponse::error(id, -32602, err),
        };

        let priority = params
            .get("priority")
            .and_then(|v| v.as_u64())
            .map(|v| (v & 0xFF) as u8)
            .unwrap_or(128);

        match self.task_dispatcher.submit(node, priority) {
            Ok(task_id) => {
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "task_id": task_id,
                    "task_status": "Queued",
                    "priority": priority,
                    "queue_depth": self.task_dispatcher.stats()
                });
                JsonRpcResponse::success(id, resp_val)
            }
            Err(err) => JsonRpcResponse::error(id, -32001, err),
        }
    }

    pub fn handle_task_status(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let task_id = match params.get("task_id").and_then(|v| v.as_str()) {
            Some(t) => match validate_param_string_len(t) {
                Ok(_) => t,
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => return JsonRpcResponse::error(id, -32602, "Missing 'task_id' parameter"),
        };

        if let Some(entry) = self.task_dispatcher.status(task_id) {
            let resp_val = serde_json::json!({
                "status": "ok",
                "task_id": task_id,
                "task_status": format!("{:?}", entry.status),
                "task": entry
            });
            JsonRpcResponse::success(id, resp_val)
        } else {
            JsonRpcResponse::error(id, -32602, format!("Task '{}' not found", task_id))
        }
    }

    pub fn handle_task_cancel(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let task_id = match params.get("task_id").and_then(|v| v.as_str()) {
            Some(t) => match validate_param_string_len(t) {
                Ok(_) => t,
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => return JsonRpcResponse::error(id, -32602, "Missing 'task_id' parameter"),
        };

        let cancelled = self.task_dispatcher.cancel(task_id);
        let resp_val = serde_json::json!({
            "status": "ok",
            "cancelled": cancelled,
            "task_id": task_id
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_task_steal(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let worker_cpu = params
            .get("worker_cpu_load")
            .or_else(|| params.get("worker_cpu_percent"))
            .and_then(|v| v.as_f64());

        let local_metrics = self.metrics_collector.collect(self.task_dispatcher.stats());

        let is_throttled = local_metrics.is_overloaded || worker_cpu.is_some_and(|cpu| cpu > 80.0);

        if is_throttled {
            let resp_val = serde_json::json!({
                "status": "ok",
                "stolen_count": 0,
                "stolen": [],
                "tasks": [],
                "throttled": true,
                "reason": "Throttled: Local node CPU/memory load exceeds safety threshold"
            });
            return JsonRpcResponse::success(id, resp_val);
        }

        let max_tasks = params
            .get("max_tasks")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(1);

        let worker_id = params
            .get("worker_node_id")
            .or_else(|| params.get("node_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown_worker");

        let stolen = self.task_dispatcher.steal(max_tasks, worker_id);
        let resp_val = serde_json::json!({
            "status": "ok",
            "stolen_count": stolen.len(),
            "stolen": stolen,
            "tasks": stolen,
            "throttled": false
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_task_complete(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let signed_res_val = match params.get("signed_result") {
            Some(v) => v,
            None => return JsonRpcResponse::error(id, -32602, "Missing 'signed_result' parameter"),
        };

        let signed_result: SignedTaskResult = match serde_json::from_value(signed_res_val.clone()) {
            Ok(r) => r,
            Err(e) => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    format!("Invalid SignedTaskResult payload: {}", e),
                );
            }
        };

        match self
            .task_dispatcher
            .complete_signed(signed_result, |pk| self.is_peer_key_revoked(pk))
        {
            Ok(()) => JsonRpcResponse::success(
                id,
                serde_json::json!({ "status": "ok", "completed": true }),
            ),
            Err(err) => JsonRpcResponse::error(id, -32001, err),
        }
    }
}
