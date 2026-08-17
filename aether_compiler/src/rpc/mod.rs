use std::collections::{HashMap, HashSet};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde_json::Value;

use crate::crypto_ed25519::Ed25519KeyPair;
use crate::executor::AgentPermissions;
use crate::vm::machine::VmEvent;

pub mod auth;
pub mod handlers;
pub mod types;

pub use auth::*;
pub use handlers::*;
pub use types::*;

/// JSON-RPC 2.0 Server Handler & Mesh Node Context.
pub struct RpcServer {
    pub permissions: AgentPermissions,
    pub sessions: Arc<Mutex<HashMap<String, RpcSession>>>,
    pub node_id: String,
    pub node_address: String,
    pub mesh_auth_token: Option<String>,
    pub peers: Arc<Mutex<HashMap<String, MeshPeer>>>,
    pub task_dispatcher: Arc<TaskDispatcher>,
    pub metrics_collector: Arc<MetricsCollector>,
    pub store: Arc<MeshKvStore>,
    pub swarm_governance: Arc<SwarmGovernance>,
    pub ed25519_keypair: Arc<Mutex<Ed25519KeyPair>>,
    pub verified_peer_keys: Arc<Mutex<HashMap<String, String>>>,
    pub revoked_peer_keys: Arc<Mutex<HashSet<String>>>,
    pub revoked_keys_path: Arc<Mutex<Option<PathBuf>>>,
    pub used_nonces: Arc<Mutex<NonceCache>>,
    pub zero_trust_mode: Arc<Mutex<bool>>,
}

impl Default for RpcServer {
    fn default() -> Self {
        Self::new(AgentPermissions::default())
    }
}

impl RpcServer {
    pub fn new(permissions: AgentPermissions) -> Self {
        Self::with_mesh(permissions, "node-local", "127.0.0.1:0", None)
    }

    pub fn with_mesh(
        permissions: AgentPermissions,
        node_id: impl Into<String>,
        node_address: impl Into<String>,
        mesh_auth_token: Option<String>,
    ) -> Self {
        let keypair = Ed25519KeyPair::generate();
        let server = Self {
            permissions,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            node_id: node_id.into(),
            node_address: node_address.into(),
            mesh_auth_token,
            peers: Arc::new(Mutex::new(HashMap::new())),
            task_dispatcher: Arc::new(TaskDispatcher::new()),
            metrics_collector: Arc::new(MetricsCollector::new()),
            store: Arc::new(MeshKvStore::new()),
            swarm_governance: Arc::new(SwarmGovernance::new()),
            ed25519_keypair: Arc::new(Mutex::new(keypair)),
            verified_peer_keys: Arc::new(Mutex::new(HashMap::new())),
            revoked_peer_keys: Arc::new(Mutex::new(HashSet::new())),
            revoked_keys_path: Arc::new(Mutex::new(Some(PathBuf::from("revoked_keys.json")))),
            used_nonces: Arc::new(Mutex::new(NonceCache::new())),
            zero_trust_mode: Arc::new(Mutex::new(false)),
        };
        server.load_revoked_keys();
        server
    }

