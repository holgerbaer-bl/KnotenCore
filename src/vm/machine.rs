use crate::executor::{AgentPermissions, ExecutionEngine, RelType};
use crate::vm::opcode::OpCode;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct CallFrame {
    pub ip: usize,
    pub base_pointer: usize,
}

#[derive(Default)]
pub struct VM {
    pub stack: Vec<RelType>,
    pub globals: HashMap<String, RelType>,
    pub frames: Vec<CallFrame>,
    pub ip: usize,
    pub base_pointer: usize,
}

impl VM {
    pub fn new() -> Self {
        Self {
            stack: Vec::with_capacity(256),
            globals: HashMap::new(),
            frames: Vec::with_capacity(64),
            ip: 0,
            base_pointer: 0,
        }
    }

    #[inline(always)]
    pub fn run(
        &mut self,
        instructions: &[OpCode],
        constants: &[RelType],
        permissions: &AgentPermissions,
        bridge: Option<&dyn crate::natives::bridge::BridgeModule>,
    ) -> Result<RelType, String> {
        self.stack.clear();
        self.frames.clear();
        self.ip = 0;
        self.base_pointer = 0;

        let mut start = Instant::now();
        let mut instr_count: u64 = 0;

        while self.ip < instructions.len() {
            let op = &instructions[self.ip];
            self.ip += 1;

            instr_count += 1;
            if instr_count.is_multiple_of(1000)
                && start.elapsed() >= std::time::Duration::from_millis(50)
            {
                eprintln!(
                    "[KnotenCore Watchdog] Execution timeout exceeded (50ms). Terminating script to prevent CPU freeze."
                );
                return Err("Watchdog: Execution timeout exceeded (50ms)".into());
            }

            match op {
                OpCode::Constant(idx) => {
                    if *idx < constants.len() {
                        self.stack.push(constants[*idx].clone());
                    } else {
                        return Err("Constant index out of bounds".into());
                    }
                }
                OpCode::Add => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Add".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Add".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a + b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a + b))
                        }
                        (RelType::Int(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a as f64 + b))
                        }
                        (RelType::Float(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Float(a + b as f64))
                        }
                        (RelType::Str(a), RelType::Str(b)) => self.stack.push(RelType::Str(a + &b)),
                        _ => return Err("Invalid types for Add".into()),
                    }
                }
                OpCode::Subtract => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Subtract".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Subtract".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a - b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a - b))
                        }
                        (RelType::Int(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a as f64 - b))
                        }
                        (RelType::Float(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Float(a - b as f64))
                        }
                        _ => return Err("Invalid types for Subtract".into()),
                    }
                }
                OpCode::Multiply => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Multiply".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Multiply".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a * b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a * b))
                        }
                        (RelType::Int(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Float(a as f64 * b))
                        }
                        (RelType::Float(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Float(a * b as f64))
                        }
                        _ => return Err("Invalid types for Multiply".into()),
                    }
                }
                OpCode::Divide => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Divide".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Divide".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            if b == 0 {
                                return Err("Fault: Div by zero (at Node::MathDiv)".into());
                            }
                            self.stack.push(RelType::Int(a / b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            if b == 0.0 {
                                return Err("Fault: Div by zero (at Node::MathDiv)".into());
                            }
                            self.stack.push(RelType::Float(a / b))
                        }
                        _ => return Err("Invalid types for Divide".into()),
                    }
                }
                OpCode::Modulo => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Modulo".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Modulo".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            if b == 0 {
                                return Err("Fault: Div by zero (at Node::Modulo)".into());
                            }
                            self.stack.push(RelType::Int(a % b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            if b == 0.0 {
                                return Err("Fault: Div by zero (at Node::Modulo)".into());
                            }
                            self.stack.push(RelType::Float(a % b))
                        }
                        _ => return Err("Invalid types for Modulo".into()),
                    }
                }
                OpCode::Neg => {
                    let v = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Neg".to_string())?;
                    match v {
                        RelType::Int(a) => self.stack.push(RelType::Int(-a)),
                        RelType::Float(a) => self.stack.push(RelType::Float(-a)),
                        _ => return Err("Invalid type for Neg".into()),
                    }
                }
                OpCode::Equal => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Equal".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Equal".to_string())?;
                    self.stack.push(RelType::Bool(l == r));
                }
                OpCode::Less => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Less".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Less".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Bool(a < b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a < b))
                        }
                        _ => return Err("Invalid types for Less comparison".into()),
                    }
                }
                OpCode::Greater => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Greater".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Greater".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Bool(a > b)),
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a > b))
                        }
                        _ => return Err("Invalid types for Greater comparison".into()),
                    }
                }
                OpCode::NotEqual => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in NotEqual".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in NotEqual".to_string())?;
                    self.stack.push(RelType::Bool(l != r));
                }
                OpCode::LessEqual => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in LessEqual".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in LessEqual".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Bool(a <= b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a <= b))
                        }
                        _ => return Err("Invalid types for LessEqual comparison".into()),
                    }
                }
                OpCode::GreaterEqual => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in GreaterEqual".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in GreaterEqual".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            self.stack.push(RelType::Bool(a >= b))
                        }
                        (RelType::Float(a), RelType::Float(b)) => {
                            self.stack.push(RelType::Bool(a >= b))
                        }
                        _ => return Err("Invalid types for GreaterEqual comparison".into()),
                    }
                }
                OpCode::And => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in And".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in And".to_string())?;
                    match (l, r) {
                        (RelType::Bool(a), RelType::Bool(b)) => {
                            self.stack.push(RelType::Bool(a && b))
                        }
                        _ => return Err("And expects two booleans".into()),
                    }
                }
                OpCode::Or => {
                    let r = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Or".to_string())?;
                    let l = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Or".to_string())?;
                    match (l, r) {
                        (RelType::Bool(a), RelType::Bool(b)) => {
                            self.stack.push(RelType::Bool(a || b))
                        }
                        _ => return Err("Or expects two booleans".into()),
                    }
                }
                OpCode::Not => {
                    let v = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Not".to_string())?;
                    match v {
                        RelType::Bool(b) => self.stack.push(RelType::Bool(!b)),
                        _ => return Err("Not expects a boolean".into()),
                    }
                }
                OpCode::JumpIfFalse(target_ip) => {
                    let cond = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in JumpIfFalse".to_string())?;
                    let is_true = match cond {
                        RelType::Bool(b) => b,
                        RelType::Int(i) => i != 0,
                        _ => false,
                    };
                    if !is_true {
                        self.ip = *target_ip;
                    }
                }
                OpCode::Jump(target_ip) => {
                    self.ip = *target_ip;
                }
                OpCode::SetLocal(idx) => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in SetLocal".to_string())?;
                    let target_idx = self.base_pointer + *idx;
                    // Dynamically allocate stack for isolated variables
                    if target_idx >= self.stack.len() {
                        self.stack.resize(target_idx + 1, RelType::Void);
                    }
                    self.stack[target_idx] = val;
                }
                OpCode::GetLocal(idx) => {
                    let val = self
                        .stack
                        .get(self.base_pointer + *idx)
                        .cloned()
                        .ok_or_else(|| format!("Stack underflow in GetLocal({})", idx))?;
                    self.stack.push(val);
                }
                OpCode::Call(target_ip, arg_count) => {
                    self.frames.push(CallFrame {
                        ip: self.ip,
                        base_pointer: self.base_pointer,
                    });
                    self.base_pointer = self.stack.len().saturating_sub(*arg_count);
                    self.ip = *target_ip;
                }
                OpCode::SetGlobal(idx) => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in SetGlobal".to_string())?;
                    if let Some(RelType::Str(name)) = constants.get(*idx) {
                        self.globals.insert(name.clone(), val);
                    } else {
                        return Err("Invalid constant index for SetGlobal".into());
                    }
                }
                OpCode::GetGlobal(idx) => {
                    if let Some(RelType::Str(name)) = constants.get(*idx) {
                        if let Some(val) = self.globals.get(name) {
                            self.stack.push(val.clone());
                        } else {
                            self.stack.push(RelType::Void);
                        }
                    } else {
                        return Err("Invalid constant index for GetGlobal".into());
                    }
                }
                OpCode::StringLength => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringLength".to_string())?;
                    if let RelType::Str(s) = val {
                        self.stack.push(RelType::Int(s.chars().count() as i64));
                    } else {
                        return Err("StringLength expects a Str".into());
                    }
                }
                OpCode::StringContainsChars => {
                    let pattern = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringContainsChars".to_string())?;
                    let target = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringContainsChars".to_string())?;
                    if let (RelType::Str(s), RelType::Str(p)) = (target, pattern) {
                        let contains = match p.as_str() {
                            "numbers" => s.chars().any(|c| c.is_ascii_digit()),
                            "special" => s
                                .chars()
                                .any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace()),
                            "uppercase" => s.chars().any(|c| c.is_ascii_uppercase()),
                            "lowercase" => s.chars().any(|c| c.is_ascii_lowercase()),
                            other => s.contains(other),
                        };
                        self.stack.push(RelType::Bool(contains));
                    } else {
                        self.stack.push(RelType::Bool(false));
                    }
                }
                OpCode::StringSplit => {
                    let delim = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringSplit".to_string())?;
                    let target = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in StringSplit".to_string())?;
                    if let (RelType::Str(s), RelType::Str(d)) = (target, delim) {
                        let parts = s
                            .split(&d)
                            .map(|part| RelType::Str(part.to_string()))
                            .collect();
                        self.stack.push(RelType::Array(parts));
                    } else {
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::ArrayContains => {
                    let search = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ArrayContains".to_string())?;
                    let array = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ArrayContains".to_string())?;
                    if let (RelType::Array(arr), search_val) = (array, search) {
                        self.stack.push(RelType::Bool(arr.contains(&search_val)));
                    } else {
                        self.stack.push(RelType::Bool(false));
                    }
                }
                OpCode::ReadFile => {
                    let path_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in ReadFile".to_string())?;
                    if let RelType::Str(path) = path_val {
                        if !permissions.allow_fs_read {
                            return Err(
                                "Permission Denied: allow_fs_read is false (VM: ReadFile)".into()
                            );
                        } else {
                            match ExecutionEngine::validate_fs_path(&path) {
                                Ok(safe_path) => {
                                    if let Ok(content) = std::fs::read_to_string(safe_path) {
                                        self.stack.push(RelType::Str(content));
                                    } else {
                                        self.stack.push(RelType::Void);
                                    }
                                }
                                Err(_) => self.stack.push(RelType::Void),
                            }
                        }
                    } else {
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::ArrayCreate(count) => {
                    let mut elements = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        elements.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    elements.reverse();
                    self.stack.push(RelType::Array(elements));
                }
                OpCode::ArrayGet => {
                    let idx_val = self.stack.pop().unwrap_or(RelType::Void);
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let (RelType::Array(arr), RelType::Int(idx)) = (arr_val, idx_val) {
                        if idx >= 0 && (idx as usize) < arr.len() {
                            self.stack.push(arr[idx as usize].clone());
                        } else {
                            return Err(format!("ArrayGet index out of bounds: {}", idx));
                        }
                    } else {
                        return Err("ArrayGet expects Array and Int".into());
                    }
                }
                OpCode::ArraySet => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    let idx_val = self.stack.pop().unwrap_or(RelType::Void);
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let (RelType::Array(mut arr), RelType::Int(idx)) = (arr_val, idx_val) {
                        if idx >= 0 && (idx as usize) < arr.len() {
                            arr[idx as usize] = val;
                            self.stack.push(RelType::Array(arr));
                        } else {
                            return Err(format!("ArraySet index out of bounds: {}", idx));
                        }
                    } else {
                        return Err("ArraySet expects Array and Int".into());
                    }
                }
                OpCode::ArrayPush => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let RelType::Array(mut arr) = arr_val {
                        arr.push(val);
                        self.stack.push(RelType::Array(arr));
                    } else {
                        return Err("ArrayPush expects Array".into());
                    }
                }
                OpCode::ArrayLen => {
                    let arr_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let RelType::Array(arr) = arr_val {
                        self.stack.push(RelType::Int(arr.len() as i64));
                    } else {
                        return Err("ArrayLen expects Array".into());
                    }
                }
                OpCode::Concat => {
                    let r_val = self.stack.pop().unwrap_or(RelType::Void);
                    let l_val = self.stack.pop().unwrap_or(RelType::Void);
                    match (l_val, r_val) {
                        (RelType::Str(a), RelType::Str(b)) => self.stack.push(RelType::Str(a + &b)),
                        (RelType::Array(mut a), RelType::Array(b)) => {
                            a.extend(b);
                            self.stack.push(RelType::Array(a));
                        }
                        _ => return Err("Concat expects Strings or Arrays".into()),
                    }
                }
                OpCode::ToString => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    self.stack.push(RelType::Str(val.to_string()));
                }
                OpCode::WriteFile => {
                    let data_val = self.stack.pop().unwrap_or(RelType::Void);
                    let path_val = self.stack.pop().unwrap_or(RelType::Void);

                    if !permissions.allow_fs_write {
                        return Err(
                            "Permission Denied: allow_fs_write is false (VM: WriteFile)".into()
                        );
                    }

                    if let (RelType::Str(path), RelType::Str(data)) = (path_val, data_val) {
                        match crate::executor::ExecutionEngine::validate_fs_path_write(&path) {
                            Ok(safe_path) => {
                                if let Err(e) = std::fs::write(&safe_path, data) {
                                    return Err(format!("File write error: {}", e));
                                }
                            }
                            Err(e) => return Err(format!("Security: {}", e)),
                        }
                    } else {
                        return Err("WriteFile expects string path and data".into());
                    }
                    self.stack.push(RelType::Void);
                }
                OpCode::NativeExternCall {
                    module_idx,
                    func_idx,
                    arg_count,
                } => {
                    let module = match constants.get(*module_idx) {
                        Some(RelType::Str(s)) => s.clone(),
                        _ => "global".to_string(),
                    };
                    let func = match constants.get(*func_idx) {
                        Some(RelType::Str(s)) => s.clone(),
                        _ => "unknown".to_string(),
                    };

                    let mut args = Vec::with_capacity(*arg_count);
                    for _ in 0..*arg_count {
                        args.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    args.reverse();

                    if module == "registry" {
                        if func == "registry_play_sound" {
                            if args.len() == 1
                                && let RelType::Str(path) = &args[0]
                            {
                                match crate::executor::ExecutionEngine::validate_fs_path(path) {
                                    Ok(safe_path) => {
                                        crate::natives::registry::init_audio_state();
                                        if let Ok(mut lock) =
                                            crate::natives::registry::AUDIO_STATE.lock()
                                            && let Some(audio) = lock.as_mut()
                                        {
                                            let _ = audio.play_sound(&safe_path.to_string_lossy());
                                        }
                                        self.stack.push(RelType::Void);
                                        continue;
                                    }
                                    Err(e) => {
                                        return Err(format!("Fault: {} (at Node::ExternCall)", e));
                                    }
                                }
                            }
                            return Err(
                                "Fault: registry_play_sound expects (String) (at Node::ExternCall)"
                                    .to_string(),
                            );
                        }
                        if func == "registry_loop_music" {
                            if args.len() == 1
                                && let RelType::Str(path) = &args[0]
                            {
                                match crate::executor::ExecutionEngine::validate_fs_path(path) {
                                    Ok(safe_path) => {
                                        crate::natives::registry::init_audio_state();
                                        if let Ok(mut lock) =
                                            crate::natives::registry::AUDIO_STATE.lock()
                                            && let Some(audio) = lock.as_mut()
                                        {
                                            let _ = audio.loop_music(&safe_path.to_string_lossy());
                                        }
                                        self.stack.push(RelType::Void);
                                        continue;
                                    }
                                    Err(e) => {
                                        return Err(format!("Fault: {} (at Node::ExternCall)", e));
                                    }
                                }
                            }
                            return Err(
                                "Fault: registry_loop_music expects (String) (at Node::ExternCall)"
                                    .to_string(),
                            );
                        }
                        if func == "registry_set_volume" {
                            if args.len() == 1 {
                                let level = match &args[0] {
                                    RelType::Float(f) => *f as f32,
                                    RelType::Int(i) => *i as f32,
                                    _ => return Err("Fault: registry_set_volume expects (Float/Int) (at Node::ExternCall)".to_string()),
                                };
                                crate::natives::registry::init_audio_state();
                                if let Ok(mut lock) = crate::natives::registry::AUDIO_STATE.lock()
                                    && let Some(audio) = lock.as_mut()
                                {
                                    audio.set_volume(level);
                                }
                                self.stack.push(RelType::Void);
                                continue;
                            }
                            return Err("Fault: registry_set_volume expects (Float/Int) (at Node::ExternCall)".to_string());
                        }
                    }

                    if let Some(b) = bridge {
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            b.handle(&module, &func, &args, permissions)
                        }));
                        start = std::time::Instant::now();
                        match result {
                            Ok(Some(crate::executor::ExecResult::Value(v))) => self.stack.push(v),
                            Ok(Some(crate::executor::ExecResult::Fault { msg, .. })) => {
                                return Err(format!("FFI Fault: {}", msg));
                            }
                            Ok(None) => {
                                return Err(format!(
                                    "FFI Function '{}.{}' not handled by active BridgeModule",
                                    module, func
                                ));
                            }
                            Ok(_) => self.stack.push(RelType::Void),
                            Err(panic_payload) => {
                                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                                    s.to_string()
                                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                                    s.clone()
                                } else {
                                    "Unknown panic".to_string()
                                };
                                eprintln!(
                                    "[KnotenCore Panic] Caught panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                );
                                return Err(format!(
                                    "VM Panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                ));
                            }
                        }
                    } else {
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::UILabel => {
                    let text_val = self.stack.pop().unwrap_or(RelType::Void);
                    let text = match text_val {
                        RelType::Str(s) => s,
                        v => v.to_string(),
                    };
                    self.stack
                        .push(RelType::ASTNode(Box::new(crate::ast::Node::UILabel(
                            Box::new(crate::ast::Node::StringLiteral(text)),
                        ))));
                }
                OpCode::UIButton => {
                    let text_val = self.stack.pop().unwrap_or(RelType::Void);
                    let text = match text_val {
                        RelType::Str(s) => s,
                        v => v.to_string(),
                    };
                    self.stack
                        .push(RelType::ASTNode(Box::new(crate::ast::Node::UIButton(
                            Box::new(crate::ast::Node::StringLiteral(text)),
                        ))));
                }
                OpCode::UIHBox(count) => {
                    let mut children = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        if let RelType::ASTNode(node) = self.stack.pop().unwrap_or(RelType::Void) {
                            children.push(*node);
                        }
                    }
                    children.reverse();
                    self.stack
                        .push(RelType::ASTNode(Box::new(crate::ast::Node::UIHBox(
                            children,
                        ))));
                }
                OpCode::UIVBox(count) => {
                    let mut children = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        if let RelType::ASTNode(node) = self.stack.pop().unwrap_or(RelType::Void) {
                            children.push(*node);
                        }
                    }
                    children.reverse();
                    self.stack
                        .push(RelType::ASTNode(Box::new(crate::ast::Node::UIVBox(
                            children,
                        ))));
                }
                OpCode::UIWindow(_id_idx, count) => {
                    let mut children = Vec::with_capacity(*count);
                    for _ in 0..*count {
                        if let RelType::ASTNode(node) = self.stack.pop().unwrap_or(RelType::Void) {
                            children.push(*node);
                        }
                    }
                    children.reverse();

                    let _title_val = self.stack.pop().unwrap_or(RelType::Void);

                    crate::natives::registry::send_ui_nodes(children);
                    self.stack.push(RelType::Void);
                }
                OpCode::LoadComputeShader => {
                    let source_val = self.stack.pop().unwrap_or(RelType::Void);
                    if let RelType::Str(source) = source_val {
                        // Generate ID by hashing the source to deduplicate shader compilations
                        use std::hash::{Hash, Hasher};
                        let mut hasher = std::collections::hash_map::DefaultHasher::new();
                        source.hash(&mut hasher);
                        let id = hasher.finish() as usize;
                        crate::natives::registry::send_render_command(
                            crate::natives::registry::RenderCommand::LoadComputeShader {
                                id,
                                source,
                            },
                        );
                        self.stack.push(RelType::Int(id as i64));
                    } else {
                        return Err("LoadComputeShader expects a String source".into());
                    }
                }
                OpCode::DispatchCompute(arg_count) => {
                    let mut inputs = Vec::with_capacity(*arg_count);
                    for _ in 0..*arg_count {
                        inputs.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    inputs.reverse();

                    let z = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as u32,
                        RelType::Float(v) => v as u32,
                        _ => return Err("DispatchCompute expects numeric Z dimension".into()),
                    };
                    let y = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as u32,
                        RelType::Float(v) => v as u32,
                        _ => return Err("DispatchCompute expects numeric Y dimension".into()),
                    };
                    let x = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as u32,
                        RelType::Float(v) => v as u32,
                        _ => return Err("DispatchCompute expects numeric X dimension".into()),
                    };
                    let shader_id = match self.stack.pop().unwrap_or(RelType::Void) {
                        RelType::Int(v) => v as usize,
                        _ => return Err("DispatchCompute expects integer Shader ID".into()),
                    };

                    crate::natives::registry::send_render_command(
                        crate::natives::registry::RenderCommand::DispatchCompute {
                            shader_id,
                            x,
                            y,
                            z,
                            inputs,
                        },
                    );
                    self.stack.push(RelType::Void);
                }
                OpCode::ExternCall {
                    name_idx,
                    arg_count,
                } => {
                    let name = match constants.get(*name_idx) {
                        Some(RelType::Str(s)) => s.clone(),
                        _ => {
                            return Err(
                                "OpExternCall: valid function name not found in constant pool"
                                    .to_string(),
                            );
                        }
                    };

                    // Pop arg_count items from Stack
                    let mut args = Vec::with_capacity(*arg_count);
                    for _ in 0..*arg_count {
                        args.push(self.stack.pop().unwrap_or(RelType::Void));
                    }
                    args.reverse(); // Standard reverse popping mapping

                    // Dynamic module routing from script prefix conventions
                    let (module, func) = if name.starts_with("registry_") {
                        ("registry", name.as_str())
                    } else if name.starts_with("ui_") {
                        ("ui", name.as_str())
                    } else if name.starts_with("fs_") || name.starts_with("file_") {
                        ("fs", name.as_str())
                    } else if name.starts_with("test_") {
                        ("test_lib", name.as_str())
                    } else if name.starts_with("array_") || name.starts_with("obj_") {
                        ("fs", name.as_str())
                    } else if name.starts_with("net_") || name.starts_with("network_") {
                        ("net", name.as_str())
                    } else if name.starts_with("json_") {
                        ("json", name.as_str())
                    } else if name.starts_with("time_") {
                        ("time", name.as_str())
                    } else if name.starts_with("math_") {
                        ("math", name.as_str())
                    } else if name.starts_with("string_") {
                        ("string", name.as_str())
                    } else {
                        // Global scope for unmapped builtins if the user writes flat names
                        ("global", name.as_str())
                    };

                    if let Some(b) = bridge {
                        let result = catch_unwind(AssertUnwindSafe(|| {
                            b.handle(module, func, &args, permissions)
                        }));
                        start = std::time::Instant::now();
                        match result {
                            Ok(Some(crate::executor::ExecResult::Value(v))) => self.stack.push(v),
                            Ok(Some(crate::executor::ExecResult::Fault { msg, .. })) => {
                                return Err(format!("FFI Fault: {}", msg));
                            }
                            Ok(None) => {
                                return Err(format!(
                                    "FFI Function '{}.{}' not handled by active BridgeModule",
                                    module, func
                                ));
                            }
                            Ok(_) => self.stack.push(RelType::Void),
                            Err(panic_payload) => {
                                let msg = if let Some(s) = panic_payload.downcast_ref::<&str>() {
                                    s.to_string()
                                } else if let Some(s) = panic_payload.downcast_ref::<String>() {
                                    s.clone()
                                } else {
                                    "Unknown panic".to_string()
                                };
                                eprintln!(
                                    "[KnotenCore Panic] Caught panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                );
                                return Err(format!(
                                    "VM Panic in FFI call '{}.{}': {}",
                                    module, func, msg
                                ));
                            }
                        }
                    } else {
                        self.stack.push(RelType::Void); // Running without a connected FFI proxy
                    }
                }
                OpCode::AllocateDict => {
                    self.stack
                        .push(RelType::Dict(std::sync::Arc::new(std::sync::Mutex::new(
                            HashMap::new(),
                        ))));
                }
                OpCode::SetProperty => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    let key = self.stack.pop().unwrap_or(RelType::Void);
                    let obj = self.stack.pop().unwrap_or(RelType::Void);

                    if let (RelType::Dict(map_arc), RelType::Str(k)) = (&obj, &key) {
                        map_arc
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .insert(k.clone(), val);
                        self.stack.push(obj); // Push back the reference
                    } else if let (RelType::Object(map), RelType::Str(k)) = (&obj, &key) {
                        let mut new_map = map.clone();
                        new_map.insert(k.clone(), val);
                        self.stack.push(RelType::Object(new_map));
                    } else {
                        return Err("SetProperty expects (Dict/Object, Str, Any).".to_string());
                    }
                }
                OpCode::GetProperty => {
                    let key = self.stack.pop().unwrap_or(RelType::Void);
                    let obj = self.stack.pop().unwrap_or(RelType::Void);

                    if let (RelType::Dict(map_arc), RelType::Str(k)) = (&obj, &key) {
                        let res = map_arc
                            .lock()
                            .unwrap_or_else(|e| e.into_inner())
                            .get(k)
                            .cloned()
                            .unwrap_or(RelType::Void);
                        self.stack.push(res);
                    } else if let (RelType::Object(map), RelType::Str(k)) = (&obj, &key) {
                        let res = map.get(k).cloned().unwrap_or(RelType::Void);
                        self.stack.push(res);
                    } else {
                        // Silent fail mimicking optional structures or missing keys natively
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::Pop => {
                    self.stack.pop();
                }
                OpCode::Print => {
                    let val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Print".to_string())?;
                    println!("{}", val);
                }
                OpCode::Return => {
                    let ret_val = self
                        .stack
                        .pop()
                        .ok_or_else(|| "Stack underflow in Return".to_string())?;
                    self.stack.truncate(self.base_pointer); // Clean up the local variables / arguments frame

                    if let Some(frame) = self.frames.pop() {
                        // Return from function
                        self.ip = frame.ip;
                        self.base_pointer = frame.base_pointer;
                        self.stack.push(ret_val);
                    } else {
                        // Top level return exit
                        return Ok(ret_val);
                    }
                }
            }
        }

        Ok(self.stack.pop().unwrap_or(RelType::Void))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RelType;
    use crate::vm::opcode::OpCode;

    #[test]
    fn test_vm_execution_add() {
        let mut vm = VM::new();
        // Represents: 10 + 5
        let instructions = vec![
            OpCode::Constant(0), // Push 10
            OpCode::Constant(1), // Push 5
            OpCode::Add,         // Pop 5, Pop 10, Push 15
            OpCode::Return,
        ];
        let constants = vec![RelType::Int(10), RelType::Int(5)];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(15));
    }

    #[test]
    fn test_vm_execution_complex() {
        let mut vm = VM::new();
        // Represents: (10 - 2) * 3
        let instructions = vec![
            OpCode::Constant(0), // Push 10
            OpCode::Constant(1), // Push 2
            OpCode::Subtract,    // Pop 2, Pop 10, Push 8
            OpCode::Constant(2), // Push 3
            OpCode::Multiply,    // Pop 3, Pop 8, Push 24
            OpCode::Return,
        ];
        let constants = vec![RelType::Int(10), RelType::Int(2), RelType::Int(3)];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(24));
    }

    #[test]
    fn test_vm_jump_if_false() {
        let mut vm = VM::new();
        // Represents: if (false) { 10 } else { 20 }
        let instructions = vec![
            OpCode::Constant(0),    // Push false
            OpCode::JumpIfFalse(4), // If false, jump to index 4
            OpCode::Constant(1),    // Push 10
            OpCode::Jump(5),        // Jump to end (index 5)
            OpCode::Constant(2),    // Push 20 (index 4)
            OpCode::Return,         // Return (index 5)
        ];
        let constants = vec![RelType::Bool(false), RelType::Int(10), RelType::Int(20)];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(20));
    }

    #[test]
    fn test_vm_variables_and_strings() {
        let mut vm = VM::new();
        // script:
        // let pwd = "Test1"
        // let len = str_len(pwd)
        // return len
        let instructions = vec![
            OpCode::Constant(0),  // Push "Test1"
            OpCode::SetGlobal(1), // Set 'pwd'
            OpCode::GetGlobal(1), // Get 'pwd'
            OpCode::StringLength, // Length -> 5
            OpCode::SetGlobal(2), // Set 'len'
            OpCode::GetGlobal(2), // Get 'len'
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("Test1".to_string()),
            RelType::Str("pwd".to_string()),
            RelType::Str("len".to_string()),
        ];

        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                None,
            )
            .unwrap();
        assert_eq!(result, RelType::Int(5));
    }
    #[test]
    fn test_vm_network_sandbox_block() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push "https://api.github.com"
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // "net", "net_fetch", 1 arg
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("https://api.github.com".to_string()),
            RelType::Str("net".to_string()),
            RelType::Str("net_fetch".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        // Assert the fault is securely caught
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Permission Denied: allow_network is false")
        );
    }

    #[test]
    fn test_vm_network_get_sandbox_block() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push "https://api.github.com"
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // "net", "network_get", 1 arg
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("https://api.github.com".to_string()),
            RelType::Str("net".to_string()),
            RelType::Str("network_get".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Permission Denied: allow_network is false")
        );
    }

    #[test]
    fn test_vm_network_get_failed_url() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push invalid url
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            },
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("http://this-does-not-exist.invalid".to_string()),
            RelType::Str("net".to_string()),
            RelType::Str("network_get".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: true,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Network Error: HTTP Request Failed")
        );
    }

    #[test]
    fn test_vm_json_parsing() -> Result<(), String> {
        let mut vm = VM::new();
        // Valid JSON Test
        let instructions = vec![
            OpCode::Constant(0), // Push Valid JSON Object
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // block: json, json_parse
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("{\"api_version\":\"1.0\"}".to_string()),
            RelType::Str("json".to_string()),
            RelType::Str("json_parse".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm
            .run(
                &instructions,
                &constants,
                &AgentPermissions {
                    allow_network: false,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: false,
                },
                Some(&bridge),
            )
            .unwrap();

        // Ensure Map parses flawlessly capturing "api_version" natively
        if let RelType::Object(map) = result {
            assert_eq!(
                map.get("api_version").unwrap(),
                &RelType::Str("1.0".to_string())
            );
        } else {
            return Err("Expected JSON to parse into an Object natively!".into());
        }

        // Invalid JSON Test capturing gracefully Without Panics
        // Sprint 182: json_parse now returns Void on error instead of Fault
        let mut vm_err = VM::new();
        let constants_err = vec![
            RelType::Str("{ invalid...".to_string()),
            RelType::Str("json".to_string()),
            RelType::Str("json_parse".to_string()),
        ];
        let _ = vm_err.run(
            &instructions,
            &constants_err,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );

        // Sprint 183: file_read / file_write sandbox defense — missing permission must fault
        let mut vm3 = VM::new();
        let file_instructions = vec![
            OpCode::Constant(0), // Push path
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // file_read
            OpCode::Return,
        ];
        let file_constants = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_read".to_string()),
        ];
        let file_result = vm3.run(
            &file_instructions,
            &file_constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: false,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            file_result.is_err(),
            "file_read without allow_fs_read must fault"
        );
        assert!(file_result.unwrap_err().contains("Permission Denied"));

        let mut vm4 = VM::new();
        let write_instructions = vec![
            OpCode::Constant(0), // path
            OpCode::Constant(1), // content
            OpCode::ExternCall {
                name_idx: 4,
                arg_count: 2,
            }, // file_write
            OpCode::Return,
        ];
        let write_constants = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("test".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_write".to_string()),
            RelType::Str("file_write".to_string()),
        ];
        let write_result = vm4.run(
            &write_instructions,
            &write_constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            write_result.is_err(),
            "file_write without allow_fs_write must fault"
        );
        assert!(write_result.unwrap_err().contains("Permission Denied"));
        Ok(())
    }

    // Sprint 183: file_read / file_write sandbox defense — missing permission must fault
    #[test]
    fn test_vm_file_io_sandbox() {
        let mut vm = VM::new();
        let instructions = vec![
            OpCode::Constant(0), // Push path
            OpCode::ExternCall {
                name_idx: 2,
                arg_count: 1,
            }, // file_read
            OpCode::Return,
        ];
        let constants = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_read".to_string()),
        ];

        let bridge = crate::natives::bridge::CoreBridge;
        let result = vm.run(
            &instructions,
            &constants,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: false,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            result.is_err(),
            "file_read without allow_fs_read must fault"
        );
        assert!(result.unwrap_err().contains("Permission Denied"));

        // file_write without allow_fs_write must also fault
        let mut vm2 = VM::new();
        let instructions_write = vec![
            OpCode::Constant(0), // path
            OpCode::Constant(1), // content
            OpCode::ExternCall {
                name_idx: 4,
                arg_count: 2,
            }, // file_write
            OpCode::Return,
        ];
        let constants_write = vec![
            RelType::Str("examples/cache.json".to_string()),
            RelType::Str("test".to_string()),
            RelType::Str("fs".to_string()),
            RelType::Str("file_write".to_string()),
            RelType::Str("file_write".to_string()),
        ];
        let result2 = vm2.run(
            &instructions_write,
            &constants_write,
            &AgentPermissions {
                allow_network: false,
                allowed_domains: vec![],
                allow_fs_read: true,
                allow_fs_write: false,
            },
            Some(&bridge),
        );
        assert!(
            result2.is_err(),
            "file_write without allow_fs_write must fault"
        );
        assert!(result2.unwrap_err().contains("Permission Denied"));
    }

    fn run_logic_ops(ops: Vec<OpCode>, constants: Vec<RelType>) -> Result<RelType, String> {
        let mut vm = VM::new();
        let perms = AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: false,
            allow_fs_write: false,
        };
        vm.run(&ops, &constants, &perms, None)
    }

    #[test]
    fn test_vm_lte() {
        // 3 <= 5 → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::LessEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(3), RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // 5 <= 5 → true (boundary)
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::LessEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(true));
        // 7 <= 5 → false
        let r3 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::LessEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(7), RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r3, RelType::Bool(false));
    }

    #[test]
    fn test_vm_gte() {
        // 5 >= 3 → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::GreaterEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(5), RelType::Int(3)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // 5 >= 5 → true (boundary)
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::GreaterEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(true));
        // 3 >= 5 → false
        let r3 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::GreaterEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(3), RelType::Int(5)],
        )
        .unwrap();
        assert_eq!(r3, RelType::Bool(false));
    }

    #[test]
    fn test_vm_not_equal() {
        // 1 != 2 → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::NotEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(1), RelType::Int(2)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // 2 != 2 → false
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::NotEqual,
                OpCode::Return,
            ],
            vec![RelType::Int(2)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(false));
    }

    #[test]
    fn test_vm_and() {
        // true && true → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::And,
                OpCode::Return,
            ],
            vec![RelType::Bool(true)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // true && false → false
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::And,
                OpCode::Return,
            ],
            vec![RelType::Bool(true), RelType::Bool(false)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(false));
    }

    #[test]
    fn test_vm_or() {
        // false || true → true
        let r = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(1),
                OpCode::Or,
                OpCode::Return,
            ],
            vec![RelType::Bool(false), RelType::Bool(true)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(true));
        // false || false → false
        let r2 = run_logic_ops(
            vec![
                OpCode::Constant(0),
                OpCode::Constant(0),
                OpCode::Or,
                OpCode::Return,
            ],
            vec![RelType::Bool(false)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(false));
    }

    #[test]
    fn test_vm_not() {
        // !true → false
        let r = run_logic_ops(
            vec![OpCode::Constant(0), OpCode::Not, OpCode::Return],
            vec![RelType::Bool(true)],
        )
        .unwrap();
        assert_eq!(r, RelType::Bool(false));
        // !false → true
        let r2 = run_logic_ops(
            vec![OpCode::Constant(0), OpCode::Not, OpCode::Return],
            vec![RelType::Bool(false)],
        )
        .unwrap();
        assert_eq!(r2, RelType::Bool(true));
    }
}
