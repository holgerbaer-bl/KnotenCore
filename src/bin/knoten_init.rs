use std::env;
use std::fs;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: knoten-init --init | --cluster-sim");
        return;
    }

    match args[1].as_str() {
        "--init" => scaffold_workspace(),
        "--cluster-sim" => run_cluster_simulation(),
        _ => eprintln!("Unknown flag: {}", args[1]),
    }
}

fn scaffold_workspace() {
    let dirs = [".knoten_data/storage"];
    for d in &dirs {
        fs::create_dir_all(d).expect("Failed to create directory");
        println!("Created directory: {}", d);
    }

    let config = serde_json::json!({
        "version": "1.7.0",
        "cluster": {
            "nodes": ["Knoten_Berlin", "Knoten_Balingen", "Knoten_Zadar"],
            "default_shader": "data_preprocessor.wgsl"
        },
        "storage": {
            "dir": ".knoten_data/storage"
        }
    });
    let config_str = serde_json::to_string_pretty(&config).unwrap();
    fs::write("knoten_config.json", &config_str).expect("Failed to write config");
    println!("Created: knoten_config.json");

    let main_nod = serde_json::json!({
        "Main": [
            {"ExternCall": ["registry_play_boot_tone"]},
            {"Return": []}
        ]
    });
    let nod_str = serde_json::to_string_pretty(&main_nod).unwrap();
    fs::write("main.nod", &nod_str).expect("Failed to write main.nod");
    println!("Created: main.nod");

    println!("Workspace scaffolding complete.");
}

fn run_cluster_simulation() {
    use aether_compiler::vm::machine::VM;
    use aether_compiler::vm::scheduler;

    let nodes = ["Knoten_Berlin", "Knoten_Balingen", "Knoten_Zadar"];
    for &node in &nodes {
        println!("Initializing cluster node: {}", node);
        scheduler::push_cluster_work_batch(node, Vec::new());
    }

    let mut vm = VM::new();
    vm.globals.insert(
        "test".to_string(),
        aether_compiler::executor::RelType::Int(42),
    );
    let state = vm.snapshot();
    aether_compiler::vm::snapshot::store_snapshot(1, state);

    let registry = aether_compiler::vm::isolate::get_hot_swap_registry();
    registry.lock().unwrap().insert(
        1,
        std::sync::Arc::new(std::sync::Mutex::new((Vec::new(), Vec::new()))),
    );

    match scheduler::migrate_active_isolate(1, "Knoten_Zadar") {
        Ok(()) => println!("Cluster migration simulation succeeded."),
        Err(e) => eprintln!("Migration simulation failed: {}", e),
    }

    scheduler::drain_cluster_work_queues();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    #[test]
    fn test_cli_scaffolding_and_validation() {
        let tmp = std::env::temp_dir().join("knoten_test_scaffold");
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(&tmp).unwrap();
        let cwd = env::current_dir().unwrap();
        env::set_current_dir(&tmp).unwrap();

        scaffold_workspace();

        assert!(
            Path::new(".knoten_data/storage").exists(),
            "Storage directory must be created"
        );
        assert!(
            Path::new("knoten_config.json").exists(),
            "Config file must be created"
        );
        assert!(Path::new("main.nod").exists(), "main.nod must be created");

        let config_bytes = fs::read_to_string("knoten_config.json").unwrap();
        let config: serde_json::Value =
            serde_json::from_str(&config_bytes).expect("Config must be valid JSON");
        assert_eq!(config["version"], "1.7.0");

        let nod_bytes = fs::read_to_string("main.nod").unwrap();
        let nod: serde_json::Value =
            serde_json::from_str(&nod_bytes).expect("main.nod must be valid JSON");
        assert!(nod["Main"].is_array());

        env::set_current_dir(&cwd).unwrap();
        let _ = fs::remove_dir_all(&tmp);
    }
}