    pub fn is_zero_trust(&self) -> bool {
        *self
            .zero_trust_mode
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn set_zero_trust(&self, enabled: bool) {
        let mut mode = self
            .zero_trust_mode
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *mode = enabled;
    }

    pub fn enable_zero_trust(&self) {
        self.set_zero_trust(true);
    }

    pub fn sign_envelope(&self, nonce: &str, timestamp: u64) -> (String, String) {
        let kp = self
            .ed25519_keypair
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let pubkey = kp.public_key_hex();
        let msg = format!("{}:{}:{}", timestamp, nonce, self.node_id);
        let sig = kp.sign_hex(msg.as_bytes());
        (pubkey, sig)
    }

    pub fn public_key_hex(&self) -> String {
        let kp = self
            .ed25519_keypair
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        kp.public_key_hex()
    }

    pub fn rotate_key(&self) -> (String, String) {
        let mut kp = self
            .ed25519_keypair
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let old_pub = kp.public_key_hex();
        *kp = Ed25519KeyPair::generate();
        let new_pub = kp.public_key_hex();
        (old_pub, new_pub)
    }

    pub fn set_revoked_keys_path(&self, path: Option<PathBuf>) {
        let mut p = self
            .revoked_keys_path
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *p = path;
    }

    pub fn load_revoked_keys(&self) {
        let path_opt = self
            .revoked_keys_path
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        if let Some(path) = path_opt
            && path.exists()
            && let Ok(data) = std::fs::read_to_string(&path)
            && let Ok(keys) = serde_json::from_str::<HashSet<String>>(&data)
        {
            let mut revoked = self
                .revoked_peer_keys
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            *revoked = keys;
        }
    }

    pub fn save_revoked_keys(&self) {
        let path_opt = self
            .revoked_keys_path
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        if let Some(path) = path_opt {
            let revoked = self
                .revoked_peer_keys
                .lock()
                .unwrap_or_else(|e| e.into_inner());

            if let Ok(json_data) = serde_json::to_string_pretty(&*revoked) {
                if let Some(parent) = path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(path, json_data);
            }
        }
    }

    pub fn is_peer_key_revoked(&self, pubkey_hex: &str) -> bool {
        let normalized = pubkey_hex.trim().to_lowercase();
        let revoked = self
            .revoked_peer_keys
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        revoked.contains(&normalized)
    }

    pub fn revoke_peer_key(&self, pubkey_hex: &str) {
        let normalized = pubkey_hex.trim().to_lowercase();
        {
            let mut revoked = self
                .revoked_peer_keys
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            revoked.insert(normalized.clone());
        }

        {
            let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            let verified = self
                .verified_peer_keys
                .lock()
                .unwrap_or_else(|e| e.into_inner());

            let node_to_remove = verified.iter().find_map(|(node_id, pk)| {
                if pk.to_lowercase() == normalized {
                    Some(node_id.clone())
                } else {
                    None
                }
            });

            if let Some(node_id) = node_to_remove {
                peers.remove(&node_id);
            }
        }
        self.save_revoked_keys();
    }

    pub fn dispatch_request(&self, request_raw: &str) -> String {
        if request_raw.len() > MAX_BODY_BYTES {
            let response = JsonRpcResponse::error(
                None,
                -32700,
                format!(
                    "Parse Error: Request payload size ({} bytes) exceeds MAX_BODY_BYTES limit ({} bytes)",
                    request_raw.len(),
                    MAX_BODY_BYTES
                ),
            );
            return serde_json::to_string(&response).unwrap_or_default();
        }

        let req: JsonRpcRequest = match serde_json::from_str(request_raw) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Invalid JSON-RPC 2.0 payload: {}", e),
                );
                return serde_json::to_string(&response).unwrap_or_default();
            }
        };

        if req.jsonrpc != "2.0" {
            let response =
                JsonRpcResponse::error(req.id, -32600, "Invalid jsonrpc version. Must be '2.0'");
            return serde_json::to_string(&response).unwrap_or_default();
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
            "knc_agent_teleport" => self.handle_agent_teleport(req.id, req.params),
            "knc_task_submit" => self.handle_task_submit(req.id, req.params),
            "knc_task_status" => self.handle_task_status(req.id, req.params),
            "knc_task_cancel" => self.handle_task_cancel(req.id, req.params),
            "knc_task_steal" => self.handle_task_steal(req.id, req.params),
            "knc_mesh_ping" => self.handle_mesh_ping(req.id, req.params),
            "knc_mesh_metrics" => self.handle_mesh_metrics(req.id, req.params),
            "knc_store_put" => self.handle_store_put(req.id, req.params),
            "knc_store_get" => self.handle_store_get(req.id, req.params),
            "knc_store_sync" => self.handle_store_sync(req.id, req.params),
            "knc_swarm_elect" => self.handle_swarm_elect(req.id, req.params),
            "knc_swarm_roles" => self.handle_swarm_roles(req.id, req.params),
            "knc_swarm_quorum" => self.handle_swarm_quorum(req.id, req.params),
            "knc_swarm_request_vote" => self.handle_swarm_request_vote(req.id, req.params),
            "knc_swarm_heartbeat" => self.handle_swarm_heartbeat(req.id, req.params),
            "knc_mesh_verify_peer" => self.handle_mesh_verify_peer(req.id, req.params),
            "knc_mesh_rotate_key" => self.handle_mesh_rotate_key(req.id, req.params),
            "knc_mesh_revoke_peer" => self.handle_mesh_revoke_peer(req.id, req.params),
            "knc_isolate_reload" => self.handle_isolate_reload(req.id, req.params),
            "knc_meaning_of_life" | "sys.meaning_of_life" => {
                self.handle_meaning_of_life(req.id, req.params)
            }
            _ => {
                JsonRpcResponse::error(req.id, -32601, format!("Method not found: {}", req.method))
            }
        };

        serde_json::to_string(&response).unwrap_or_default()
    }

    pub fn send_rpc_to_node(&self, address: &str, request_json: &str) -> Result<String, String> {
        let mut stream = TcpStream::connect(address)
            .map_err(|e| format!("Failed to connect to node at {}: {}", address, e))?;

        stream
            .write_all(request_json.as_bytes())
            .map_err(|e| format!("Failed to write payload to node {}: {}", address, e))?;
        stream
            .write_all(b"\n")
            .map_err(|e| format!("Failed to write newline to node {}: {}", address, e))?;

        let mut reader = BufReader::new(stream);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .map_err(|e| format!("Failed to read response from node {}: {}", address, e))?;

        Ok(response_line.trim().to_string())
    }

    pub fn dispatch_request_over_network(
        &self,
        address: &str,
        request_json: &str,
    ) -> Result<String, String> {
        if address == self.node_address || address == "127.0.0.1:0" {
            Ok(self.dispatch_request(request_json))
        } else {
            self.send_rpc_to_node(address, request_json)
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

    pub fn spawn_background_tcp_server(
        server: Arc<RpcServer>,
        port: u16,
    ) -> std::io::Result<(u16, std::thread::JoinHandle<()>)> {
        let listener = TcpListener::bind(("127.0.0.1", port))?;
        let bound_port = listener.local_addr()?.port();
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let s = Arc::clone(&server);
                std::thread::spawn(move || {
                    s.handle_connection(stream);
                });
            }
        });
        Ok((bound_port, handle))
    }

    fn handle_connection(&self, mut stream: TcpStream) {
        let cloned_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        };
        let mut reader = BufReader::new(cloned_stream);
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
            let swarm_governance = self.swarm_governance.clone();
            let ed25519_keypair = self.ed25519_keypair.clone();
            let verified_peer_keys = self.verified_peer_keys.clone();
            let revoked_peer_keys = self.revoked_peer_keys.clone();
            let revoked_keys_path = self.revoked_keys_path.clone();
            let used_nonces = self.used_nonces.clone();
            let zero_trust_mode = self.zero_trust_mode.clone();
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
                    swarm_governance,
                    ed25519_keypair,
                    verified_peer_keys,
                    revoked_peer_keys,
                    revoked_keys_path,
                    used_nonces,
                    zero_trust_mode,
                };
                server.handle_ws_connection(stream);
            });
        }
        Ok(())
    }

    pub fn handle_ws_connection(&self, mut stream: TcpStream) {
        let cloned_stream = match stream.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        };
        let mut reader = BufReader::new(cloned_stream);
        let mut line = String::new();

        while reader.read_line(&mut line).unwrap_or(0) > 0 {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                line.clear();
                continue;
            }

            if trimmed.to_uppercase().starts_with("GET ")
                || trimmed.to_uppercase().starts_with("POST ")
                || trimmed.to_lowercase().contains("upgrade: websocket")
            {
                let mut sec_websocket_key = None;
                let mut headers = vec![line.clone()];
                loop {
                    let mut h_line = String::new();
                    if reader.read_line(&mut h_line).unwrap_or(0) == 0 || h_line.trim().is_empty() {
                        break;
                    }
                    headers.push(h_line);
                }
                for hdr in &headers {
                    let lower = hdr.to_lowercase();
                    if lower.starts_with("sec-websocket-key:") {
                        sec_websocket_key = hdr.split(':').nth(1).map(|v| v.trim().to_string());
                        break;
                    }
                }

                if let Some(key) = sec_websocket_key {
                    let accept_key = compute_websocket_accept_key(&key);
                    let handshake_response = format!(
                        "HTTP/1.1 101 Switching Protocols\r\n\
                         Upgrade: websocket\r\n\
                         Connection: Upgrade\r\n\
                         Sec-WebSocket-Accept: {}\r\n\r\n",
                        accept_key
                    );
                    let _ = stream.write_all(handshake_response.as_bytes());
                    self.handle_websocket_frames(stream);
                    return;
                }
            }

            let response = self.dispatch_request(trimmed);
            let _ = writeln!(stream, "{}", response);
            line.clear();
        }
    }

    fn handle_websocket_frames(&self, mut stream: TcpStream) {
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        loop {
            let mut header = [0u8; 2];
            if reader.read_exact(&mut header).is_err() {
                break;
            }

            let _fin = (header[0] & 0x80) != 0;
            let opcode = header[0] & 0x0F;
            if opcode == 0x08 {
                break;
            }

            let is_masked = (header[1] & 0x80) != 0;
            let mut payload_len = (header[1] & 0x7F) as usize;

            if payload_len == 126 {
                let mut len_bytes = [0u8; 2];
                if reader.read_exact(&mut len_bytes).is_err() {
                    break;
                }
                payload_len = u16::from_be_bytes(len_bytes) as usize;
            } else if payload_len == 127 {
                let mut len_bytes = [0u8; 8];
                if reader.read_exact(&mut len_bytes).is_err() {
                    break;
                }
                payload_len = u64::from_be_bytes(len_bytes) as usize;
            }

            if payload_len > MAX_WS_PAYLOAD {
                break;
            }

            let mut mask_key = [0u8; 4];
            if is_masked && reader.read_exact(&mut mask_key).is_err() {
                break;
            }

            let mut payload = vec![0u8; payload_len];
            if reader.read_exact(&mut payload).is_err() {
                break;
            }

            if is_masked {
                for (i, byte) in payload.iter_mut().enumerate() {
                    *byte ^= mask_key[i % 4];
                }
            }

            if let Ok(request_str) = String::from_utf8(payload) {
                let (session_id, response_json, events) =
                    self.dispatch_request_ws(request_str.trim());

                let mut ws_payload = serde_json::from_str::<Value>(&response_json)
                    .unwrap_or_else(|_| serde_json::json!({ "result": response_json }));
                if let Value::Object(ref mut map) = ws_payload {
                    map.insert("events".to_string(), serde_json::json!(events));
                    if let Some(sid) = session_id {
                        map.insert("session_id".to_string(), serde_json::Value::String(sid));
                    }
                }

                let ws_response_bytes = serde_json::to_vec(&ws_payload).unwrap_or_default();
                if write_websocket_text_frame(&mut stream, &ws_response_bytes).is_err() {
                    break;
                }
            }
        }
    }
}

