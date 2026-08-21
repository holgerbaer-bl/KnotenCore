// Sprint 338: Architectural Modularization & Codebase Detox Tests (v2.23.1)

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{
    CrdtEntry, HmrReport, JsonRpcRequest, JsonRpcResponse, KNC_PROTOCOL_VERSION, MeshPeer,
    NodeRole, NonceCache, RpcServer, RpcSession, SwarmGovernance, TaskDispatcher, TaskStatus,
};
use knoten_core_types::ast::Node;
use serde_json::json;

#[test]
fn test_version_assertion_sprint338() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19"
    );
}

#[test]
fn test_version_assertion_sprint351() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19 for Sprint 351"
    );
}

#[test]
fn test_version_assertion_sprint352() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19 for Sprint 352"
    );
}

#[test]
fn test_version_assertion_sprint353() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19 for Sprint 353"
    );
}

#[test]
fn test_version_assertion_sprint354() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19 for Sprint 354"
    );
}

#[test]
fn test_version_assertion_sprint355() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19 for Sprint 355"
    );
}

#[test]
fn test_version_assertion_sprint356() {
    assert_eq!(
        KNC_PROTOCOL_VERSION, "v2.24.19",
        "Protocol version must be synchronized to v2.24.19 for Sprint 356"
    );
}

#[test]
fn test_rpc_reexports_and_types_accessibility() {
    // Verify public re-exports from aether_compiler::rpc
    let req = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: "knc_compile".to_string(),
        params: json!({}),
        id: Some(json!(1)),
    };
    assert_eq!(req.jsonrpc, "2.0");

    let resp = JsonRpcResponse::success(Some(json!(1)), json!({"status": "ok"}));
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_some());

    let peer = MeshPeer {
        node_id: "test-node".to_string(),
        address: "127.0.0.1:9090".to_string(),
        capabilities: vec!["mesh_ping".to_string()],
        last_seen: 100,
        latency_ms: 5,
        status: "Active".to_string(),
    };
    assert_eq!(peer.status, "Active");

    let mut cache = NonceCache::new();
    assert!(cache.insert("node1:nonce1".to_string(), 100));
    assert!(!cache.insert("node1:nonce1".to_string(), 100));

    let mut session = RpcSession::default();
    let ast = Node::IntLiteral(42);
    let report: HmrReport = session.hot_reload_code(&ast).expect("Hot reload success");
    assert!(report.reloaded);

    let crdt = CrdtEntry {
        key: "k1".to_string(),
        value: json!("v1"),
        timestamp: 100,
        writer_id: "w1".to_string(),
    };
    assert_eq!(crdt.key, "k1");

    let gov = SwarmGovernance::new();
    assert_eq!(gov.role(), NodeRole::Worker);

    let dispatcher = TaskDispatcher::new();
    assert_eq!(dispatcher.gc_completed(), 0);
    assert_eq!(TaskStatus::Queued, TaskStatus::Queued);
}

#[test]
fn test_all_28_rpc_methods_dispatchability() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "node-modular-master",
        "127.0.0.1:9100",
        None,
    );

    let methods = vec![
        "knc_compile",
        "knc_execute",
        "knc_yield_resume",
        "knc_inspect_state",
        "knc_agent_handshake",
        "knc_agent_snapshot",
        "knc_agent_restore",
        "knc_mesh_discover",
        "knc_mesh_peers",
        "knc_agent_teleport",
        "knc_task_submit",
        "knc_task_status",
        "knc_task_cancel",
        "knc_task_steal",
        "knc_mesh_ping",
        "knc_mesh_metrics",
        "knc_store_put",
        "knc_store_get",
        "knc_store_sync",
        "knc_swarm_elect",
        "knc_swarm_roles",
        "knc_swarm_quorum",
        "knc_swarm_request_vote",
        "knc_swarm_heartbeat",
        "knc_mesh_verify_peer",
        "knc_mesh_rotate_key",
        "knc_mesh_revoke_peer",
        "knc_isolate_reload",
    ];

    assert_eq!(methods.len(), 28, "Must test exactly 28 JSON-RPC endpoints");

    for method in methods {
        let req = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": {},
            "id": 1
        });
        let resp_str = server.dispatch_request(&req.to_string());
        let resp: JsonRpcResponse =
            serde_json::from_str(&resp_str).expect("Valid JSON-RPC response");
        assert_eq!(resp.jsonrpc, "2.0");
        // Ensure method is registered and dispatched (not method not found -32601)
        if let Some(err) = &resp.error {
            assert_ne!(
                err.code, -32601,
                "Method '{}' must be registered in dispatcher",
                method
            );
        }
    }
}
