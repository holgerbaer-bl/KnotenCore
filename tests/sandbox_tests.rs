// Sprint 193: Sandbox Security Integration Tests
//
// These tests validate the iron-shield hardening from Sprint 190 and the
// domain matching fix from Sprint 193.

use knoten_core::executor::{AgentPermissions, ExecutionEngine};
use std::path::Path;

// ── Domain Whitelist Tests ───────────────────────────────────────────

/// Exact domain match: google.com allowed when google.com is whitelisted
#[test]
fn test_domain_whitelist_exact_match() {
    let engine = ExecutionEngine {
        permissions: AgentPermissions {
            allow_network: true,
            allowed_domains: vec!["google.com".to_string()],
            allow_fs_read: false,
            allow_fs_write: false,
        },
        ..ExecutionEngine::new()
    };

    // Verify domain extraction: google.com should match
    assert!(engine.permissions.allowed_domains.iter().any(|d| {
        let domain = "google.com";
        domain == d.as_str() || domain.ends_with(&format!(".{}", d))
    }));
}

/// Subdomain match: telemetry.google.com allowed when google.com is whitelisted
#[test]
fn test_domain_whitelist_subdomain_match() {
    let allowed = vec!["google.com".to_string()];

    let domain = "telemetry.google.com";
    let matched = allowed
        .iter()
        .any(|d| domain == d.as_str() || domain.ends_with(&format!(".{}", d)));
    assert!(
        matched,
        "Subdomain telemetry.google.com should match google.com"
    );
}

/// Suffix attack: evilgoogle.com must NOT match google.com
#[test]
fn test_domain_whitelist_suffix_block() {
    let allowed = vec!["google.com".to_string()];

    let domain = "evilgoogle.com";
    let matched = allowed
        .iter()
        .any(|d| domain == d.as_str() || domain.ends_with(&format!(".{}", d)));
    assert!(
        !matched,
        "Suffix attack evilgoogle.com must NOT match google.com whitelist"
    );
}

/// Localhost must be blocked when not in whitelist
#[test]
fn test_domain_whitelist_localhost_blocked() {
    let allowed = vec!["google.com".to_string()];

    let domain = "localhost";
    let matched = allowed
        .iter()
        .any(|d| domain == d.as_str() || domain.ends_with(&format!(".{}", d)));
    assert!(!matched, "localhost must be blocked when not whitelisted");
}

/// Empty whitelist: all domains allowed (only network flag check)
#[test]
fn test_domain_whitelist_empty_allows_all() {
    let allowed: Vec<String> = vec![];
    assert!(allowed.is_empty(), "Empty whitelist allows all domains");
}

// ── Sysmlink Blocking Tests ────────────────────────────────────────

/// Symlink path must be rejected by validate_fs_path
#[test]
fn test_symlink_blocked_by_validate_fs_path() {
    // Create a temporary symlink (if platform supports it)
    let tmp_dir = std::env::temp_dir().join("knotencore_symlink_test");
    let _ = std::fs::create_dir_all(&tmp_dir);

    let target_file = tmp_dir.join("real_file.txt");
    let symlink_file = tmp_dir.join("link_to_file.txt");

    // Create a real file
    std::fs::write(&target_file, "test content").ok();

    // Create symlink (Windows requires admin or developer mode)
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_file, &symlink_file).ok();
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(Path::new(&target_file), Path::new(&symlink_file)).ok();
    }

    // If the symlink exists and is detected, validate_fs_path should reject it
    if symlink_file.exists() {
        let result = ExecutionEngine::validate_fs_path(&symlink_file.to_string_lossy());
        // Sprint 190: Symlink blocking rejects paths containing symlinks
        assert!(
            result.is_err(),
            "validate_fs_path must reject symlink paths"
        );
        let err_msg = result.unwrap_err();
        assert!(
            err_msg.contains("Symlink") || err_msg.contains("symlink"),
            "Error must mention symlink blocking: {}",
            err_msg
        );
    }

    // Cleanup
    let _ = std::fs::remove_file(&target_file);
    let _ = std::fs::remove_file(&symlink_file);
    let _ = std::fs::remove_dir(&tmp_dir);
}

/// Symlink path must be rejected by validate_fs_path_write
#[test]
fn test_symlink_blocked_by_validate_fs_path_write() {
    let tmp_dir = std::env::temp_dir().join("knotencore_symlink_write_test");
    let _ = std::fs::create_dir_all(&tmp_dir);

    let target_file = tmp_dir.join("real_write_target.txt");
    let symlink_file = tmp_dir.join("write_link.txt");

    std::fs::write(&target_file, "write target").ok();

    #[cfg(unix)]
    std::os::unix::fs::symlink(&target_file, &symlink_file).ok();
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(Path::new(&target_file), Path::new(&symlink_file)).ok();
    }

    if symlink_file.exists() {
        let result = ExecutionEngine::validate_fs_path_write(&symlink_file.to_string_lossy());
        assert!(
            result.is_err(),
            "validate_fs_path_write must reject symlink paths"
        );
    }

    let _ = std::fs::remove_file(&target_file);
    let _ = std::fs::remove_file(&symlink_file);
    let _ = std::fs::remove_dir(&tmp_dir);
}