pub fn compute_websocket_accept_key(key: &str) -> String {
    use ring::digest::{SHA1_FOR_LEGACY_USE_ONLY, digest};
    let guid = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";
    let combined = format!("{}{}", key.trim(), guid);
    let digest_result = digest(&SHA1_FOR_LEGACY_USE_ONLY, combined.as_bytes());
    base64_encode(digest_result.as_ref())
}

pub fn base64_encode(input: &[u8]) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    let mut i = 0;

    while i < input.len() {
        let b0 = input[i] as usize;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as usize
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as usize
        } else {
            0
        };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        result.push(CHARSET[(triple >> 18) & 63] as char);
        result.push(CHARSET[(triple >> 12) & 63] as char);

        if i + 1 < input.len() {
            result.push(CHARSET[(triple >> 6) & 63] as char);
        } else {
            result.push('=');
        }

        if i + 2 < input.len() {
            result.push(CHARSET[triple & 63] as char);
        } else {
            result.push('=');
        }

        i += 3;
    }

    result
}

pub fn write_websocket_text_frame(stream: &mut TcpStream, payload: &[u8]) -> std::io::Result<()> {
    let mut frame = Vec::new();
    frame.push(0x81);

    let len = payload.len();
    if len <= 125 {
        frame.push(len as u8);
    } else if len <= 65535 {
        frame.push(126);
        frame.extend_from_slice(&(len as u16).to_be_bytes());
    } else {
        frame.push(127);
        frame.extend_from_slice(&(len as u64).to_be_bytes());
    }

    frame.extend_from_slice(payload);
    stream.write_all(&frame)?;
    stream.flush()?;
    Ok(())
}

