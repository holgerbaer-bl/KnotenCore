// Sprint 310: Headless JSON-RPC 2.0 Server Engine & Agentic Transport Protocol
//
// Exposes KnotenCore AST compilation, VM execution, yield/resume suspension,
// event streaming hooks, and state inspection via JSON-RPC 2.0.
//
// Sprint 319: Distributed Task Queue & Mesh Work-Stealing Engine
//
// Adds TaskDispatcher with knc_task_submit / knc_task_status / knc_task_cancel
// and a cooperative work-stealing protocol (knc_task_steal) for mesh peers.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use knoten_core_types::ast::{IsolateQuota, Node};
use knoten_core_types::opcode::OpCode;

use crate::executor::{AgentPermissions, RelType};
use crate::optimizer::optimize;
use crate::validator::Validator;
use crate::vm::compiler::Compiler;
use crate::vm::machine::{VM, VmEvent, VmExecutionState};

pub const KNC_PROTOCOL_VERSION: &str = "v2.17.0-store";

/// JSON-RPC 2.0 Request Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: Value,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Response Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: Option<Value>,
}

/// JSON-RPC 2.0 Error Payload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: Option<Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
            id,
        }
    }
}

/// Thread-safe RPC Session State
#[derive(Default)]
pub struct RpcSession {
    pub vm: VM,
    pub instructions: Vec<OpCode>,
    pub constants: Vec<RelType>,
    pub events: Vec<VmEvent>,
}

fn default_peer_status() -> String {
    "Active".to_string()
}

/// Mesh Peer Topology Information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshPeer {
    pub node_id: String,
    pub address: String,
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub last_seen: u64,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default = "default_peer_status")]
    pub status: String,
}

/// JSON-RPC 2.0 Server Handler
pub struct RpcServer {
    pub permissions: AgentPermissions,
    pub sessions: Arc<Mutex<HashMap<String, RpcSession>>>,
    pub node_id: String,
    pub node_address: String,
    pub mesh_auth_token: Option<String>,
    pub peers: Arc<Mutex<HashMap<String, MeshPeer>>>,
    /// Sprint 319: Distributed Task Queue — shared across all handler calls
    pub task_dispatcher: Arc<TaskDispatcher>,
    /// Sprint 320: Cluster Metrics Collector & Adaptive Work-Stealing Guard
    pub metrics_collector: Arc<MetricsCollector>,
    /// Sprint 321: Distributed CRDT Key-Value Storage & State Sync
    pub store: Arc<MeshKvStore>,
}

impl RpcServer {
    pub fn new(permissions: AgentPermissions) -> Self {
        Self::with_mesh(permissions, "knc-node-local", "127.0.0.1:0", None)
    }

    pub fn with_mesh(
        permissions: AgentPermissions,
        node_id: impl Into<String>,
        node_address: impl Into<String>,
        mesh_auth_token: Option<String>,
    ) -> Self {
        Self {
            permissions,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            node_id: node_id.into(),
            node_address: node_address.into(),
            mesh_auth_token,
            peers: Arc::new(Mutex::new(HashMap::new())),
            task_dispatcher: Arc::new(TaskDispatcher::new()),
            metrics_collector: Arc::new(MetricsCollector::new()),
            store: Arc::new(MeshKvStore::new()),
        }
    }

    pub fn dispatch_request(&self, request_raw: &str) -> String {
        let req: JsonRpcRequest = match serde_json::from_str(request_raw) {
            Ok(r) => r,
            Err(e) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                return serde_json::to_string(&resp).unwrap_or_default();
            }
        };

        if req.jsonrpc != "2.0" {
            let resp = JsonRpcResponse::error(
                req.id,
                -32600,
                "Invalid Request: jsonrpc version must be '2.0'",
            );
            return serde_json::to_string(&resp).unwrap_or_default();
        }

