// Sprint 319: Distributed Task Queue & Mesh Work-Stealing Engine
// Validates knc_task_submit, knc_task_status, knc_task_cancel, and
// the cooperative work-stealing protocol (knc_task_steal) across
// simulated single- and multi-node mesh configurations.

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{JsonRpcResponse, RpcServer, TaskDispatcher, TaskStatus};
use knoten_core_types::ast::Node;
use serde_json::Value;
use std::sync::Arc;

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn make_server() -> RpcServer {
    RpcServer::with_mesh(
        AgentPermissions::default(),
        "test-node-a",
        "127.0.0.1:0",
        None,
    )
}

fn make_authed_server(token: &str) -> RpcServer {
    RpcServer::with_mesh(
        AgentPermissions::default(),
        "test-node-b",
        "127.0.0.1:0",
        Some(token.to_string()),
    )
}

/// Minimal JSON-AST: `{ "IntLiteral": 42 }` serialised as a JSON-RPC request.
fn int_ast_param(value: i64) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_submit",
        "params": { "ast": { "IntLiteral": value } },
        "id": 1
    })
    .to_string()
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

fn submit_int_task(server: &RpcServer, value: i64) -> String {
    let raw = server.dispatch_request(&int_ast_param(value));
    let resp = parse_response(&raw);
    assert!(
        resp.error.is_none(),
        "submit should not error: {:?}",
        resp.error
    );
    result_field(&resp, "task_id").as_str().unwrap().to_string()
}

// ─────────────────────────────────────────────────────────────────────────────
// 1. Task Submission
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_task_submit_returns_queued_status() {
    let server = make_server();
    let req = int_ast_param(99);
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);

    assert!(resp.error.is_none(), "unexpected error: {:?}", resp.error);
    let result = resp.result.unwrap();
    assert_eq!(result["task_status"].as_str().unwrap(), "Queued");
    assert!(
        result["task_id"].as_str().is_some(),
        "task_id must be a string"
    );
}

#[test]
fn test_task_submit_invalid_ast_returns_error() {
    let server = make_server();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_submit",
        "params": { "ast": { "NotANode": true } },
        "id": 2
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_some(), "invalid AST must return an error");
}

#[test]
fn test_multiple_submissions_get_unique_ids() {
    let server = make_server();
    let id1 = submit_int_task(&server, 1);
    let id2 = submit_int_task(&server, 2);
    let id3 = submit_int_task(&server, 3);
    assert_ne!(id1, id2, "task IDs must be unique");
    assert_ne!(id2, id3);
    assert_ne!(id1, id3);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Status Polling
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_task_status_known_task_is_queued() {
    let server = make_server();
    let task_id = submit_int_task(&server, 7);

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_status",
        "params": { "task_id": task_id },
        "id": 10
    })
    .to_string();
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);

    assert!(
        resp.error.is_none(),
        "status of known task should not error"
    );
    assert_eq!(
        result_field(&resp, "task_status").as_str().unwrap(),
        "Queued"
    );
    assert_eq!(result_field(&resp, "task_id").as_str().unwrap(), task_id);
}

#[test]
fn test_task_status_unknown_task_returns_error() {
    let server = make_server();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_status",
        "params": { "task_id": "nonexistent-999" },
        "id": 11
    })
    .to_string();
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(
        resp.error.is_some(),
        "unknown task_id should return an error"
    );
}

#[test]
fn test_task_status_missing_param_returns_error() {
    let server = make_server();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_status",
        "params": {},
        "id": 12
    })
    .to_string();
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_some());
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Cancellation
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_task_cancel_queued_task_succeeds() {
    let server = make_server();
    let task_id = submit_int_task(&server, 100);

    let cancel_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_cancel",
        "params": { "task_id": task_id },
        "id": 20
    })
    .to_string();
    let raw = server.dispatch_request(&cancel_req);
    let resp = parse_response(&raw);

    assert!(resp.error.is_none());
    assert!(result_field(&resp, "cancelled").as_bool().unwrap());

    // Status should now be Cancelled
    let status_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_status",
        "params": { "task_id": task_id },
        "id": 21
    })
    .to_string();
    let status_raw = server.dispatch_request(&status_req);
    let status_resp = parse_response(&status_raw);
    assert_eq!(
        result_field(&status_resp, "task_status").as_str().unwrap(),
        "Cancelled"
    );
}

