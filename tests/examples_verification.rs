// Sprint 347: Two-Tier Automated CI Verification Harness for Examples Directory (v2.24.10)
//
// Tier 1 (End-to-End Runtime Execution):
// Automatically parses, compiles, and executes all scripts under:
//   - examples/01_getting_started/
//   - examples/02_vector_and_compute/
// Asserts successful runtime termination without panics, gas exhaustion, or VM errors.
//
// Tier 2 (Syntax & Bytecode Compilation Integrity):
// Automatically parses and compiles all scripts under:
//   - examples/03_agents_and_zero_trust/
//     (Execution bypassed: Script demonstrates zero-trust quota-sandboxed agent isolate evaluation designed for multi-node cluster runtime / quota RPC harness)
//   - examples/04_interactive_and_ui/
//     (Execution bypassed: Script demonstrates egui interactive window rendering loops that require an active GUI window loop and display context)
// Asserts valid AST parsing and non-empty VM bytecode emission.

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use aether_compiler::vm::compiler::Compiler;
use aether_compiler::vm::machine::VM;
use knoten_core::parser::Parser;
use knoten_core::validator::Validator;
use std::fs;
use std::path::Path;

#[test]
fn test_version_assertion_sprint347() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.10");
    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_metrics",
        "params": {}
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"protocol_version\":\"v2.24.10\""));
}

#[test]
fn test_tier1_examples_runtime_execution() {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let tier1_folders = ["01_getting_started", "02_vector_and_compute"];

    let mut total_executed = 0;

    for folder in &tier1_folders {
        let dir = base_dir.join(folder);
        assert!(dir.exists(), "Tier 1 directory must exist: {:?}", dir);
        let mut script_count = 0;

        let entries =
            fs::read_dir(&dir).unwrap_or_else(|e| panic!("Failed to read dir {:?}: {}", dir, e));
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("knoten") {
                let source = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Failed to read script {:?}: {}", path, e));

                let mut parser = Parser::new(&source);
                let ast = parser
                    .parse()
                    .unwrap_or_else(|e| panic!("Parse failure in {:?}: {:?}", path, e));

                let mut validator = Validator::new();
                let _ = validator.validate(&ast);

                let mut compiler = Compiler::new();
                assert!(
                    compiler.compile_node(&ast),
                    "Compilation failed for {:?}",
                    path
                );

                let mut vm = VM::default();
                let perms = AgentPermissions {
                    allow_network: true,
                    allowed_domains: vec![],
                    allow_fs_read: true,
                    allow_fs_write: true,
                };
                let run_result = vm.run(&compiler.instructions, &compiler.constants, &perms, None);

                assert!(
                    run_result.is_ok(),
                    "Runtime execution failure in {:?}: {:?}",
                    path,
                    run_result.err()
                );

                script_count += 1;
                total_executed += 1;
            }
        }

        assert!(
            script_count >= 1,
            "Safeguard failed: Expected at least 1 script in Tier 1 folder '{}', found {}",
            folder,
            script_count
        );
    }

    assert!(
        total_executed >= 4,
        "Expected at least 4 total Tier 1 scripts executed, found {}",
        total_executed
    );
}

#[test]
fn test_tier2_examples_compilation_integrity() {
    let base_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples");
    let tier2_folders = ["03_agents_and_zero_trust", "04_interactive_and_ui"];

    let mut total_compiled = 0;

    for folder in &tier2_folders {
        let dir = base_dir.join(folder);
        assert!(dir.exists(), "Tier 2 directory must exist: {:?}", dir);
        let mut script_count = 0;

        let entries =
            fs::read_dir(&dir).unwrap_or_else(|e| panic!("Failed to read dir {:?}: {}", dir, e));
        for entry in entries {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("knoten") {
                let source = fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("Failed to read script {:?}: {}", path, e));

                let mut parser = Parser::new(&source);
                let ast = parser
                    .parse()
                    .unwrap_or_else(|e| panic!("Parse failure in {:?}: {:?}", path, e));

                let mut validator = Validator::new();
                let _ = validator.validate(&ast);

                let mut compiler = Compiler::new();
                assert!(
                    compiler.compile_node(&ast),
                    "Compilation failed for {:?}",
                    path
                );
                assert!(
                    !compiler.instructions.is_empty(),
                    "Bytecode emission empty for {:?}",
                    path
                );

                script_count += 1;
                total_compiled += 1;
            }
        }

        assert!(
            script_count >= 1,
            "Safeguard failed: Expected at least 1 script in Tier 2 folder '{}', found {}",
            folder,
            script_count
        );
    }

    assert!(
        total_compiled >= 3,
        "Expected at least 3 total Tier 2 scripts compiled, found {}",
        total_compiled
    );
}
