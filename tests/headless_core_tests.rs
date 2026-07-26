// Sprint 306: Headless Core Tests — run under `cargo test --workspace --no-default-features`
//
// Tests verify:
//   1. Native Cast Opcodes (ToInt, ToFloat)
//   2. High-Performance String & Array Primitives (StringConcat, StringContains, ArraySlice)
//   3. Sandboxed In-Memory VFS (VfsRead, VfsWrite, VfsExists, VfsList)
//
// All tests are headless (no UI/GPU/display) and deterministic across Linux & Windows.

use aether_compiler::executor::{AgentPermissions, RelType};
use aether_compiler::vm::machine::VM;
use aether_compiler::vm::vfs::VirtualFs;
use knoten_core_types::opcode::OpCode;

/// Headless AgentPermissions with all host I/O disabled.
fn sandbox_perms() -> AgentPermissions {
    AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Cast Opcode Tests — ToInt
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_toint_from_float_truncates() {
    let mut vm = VM::default();
    let constants = vec![RelType::Float(3.9)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToInt, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Int(3), "3.9 truncated to 3");
}

#[test]
fn test_toint_negative_float_truncates() {
    let mut vm = VM::default();
    let constants = vec![RelType::Float(-2.7)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToInt, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Int(-2), "-2.7 truncated to -2");
}

#[test]
fn test_toint_from_int_is_noop() {
    let mut vm = VM::default();
    let constants = vec![RelType::Int(42)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToInt, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Int(42));
}

#[test]
fn test_toint_from_str() {
    let mut vm = VM::default();
    let constants = vec![RelType::Str("  99  ".to_string())];
    let instructions = vec![OpCode::Constant(0), OpCode::ToInt, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Int(99), "String '  99  ' parses to 99");
}

#[test]
fn test_toint_from_str_invalid_returns_error() {
    let mut vm = VM::default();
    let constants = vec![RelType::Str("abc".to_string())];
    let instructions = vec![OpCode::Constant(0), OpCode::ToInt, OpCode::Return];
    let result = vm.run(&instructions, &constants, &sandbox_perms(), None);
    assert!(result.is_err(), "ToInt of 'abc' must return an error");
}

#[test]
fn test_toint_from_bool_true() {
    let mut vm = VM::default();
    let constants = vec![RelType::Bool(true)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToInt, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Int(1));
}

#[test]
fn test_toint_from_bool_false() {
    let mut vm = VM::default();
    let constants = vec![RelType::Bool(false)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToInt, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Int(0));
}

// ─────────────────────────────────────────────────────────────────────────────
// Cast Opcode Tests — ToFloat
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_tofloat_from_int() {
    let mut vm = VM::default();
    let constants = vec![RelType::Int(7)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToFloat, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Float(7.0));
}

#[test]
fn test_tofloat_from_float_is_noop() {
    let mut vm = VM::default();
    let constants = vec![RelType::Float(3.14)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToFloat, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Float(3.14));
}

#[test]
fn test_tofloat_from_str() {
    let mut vm = VM::default();
    let constants = vec![RelType::Str("2.718".to_string())];
    let instructions = vec![OpCode::Constant(0), OpCode::ToFloat, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Float(2.718));
}

#[test]
fn test_tofloat_from_str_invalid_returns_error() {
    let mut vm = VM::default();
    let constants = vec![RelType::Str("xyz".to_string())];
    let instructions = vec![OpCode::Constant(0), OpCode::ToFloat, OpCode::Return];
    let result = vm.run(&instructions, &constants, &sandbox_perms(), None);
    assert!(result.is_err(), "ToFloat of 'xyz' must return an error");
}

#[test]
fn test_tofloat_from_bool() {
    let mut vm = VM::default();
    let constants = vec![RelType::Bool(true)];
    let instructions = vec![OpCode::Constant(0), OpCode::ToFloat, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Float(1.0));
}

// ─────────────────────────────────────────────────────────────────────────────
// String Primitive Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_string_concat_two_strings() {
    let mut vm = VM::default();
    let constants = vec![
        RelType::Str("Hello, ".to_string()),
        RelType::Str("World!".to_string()),
    ];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::StringConcat,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Str("Hello, World!".to_string()));
}

#[test]
fn test_string_concat_empty_strings() {
    let mut vm = VM::default();
    let constants = vec![
        RelType::Str("".to_string()),
        RelType::Str("suffix".to_string()),
    ];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::StringConcat,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Str("suffix".to_string()));
}

