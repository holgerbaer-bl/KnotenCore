use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: knoten_build <path_to.json>");
        std::process::exit(1);
    }

    let input_path = &args[1];
    let path = PathBuf::from(input_path);

    if !path.exists() {
        eprintln!("Error: File '{}' not found.", input_path);
        std::process::exit(1);
    }

    let mut absolute_path = fs::canonicalize(&path).expect("Failed to get absolute path");
    let mut temp_ast_path = None;

    if input_path.ends_with(".knoten") {
        println!("🔧 Compiling .knoten DSL to JSON AST...");
        let content = fs::read_to_string(&absolute_path).expect("Failed to read .knoten file");
        let mut parser = knoten_core::parser::Parser::new(&content);
        let ast = parser.parse().expect("Failed to parse .knoten file");
        let mut json_ast = serde_json::to_string(&ast).expect("Failed to serialize AST to JSON");

        // --- Bundler Import Resolution (Flattening) ---
        let re = regex::Regex::new(r#"\{"Import":"([^"]+)"\}"#).expect("Failed to compile regex");
        let mut previous_len = 0;
        // Keep flattening until no more imports remain (handles nested imports)
        while json_ast.len() != previous_len {
            previous_len = json_ast.len();
            json_ast = re
                .replace_all(&json_ast, |caps: &regex::Captures| {
                    let path_str = &caps[1];
                    let resolved_path = PathBuf::from(path_str);

                    // Only embed standard library imports for security
                    if !resolved_path.exists() {
                        eprintln!(
                            "⚠️ Warning: Failed to bundle import '{}' (File not found)",
                            path_str
                        );
                        return caps[0].to_string();
                    }

                    println!("📦 Bundling import '{}' into AST...", path_str);
                    let content = fs::read_to_string(&resolved_path).unwrap_or_default();
                    if content.trim().starts_with("{") || content.trim().starts_with("[") {
                        // It's already JSON AST (like stdlib/ui.nod)
                        content
                    } else {
                        // It's DSL (like core/time.nod), parse it to AST JSON
                        let mut p = knoten_core::parser::Parser::new(&content);
                        let imported_ast = match p.parse() {
                            Ok(ast) => ast,
                            Err(_) => return caps[0].to_string(),
                        };
                        serde_json::to_string(&imported_ast).unwrap_or_else(|_| caps[0].to_string())
                    }
                })
                .to_string();
        }
        // ----------------------------------------------

        let temp_path = PathBuf::from("_bundled_ast_temp.json");
        fs::write(&temp_path, json_ast).expect("Failed to write temporary JSON AST");
        absolute_path = fs::canonicalize(&temp_path).expect("Failed to get absolute temp path");
        temp_ast_path = Some(temp_path);
    }

    let absolute_path_str = absolute_path.to_str().expect("Path is not valid UTF-8");

    // Windows backslash fix for include_str!()
    let safe_path_str = absolute_path_str.replace("\\", "/");

    let file_stem = path
        .file_stem()
        .expect("Invalid filename")
        .to_str()
        .expect("Invalid UTF-8 filename");

    #[cfg(windows)]
    let out_file_name = format!("{}.exe", file_stem);
    #[cfg(not(windows))]
    let out_file_name = file_stem.to_string();

    println!(
        "🎨 KnotenCore Bundler: Compiling '{}' into standalone executable '{}'",
        input_path, out_file_name
    );

    // Generate the standalone launcher source file dynamically
    let launcher_source = format!(
        r#"
use knoten_core::executor::ExecutionEngine;
use std::sync::Arc;
use winit::event_loop::EventLoop;

fn main() {{
    println!("Running embedded KnotenCore bundle...");
    
    // Statically bake the JSON file content into the binary string section
    let bundled_json = include_str!("{}");
    
    let ast = serde_json::from_str(bundled_json)
        .expect("Failed to parse bundled KnotenCore JSON AST");

    // ── Event Loop Setup ─────────────────────────────────────────────
    let mut builder = EventLoop::<knoten_core::natives::registry::RenderCommand>::with_user_event();
    #[cfg(target_os = "windows")]
    {{
        use winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }}
    let event_loop = builder.build().expect("Failed to create event loop");
    let proxy = event_loop.create_proxy();
    knoten_core::natives::registry::set_render_channel(proxy);

    // ── Background Execution Thread ──────────────────────────────────
    let ast_arc = Arc::new(ast);
    std::thread::Builder::new()
        .stack_size(8 * 1024 * 1024)
        .spawn(move || {{
            let mut engine = ExecutionEngine::new();
            engine.permissions.allow_fs_read = true;
            engine.permissions.allow_fs_write = true;
            
            let result = engine.execute(&ast_arc);
            println!("\nExecution Finished.\nResult: {{}}", result);
            
            // Signal the main thread to exit if it's just a one-off execution
            knoten_core::natives::registry::exit_event_loop();
        }})
        .expect("Failed to spawn executor thread");

    // ── Main UI Thread ───────────────────────────────────────────────
    let mut app = knoten_core::window::KnotenApp::new();
    let _ = event_loop.run_app(&mut app);
}}
"#,
        safe_path_str
    );

    let temp_launcher_path = "src/bin/_knoten_temp_launcher.rs";
    fs::write(temp_launcher_path, launcher_source)
        .expect("Failed to write temporary launcher source file");

    // Call Cargo Build on the temporary binary hook
    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .arg("--bin")
        .arg("_knoten_temp_launcher")
        .status();

    // Clean up the temporary rust script immediately regardless of build success
    let _ = fs::remove_file(temp_launcher_path);

    if !status
        .expect("Failed to execute cargo build process")
        .success()
    {
        eprintln!("❌ Build failed.");
        std::process::exit(1);
    }

    // Locate the compiled binary
    #[cfg(windows)]
    let compiled_bin = PathBuf::from("target")
        .join("release")
        .join("_knoten_temp_launcher.exe");
    #[cfg(not(windows))]
    let compiled_bin = PathBuf::from("target")
        .join("release")
        .join("_knoten_temp_launcher");

    if !compiled_bin.exists() {
        eprintln!(
            "❌ Could not find the compiled executable at {:?}",
            compiled_bin
        );
        std::process::exit(1);
    }

    // Copy to the current directory matching the original json name
    let dest_path = PathBuf::from(&out_file_name);
    fs::copy(&compiled_bin, &dest_path).expect("Failed to copy executable to output directory");

    if let Some(ast_path) = temp_ast_path {
        let _ = fs::remove_file(ast_path);
    }

    println!("✅ Bundle Successful!");
    println!("Standalone Application created: {}", dest_path.display());
}