        let response = match req.method.as_str() {
            "knc_compile" => self.handle_compile(req.id, req.params),
            "knc_execute" => self.handle_execute(req.id, req.params),
            "knc_yield_resume" => self.handle_yield_resume(req.id, req.params),
            "knc_inspect_state" => self.handle_inspect_state(req.id, req.params),
            "knc_agent_handshake" => self.handle_agent_handshake(req.id, req.params),
            "knc_agent_snapshot" => self.handle_agent_snapshot(req.id, req.params),
            "knc_agent_restore" => self.handle_agent_restore(req.id, req.params),
            "knc_mesh_discover" => self.handle_mesh_discover(req.id, req.params),
            "knc_mesh_peers" => self.handle_mesh_peers(req.id, req.params),
            "knc_mesh_ping" => self.handle_mesh_ping(req.id, req.params),
            "knc_agent_teleport" => self.handle_agent_teleport(req.id, req.params),
            // Sprint 319: Distributed Task Queue & Work-Stealing
            "knc_task_submit" => self.handle_task_submit(req.id, req.params),
            "knc_task_status" => self.handle_task_status(req.id, req.params),
            "knc_task_cancel" => self.handle_task_cancel(req.id, req.params),
            "knc_task_steal" => self.handle_task_steal(req.id, req.params),
            // Sprint 320: Cluster Metrics
            "knc_mesh_metrics" => self.handle_mesh_metrics(req.id, req.params),
            // Sprint 321: Distributed CRDT KV-Storage & State Sync
            "knc_store_put" => self.handle_store_put(req.id, req.params),
            "knc_store_get" => self.handle_store_get(req.id, req.params),
            "knc_store_sync" => self.handle_store_sync(req.id, req.params),
            _ => {
                JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method))
            }
        };

        serde_json::to_string(&response).unwrap_or_default()
    }

    fn handle_compile(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let node: Node = match self.extract_ast_node(&params) {
            Ok(n) => n,
            Err(err) => return JsonRpcResponse::error(id, -32602, err),
        };

        let mut validator = Validator::new();
        if let Err(errs) = validator.validate(&node) {
            let msg = errs.join("; ");
            return JsonRpcResponse::error(id, -32602, format!("AST Validation Error: {}", msg));
        }

        let opt_node = optimize(node);

        if let Some(q) = params
            .get("quota")
            .and_then(|v| serde_json::from_value::<IsolateQuota>(v.clone()).ok())
        {
            let count = crate::optimizer::count_nodes(&opt_node);
            if (count as u64) > q.max_instructions {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!(
                        "Quota Exceeded: AST node count ({}) exceeds max_instructions ({})",
                        count, q.max_instructions
                    ),
                );
            }
        }

        let mut compiler = Compiler::new();
        if !compiler.compile_node(&opt_node) {
            return JsonRpcResponse::error(id, -32603, "Compiler Error: Node compilation failed");
        }

        let result_json = serde_json::json!({
            "status": "ok",
            "instruction_count": compiler.instructions.len(),
            "constants_count": compiler.constants.len(),
            "instructions": compiler.instructions,
            "constants": compiler.constants
        });

        JsonRpcResponse::success(id, result_json)
    }

    fn handle_execute(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let (instructions, constants) = if let Ok(node) = self.extract_ast_node(&params) {
            let opt_node = optimize(node);
            let mut compiler = Compiler::new();
            if !compiler.compile_node(&opt_node) {
                return JsonRpcResponse::error(id, -32603, "Compilation failed for execute");
            }
            (compiler.instructions, compiler.constants)
        } else if let (Some(inst_val), Some(const_val)) =
            (params.get("instructions"), params.get("constants"))
        {
            let inst: Vec<OpCode> = match serde_json::from_value(inst_val.clone()) {
                Ok(i) => i,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("Invalid instructions param: {}", e),
                    );
                }
            };
            let cnst: Vec<RelType> = match serde_json::from_value(const_val.clone()) {
                Ok(c) => c,
                Err(e) => {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        format!("Invalid constants param: {}", e),
                    );
                }
            };
            (inst, cnst)
        } else {
            return JsonRpcResponse::error(
                id,
                -32602,
                "Params must contain either 'ast' or ('instructions' and 'constants')",
            );
        };

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.entry(session_id.clone()).or_default();

        if let Some(q) = params
            .get("quota")
            .and_then(|v| serde_json::from_value::<IsolateQuota>(v.clone()).ok())
        {
            session.vm.set_quota(q);
        }

        session.instructions = instructions;
        session.constants = constants;
        session.events.clear();

        let events_container = Arc::new(Mutex::new(Vec::new()));
        let events_clone = events_container.clone();
        session.vm.set_event_hook(Arc::new(move |ev| {
            if let Ok(mut lock) = events_clone.lock() {
                lock.push(ev);
            }
        }));

        let exec_result = session.vm.run(
            &session.instructions,
            &session.constants,
            &self.permissions,
            None,
        );

        if let Ok(collected) = events_container.lock() {
            session.events = collected.clone();
        }

        match exec_result {
            Ok(val) => {
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "session_id": session_id,
                    "result": val,
                    "execution_state": format!("{:?}", session.vm.execution_state()),
                    "is_yielded": session.vm.is_yielded(),
                    "events": session.events
                });
                JsonRpcResponse::success(id, resp_val)
            }
            Err(err) => {
                if err.contains("ERR_QUOTA_EXCEEDED")
                    || err.contains("ERR_SANDBOX_TIMEOUT")
                    || err.contains("ERR_MEMORY_LIMIT_EXCEEDED")
                    || err.contains("Watchdog")
                {
                    JsonRpcResponse::error(id, -32000, format!("Quota Exceeded: {}", err))
                } else {
                    JsonRpcResponse::error(id, -32603, format!("Runtime Fault: {}", err))
                }
            }
        }
    }

    fn handle_yield_resume(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = match sessions.get_mut(&session_id) {
            Some(s) => s,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    format!("Session '{}' not found", session_id),
                );
            }
        };

        if let Some(q) = params
            .get("quota")
            .and_then(|v| serde_json::from_value::<IsolateQuota>(v.clone()).ok())
        {
            session.vm.set_quota(q);
        }

        if !session.vm.is_yielded() {
            return JsonRpcResponse::error(
                id,
                -32603,
                format!(
                    "Session '{}' is not in Yielded state ({:?})",
                    session_id,
                    session.vm.execution_state()
                ),
            );
        }

        let resume_res = session.vm.resume(
            &session.instructions,
            &session.constants,
            &self.permissions,
            None,
        );

        match resume_res {
            Ok(val) => {
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "session_id": session_id,
                    "result": val,
                    "execution_state": format!("{:?}", session.vm.execution_state()),
                    "is_yielded": session.vm.is_yielded()
                });
                JsonRpcResponse::success(id, resp_val)
            }
            Err(err) => {
                if err.contains("ERR_QUOTA_EXCEEDED")
                    || err.contains("ERR_SANDBOX_TIMEOUT")
                    || err.contains("ERR_MEMORY_LIMIT_EXCEEDED")
                    || err.contains("Watchdog")
                {
                    JsonRpcResponse::error(id, -32000, format!("Quota Exceeded: {}", err))
                } else {
                    JsonRpcResponse::error(id, -32603, format!("Resume Fault: {}", err))
                }
            }
        }
    }

    fn handle_inspect_state(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = match sessions.get(&session_id) {
            Some(s) => s,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    format!("Session '{}' not found", session_id),
                );
            }
        };

        let inspector_data = session.vm.inspect();
        let resp_val = serde_json::json!({
            "status": "ok",
            "session_id": session_id,
            "execution_state": format!("{:?}", session.vm.execution_state()),
            "is_yielded": session.vm.is_yielded(),
            "ip": session.vm.ip,
            "stack_size": session.vm.stack.len(),
            "frames_count": session.vm.frames.len(),
            "globals_count": session.vm.globals.len(),
            "inspector": inspector_data
        });

        JsonRpcResponse::success(id, resp_val)
    }

    fn handle_agent_handshake(&self, id: Option<Value>, _params: Value) -> JsonRpcResponse {
        let default_quota = IsolateQuota::default();
        let resp_val = serde_json::json!({
            "status": "ok",
            "protocol_version": KNC_PROTOCOL_VERSION,
            "engine": "KnotenCore",
            "capabilities": {
                "jsonrpc": true,
                "websocket": true,
                "isolate_quotas": true,
                "async_yield": true,
                "state_snapshots": true,
                "mesh_protocol": true,
                "task_queue": true,
                "work_stealing": true,
                "cluster_metrics": true,
                "adaptive_work_stealing": true,
                "crdt_store": true,
                "peer_state_sync": true
            },
            "default_quota": default_quota
        });
        JsonRpcResponse::success(id, resp_val)
    }

    // -------------------------------------------------------------------------
    // Sprint 319: Distributed Task Queue handlers
    // -------------------------------------------------------------------------

    /// `knc_task_submit` — submit a JSON-AST as a steerable task.
    ///
    /// Params: `{ "ast": <Node>, "priority": <u8 opt> }`
    /// Returns: `{ "task_id": "<uuid>", "status": "Queued" }`
    fn handle_task_submit(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let node = match self.extract_ast_node(&params) {
            Ok(n) => n,
            Err(e) => return JsonRpcResponse::error(id, -32602, e),
        };
        let priority = params
            .get("priority")
            .and_then(|v| v.as_u64())
            .unwrap_or(128) as u8;
        let task_id = self.task_dispatcher.submit(node, priority);
        let resp = serde_json::json!({
            "status": "ok",
            "task_id": task_id,
            "task_status": "Queued"
        });
        JsonRpcResponse::success(id, resp)
    }

    /// `knc_task_status` — poll the status and result of a task.
    ///
    /// Params: `{ "task_id": "<uuid>" }`
    /// Returns: `{ "task_id": "...", "task_status": "Queued|Running|Completed|Cancelled|Failed",
    ///             "result": <value or null> }`
    fn handle_task_status(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let task_id = match params.get("task_id").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "Missing 'task_id' parameter"),
        };
        match self.task_dispatcher.status(&task_id) {
            Some(entry) => {
                let resp = serde_json::json!({
                    "status": "ok",
                    "task_id": task_id,
                    "task_status": format!("{:?}", entry.status),
                    "result": entry.result
                });
                JsonRpcResponse::success(id, resp)
            }
            None => JsonRpcResponse::error(id, -32602, format!("Task '{}' not found", task_id)),
        }
    }

    /// `knc_task_cancel` — request cancellation of a queued or running task.
    ///
    /// Params: `{ "task_id": "<uuid>" }`
    /// Returns: `{ "task_id": "...", "cancelled": true|false }`
    fn handle_task_cancel(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let task_id = match params.get("task_id").and_then(|v| v.as_str()) {
            Some(t) => t.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "Missing 'task_id' parameter"),
        };
        let cancelled = self.task_dispatcher.cancel(&task_id);
        let resp = serde_json::json!({
            "status": "ok",
            "task_id": task_id,
            "cancelled": cancelled
        });
        JsonRpcResponse::success(id, resp)
    }

    /// `knc_task_steal` — work-stealing: a free mesh peer requests up to N unassigned tasks.
    /// Includes adaptive throttling guard (throttles when CPU > 80% or memory > 85%).
    ///
    /// Params: `{ "max_tasks": <u64 opt>, "worker_node_id": "<str>", "worker_cpu_load": <f64 opt>, "worker_memory_usage": <f64 opt> }`
    /// Returns: `{ "stolen": [ ... ], "throttled": true|false }`
    fn handle_task_steal(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        // Adaptive Work-Stealing Guard
        let task_stats = self.task_dispatcher.stats();
        let current_metrics = self.metrics_collector.collect(task_stats);

        let worker_cpu = params
            .get("worker_cpu_load")
            .and_then(|v| v.as_f64())
            .unwrap_or(current_metrics.cpu_load_percent);

        let worker_mem = params
            .get("worker_memory_usage")
            .and_then(|v| v.as_f64())
            .unwrap_or(current_metrics.memory_usage_percent);

        let is_overloaded = current_metrics.is_overloaded || worker_cpu > 80.0 || worker_mem > 85.0;

        if is_overloaded {
            let resp = serde_json::json!({
                "status": "ok",
                "stolen": [],
                "throttled": true,
                "reason": "Throttled: CPU load > 80% or memory threshold exceeded"
            });
            return JsonRpcResponse::success(id, resp);
        }

        let max_tasks = params
            .get("max_tasks")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as usize;
        let worker_id = params
            .get("worker_node_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let stolen = self.task_dispatcher.steal(max_tasks, &worker_id);
        let stolen_json: Vec<Value> = stolen
            .into_iter()
            .map(|t| {
                serde_json::json!({
                    "task_id": t.task_id,
                    "ast": t.ast,
                    "priority": t.priority
                })
            })
            .collect();

        let resp = serde_json::json!({
            "status": "ok",
            "stolen": stolen_json,
            "throttled": false
        });
        JsonRpcResponse::success(id, resp)
    }

    /// `knc_mesh_metrics` — returns node performance metrics (CPU, RAM, queue depth).
    fn handle_mesh_metrics(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let task_stats = self.task_dispatcher.stats();
        let metrics = self.metrics_collector.collect(task_stats);

        let resp_val = serde_json::json!({
            "status": "ok",
            "node_id": self.node_id,
            "address": self.node_address,
            "protocol_version": KNC_PROTOCOL_VERSION,
            "metrics": metrics
        });

        JsonRpcResponse::success(id, resp_val)
    }

    // -------------------------------------------------------------------------
    // Sprint 321: Distributed CRDT Key-Value Storage & State Sync
    // -------------------------------------------------------------------------

    /// `knc_store_put` — write or update a CRDT LWW key-value entry.
    ///
    /// Params: `{ "key": "<str>", "value": <Value>, "timestamp": <u64 opt>, "writer_id": "<str opt>" }`
    /// Returns: `{ "status": "ok", "key": "...", "updated": true|false, "entry": <CrdtEntry> }`
    fn handle_store_put(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "Missing 'key' parameter"),
        };
        let value = match params.get("value") {
            Some(v) => v.clone(),
            None => Value::Null,
        };
        let timestamp = params
            .get("timestamp")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis() as u64
            });
        let writer_id = params
            .get("writer_id")
            .and_then(|v| v.as_str())
            .unwrap_or(&self.node_id)
            .to_string();

        let updated = self.store.put(&key, value, timestamp, &writer_id);
        let entry = self.store.get(&key);

        let resp_val = serde_json::json!({
            "status": "ok",
            "key": key,
            "updated": updated,
            "entry": entry
        });
        JsonRpcResponse::success(id, resp_val)
    }

    /// `knc_store_get` — read the CRDT LWW entry for a key.
    ///
    /// Params: `{ "key": "<str>" }`
    /// Returns: `{ "status": "ok", "key": "...", "entry": <CrdtEntry or null> }`
    fn handle_store_get(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let key = match params.get("key").and_then(|v| v.as_str()) {
            Some(k) => k.to_string(),
            None => return JsonRpcResponse::error(id, -32602, "Missing 'key' parameter"),
        };

        let entry = self.store.get(&key);
        let resp_val = serde_json::json!({
            "status": "ok",
            "key": key,
            "entry": entry
        });
        JsonRpcResponse::success(id, resp_val)
    }

    /// `knc_store_sync` — merge incoming CRDT entries from a peer and return full snapshot.
    ///
    /// Params: `{ "entries": [ { "key": "...", "value": ..., "timestamp": ..., "writer_id": ... }, ... ] }`
    /// Returns: `{ "status": "ok", "synced_count": N, "entries": [ ... ] }`
    fn handle_store_sync(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let incoming: Vec<CrdtEntry> = params
            .get("entries")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let synced_count = self.store.sync(incoming);
        let full_entries = self.store.dump_entries();

        let resp_val = serde_json::json!({
            "status": "ok",
            "synced_count": synced_count,
            "entries": full_entries
        });
        JsonRpcResponse::success(id, resp_val)
    }

    fn handle_agent_snapshot(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = match sessions.get(&session_id) {
            Some(s) => s,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    format!("Session '{}' not found", session_id),
                );
            }
        };

        let vm_state = session.vm.snapshot();
        let snapshot_data = serde_json::json!({
            "session_id": session_id,
            "execution_state": session.vm.execution_state(),
            "vm_state": vm_state,
            "instructions": session.instructions,
            "constants": session.constants,
            "quota": session.vm.quota
        });

        let resp_val = serde_json::json!({
            "status": "ok",
            "session_id": session_id,
            "snapshot": snapshot_data
        });

        JsonRpcResponse::success(id, resp_val)
    }

    fn handle_agent_restore(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let snapshot_val = match params.get("snapshot") {
            Some(v) => v,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    "Missing 'snapshot' parameter in restore request",
                );
            }
        };

        let execution_state: VmExecutionState = match snapshot_val
            .get("execution_state")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
        {
            Some(st) => st,
            None => VmExecutionState::Ready,
        };

        let vm_state: crate::vm::machine::VMState = match snapshot_val
            .get("vm_state")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
        {
            Some(st) => st,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    "Invalid 'vm_state' in snapshot payload",
                );
            }
        };

        let instructions: Vec<OpCode> = snapshot_val
            .get("instructions")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let constants: Vec<RelType> = snapshot_val
            .get("constants")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        let quota: IsolateQuota = snapshot_val
            .get("quota")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();

        const MAX_STACK_DEPTH: usize = 4096;
        const MAX_GLOBALS: usize = 10000;
        const MAX_FRAMES: usize = 256;

        if vm_state.stack.len() > MAX_STACK_DEPTH {
            return JsonRpcResponse::error(
                id,
                -32602,
                format!(
                    "Snapshot stack depth ({}) exceeds MAX_STACK_DEPTH ({})",
                    vm_state.stack.len(),
                    MAX_STACK_DEPTH
                ),
            );
        }
        if vm_state.globals.len() > MAX_GLOBALS {
            return JsonRpcResponse::error(
                id,
                -32602,
                format!(
                    "Snapshot globals count ({}) exceeds MAX_GLOBALS ({})",
                    vm_state.globals.len(),
                    MAX_GLOBALS
                ),
            );
        }
        if vm_state.frames.len() > MAX_FRAMES {
            return JsonRpcResponse::error(
                id,
                -32602,
                format!(
                    "Snapshot call frames count ({}) exceeds MAX_FRAMES ({})",
                    vm_state.frames.len(),
                    MAX_FRAMES
                ),
            );
        }

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = sessions.entry(session_id.clone()).or_default();

        session.vm.rollback(vm_state);
        session.vm.set_quota(quota);
        session.vm.execution_state = execution_state;
        session.instructions = instructions;
        session.constants = constants;

        let resp_val = serde_json::json!({
            "status": "ok",
            "session_id": session_id,
            "execution_state": format!("{:?}", session.vm.execution_state()),
            "is_yielded": session.vm.is_yielded()
        });

        JsonRpcResponse::success(id, resp_val)
    }

    fn check_mesh_auth(&self, params: &Value) -> Result<(), String> {
        if let Some(expected_token) = &self.mesh_auth_token {
            if let Some(sig) = params
                .get("mesh_auth_signature")
                .or_else(|| params.get("signature"))
                .and_then(|v| v.as_str())
            {
                let timestamp_or_nonce = params
                    .get("timestamp")
                    .map(|v| v.to_string())
                    .or_else(|| {
                        params
                            .get("nonce")
                            .and_then(|v| v.as_str())
                            .map(|s| s.to_string())
                    })
                    .unwrap_or_default();
                let sender = params
                    .get("sender_node_id")
                    .or_else(|| params.get("node_id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or_default();

                let message = format!("{}:{}", timestamp_or_nonce, sender);
                let expected_sig = hmac_sha256(expected_token.as_bytes(), message.as_bytes());

                if constant_time_eq(sig.as_bytes(), expected_sig.as_bytes()) {
                    return Ok(());
                }
                return Err("Unauthorized: Invalid mesh_auth_signature".to_string());
            }

            let token = params
                .get("mesh_auth_token")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !constant_time_eq(token.as_bytes(), expected_token.as_bytes()) {
                return Err("Unauthorized: Invalid or missing mesh_auth_token".to_string());
            }
        }
        Ok(())
    }

    fn handle_mesh_discover(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
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

    fn handle_mesh_peers(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or("list");

        let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());

        if action == "prune" {
            let before = peers.len();
            peers.retain(|_, peer| peer.status != "Evicted");
            let pruned_count = before - peers.len();
            let peer_list: Vec<MeshPeer> = peers.values().cloned().collect();
            let resp_val = serde_json::json!({
                "status": "ok",
                "pruned_count": pruned_count,
                "peers": peer_list
            });
            return JsonRpcResponse::success(id, resp_val);
        }

        if action == "register" || action == "add" {
            if let Some(peer_val) = params.get("peer") {
                if let Ok(mut peer) = serde_json::from_value::<MeshPeer>(peer_val.clone()) {
                    if !peers.contains_key(&peer.node_id) && peers.len() >= 256 {
                        return JsonRpcResponse::error(
                            id,
                            -32000,
                            "Peer capacity limit exceeded (max 256 peers)",
                        );
                    }
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    if peer.last_seen == 0 {
                        peer.last_seen = now;
                    }
                    if peer.status.is_empty() {
                        peer.status = "Active".to_string();
                    }
                    peers.insert(peer.node_id.clone(), peer.clone());
                    let resp_val = serde_json::json!({
                        "status": "ok",
                        "registered_peer": peer
                    });
                    return JsonRpcResponse::success(id, resp_val);
                } else {
                    return JsonRpcResponse::error(
                        id,
                        -32602,
                        "Invalid 'peer' parameter structure",
                    );
                }
            } else {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    "Missing 'peer' parameter for registration",
                );
            }
        }

        let status_filter = params.get("status_filter").and_then(|v| v.as_str());
        let peer_list: Vec<MeshPeer> = peers
            .values()
            .filter(|p| {
                if let Some(st) = status_filter {
                    p.status == st
                } else {
                    true
                }
            })
            .cloned()
            .collect();

        let resp_val = serde_json::json!({
            "status": "ok",
            "peers": peer_list
        });

        JsonRpcResponse::success(id, resp_val)
    }

    fn handle_mesh_ping(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        if let Some(sender_id) = params.get("sender_node_id").and_then(|v| v.as_str()) {
            let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(peer) = peers.get_mut(sender_id) {
                peer.last_seen = now;
                peer.status = "Active".to_string();
                if let Some(lat) = params.get("latency_ms").and_then(|v| v.as_u64()) {
                    peer.latency_ms = lat;
                }
            } else if let Some(sender_addr) = params.get("sender_address").and_then(|v| v.as_str())
            {
                if peers.len() >= 256 {
                    return JsonRpcResponse::error(
                        id,
                        -32000,
                        "Peer capacity limit exceeded (max 256 peers)",
                    );
                }
                peers.insert(
                    sender_id.to_string(),
                    MeshPeer {
                        node_id: sender_id.to_string(),
                        address: sender_addr.to_string(),
                        capabilities: vec![
                            "mesh_discover".to_string(),
                            "mesh_peers".to_string(),
                            "mesh_ping".to_string(),
                            "mesh_gossip".to_string(),
                            "agent_teleport".to_string(),
                        ],
                        last_seen: now,
                        latency_ms: params
                            .get("latency_ms")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0),
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
            "timestamp": now,
            "latency_ms": params.get("latency_ms").unwrap_or(&serde_json::json!(0))
        });

        JsonRpcResponse::success(id, resp_val)
    }

    fn handle_agent_teleport(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let target_session_id = params
            .get("target_session_id")
            .or_else(|| params.get("session_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("teleported_session")
            .to_string();

        if let Some(target_addr) = params
            .get("target_node_address")
            .and_then(|v| v.as_str())
            .filter(|addr| !addr.is_empty() && *addr != self.node_address)
        {
            let req_payload = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "knc_agent_teleport",
                "params": {
                    "target_session_id": target_session_id,
                    "mesh_auth_token": params.get("mesh_auth_token"),
                    "snapshot": params.get("snapshot")
                },
                "id": id.clone().unwrap_or(serde_json::json!(1))
            });

            return match self.send_rpc_to_node(target_addr, &req_payload.to_string()) {
                Ok(resp_str) => {
                    if let Ok(resp_json) = serde_json::from_str::<JsonRpcResponse>(&resp_str) {
                        resp_json
                    } else {
                        JsonRpcResponse::error(
                            id,
                            -32603,
                            format!("Invalid response from target node {}", target_addr),
                        )
                    }
                }
                Err(e) => JsonRpcResponse::error(
                    id,
                    -32603,
                    format!("Teleport transport failure to {}: {}", target_addr, e),
                ),
            };
        }

        let snapshot_val = match params.get("snapshot") {
            Some(v) => v,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    "Missing 'snapshot' parameter in teleport request",
                );
            }
        };

        let restore_params = serde_json::json!({
            "session_id": target_session_id,
            "snapshot": snapshot_val
        });

        let restore_resp = self.handle_agent_restore(id.clone(), restore_params);
        if restore_resp.error.is_some() {
            return restore_resp;
        }

        let resp_val = serde_json::json!({
            "status": "ok",
            "session_id": target_session_id,
            "teleported": true
        });

        JsonRpcResponse::success(id, resp_val)
    }

    pub fn send_rpc_to_node(&self, address: &str, payload: &str) -> Result<String, String> {
        self.send_rpc_to_node_with_timeout(address, payload, 5000)
    }

    pub fn send_rpc_to_node_with_timeout(
        &self,
        address: &str,
        payload: &str,
        timeout_ms: u64,
    ) -> Result<String, String> {
        let socket_addr: std::net::SocketAddr = address
            .parse()
            .map_err(|e| format!("Invalid address '{}': {}", address, e))?;
        let stream =
            TcpStream::connect_timeout(&socket_addr, std::time::Duration::from_millis(timeout_ms))
                .map_err(|e| format!("Failed to connect to node {}: {}", address, e))?;

        stream
            .set_read_timeout(Some(std::time::Duration::from_millis(timeout_ms)))
            .ok();
        stream
            .set_write_timeout(Some(std::time::Duration::from_millis(timeout_ms)))
            .ok();

        let mut stream = stream;
        let request_bytes = format!("{}\n", payload);
        stream
            .write_all(request_bytes.as_bytes())
            .map_err(|e| format!("Failed to write to node {}: {}", address, e))?;
        stream
            .flush()
            .map_err(|e| format!("Failed to flush stream to node {}: {}", address, e))?;

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| format!("Failed to read response from node {}: {}", address, e))?;

        Ok(response_line.trim().to_string())
    }

    fn extract_ast_node(&self, params: &Value) -> Result<Node, String> {
        if let Some(ast_val) = params.get("ast") {
            serde_json::from_value::<Node>(ast_val.clone())
                .map_err(|e| format!("Invalid 'ast' param format: {}", e))
        } else if let Some(code_val) = params.get("code") {
            if let Some(code_str) = code_val.as_str() {
                serde_json::from_str::<Node>(code_str)
                    .map_err(|e| format!("Invalid 'code' JSON AST string: {}", e))
            } else {
                serde_json::from_value::<Node>(code_val.clone())
                    .map_err(|e| format!("Invalid 'code' param object: {}", e))
            }
        } else if let Ok(node) = serde_json::from_value::<Node>(params.clone()) {
            Ok(node)
        } else {
            Err("Failed to parse Node from params. Provide 'ast' or 'code'".to_string())
        }
    }

    pub fn listen_tcp(&self, port: u16) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        println!(
            "[KnotenCore JSON-RPC] Server listening on 127.0.0.1:{}",
            port
        );

        for stream in listener.incoming().flatten() {
            self.handle_connection(stream);
        }
        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();

        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                let response = self.dispatch_request(trimmed);
                let _ = writeln!(stream, "{}", response);
            }
            line.clear();
        }
    }

    pub fn dispatch_request_ws(&self, request_raw: &str) -> (Option<String>, String, Vec<VmEvent>) {
        let response_str = self.dispatch_request(request_raw);
        let mut session_id = None;
        let mut events = Vec::new();

        if let Some(s) = serde_json::from_str::<Value>(request_raw)
            .ok()
            .and_then(|v| v.get("params").cloned())
            .and_then(|p| p.get("session_id").cloned())
            .and_then(|v| v.as_str().map(|s| s.to_string()))
        {
            session_id = Some(s.clone());
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(session) = sessions.get(&s) {
                events = session.events.clone();
            }
        }

        (session_id, response_str, events)
    }

    pub fn listen_ws(&self, port: u16) -> std::io::Result<()> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        println!(
            "[KnotenCore WebSocket RPC] Server listening on 127.0.0.1:{}",
            port
        );

        for stream in listener.incoming().flatten() {
            let permissions = self.permissions.clone();
            let sessions = self.sessions.clone();
            let node_id = self.node_id.clone();
            let node_address = self.node_address.clone();
            let mesh_auth_token = self.mesh_auth_token.clone();
            let peers = self.peers.clone();
            let task_dispatcher = self.task_dispatcher.clone();
            let metrics_collector = self.metrics_collector.clone();
            let store = self.store.clone();
            std::thread::spawn(move || {
                let server = RpcServer {
                    permissions,
                    sessions,
                    node_id,
                    node_address,
                    mesh_auth_token,
                    peers,
                    task_dispatcher,
                    metrics_collector,
                    store,
                };
                server.handle_ws_connection(stream);
            });
        }
        Ok(())
    }

    pub fn handle_ws_connection(&self, mut stream: TcpStream) {
        let mut buf_reader = BufReader::new(stream.try_clone().unwrap());
        let mut key = String::new();

        loop {
            let mut line = String::new();
            if buf_reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                break;
            }
            if trimmed.to_lowercase().starts_with("sec-websocket-key:") {
                key = trimmed["sec-websocket-key:".len()..].trim().to_string();
            }
        }

        if key.is_empty() {
            return;
        }

        let magic = format!("{}258EAFA5-E914-47DA-95CA-C5AB0DC85B11", key);
        let accept_val = base64_encode(&sha1_digest(magic.as_bytes()));

        let handshake_response = format!(
            "HTTP/1.1 101 Switching Protocols\r\n\
             Upgrade: websocket\r\n\
             Connection: Upgrade\r\n\
             Sec-WebSocket-Accept: {}\r\n\r\n",
            accept_val
        );

        if stream.write_all(handshake_response.as_bytes()).is_err() {
            return;
        }
        let _ = stream.flush();

        while let Ok(Some(req_str)) = read_ws_frame(&mut buf_reader) {
            let (session_id_opt, resp_str, live_events) = self.dispatch_request_ws(&req_str);

            for ev in live_events {
                let ev_notice = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "knc_event",
                    "params": {
                        "session_id": session_id_opt.as_deref().unwrap_or("default"),
                        "event": ev
                    }
                });
                let _ = write_ws_frame(&mut stream, &ev_notice.to_string());
            }

            if write_ws_frame(&mut stream, &resp_str).is_err() {
                break;
            }
        }
    }
}