#[test]
fn test_string_contains_true() {
    let mut vm = VM::default();
    let constants = vec![
        RelType::Str("Hello, World!".to_string()),
        RelType::Str("World".to_string()),
    ];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::StringContains,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Bool(true));
}

#[test]
fn test_string_contains_false() {
    let mut vm = VM::default();
    let constants = vec![
        RelType::Str("Hello, World!".to_string()),
        RelType::Str("Rust".to_string()),
    ];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::StringContains,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Bool(false));
}

#[test]
fn test_string_contains_empty_needle_always_true() {
    let mut vm = VM::default();
    let constants = vec![
        RelType::Str("anything".to_string()),
        RelType::Str("".to_string()),
    ];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::StringContains,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(
        result,
        RelType::Bool(true),
        "Empty needle is always contained"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Array Primitive Tests — ArraySlice
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_array_slice_middle() {
    let mut vm = VM::default();
    let arr = RelType::Array(vec![
        RelType::Int(10),
        RelType::Int(20),
        RelType::Int(30),
        RelType::Int(40),
        RelType::Int(50),
    ]);
    let constants = vec![arr, RelType::Int(1), RelType::Int(4)];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::Constant(2),
        OpCode::ArraySlice,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(
        result,
        RelType::Array(vec![RelType::Int(20), RelType::Int(30), RelType::Int(40)])
    );
}

#[test]
fn test_array_slice_from_start() {
    let mut vm = VM::default();
    let arr = RelType::Array(vec![RelType::Int(1), RelType::Int(2), RelType::Int(3)]);
    let constants = vec![arr, RelType::Int(0), RelType::Int(2)];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::Constant(2),
        OpCode::ArraySlice,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(
        result,
        RelType::Array(vec![RelType::Int(1), RelType::Int(2)])
    );
}

#[test]
fn test_array_slice_out_of_bounds_clamped() {
    let mut vm = VM::default();
    let arr = RelType::Array(vec![RelType::Int(1), RelType::Int(2)]);
    // end=999 should be clamped to arr.len()=2
    let constants = vec![arr, RelType::Int(0), RelType::Int(999)];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::Constant(2),
        OpCode::ArraySlice,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(
        result,
        RelType::Array(vec![RelType::Int(1), RelType::Int(2)])
    );
}

#[test]
fn test_array_slice_empty_when_start_equals_end() {
    let mut vm = VM::default();
    let arr = RelType::Array(vec![RelType::Int(1), RelType::Int(2), RelType::Int(3)]);
    let constants = vec![arr, RelType::Int(1), RelType::Int(1)];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::Constant(2),
        OpCode::ArraySlice,
        OpCode::Return,
    ];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Array(vec![]));
}

// ─────────────────────────────────────────────────────────────────────────────
// Sandboxed VFS Opcode Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_vfs_write_and_read_via_opcodes() {
    let mut vm = VM::default();
    // Step 1: VfsWrite "/data.txt" "hello"
    let c_path = RelType::Str("/data.txt".to_string());
    let c_data = RelType::Str("hello vfs".to_string());
    let constants = vec![c_path, c_data];
    let write_instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::VfsWrite,
        OpCode::Return,
    ];
    let write_result = vm
        .run(&write_instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(write_result, RelType::Void);

    // Step 2: VfsRead "/data.txt" on the SAME vm (vfs persists across run calls)
    let c_path2 = RelType::Str("/data.txt".to_string());
    let constants2 = vec![c_path2];
    let read_instructions = vec![OpCode::Constant(0), OpCode::VfsRead, OpCode::Return];
    let read_result = vm
        .run(&read_instructions, &constants2, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(read_result, RelType::Str("hello vfs".to_string()));
}

#[test]
fn test_vfs_read_missing_returns_void() {
    let mut vm = VM::default();
    let constants = vec![RelType::Str("/missing.txt".to_string())];
    let instructions = vec![OpCode::Constant(0), OpCode::VfsRead, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Void);
}

#[test]
fn test_vfs_exists_opcode() {
    let mut vm = VM::default();

    // Write first
    let c_path = RelType::Str("/exists_test.txt".to_string());
    let c_data = RelType::Str("data".to_string());
    let write_instr = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::VfsWrite,
        OpCode::Return,
    ];
    vm.run(&write_instr, &[c_path, c_data], &sandbox_perms(), None)
        .unwrap();

    // Now check exists
    let c_path2 = RelType::Str("/exists_test.txt".to_string());
    let exists_instr = vec![OpCode::Constant(0), OpCode::VfsExists, OpCode::Return];
    let result = vm
        .run(&exists_instr, &[c_path2], &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Bool(true));
}

