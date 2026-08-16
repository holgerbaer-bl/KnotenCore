use knoten_core_types::ast::Node;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};

use crate::rpc::types::{JsonRpcResponse, MAX_TASK_QUEUE_DEPTH, validate_param_string_len};

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
    pub result: Option<Value>,
}

/// Thread-safe distributed task queue with cooperative work-stealing.
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
            result: None,
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
}