/// Configuration for Mesh Gossip Protocol and Auto-Healing Eviction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeshGossipConfig {
    pub gossip_interval_secs: u64,
    pub stale_timeout_secs: u64,
    pub eviction_timeout_secs: u64,
    pub ping_timeout_ms: u64,
}

impl Default for MeshGossipConfig {
    fn default() -> Self {
        Self {
            gossip_interval_secs: 2,
            stale_timeout_secs: 5,
            eviction_timeout_secs: 15,
            ping_timeout_ms: 1000,
        }
    }
}

/// Mesh Gossip Worker for periodic heartbeats, latency measurement, and auto-healing eviction
pub struct MeshGossipWorker {
    pub server: Arc<RpcServer>,
    pub config: MeshGossipConfig,
}

impl MeshGossipWorker {
    pub fn new(server: Arc<RpcServer>, config: MeshGossipConfig) -> Self {
        Self { server, config }
    }

    /// Performs one gossip cycle across all registered peers in the topology.
    /// Returns a tuple `(active_count, stale_count, evicted_count)`.
    pub fn run_gossip_cycle(&self) -> (usize, usize, usize) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let peers_snapshot: Vec<MeshPeer> = {
            let peers = self.server.peers.lock().unwrap_or_else(|e| e.into_inner());
            peers.values().cloned().collect()
        };

