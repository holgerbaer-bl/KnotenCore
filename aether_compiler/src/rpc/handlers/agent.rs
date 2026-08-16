use knoten_core_types::ast::IsolateQuota;
use knoten_core_types::opcode::OpCode;
use serde_json::Value;

use crate::executor::RelType;
use crate::rpc::types::{validate_param_string_len, JsonRpcResponse, KNC_PROTOCOL_VERSION};
use crate::vm::machine::VmExecutionState;

impl super::super::RpcServer {
    pub fn handle_agent_handshake(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

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
                "peer_state_sync": true,
                "swarm_governance": true,
                "swarm_leadership": true,
                "node_roles": true,
                "raft_consensus": true,
                "raft_voting": true,
                "zero_trust_mesh": true,
                "key_rotation": true,
                "peer_revocation": true
            },
            "default_quota": default_quota,
            "local_public_key": self.public_key_hex(),
            "zero_trust_mode": self.is_zero_trust()
        });
        JsonRpcResponse::success(id, resp_val)
    }

    pub fn handle_agent_snapshot(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
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

    pub fn handle_agent_restore(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let session_id = match self.parse_session_id(&params) {
            Ok(s) => s,
            Err(err) => return JsonRpcResponse::error(id, -32602, err),
        };

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

    pub fn handle_agent_teleport(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        let target_session_id_str = params
            .get("target_session_id")
            .or_else(|| params.get("session_id"))
            .and_then(|v| v.as_str())
            .unwrap_or("default");
        if let Err(err) = validate_param_string_len(target_session_id_str) {
            return JsonRpcResponse::error(id, -32602, err);
        }
        let target_session_id = target_session_id_str.to_string();

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

        let mut restore_params = serde_json::json!({
            "session_id": target_session_id,
            "snapshot": snapshot_val
        });
        if let Some(token) = params.get("mesh_auth_token") {
            restore_params["mesh_auth_token"] = token.clone();
        }
        if let Some(sig) = params.get("mesh_auth_signature") {
            restore_params["mesh_auth_signature"] = sig.clone();
        }
        if let Some(zt) = params.get("zero_trust_envelope") {
            restore_params["zero_trust_envelope"] = zt.clone();
        }

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
}
