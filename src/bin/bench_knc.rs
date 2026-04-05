use std::env;
use std::fs;
use std::time::Instant;

use knoten_core::ast::Node;
use knoten_core::executor::{AgentPermissions, ExecutionEngine};
use knoten_core::vm::compiler::Compiler;
use knoten_core::vm::machine::VM;
use knoten_core::natives::bridge::CoreBridge;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: bench_knc <path_to_nod_file>");
        std::process::exit(1);
    }
    
    let file_path = &args[1];
    let source = fs::read_to_string(file_path).expect("Failed to read test file");
    
    // Parse mapping standard JSON serialization
    let ast: Node = serde_json::from_str(&source).expect("Failed to parse JSON AST");
    
    // Common permissions
    let perms = AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: true,
        allow_fs_write: true,
    };
    
    // --- 1. JIT EVALUATOR ---
    println!("═══════════════════════════════════════════════");
    println!(" JIT Evaluator Benchmark (evaluator.rs)");
    println!("═══════════════════════════════════════════════");
    
    let mut engine = ExecutionEngine::new();
    engine.bridge = Box::new(CoreBridge);
    engine.permissions = perms.clone();
    
    let jit_start = Instant::now();
    let res_jit = engine.evaluate(&ast);
    let jit_duration = jit_start.elapsed();
    
    println!("Result: {}", res_jit);
    println!("Time: {:.2} ms", jit_duration.as_secs_f64() * 1000.0);
    
    // --- 2. AOT COMPILER & VM ---
    println!("\n═══════════════════════════════════════════════");
    println!(" AOT Output Benchmark (machine.rs)");
    println!("═══════════════════════════════════════════════");
    
    let compile_start = Instant::now();
    let mut compiler = Compiler::new();
    if let Some(parent) = std::path::Path::new(file_path).parent() {
        if !parent.as_os_str().is_empty() {
            compiler.current_dir = parent.to_path_buf();
        }
    }
    
    if !compiler.compile_node(&ast) {
        eprintln!("\n[VM Crash] AST transpilation validation natively failed inline.");
        std::process::exit(1);
    }
    let compile_duration = compile_start.elapsed();
    
    let mut vm = VM::new();
    let vm_start = Instant::now();
    
    let raw_result = vm.run(
        &compiler.instructions, 
        &compiler.constants, 
        &perms, 
        Some(&*engine.bridge)
    );
    let vm_duration = vm_start.elapsed();
    
    let res_vm = match raw_result {
        Ok(v) => v.to_string(),
        Err(e) => format!("VM Evaluation Fault: {}", e),
    };
    
    println!("Result: {}", res_vm);
    println!("AOT Compile Time: {:.3} ms", compile_duration.as_secs_f64() * 1000.0);
    println!("AOT Runtime Time: {:.2} ms", vm_duration.as_secs_f64() * 1000.0);
    
    let speedup = jit_duration.as_secs_f64() / vm_duration.as_secs_f64();
    println!("\n🚀 Speedup: AOT VM is {:.2}x faster than JIT Evaluator!", speedup);
}