        if peers_snapshot.is_empty() {
            return (0, 0, 0);
        }

        let timestamp = now;
        let signature = self.server.mesh_auth_token.as_ref().map(|secret| {
            hmac_sha256(
                secret.as_bytes(),
                format!("{}:{}", timestamp, self.server.node_id).as_bytes(),
            )
        });

        let req_payload = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_mesh_ping",
            "params": {
                "sender_node_id": self.server.node_id,
                "sender_address": self.server.node_address,
                "timestamp": timestamp,
                "mesh_auth_signature": signature
            },
            "id": 1
        })
        .to_string();

        let (tx, rx) = crossbeam_channel::unbounded();

        for peer in peers_snapshot {
            let server = self.server.clone();
            let timeout_ms = self.config.ping_timeout_ms;
            let tx = tx.clone();
            let payload = req_payload.clone();

            std::thread::spawn(move || {
                let start = std::time::Instant::now();
                let ping_res =
                    server.send_rpc_to_node_with_timeout(&peer.address, &payload, timeout_ms);
                let latency = start.elapsed().as_millis() as u64;
                let is_success = ping_res
                    .ok()
                    .and_then(|s| serde_json::from_str::<JsonRpcResponse>(&s).ok())
                    .map(|v| v.error.is_none() && v.result.is_some())
                    .unwrap_or(false);

                let _ = tx.send((peer, is_success, latency));
            });
        }

        drop(tx);

        let mut active = 0;
        let mut stale = 0;
        let mut evicted = 0;

        let recv_timeout = std::time::Duration::from_millis(self.config.ping_timeout_ms + 500);

        while let Ok((peer, is_success, latency)) = rx.recv_timeout(recv_timeout) {
            let elapsed_since_last_seen = now.saturating_sub(peer.last_seen);
            let mut peers = self.server.peers.lock().unwrap_or_else(|e| e.into_inner());

            if let Some(target_peer) = peers.get_mut(&peer.node_id) {
                if is_success {
                    target_peer.last_seen = now;
                    target_peer.latency_ms = latency;
                    target_peer.status = "Active".to_string();
                    active += 1;
                } else if elapsed_since_last_seen >= self.config.eviction_timeout_secs {
                    target_peer.status = "Evicted".to_string();
                    evicted += 1;
                } else if elapsed_since_last_seen >= self.config.stale_timeout_secs {
                    target_peer.status = "Stale".to_string();
                    stale += 1;
                } else {
                    active += 1;
                }
            }
        }

        (active, stale, evicted)
    }

    /// Evaluates peer timeouts without network calls (useful for simulated time or deterministic testing).
    pub fn evaluate_timeouts(&self, simulated_now: u64) -> (usize, usize, usize) {
        let mut peers = self.server.peers.lock().unwrap_or_else(|e| e.into_inner());
        let mut active = 0;
        let mut stale = 0;
        let mut evicted = 0;

        for peer in peers.values_mut() {
            let elapsed = simulated_now.saturating_sub(peer.last_seen);
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

    /// Evicts (removes) all peers currently marked as "Evicted" from the routing table.
    pub fn prune_evicted(&self) -> usize {
        let mut peers = self.server.peers.lock().unwrap_or_else(|e| e.into_inner());
        let before = peers.len();
        peers.retain(|_, peer| peer.status != "Evicted");
        before - peers.len()
    }
}

/// Spawns a background thread running the MeshGossipWorker loop with shutdown signal support.
pub fn start_gossip_worker(
    server: Arc<RpcServer>,
    config: MeshGossipConfig,
    shutdown_signal: Arc<std::sync::atomic::AtomicBool>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let worker = MeshGossipWorker::new(server, config.clone());
        while !shutdown_signal.load(std::sync::atomic::Ordering::Relaxed) {
            std::thread::sleep(std::time::Duration::from_secs(config.gossip_interval_secs));
            if shutdown_signal.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            worker.run_gossip_cycle();
            worker.prune_evicted();
        }
    })
}

