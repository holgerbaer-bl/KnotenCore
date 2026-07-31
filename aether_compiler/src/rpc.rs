// Sprint 310: Headless JSON-RPC 2.0 Server Engine & Agentic Transport Protocol
//
// Exposes KnotenCore AST compilation, VM execution, yield/resume suspension,
// event streaming hooks, and state inspection via JSON-RPC 2.0.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use knoten_core_types::ast::Node;
use knoten_core_types::opcode::OpCode;

use crate::executor::{AgentPermissions, RelType};
use crate::optimizer::optimize;
use crate::validator::Validator;
use crate::vm::compiler::Compiler;
use crate::vm::machine::{VM, VmEvent};

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

/// JSON-RPC 2.0 Server Handler
pub struct RpcServer {
    pub permissions: AgentPermissions,
    pub sessions: Arc<Mutex<HashMap<String, RpcSession>>>,
}

impl RpcServer {
    pub fn new(permissions: AgentPermissions) -> Self {
        Self {
            permissions,
            sessions: Arc::new(Mutex::new(HashMap::new())),
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

        let mut sessions = self.sessions.lock().unwrap();
        let session = sessions.entry(session_id.clone()).or_default();

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
            Err(err) => JsonRpcResponse::error(id, -32603, format!("Runtime Fault: {}", err)),
        }
    }

    fn handle_yield_resume(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let mut sessions = self.sessions.lock().unwrap();
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
            Err(err) => JsonRpcResponse::error(id, -32603, format!("Resume Fault: {}", err)),
        }
    }

    fn handle_inspect_state(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let sessions = self.sessions.lock().unwrap();
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
}
