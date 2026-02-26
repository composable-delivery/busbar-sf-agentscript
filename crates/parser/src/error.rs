//! Error types and error reporting for AgentScript.
//!
//! This module provides structured error types and pretty error reporting
//! using the [ariadne](https://crates.io/crates/ariadne) crate for colorful,
//! context-aware error messages.
//!
//! # Error Types
//!
//! - [`AgentScriptError`] - Main error enum for parse and validation errors
//! - [`ParseErrorInfo`] - Details about parse failures
//! - [`ValidationError`] - Semantic validation errors
//!
//! # Pretty Printing
//!
//! Use [`ErrorReporter`] for user-friendly error output:
//!
//! ```rust
//! use busbar_sf_agentscript_parser::error::{ErrorReporter, ParseErrorInfo};
//!
//! let source = "config:\n   agent_name: bad";
//! let reporter = ErrorReporter::new("example.agent", source);
//!
//! // Create and report an error
//! let error = ParseErrorInfo {
//!     message: "Expected string literal".to_string(),
//!     span: Some(18..21),
//!     expected: vec!["string".to_string()],
//!     found: Some("identifier".to_string()),
//!     contexts: vec![],
//! };
//! // reporter.report_parse_error(&error); // Prints colorful error
//! ```

use ariadne::{Color, Label, Report, ReportKind, Source};
use std::fmt;

/// The main error type for AgentScript operations.
///
/// This enum encompasses all errors that can occur during parsing
/// and validation of AgentScript source code.
#[derive(Debug)]
pub enum AgentScriptError {
    /// Parse error.
    Parse(ParseErrorInfo),
    /// Validation error.
    Validation(ValidationError),
}

impl fmt::Display for AgentScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentScriptError::Parse(e) => write!(f, "Parse error: {}", e),
            AgentScriptError::Validation(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl std::error::Error for AgentScriptError {}

/// Information about a parse error.
#[derive(Debug)]
pub struct ParseErrorInfo {
    pub message: String,
    pub span: Option<std::ops::Range<usize>>,
    pub expected: Vec<String>,
    pub found: Option<String>,
    /// Context chain from labelled parsers - shows parse tree path to failure
    pub contexts: Vec<(String, std::ops::Range<usize>)>,
}

impl fmt::Display for ParseErrorInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref found) = self.found {
            write!(f, ", found '{}'", found)?;
        }
        if !self.expected.is_empty() {
            write!(f, ", expected one of: {}", self.expected.join(", "))?;
        }
        Ok(())
    }
}

/// Validation error for semantic issues.
#[derive(Debug)]
pub struct ValidationError {
    pub message: String,
    pub span: Option<std::ops::Range<usize>>,
    pub hint: Option<String>,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref hint) = self.hint {
            write!(f, " (hint: {})", hint)?;
        }
        Ok(())
    }
}

/// Error reporter that uses ariadne for pretty error output.
pub struct ErrorReporter<'src> {
    source_name: String,
    source: &'src str,
}

impl<'src> ErrorReporter<'src> {
    /// Create a new error reporter.
    pub fn new(source_name: impl Into<String>, source: &'src str) -> Self {
        Self {
            source_name: source_name.into(),
            source,
        }
    }

    /// Report a parse error to stderr.
    pub fn report_parse_error(&self, error: &ParseErrorInfo) {
        let span = error.span.clone().unwrap_or(0..0);

        let mut report = Report::build(ReportKind::Error, &self.source_name, span.start)
            .with_message(&error.message);

        let mut label = Label::new((&self.source_name, span.clone())).with_color(Color::Red);

        if let Some(ref found) = error.found {
            label = label.with_message(format!("found '{}'", found));
        }

        report = report.with_label(label);

        // Add context labels for the parse tree path
        for (i, (ctx_label, ctx_span)) in error.contexts.iter().enumerate() {
            let color = if i == 0 { Color::Yellow } else { Color::Cyan };
            report = report.with_label(
                Label::new((&self.source_name, ctx_span.clone()))
                    .with_color(color)
                    .with_message(format!("while parsing {}", ctx_label))
                    .with_order(i as i32 + 1), // Order labels by depth
            );
        }

        if !error.expected.is_empty() {
            report = report.with_note(format!("expected one of: {}", error.expected.join(", ")));
        }

        report
            .finish()
            .eprint((&self.source_name, Source::from(self.source)))
            .unwrap();
    }

    /// Report a validation error to stderr.
    pub fn report_validation_error(&self, error: &ValidationError) {
        let span = error.span.clone().unwrap_or(0..0);

        let mut report = Report::build(ReportKind::Error, &self.source_name, span.start)
            .with_message(&error.message)
            .with_label(
                Label::new((&self.source_name, span))
                    .with_color(Color::Yellow)
                    .with_message("here"),
            );

        if let Some(ref hint) = error.hint {
            report = report.with_help(hint);
        }

        report
            .finish()
            .eprint((&self.source_name, Source::from(self.source)))
            .unwrap();
    }
}

/// Result type for AgentScript operations.
pub type Result<T> = std::result::Result<T, AgentScriptError>;
