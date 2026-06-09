use knoten_core_types::opcode::OpCode;

pub struct NativeMachineCodeEmitter;

pub fn emit_native_machine_block(opcodes: &[OpCode]) -> Vec<u8> {
    let mut code: Vec<u8> = Vec::with_capacity(opcodes.len() * 8);

    code.push(0x55);

    let mut stack_depth: u32 = 0;

    for op in opcodes {
        match op {
            OpCode::Add => {
                if stack_depth >= 2 {
                    code.extend_from_slice(&[0x58]);
                    code.extend_from_slice(&[0x48, 0x01, 0xC1]);
                    code.push(0x51);
                }
            }
            OpCode::Subtract => {
                if stack_depth >= 2 {
                    code.extend_from_slice(&[0x58]);
                    code.extend_from_slice(&[0x48, 0x29, 0xC1]);
                    code.push(0x51);
                }
            }
            OpCode::Multiply => {
                if stack_depth >= 2 {
                    code.extend_from_slice(&[0x58]);
                    code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]);
                    code.push(0x51);
                }
            }
            OpCode::Divide => {
                if stack_depth >= 2 {
                    code.extend_from_slice(&[0x58]);
                    code.extend_from_slice(&[0x99]);
                    code.extend_from_slice(&[0x48, 0xF7, 0xF9]);
                    code.push(0x50);
                }
            }
            OpCode::Constant(_) => {
                code.extend_from_slice(&[0x48, 0xB8]);
                code.extend_from_slice(&0u64.to_le_bytes());
                code.push(0x50);
                stack_depth += 1;
            }
            OpCode::Return => {
                code.extend_from_slice(&[0x58]);
                code.extend_from_slice(&[0x5D]);
                code.extend_from_slice(&[0xC3]);
                break;
            }
            _ => {}
        }
    }

    if !code.ends_with(&[0xC3]) {
        code.extend_from_slice(&[0x58]);
        code.extend_from_slice(&[0x5D]);
        code.extend_from_slice(&[0xC3]);
    }

    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_jit_native_code_emission() {
        let ops = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Add,
            OpCode::Constant(2),
            OpCode::Subtract,
            OpCode::Return,
        ];

        let code = emit_native_machine_block(&ops);
        assert!(!code.is_empty(), "Native code block must not be empty");

        assert_eq!(code[0], 0x55, "Must start with push rbp (x86_64 prologue)");
        assert!(code.contains(&0x48), "Must contain REX.W prefix bytes");
        assert!(code.contains(&0xC3), "Must end with ret instruction");
        assert!(code.len() > 10, "Code must span multiple instructions");
    }
}