#[allow(clippy::needless_range_loop)]
pub fn sha1_digest(data: &[u8]) -> [u8; 20] {
    let mut h0: u32 = 0x67452301;
    let mut h1: u32 = 0xEFCDAB89;
    let mut h2: u32 = 0x98BADCFE;
    let mut h3: u32 = 0x10325476;
    let mut h4: u32 = 0xC3D2E1F0;

    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 80];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..80 {
            w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
        }

        let mut a = h0;
        let mut b = h1;
        let mut c = h2;
        let mut d = h3;
        let mut e = h4;

        for i in 0..80 {
            let (f, k) = match i {
                0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                _ => (b ^ c ^ d, 0xCA62C1D6u32),
            };
            let temp = a
                .rotate_left(5)
                .wrapping_add(f)
                .wrapping_add(e)
                .wrapping_add(k)
                .wrapping_add(w[i]);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temp;
        }

        h0 = h0.wrapping_add(a);
        h1 = h1.wrapping_add(b);
        h2 = h2.wrapping_add(c);
        h3 = h3.wrapping_add(d);
        h4 = h4.wrapping_add(e);
    }

    let mut out = [0u8; 20];
    out[0..4].copy_from_slice(&h0.to_be_bytes());
    out[4..8].copy_from_slice(&h1.to_be_bytes());
    out[8..12].copy_from_slice(&h2.to_be_bytes());
    out[12..16].copy_from_slice(&h3.to_be_bytes());
    out[16..20].copy_from_slice(&h4.to_be_bytes());
    out
}

