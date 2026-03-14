//! Tests for parser error paths — invalid/malformed AgentScript input.
//!
//! These tests verify that the parser correctly rejects malformed input and
//! surfaces meaningful errors, rather than silently accepting bad syntax.

use busbar_sf_agentscript::parse;
use busbar_sf_agentscript::parser::parse_with_errors;

// ============================================================================
// Lexer-level errors
// ============================================================================

#[test]
fn test_parse_error_unclosed_string_literal() {
    // An unterminated string literal is a lexer error: the opening `"` is
    // consumed but the lexer never finds the matching closing `"`.
    // `parse()` must return Err, not silently produce a partial AST.
    let source = r#"config:
   agent_name: "unclosed string literal
"#;
    let result = parse(source);
    assert!(
        result.is_err(),
        "Expected Err for unclosed string literal, got Ok"
    );
}

#[test]
fn test_parse_with_errors_returns_error_info_for_unclosed_string() {
    // `parse_with_errors` should return a non-empty error vector (and no AST)
    // when the source contains a lexer-level error like an unclosed string.
    let source = "agent_name: \"not closed";
    let (ast, errors) = parse_with_errors(source);
    assert!(
        !errors.is_empty(),
        "Expected at least one error from parse_with_errors for unclosed string"
    );
    // The AST should be absent — the lexer fails before parsing begins.
    assert!(
        ast.is_none(),
        "Expected no AST when lexer encounters an unclosed string literal"
    );
}

// ============================================================================
// Parser validation errors
// ============================================================================

#[test]
fn test_parse_error_none_default_on_integer_variable() {
    // `= None` is only valid for boolean-type variables.  Using it on an
    // `integer` variable should trigger the inline `.validate()` check in the
    // variables parser and cause `parse()` to return Err.
    let source = r#"config:
   agent_name: "Test"

variables:
   count: mutable integer = None
      description: "A counter"

topic main:
   description: "Main topic"
"#;
    let result = parse(source);
    assert!(
        result.is_err(),
        "Expected Err when '= None' is used on a non-boolean variable, got Ok"
    );
}

#[test]
fn test_parse_error_none_default_on_string_variable() {
    // Same constraint: `= None` on a `string` variable should also fail.
    let source = r#"config:
   agent_name: "Test"

variables:
   name: mutable string = None
      description: "A name"
"#;
    let result = parse(source);
    assert!(
        result.is_err(),
        "Expected Err when '= None' is used on a 'string' variable, got Ok"
    );
}

// ============================================================================
// Parser structural errors
// ============================================================================

#[test]
fn test_parse_error_topic_missing_name() {
    // `topic:` without an identifier after it does not match the grammar for a
    // topic block, which requires `topic <ident>:`.  The parser's error-
    // recovery strategy skips the malformed block and emits an error, so
    // `parse()` returns Err with a non-empty error vector.
    let source = r#"config:
   agent_name: "Test"

topic:
   description: "Topic without a name"
"#;
    let (_, errors) = parse_with_errors(source);
    assert!(
        !errors.is_empty(),
        "Expected parse errors when a topic block is missing its name identifier"
    );
}
