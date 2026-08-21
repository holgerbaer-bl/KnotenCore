use crate::executor::{AgentPermissions, ExecResult, ExecutionEngine, RelType};
use crate::optimizer::optimize;
use crate::vm::compiler::Compiler;
use crate::vm::machine::{GasMeter, VM, VMError};
use knoten_core_types::ast::{IsolateQuota, Node};
use std::collections::BTreeMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

/// Symmetrical error classification across reference Tree-Walker evaluator and AOT Stack-VM.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FaultCategory {
    DivisionByZero,
    ModuloByZero,
    TypeError,
    VariableNotFound,
    IndexOutOfBounds,
    PermissionDenied,
    GasExhausted,
    MemoryQuotaExceeded,
    WatchdogTimeout,
    CompilationFailed,
    StackUnderflow,
    RuntimeFault,
}

impl std::fmt::Display for FaultCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FaultCategory::DivisionByZero => write!(f, "DivisionByZero"),
            FaultCategory::ModuloByZero => write!(f, "ModuloByZero"),
            FaultCategory::TypeError => write!(f, "TypeError"),
            FaultCategory::VariableNotFound => write!(f, "VariableNotFound"),
            FaultCategory::IndexOutOfBounds => write!(f, "IndexOutOfBounds"),
            FaultCategory::PermissionDenied => write!(f, "PermissionDenied"),
            FaultCategory::GasExhausted => write!(f, "GasExhausted"),
            FaultCategory::MemoryQuotaExceeded => write!(f, "MemoryQuotaExceeded"),
            FaultCategory::WatchdogTimeout => write!(f, "WatchdogTimeout"),
            FaultCategory::CompilationFailed => write!(f, "CompilationFailed"),
            FaultCategory::StackUnderflow => write!(f, "StackUnderflow"),
            FaultCategory::RuntimeFault => write!(f, "RuntimeFault"),
        }
    }
}

/// Classify error strings from either engine into a canonical `FaultCategory`.
pub fn classify_fault(msg: &str) -> FaultCategory {
    let lower = msg.to_lowercase();
    if lower.contains("div by zero")
        || lower.contains("division by zero")
        || lower.contains("divide by zero")
    {
        FaultCategory::DivisionByZero
    } else if lower.contains("mod by zero") || lower.contains("modulo by zero") {
        FaultCategory::ModuloByZero
    } else if lower.contains("invalid types")
        || lower.contains("type mismatch")
        || lower.contains("expects")
        || lower.contains("invalid operand")
        || lower.contains("expected ")
    {
        FaultCategory::TypeError
    } else if lower.contains("variable '")
        || lower.contains("undefined variable")
        || lower.contains("not found")
    {
        FaultCategory::VariableNotFound
    } else if lower.contains("out of bounds") || lower.contains("index out of bounds") {
        FaultCategory::IndexOutOfBounds
    } else if lower.contains("permission denied") || lower.contains("sandbox") {
        FaultCategory::PermissionDenied
    } else if lower.contains("gasexhausted")
        || lower.contains("err_quota_exceeded")
        || lower.contains("gas limit")
    {
        FaultCategory::GasExhausted
    } else if lower.contains("memoryquotaexceeded")
        || lower.contains("err_memory_limit_exceeded")
        || lower.contains("memory limit")
    {
        FaultCategory::MemoryQuotaExceeded
    } else if lower.contains("watchdogtimeout")
        || lower.contains("watchdog_timeout")
        || lower.contains("timeout exceeded")
    {
        FaultCategory::WatchdogTimeout
    } else if lower.contains("compilation failed") {
        FaultCategory::CompilationFailed
    } else if lower.contains("stack underflow") {
        FaultCategory::StackUnderflow
    } else {
        FaultCategory::RuntimeFault
    }
}

/// Symmetrical validation outcome on valid dual execution.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DualValidationOutcome {
    Success {
        return_value: RelType,
        state_mutations: BTreeMap<String, RelType>,
    },
    SymmetricalFault {
        category: FaultCategory,
        eval_message: String,
        vm_message: String,
    },
}

/// Specific discrepancy details when dual-engine validation fails.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum DualValidationError {
    ReturnValueMismatch {
        eval_return: RelType,
        vm_return: RelType,
    },
    StateMutationMismatch {
        eval_state: BTreeMap<String, RelType>,
        vm_state: BTreeMap<String, RelType>,
    },
    DivergentFaultCategory {
        eval_category: FaultCategory,
        vm_category: FaultCategory,
        eval_message: String,
        vm_message: String,
    },
    EngineDivergence {
        eval_succeeded: bool,
        eval_detail: String,
        vm_succeeded: bool,
        vm_detail: String,
    },
    EvaluatorPanic(String),
    VmPanic(String),
}