#[test]
fn test_vfs_exists_false_for_missing() {
    let mut vm = VM::default();
    let constants = vec![RelType::Str("/no_such_file.txt".to_string())];
    let instructions = vec![OpCode::Constant(0), OpCode::VfsExists, OpCode::Return];
    let result = vm
        .run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();
    assert_eq!(result, RelType::Bool(false));
}

#[test]
fn test_vfs_list_opcode() {
    let mut vm = VM::default();

    // Write two files under /data/
    let pairs = [
        ("/data/a.txt", "aaa"),
        ("/data/b.txt", "bbb"),
        ("/other/c.txt", "ccc"),
    ];
    for (path, content) in &pairs {
        let c_path = RelType::Str(path.to_string());
        let c_data = RelType::Str(content.to_string());
        let instr = vec![
            OpCode::Constant(0),
            OpCode::Constant(1),
            OpCode::VfsWrite,
            OpCode::Return,
        ];
        vm.run(&instr, &[c_path, c_data], &sandbox_perms(), None)
            .unwrap();
    }

    // List /data/
    let c_prefix = RelType::Str("/data/".to_string());
    let list_instr = vec![OpCode::Constant(0), OpCode::VfsList, OpCode::Return];
    let result = vm
        .run(&list_instr, &[c_prefix], &sandbox_perms(), None)
        .unwrap();
    match result {
        RelType::Array(entries) => {
            assert_eq!(entries.len(), 2, "Expected 2 files under /data/");
            let paths: Vec<String> = entries
                .iter()
                .map(|e| match e {
                    RelType::Str(s) => s.clone(),
                    _ => "".to_string(),
                })
                .collect();
            assert!(paths.contains(&"/data/a.txt".to_string()));
            assert!(paths.contains(&"/data/b.txt".to_string()));
        }
        _ => panic!("VfsList must return an Array"),
    }
}

#[test]
fn test_vfs_path_traversal_blocked_via_opcode() {
    let mut vm = VM::default();
    // Path with ".." should cause an error
    let constants = vec![
        RelType::Str("/../etc/passwd".to_string()),
        RelType::Str("evil data".to_string()),
    ];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::VfsWrite,
        OpCode::Return,
    ];
    let result = vm.run(&instructions, &constants, &sandbox_perms(), None);
    assert!(
        result.is_err(),
        "Path traversal via VfsWrite must be blocked"
    );
}

#[test]
fn test_vfs_is_isolated_from_host_fs() {
    // Write a file to the VFS and confirm it does NOT appear on the host filesystem
    let mut vm = VM::default();
    let vfs_path = "/sprint306_isolation_test.txt";
    let constants = vec![
        RelType::Str(vfs_path.to_string()),
        RelType::Str("secret".to_string()),
    ];
    let instructions = vec![
        OpCode::Constant(0),
        OpCode::Constant(1),
        OpCode::VfsWrite,
        OpCode::Return,
    ];
    vm.run(&instructions, &constants, &sandbox_perms(), None)
        .unwrap();

    // The file MUST NOT exist on the host OS
    // (On Windows this resolves to the drive root which also won't have this file)
    let host_exists = std::path::Path::new(vfs_path).exists();
    assert!(
        !host_exists,
        "VFS file must NOT exist on the host filesystem!"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// VirtualFs Unit Tests (direct API, no VM)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_virtual_fs_api_roundtrip() {
    let vfs = VirtualFs::new();
    vfs.write("/hello.txt", "world").unwrap();
    assert_eq!(vfs.read("/hello.txt").unwrap(), Some("world".to_string()));
    assert!(vfs.exists("/hello.txt").unwrap());
    assert!(!vfs.is_empty());
}

#[test]
fn test_virtual_fs_delete() {
    let vfs = VirtualFs::new();
    vfs.write("/temp.txt", "data").unwrap();
    assert!(vfs.delete("/temp.txt").unwrap());
    assert!(!vfs.exists("/temp.txt").unwrap());
    assert!(vfs.is_empty());
}
