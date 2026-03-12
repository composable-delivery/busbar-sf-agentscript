//! Tests for parser error paths — verifies that malformed AgentScript inputs
//! are rejected with errors rather than silently producing incorrect ASTs.
//!
//! These tests complement the success-case integration tests by ensuring the
//! parser correctly diagnoses invalid syntax.

use busbar_sf_agentscript::parse;

// ─── Variable declaration validators ─────────────────────────────────────────

#[test]
fn test_none_default_for_string_variable_is_rejected() {
    // The parser validator rejects `= None` for non-boolean types.
    // A `mutable string` variable must not have `None` as its default value.
    let source = r#"config:
   agent_name: "Test"

variables:
   order_id: mutable string = None

topic main:
   description: "Main"
"#;
    let result = parse(source);
    assert!(
        result.is_err(),
        "Expected parse error for `mutable string = None`, but parsing succeeded"
    );
}

#[test]
fn test_none_default_for_integer_variable_is_rejected() {
    // Same validator: `= None` is also invalid for integer types.
    let source = r#"config:
   agent_name: "Test"

variables:
   count: mutable integer = None

topic main:
   description: "Main"
"#;
    let result = parse(source);
    assert!(
        result.is_err(),
        "Expected parse error for `mutable integer = None`, but parsing succeeded"
    );
}

#[test]
fn test_none_default_for_boolean_variable_is_valid() {
    // `= None` is explicitly allowed for boolean types (represents "not set").
    // This test confirms the validator does not fire for boolean = None.
    let source = r#"config:
   agent_name: "Test"

variables:
   confirmed: mutable boolean = None

topic main:
   description: "Main"
"#;
    let result = parse(source);
    assert!(
        result.is_ok(),
        "Expected `mutable boolean = None` to be valid, but got errors: {:?}",
        result.err()
    );
}

// ─── Config block type validation ────────────────────────────────────────────

#[test]
fn test_config_field_value_must_be_a_string_literal() {
    // Config entries expect string literals.  A bare number (`42`) is not a
    // valid string and the parser must reject it.
    let source = r#"config:
   agent_name: 42
"#;
    let result = parse(source);
    assert!(
        result.is_err(),
        "Expected parse error when config field value is not a string literal"
    );
}

// ─── Edge case: empty input ───────────────────────────────────────────────────

#[test]
fn test_empty_file_parses_to_default_agent() {
    // An empty file is syntactically valid — it produces an AgentFile with all
    // optional fields absent (no config, no topics, etc.).
    let result = parse("");
    assert!(
        result.is_ok(),
        "Expected empty file to parse successfully, got: {:?}",
        result.err()
    );
    let agent = result.unwrap();
    assert!(agent.config.is_none(), "Empty file should have no config block");
    assert!(agent.topics.is_empty(), "Empty file should have no topics");
    assert!(agent.variables.is_none(), "Empty file should have no variables block");
}
