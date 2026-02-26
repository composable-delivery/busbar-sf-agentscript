use busbar_sf_agentscript_parser::ast::*;
use tower_lsp::lsp_types::*;

// Token types: indices into this array are used by the LSP protocol
const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE, // 0: @variables, @actions, @topic, @utils, @outputs, @context
    SemanticTokenType::TYPE,      // 1: string, boolean, date, number, etc.
    SemanticTokenType::CLASS,     // 2: topic names
    SemanticTokenType::FUNCTION,  // 3: action names
    SemanticTokenType::VARIABLE,  // 4: variable names
    SemanticTokenType::PARAMETER, // 5: action parameters
    SemanticTokenType::PROPERTY,  // 6: properties (description, target, etc.)
    SemanticTokenType::KEYWORD,   // 7: config, variables, topic, start_agent, etc.
    SemanticTokenType::STRING,    // 8: string values
    SemanticTokenType::COMMENT,   // 9: comments
    SemanticTokenType::NUMBER,    // 10: numeric literals
    SemanticTokenType::OPERATOR,  // 11: operators
];

const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,  // 0
    SemanticTokenModifier::DEFINITION,   // 1
    SemanticTokenModifier::READONLY,     // 2
    SemanticTokenModifier::MODIFICATION, // 3 (mutable)
];

lazy_static::lazy_static! {
    pub static ref LEGEND: SemanticTokensLegend = SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    };
}

/// Compute delta-encoded semantic tokens for the document.
pub fn compute_semantic_tokens(source: &str, ast: Option<&AgentFile>) -> Vec<SemanticToken> {
    let mut raw_tokens: Vec<RawToken> = Vec::new();

    // 1. Comments (scan source directly)
    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with('#') {
            let col = line.len() - trimmed.len();
            raw_tokens.push(RawToken {
                line: i as u32,
                start_char: col as u32,
                length: trimmed.len() as u32,
                token_type: 9, // comment
                modifiers: 0,
            });
        }
    }

    // 2. AST-based tokens
    if let Some(ast) = ast {
        emit_ast_tokens(source, ast, &mut raw_tokens);
    }

    // Sort by (line, start_char), then delta-encode
    raw_tokens.sort_by(|a, b| a.line.cmp(&b.line).then(a.start_char.cmp(&b.start_char)));

    let mut result = Vec::with_capacity(raw_tokens.len());
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    for tok in &raw_tokens {
        let delta_line = tok.line - prev_line;
        let delta_start = if delta_line == 0 {
            tok.start_char - prev_start
        } else {
            tok.start_char
        };
        result.push(SemanticToken {
            delta_line,
            delta_start,
            length: tok.length,
            token_type: tok.token_type,
            token_modifiers_bitset: tok.modifiers,
        });
        prev_line = tok.line;
        prev_start = tok.start_char;
    }

    result
}

