use std::collections::HashMap;
use std::sync::Arc;

use busbar_sf_agentscript::ast::*;
use busbar_sf_agentscript::error::ParseErrorInfo;
use busbar_sf_agentscript::graph::dependencies::{extract_dependencies, DependencyReport};
use busbar_sf_agentscript::graph::{GraphRepr, RefGraphBuilder};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

mod semantic_tokens;

use semantic_tokens::LEGEND;

// =============================================================================
// Document State
// =============================================================================

/// Cached parse state for a single document.
struct DocumentState {
    source: String,
    ast: Option<AgentFile>,
    parse_errors: Vec<ParseErrorInfo>,
    graph: Option<busbar_sf_agentscript::graph::RefGraph>,
}

impl DocumentState {
    fn new(source: String) -> Self {
        let (ast, parse_errors) =
            busbar_sf_agentscript::parser::parse_with_structured_errors_all(&source);

        let graph = ast
            .as_ref()
            .and_then(|a| RefGraphBuilder::new().build(a).ok());

        Self {
            source,
            ast,
            parse_errors,
            graph,
        }
    }
}

// =============================================================================
// Backend
// =============================================================================

struct Backend {
    client: Client,
    documents: Arc<RwLock<HashMap<Url, DocumentState>>>,
}

impl std::fmt::Debug for Backend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backend").finish()
    }
}

