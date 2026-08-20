use aether_compiler::executor::AgentPermissions;
use aether_compiler::rpc::{KNC_PROTOCOL_VERSION, RpcServer};
use std::fs;
use std::path::Path;

#[test]
fn test_version_assertion_sprint341() {
    assert_eq!(KNC_PROTOCOL_VERSION, "v2.24.12");
    let server = RpcServer::new(AgentPermissions::default());
    let req = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "knc_mesh_metrics",
        "params": {}
    });
    let resp = server.dispatch_request(&req.to_string());
    assert!(resp.contains("\"protocol_version\":\"v2.24.12\""));
}

#[test]
fn test_ai_manifest_and_template_presence() {
    let ai_manifest_path = Path::new("AI.md");
    assert!(
        ai_manifest_path.exists(),
        "AI.md manifest must exist in repository root"
    );

    let ai_content = fs::read_to_string(ai_manifest_path).expect("Failed to read AI.md manifest");
    assert!(ai_content.contains("KnotenCore AI Agent Directives Manifest"));
    assert!(ai_content.contains("#KnotenCore"));
    assert!(ai_content.contains("#OpenClaw"));
    assert!(ai_content.contains("#Moltbook"));
    assert!(ai_content.contains("docs/KNOTEN_SPEC.md"));
    assert!(ai_content.contains("Human Review Invariant"));

    let template_path = Path::new(".github/ISSUE_TEMPLATE/bot_report.md");
    assert!(
        template_path.exists(),
        "Bot issue template must exist in .github/ISSUE_TEMPLATE/"
    );

    let template_content =
        fs::read_to_string(template_path).expect("Failed to read bot_report.md template");
    assert!(template_content.contains("Autonomous Bot Report"));
    assert!(template_content.contains("bot_identity"));
    assert!(template_content.contains("v2.24.2"));

    let workflow_path = Path::new("docs/workflows/agent-ci-feedback.yml");
    assert!(
        workflow_path.exists(),
        "Agent CI feedback workflow must exist in docs/workflows/"
    );
    let workflow_content =
        fs::read_to_string(workflow_path).expect("Failed to read agent-ci-feedback.yml");
    assert!(workflow_content.contains("pull-requests: write"));
    assert!(workflow_content.contains("v2.24.2"));
}