struct RawToken {
    line: u32,
    start_char: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

fn offset_to_line_col(source: &str, offset: usize) -> (u32, u32) {
    let offset = offset.min(source.len());
    let mut line = 0u32;
    let mut last_line_start = 0;
    for (i, c) in source[..offset].char_indices() {
        if c == '\n' {
            line += 1;
            last_line_start = i + 1;
        }
    }
    let col = source[last_line_start..offset].chars().count() as u32;
    (line, col)
}

fn push_span_token(
    source: &str,
    tokens: &mut Vec<RawToken>,
    span: &std::ops::Range<usize>,
    token_type: u32,
    modifiers: u32,
) {
    if span.start >= span.end || span.start >= source.len() {
        return;
    }
    let (line, col) = offset_to_line_col(source, span.start);
    let text = &source[span.start..span.end.min(source.len())];
    // Only emit single-line tokens; for multi-line, emit first line only
    let first_line_len = text.find('\n').unwrap_or(text.len());
    if first_line_len > 0 {
        tokens.push(RawToken {
            line,
            start_char: col,
            length: first_line_len as u32,
            token_type,
            modifiers,
        });
    }
}

fn emit_ast_tokens(source: &str, ast: &AgentFile, tokens: &mut Vec<RawToken>) {
    // Config block
    if let Some(config) = &ast.config {
        // "config" keyword - find it in source
        if let Some(kw_start) = source[config.span.start..].find("config:") {
            let abs = config.span.start + kw_start;
            push_span_token(source, tokens, &(abs..abs + 6), 7, 0); // "config" keyword
        }
        push_span_token(source, tokens, &config.node.agent_name.span, 8, 0); // string
    }

    // Variables block
    if let Some(vars) = &ast.variables {
        if let Some(kw_start) = source[vars.span.start..].find("variables:") {
            let abs = vars.span.start + kw_start;
            push_span_token(source, tokens, &(abs..abs + 9), 7, 0);
        }
        for v in &vars.node.variables {
            let modifier = match v.node.kind {
                VariableKind::Mutable => 1 << 3, // modification
                VariableKind::Linked => 1 << 2,  // readonly
            };
            push_span_token(source, tokens, &v.node.name.span, 4, 1 | modifier); // variable + declaration
            push_span_token(source, tokens, &v.node.ty.span, 1, 0); // type
        }
    }

    // System block
    if let Some(system) = &ast.system {
        if let Some(kw_start) = source[system.span.start..].find("system:") {
            let abs = system.span.start + kw_start;
            push_span_token(source, tokens, &(abs..abs + 6), 7, 0);
        }
    }

    // Start agent
    if let Some(sa) = &ast.start_agent {
        if let Some(kw_start) = source[sa.span.start..].find("start_agent") {
            let abs = sa.span.start + kw_start;
            push_span_token(source, tokens, &(abs..abs + 11), 7, 0);
        }
        push_span_token(source, tokens, &sa.node.name.span, 2, 1); // class + declaration
        emit_actions_tokens(source, &sa.node.actions, tokens);
        emit_reasoning_tokens(source, &sa.node.reasoning, tokens);
    }

    // Topics
    for topic in &ast.topics {
        if let Some(kw_start) = source[topic.span.start..].find("topic") {
            let abs = topic.span.start + kw_start;
            push_span_token(source, tokens, &(abs..abs + 5), 7, 0);
        }
        push_span_token(source, tokens, &topic.node.name.span, 2, 1); // class + declaration
        emit_actions_tokens(source, &topic.node.actions, tokens);
        emit_reasoning_tokens(source, &topic.node.reasoning, tokens);
    }

    // Connections
    for conn in &ast.connections {
        if let Some(kw_start) = source[conn.span.start..].find("connections:") {
            let abs = conn.span.start + kw_start;
            push_span_token(source, tokens, &(abs..abs + 11), 7, 0);
        }
        push_span_token(source, tokens, &conn.node.name.span, 6, 0); // property
    }

    // Scan for @references across the entire source
    emit_reference_tokens(source, tokens);
}

fn emit_actions_tokens(
    source: &str,
    actions: &Option<Spanned<ActionsBlock>>,
    tokens: &mut Vec<RawToken>,
) {
    let Some(actions) = actions else { return };
    if let Some(kw_start) = source[actions.span.start..].find("actions:") {
        let abs = actions.span.start + kw_start;
        push_span_token(source, tokens, &(abs..abs + 7), 7, 0);
    }
    for action in &actions.node.actions {
        push_span_token(source, tokens, &action.node.name.span, 3, 1); // function + declaration
        if let Some(inputs) = &action.node.inputs {
            for input in &inputs.node {
                push_span_token(source, tokens, &input.node.name.span, 5, 0); // parameter
                push_span_token(source, tokens, &input.node.ty.span, 1, 0); // type
            }
        }
        if let Some(outputs) = &action.node.outputs {
            for output in &outputs.node {
                push_span_token(source, tokens, &output.node.name.span, 5, 0);
                push_span_token(source, tokens, &output.node.ty.span, 1, 0);
            }
        }
    }
}

fn emit_reasoning_tokens(
    source: &str,
    reasoning: &Option<Spanned<ReasoningBlock>>,
    tokens: &mut Vec<RawToken>,
) {
    let Some(reasoning) = reasoning else { return };
    if let Some(kw_start) = source[reasoning.span.start..].find("reasoning:") {
        let abs = reasoning.span.start + kw_start;
        push_span_token(source, tokens, &(abs..abs + 9), 7, 0);
    }
    if let Some(actions) = &reasoning.node.actions {
        for action in &actions.node {
            push_span_token(source, tokens, &action.node.name.span, 3, 1); // function + declaration
        }
    }
}

fn emit_reference_tokens(source: &str, tokens: &mut Vec<RawToken>) {
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'@' {
            let start = i;
            i += 1;
            // Read namespace
            let ns_start = i;
            while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                i += 1;
            }
            if i > ns_start {
                let namespace = &source[ns_start..i];
                let is_valid_ns = matches!(
                    namespace,
                    "variables" | "actions" | "outputs" | "topic" | "utils" | "context"
                );
                if is_valid_ns {
                    // Emit namespace token
                    push_span_token(source, tokens, &(start..i), 0, 0);
                    // Read .member if present
                    if i < bytes.len() && bytes[i] == b'.' {
                        i += 1;
                        let member_start = i;
                        while i < bytes.len()
                            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_')
                        {
                            i += 1;
                        }
                        if i > member_start {
                            let member_type = match namespace {
                                "variables" => 4, // variable
                                "actions" => 3,   // function
                                "topic" => 2,     // class
                                "outputs" => 6,   // property
                                "utils" => 3,     // function
                                "context" => 6,   // property
                                _ => 6,
                            };
                            push_span_token(source, tokens, &(member_start..i), member_type, 0);
                        }
                    }
                    continue;
                }
            }
        }
        i += 1;
    }
}