impl Backend {
    fn new(client: Client) -> Self {
        Self {
            client,
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    // -------------------------------------------------------------------------
    // Diagnostics
    // -------------------------------------------------------------------------

    async fn publish_diagnostics(&self, uri: &Url) {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(uri) else { return };

        let mut diagnostics = Vec::new();

        // Parse errors
        for err in &doc.parse_errors {
            diagnostics.push(parse_error_to_diagnostic(&doc.source, err));
        }

        // Semantic validation from the AST
        if let Some(ast) = &doc.ast {
            let semantic_errors = busbar_sf_agentscript::validate_ast(ast);
            for err in &semantic_errors {
                if let Some(span) = &err.span {
                    diagnostics.push(Diagnostic {
                        range: span_to_range(&doc.source, span.clone()),
                        severity: Some(match err.severity {
                            busbar_sf_agentscript::validation::Severity::Error => {
                                DiagnosticSeverity::ERROR
                            }
                            busbar_sf_agentscript::validation::Severity::Warning => {
                                DiagnosticSeverity::WARNING
                            }
                        }),
                        source: Some("agentscript".to_string()),
                        message: err.message.clone(),
                        ..Default::default()
                    });
                }
            }
        }

        // Graph validation (unresolved refs, cycles, unreachable topics, unused symbols)
        if doc.parse_errors.is_empty() {
            if let Some(graph) = &doc.graph {
                let validation = graph.validate();
                for error in &validation.errors {
                    if let Some(span) = error.span() {
                        diagnostics.push(Diagnostic {
                            range: span_to_range(&doc.source, span.0..span.1),
                            severity: Some(DiagnosticSeverity::ERROR),
                            source: Some("agentscript".to_string()),
                            message: error.message(),
                            ..Default::default()
                        });
                    }
                }
                for warning in &validation.warnings {
                    if let Some(span) = warning.span() {
                        diagnostics.push(Diagnostic {
                            range: span_to_range(&doc.source, span.0..span.1),
                            severity: Some(DiagnosticSeverity::WARNING),
                            source: Some("agentscript".to_string()),
                            message: warning.message(),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }
}

fn parse_error_to_diagnostic(text: &str, err: &ParseErrorInfo) -> Diagnostic {
    let range = if let Some(span) = &err.span {
        span_to_range(text, span.clone())
    } else {
        Range::default()
    };

    let mut message = err.message.clone();
    if let Some(found) = &err.found {
        message.push_str(&format!("\nFound: {}", found));
    }
    if !err.expected.is_empty() {
        message.push_str(&format!("\nExpected one of: {}", err.expected.join(", ")));
    }

    Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::ERROR),
        source: Some("agentscript".to_string()),
        message,
        ..Default::default()
    }
}

// =============================================================================
// Completions
// =============================================================================

fn get_completions(doc: &DocumentState, position: Position) -> Vec<CompletionItem> {
    let offset = position_to_offset(&doc.source, position);
    let before = &doc.source[..offset];
    let line_start = before.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let line = &before[line_start..];
    let indent = line.len() - line.trim_start().len();

    let mut items = Vec::new();

    // Reference completions: @namespace.partial
    if let Some(ref_ctx) = extract_reference_context(line) {
        match ref_ctx {
            RefContext::Namespace(partial) => {
                let namespaces = [
                    ("variables", "Variable references"),
                    ("actions", "Action references"),
                    ("outputs", "Action output references"),
                    ("topic", "Topic references"),
                    ("utils", "Utility functions"),
                    ("context", "Context references"),
                ];
                for (ns, detail) in namespaces {
                    if ns.starts_with(partial) {
                        items.push(CompletionItem {
                            label: format!("@{}", ns),
                            kind: Some(CompletionItemKind::MODULE),
                            detail: Some(detail.to_string()),
                            insert_text: Some(format!("{}.", ns)),
                            ..Default::default()
                        });
                    }
                }
            }
            RefContext::Member { namespace, partial } => {
                if let Some(ast) = &doc.ast {
                    match namespace {
                        "variables" => {
                            if let Some(vars) = &ast.variables {
                                for v in &vars.node.variables {
                                    let name = &v.node.name.node;
                                    if name.starts_with(partial) {
                                        items.push(CompletionItem {
                                            label: name.clone(),
                                            kind: Some(CompletionItemKind::VARIABLE),
                                            detail: Some(format!(
                                                "{:?} {:?}",
                                                v.node.kind, v.node.ty.node
                                            )),
                                            documentation: v
                                                .node
                                                .description
                                                .as_ref()
                                                .map(|d| Documentation::String(d.node.clone())),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                        }
                        "topic" => {
                            for t in &ast.topics {
                                let name = &t.node.name.node;
                                if name.starts_with(partial) {
                                    items.push(CompletionItem {
                                        label: name.clone(),
                                        kind: Some(CompletionItemKind::CLASS),
                                        detail: t.node.description.as_ref().map(|d| d.node.clone()),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        "actions" => {
                            let topic_actions = find_actions_at_offset(ast, offset);
                            for action in topic_actions {
                                let name = &action.node.name.node;
                                if name.starts_with(partial) {
                                    items.push(CompletionItem {
                                        label: name.clone(),
                                        kind: Some(CompletionItemKind::FUNCTION),
                                        detail: action
                                            .node
                                            .description
                                            .as_ref()
                                            .map(|d| d.node.clone()),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        "utils" => {
                            let utils = [
                                ("transition", "Navigate to a topic", "transition to @topic."),
                                ("escalate", "Escalate to a human agent", "escalate"),
                                ("setVariables", "Set multiple variables", "setVariables"),
                            ];
                            for (name, detail, insert) in utils {
                                if name.starts_with(partial) {
                                    items.push(CompletionItem {
                                        label: name.to_string(),
                                        kind: Some(CompletionItemKind::FUNCTION),
                                        detail: Some(detail.to_string()),
                                        insert_text: Some(insert.to_string()),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        "outputs" => {
                            let topic_actions = find_actions_at_offset(ast, offset);
                            for action in topic_actions {
                                let name = &action.node.name.node;
                                if name.starts_with(partial) {
                                    items.push(CompletionItem {
                                        label: name.clone(),
                                        kind: Some(CompletionItemKind::PROPERTY),
                                        detail: Some("Action outputs".to_string()),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        return items;
    }

    // Top-level block keywords (indent == 0)
    if indent == 0 && line.trim().is_empty() {
        let blocks: &[(&str, &str, &str)] = &[
            ("config:", "Agent configuration block", "config:\n   agent_name: \"$1\"\n   description: \"$2\""),
            ("variables:", "Variable declarations", "variables:\n   $1: mutable string = \"\""),
            ("system:", "System instructions and messages", "system:\n   instructions: \"$1\""),
            ("start_agent ", "Entry point for agent execution", "start_agent ${1:topic_selector}:\n   reasoning:\n      instructions: \"$1\""),
            ("topic ", "Define a conversation topic", "topic ${1:name}:\n   description: \"$2\"\n   reasoning:\n      instructions: \"$3\""),
            ("connections:", "Escalation routing", "connections:\n   $1:"),
            ("knowledge:", "Knowledge base configuration", "knowledge:\n   $1:"),
            ("language:", "Locale settings", "language:\n   $1:"),
        ];
        for &(label, detail, snippet) in blocks {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail.to_string()),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }
        return items;
    }

    // Sub-block keywords inside topic/start_agent (indent == 3)
    if indent == 3 && line.trim().is_empty() {
        // Config properties
        let in_config = before
            .rfind("config:")
            .map(|i| {
                !before[i..].contains("\ntopic ")
                    && !before[i..].contains("\nstart_agent ")
                    && !before[i..].contains("\nvariables:")
                    && !before[i..].contains("\nsystem:")
            })
            .unwrap_or(false);
        if in_config {
            let props: &[(&str, &str, &str)] = &[
                ("agent_name:", "Required agent identifier", "agent_name: \"$1\""),
                ("agent_label:", "Display label", "agent_label: \"$1\""),
                ("description:", "Agent description", "description: \"$1\""),
                ("agent_type:", "Agent type (e.g. ServiceAgent)", "agent_type: \"$1\""),
            ];
            for &(label, detail, snippet) in props {
                items.push(CompletionItem {
                    label: label.to_string(),
                    kind: Some(CompletionItemKind::PROPERTY),
                    detail: Some(detail.to_string()),
                    insert_text: Some(snippet.to_string()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    ..Default::default()
                });
            }
        }

        // Variable keyword
        let in_vars = before
            .rfind("variables:")
            .map(|i| {
                !before[i..].contains("\nconfig:")
                    && !before[i..].contains("\nsystem:")
                    && !before[i..].contains("\ntopic ")
                    && !before[i..].contains("\nstart_agent ")
            })
            .unwrap_or(false);
        if in_vars {
            items.push(CompletionItem {
                label: "mutable".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Mutable variable (read-write)".to_string()),
                ..Default::default()
            });
            items.push(CompletionItem {
                label: "linked".to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("Linked variable (read-only from context)".to_string()),
                ..Default::default()
            });
        }

        // Topic/start_agent sub-blocks
        let sub_blocks: &[(&str, &str, &str)] = &[
            ("description:", "Block description", "description: \"$1\""),
            ("reasoning:", "Reasoning block", "reasoning:\n      instructions: \"$1\""),
            ("actions:", "Action definitions", "actions:\n      $1:"),
            ("before_reasoning:", "Pre-reasoning directives", "before_reasoning:\n      $1"),
            ("after_reasoning:", "Post-reasoning directives", "after_reasoning:\n      $1"),
            ("system:", "System instruction override", "system:\n      instructions: \"$1\""),
        ];
        for &(label, detail, snippet) in sub_blocks {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail.to_string()),
                insert_text: Some(snippet.to_string()),
                insert_text_format: Some(InsertTextFormat::SNIPPET),
                ..Default::default()
            });
        }
    }

    // Type completions after "mutable" or "linked"
    let trimmed = line.trim();
    if trimmed.ends_with("mutable ") || trimmed.ends_with("linked ") {
        let types = [
            "string",
            "number",
            "boolean",
            "date",
            "object",
            "timestamp",
            "currency",
            "id",
            "datetime",
            "time",
            "integer",
            "long",
        ];
        for ty in types {
            items.push(CompletionItem {
                label: ty.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                ..Default::default()
            });
        }
    }

    // Reasoning action keywords at deep indent
    if indent >= 9 && line.trim().is_empty() {
        let kw: &[(&str, &str)] = &[
            ("description:", "Action description"),
            ("with ", "Input parameter binding"),
            ("set ", "Output variable binding"),
            ("available_when:", "Availability condition"),
        ];
        for &(label, detail) in kw {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }
    }

    items
}

// =============================================================================
// Hover
// =============================================================================

fn get_hover(doc: &DocumentState, position: Position) -> Option<Hover> {
    let ast = doc.ast.as_ref()?;
    let offset = position_to_offset(&doc.source, position);

    // Check variables block
    if let Some(vars) = &ast.variables {
        if vars.span.contains(&offset) {
            for var in &vars.node.variables {
                if var.span.contains(&offset) {
                    let desc = var
                        .node
                        .description
                        .as_ref()
                        .map(|d| format!("\n\n{}", d.node))
                        .unwrap_or_default();
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!(
                                "**Variable** `{}`\n\n**Kind:** {:?}  \n**Type:** `{:?}`{}",
                                var.node.name.node, var.node.kind, var.node.ty.node, desc
                            ),
                        }),
                        range: Some(span_to_range(&doc.source, var.node.name.span.clone())),
                    });
                }
            }
        }
    }

    // Check config
    if let Some(config) = &ast.config {
        if config.span.contains(&offset) {
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!(
                        "**Agent** `{}`{}",
                        config.node.agent_name.node,
                        config
                            .node
                            .description
                            .as_ref()
                            .map(|d| format!("\n\n{}", d.node))
                            .unwrap_or_default()
                    ),
                }),
                range: Some(span_to_range(&doc.source, config.span.clone())),
            });
        }
    }

    // Check start_agent
    if let Some(sa) = &ast.start_agent {
        if sa.span.contains(&offset) {
            if sa.node.name.span.contains(&offset) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!(
                            "**Start Agent** `{}`{}",
                            sa.node.name.node,
                            sa.node
                                .description
                                .as_ref()
                                .map(|d| format!("\n\n{}", d.node))
                                .unwrap_or_default()
                        ),
                    }),
                    range: Some(span_to_range(&doc.source, sa.node.name.span.clone())),
                });
            }
            if let Some(hover) = hover_actions_block(&doc.source, &sa.node.actions, offset) {
                return Some(hover);
            }
            if let Some(hover) = hover_reasoning_block(&doc.source, &sa.node.reasoning, offset) {
                return Some(hover);
            }
        }
    }

    // Check topics
    for topic in &ast.topics {
        if topic.span.contains(&offset) {
            if topic.node.name.span.contains(&offset) {
                let desc = topic
                    .node
                    .description
                    .as_ref()
                    .map(|d| format!("\n\n{}", d.node))
                    .unwrap_or_default();
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("**Topic** `{}`{}", topic.node.name.node, desc),
                    }),
                    range: Some(span_to_range(&doc.source, topic.node.name.span.clone())),
                });
            }
            if let Some(hover) = hover_actions_block(&doc.source, &topic.node.actions, offset) {
                return Some(hover);
            }
            if let Some(hover) = hover_reasoning_block(&doc.source, &topic.node.reasoning, offset) {
                return Some(hover);
            }
        }
    }

    // Check @references in source text
    hover_reference_at_offset(ast, &doc.source, offset)
}

// =============================================================================
// Go-to-Definition
// =============================================================================

fn get_definition(doc: &DocumentState, position: Position) -> Option<Range> {
    let ast = doc.ast.as_ref()?;
    let offset = position_to_offset(&doc.source, position);
    let reference = find_reference_at_offset(&doc.source, offset)?;

    match reference.namespace.as_str() {
        "variables" => {
            let name = reference.path.first()?;
            if let Some(vars) = &ast.variables {
                for v in &vars.node.variables {
                    if &v.node.name.node == name {
                        return Some(span_to_range(&doc.source, v.node.name.span.clone()));
                    }
                }
            }
        }
        "topic" => {
            let name = reference.path.first()?;
            for t in &ast.topics {
                if &t.node.name.node == name {
                    return Some(span_to_range(&doc.source, t.node.name.span.clone()));
                }
            }
        }
        "actions" => {
            let name = reference.path.first()?;
            let all_actions = collect_all_action_defs(ast);
            for action in all_actions {
                if &action.node.name.node == name {
                    return Some(span_to_range(&doc.source, action.node.name.span.clone()));
                }
            }
        }
        _ => {}
    }

    None
}

// =============================================================================
// Find References
// =============================================================================

fn get_references(doc: &DocumentState, position: Position) -> Vec<Range> {
    let ast = match &doc.ast {
        Some(a) => a,
        None => return Vec::new(),
    };
    let offset = position_to_offset(&doc.source, position);
    let symbol_name = match find_symbol_name_at_offset(ast, &doc.source, offset) {
        Some(s) => s,
        None => return Vec::new(),
    };

    let mut ranges = Vec::new();
    let patterns = [
        format!("@variables.{}", symbol_name),
        format!("@actions.{}", symbol_name),
        format!("@topic.{}", symbol_name),
        format!("@outputs.{}", symbol_name),
    ];

    for pattern in &patterns {
        let mut search_start = 0;
        while let Some(idx) = doc.source[search_start..].find(pattern.as_str()) {
            let abs_idx = search_start + idx;
            ranges.push(span_to_range(&doc.source, abs_idx..abs_idx + pattern.len()));
            search_start = abs_idx + pattern.len();
        }
    }

    ranges
}

// =============================================================================
// Rename
// =============================================================================

fn get_rename_edits(
    doc: &DocumentState,
    position: Position,
    new_name: &str,
) -> Option<Vec<TextEdit>> {
    let ast = doc.ast.as_ref()?;
    let offset = position_to_offset(&doc.source, position);
    let symbol_name = find_symbol_name_at_offset(ast, &doc.source, offset)?;

    let mut edits: Vec<TextEdit> = Vec::new();

    // Rename definitions
    if let Some(vars) = &ast.variables {
        for v in &vars.node.variables {
            if v.node.name.node == symbol_name {
                edits.push(TextEdit {
                    range: span_to_range(&doc.source, v.node.name.span.clone()),
                    new_text: new_name.to_string(),
                });
            }
        }
    }
    for t in &ast.topics {
        if t.node.name.node == symbol_name {
            edits.push(TextEdit {
                range: span_to_range(&doc.source, t.node.name.span.clone()),
                new_text: new_name.to_string(),
            });
        }
        if let Some(actions) = &t.node.actions {
            for a in &actions.node.actions {
                if a.node.name.node == symbol_name {
                    edits.push(TextEdit {
                        range: span_to_range(&doc.source, a.node.name.span.clone()),
                        new_text: new_name.to_string(),
                    });
                }
            }
        }
    }
    if let Some(sa) = &ast.start_agent {
        if let Some(actions) = &sa.node.actions {
            for a in &actions.node.actions {
                if a.node.name.node == symbol_name {
                    edits.push(TextEdit {
                        range: span_to_range(&doc.source, a.node.name.span.clone()),
                        new_text: new_name.to_string(),
                    });
                }
            }
        }
    }

    // Rename all @references
    let ref_patterns = [
        (format!("@variables.{}", symbol_name), format!("@variables.{}", new_name)),
        (format!("@actions.{}", symbol_name), format!("@actions.{}", new_name)),
        (format!("@topic.{}", symbol_name), format!("@topic.{}", new_name)),
        (format!("@outputs.{}", symbol_name), format!("@outputs.{}", new_name)),
    ];

    for (pattern, new_pattern) in &ref_patterns {
        let mut search_start = 0;
        while let Some(idx) = doc.source[search_start..].find(pattern.as_str()) {
            let abs_idx = search_start + idx;
            edits.push(TextEdit {
                range: span_to_range(&doc.source, abs_idx..abs_idx + pattern.len()),
                new_text: new_pattern.clone(),
            });
            search_start = abs_idx + pattern.len();
        }
    }

    if edits.is_empty() {
        None
    } else {
        Some(edits)
    }
}

// =============================================================================
// Document Symbols
// =============================================================================

fn get_document_symbols(doc: &DocumentState) -> Vec<DocumentSymbol> {
    let ast = match &doc.ast {
        Some(a) => a,
        None => return Vec::new(),
    };
    let text = &doc.source;
    let mut symbols = Vec::new();

    // Config
    if let Some(config) = &ast.config {
        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: "config".to_string(),
            detail: Some(config.node.agent_name.node.clone()),
            kind: SymbolKind::MODULE,
            tags: None,
            deprecated: None,
            range: span_to_range(text, config.span.clone()),
            selection_range: span_to_range(text, config.span.clone()),
            children: None,
        });
    }

    // Variables
    if let Some(vars) = &ast.variables {
        let children: Vec<DocumentSymbol> = vars
            .node
            .variables
            .iter()
            .map(|v| {
                #[allow(deprecated)]
                DocumentSymbol {
                    name: v.node.name.node.clone(),
                    detail: Some(format!("{:?} {:?}", v.node.kind, v.node.ty.node)),
                    kind: SymbolKind::VARIABLE,
                    tags: None,
                    deprecated: None,
                    range: span_to_range(text, v.span.clone()),
                    selection_range: span_to_range(text, v.node.name.span.clone()),
                    children: None,
                }
            })
            .collect();

        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: "variables".to_string(),
            detail: Some(format!("{} variables", children.len())),
            kind: SymbolKind::NAMESPACE,
            tags: None,
            deprecated: None,
            range: span_to_range(text, vars.span.clone()),
            selection_range: span_to_range(text, vars.span.clone()),
            children: Some(children),
        });
    }

    // System
    if let Some(system) = &ast.system {
        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: "system".to_string(),
            detail: None,
            kind: SymbolKind::MODULE,
            tags: None,
            deprecated: None,
            range: span_to_range(text, system.span.clone()),
            selection_range: span_to_range(text, system.span.clone()),
            children: None,
        });
    }

    // Connections
    for conn in &ast.connections {
        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: format!("connection {}", conn.node.name.node),
            detail: None,
            kind: SymbolKind::INTERFACE,
            tags: None,
            deprecated: None,
            range: span_to_range(text, conn.span.clone()),
            selection_range: span_to_range(text, conn.node.name.span.clone()),
            children: None,
        });
    }

    // Start Agent
    if let Some(sa) = &ast.start_agent {
        let mut sa_children = Vec::new();
        if let Some(actions) = &sa.node.actions {
            for a in &actions.node.actions {
                #[allow(deprecated)]
                sa_children.push(DocumentSymbol {
                    name: a.node.name.node.clone(),
                    detail: Some("Action".to_string()),
                    kind: SymbolKind::METHOD,
                    tags: None,
                    deprecated: None,
                    range: span_to_range(text, a.span.clone()),
                    selection_range: span_to_range(text, a.node.name.span.clone()),
                    children: None,
                });
            }
        }
        if let Some(reasoning) = &sa.node.reasoning {
            if let Some(actions) = &reasoning.node.actions {
                for a in &actions.node {
                    #[allow(deprecated)]
                    sa_children.push(DocumentSymbol {
                        name: a.node.name.node.clone(),
                        detail: Some("Reasoning Action".to_string()),
                        kind: SymbolKind::EVENT,
                        tags: None,
                        deprecated: None,
                        range: span_to_range(text, a.span.clone()),
                        selection_range: span_to_range(text, a.node.name.span.clone()),
                        children: None,
                    });
                }
            }
        }

        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: format!("start_agent {}", sa.node.name.node),
            detail: sa.node.description.as_ref().map(|d| d.node.clone()),
            kind: SymbolKind::CONSTRUCTOR,
            tags: None,
            deprecated: None,
            range: span_to_range(text, sa.span.clone()),
            selection_range: span_to_range(text, sa.node.name.span.clone()),
            children: if sa_children.is_empty() {
                None
            } else {
                Some(sa_children)
            },
        });
    }

    // Topics
    for topic in &ast.topics {
        let mut children = Vec::new();

        if let Some(actions) = &topic.node.actions {
            for a in &actions.node.actions {
                #[allow(deprecated)]
                children.push(DocumentSymbol {
                    name: a.node.name.node.clone(),
                    detail: a.node.target.as_ref().map(|t| t.node.clone()),
                    kind: SymbolKind::METHOD,
                    tags: None,
                    deprecated: None,
                    range: span_to_range(text, a.span.clone()),
                    selection_range: span_to_range(text, a.node.name.span.clone()),
                    children: None,
                });
            }
        }
        if let Some(reasoning) = &topic.node.reasoning {
            if let Some(actions) = &reasoning.node.actions {
                for a in &actions.node {
                    #[allow(deprecated)]
                    children.push(DocumentSymbol {
                        name: a.node.name.node.clone(),
                        detail: Some(format!("{:?}", a.node.target.node)),
                        kind: SymbolKind::EVENT,
                        tags: None,
                        deprecated: None,
                        range: span_to_range(text, a.span.clone()),
                        selection_range: span_to_range(text, a.node.name.span.clone()),
                        children: None,
                    });
                }
            }
        }

        #[allow(deprecated)]
        symbols.push(DocumentSymbol {
            name: format!("topic {}", topic.node.name.node),
            detail: topic.node.description.as_ref().map(|d| d.node.clone()),
            kind: SymbolKind::CLASS,
            tags: None,
            deprecated: None,
            range: span_to_range(text, topic.span.clone()),
            selection_range: span_to_range(text, topic.node.name.span.clone()),
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        });
    }

    symbols
}

// =============================================================================
// Formatting (uses the real serializer)
// =============================================================================

fn format_document(doc: &DocumentState) -> Option<Vec<TextEdit>> {
    let ast = doc.ast.as_ref()?;
    let formatted = busbar_sf_agentscript::serialize(ast);
    if formatted == doc.source {
        return None;
    }
    let end = offset_to_position(&doc.source, doc.source.len());
    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end,
        },
        new_text: formatted,
    }])
}

// =============================================================================
// Folding Ranges
// =============================================================================

fn get_folding_ranges(doc: &DocumentState) -> Vec<FoldingRange> {
    let ast = match &doc.ast {
        Some(a) => a,
        None => return Vec::new(),
    };
    let text = &doc.source;
    let mut ranges = Vec::new();

    let mut add_fold = |span: &std::ops::Range<usize>| {
        let start = offset_to_position(text, span.start);
        let end = offset_to_position(text, span.end);
        if start.line < end.line {
            ranges.push(FoldingRange {
                start_line: start.line,
                start_character: Some(start.character),
                end_line: end.line,
                end_character: Some(end.character),
                kind: Some(FoldingRangeKind::Region),
                collapsed_text: None,
            });
        }
    };

    if let Some(config) = &ast.config {
        add_fold(&config.span);
    }
    if let Some(vars) = &ast.variables {
        add_fold(&vars.span);
    }
    if let Some(system) = &ast.system {
        add_fold(&system.span);
    }
    if let Some(sa) = &ast.start_agent {
        add_fold(&sa.span);
        if let Some(actions) = &sa.node.actions {
            add_fold(&actions.span);
        }
        if let Some(reasoning) = &sa.node.reasoning {
            add_fold(&reasoning.span);
        }
    }
    for topic in &ast.topics {
        add_fold(&topic.span);
        if let Some(actions) = &topic.node.actions {
            add_fold(&actions.span);
        }
        if let Some(reasoning) = &topic.node.reasoning {
            add_fold(&reasoning.span);
        }
    }
    for conn in &ast.connections {
        add_fold(&conn.span);
    }

    // Comment folding: consecutive comment lines
    let mut comment_start: Option<u32> = None;
    for (i, line) in text.lines().enumerate() {
        if line.trim_start().starts_with('#') {
            if comment_start.is_none() {
                comment_start = Some(i as u32);
            }
        } else {
            if let Some(start) = comment_start {
                let end = i as u32 - 1;
                if end > start {
                    ranges.push(FoldingRange {
                        start_line: start,
                        start_character: None,
                        end_line: end,
                        end_character: None,
                        kind: Some(FoldingRangeKind::Comment),
                        collapsed_text: None,
                    });
                }
            }
            comment_start = None;
        }
    }

    ranges
}

// =============================================================================
// Code Actions
// =============================================================================

fn get_code_actions(doc: &DocumentState, range: Range) -> Vec<CodeActionOrCommand> {
    let ast = match &doc.ast {
        Some(a) => a,
        None => return Vec::new(),
    };
    let mut actions = Vec::new();
    let start_offset = position_to_offset(&doc.source, range.start);

    // Quick fix: add missing description to topics
    for topic in &ast.topics {
        if topic.span.contains(&start_offset) && topic.node.description.is_none() {
            actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Add description to topic '{}'", topic.node.name.node),
                kind: Some(CodeActionKind::QUICKFIX),
                is_preferred: Some(false),
                ..Default::default()
            }));
        }
    }

    // Quick fix: add missing description to variables
    if let Some(vars) = &ast.variables {
        for var in &vars.node.variables {
            if var.span.contains(&start_offset) && var.node.description.is_none() {
                actions.push(CodeActionOrCommand::CodeAction(CodeAction {
                    title: format!("Add description to variable '{}'", var.node.name.node),
                    kind: Some(CodeActionKind::QUICKFIX),
                    is_preferred: Some(false),
                    ..Default::default()
                }));
            }
        }
    }

    actions
}

