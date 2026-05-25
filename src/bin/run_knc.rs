use knoten_core::executor::ExecutionEngine;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

// Embedded at compile-time: absolute path to the knoten_core library source.
const KNOTEN_CORE_PATH: &str = env!("CARGO_MANIFEST_DIR");

fn main() {
    // Spawn with 8MB stack to support deep recursion in KnotenCore scripts
    let builder = std::thread::Builder::new().stack_size(8 * 1024 * 1024);
    let handler = builder
        .spawn(run)
        .expect("Failed to spawn KnotenCore runtime thread");
    handler.join().unwrap_or_else(|_| {
        eprintln!("[CLI Error] KnotenCore runtime thread panicked.");
        std::process::exit(1);
    });
}

fn run() {
    let mut engine = ExecutionEngine::new();
    engine.permissions.allow_fs_read = false;
    engine.permissions.allow_fs_write = false;

    let args: Vec<String> = env::args().collect();

    // ── Subcommand: build ─────────────────────────────────────────────
    // Usage: run_knc build <file.nod>
    if args.len() >= 2 && args[1] == "build" {
        if args.len() < 3 {
            eprintln!("Usage: run_knc build <path_to.nod>");
            std::process::exit(1);
        }
        build_standalone(&args[2]);
        return;
    }

    // ── Legacy flags & Permissions ─────────────────────────────────────
    let mut is_check = false;
    let mut no_opt = false;
    let mut transpile = false;
    let mut output_format_json = false;
    let mut file_path = String::new();

    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--check" {
            is_check = true;
        } else if arg == "--no-opt" {
            no_opt = true;
        } else if arg == "--transpile" {
            transpile = true;
        } else if arg == "--allow-read" {
            engine.permissions.allow_fs_read = true;
        } else if arg == "--allow-write" {
            engine.permissions.allow_fs_write = true;
        } else if arg == "--allow-net" {
            engine.permissions.allow_network = true;
        } else if arg == "--output-format" {
            if let Some(next_arg) = args.get(i + 1) {
                if next_arg == "json" {
                    output_format_json = true;
                }
                skip_next = true;
            }
        } else if arg.starts_with("--output-format=") && arg.ends_with("json") {
            output_format_json = true;
        } else {
            file_path = arg.clone();
        }
    }

    if output_format_json {
        std::panic::set_hook(Box::new(|_| {}));
    }

    // Check if we are bundled (Sprint 11) - Respects permissions set above
    if let Some(bundled_json) = option_env!("KNOTEN_BUNDLE") {
        println!("Running embedded KnotenCore bundle...");
        let ast = match serde_json::from_str(bundled_json) {
            Ok(ast) => ast,
            Err(e) => {
                eprintln!(
                    "[CLI Error] Failed to parse bundled KnotenCore JSON AST: {}",
                    e
                );
                std::process::exit(1);
            }
        };
        let result = engine.execute(&ast);
        println!("\nExecution Finished.\nResult: {}", result);
        return;
    }

    if file_path.is_empty() {
        eprintln!(
            "Usage: run_knc [--check] [--no-opt] [--transpile] [--allow-read] [--allow-write] [--allow-network] <path_to.nod>"
        );
        eprintln!("       run_knc build <path_to.nod>");
        std::process::exit(1);
    }

    if !output_format_json {
        println!(
            "CWD: {:?}",
            env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
        );
        println!("Loading KnotenCore Script: {}", file_path);
    }
    let json_string = match fs::read_to_string(&file_path) {
        Ok(s) => s,
        Err(e) => {
            if output_format_json {
                println!(
                    r#"{{"status": "error", "errors": [{{"code": "ERR_IO_PERMISSION", "message": "{}", "agent_hint": "Check if file exists or permissions are set."}}]}}"#,
                    e.to_string().replace("\"", "\\\"")
                );
                std::process::exit(1);
            } else {
                eprintln!("Failed to read file: {}", e);
                std::process::exit(1);
            }
        }
    };

    // Explicit syntax catch block gracefully handling parse errors internally yielding JSON validation outputs mapping ERR_UNKNOWN_NODE natively.
    let ast_result: Result<knoten_core::ast::Node, String> =
        if file_path.ends_with(".knoten") || file_path.ends_with(".nod") {
            // For AI Test Suite FAIL_01_unknown_node.nod JSON tests natively parse JSON properly
            if json_string.trim_start().starts_with('{') {
                serde_json::from_str(&json_string).map_err(|e| format!("JSON parse error: {}", e))
            } else {
                let mut parser = knoten_core::parser::Parser::new(&json_string);
                parser.parse().map_err(|e| format!("Parser error: {:?}", e))
            }
        } else {
            serde_json::from_str(&json_string).map_err(|e| format!("JSON parse error: {}", e))
        };

    let mut ast = match ast_result {
        Ok(node) => node,
        Err(_) => {
            if output_format_json {
                println!(
                    r#"{{"status": "error", "errors": [{{"code": "ERR_UNKNOWN_NODE", "message": "Unrecognized node type or parser fault", "agent_hint": "Check node_types.json! You probably emitted a hallucinated or deprecated node type instead of a valid AST node."}}]}}"#
                );
                std::process::exit(1);
            } else {
                eprintln!("Failed to parse KnotenCore AST.");
                std::process::exit(1);
            }
        }
    };

    let mut typer = knoten_core::optimizer::TypeChecker::new();
    let _ = typer.check(&ast);
    if !typer.errors.is_empty() {
        if output_format_json {
            println!(
                r#"{{"status": "error", "errors": [{{"code": "ERR_ARITY_MISMATCH", "message": "{}", "agent_hint": "Check the exact array structure for this node in node_types.json or nod_grammar.ebnf. Some parameters must be present, some can be optional."}}]}}"#,
                typer.errors[0].replace("\"", "\\\"")
            );
            std::process::exit(1);
        } else {
            eprintln!("\n[TypeError] Static Type Inference Failed:");
            for err in typer.errors {
                eprintln!(" - {}", err);
            }
            std::process::exit(1);
        }
    }

    if !no_opt {
        let before_nodes = knoten_core::optimizer::count_nodes(&ast);
        ast = knoten_core::optimizer::optimize(ast);
        let after_nodes = knoten_core::optimizer::count_nodes(&ast);
        if !output_format_json {
            println!(
                "Compiler Optimization: Reduced AST from {} to {} nodes.",
                before_nodes, after_nodes
            );
        }
    }

    if is_check {
        use knoten_core::validator::Validator;
        let mut validator = Validator::new();
        match validator.validate(&ast) {
            Ok(_) => {
                if !output_format_json {
                    println!("\nSyntax OK");
                }
                std::process::exit(0);
            }
            Err(errors) => {
                if output_format_json {
                    println!(
                        r#"{{"status": "error", "errors": [{{"code": "ERR_UNKNOWN_NODE", "message": "{}", "agent_hint": "Check node_types.json! You probably emitted a hallucinated or deprecated node type instead of a valid AST node."}}]}}"#,
                        errors[0].replace("\"", "\\\"")
                    );
                    std::process::exit(1);
                } else {
                    eprintln!("\nValidation Failed:");
                    for err in errors {
                        eprintln!(" - {}", err);
                    }
                    std::process::exit(1);
                }
            }
        }
    }

    // ── Main Thread Loop & Proxy Setup ─────────────────────────────
    use winit::event_loop::EventLoop;
    #[cfg(target_os = "windows")]
    use winit::platform::windows::EventLoopBuilderExtWindows;

    let mut builder = EventLoop::<knoten_core::natives::registry::RenderCommand>::with_user_event();
    #[cfg(target_os = "windows")]
    builder.with_any_thread(true);
    let event_loop = builder.build().expect("Failed to create event loop");

    // ── Pre-Execution Setup ──────────────────────────────────────────
    if transpile {
        eprintln!(
            "'--transpile' is not yet connected to the VM pipeline. Use the default bytecode path."
        );
        std::process::exit(1);
    }

    let proxy = event_loop.create_proxy();
    knoten_core::natives::registry::set_render_channel(proxy);

    let ast_arc = Arc::new(ast);
    let ast_for_thread = ast_arc.clone();
    let thread_engine = engine; // Move the engine with set permissions
    let file_path_clone = file_path.clone();

    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {
            let mut compiler = knoten_core::vm::compiler::Compiler::new();
            if let Some(parent) = std::path::Path::new(&file_path_clone).parent()
                && !parent.as_os_str().is_empty()
            {
                compiler.current_dir = parent.to_path_buf();
            }
            if !compiler.compile_node(&ast_for_thread) {
                eprintln!("\n[VM Crash] AST transpilation validation natively failed inline.");
                std::process::exit(1);
            }

            let mut vm = knoten_core::vm::machine::VM::new();
            let raw_result = vm.run(
                &compiler.instructions,
                &compiler.constants,
                &thread_engine.permissions,
                Some(&*thread_engine.bridge),
            );

            let result = match raw_result {
                Ok(v) => v.to_string(),
                Err(e) => e,
            };

            println!("\nExecution Finished.\nResult: {}", result);
            knoten_core::natives::registry::exit_event_loop();
        })
        .expect("Failed to spawn executor thread");

    let mut app = knoten_core::window::KnotenApp::new();
    let _ = event_loop.run_app(&mut app);
}

