// Sprint 323/324: Local Swarm Role Management & Leadership Claim Primitives (Phase 1) Tests
//
// Verifies Local Swarm Role Management & Leadership Claim Primitives (Phase 1), cluster node roles,
// quorum consensus voting, auth protection, and agentic handshake capabilities.
//
// Hinweis: knc_swarm_elect verwaltet aktuell den lokalen Knotenzustand und Leadership-Claim (Phase 1).
// Ein vollwertiger Cross-Node Consensus Broadcast via Mesh ist für ein folgendes Release geplant.

use serde_json::Value;

use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{JsonRpcResponse, MeshPeer, NodeRole, RpcServer};

fn make_server(node_id: &str) -> RpcServer {
    RpcServer::with_mesh(AgentPermissions::default(), node_id, "127.0.0.1:0", None)
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

// ─────────────────────────────────────────────────────────────────────────────
// 1. Raft Leader Election Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_swarm_elect_leader_and_term_increment() {
    let server = make_server("node-alpha");

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_swarm_elect",
        "params": { "candidate_node_id": "node-alpha", "term": 2, "force": true },
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let leader = result_field(&resp, "leader_node_id").as_str().unwrap();
    let term = result_field(&resp, "term").as_u64().unwrap();
    let role = result_field(&resp, "role").as_str().unwrap();

    assert_eq!(leader, "node-alpha");
    assert_eq!(term, 2);
    assert_eq!(role, "Leader");
}

#[test]
fn test_swarm_governance_direct_unit_test() {
    let server = make_server("node-beta");
    assert_eq!(server.swarm_governance.role(), NodeRole::Worker);

    let (leader, term, role) =
        server
            .swarm_governance
            .elect("node-beta", Some("node-beta"), Some(5), true);
    assert_eq!(leader, "node-beta");
    assert_eq!(term, 5);
    assert_eq!(role, NodeRole::Leader);
}

// ─────────────────────────────────────────────────────────────────────────────
// 2. Node Roles Listing Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_swarm_roles_listing() {
    let server = make_server("local-leader");
    server.swarm_governance.set_role(NodeRole::Leader);

    // Register a peer node
    {
        let mut peers = server.peers.lock().unwrap();
        peers.insert(
            "peer-storage".to_string(),
            MeshPeer {
                node_id: "peer-storage".to_string(),
                address: "127.0.0.1:9001".to_string(),
                capabilities: vec!["storage".to_string()],
                last_seen: 100,
                latency_ms: 5,
                status: "Active".to_string(),
            },
        );
    }

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_swarm_roles",
        "params": {},
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let local_role = result_field(&resp, "local_role").as_str().unwrap();
    assert_eq!(local_role, "Leader");

    let roles = result_field(&resp, "roles");
    assert_eq!(roles["local-leader"].as_str().unwrap(), "Leader");
    assert_eq!(roles["peer-storage"].as_str().unwrap(), "Storage");
}

// ─────────────────────────────────────────────────────────────────────────────
// 3. Quorum Consensus Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_swarm_quorum_evaluation() {
    let server = make_server("quorum-node");

    // Add 2 active peers -> 3 total active nodes -> quorum threshold = (3/2) + 1 = 2
    {
        let mut peers = server.peers.lock().unwrap();
        peers.insert(
            "peer-1".to_string(),
            MeshPeer {
                node_id: "peer-1".to_string(),
                address: "127.0.0.1:9001".to_string(),
                capabilities: vec![],
                last_seen: 100,
                latency_ms: 5,
                status: "Active".to_string(),
            },
        );
        peers.insert(
            "peer-2".to_string(),
            MeshPeer {
                node_id: "peer-2".to_string(),
                address: "127.0.0.1:9002".to_string(),
                capabilities: vec![],
                last_seen: 100,
                latency_ms: 10,
                status: "Active".to_string(),
            },
        );
    }

    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_swarm_quorum",
        "params": { "operation": "deploy_isolate" },
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    assert_eq!(
        result_field(&resp, "operation").as_str().unwrap(),
        "deploy_isolate"
    );
    assert!(result_field(&resp, "quorum_reached").as_bool().unwrap());
    assert_eq!(result_field(&resp, "active_nodes").as_u64().unwrap(), 3);
    assert_eq!(result_field(&resp, "quorum_threshold").as_u64().unwrap(), 2);
}

// ─────────────────────────────────────────────────────────────────────────────
// 4. Mesh Auth Enforcement Tests
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_swarm_auth_protection() {
    let server = RpcServer::with_mesh(
        AgentPermissions::default(),
        "authed-swarm-node",
        "127.0.0.1:0",
        Some("swarm-auth-token".to_string()),
    );

    // Unauthenticated elect
    let req_elect = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_swarm_elect",
        "params": { "candidate_node_id": "attacker" },
        "id": 1
    })
    .to_string();

    let resp = parse_response(&server.dispatch_request(&req_elect));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);

    // Unauthenticated quorum
    let req_quorum = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_swarm_quorum",
        "params": { "operation": "destructive_op" },
        "id": 2
    })
    .to_string();

    let resp = parse_response(&server.dispatch_request(&req_quorum));
    assert!(resp.error.is_some());
    assert_eq!(resp.error.unwrap().code, -32001);
}

// ─────────────────────────────────────────────────────────────────────────────
// 5. Handshake Capabilities Test
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn test_handshake_advertises_swarm_governance() {
    let server = make_server("capability-node");
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "knc_agent_handshake",
        "params": {},
        "id": 1
    })
    .to_string();

    let raw = server.dispatch_request(&req);
    let resp = parse_response(&raw);
    assert!(resp.error.is_none());

    let caps = result_field(&resp, "capabilities");
    assert!(caps["swarm_governance"].as_bool().unwrap());
    assert!(caps["swarm_leadership"].as_bool().unwrap());
    assert!(caps["node_roles"].as_bool().unwrap());
    assert_eq!(
        result_field(&resp, "protocol_version").as_str().unwrap(),
        "v2.18.2"
    );
}
