use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};
use tracing::{error, info, warn};

/// Documentation for a native function.
#[derive(Debug, Serialize, Deserialize, Clone)]
struct NativeFuncDoc {
    name: String,
    module: String,
    description: String,
    parameters: Vec<ParamDoc>,
    returns: String,
    permissions: Vec<String>,
    errors: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ParamDoc {
    name: String,
    #[serde(rename = "type")]
    param_type: String,
    description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NativeRegistryJson {
    functions: Vec<NativeFuncDoc>,
}

/// All OpCode names that KnotenCore's Stack-VM understands.
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

struct KnotenBackend {
    client: Client,
    known_opcodes: HashSet<&'static str>,
    registry: Arc<HashMap<String, NativeFuncDoc>>,
    documents: dashmap::DashMap<Url, String>,
    symbols: dashmap::DashMap<Url, HashMap<String, Position>>,
}

impl KnotenBackend {
    fn with_registry(client: Client, registry: HashMap<String, NativeFuncDoc>) -> Self {
        Self {
            client,
            known_opcodes: KNOWN_OPCODES.iter().copied().collect(),
            registry: Arc::new(registry),
            documents: dashmap::DashMap::new(),
            symbols: dashmap::DashMap::new(),
        }
    }

    async fn validate_nod_document(&self, uri: Url, text: &str) {
        let mut diagnostics: Vec<Diagnostic> = Vec::new();

        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(json) => {
                self.collect_diagnostics(&json, &mut diagnostics, 0);
            }
            Err(e) => {
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

        self.index_symbols(uri.clone(), text);

        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }

    fn index_symbols(&self, uri: Url, text: &str) {
        let mut map = HashMap::new();
        // Regex to find "FnDef": ["FunctionName",
        let re = regex::Regex::new(r#""FnDef":\s*\[\s*"([^"]+)""#).unwrap();

        for cap in re.captures_iter(text) {
            if let Some(name_match) = cap.get(1) {
                let name = name_match.as_str().to_string();
                let offset = name_match.start();

                // Convert byte offset to Position
                let mut line = 0;
                let mut col = 0;
                let mut current_offset = 0;

                for c in text.chars() {
                    if current_offset >= offset {
                        break;
                    }
                    if c == '\n' {
                        line += 1;
                        col = 0;
                    } else {
                        col += 1;
                    }
                    current_offset += c.len_utf8();
                }

                map.insert(name, Position::new(line, col));
            }
        }
        self.symbols.insert(uri, map);
    }

    fn collect_diagnostics(
        &self,
        value: &serde_json::Value,
        diagnostics: &mut Vec<Diagnostic>,
        depth: usize,
    ) {
        if let Some(obj) = value.as_object() {
            for (key, child) in obj {
                let looks_like_node = key
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_uppercase())
                    .unwrap_or(false);

                if looks_like_node {
                    if !self.known_opcodes.contains(key.as_str()) {
                        if depth <= 2 {
                            diagnostics.push(Diagnostic {
                                range: Range::new(Position::new(0, 0), Position::new(0, 1)),
                                severity: Some(DiagnosticSeverity::WARNING),
                                code: Some(NumberOrString::String("ERR_UNKNOWN_NODE".to_string())),
                                source: Some("knoten-lsp".to_string()),
                                message: format!(
                                    "Unknown KnotenCore node: \"{key}\". Hallucinated nodes are rejected at runtime."
                                ),
                                ..Default::default()
                            });
                        }
                    } else {
                        // Known node -> check structure
                        self.validate_structure(key, child, diagnostics);
                    }
                }
                self.collect_diagnostics(child, diagnostics, depth + 1);
            }
        } else if let Some(arr) = value.as_array() {
            for item in arr {
                self.collect_diagnostics(item, diagnostics, depth + 1);
            }
        }
    }

    fn validate_structure(
        &self,
        key: &str,
        value: &serde_json::Value,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match key {
            "IntLiteral" => {
                if !value.is_i64() && !value.is_u64() {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        &format!("IntLiteral must be an integer, found {value}"),
                    );
                }
            }
            "FloatLiteral" => {
                if !value.is_f64() && !value.is_number() {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        &format!("FloatLiteral must be a number, found {value}"),
                    );
                }
            }
            "BoolLiteral" => {
                if !value.is_boolean() {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        &format!("BoolLiteral must be a boolean, found {value}"),
                    );
                }
            }
            "StringLiteral" | "Identifier" | "Import" => {
                if !value.is_string() {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        &format!("{key} must be a string, found {value}"),
                    );
                }
            }
            "If" => {
                if let Some(arr) = value.as_array() {
                    if arr.len() < 2 || arr.len() > 3 {
                        self.push_error(
                            diagnostics,
                            "ERR_INVALID_ARITY",
                            "If node requires [condition, then] or [condition, then, else]",
                        );
                    }
                } else {
                    self.push_error(diagnostics, "ERR_TYPE_MISMATCH", "If node must be an array");
                }
            }
            "While" | "Add" | "Sub" | "Mul" | "Div" | "Lt" | "Gt" | "Eq" | "BitAnd"
            | "BitShiftLeft" | "BitShiftRight" | "Concat" | "ArrayGet" | "ArrayPush" | "MapGet"
            | "Call" | "NativeCall" => {
                if let Some(arr) = value.as_array() {
                    if arr.len() != 2 {
                        self.push_error(
                            diagnostics,
                            "ERR_INVALID_ARITY",
                            &format!("{key} node requires exactly 2 arguments"),
                        );
                    }
                } else {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        &format!("{key} node must be an array"),
                    );
                }
            }
            "Assign" => {
                if let Some(arr) = value.as_array() {
                    if arr.len() != 2 {
                        self.push_error(
                            diagnostics,
                            "ERR_INVALID_ARITY",
                            "Assign node requires exactly 2 arguments [name:string, value:node]",
                        );
                    } else if !arr[0].is_string() {
                        self.push_error(
                            diagnostics,
                            "ERR_TYPE_MISMATCH",
                            "First argument of Assign must be a string (variable name)",
                        );
                    }
                } else {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        "Assign node must be an array",
                    );
                }
            }
            "ArraySet" | "PropertySet" | "MapSet" | "UIWindow" | "UIGrid" | "InitWindow" => {
                if let Some(arr) = value.as_array() {
                    if arr.len() != 3 {
                        self.push_error(
                            diagnostics,
                            "ERR_INVALID_ARITY",
                            &format!("{key} node requires exactly 3 arguments"),
                        );
                    }
                } else {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        &format!("{key} node must be an array"),
                    );
                }
            }
            "Block" | "ArrayCreate" | "UIHBox" | "UIVBox" => {
                if !value.is_array() {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        &format!("{key} node must be an array of children"),
                    );
                }
            }
            "FnDef" => {
                if let Some(arr) = value.as_array() {
                    if arr.len() != 3 {
                        self.push_error(
                            diagnostics,
                            "ERR_INVALID_ARITY",
                            "FnDef node requires [name:string, params:[string...], body:node]",
                        );
                    } else {
                        if !arr[0].is_string() {
                            self.push_error(
                                diagnostics,
                                "ERR_TYPE_MISMATCH",
                                "FnDef name must be a string",
                            );
                        }
                        if !arr[1].is_array() {
                            self.push_error(
                                diagnostics,
                                "ERR_TYPE_MISMATCH",
                                "FnDef params must be an array of strings",
                            );
                        }
                    }
                } else {
                    self.push_error(
                        diagnostics,
                        "ERR_TYPE_MISMATCH",
                        "FnDef node must be an array",
                    );
                }
            }
            _ => {}
        }
    }

    fn push_error(&self, diagnostics: &mut Vec<Diagnostic>, code: &str, message: &str) {
        diagnostics.push(Diagnostic {
            range: Range::new(Position::new(0, 0), Position::new(0, 1)),
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String(code.to_string())),
            source: Some("knoten-lsp".to_string()),
            message: message.to_string(),
            ..Default::default()
        });
    }

    fn get_word_at(&self, text: &str, position: Position) -> Option<String> {
        let lines: Vec<&str> = text.lines().collect();
        let line = lines.get(position.line as usize)?;
        let chars: Vec<char> = line.chars().collect();
        let col = position.character as usize;

        if col >= chars.len() {
            return None;
        }

        let mut start = col;
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        let mut end = col;
        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        if start == end {
            None
        } else {
            Some(chars[start..end].iter().collect())
        }
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for KnotenBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![".".to_string(), "\"".to_string()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "knoten-lsp".to_string(),
                version: Some(env!("CARGO_PKG_VERSION").to_string()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        info!("knoten-lsp active — Hover & Completion ready.");
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let text = params.text_document.text;
        self.documents.insert(uri.clone(), text.clone());
        self.index_symbols(uri.clone(), &text);
        self.validate_nod_document(uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        if let Some(change) = params.content_changes.into_iter().last() {
            let text = change.text;
            self.documents.insert(uri.clone(), text.clone());
            self.index_symbols(uri.clone(), &text);
            self.validate_nod_document(uri, &text).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.documents.remove(&uri);
        self.symbols.remove(&uri);
        self.client.publish_diagnostics(uri, vec![], None).await;
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        if let Some(func) = self.documents.get(&uri).and_then(|doc_text| {
            self.get_word_at(&doc_text, position)
                .and_then(|w| self.registry.get(&w).cloned())
        }) {
            let mut markdown = format!(
                "### `{}`\n\n**Module**: `{}`\n\n{}\n\n",
                func.name, func.module, func.description
            );

            if !func.parameters.is_empty() {
                markdown.push_str("**Parameters**:\n");
                for p in &func.parameters {
                    markdown.push_str(&format!(
                        "- `{}: {}` - {}\n",
                        p.name,
                        p.param_type,
                        p.description.as_deref().unwrap_or("")
                    ));
                }
                markdown.push('\n');
            }

            markdown.push_str(&format!("**Returns**: `{}`\n\n", func.returns));

            if !func.permissions.is_empty() {
                markdown.push_str(&format!(
                    "**Permissions**: `{}`\n\n",
                    func.permissions.join(", ")
                ));
            }

            return Ok(Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: markdown,
                }),
                range: None,
            }));
        }

        Ok(None)
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let word_opt = self
            .documents
            .get(&uri)
            .and_then(|doc_text| self.get_word_at(&doc_text, position));
        let def_pos_opt = word_opt.as_ref().and_then(|word| {
            self.symbols
                .get(&uri)
                .and_then(|doc_symbols| doc_symbols.get(word).cloned())
        });

        if let (Some(word), Some(def_pos)) = (word_opt, def_pos_opt) {
            return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                uri.clone(),
                Range::new(
                    def_pos,
                    Position::new(def_pos.line, def_pos.character + word.len() as u32),
                ),
            ))));
        }

        Ok(None)
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let new_name = params.new_name;

        if let (Some(old_name), Some(doc_text)) = (
            self.documents
                .get(&uri)
                .and_then(|t| self.get_word_at(&t, position)),
            self.documents.get(&uri),
        ) {
            // Find all occurrences of "old_name" in the document
            let mut edits = Vec::new();
            let pattern = format!("\"{}\"", old_name);

            let lines: Vec<&str> = doc_text.lines().collect();
            for (i, line) in lines.iter().enumerate() {
                let mut start = 0;
                while let Some(pos) = line[start..].find(&pattern) {
                    let absolute_start = start + pos + 1; // +1 to skip the opening quote
                    let absolute_end = absolute_start + old_name.len();

                    edits.push(TextEdit::new(
                        Range::new(
                            Position::new(i as u32, absolute_start as u32),
                            Position::new(i as u32, absolute_end as u32),
                        ),
                        new_name.clone(),
                    ));
                    start = absolute_end;
                }
            }

            let mut changes = HashMap::new();
            changes.insert(uri, edits);
            return Ok(Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }));
        }

        Ok(None)
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let position = params.position;

        if let Some(word) = self
            .documents
            .get(&uri)
            .and_then(|doc_text| self.get_word_at(&doc_text, position))
        {
            // Check if the word is in our symbol table (it's a function we defined)
            let is_symbol = self
                .symbols
                .get(&uri)
                .map(|s| s.contains_key(&word))
                .unwrap_or(false);
            let is_call_target = word
                .chars()
                .next()
                .map(|c| c.is_ascii_lowercase())
                .unwrap_or(false);

            if is_symbol || is_call_target {
                return Ok(Some(PrepareRenameResponse::Range(Range::new(
                    position,
                    Position::new(position.line, position.character + word.len() as u32),
                ))));
            }
        }

        Ok(None)
    }

    async fn completion(&self, _: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items = Vec::new();

        for (name, doc) in self.registry.iter() {
            items.push(CompletionItem {
                label: name.clone(),
                kind: Some(CompletionItemKind::FUNCTION),
                detail: Some(format!("Module: {}", doc.module)),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: doc.description.clone(),
                })),
                ..Default::default()
            });
        }

        for opcode in KNOWN_OPCODES {
            items.push(CompletionItem {
                label: opcode.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("KnotenCore OpCode".to_string()),
                ..Default::default()
            });
        }

        Ok(Some(CompletionResponse::Array(items)))
    }
}

