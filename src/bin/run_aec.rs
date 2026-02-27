use aether_compiler::executor::ExecutionEngine;
use aether_compiler::parser::Parser;
use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: run_aec <path_to.aec>");
        std::process::exit(1);
    }

    let file_path = &args[1];
    println!("Loading AetherCore Script: {}", file_path);

    let bytes = fs::read(file_path).expect("Failed to read file");
    let ast = Parser::parse_bytes(&bytes).expect("Failed to parse AetherCore AST Binary");

    let mut engine = ExecutionEngine::new();
    let result = engine.execute(&ast);

    println!("\nExecution Finished.\nResult: {}", result);
}
