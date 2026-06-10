use crate::executor::RelType;
use crate::vm::compiler::Compiler;
use crate::vm::machine::VM;
use knoten_core_types::opcode::OpCode;

pub fn wasm_instanciate_vm() -> VM {
    VM::new()
}

pub fn wasm_compile_json(json: &str) -> Option<(Vec<OpCode>, Vec<RelType>)> {
    let node: knoten_core_types::ast::Node = match serde_json::from_str(json) {
        Ok(n) => n,
        Err(_) => return None,
    };
    let mut compiler = Compiler::new();
    compiler.compile_node(&node);
    Some((compiler.instructions, compiler.constants))
}

pub fn wasm_dispatch_compute(
    vm: &mut VM,
    instructions: &[OpCode],
    constants: &[RelType],
) -> Result<RelType, String> {
    let perms = crate::executor::AgentPermissions::default();
    vm.run(instructions, constants, &perms, None)
}

pub fn wasm_edge_steal_work(thief_id: i64) -> Option<(OpCode, Vec<RelType>)> {
    crate::vm::scheduler::try_steal_cluster_work("wasm_edge_node", thief_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::RelType;
    use knoten_core_types::opcode::OpCode;

    #[test]
    fn test_wasm_edge_isolate_dispatches() {
        let json = "{\"Add\": [{\"IntLiteral\": 10}, {\"IntLiteral\": 5}]}";
        let (instructions, constants) =
            wasm_compile_json(json).expect("JSON compilation must succeed");
        assert!(!instructions.is_empty());

        let mut vm = wasm_instanciate_vm();
        let result = wasm_dispatch_compute(&mut vm, &instructions, &constants)
            .expect("WASM dispatch must succeed");
        assert_eq!(result, RelType::Int(15), "10 + 5 = 15 in WASM edge context");

        let constants_simple = vec![RelType::Int(7), RelType::Int(3)];
        let instructions_simple = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::Subtract,
            OpCode::Return,
        ];
        let mut vm2 = wasm_instanciate_vm();
        let r2 = wasm_dispatch_compute(&mut vm2, &instructions_simple, &constants_simple)
            .expect("WASM dispatch must succeed");
        assert_eq!(r2, RelType::Int(4), "7 - 3 = 4 in WASM edge context");

        let json_nested = "{\"Mul\": [{\"IntLiteral\": 6}, {\"IntLiteral\": 7}]}";
        let (instr, consts) = wasm_compile_json(json_nested).expect("Nested JSON must compile");
        let mut vm3 = wasm_instanciate_vm();
        let r3 =
            wasm_dispatch_compute(&mut vm3, &instr, &consts).expect("WASM dispatch must succeed");
        assert_eq!(r3, RelType::Int(42), "6 * 7 = 42 in WASM edge context");
    }
}