fn load_registry(docs_path: Option<PathBuf>) -> HashMap<String, NativeFuncDoc> {
    let mut registry = HashMap::new();

    // Heuristic for documentation path
    let path = if let Some(p) = docs_path {
        p.join("docs/LANGUAGE_REFERENCE/native_functions.json")
    } else {
        PathBuf::from("docs/LANGUAGE_REFERENCE/native_functions.json")
    };

    info!("Loading Native Registry from: {:?}", path);

    if let Ok(content) = std::fs::read_to_string(&path) {
        if let Ok(json) = serde_json::from_str::<NativeRegistryJson>(&content) {
            for func in json.functions {
                registry.insert(func.name.clone(), func);
            }
            info!("Successfully loaded {} native functions.", registry.len());
        } else {
            error!("Failed to parse native_functions.json");
        }
    } else {
        warn!("Could not find native_functions.json at {:?}", path);
    }

    registry
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("knoten_lsp=info".parse().expect("valid directive")),
        )
        .without_time()
        .init();

    let args: Vec<String> = std::env::args().collect();
    let docs_path = if let Some(idx) = args.iter().position(|a| a == "--docs") {
        args.get(idx + 1).map(PathBuf::from)
    } else {
        None
    };

    let registry = load_registry(docs_path);

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) =
        LspService::new(|client| KnotenBackend::with_registry(client, registry));
    Server::new(stdin, stdout, socket).serve(service).await;
}