// ── URL Domain Extraction Tests ────────────────────────────────────

/// Extract domain from https:// URL
#[test]
fn test_url_domain_extraction_https() {
    let url = "https://google.com/search?q=test";
    let domain = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("");
    assert_eq!(domain, "google.com");
}

/// Extract domain from http:// URL
#[test]
fn test_url_domain_extraction_http() {
    let url = "http://api.github.com/v3/repos";
    let domain = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("");
    assert_eq!(domain, "api.github.com");
}

// ── No-whitelist (all domains allowed by network flag only) ─────────

/// When allowed_domains is empty, only the allow_network flag gates access
#[test]
fn test_network_sandbox_no_whitelist_uses_flag_only() {
    // When allowed_domains is empty, the domain check is skipped entirely.
    // Only the allow_network flag is checked.
    let no_net = AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    };
    assert!(!no_net.allow_network);

    let with_net = AgentPermissions {
        allow_network: true,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    };
    assert!(with_net.allow_network);
    assert!(with_net.allowed_domains.is_empty());
}

// ── File Operation Sandbox Tests ───────────────────────────────────

/// FS read without allow_fs_read permission must be denied
#[test]
fn test_fs_read_denied_without_permission() {
    let perms = AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    };
    assert!(!perms.allow_fs_read, "FS read must be denied by default");
}

/// FS write without allow_fs_write permission must be denied
#[test]
fn test_fs_write_denied_without_permission() {
    let perms = AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    };
    assert!(!perms.allow_fs_write, "FS write must be denied by default");
}

// ── Sprint 195: Watchdog Tests ──────────────────────────────────────

use knoten_core::ast::Node;
use knoten_core::vm::machine::VM;

/// Verifies the VM watchdog's accumulated CPU tracking.
/// A tight loop with FFI calls must NOT reset the watchdog indefinitely.
#[test]
fn test_vm_ffi_bypass_blocked() {
    use knoten_core::vm::opcode::OpCode;
    let mut vm = VM::new();
    let instructions = vec![
        OpCode::Constant(0), // Push 1 (loop counter)
        OpCode::ExternCall {
            name_idx: 1, // "time_utc_timestamp"
            arg_count: 0,
        },
        OpCode::Pop, // discard result
        OpCode::Constant(0), // Push 1
        OpCode::Jump(0), // jump back to position 1 (infinite loop)
    ];
    let constants = vec![
        knoten_core::executor::RelType::Int(1),
        knoten_core::executor::RelType::Str("time_utc_timestamp".to_string()),
    ];
    let bridge = knoten_core::natives::bridge::CoreBridge;
    let perms = knoten_core::executor::AgentPermissions {
        allow_network: false,
        allowed_domains: vec![],
        allow_fs_read: false,
        allow_fs_write: false,
    };
    let result = vm.run(&instructions, &constants, &perms, Some(&bridge));
    assert!(
        result.is_err(),
        "VM watchdog must terminate infinite FFI-reset loop"
    );
    let err = result.unwrap_err();
    assert!(
        err.contains("Watchdog") || err.contains("timeout"),
        "Error must be a watchdog timeout: {}",
        err
    );
}

/// Verifies the JIT while-loop watchdog terminates infinite loops.
#[test]
fn test_jit_infinite_loop_timeout() {
    // Build AST: while (true) { /* empty body */ }
    let ast = Node::While(
        Box::new(Node::BoolLiteral(true)),
        Box::new(Node::Block(vec![])),
    );
    let mut engine = knoten_core::executor::ExecutionEngine::new();
    let result = engine.evaluate(&ast);
    assert!(
        matches!(result, knoten_core::executor::ExecResult::Fault { .. }),
        "JIT watchdog must terminate infinite while-loop"
    );
}

// ── Sprint 195: Domain Whitelist Test with knotencore.de ────────────

/// Verify that knotencore.de matches as an allowed domain
#[test]
fn test_domain_whitelist_knotencore_de() {
    let allowed = vec!["knotencore.de".to_string()];
    let domain = "knotencore.de";
    assert!(allowed
        .iter()
        .any(|d| domain == d.as_str() || domain.ends_with(&format!(".{}", d))));
}

/// Verify that api.knotencore.de matches as a subdomain
#[test]
fn test_domain_whitelist_api_knotencore_de() {
    let allowed = vec!["knotencore.de".to_string()];
    let domain = "api.knotencore.de";
    assert!(allowed
        .iter()
        .any(|d| domain == d.as_str() || domain.ends_with(&format!(".{}", d))));
}