#[test]
fn test_task_cancel_unknown_task_returns_not_cancelled() {
    let server = make_server();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_cancel",
        "params": { "task_id": "ghost-task" },
        "id": 22
    })
    .to_string();
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());
    assert!(!result_field(&resp, "cancelled").as_bool().unwrap());
}

#[test]
fn test_cancel_already_running_task_is_idempotent() {
    let dispatcher = TaskDispatcher::new();
    let task_id = dispatcher.submit(Node::IntLiteral(5), 128).unwrap();

    // Simulate the task being picked up
    assert!(dispatcher.mark_running(&task_id, "worker-1"));

    // Cancel must return false (Running tasks cannot be cancelled via this path)
    assert!(!dispatcher.cancel(&task_id));

    // Status must remain Running
    let entry = dispatcher.status(&task_id).unwrap();
    assert_eq!(entry.status, TaskStatus::Running);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Work-Stealing (knc_task_steal)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_work_steal_claims_queued_tasks() {
    let server = make_server();

    // Submit 6 tasks
    let mut submitted_ids = Vec::new();
    for i in 0..6i64 {
        submitted_ids.push(submit_int_task(&server, i));
    }

    let steal_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_steal",
        "params": { "max_tasks": 4, "worker_node_id": "worker-peer-1" },
        "id": 30
    })
    .to_string();
    let raw = server.dispatch_request(&steal_req);
    let resp = parse_response(&raw);

    assert!(
        resp.error.is_none(),
        "steal must not error: {:?}",
        resp.error
    );
    let stolen = result_field(&resp, "stolen").as_array().unwrap();
    assert_eq!(stolen.len(), 4, "exactly 4 tasks should be stolen");

    // All stolen tasks must now be Running
    for item in stolen {
        let tid = item["task_id"].as_str().unwrap();
        let status_req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "knc_task_status",
            "params": { "task_id": tid },
            "id": 31
        })
        .to_string();
        let s_raw = server.dispatch_request(&status_req);
        let s_resp = parse_response(&s_raw);
        assert_eq!(
            result_field(&s_resp, "task_status").as_str().unwrap(),
            "Running",
            "stolen task {} should be Running",
            tid
        );
    }
}

#[test]
fn test_work_steal_empty_pool_returns_empty_array() {
    let server = make_server();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_steal",
        "params": { "max_tasks": 10, "worker_node_id": "idle-worker" },
        "id": 40
    })
    .to_string();
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());
    assert_eq!(
        result_field(&resp, "stolen").as_array().unwrap().len(),
        0,
        "empty pool should yield empty stolen array"
    );
}

#[test]
fn test_work_steal_respects_priority_order() {
    let dispatcher = Arc::new(TaskDispatcher::new());

    // Submit tasks with varying priority (lower = higher urgency)
    let id_low = dispatcher.submit(Node::IntLiteral(1), 255).unwrap(); // lowest priority
    let id_high = dispatcher.submit(Node::IntLiteral(2), 0).unwrap(); // highest priority
    let id_mid = dispatcher.submit(Node::IntLiteral(3), 128).unwrap(); // medium priority

    let stolen = dispatcher.steal(2, "priority-worker");
    assert_eq!(stolen.len(), 2);

    // Highest priority (0) must come first
    assert_eq!(
        stolen[0].task_id, id_high,
        "priority=0 must be stolen first"
    );
    assert_eq!(
        stolen[1].task_id, id_mid,
        "priority=128 must be stolen second"
    );

    // The low-priority task must still be Queued
    let remaining = dispatcher.status(&id_low).unwrap();
    assert_eq!(remaining.status, TaskStatus::Queued);
}

#[test]
fn test_work_steal_auth_required_on_authed_server() {
    let server = make_authed_server("secret-mesh-key");

    // Submit a task with auth token
    let submit_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_submit",
        "params": {
            "ast": { "IntLiteral": 0 },
            "mesh_auth_token": "secret-mesh-key"
        },
        "id": 1
    })
    .to_string();
    let raw_sub = server.dispatch_request(&submit_req);
    let resp_sub = parse_response(&raw_sub);
    assert!(resp_sub.error.is_none());

    // Steal without auth token → must be rejected
    let steal_req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_steal",
        "params": { "max_tasks": 1, "worker_node_id": "rogue-worker" },
        "id": 50
    })
    .to_string();
    let raw = server.dispatch_request(&steal_req);
    let resp = parse_response(&raw);
    assert!(
        resp.error.is_some(),
        "unauthenticated steal must return an auth error"
    );
    assert_eq!(resp.error.unwrap().code, -32001);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. TaskDispatcher direct-unit tests (no RPC layer)
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_dispatcher_submit_and_status() {
    let d = TaskDispatcher::new();
    let id = d.submit(Node::BoolLiteral(true), 0).unwrap();
    let entry = d.status(&id).unwrap();
    assert_eq!(entry.status, TaskStatus::Queued);
    assert_eq!(entry.priority, 0);
    assert!(entry.worker_node_id.is_none());
}

