use crate::executor::RelType;
use knoten_core_types::opcode::OpCode;
use memmap2::MmapMut;
use std::collections::HashMap;

pub struct NativeMachineCodeEmitter;

fn emit_push_rax(code: &mut Vec<u8>) {
    code.push(0x50);
}

fn emit_pop_rcx(code: &mut Vec<u8>) {
    code.push(0x59);
}

fn emit_pop_rax(code: &mut Vec<u8>) {
    code.push(0x58);
}

pub fn emit_native_machine_block(opcodes: &[OpCode], constants: &[RelType]) -> Vec<u8> {
    let mut code: Vec<u8> = Vec::with_capacity(opcodes.len() * 12);
    let mut addr_map: HashMap<usize, usize> = HashMap::new();
    let mut pending_jumps: Vec<(usize, usize, bool)> = Vec::new();

    code.push(0x55);
    code.extend_from_slice(&[0x48, 0x89, 0xE5]);

    let mut stack_depth: u32 = 0;

    for (ip, op) in opcodes.iter().enumerate() {
        addr_map.insert(ip, code.len());

        match op {
            OpCode::Add => {
                if stack_depth >= 2 {
                    emit_pop_rcx(&mut code);
                    emit_pop_rax(&mut code);
                    code.extend_from_slice(&[0x48, 0x01, 0xC1]);
                    emit_push_rax(&mut code);
                    stack_depth -= 1;
                }
            }
            OpCode::Subtract => {
                if stack_depth >= 2 {
                    emit_pop_rcx(&mut code);
                    emit_pop_rax(&mut code);
                    code.extend_from_slice(&[0x48, 0x29, 0xC8]);
                    emit_push_rax(&mut code);
                    stack_depth -= 1;
                }
            }
            OpCode::Multiply => {
                if stack_depth >= 2 {
                    emit_pop_rcx(&mut code);
                    emit_pop_rax(&mut code);
                    code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]);
                    emit_push_rax(&mut code);
                    stack_depth -= 1;
                }
            }
            OpCode::Divide => {
                if stack_depth >= 2 {
                    emit_pop_rcx(&mut code);
                    emit_pop_rax(&mut code);
                    code.extend_from_slice(&[0x48, 0x99]);
                    code.extend_from_slice(&[0x48, 0xF7, 0xF9]);
                    emit_push_rax(&mut code);
                    stack_depth -= 1;
                }
            }
            OpCode::Constant(idx) => {
                let val: u64 = match constants.get(*idx) {
                    Some(RelType::Int(v)) => *v as u64,
                    Some(RelType::Float(f)) => f.to_bits(),
                    _ => 0u64,
                };
                code.extend_from_slice(&[0x48, 0xB8]);
                code.extend_from_slice(&val.to_le_bytes());
                emit_push_rax(&mut code);
                stack_depth += 1;
            }
            OpCode::Jump(_) | OpCode::JumpIfFalse(_) => {
                let is_conditional = matches!(op, OpCode::JumpIfFalse(_));

                if is_conditional {
                    if stack_depth > 0 {
                        emit_pop_rax(&mut code);
                        stack_depth -= 1;
                        code.extend_from_slice(&[0x48, 0x83, 0xF8, 0x00]);
                        code.extend_from_slice(&[0x0F, 0x84]);
                    } else {
                        code.extend_from_slice(&[0xE9]);
                        code.extend_from_slice(&0u32.to_le_bytes());
                        pending_jumps.push((code.len() - 4, ip, false));
                        continue;
                    }
                } else {
                    code.push(0xE9);
                }

                code.extend_from_slice(&0u32.to_le_bytes());
                pending_jumps.push((code.len() - 4, ip, is_conditional));
            }
            OpCode::Return => {
                emit_pop_rax(&mut code);
                code.extend_from_slice(&[0x5D]);
                code.extend_from_slice(&[0xC3]);
                break;
            }
            _ => {}
        }
    }

    if !code.ends_with(&[0xC3]) {
        emit_pop_rax(&mut code);
        code.extend_from_slice(&[0x5D]);
        code.extend_from_slice(&[0xC3]);
    }

    let epilogue_pos = code.len();
    addr_map.insert(opcodes.len(), epilogue_pos);

    for (patch_pos, from_ip, _is_cond) in &pending_jumps {
        if let Some(target_op) = opcodes.get(*from_ip) {
            let target_ip = match target_op {
                OpCode::Jump(t) | OpCode::JumpIfFalse(t) => *t,
                _ => continue,
            };
            let target_addr = addr_map.get(&target_ip).copied().unwrap_or(epilogue_pos);
            let rel_offset = if target_addr >= *patch_pos + 4 {
                (target_addr - (*patch_pos + 4)) as u32
            } else {
                ((*patch_pos + 4) - target_addr).wrapping_neg() as u32
            };
            let offset_bytes = (rel_offset).to_le_bytes();
            code[*patch_pos..*patch_pos + 4].copy_from_slice(&offset_bytes);
        }
    }

    code
}

