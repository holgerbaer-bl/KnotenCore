use crate::executor::{AgentPermissions, ExecutionEngine, RelType};
use crate::vm::opcode::OpCode;
use std::collections::HashMap;

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

        while self.ip < instructions.len() {
            let op = &instructions[self.ip];
            self.ip += 1;

            match op {
                OpCode::Constant(idx) => {
                    if *idx < constants.len() {
                        self.stack.push(constants[*idx].clone());
                    } else {
                        return Err("Constant index out of bounds".into());
                    }
                }
                OpCode::Add => {
                    let r = self.stack.pop().ok_or_else(|| "Stack underflow in Add".to_string())?;
                    let l = self.stack.pop().ok_or_else(|| "Stack underflow in Add".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a + b)),
                        (RelType::Float(a), RelType::Float(b)) => self.stack.push(RelType::Float(a + b)),
                        (RelType::Int(a), RelType::Float(b)) => self.stack.push(RelType::Float(a as f64 + b)),
                        (RelType::Float(a), RelType::Int(b)) => self.stack.push(RelType::Float(a + b as f64)),
                        (RelType::Str(a), RelType::Str(b)) => self.stack.push(RelType::Str(a + &b)),
                        _ => return Err("Invalid types for Add".into()),
                    }
                }
                OpCode::Subtract => {
                    let r = self.stack.pop().ok_or_else(|| "Stack underflow in Subtract".to_string())?;
                    let l = self.stack.pop().ok_or_else(|| "Stack underflow in Subtract".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a - b)),
                        (RelType::Float(a), RelType::Float(b)) => self.stack.push(RelType::Float(a - b)),
                        (RelType::Int(a), RelType::Float(b)) => self.stack.push(RelType::Float(a as f64 - b)),
                        (RelType::Float(a), RelType::Int(b)) => self.stack.push(RelType::Float(a - b as f64)),
                        _ => return Err("Invalid types for Subtract".into()),
                    }
                }
                OpCode::Multiply => {
                    let r = self.stack.pop().ok_or_else(|| "Stack underflow in Multiply".to_string())?;
                    let l = self.stack.pop().ok_or_else(|| "Stack underflow in Multiply".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Int(a * b)),
                        (RelType::Float(a), RelType::Float(b)) => self.stack.push(RelType::Float(a * b)),
                        (RelType::Int(a), RelType::Float(b)) => self.stack.push(RelType::Float(a as f64 * b)),
                        (RelType::Float(a), RelType::Int(b)) => self.stack.push(RelType::Float(a * b as f64)),
                        _ => return Err("Invalid types for Multiply".into()),
                    }
                }
                OpCode::Divide => {
                    let r = self.stack.pop().ok_or_else(|| "Stack underflow in Divide".to_string())?;
                    let l = self.stack.pop().ok_or_else(|| "Stack underflow in Divide".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => {
                            if b == 0 { return Err("Div by zero".into()); }
                            self.stack.push(RelType::Int(a / b))
                        },
                        (RelType::Float(a), RelType::Float(b)) => {
                            if b == 0.0 { return Err("Div by zero".into()); }
                            self.stack.push(RelType::Float(a / b))
                        },
                        _ => return Err("Invalid types for Divide".into()),
                    }
                }
                OpCode::Equal => {
                    let r = self.stack.pop().ok_or_else(|| "Stack underflow in Equal".to_string())?;
                    let l = self.stack.pop().ok_or_else(|| "Stack underflow in Equal".to_string())?;
                    self.stack.push(RelType::Bool(l == r));
                }
                OpCode::Less => {
                    let r = self.stack.pop().ok_or_else(|| "Stack underflow in Less".to_string())?;
                    let l = self.stack.pop().ok_or_else(|| "Stack underflow in Less".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Bool(a < b)),
                        (RelType::Float(a), RelType::Float(b)) => self.stack.push(RelType::Bool(a < b)),
                        _ => return Err("Invalid types for Less comparison".into()),
                    }
                }
                OpCode::Greater => {
                    let r = self.stack.pop().ok_or_else(|| "Stack underflow in Greater".to_string())?;
                    let l = self.stack.pop().ok_or_else(|| "Stack underflow in Greater".to_string())?;
                    match (l, r) {
                        (RelType::Int(a), RelType::Int(b)) => self.stack.push(RelType::Bool(a > b)),
                        (RelType::Float(a), RelType::Float(b)) => self.stack.push(RelType::Bool(a > b)),
                        _ => return Err("Invalid types for Greater comparison".into()),
                    }
                }
                OpCode::JumpIfFalse(target_ip) => {
                    let cond = self.stack.pop().ok_or_else(|| "Stack underflow in JumpIfFalse".to_string())?;
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
                    let val = self.stack.pop().ok_or_else(|| "Stack underflow in SetLocal".to_string())?;
                    let target_idx = self.base_pointer + *idx;
                    // Dynamically allocate stack for isolated variables
                    if target_idx >= self.stack.len() {
                        self.stack.resize(target_idx + 1, RelType::Void);
                    }
                    self.stack[target_idx] = val;
                }
                OpCode::GetLocal(idx) => {
                    let val = self.stack.get(self.base_pointer + *idx)
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
                    let val = self.stack.pop().ok_or_else(|| "Stack underflow in SetGlobal".to_string())?;
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
                    let val = self.stack.pop().ok_or_else(|| "Stack underflow in StringLength".to_string())?;
                    if let RelType::Str(s) = val {
                        self.stack.push(RelType::Int(s.chars().count() as i64));
                    } else {
                        return Err("StringLength expects a Str".into());
                    }
                }
                OpCode::StringContainsChars => {
                    let pattern = self.stack.pop().ok_or_else(|| "Stack underflow in StringContainsChars".to_string())?;
                    let target = self.stack.pop().ok_or_else(|| "Stack underflow in StringContainsChars".to_string())?;
                    if let (RelType::Str(s), RelType::Str(p)) = (target, pattern) {
                        let contains = match p.as_str() {
                            "numbers" => s.chars().any(|c| c.is_ascii_digit()),
                            "special" => s.chars().any(|c| !c.is_ascii_alphanumeric() && !c.is_whitespace()),
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
                    let delim = self.stack.pop().ok_or_else(|| "Stack underflow in StringSplit".to_string())?;
                    let target = self.stack.pop().ok_or_else(|| "Stack underflow in StringSplit".to_string())?;
                    if let (RelType::Str(s), RelType::Str(d)) = (target, delim) {
                        let parts = s.split(&d).map(|part| RelType::Str(part.to_string())).collect();
                        self.stack.push(RelType::Array(parts));
                    } else {
                        self.stack.push(RelType::Void);
                    }
                }
                OpCode::ArrayContains => {
                    let search = self.stack.pop().ok_or_else(|| "Stack underflow in ArrayContains".to_string())?;
                    let array = self.stack.pop().ok_or_else(|| "Stack underflow in ArrayContains".to_string())?;
                    if let (RelType::Array(arr), search_val) = (array, search) {
                        self.stack.push(RelType::Bool(arr.contains(&search_val)));
                    } else {
                        self.stack.push(RelType::Bool(false));
                    }
                }
                OpCode::ReadFile => {
                    let path_val = self.stack.pop().ok_or_else(|| "Stack underflow in ReadFile".to_string())?;
                    if let RelType::Str(path) = path_val {
                        if !permissions.allow_fs_read {
                            return Err("Permission Denied: allow_fs_read is false (VM: ReadFile)".into());
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
                OpCode::ExternCall { name_idx, arg_count } => {
                    let name = match constants.get(*name_idx) {
                        Some(RelType::Str(s)) => s.clone(),
                        _ => return Err("OpExternCall: valid function name not found in constant pool".to_string()),
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
                    } else if name.starts_with("fs_") {
                        ("fs", name.as_str())
                    } else if name.starts_with("test_") {
                        ("test_lib", name.as_str())
                    } else {
                        // Global scope for unmapped builtins if the user writes flat names
                        ("global", name.as_str())
                    };

                    if let Some(b) = bridge {
                        match b.handle(module, func, &args, permissions) {
                            Some(crate::executor::ExecResult::Value(v)) => self.stack.push(v),
                            Some(crate::executor::ExecResult::Fault { msg, .. }) => return Err(format!("FFI Fault: {}", msg)),
                            None => return Err(format!("FFI Function '{}.{}' not handled by active BridgeModule", module, func)),
                            _ => self.stack.push(RelType::Void),
                        }
                    } else {
                        self.stack.push(RelType::Void); // Running without a connected FFI proxy
                    }
                }
                OpCode::AllocateDict => {
                    self.stack.push(RelType::Dict(std::sync::Arc::new(std::sync::Mutex::new(HashMap::new()))));
                }
                OpCode::SetProperty => {
                    let val = self.stack.pop().unwrap_or(RelType::Void);
                    let key = self.stack.pop().unwrap_or(RelType::Void);
                    let obj = self.stack.pop().unwrap_or(RelType::Void);
                    
                    if let (RelType::Dict(map_arc), RelType::Str(k)) = (&obj, key) {
                        map_arc.lock().unwrap().insert(k, val);
                        self.stack.push(obj); // Push back the reference
                    } else {
                        return Err("SetProperty expects (Dict, Str, Any).".to_string());
                    }
                }
                OpCode::GetProperty => {
                    let key = self.stack.pop().unwrap_or(RelType::Void);
                    let obj = self.stack.pop().unwrap_or(RelType::Void);
                    
                    if let (RelType::Dict(map_arc), RelType::Str(k)) = (&obj, key) {
                        let res = map_arc.lock().unwrap().get(&k).cloned().unwrap_or(RelType::Void);
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
                    let val = self.stack.pop().ok_or_else(|| "Stack underflow in Print".to_string())?;
                    println!("{}", val);
                }
                OpCode::Return => {
                    let ret_val = self.stack.pop().ok_or_else(|| "Stack underflow in Return".to_string())?;
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

        let result = vm.run(&instructions, &constants, &AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: true,
            allow_fs_write: false,
        }, None).unwrap();
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

        let result = vm.run(&instructions, &constants, &AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: true,
            allow_fs_write: false,
        }, None).unwrap();
        assert_eq!(result, RelType::Int(24));
    }

    #[test]
    fn test_vm_jump_if_false() {
        let mut vm = VM::new();
        // Represents: if (false) { 10 } else { 20 }
        let instructions = vec![
            OpCode::Constant(0),       // Push false
            OpCode::JumpIfFalse(4),    // If false, jump to index 4
            OpCode::Constant(1),       // Push 10
            OpCode::Jump(5),           // Jump to end (index 5)
            OpCode::Constant(2),       // Push 20 (index 4)
            OpCode::Return,            // Return (index 5)
        ];
        let constants = vec![RelType::Bool(false), RelType::Int(10), RelType::Int(20)];

        let result = vm.run(&instructions, &constants, &AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: true,
            allow_fs_write: false,
        }, None).unwrap();
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
            OpCode::Constant(0), // Push "Test1"
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
        
        let result = vm.run(&instructions, &constants, &AgentPermissions {
            allow_network: false,
            allowed_domains: vec![],
            allow_fs_read: true,
            allow_fs_write: false,
        }, None).unwrap();
        assert_eq!(result, RelType::Int(5));
    }
}