impl std::fmt::Display for DualValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DualValidationError::ReturnValueMismatch {
                eval_return,
                vm_return,
            } => {
                write!(
                    f,
                    "Return value mismatch: Evaluator produced '{}', VM produced '{}'",
                    eval_return, vm_return
                )
            }
            DualValidationError::StateMutationMismatch {
                eval_state,
                vm_state,
            } => {
                write!(
                    f,
                    "State mutation mismatch: Evaluator state '{:?}', VM state '{:?}'",
                    eval_state, vm_state
                )
            }
            DualValidationError::DivergentFaultCategory {
                eval_category,
                vm_category,
                eval_message,
                vm_message,
            } => {
                write!(
                    f,
                    "Divergent fault categories: Evaluator=[{}: '{}'], VM=[{}: '{}']",
                    eval_category, eval_message, vm_category, vm_message
                )
            }
            DualValidationError::EngineDivergence {
                eval_succeeded,
                eval_detail,
                vm_succeeded,
                vm_detail,
            } => {
                write!(
                    f,
                    "Engine divergence: Evaluator (success={}): {}, VM (success={}): {}",
                    eval_succeeded, eval_detail, vm_succeeded, vm_detail
                )
            }
            DualValidationError::EvaluatorPanic(msg) => {
                write!(f, "Tree-Walker Evaluator panicked: {}", msg)
            }
            DualValidationError::VmPanic(msg) => {
                write!(f, "AOT Stack-VM panicked: {}", msg)
            }
        }
    }
}

/// Comprehensive telemetry report from dual-engine validation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DualValidationReport {
    pub is_valid: bool,
    pub outcome: Option<DualValidationOutcome>,
    pub error: Option<DualValidationError>,
    pub eval_duration_ns: u64,
    pub vm_duration_ns: u64,
    pub vm_gas_consumed: u64,
    pub vm_instructions_executed: usize,
}

/// DualEngineValidator: Executes ASTs concurrently on both Tree-Walker and AOT Stack-VM
/// to assert semantic equivalence, observable state mutations, and fault classification.
#[derive(Debug, Clone)]
pub struct DualEngineValidator {
    pub permissions: AgentPermissions,
    pub quota: IsolateQuota,
    pub optimize_ast: bool,
}

impl Default for DualEngineValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl DualEngineValidator {
    pub fn new() -> Self {
        Self {
            permissions: AgentPermissions::default(),
            quota: IsolateQuota::default(),
            optimize_ast: true,
        }
    }

    pub fn with_permissions(mut self, perms: AgentPermissions) -> Self {
        self.permissions = perms;
        self
    }

    pub fn with_quota(mut self, quota: IsolateQuota) -> Self {
        self.quota = quota;
        self
    }

    pub fn with_optimization(mut self, opt: bool) -> Self {
        self.optimize_ast = opt;
        self
    }

    /// Execute the AST on both engines with zero-panic isolation and compare parity.
    pub fn validate(&self, node: &Node) -> DualValidationReport {
        let target_ast = if self.optimize_ast {
            optimize(node.clone())
        } else {
            node.clone()
        };

        // 1. Run Tree-Walker Evaluator with Panic Containment
        let eval_perms = self.permissions.clone();
        let eval_ast = target_ast.clone();
        let eval_start = Instant::now();

        let eval_execution = catch_unwind(AssertUnwindSafe(|| {
            let mut engine = ExecutionEngine::new();
            engine.permissions = eval_perms;
            let result = engine.evaluate(&eval_ast);
            let state: BTreeMap<String, RelType> = engine.memory.into_iter().collect();
            (result, state)
        }));
        let eval_duration_ns = eval_start.elapsed().as_nanos() as u64;

        // 2. Run AOT Stack-VM with Panic Containment
        let vm_perms = self.permissions.clone();
        let vm_quota = self.quota.clone();
        let vm_ast = target_ast;
        let vm_start = Instant::now();

        let vm_execution = catch_unwind(AssertUnwindSafe(|| {
            let mut compiler = Compiler::new();
            if !compiler.compile_node(&vm_ast) {
                return (
                    Err(VMError::RuntimeFault("Compilation failed".to_string())),
                    BTreeMap::new(),
                    0,
                    0,
                );
            }

            let mut vm = VM::new();
            vm.quota = vm_quota;
            vm.gas_meter = GasMeter::new(vm.quota.max_instructions);

            let run_res = vm.run(&compiler.instructions, &compiler.constants, &vm_perms, None);

            let gas_consumed = vm.gas_meter.consumed_gas;
            let instructions_len = compiler.instructions.len();
            let state: BTreeMap<String, RelType> = vm
                .globals
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();

            let mapped_res = match run_res {
                Ok(val) => Ok(val),
                Err(err_msg) => {
                    if err_msg.contains("GasExhausted") || err_msg.contains("ERR_QUOTA_EXCEEDED") {
                        Err(VMError::GasExhausted {
                            executed_instructions: gas_consumed,
                            limit: vm.quota.max_instructions,
                        })
                    } else if err_msg.contains("MemoryQuotaExceeded")
                        || err_msg.contains("ERR_MEMORY_LIMIT_EXCEEDED")
                    {
                        Err(VMError::MemoryQuotaExceeded {
                            current_bytes: vm.estimate_memory_bytes(),
                            limit_bytes: vm.quota.max_memory_bytes,
                        })
                    } else if err_msg.contains("WatchdogTimeout")
                        || err_msg.contains("WATCHDOG_TIMEOUT")
                    {
                        Err(VMError::WatchdogTimeout {
                            elapsed_us: vm.quota.execution_timeout_ms * 1000,
                            timeout_us: vm.quota.execution_timeout_ms * 1000,
                        })
                    } else {
                        Err(VMError::RuntimeFault(err_msg))
                    }
                }
            };

            (mapped_res, state, gas_consumed, instructions_len)
        }));
        let vm_duration_ns = vm_start.elapsed().as_nanos() as u64;

        // 3. Process Evaluator Unwind / Panic
        let (eval_result, eval_state) = match eval_execution {
            Ok(res) => res,
            Err(payload) => {
                let panic_msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic unwind in evaluator".to_string()
                };
                return DualValidationReport {
                    is_valid: false,
                    outcome: None,
                    error: Some(DualValidationError::EvaluatorPanic(panic_msg)),
                    eval_duration_ns,
                    vm_duration_ns,
                    vm_gas_consumed: 0,
                    vm_instructions_executed: 0,
                };
            }
        };

