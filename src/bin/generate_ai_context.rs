use std::fs;
use std::io::Write;
use std::path::Path;

fn main() {
    println!("Generating AI Context Bundle (Sprint 124)...");

    let output_path = Path::new("docs/knoten_ai_context_v124.md");
    let mut out = fs::File::create(output_path).expect("Failed to create context file");

    writeln!(out, "# KnotenCore AI Context Bundle v124").unwrap();
    writeln!(out, "This document contains the complete structural DSL constraints and grammar schemas enabling self-healing deterministic generative operations.").unwrap();

    let modules = vec![
        ("docs/LANGUAGE_REFERENCE/nod_grammar.ebnf", "EBNF Grammar Specification", "ebnf"),
        ("docs/LANGUAGE_REFERENCE/node_types.json", "Valid AST Node Types", "json"),
        ("docs/LANGUAGE_REFERENCE/native_functions.json", "Standard Native FFI Functions", "json"),
        ("docs/LANGUAGE_REFERENCE/error_catalog.json", "Error Code Output Matrix", "json"),
        ("docs/LANGUAGE_REFERENCE/examples/99_antipatterns.nod", "Antipatterns & Pitfalls", "javascript"),
    ];

    for (filepath, title, code_type) in modules {
        let path = Path::new(filepath);
        if !path.exists() {
            eprintln!("Warning: File not found: {}", filepath);
            continue;
        }

        let content = fs::read_to_string(path).expect("Failed to read file");
        writeln!(out, "\n## {}\n", title).unwrap();
        writeln!(out, "```{}", code_type).unwrap();
        writeln!(out, "{}", content).unwrap();
        writeln!(out, "```").unwrap();
    }

    println!("Success: Written generated context logically to {}", output_path.display());
}