// =============================================================================
// LSP Trait Implementation
// =============================================================================

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![
                        "@".to_string(),
                        ".".to_string(),
                        ":".to_string(),
                        " ".to_string(),
                    ]),
                    ..Default::default()
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: LEGEND.clone(),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: None,
                            ..Default::default()
                        },
                    ),
                ),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "AgentScript LSP initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let doc = DocumentState::new(params.text_document.text);
        self.documents.write().await.insert(uri.clone(), doc);
        self.publish_diagnostics(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        if let Some(change) = params.content_changes.into_iter().next() {
            let doc = DocumentState::new(change.text);
            self.documents.write().await.insert(uri.clone(), doc);
            self.publish_diagnostics(&uri).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.documents
            .write()
            .await
            .remove(&params.text_document.uri);
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };
        let items = get_completions(doc, params.text_document_position.position);
        if items.is_empty() {
            Ok(None)
        } else {
            Ok(Some(CompletionResponse::Array(items)))
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document_position_params.text_document.uri) else {
            return Ok(None);
        };
        Ok(get_hover(doc, params.text_document_position_params.position))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let docs = self.documents.read().await;
        let uri = params
            .text_document_position_params
            .text_document
            .uri
            .clone();
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        Ok(get_definition(doc, params.text_document_position_params.position)
            .map(|range| GotoDefinitionResponse::Scalar(Location { uri, range })))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let docs = self.documents.read().await;
        let uri = params.text_document_position.text_document.uri.clone();
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let ranges = get_references(doc, params.text_document_position.position);
        if ranges.is_empty() {
            Ok(None)
        } else {
            Ok(Some(
                ranges
                    .into_iter()
                    .map(|range| Location {
                        uri: uri.clone(),
                        range,
                    })
                    .collect(),
            ))
        }
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let docs = self.documents.read().await;
        let uri = params.text_document_position.text_document.uri.clone();
        let Some(doc) = docs.get(&uri) else {
            return Ok(None);
        };
        let edits = get_rename_edits(doc, params.text_document_position.position, &params.new_name);
        Ok(edits.map(|text_edits| {
            let mut changes = HashMap::new();
            changes.insert(uri, text_edits);
            WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }
        }))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let symbols = get_document_symbols(doc);
        if symbols.is_empty() {
            Ok(None)
        } else {
            Ok(Some(DocumentSymbolResponse::Nested(symbols)))
        }
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(format_document(doc))
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let ranges = get_folding_ranges(doc);
        if ranges.is_empty() {
            Ok(None)
        } else {
            Ok(Some(ranges))
        }
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let ast = doc.ast.as_ref();
        let tokens = semantic_tokens::compute_semantic_tokens(&doc.source, ast);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        })))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let docs = self.documents.read().await;
        let Some(doc) = docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let actions = get_code_actions(doc, params.range);
        if actions.is_empty() {
            Ok(None)
        } else {
            Ok(Some(actions))
        }
    }
}