#[test]
fn test_dispatcher_complete_sets_result() {
    let d = TaskDispatcher::new();
    let id = d.submit(Node::IntLiteral(42), 128).unwrap();
    d.mark_running(&id, "test-worker");
    d.complete(&id, serde_json::json!({"value": 42}));

    let entry = d.status(&id).unwrap();
    assert_eq!(entry.status, TaskStatus::Completed);
    assert!(entry.result.is_some());
}

#[test]
fn test_dispatcher_fail_sets_error_message() {
    let d = TaskDispatcher::new();
    let id = d.submit(Node::IntLiteral(0), 128).unwrap();
    d.mark_running(&id, "fault-worker");
    d.fail(&id, "Division by zero");

    let entry = d.status(&id).unwrap();
    assert_eq!(entry.status, TaskStatus::Failed);
    assert_eq!(entry.result.unwrap().as_str().unwrap(), "Division by zero");
}

#[test]
fn test_dispatcher_stats_counts_correctly() {
    let d = TaskDispatcher::new();

    let id1 = d.submit(Node::IntLiteral(1), 0).unwrap();
    let id2 = d.submit(Node::IntLiteral(2), 0).unwrap();
    let id3 = d.submit(Node::IntLiteral(3), 0).unwrap();
    let _id4 = d.submit(Node::IntLiteral(4), 0).unwrap();

    d.cancel(&id1);
    d.mark_running(&id2, "w");
    d.complete(&id2, serde_json::json!(null));
    d.mark_running(&id3, "w");
    d.fail(&id3, "err");
    // id4 remains Queued

    let stats = d.stats();
    assert_eq!(stats["queued"].as_u64().unwrap(), 1, "1 queued");
    assert_eq!(stats["completed"].as_u64().unwrap(), 1, "1 completed");
    assert_eq!(stats["cancelled"].as_u64().unwrap(), 1, "1 cancelled");
    assert_eq!(stats["failed"].as_u64().unwrap(), 1, "1 failed");
    assert_eq!(stats["total"].as_u64().unwrap(), 4, "4 total");
}

#[test]
fn test_dispatcher_steal_max_zero_returns_empty() {
    let d = TaskDispatcher::new();
    d.submit(Node::IntLiteral(1), 0).unwrap();
    let stolen = d.steal(0, "w");
    assert!(stolen.is_empty());
}

// ─────────────────────────────────────────────────────────────────────────────
// 6. Handshake advertises task_queue capability
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_handshake_advertises_task_queue_and_work_stealing() {
    let server = make_server();
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {},
        "id": 99
    })
    .to_string();
    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);

    assert!(resp.error.is_none());
    let caps = result_field(&resp, "capabilities");
    assert_eq!(caps["task_queue"].as_bool(), Some(true));
    assert_eq!(caps["work_stealing"].as_bool(), Some(true));
    assert_eq!(
        result_field(&resp, "protocol_version").as_str().unwrap(),
        "v2.24.1"
    );
}

#[test]
fn test_task_queue_limit_rejection() {
    let dispatcher = TaskDispatcher::new();
    for i in 0..10_000 {
        dispatcher.submit(Node::IntLiteral(i), 128).unwrap();
    }
    let res = dispatcher.submit(Node::IntLiteral(10001), 128);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("capacity limit exceeded"));
}

#[test]
fn test_task_submit_cancel_status_auth_enforcement() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "task-auth-node",
        "127.0.0.1:0",
        Some("task-secret".to_string()),
    );

    let req_submit = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_submit",
        "params": { "ast": { "IntLiteral": 42 } },
        "id": 1
    })
    .to_string();
    let resp = parse_response(&server.dispatch_request(&req_submit));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);

    let req_status = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_status",
        "params": { "task_id": "1" },
        "id": 2
    })
    .to_string();
    let resp = parse_response(&server.dispatch_request(&req_status));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);

    let req_cancel = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_task_cancel",
        "params": { "task_id": "1" },
        "id": 3
    })
    .to_string();
    let resp = parse_response(&server.dispatch_request(&req_cancel));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);
}
