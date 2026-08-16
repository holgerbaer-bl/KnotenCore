use knoten_core_types::ast::{IsolateQuota, Node};
use knoten_core_types::opcode::OpCode;
use serde_json::Value;

use crate::executor::RelType;
use crate::optimizer::optimize;
use crate::rpc::types::{JsonRpcResponse, validate_param_string_len};
use crate::validator::Validator;
use crate::vm::compiler::Compiler;

fn count_ast_nodes(node: &Node) -> usize {
    1 + match node {
        Node::Block(nodes) => nodes.iter().map(count_ast_nodes).sum(),
        Node::Assign(_, val) => count_ast_nodes(val),
        Node::Store { value, .. } => count_ast_nodes(value),
        Node::Add(l, r)
        | Node::Sub(l, r)
        | Node::Mul(l, r)
        | Node::Div(l, r)
        | Node::Eq(l, r)
        | Node::Lt(l, r)
        | Node::Gt(l, r)
        | Node::Lte(l, r)
        | Node::Gte(l, r)
        | Node::NotEq(l, r)
        | Node::And(l, r)
        | Node::Or(l, r) => count_ast_nodes(l) + count_ast_nodes(r),
        Node::Not(n) => count_ast_nodes(n),
        Node::If(cond, then_b, else_b) => {
            count_ast_nodes(cond)
                + count_ast_nodes(then_b)
                + else_b.as_ref().map_or(0, |b| count_ast_nodes(b))
        }
        Node::While(cond, body) => count_ast_nodes(cond) + count_ast_nodes(body),
        Node::FnDef(_, _, body) => count_ast_nodes(body),
        Node::Call(_, args) => args.iter().map(count_ast_nodes).sum(),
        _ => 0,
    }
}

impl super::super::RpcServer {
    pub fn parse_session_id(&self, params: &Value) -> Result<String, String> {
        let raw = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("default_session");
        validate_param_string_len(raw)?;
        Ok(raw.to_string())
    }

    pub fn extract_ast_node(&self, params: &Value) -> Result<Node, String> {
        let ast_val = params.get("ast").ok_or("Missing 'ast' parameter")?;
        serde_json::from_value(ast_val.clone())
            .map_err(|e| format!("Invalid AST node payload: {}", e))
    }

    pub fn handle_compile(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        if params.get("session_id").is_some()
            && let Err(err) = self.parse_session_id(&params)
        {
            return JsonRpcResponse::error(id, -32602, err);
        }

        let node = match self.extract_ast_node(&params) {
            Ok(n) => n,
            Err(err) => return JsonRpcResponse::error(id, -32602, err),
        };

        let opt_node = optimize(node);
        let mut validator = Validator::new();
        if let Err(errs) = validator.validate(&opt_node) {
            return JsonRpcResponse::error(
                id,
                -32602,
                format!("Validation Error: {}", errs.join("; ")),
            );
        }

        if let Some(q) = params
            .get("quota")
            .and_then(|v| serde_json::from_value::<IsolateQuota>(v.clone()).ok())
        {
            let count = count_ast_nodes(&opt_node);
            if count > q.max_instructions as usize {
                return JsonRpcResponse::error(
                    id,
                    -32000,
                    format!(
                        "Quota Exceeded: AST node count quota exceeded: {} nodes > max {}",
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

    pub fn handle_execute(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let session_id = match self.parse_session_id(&params) {
            Ok(s) => s,
            Err(err) => return JsonRpcResponse::error(id, -32602, err),
        };

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
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(existing_session) = sessions.get(&session_id)
                && !existing_session.instructions.is_empty()
            {
                (
                    existing_session.instructions.clone(),
                    existing_session.constants.clone(),
                )
            } else {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    "Params must contain either 'ast' or ('instructions' and 'constants')",
                );
            }
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

        let events_container = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events_container.clone();
        session.vm.set_event_hook(std::sync::Arc::new(move |ev| {
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
                let msg = if err.to_uppercase().contains("QUOTA") && !err.contains("Quota Exceeded")
                {
                    format!("Quota Exceeded: {}", err)
                } else {
                    err.to_string()
                };
                JsonRpcResponse::error(id, -32000, msg)
            }
        }
    }

    pub fn handle_yield_resume(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let session_id = match self.parse_session_id(&params) {
            Ok(s) => s,
            Err(err) => return JsonRpcResponse::error(id, -32602, err),
        };

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

        let resume_val = params
            .get("resume_value")
            .and_then(|v| serde_json::from_value::<RelType>(v.clone()).ok());

        let events_container = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        let events_clone = events_container.clone();
        session.vm.set_event_hook(std::sync::Arc::new(move |ev| {
            if let Ok(mut lock) = events_clone.lock() {
                lock.push(ev);
            }
        }));

        if let Some(res_val) = resume_val {
            session.vm.stack.push(res_val);
        }

        let exec_result = session.vm.resume(
            &session.instructions,
            &session.constants,
            &self.permissions,
            None,
        );

        if let Ok(collected) = events_container.lock() {
            session.events.extend(collected.clone());
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
            Err(err) => JsonRpcResponse::error(id, -32000, format!("Resume Error: {}", err)),
        }
    }

    pub fn handle_inspect_state(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let session_id = match self.parse_session_id(&params) {
            Ok(s) => s,
            Err(err) => return JsonRpcResponse::error(id, -32602, err),
        };

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
        let resp_val = serde_json::json!({
            "status": "ok",
            "session_id": session_id,
            "is_yielded": session.vm.is_yielded(),
            "globals_count": session.vm.globals.len(),
            "inspector": session.vm.inspect(),
            "ip": vm_state.ip,
            "base_pointer": vm_state.base_pointer,
            "stack_depth": vm_state.stack.len(),
            "stack": vm_state.stack,
            "globals": vm_state.globals,
            "call_frames": vm_state.frames,
            "crypto_state_hash": vm_state.crypto_state_hash,
            "execution_state": format!("{:?}", session.vm.execution_state()),
            "events": session.events
        });

        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_isolate_reload(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(s) => match validate_param_string_len(s) {
                Ok(_) => s.to_string(),
                Err(err) => return JsonRpcResponse::error(id, -32602, err),
            },
            None => return JsonRpcResponse::error(id, -32602, "Missing 'session_id' parameter"),
        };

        let ast_val = match params.get("ast") {
            Some(v) => v,
            None => return JsonRpcResponse::error(id, -32602, "Missing 'ast' parameter"),
        };

        let new_ast: Node = match serde_json::from_value(ast_val.clone()) {
            Ok(ast) => ast,
            Err(e) => {
                return JsonRpcResponse::error(id, -32602, format!("Invalid AST structure: {}", e));
            }
        };

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let session = match sessions.get_mut(&session_id) {
            Some(s) => s,
            None => {
                return JsonRpcResponse::error(
                    id,
                    -32602,
                    format!("Session not found: {}", session_id),
                );
            }
        };

        match session.hot_reload_code(&new_ast) {
            Ok(report) => {
                let resp_val = serde_json::json!({
                    "status": "ok",
                    "report": report
                });
                JsonRpcResponse::success(id, resp_val)
            }
            Err(err) => {
                let code =
                    if err.contains("ERR_HMR_ACTIVE_EXECUTION") || err.contains("Unauthorized") {
                        -32001
                    } else {
                        -32602
                    };
                JsonRpcResponse::error(id, code, err)
            }
        }
    }
}