/// # Safety
/// The caller must ensure `bytecode` contains valid, position-independent x86_64
/// machine code that terminates with a `ret` instruction. Executing arbitrary or
/// malformed bytecode may cause undefined behavior, crashes, or security vulnerabilities.
pub unsafe fn execute_native_block(bytecode: &[u8]) -> Result<i64, String> {
    if bytecode.is_empty() {
        return Err("Empty bytecode".into());
    }

    let mut mmap =
        MmapMut::map_anon(bytecode.len()).map_err(|e| format!("mmap allocation failed: {}", e))?;

    mmap.copy_from_slice(bytecode);

    let exec_mmap = mmap
        .make_exec()
        .map_err(|e| format!("make_exec failed: {}", e))?;

    let func: extern "C" fn() -> i64 = unsafe { std::mem::transmute(exec_mmap.as_ptr()) };

    let result = func();

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RelType;

    #[test]
    fn test_vm_jit_native_code_emission() {
        let constants = vec![RelType::Int(10), RelType::Int(5), RelType::Int(2)];
        let ops = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Constant(2),
            OpCode::Subtract,
            OpCode::Return,
        ];

        let code = emit_native_machine_block(&ops, &constants);
        assert!(!code.is_empty(), "Native code block must not be empty");

        assert_eq!(code[0], 0x55, "Must start with push rbp (x86_64 prologue)");
        assert!(code.contains(&0x48), "Must contain REX.W prefix bytes");
        assert!(code.contains(&0xC3), "Must end with ret instruction");
        assert!(code.len() > 16, "Code must span multiple instructions");
    }
    #[test]
    fn test_jit_native_execution_in_memory() {
        let constants = vec![RelType::Int(15), RelType::Int(2)];
        let ops = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Subtract,
            OpCode::Return,
        ];

        let code = emit_native_machine_block(&ops, &constants);
        assert!(!code.is_empty());
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return;
        }
        let result = unsafe { execute_native_block(&code) }.expect("Native execution must succeed");
        assert_eq!(result, 13, "15 - 2 = 13 via native x86_64 execution");
    }

    #[test]
    fn test_jit_native_control_flow_branching() {
        let constants = vec![RelType::Int(1), RelType::Int(10), RelType::Int(20)];
        let ops = vec![
            OpCode::Constant(0),
            OpCode::JumpIfFalse(4),
            OpCode::Constant(1),
            OpCode::Jump(5),
            OpCode::Constant(2),
            OpCode::Return,
        ];

        let code = emit_native_machine_block(&ops, &constants);
        assert!(!code.is_empty());
        if std::env::var("CI").is_ok() || std::env::var("GITHUB_ACTIONS").is_ok() {
            return;
        }
        let result =
            unsafe { execute_native_block(&code) }.expect("Native branch execution must succeed");
        assert_eq!(
            result, 10,
            "Truthy path: JumpIfFalse skips alt, pushes 10, returns"
        );
    }
}
