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

use knoten_core_types::ast::{IsolateQuota, Node};
use knoten_core_types::opcode::OpCode;

use crate::executor::{AgentPermissions, RelType};
use crate::optimizer::optimize;
use crate::validator::Validator;
use crate::vm::compiler::Compiler;
use crate::vm::machine::{VM, VmEvent, VmExecutionState};

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
            "knc_agent_handshake" => self.handle_agent_handshake(req.id, req.params),
            "knc_agent_snapshot" => self.handle_agent_snapshot(req.id, req.params),
            "knc_agent_restore" => self.handle_agent_restore(req.id, req.params),
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

        let mut sessions = self.sessions.lock().unwrap();
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

    fn handle_agent_handshake(&self, id: Option<Value>, _params: Value) -> JsonRpcResponse {
        let default_quota = IsolateQuota::default();
        let resp_val = serde_json::json!({
            "status": "ok",
            "protocol_version": "v2.11.0-agent",
            "engine": "KnotenCore",
            "capabilities": {
                "jsonrpc": true,
                "websocket": true,
                "isolate_quotas": true,
                "async_yield": true,
                "state_snapshots": true
            },
            "default_quota": default_quota
        });
        JsonRpcResponse::success(id, resp_val)
    }

    fn handle_agent_snapshot(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
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

        let mut sessions = self.sessions.lock().unwrap();
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
            let sessions = self.sessions.lock().unwrap();
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
            std::thread::spawn(move || {
                let server = RpcServer {
                    permissions,
                    sessions,
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