/// Full one-click build pipeline:
/// 1. Parse & optimise the .nod file
/// 2. Transpile to Rust source
/// 3. Scaffold a temporary Cargo project with knoten_core as a local dep
/// 4. `cargo build --release` with LTO enabled
/// 5. Copy the named binary back to the current working directory
fn build_standalone(nod_path: &str) {
    // ── Step 1: Parse & optimise ──────────────────────────────────────
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" KnotenCore Build Pipeline");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("[1/5] Parsing  : {}", nod_path);

    let json_string = fs::read_to_string(nod_path).unwrap_or_else(|_| {
        eprintln!("Error: Cannot read '{}'", nod_path);
        std::process::exit(1);
    });

    let mut ast: knoten_core::ast::Node = if nod_path.ends_with(".knoten") {
        let mut parser = knoten_core::parser::Parser::new(&json_string);
        parser.parse().unwrap_or_else(|e| {
            eprintln!("Error: Parser error — {:?}", e);
            std::process::exit(1);
        })
    } else {
        serde_json::from_str(&json_string).unwrap_or_else(|e| {
            eprintln!("Error: Invalid AST JSON — {}", e);
            std::process::exit(1);
        })
    };

    let before = knoten_core::optimizer::count_nodes(&ast);
    ast = knoten_core::optimizer::optimize(ast);
    let after = knoten_core::optimizer::count_nodes(&ast);
    println!("[2/5] Optimise : {} → {} nodes", before, after);

    // ── Step 2: Transpile ─────────────────────────────────────────────
    let rs_code = knoten_core::compiler::codegen::generate_rust_code(&ast);

    // Derive output binary name from the .nod filename stem
    let stem = Path::new(nod_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("knoten_app");

    println!("[3/5] Transpile: {} → {}.rs", nod_path, stem);

    // ── Step 3: Scaffold temp Cargo project ───────────────────────────
    let tmp_dir = std::env::temp_dir().join(format!("knoten_build_{}", stem));
    let src_dir = tmp_dir.join("src");
    if let Err(e) = fs::create_dir_all(&src_dir) {
        eprintln!("[CLI Error] Cannot create temp build directory: {}", e);
        std::process::exit(1);
    }

    // Cargo.toml — path dependency points to our library source
    let cargo_toml = format!(
        r#"[package]
name = "{stem}"
version = "0.1.0"
edition = "2021"

[dependencies]
knoten_core = {{ path = "{lib_path}" }}

[profile.release]
lto = "fat"
opt-level = 3
codegen-units = 1
strip = "symbols"
"#,
        stem = stem,
        lib_path = KNOTEN_CORE_PATH.replace('\\', "/"),
    );

    if let Err(e) = fs::write(tmp_dir.join("Cargo.toml"), &cargo_toml) {
        eprintln!("[CLI Error] Cannot write temporary Cargo.toml: {}", e);
        std::process::exit(1);
    }
    if let Err(e) = fs::write(src_dir.join("main.rs"), &rs_code) {
        eprintln!("[CLI Error] Cannot write temporary main.rs: {}", e);
        std::process::exit(1);
    }

    println!("[4/5] Compile  : cargo build --release (LTO + opt-level 3)");
    println!("      Build dir: {}", tmp_dir.display());

    // ── Step 4: Compile ───────────────────────────────────────────────
    let status = match Command::new("cargo")
        .args(["build", "--release"])
        .current_dir(&tmp_dir)
        .status()
    {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "[CLI Error] Failed to invoke cargo: {}. Is it installed and in PATH?",
                e
            );
            std::process::exit(1);
        }
    };

    if !status.success() {
        eprintln!("\n[Build FAILED] cargo exited with status {}", status);
        std::process::exit(1);
    }

    // ── Step 5: Copy binary to cwd ────────────────────────────────────
    let binary_name = if cfg!(windows) {
        format!("{}.exe", stem)
    } else {
        stem.to_string()
    };

    let built = tmp_dir.join("target").join("release").join(&binary_name);
    let dest = env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join(&binary_name);

    fs::copy(&built, &dest).unwrap_or_else(|e| {
        eprintln!("Could not copy binary: {}", e);
        std::process::exit(1);
    });

    println!(
        "[5/5] Done!    : {} ({} bytes)",
        dest.display(),
        fs::metadata(&dest).map(|m| m.len()).unwrap_or(0)
    );
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(" Binary ready — run it anywhere:");
    println!("   .\\{}", binary_name);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}
