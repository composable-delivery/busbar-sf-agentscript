//! Parser for AgentScript source code.
//!
//! This module provides a complete parser that converts AgentScript source
//! into a typed Abstract Syntax Tree ([`AgentFile`]).
//!
//! # Architecture
//!
//! The parser uses a two-phase approach:
//!
//! 1. **Lexical analysis** - Source → Tokens (via [`crate::lexer`])
//! 2. **Parsing** - Tokens → AST (via chumsky combinators)
//!
//! # Usage
//!
//! ```rust
//! use busbar_sf_agentscript::parser::parse;
//!
//! let source = r#"
//! config:
//!    agent_name: "MyAgent"
//!
//! topic main:
//!    description: "Main topic"
//! "#;
//!
//! match parse(source) {
//!     Ok(agent) => {
//!         println!("Parsed {} topics", agent.topics.len());
//!     }
//!     Err(errors) => {
//!         for err in errors {
//!             eprintln!("{}", err);
//!         }
//!     }
//! }
//! ```
//!
//! # Error Handling
//!
//! Use [`parse_with_errors()`] for partial parsing that returns both
//! the result and any errors encountered:
//!
//! ```rust
//! use busbar_sf_agentscript::parser::parse_with_errors;
//!
//! let source = "config:\n   agent_name: \"Test\"";
//! let (result, errors) = parse_with_errors(source);
//!
//! if let Some(agent) = result {
//!     println!("Parsed successfully");
//! }
//! for err in errors {
//!     eprintln!("Warning: {}", err);
//! }
//! ```
//!
//! # Module Structure
//!
//! The parser is split into submodules for each block type:
//!
//! - `config` - Config block parsing
//! - `variables` - Variable declarations
//! - `system` - System instructions and messages
//! - `topics` - Topic and start_agent blocks
//! - `actions` - Action definitions
//! - `reasoning` - Reasoning blocks
//! - `expressions` - Expression parsing
//! - `instructions` - Static and dynamic instructions
//!
//! [`AgentFile`]: crate::ast::AgentFile

mod actions;
mod config;
mod connections;
mod directives;
mod expressions;
mod instructions;
mod language;
mod primitives;
mod reasoning;
mod system;
#[cfg(not(test))]
mod tests;
mod topics;
mod variables;

use crate::ast::AgentFile;
use crate::lexer;

// Re-export the span type
pub use primitives::Span;