// =============================================================================
// Custom Request Handlers (agentscript/*)
// =============================================================================

/// Parameters for agentscript/getGraph request.
#[derive(Debug, serde::Deserialize)]
struct GetGraphParams {
    uri: String,
}

/// Parameters for agentscript/getDependencies request.
#[derive(Debug, serde::Deserialize)]
struct GetDependenciesParams {
    uri: String,
}

/// Serializable dependency type for the extension.
#[derive(Debug, serde::Serialize)]
struct DependencyItemRepr {
    dep_type: DependencyTypeRepr,
    used_in: String,
    action_name: String,
    span: (usize, usize),
}

#[derive(Debug, serde::Serialize)]
struct DependencyTypeRepr {
    #[serde(rename = "type")]
    dep_category: String,
    name: String,
}

/// Serializable dependency report for the extension.
#[derive(Debug, serde::Serialize)]
struct DependencyReportRepr {
    flows: Vec<String>,
    apex_classes: Vec<String>,
    prompt_templates: Vec<String>,
    connections: Vec<String>,
    sobjects: Vec<String>,
    knowledge_bases: Vec<String>,
    external_services: Vec<String>,
    all_dependencies: Vec<DependencyItemRepr>,
}

impl From<&DependencyReport> for DependencyReportRepr {
    fn from(report: &DependencyReport) -> Self {
        Self {
            flows: report.flows.iter().cloned().collect(),
            apex_classes: report.apex_classes.iter().cloned().collect(),
            prompt_templates: report.prompt_templates.iter().cloned().collect(),
            connections: report.connections.iter().cloned().collect(),
            sobjects: report.sobjects.iter().cloned().collect(),
            knowledge_bases: report.knowledge_bases.iter().cloned().collect(),
            external_services: report.external_services.iter().cloned().collect(),
            all_dependencies: report
                .all_dependencies
                .iter()
                .map(|d| DependencyItemRepr {
                    dep_type: DependencyTypeRepr {
                        dep_category: d.dep_type.category().to_string(),
                        name: d.dep_type.name(),
                    },
                    used_in: d.used_in.clone(),
                    action_name: d.action_name.clone(),
                    span: d.span,
                })
                .collect(),
        }
    }
}

