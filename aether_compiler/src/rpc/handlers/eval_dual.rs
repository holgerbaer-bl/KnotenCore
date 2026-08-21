// Dual-Engine Evaluation RPC Handler (`knc_eval_dual`)
//
// Exposes the DualEngineValidator via a Zero-Trust authenticated JSON-RPC endpoint.
// On parity match, returns `"dual_verified"` status with the canonical result.
// On parity discrepancy, immediately aborts with error code -32020
// (ERR_ENGINE_DISCREPANCY) and returns a structured quarantine payload
// detailing the divergence. No state mutations or CRDT store persistence
// occur on discrepancy — the quarantine protocol prevents unverified
// state propagation.

use crate::vm::dual_validator::{DualEngineValidator, DualValidationError, DualValidationOutcome};
use knoten_core_types::ast::{IsolateQuota, Node};
use serde_json::Value;

use super::super::types::JsonRpcResponse;

/// RPC error code for engine discrepancy quarantine.
const ERR_ENGINE_DISCREPANCY: i32 = -32020;

impl super::super::RpcServer {
    /// Handle `knc_eval_dual` — Dual-Engine evaluation with quarantine protocol.
    ///
    /// Executes the incoming AST simultaneously on both the reference Tree-Walker
    /// evaluator and the AOT Stack-VM via `DualEngineValidator`, strictly validating
    /// semantic equivalence. On parity discrepancy, returns `-32020` with a structured
    /// quarantine payload and prevents any state mutation or CRDT persistence.
    pub fn handle_eval_dual(&self, id: Option<Value>, params: Value) -> JsonRpcResponse {
        // 1. Zero-Trust Security Gate (Day-1 Invariant)
        if let Err(err) = self.check_mesh_auth(&params) {
            return JsonRpcResponse::error(id, -32001, err);
        }

        // 2. Extract session_id (required for audit trail, not used for state)
        let _session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous");

        // 3. Extract and deserialize AST
        let ast_val = match params.get("ast") {
            Some(v) => v,
            None => return JsonRpcResponse::error(id, -32602, "Missing 'ast' parameter"),
        };

        let ast: Node = match serde_json::from_value(ast_val.clone()) {
            Ok(ast) => ast,
            Err(e) => {
                return JsonRpcResponse::error(id, -32602, format!("Invalid AST structure: {}", e));
            }
        };

        // 4. Extract optional isolate quota
        let quota = if let Some(q) = params.get("isolate_quota") {
            let max_instructions = q
                .get("max_instructions")
                .and_then(|v| v.as_u64())
                .unwrap_or(1_000_000);
            let max_memory_bytes = q
                .get("max_memory_bytes")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize)
                .unwrap_or(16 * 1024 * 1024);
            let execution_timeout_ms = q
                .get("execution_timeout_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(5000);
            IsolateQuota {
                max_instructions,
                max_memory_bytes,
                execution_timeout_ms,
            }
        } else {
            IsolateQuota::default()
        };

        // 5. Execute via DualEngineValidator — no session state, no CRDT persistence
        let validator = DualEngineValidator::new().with_quota(quota);
        let report = validator.validate(&ast);

        // 6. Process result
        if report.is_valid {
            // Parity Match — return dual_verified result
            let (result_json, fault_json) = match report.outcome {
                Some(DualValidationOutcome::Success {
                    ref return_value, ..
                }) => {
                    let json_val = crate::natives::fs::reltype_to_json_value(return_value);
                    (json_val, Value::Null)
                }
                Some(DualValidationOutcome::SymmetricalFault {
                    ref category,
                    ref eval_message,
                    ..
                }) => (
                    Value::Null,
                    serde_json::json!({
                        "category": category.to_string(),
                        "message": eval_message
                    }),
                ),
                None => (Value::Null, Value::Null),
            };

            JsonRpcResponse::success(
                id,
                serde_json::json!({
                    "status": "ok",
                    "result": result_json,
                    "execution_mode": "dual_verified",
                    "fault": fault_json,
                    "telemetry": {
                        "eval_duration_ns": report.eval_duration_ns,
                        "vm_duration_ns": report.vm_duration_ns,
                        "vm_gas_consumed": report.vm_gas_consumed,
                        "vm_instructions_executed": report.vm_instructions_executed
                    }
                }),
            )
        } else {
            // Parity Discrepancy — Quarantine Protocol: abort with -32020
            // No state mutations, no CRDT persistence, no session write-through.
            let quarantine_payload = match &report.error {
                Some(DualValidationError::ReturnValueMismatch {
                    eval_return,
                    vm_return,
                }) => serde_json::json!({
                    "discrepancy_type": "ReturnValueMismatch",
                    "tree_walker_result": eval_return.to_string(),
                    "stack_vm_result": vm_return.to_string()
                }),
                Some(DualValidationError::StateMutationMismatch {
                    eval_state,
                    vm_state,
                }) => serde_json::json!({
                    "discrepancy_type": "StateMutationMismatch",
                    "tree_walker_result": format!("{:?}", eval_state),
                    "stack_vm_result": format!("{:?}", vm_state)
                }),
                Some(DualValidationError::DivergentFaultCategory {
                    eval_category,
                    vm_category,
                    eval_message,
                    vm_message,
                }) => serde_json::json!({
                    "discrepancy_type": "DivergentFaultCategory",
                    "tree_walker_result": format!("{}: {}", eval_category, eval_message),
                    "stack_vm_result": format!("{}: {}", vm_category, vm_message)
                }),
                Some(DualValidationError::EngineDivergence {
                    eval_succeeded,
                    eval_detail,
                    vm_succeeded,
                    vm_detail,
                }) => serde_json::json!({
                    "discrepancy_type": "EngineDivergence",
                    "tree_walker_result": format!("success={}: {}", eval_succeeded, eval_detail),
                    "stack_vm_result": format!("success={}: {}", vm_succeeded, vm_detail)
                }),
                Some(DualValidationError::EvaluatorPanic(msg)) => serde_json::json!({
                    "discrepancy_type": "EvaluatorPanic",
                    "tree_walker_result": msg,
                    "stack_vm_result": "N/A"
                }),
                Some(DualValidationError::VmPanic(msg)) => serde_json::json!({
                    "discrepancy_type": "VmPanic",
                    "tree_walker_result": "N/A",
                    "stack_vm_result": msg
                }),
                None => serde_json::json!({
                    "discrepancy_type": "Unknown",
                    "tree_walker_result": "N/A",
                    "stack_vm_result": "N/A"
                }),
            };

            JsonRpcResponse::error(
                id,
                ERR_ENGINE_DISCREPANCY,
                format!(
                    "ERR_ENGINE_DISCREPANCY: Dual-engine parity validation failed. Quarantine active. Details: {}",
                    quarantine_payload
                ),
            )
        }
    }
}