pub fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(CHARSET[((triple >> 18) & 63) as usize] as char);
        out.push(CHARSET[((triple >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(CHARSET[((triple >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(CHARSET[(triple & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

pub fn read_ws_frame<R: std::io::Read>(reader: &mut R) -> std::io::Result<Option<String>> {
    let mut header = [0u8; 2];
    if reader.read_exact(&mut header).is_err() {
        return Ok(None);
    }

    let opcode = header[0] & 0x0F;
    if opcode == 0x8 {
        return Ok(None);
    }

    let masked = (header[1] & 0x80) != 0;
    let mut payload_len = (header[1] & 0x7F) as usize;

    if payload_len == 126 {
        let mut extended = [0u8; 2];
        reader.read_exact(&mut extended)?;
        payload_len = u16::from_be_bytes(extended) as usize;
    } else if payload_len == 127 {
        let mut extended = [0u8; 8];
        reader.read_exact(&mut extended)?;
        payload_len = u64::from_be_bytes(extended) as usize;
    }

    let mask = if masked {
        let mut m = [0u8; 4];
        reader.read_exact(&mut m)?;
        Some(m)
    } else {
        None
    };

    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload)?;

    if let Some(m) = mask {
        for i in 0..payload_len {
            payload[i] ^= m[i % 4];
        }
    }

    let text = String::from_utf8(payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(text))
}

pub fn write_ws_frame<W: std::io::Write>(writer: &mut W, text: &str) -> std::io::Result<()> {
    let bytes = text.as_bytes();
    let len = bytes.len();

    let mut frame = Vec::new();
    frame.push(0x81); // FIN = 1, Opcode = 1 (Text)

    if len <= 125 {
        frame.push(len as u8);
    } else if len <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    frame.extend_from_slice(bytes);
    writer.write_all(&frame)?;
    writer.flush()
}

pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut res = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        res |= x ^ y;
    }
    res == 0
}

pub fn hex_encode(data: &[u8]) -> String {
    let mut s = String::with_capacity(data.len() * 2);
    for &b in data {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

#[allow(clippy::needless_range_loop)]
pub fn sha256_digest(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef4a3f7,
        0xc67178f2,
    ];

    let mut msg = data.to_vec();
    let bit_len = (data.len() as u64) * 8;
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0x00);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut h_var = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = h_var
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            h_var = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(h_var);
    }

    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..(i + 1) * 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

pub fn hmac_sha256(key: &[u8], message: &[u8]) -> String {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        let hash = sha256_digest(key);
        k[..32].copy_from_slice(&hash);
    } else {
        k[..key.len()].copy_from_slice(key);
    }

    let mut ipad = [0x36u8; 64];
    let mut opad = [0x5cu8; 64];
    for i in 0..64 {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }

    let mut inner = ipad.to_vec();
    inner.extend_from_slice(message);
    let inner_hash = sha256_digest(&inner);

    let mut outer = opad.to_vec();
    outer.extend_from_slice(&inner_hash);
    let outer_hash = sha256_digest(&outer);

    hex_encode(&outer_hash)
}

// =============================================================================
// Sprint 319: Distributed Task Queue & Mesh Work-Stealing Engine
// =============================================================================

/// Lifecycle state of a dispatched task.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TaskStatus {
    /// Submitted, not yet picked up by any worker.
    Queued,
    /// Actively being executed by a worker node.
    Running,
    /// Finished successfully; `result` is populated.
    Completed,
    /// Cancelled by the submitter before or during execution.
    Cancelled,
    /// The worker returned an error; `result` contains the fault message.
    Failed,
}

/// A single entry in the distributed work pool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEntry {
    /// Globally unique task identifier (monotonic u64 formatted as decimal string).
    pub task_id: String,
    /// The JSON-AST program to execute.
    pub ast: Node,
    /// Lower value = higher priority; 0 is highest, 255 is lowest.
    pub priority: u8,
    /// Current lifecycle state.
    pub status: TaskStatus,
    /// Node ID that picked up this task via work-stealing, if any.
    pub worker_node_id: Option<String>,
    /// Serialised execution result or fault string once complete.
    pub result: Option<Value>,
}

/// Thread-safe distributed task queue with cooperative work-stealing.
///
/// Internally backed by a `Mutex<HashMap>` keyed by `task_id`.  All operations
/// are O(n) in the worst case but task pools in a mesh are expected to be small
/// (hundreds, not millions of concurrent tasks).
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
    /// Create an empty dispatcher.
    pub fn new() -> Self {
        Self {
            tasks: Mutex::new(HashMap::new()),
            counter: AtomicU64::new(1),
        }
    }

    /// Submit a new task.  Returns the assigned `task_id`.
    pub fn submit(&self, ast: Node, priority: u8) -> String {
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
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        tasks.insert(task_id.clone(), entry);
        task_id
    }

    /// Return a snapshot of a task entry, or `None` if unknown.
    pub fn status(&self, task_id: &str) -> Option<TaskEntry> {
        let tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        tasks.get(task_id).cloned()
    }

    /// Attempt to cancel a task.  Only `Queued` tasks can be cancelled.
    /// Returns `true` if the task was transitioned to `Cancelled`.
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

    /// Mark a task as `Running` and record which worker claimed it.
    /// Returns `false` if the task is not in `Queued` state.
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

    /// Record a successful result for a task.
    pub fn complete(&self, task_id: &str, result: Value) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = tasks.get_mut(task_id) {
            entry.status = TaskStatus::Completed;
            entry.result = Some(result);
        }
    }

    /// Record a failure result for a task.
    pub fn fail(&self, task_id: &str, error_msg: &str) {
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = tasks.get_mut(task_id) {
            entry.status = TaskStatus::Failed;
            entry.result = Some(Value::String(error_msg.to_string()));
        }
    }

    /// Work-stealing: atomically claims up to `max_tasks` `Queued` tasks for
    /// `worker_node_id`, ordered by ascending priority (lowest value first).
    /// Returns clones of the claimed entries so the caller can execute them.
    pub fn steal(&self, max_tasks: usize, worker_node_id: &str) -> Vec<TaskEntry> {
        if max_tasks == 0 {
            return Vec::new();
        }
        let mut tasks = self.tasks.lock().unwrap_or_else(|e| e.into_inner());

        // Collect queued task IDs sorted by priority.
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

    /// Return the total number of tasks in each state as a JSON object.
    /// Useful for monitoring and load-balancing decisions.
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

// =============================================================================
// Sprint 320: Cluster Metrics & Adaptive Work-Stealing Protocol
// =============================================================================

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

/// Thread-safe collector for node metrics, supporting simulated overrides for testing.
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
    /// Create a new default metrics collector.
    pub fn new() -> Self {
        Self {
            simulated_cpu_load: Mutex::new(None),
            simulated_memory_used: Mutex::new(None),
            simulated_memory_total: Mutex::new(None),
        }
    }

    /// Override simulated CPU load percentage (0.0..100.0) for testing load throttling.
    pub fn set_simulated_cpu_load(&self, load: Option<f64>) {
        let mut guard = self
            .simulated_cpu_load
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *guard = load;
    }

    /// Override simulated memory usage (used_bytes, total_bytes) for testing.
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

    /// Sample or return current metrics given task queue depth stats.
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
            .unwrap_or(134_217_728); // 128 MB default baseline

        let mem_total = self
            .simulated_memory_total
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .unwrap_or(8_589_934_592); // 8 GB default baseline

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

// =============================================================================
// Sprint 321: Distributed CRDT Key-Value Storage & State Sync
// =============================================================================

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

    /// Write or update a key with LWW conflict resolution.
    /// Returns `true` if the entry was written/updated, or `false` if existing entry is newer.
    pub fn put(&self, key: &str, value: Value, timestamp: u64, writer_id: &str) -> bool {
        let new_entry = CrdtEntry {
            key: key.to_string(),
            value,
            timestamp,
            writer_id: writer_id.to_string(),
        };

        let mut store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
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

    /// Read CRDT entry for a key.
    pub fn get(&self, key: &str) -> Option<CrdtEntry> {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        store.get(key).cloned()
    }

    /// Merge an incoming vector of CRDT entries using LWW semantics.
    /// Returns the number of entries updated or inserted.
    pub fn sync(&self, incoming: Vec<CrdtEntry>) -> usize {
        let mut store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let mut updated = 0;
        for entry in incoming {
            let key = entry.key.clone();
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

    /// Export a full snapshot vector of stored CRDT entries.
    pub fn dump_entries(&self) -> Vec<CrdtEntry> {
        let store = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        store.values().cloned().collect()
    }
}