/// Parameters for agentscript/simulate request.
#[derive(Debug, serde::Deserialize)]
struct SimulateParams {
    uri: String,
    #[serde(default)]
    mock_data: serde_json::Value,
}

/// Simplified execution trace for the extension.
#[derive(Debug, serde::Serialize)]
struct SimulationResult {
    steps: Vec<SimulationStep>,
    final_context: serde_json::Value,
    outcome: String,
    topic_transitions: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
struct SimulationStep {
    phase: String,
    statement_type: String,
    detail: String,
    variable_changes: Vec<VarChange>,
    action_invocations: Vec<ActionInvoke>,
}

#[derive(Debug, serde::Serialize)]
struct VarChange {
    name: String,
    old_value: serde_json::Value,
    new_value: serde_json::Value,
}

#[derive(Debug, serde::Serialize)]
struct ActionInvoke {
    action_name: String,
    inputs: serde_json::Value,
    outputs: serde_json::Value,
}

impl Backend {
    /// Handle agentscript/getGraph — returns the GraphRepr JSON for the given document.
    async fn handle_get_graph(
        &self,
        params: serde_json::Value,
    ) -> tower_lsp::jsonrpc::Result<serde_json::Value> {
        let params: GetGraphParams = serde_json::from_value(params)
            .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))?;