pub fn sha1_digest(input: &[u8]) -> Vec<u8> {
    use ring::digest::{SHA1_FOR_LEGACY_USE_ONLY, digest};
    digest(&SHA1_FOR_LEGACY_USE_ONLY, input).as_ref().to_vec()
}

pub fn read_ws_frame<R: Read>(reader: &mut R) -> std::io::Result<Option<String>> {
    let mut header = [0u8; 2];
    if reader.read_exact(&mut header).is_err() {
        return Ok(None);
    }
    let _fin = (header[0] & 0x80) != 0;
    let opcode = header[0] & 0x0F;
    if opcode == 0x08 {
        return Ok(None);
    }
    let is_masked = (header[1] & 0x80) != 0;
    let mut payload_len = (header[1] & 0x7F) as usize;

    if payload_len == 126 {
        let mut len_bytes = [0u8; 2];
        reader.read_exact(&mut len_bytes)?;
        payload_len = u16::from_be_bytes(len_bytes) as usize;
    } else if payload_len == 127 {
        let mut len_bytes = [0u8; 8];
        reader.read_exact(&mut len_bytes)?;
        payload_len = u64::from_be_bytes(len_bytes) as usize;
    }

    if payload_len > MAX_WS_PAYLOAD {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "WebSocket payload size {} exceeds MAX_WS_PAYLOAD limit ({} bytes)",
                payload_len, MAX_WS_PAYLOAD
            ),
        ));
    }

    let mut mask_key = [0u8; 4];
    if is_masked {
        reader.read_exact(&mut mask_key)?;
    }

    let mut payload = vec![0u8; payload_len];
    reader.read_exact(&mut payload)?;

    if is_masked {
        for (i, byte) in payload.iter_mut().enumerate() {
            *byte ^= mask_key[i % 4];
        }
    }

    let payload_str = String::from_utf8(payload)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    Ok(Some(payload_str))
}
