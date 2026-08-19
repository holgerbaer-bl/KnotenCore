// Sprint 320: Cluster Metrics & Adaptive Work-Stealing Protocol
// Validates knc_mesh_metrics, MetricsCollector load simulation, auth guards,
// and adaptive work-stealing throttling under high CPU / memory load.

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{JsonRpcResponse, MetricsCollector, RpcServer};
use serde_json::Value;

fn make_server() -> RpcServer {
    RpcServer::with_mesh(
        AgentPermissions::default(),
        "metrics-node-a",
        "127.0.0.1:0",
        None,
    )
}

fn make_authed_server(token: &str) -> RpcServer {
    RpcServer::with_mesh(
        AgentPermissions::default(),
        "metrics-node-b",
        "127.0.0.1:0",
        Some(token.to_string()),
    )
}

fn parse_response(raw: &str) -> JsonRpcResponse {
    serde_json::from_str(raw).expect("response must be valid JSON-RPC 2.0")
}

fn result_field<'a>(resp: &'a JsonRpcResponse, key: &str) -> &'a Value {
    resp.result
        .as_ref()
        .unwrap_or_else(|| panic!("expected result, got error: {:?}", resp.error))
        .get(key)
        .unwrap_or_else(|| panic!("key '{}' missing in result", key))
}

fn submit_task(server: &RpcServer, val: i64) -> String {
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_submit",
        "params": { "ast": { "IntLiteral": val } },
        "id": 1
    })
    .to_string();
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    result_field(&resp, "task_id").as_str().unwrap().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Cluster Metrics Endpoint (knc_mesh_metrics)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_mesh_metrics_returns_node_metrics() {
    let server = make_server();
    submit_task(&server, 10);

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_metrics",
        "params": {},
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);

    assert_eq!(result_field(&resp, "status").as_str().unwrap(), "ok");
    assert_eq!(
        result_field(&resp, "node_id").as_str().unwrap(),
        "metrics-node-a"
    );
    assert_eq!(
        result_field(&resp, "protocol_version").as_str().unwrap(),
        "v2.24.8"
    );

    let metrics = result_field(&resp, "metrics");
    assert!(metrics.get("cpu_load_percent").is_some());
    assert!(metrics.get("memory_used_bytes").is_some());
    assert!(metrics.get("memory_total_bytes").is_some());
    assert!(metrics.get("memory_usage_percent").is_some());
    assert!(metrics.get("is_overloaded").is_some());

    let q_depth = metrics.get("task_queue_depth").unwrap();
    assert_eq!(q_depth["queued"].as_u64().unwrap(), 1);
}

#[test]
fn test_mesh_metrics_auth_required_on_authed_server() {
    let server = make_authed_server("secret-key-123");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_mesh_metrics",
        "params": {},
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(
        resp.error.is_some(),
        "unauthenticated metrics request must fail"
    );
    assert_eq!(resp.error.unwrap().code, -32001);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. MetricsCollector Unit Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_metrics_collector_simulated_load() {
    let collector = MetricsCollector::new();
    let stats = serde_json::json!({"queued": 2, "running": 1});

    let m1 = collector.collect(stats.clone());
    assert_eq!(m1.cpu_load_percent, 15.0);
    assert!(!m1.is_overloaded);

    // Override CPU load to 85% (>80% overload threshold)
    collector.set_simulated_cpu_load(Some(85.0));
    let m2 = collector.collect(stats.clone());
    assert_eq!(m2.cpu_load_percent, 85.0);
    assert!(m2.is_overloaded, "CPU load 85% must trigger overload flag");

    // Clear override
    collector.set_simulated_cpu_load(None);
    let m3 = collector.collect(stats);
    assert_eq!(m3.cpu_load_percent, 15.0);
    assert!(!m3.is_overloaded);
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Adaptive Work-Stealing Throttling Guard
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_work_stealing_normal_load_allows_stealing() {
    let server = make_server();
    submit_task(&server, 1);
    submit_task(&server, 2);

    let steal_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_steal",
        "params": { "max_tasks": 2, "worker_node_id": "idle-worker" },
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&steal_req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let stolen = result_field(&resp, "stolen").as_array().unwrap();
    assert_eq!(stolen.len(), 2);
    assert!(!result_field(&resp, "throttled").as_bool().unwrap());
}

#[test]
fn test_work_stealing_throttles_when_local_node_overloaded() {
    let server = make_server();
    submit_task(&server, 1);

    // Simulate local CPU load overload (> 80.0%)
    server.metrics_collector.set_simulated_cpu_load(Some(90.0));

    let steal_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_steal",
        "params": { "max_tasks": 2, "worker_node_id": "worker-1" },
        "id": 2
    })
    .to_string();

    let raw = server.dispatch_request(&steal_req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let stolen = result_field(&resp, "stolen").as_array().unwrap();
    assert!(
        stolen.is_empty(),
        "overloaded node must not release stolen tasks"
    );
    assert!(
        result_field(&resp, "throttled").as_bool().unwrap(),
        "throttled flag must be true under high CPU load"
    );
    assert!(
        result_field(&resp, "reason")
            .as_str()
            .unwrap()
            .contains("Throttled")
    );
}

#[test]
fn test_work_stealing_throttles_when_worker_cpu_param_overloaded() {
    let server = make_server();
    submit_task(&server, 1);

    // Requesting worker reports 88% CPU load
    let steal_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_steal",
        "params": {
            "max_tasks": 2,
            "worker_node_id": "overloaded-worker",
            "worker_cpu_load": 88.0
        },
        "id": 3
    })
    .to_string();

    let raw = server.dispatch_request(&steal_req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let stolen = result_field(&resp, "stolen").as_array().unwrap();
    assert!(stolen.is_empty());
    assert!(result_field(&resp, "throttled").as_bool().unwrap());
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Handshake Capability Verification
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_handshake_advertises_metrics_and_adaptive_stealing() {
    let server = make_server();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {},
        "id": 10
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let caps = result_field(&resp, "capabilities");
    assert!(caps["cluster_metrics"].as_bool().unwrap());
    assert!(caps["adaptive_work_stealing"].as_bool().unwrap());
    assert_eq!(
        result_field(&resp, "protocol_version").as_str().unwrap(),
        "v2.24.8"
    );
}