        let uri: Url = params
            .uri
            .parse()
            .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(format!("{}", e)))?;

        let docs = self.documents.read().await;
        let doc = docs
            .get(&uri)
            .ok_or_else(|| tower_lsp::jsonrpc::Error::invalid_params("Document not found"))?;

        let graph = doc.graph.as_ref().ok_or_else(|| {
            tower_lsp::jsonrpc::Error::invalid_params("No graph available (parse errors?)")
        })?;

        let repr = GraphRepr::from(graph);
        serde_json::to_value(&repr).map_err(|e| tower_lsp::jsonrpc::Error {
            code: tower_lsp::jsonrpc::ErrorCode::InternalError,
            message: e.to_string().into(),
            data: None,
        })
    }

    /// Handle agentscript/getDependencies — returns external dependency analysis.
    async fn handle_get_dependencies(
        &self,
        params: serde_json::Value,
    ) -> tower_lsp::jsonrpc::Result<serde_json::Value> {
        let params: GetDependenciesParams = serde_json::from_value(params)
            .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))?;

        let uri: Url = params
            .uri
            .parse()
            .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(format!("{}", e)))?;

        let docs = self.documents.read().await;
        let doc = docs
            .get(&uri)
            .ok_or_else(|| tower_lsp::jsonrpc::Error::invalid_params("Document not found"))?;

        let ast = doc.ast.as_ref().ok_or_else(|| {
            tower_lsp::jsonrpc::Error::invalid_params("No AST available (parse errors?)")
        })?;

        let report = extract_dependencies(ast);
        let repr = DependencyReportRepr::from(&report);
        serde_json::to_value(&repr).map_err(|e| tower_lsp::jsonrpc::Error {
            code: tower_lsp::jsonrpc::ErrorCode::InternalError,
            message: e.to_string().into(),
            data: None,
        })
    }

    /// Handle agentscript/simulate — runs a dry simulation of the agent.
    ///
    /// This performs a static analysis simulation (walk through start_agent and
    /// first topic's before_reasoning/after_reasoning without LLM calls).
    async fn handle_simulate(
        &self,
        params: serde_json::Value,
    ) -> tower_lsp::jsonrpc::Result<serde_json::Value> {
        let params: SimulateParams = serde_json::from_value(params)
            .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(e.to_string()))?;

        let uri: Url = params
            .uri
            .parse()
            .map_err(|e| tower_lsp::jsonrpc::Error::invalid_params(format!("{}", e)))?;

        let docs = self.documents.read().await;
        let doc = docs
            .get(&uri)
            .ok_or_else(|| tower_lsp::jsonrpc::Error::invalid_params("Document not found"))?;

        let ast = doc.ast.as_ref().ok_or_else(|| {
            tower_lsp::jsonrpc::Error::invalid_params("No AST available (parse errors?)")
        })?;

        // Build a static simulation trace by walking the AST
        let result = build_static_simulation(ast, &params.mock_data);
        serde_json::to_value(&result).map_err(|e| tower_lsp::jsonrpc::Error {
            code: tower_lsp::jsonrpc::ErrorCode::InternalError,
            message: e.to_string().into(),
            data: None,
        })
    }
}

/// Walk the AST to produce a static execution trace (no runtime needed).
/// This shows the structural flow without actually executing expressions.
fn build_static_simulation(ast: &AgentFile, _mock_data: &serde_json::Value) -> SimulationResult {
    let mut steps = Vec::new();
    let mut topic_transitions = Vec::new();

    // Walk start_agent
    if let Some(start) = &ast.start_agent {
        let sa = &start.node;
        topic_transitions.push(format!("start_agent:{}", sa.name.node));

        // before_reasoning statements
        if let Some(before) = &sa.before_reasoning {
            for stmt in &before.node.statements {
                steps.push(statement_to_step("before_reasoning", &stmt.node));
            }
        }

        // reasoning actions
        if let Some(reasoning) = &sa.reasoning {
            let r = &reasoning.node;
            if let Some(actions) = &r.actions {
                for action in &actions.node {
                    let a = &action.node;
                    steps.push(SimulationStep {
                        phase: "reasoning".to_string(),
                        statement_type: "reasoning_action".to_string(),
                        detail: format!(
                            "{}: {}",
                            a.name.node,
                            a.description
                                .as_ref()
                                .map(|d| d.node.as_str())
                                .unwrap_or("")
                        ),
                        variable_changes: vec![],
                        action_invocations: vec![],
                    });
                }
            }
        }

        // after_reasoning statements
        if let Some(after) = &sa.after_reasoning {
            for stmt in &after.node.statements {
                steps.push(statement_to_step("after_reasoning", &stmt.node));
            }
        }
    }

    // Walk topics
    for topic in &ast.topics {
        let t = &topic.node;
        let name = t.name.node.clone();
        topic_transitions.push(name.clone());

        if let Some(before) = &t.before_reasoning {
            for stmt in &before.node.statements {
                steps.push(statement_to_step(&format!("{}:before_reasoning", name), &stmt.node));
            }
        }

        if let Some(reasoning) = &t.reasoning {
            let r = &reasoning.node;
            if let Some(actions) = &r.actions {
                for action in &actions.node {
                    let a = &action.node;
                    steps.push(SimulationStep {
                        phase: format!("{}:reasoning", name),
                        statement_type: "reasoning_action".to_string(),
                        detail: format!(
                            "{}: {}",
                            a.name.node,
                            a.description
                                .as_ref()
                                .map(|d| d.node.as_str())
                                .unwrap_or("")
                        ),
                        variable_changes: vec![],
                        action_invocations: vec![],
                    });
                }
            }
        }

        if let Some(after) = &t.after_reasoning {
            for stmt in &after.node.statements {
                steps.push(statement_to_step(&format!("{}:after_reasoning", name), &stmt.node));
            }
        }
    }

    SimulationResult {
        steps,
        final_context: serde_json::json!({}),
        outcome: "static_analysis".to_string(),
        topic_transitions,
    }
}

