use std::collections::HashSet;
use std::sync::Arc;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::info;

/// All OpCode names that KnotenCore's Stack-VM understands.
/// Kept in sync with `src/vm/opcode.rs`.
const KNOWN_OPCODES: &[&str] = &[
    "Constant",
    "Add",
    "Subtract",
    "Multiply",
    "Divide",
    "Equal",
    "NotEqual",
    "Greater",
    "Less",
    "LessEqual",
    "GreaterEqual",
    "And",
    "Or",
    "Not",
    "Jump",
    "JumpIfFalse",
    "StringLength",
    "StringContainsChars",
    "StringSplit",
    "ArrayContains",
    "ReadFile",
    "ExternCall",
    "SetGlobal",
    "GetGlobal",
    "SetLocal",
    "GetLocal",
    "Call",
    "AllocateDict",
    "GetProperty",
    "SetProperty",
    "Pop",
    "Print",
    "Return",
    "ArrayCreate",
    "ArrayGet",
    "ArraySet",
    "ArrayPush",
    "ArrayLen",
    "Concat",
    "ToString",
    "WriteFile",
    "NativeExternCall",
    "UIWindow",
    "UILabel",
    "UIButton",
    "UIHBox",
    "UIVBox",
];

/// The KnotenCore Language Server backend.
/// Provides real-time validation of `.nod` JSON-AST documents for AI agents.
struct KnotenBackend {
    client: Client,
    known_opcodes: Arc<HashSet<&'static str>>,
}

impl KnotenBackend {
    fn new(client: Client) -> Self {
        let known_opcodes = Arc::new(KNOWN_OPCODES.iter().copied().collect::<HashSet<_>>());
        Self {
            client,
            known_opcodes,
        }
    }

    /// Scan a JSON document for unknown top-level node keys and publish
    /// LSP diagnostics back to the editor.
    async fn validate_nod_document(&self, uri: Url, text: &str) {
        let mut diagnostics: Vec<Diagnostic> = Vec::new();

        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(json) => {
                self.collect_diagnostics(&json, &mut diagnostics, 0);
            }
            Err(e) => {
                // Surface JSON parse errors as a diagnostic at line 0
                diagnostics.push(Diagnostic {
                    range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("ERR_JSON_PARSE".to_string())),
                    source: Some("knoten-lsp".to_string()),
                    message: format!("JSON parse error: {e}"),
                    ..Default::default()
                });
            }
        }

        info!(
            "Validated {} — {} diagnostics emitted",
            uri.path(),
            diagnostics.len()
        );
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    /// Recursively walk the JSON value tree. Any object key that looks like
    /// a KnotenCore node name but isn't in `KNOWN_OPCODES` gets flagged.
    fn collect_diagnostics(
        &self,
        value: &serde_json::Value,
        diagnostics: &mut Vec<Diagnostic>,
        depth: usize,
    ) {
        if let Some(obj) = value.as_object() {
            for (key, child) in obj {
                // Heuristic: capitalised keys at depth 0-2 are likely node names
                let looks_like_node = key
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false);

                if looks_like_node && depth <= 2 && !self.known_opcodes.contains(key.as_str()) {
                    diagnostics.push(Diagnostic {
                        range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: Some(NumberOrString::String("ERR_UNKNOWN_NODE".to_string())),
                        source: Some("knoten-lsp".to_string()),
                        message: format!(
                            "Unknown KnotenCore node: \"{key}\". \
                             Hallucinated nodes are rejected at runtime. \
                             Check docs/LANGUAGE_REFERENCE/node_types.json."
                        ),
                        ..Default::default()
                    });
                }
                self.collect_diagnostics(child, diagnostics, depth + 1);
            }
        } else if let Some(arr) = value.as_array() {
            for item in arr {
                self.collect_diagnostics(item, diagnostics, depth + 1);
            }
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for KnotenBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        info!("knoten_lsp: initialize");
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "knoten-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("knoten_lsp: client acknowledged initialization");
        self.client
            .log_message(
                MessageType::INFO,
                "knoten-lsp active — AI-Native Runtime validator ready.",
            )
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        info!("knoten_lsp: shutdown requested");
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        info!("knoten_lsp: did_open {}", uri.path());
        self.validate_nod_document(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            info!("knoten_lsp: did_change {}", uri.path());
            self.validate_nod_document(uri, &change.text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        info!(
            "knoten_lsp: did_close — clearing diagnostics for {}",
            uri.path()
        );
        // Clear diagnostics on close
        self.client.publish_diagnostics(uri, vec![], None).await;
    }
}

#[tokio::main]
async fn main() {
    // Initialise structured logging to stderr so the VS Code output channel captures it.
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("knoten_lsp=info".parse().expect("valid directive")),
        )
        .without_time()
        .init();

    info!("knoten_lsp starting — KnotenCore AI-Native Execution Runtime LSP");

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(KnotenBackend::new);
    Server::new(stdin, stdout, socket).serve(service).await;
}