/// Convert a character offset to (line, column) - both 1-indexed
fn offset_to_line_col(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

/// Get the line content at a given line number (1-indexed)
fn get_line_content(source: &str, line_num: usize) -> &str {
    source.lines().nth(line_num.saturating_sub(1)).unwrap_or("")
}

/// Format a chumsky Rich error into a human-readable string with line/column info
fn format_parse_error<'tokens, 'src>(
    source: &str,
    error: &Rich<'tokens, crate::lexer::Token<'src>, primitives::Span>,
) -> String {
    let span = error.span();
    let (line, col) = offset_to_line_col(source, span.start);
    let line_content = get_line_content(source, line);

    // Build expected string
    let expected: Vec<String> = error.expected().map(|e| format!("{}", e)).collect();
    let expected_str = if expected.is_empty() {
        String::new()
    } else if expected.len() == 1 {
        format!(", expected {}", expected[0])
    } else {
        format!(", expected one of: {}", expected.join(", "))
    };

    // Build found string
    let found_str = match error.found() {
        Some(tok) => format!("found '{}'", tok),
        None => "found end of input".to_string(),
    };

    // Build context chain from .labelled() calls - shows WHERE in the parse tree we failed
    let contexts: Vec<_> = error.contexts().collect();
    let context_str = if contexts.is_empty() {
        String::new()
    } else {
        let ctx_labels: Vec<String> = contexts
            .iter()
            .map(|(label, ctx_span)| {
                let (ctx_line, _) = offset_to_line_col(source, ctx_span.start);
                format!("{} (line {})", label, ctx_line)
            })
            .collect();
        format!("\n  while parsing: {}", ctx_labels.join(" > "))
    };

    // Format with context
    format!(
        "Error at line {}, column {}: {}{}{}\n  |\n{:>3} | {}\n  | {}{}",
        line,
        col,
        found_str,
        expected_str,
        context_str,
        line,
        line_content,
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(
            (span.end - span.start)
                .max(1)
                .min(line_content.len().saturating_sub(col - 1).max(1))
        )
    )
}

/// Format a lexer error into a human-readable string
fn format_lexer_error(
    source: &str,
    error: &impl std::fmt::Debug,
    span_start: usize,
    span_end: usize,
) -> String {
    let (line, col) = offset_to_line_col(source, span_start);
    let line_content = get_line_content(source, line);

    format!(
        "Lexer error at line {}, column {}: {:?}\n  |\n{:>3} | {}\n  | {}{}",
        line,
        col,
        error,
        line,
        line_content,
        " ".repeat(col.saturating_sub(1)),
        "^".repeat(
            (span_end - span_start)
                .max(1)
                .min(line_content.len().saturating_sub(col - 1).max(1))
        )
    )
}

// Re-export primitives needed by agent_file_parser
use primitives::{skip_toplevel_noise, ParserInput};

use chumsky::input::Input as _;
use chumsky::prelude::*;
use chumsky::recovery::skip_then_retry_until;

use config::config_block;
use connections::{connection_block, legacy_connections_block};
use language::language_block;
use system::system_block;
use topics::{start_agent_block, topic_block};
use variables::variables_block;

/// Parse an AgentScript file from source code.
///
/// Returns Ok only if parsing succeeds with no errors.
/// Use `parse_with_errors` to get partial results and all errors.
pub fn parse(source: &str) -> Result<AgentFile, Vec<String>> {
    let (result, errors) = parse_with_errors(source);
    if errors.is_empty() {
        result.ok_or_else(|| vec!["Unknown parse error".to_string()])
    } else {
        Err(errors)
    }
}

/// Parse an AgentScript file from source with full error reporting.
///
/// Returns both a partial AST (if recovery succeeded) and ALL errors found.
/// This allows collecting multiple errors in a single parse pass.
pub fn parse_with_errors(source: &str) -> (Option<AgentFile>, Vec<String>) {
    // Phase 1: Lexical analysis with indentation tokens
    let tokens = match lexer::lex_with_indentation(source) {
        Ok(tokens) => tokens,
        Err(errs) => {
            let errors: Vec<String> = errs
                .iter()
                .map(|e| {
                    let span = e.span();
                    format_lexer_error(source, &e.reason(), span.start, span.end)
                })
                .collect();
            return (None, errors);
        }
    };

    // Phase 2: Parse into AST using token-based parser
    let eoi_span = primitives::Span::new((), source.len()..source.len());
    let token_stream = tokens.as_slice().split_token_span(eoi_span);

    // Use into_output_errors to get BOTH partial results AND all errors
    let (result, errs) = agent_file_parser().parse(token_stream).into_output_errors();

    let errors: Vec<String> = errs.iter().map(|e| format_parse_error(source, e)).collect();
    (result, errors)
}

/// Parse an AgentScript file and return structured errors with span information.
///
/// Returns Ok only if parsing succeeds with no errors.
/// Use `parse_with_structured_errors_all` to get partial results and all errors.
pub fn parse_with_structured_errors(
    source: &str,
) -> Result<AgentFile, Vec<crate::error::ParseErrorInfo>> {
    let (result, errors) = parse_with_structured_errors_all(source);
    if errors.is_empty() {
        result.ok_or_else(|| {
            vec![crate::error::ParseErrorInfo {
                message: "Unknown parse error".to_string(),
                span: None,
                expected: vec![],
                found: None,
                contexts: vec![],
            }]
        })
    } else {
        Err(errors)
    }
}

/// Parse an AgentScript file and return structured errors with span information.
///
/// Returns both a partial AST (if recovery succeeded) and ALL errors found.
pub fn parse_with_structured_errors_all(
    source: &str,
) -> (Option<AgentFile>, Vec<crate::error::ParseErrorInfo>) {
    use crate::error::ParseErrorInfo;

    // Phase 1: Lexical analysis with indentation tokens
    let tokens = match lexer::lex_with_indentation(source) {
        Ok(tokens) => tokens,
        Err(errs) => {
            let errors: Vec<ParseErrorInfo> = errs
                .iter()
                .map(|e| {
                    let span = e.span();
                    let (line, col) = offset_to_line_col(source, span.start);
                    ParseErrorInfo {
                        message: format!(
                            "Lexer error at line {}, column {}: {}",
                            line,
                            col,
                            e.reason()
                        ),
                        span: Some(span.start..span.end),
                        expected: vec![],
                        found: None,
                        contexts: vec![],
                    }
                })
                .collect();
            return (None, errors);
        }
    };

    // Phase 2: Parse into AST using token-based parser
    let eoi_span = primitives::Span::new((), source.len()..source.len());
    let token_stream = tokens.as_slice().split_token_span(eoi_span);

    // Use into_output_errors to get BOTH partial results AND all errors
    let (result, errs) = agent_file_parser().parse(token_stream).into_output_errors();

    let errors: Vec<ParseErrorInfo> = errs
        .iter()
        .map(|e| {
            let span = e.span();
            let (line, col) = offset_to_line_col(source, span.start);
            // Collect contexts from labelled parsers
            let contexts: Vec<(String, std::ops::Range<usize>)> = e
                .contexts()
                .map(|(label, ctx_span)| (label.to_string(), ctx_span.start..ctx_span.end))
                .collect();

            ParseErrorInfo {
                message: format!("Parse error at line {}, column {}: {}", line, col, e.reason()),
                span: Some(span.start..span.end),
                expected: e.expected().map(|exp| format!("{}", exp)).collect(),
                found: e.found().map(|tok| format!("{}", tok)),
                contexts,
            }
        })
        .collect();

    (result, errors)
}

// ============================================================================
// Top-Level Agent File Parser
// ============================================================================

use crate::ast::{
    ConfigBlock, ConnectionBlock, LanguageBlock, Spanned, StartAgentBlock, SystemBlock, TopicBlock,
    VariablesBlock,
};
use crate::lexer::Token;

/// Enum for tracking parsed top-level blocks.
enum TopLevelBlock {
    Config(Spanned<ConfigBlock>),
    Variables(Spanned<VariablesBlock>),
    System(Spanned<SystemBlock>),
    StartAgent(Spanned<StartAgentBlock>),
    Topic(Spanned<TopicBlock>),
    Language(Spanned<LanguageBlock>),
    Connection(Spanned<ConnectionBlock>),
}

/// Parse a complete agent file.
fn agent_file_parser<'tokens, 'src: 'tokens>() -> impl Parser<
    'tokens,
    ParserInput<'tokens, 'src>,
    AgentFile,
    extra::Err<Rich<'tokens, Token<'src>, primitives::Span>>,
> + Clone {
    // Top-level blocks - each with noise handling before them (including DEDENTs)
    let config = skip_toplevel_noise()
        .ignore_then(config_block())
        .map(TopLevelBlock::Config);
    let variables = skip_toplevel_noise()
        .ignore_then(variables_block())
        .map(TopLevelBlock::Variables);
    let system = skip_toplevel_noise()
        .ignore_then(system_block())
        .map(TopLevelBlock::System);
    let start_agent = skip_toplevel_noise()
        .ignore_then(start_agent_block())
        .map(TopLevelBlock::StartAgent);
    let topic = skip_toplevel_noise()
        .ignore_then(topic_block())
        .map(TopLevelBlock::Topic);
    let language = skip_toplevel_noise()
        .ignore_then(language_block())
        .map(TopLevelBlock::Language);
    let connection = skip_toplevel_noise()
        .ignore_then(connection_block())
        .map(TopLevelBlock::Connection);

    // Legacy connections: block - emit helpful error message
    let legacy_connections = skip_toplevel_noise().ignore_then(legacy_connections_block());

    // Skip trailing whitespace including any final DEDENTs
    let trailing_noise = skip_toplevel_noise();

    // Recovery strategy: when parsing fails, skip until we find a top-level keyword
    // and retry. This captures errors with proper context from .labelled() calls.
    let recovery_until = choice((
        just(Token::Topic).ignored(),
        just(Token::StartAgent).ignored(),
        just(Token::Config).ignored(),
        just(Token::Variables).ignored(),
        just(Token::System).ignored(),
        just(Token::Language).ignored(),
        just(Token::Connection).ignored(),
    ));

    // Parse blocks with choice (try each parser)
    // Note: legacy_connections must come AFTER connection to avoid early matching
    choice((config, variables, system, start_agent, topic, language, connection))
        .or(legacy_connections.map(|_| {
            // This shouldn't be reached since legacy_connections emits an error,
            // but we need to return something for type checking
            TopLevelBlock::Config(Spanned::new(
                ConfigBlock {
                    agent_name: Spanned::new("error".to_string(), 0..0),
                    agent_label: None,
                    description: None,
                    agent_type: None,
                    default_agent_user: None,
                },
                0..0,
            ))
        }))
        .recover_with(skip_then_retry_until(any().ignored(), recovery_until))
        .repeated()
        .collect::<Vec<_>>()
        .then_ignore(trailing_noise)
        .then_ignore(end())
        .map(|blocks| {
            let mut file = AgentFile::default();

            for block in blocks {
                match block {
                    TopLevelBlock::Config(c) => file.config = Some(c),
                    TopLevelBlock::Variables(v) => file.variables = Some(v),
                    TopLevelBlock::System(s) => file.system = Some(s),
                    TopLevelBlock::StartAgent(sa) => file.start_agent = Some(sa),
                    TopLevelBlock::Topic(t) => file.topics.push(t),
                    TopLevelBlock::Language(l) => file.language = Some(l),
                    TopLevelBlock::Connection(c) => file.connections.push(c),
                }
            }

            file
        })
}