fn statement_to_step(phase: &str, stmt: &Stmt) -> SimulationStep {
    match stmt {
        Stmt::Set { target, value } => SimulationStep {
            phase: phase.to_string(),
            statement_type: "set".to_string(),
            detail: format!(
                "set {} = {}",
                reference_to_string(&target.node),
                expr_preview(&value.node)
            ),
            variable_changes: vec![VarChange {
                name: reference_to_string(&target.node),
                old_value: serde_json::Value::Null,
                new_value: serde_json::json!("(expression)"),
            }],
            action_invocations: vec![],
        },
        Stmt::Run {
            action,
            with_clauses,
            ..
        } => {
            let inputs: Vec<String> = with_clauses
                .iter()
                .map(|w| {
                    let val = match &w.node.value.node {
                        WithValue::Expr(e) => expr_preview(e),
                    };
                    format!("{}={}", w.node.param.node, val)
                })
                .collect();
            SimulationStep {
                phase: phase.to_string(),
                statement_type: "run".to_string(),
                detail: format!(
                    "run {} with [{}]",
                    reference_to_string(&action.node),
                    inputs.join(", ")
                ),
                variable_changes: vec![],
                action_invocations: vec![ActionInvoke {
                    action_name: reference_to_string(&action.node),
                    inputs: serde_json::json!(inputs),
                    outputs: serde_json::json!({}),
                }],
            }
        }
        Stmt::If {
            condition,
            then_block,
            ..
        } => SimulationStep {
            phase: phase.to_string(),
            statement_type: "if".to_string(),
            detail: format!(
                "if {} → {} statements",
                expr_preview(&condition.node),
                then_block.len()
            ),
            variable_changes: vec![],
            action_invocations: vec![],
        },
        Stmt::Transition { target } => SimulationStep {
            phase: phase.to_string(),
            statement_type: "transition".to_string(),
            detail: format!("transition to {}", reference_to_string(&target.node)),
            variable_changes: vec![],
            action_invocations: vec![],
        },
    }
}

fn reference_to_string(r: &Reference) -> String {
    format!("@{}.{}", r.namespace, r.path.join("."))
}

fn expr_preview(expr: &Expr) -> String {
    match expr {
        Expr::String(s) => format!("\"{}\"", s),
        Expr::Number(n) => n.to_string(),
        Expr::Bool(b) => b.to_string(),
        Expr::None => "None".to_string(),
        Expr::Reference(r) => reference_to_string(r),
        Expr::BinOp { op, left, right } => format!(
            "{} {} {}",
            expr_preview(&left.node),
            format!("{:?}", op).to_lowercase(),
            expr_preview(&right.node)
        ),
        _ => "(expr)".to_string(),
    }
}

// =============================================================================
// Utility Functions
// =============================================================================

fn span_to_range(text: &str, span: std::ops::Range<usize>) -> Range {
    Range {
        start: offset_to_position(text, span.start),
        end: offset_to_position(text, span.end),
    }
}

fn offset_to_position(text: &str, offset: usize) -> Position {
    let offset = offset.min(text.len());
    let mut line = 0u32;
    let mut last_line_start = 0;
    for (i, c) in text[..offset].char_indices() {
        if c == '\n' {
            line += 1;
            last_line_start = i + 1;
        }
    }
    Position {
        line,
        character: text[last_line_start..offset].chars().count() as u32,
    }
}

fn position_to_offset(text: &str, pos: Position) -> usize {
    let mut current_line = 0u32;
    let mut line_start = 0;
    for (i, c) in text.char_indices() {
        if current_line == pos.line {
            // Walk pos.character chars from line_start
            let mut chars = text[line_start..].char_indices();
            for _ in 0..pos.character {
                if chars.next().is_none() {
                    return text.len();
                }
            }
            return if let Some((ci, _)) = chars.next() {
                line_start + ci
            } else {
                text.len()
            };
        }
        if c == '\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }
    // If target line is the last line (no trailing newline)
    if current_line == pos.line {
        let mut chars = text[line_start..].char_indices();
        for _ in 0..pos.character {
            if chars.next().is_none() {
                return text.len();
            }
        }
        return if let Some((ci, _)) = chars.next() {
            line_start + ci
        } else {
            text.len()
        };
    }
    text.len()
}

/// Context for reference completions.
enum RefContext<'a> {
    /// `@` or `@part` — suggest namespaces
    Namespace(&'a str),
    /// `@namespace.part` — suggest members
    Member {
        namespace: &'a str,
        partial: &'a str,
    },
}

fn extract_reference_context(line: &str) -> Option<RefContext<'_>> {
    let at_idx = line.rfind('@')?;
    let after_at = &line[at_idx + 1..];

    if let Some(dot_idx) = after_at.find('.') {
        let namespace = &after_at[..dot_idx];
        let partial = &after_at[dot_idx + 1..];
        if namespace.chars().all(|c| c.is_alphanumeric() || c == '_')
            && partial.chars().all(|c| c.is_alphanumeric() || c == '_')
        {
            return Some(RefContext::Member { namespace, partial });
        }
    } else if after_at.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(RefContext::Namespace(after_at));
    }

    None
}

/// Find a @reference at the given byte offset.
fn find_reference_at_offset(source: &str, offset: usize) -> Option<Reference> {
    let before = &source[..offset.min(source.len())];
    let at_pos = before.rfind('@')?;
    let rest = &source[at_pos..];
    let end = rest
        .find(|c: char| !c.is_alphanumeric() && c != '_' && c != '.' && c != '@')
        .unwrap_or(rest.len());

    if offset > at_pos + end {
        return None;
    }

    let ref_text = &rest[1..end]; // strip @
    let mut parts: Vec<&str> = ref_text.split('.').collect();
    if parts.is_empty() {
        return None;
    }
    let namespace = parts.remove(0).to_string();
    let path = parts.iter().map(|s| s.to_string()).collect();

    Some(Reference { namespace, path })
}