        // 4. Process VM Unwind / Panic
        let (vm_result, vm_state, vm_gas_consumed, vm_instructions_executed) = match vm_execution {
            Ok(res) => res,
            Err(payload) => {
                let panic_msg = if let Some(s) = payload.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = payload.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic unwind in VM".to_string()
                };
                return DualValidationReport {
                    is_valid: false,
                    outcome: None,
                    error: Some(DualValidationError::VmPanic(panic_msg)),
                    eval_duration_ns,
                    vm_duration_ns,
                    vm_gas_consumed: 0,
                    vm_instructions_executed: 0,
                };
            }
        };

        // 5. Compare Parity
        let eval_success_val = match &eval_result {
            ExecResult::Value(v) | ExecResult::ReturnBlockInfo(v) => Some(v.clone()),
            ExecResult::Fault { .. } => None,
        };

        let vm_success_val = vm_result.as_ref().ok().cloned();

        let report_outcome: Result<DualValidationOutcome, DualValidationError> =
            match (eval_success_val, vm_success_val) {
                (Some(eval_val), Some(vm_val)) => {
                    if eval_val != vm_val {
                        Err(DualValidationError::ReturnValueMismatch {
                            eval_return: eval_val,
                            vm_return: vm_val,
                        })
                    } else if eval_state != vm_state {
                        Err(DualValidationError::StateMutationMismatch {
                            eval_state,
                            vm_state,
                        })
                    } else {
                        Ok(DualValidationOutcome::Success {
                            return_value: eval_val,
                            state_mutations: eval_state,
                        })
                    }
                }
                (None, None) => {
                    let eval_msg = match eval_result {
                        ExecResult::Fault { msg, .. } => msg,
                        _ => unreachable!(),
                    };
                    let vm_msg = match vm_result {
                        Err(err) => err.to_string(),
                        _ => unreachable!(),
                    };

                    let eval_cat = classify_fault(&eval_msg);
                    let vm_cat = classify_fault(&vm_msg);

                    if eval_cat == vm_cat {
                        Ok(DualValidationOutcome::SymmetricalFault {
                            category: eval_cat,
                            eval_message: eval_msg,
                            vm_message: vm_msg,
                        })
                    } else {
                        Err(DualValidationError::DivergentFaultCategory {
                            eval_category: eval_cat,
                            vm_category: vm_cat,
                            eval_message: eval_msg,
                            vm_message: vm_msg,
                        })
                    }
                }
                (Some(eval_val), None) => {
                    let vm_err_msg = match vm_result {
                        Err(err) => err.to_string(),
                        _ => unreachable!(),
                    };
                    Err(DualValidationError::EngineDivergence {
                        eval_succeeded: true,
                        eval_detail: eval_val.to_string(),
                        vm_succeeded: false,
                        vm_detail: vm_err_msg,
                    })
                }
                (None, Some(vm_val)) => {
                    let eval_err_msg = match eval_result {
                        ExecResult::Fault { msg, .. } => msg,
                        _ => unreachable!(),
                    };
                    Err(DualValidationError::EngineDivergence {
                        eval_succeeded: false,
                        eval_detail: eval_err_msg,
                        vm_succeeded: true,
                        vm_detail: vm_val.to_string(),
                    })
                }
            };

        match report_outcome {
            Ok(outcome) => DualValidationReport {
                is_valid: true,
                outcome: Some(outcome),
                error: None,
                eval_duration_ns,
                vm_duration_ns,
                vm_gas_consumed,
                vm_instructions_executed,
            },
            Err(err) => DualValidationReport {
                is_valid: false,
                outcome: None,
                error: Some(err),
                eval_duration_ns,
                vm_duration_ns,
                vm_gas_consumed,
                vm_instructions_executed,
            },
        }
    }

    /// Convenience assertion method that returns Ok(outcome) or Err(error).
    pub fn assert_parity(&self, node: &Node) -> Result<DualValidationOutcome, DualValidationError> {
        let report = self.validate(node);
        if report.is_valid {
            Ok(report.outcome.unwrap())
        } else {
            Err(report.error.unwrap())
        }
    }
}