/// Find the symbol name at cursor (definition or @reference).
fn find_symbol_name_at_offset(ast: &AgentFile, source: &str, offset: usize) -> Option<String> {
    // Variable definitions
    if let Some(vars) = &ast.variables {
        for v in &vars.node.variables {
            if v.node.name.span.contains(&offset) {
                return Some(v.node.name.node.clone());
            }
        }
    }

    // Topic names
    for t in &ast.topics {
        if t.node.name.span.contains(&offset) {
            return Some(t.node.name.node.clone());
        }
        if let Some(actions) = &t.node.actions {
            for a in &actions.node.actions {
                if a.node.name.span.contains(&offset) {
                    return Some(a.node.name.node.clone());
                }
            }
        }
    }

    // Start_agent action defs
    if let Some(sa) = &ast.start_agent {
        if let Some(actions) = &sa.node.actions {
            for a in &actions.node.actions {
                if a.node.name.span.contains(&offset) {
                    return Some(a.node.name.node.clone());
                }
            }
        }
    }

    // @references
    if let Some(reference) = find_reference_at_offset(source, offset) {
        return reference.path.into_iter().next();
    }

    None
}

/// Actions visible at a given offset (same topic/start_agent scope).
fn find_actions_at_offset(ast: &AgentFile, offset: usize) -> Vec<&Spanned<ActionDef>> {
    if let Some(sa) = &ast.start_agent {
        if sa.span.contains(&offset) {
            if let Some(actions) = &sa.node.actions {
                return actions.node.actions.iter().collect();
            }
        }
    }
    for topic in &ast.topics {
        if topic.span.contains(&offset) {
            if let Some(actions) = &topic.node.actions {
                return actions.node.actions.iter().collect();
            }
        }
    }
    Vec::new()
}

/// All action definitions in the file.
fn collect_all_action_defs(ast: &AgentFile) -> Vec<&Spanned<ActionDef>> {
    let mut all = Vec::new();
    if let Some(sa) = &ast.start_agent {
        if let Some(actions) = &sa.node.actions {
            all.extend(actions.node.actions.iter());
        }
    }
    for topic in &ast.topics {
        if let Some(actions) = &topic.node.actions {
            all.extend(actions.node.actions.iter());
        }
    }
    all
}

fn hover_actions_block(
    source: &str,
    actions: &Option<Spanned<ActionsBlock>>,
    offset: usize,
) -> Option<Hover> {
    let actions = actions.as_ref()?;
    if !actions.span.contains(&offset) {
        return None;
    }
    for action in &actions.node.actions {
        if action.span.contains(&offset) {
            let mut md = format!("**Action** `{}`", action.node.name.node);
            if let Some(desc) = &action.node.description {
                md.push_str(&format!("\n\n{}", desc.node));
            }
            if let Some(target) = &action.node.target {
                md.push_str(&format!("\n\n**Target:** `{}`", target.node));
            }
            if let Some(inputs) = &action.node.inputs {
                md.push_str("\n\n**Inputs:**");
                for input in &inputs.node {
                    md.push_str(&format!(
                        "\n- `{}`: `{:?}`",
                        input.node.name.node, input.node.ty.node
                    ));
                }
            }
            if let Some(outputs) = &action.node.outputs {
                md.push_str("\n\n**Outputs:**");
                for output in &outputs.node {
                    md.push_str(&format!(
                        "\n- `{}`: `{:?}`",
                        output.node.name.node, output.node.ty.node
                    ));
                }
            }
            return Some(Hover {
                contents: HoverContents::Markup(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: md,
                }),
                range: Some(span_to_range(source, action.node.name.span.clone())),
            });
        }
    }
    None
}

fn hover_reasoning_block(
    source: &str,
    reasoning: &Option<Spanned<ReasoningBlock>>,
    offset: usize,
) -> Option<Hover> {
    let reasoning = reasoning.as_ref()?;
    if !reasoning.span.contains(&offset) {
        return None;
    }
    if let Some(actions) = &reasoning.node.actions {
        for action in &actions.node {
            if action.span.contains(&offset) {
                let mut md = format!("**Reasoning Action** `{}`", action.node.name.node);
                md.push_str(&format!("\n\n**Target:** `{:?}`", action.node.target.node));
                if let Some(desc) = &action.node.description {
                    md.push_str(&format!("\n\n{}", desc.node));
                }
                if let Some(avail) = &action.node.available_when {
                    md.push_str(&format!("\n\n**Available when:** `{:?}`", avail.node));
                }
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: md,
                    }),
                    range: Some(span_to_range(source, action.node.name.span.clone())),
                });
            }
        }
    }
    None
}

fn hover_reference_at_offset(ast: &AgentFile, source: &str, offset: usize) -> Option<Hover> {
    let reference = find_reference_at_offset(source, offset)?;
    let name = reference.path.first()?;

    match reference.namespace.as_str() {
        "variables" => {
            if let Some(vars) = &ast.variables {
                for v in &vars.node.variables {
                    if &v.node.name.node == name {
                        let desc = v
                            .node
                            .description
                            .as_ref()
                            .map(|d| format!("\n\n{}", d.node))
                            .unwrap_or_default();
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: format!(
                                    "**Variable** `{}`\n\n**Kind:** {:?}  \n**Type:** `{:?}`{}",
                                    name, v.node.kind, v.node.ty.node, desc
                                ),
                            }),
                            range: None,
                        });
                    }
                }
            }
        }
        "topic" => {
            for t in &ast.topics {
                if &t.node.name.node == name {
                    let desc = t
                        .node
                        .description
                        .as_ref()
                        .map(|d| format!("\n\n{}", d.node))
                        .unwrap_or_default();
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: format!("**Topic** `{}`{}", name, desc),
                        }),
                        range: None,
                    });
                }
            }
        }
        "actions" => {
            let all = collect_all_action_defs(ast);
            for a in all {
                if &a.node.name.node == name {
                    let mut md = format!("**Action** `{}`", name);
                    if let Some(desc) = &a.node.description {
                        md.push_str(&format!("\n\n{}", desc.node));
                    }
                    if let Some(target) = &a.node.target {
                        md.push_str(&format!("\n\n**Target:** `{}`", target.node));
                    }
                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: md,
                        }),
                        range: None,
                    });
                }
            }
        }
        "utils" => {
            let desc = match name.as_str() {
                "transition" => Some("Navigate to a different topic"),
                "escalate" => Some("Escalate the conversation to a human agent"),
                "setVariables" => Some("Set multiple variable values at once"),
                _ => None,
            };
            if let Some(desc) = desc {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: format!("**Utility** `@utils.{}`\n\n{}", name, desc),
                    }),
                    range: None,
                });
            }
        }
        _ => {}
    }
    None
}

// =============================================================================
// Main
// =============================================================================

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(Backend::new)
        .custom_method("agentscript/getGraph", Backend::handle_get_graph)
        .custom_method("agentscript/getDependencies", Backend::handle_get_dependencies)
        .custom_method("agentscript/simulate", Backend::handle_simulate)
        .finish();
    Server::new(stdin, stdout, socket).serve(service).await;
}
