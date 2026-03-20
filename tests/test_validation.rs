//! Tests for `validate_ast` — the semantic validation layer.
//!
//! `validate_ast` enforces five rules that the parser cannot catch because they
//! require semantic context rather than syntactic structure:
//!
//! 1. Mutable variables may not use `integer`, `long`, `datetime`, or `time` types.
//! 2. Linked variables sourced from `@context.*` may not be of `object` type.
//! 3. `additional_locales` codes in the `language:` block must be from the allow-list.
//! 4. `outbound_route_type` in `connection:` blocks must be `"OmniChannelFlow"`.
//! 5. Action input parameter names must not collide with AgentScript keywords.
//!
//! None of these rules had any test coverage before this file was added.

use busbar_sf_agentscript::{parse, validation::{validate_ast, Severity}};

// ============================================================================
// Rule 1 — Mutable variable type restrictions
// ============================================================================

#[test]
fn test_mutable_integer_variable_raises_error() {
    // `integer` is not a permitted type for mutable variables.  The platform
    // only accepts: String, Boolean, Number, Currency, Date, Id, Object, Timestamp.
    let source = r#"config:
   agent_name: "Test"

variables:
   counter: mutable integer = 0
      description: "Turn counter"

topic main:
   description: "Main"
   reasoning:
      instructions: "Help"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule1_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Error && e.message.contains("counter"))
        .collect();
    assert!(
        !rule1_errors.is_empty(),
        "Expected a validation error for mutable integer variable 'counter', got: {:?}",
        errors
    );
}

#[test]
fn test_mutable_string_variable_has_no_error() {
    // `string` is a permitted type for mutable variables — no error expected.
    let source = r#"config:
   agent_name: "Test"

variables:
   name: mutable string = ""
      description: "Customer name"

topic main:
   description: "Main"
   reasoning:
      instructions: "Help"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule1_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Error)
        .collect();
    assert!(
        rule1_errors.is_empty(),
        "Expected no errors for mutable string variable, got: {:?}",
        errors
    );
}

// ============================================================================
// Rule 2 — Context variable may not be object type
// ============================================================================

#[test]
fn test_context_linked_object_variable_raises_error() {
    // A linked variable whose source namespace is `@context.*` must not use the
    // `object` type — the platform cannot deserialize a generic object from context.
    let source = r#"config:
   agent_name: "Test"

variables:
   ctx_data: linked object
      description: "Context payload"
      source: @context.payload

topic main:
   description: "Main"
   reasoning:
      instructions: "Help"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule2_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Error && e.message.contains("ctx_data"))
        .collect();
    assert!(
        !rule2_errors.is_empty(),
        "Expected error for context-sourced object variable 'ctx_data', got: {:?}",
        errors
    );
}

// ============================================================================
// Rule 3 — additional_locales must be from the allow-list
// ============================================================================

#[test]
fn test_invalid_additional_locale_raises_error() {
    // "xx_INVALID" is not in the supported locale code list and must be rejected.
    let source = r#"config:
   agent_name: "Test"

language:
   additional_locales: "xx_INVALID"

topic main:
   description: "Main"
   reasoning:
      instructions: "Help"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule3_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Error && e.message.contains("xx_INVALID"))
        .collect();
    assert!(
        !rule3_errors.is_empty(),
        "Expected error for unsupported locale 'xx_INVALID', got: {:?}",
        errors
    );
}

#[test]
fn test_valid_additional_locale_has_no_error() {
    // "es_MX" and "fr_CA" are valid locale codes — no errors expected.
    let source = r#"config:
   agent_name: "Test"

language:
   additional_locales: "es_MX,fr_CA"

topic main:
   description: "Main"
   reasoning:
      instructions: "Help"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule3_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Error)
        .collect();
    assert!(
        rule3_errors.is_empty(),
        "Expected no errors for valid locales 'es_MX,fr_CA', got: {:?}",
        errors
    );
}

// ============================================================================
// Rule 4 — outbound_route_type must be "OmniChannelFlow"
// ============================================================================

#[test]
fn test_invalid_outbound_route_type_raises_error() {
    // Any value other than the literal string "OmniChannelFlow" is invalid.
    let source = r#"config:
   agent_name: "Test"

connection live_agent:
   escalation_message: "Connecting you now."
   outbound_route_type: "Queue"
   outbound_route_name: "SupportQueue"

topic main:
   description: "Main"
   reasoning:
      instructions: "Help"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule4_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Error && e.message.contains("outbound_route_type"))
        .collect();
    assert!(
        !rule4_errors.is_empty(),
        "Expected error for outbound_route_type \"Queue\", got: {:?}",
        errors
    );
}

#[test]
fn test_valid_outbound_route_type_has_no_error() {
    // "OmniChannelFlow" is the only accepted value and must not produce an error.
    let source = r#"config:
   agent_name: "Test"

connection live_agent:
   escalation_message: "Connecting you now."
   outbound_route_type: "OmniChannelFlow"
   outbound_route_name: "SupportQueue"

topic main:
   description: "Main"
   reasoning:
      instructions: "Help"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule4_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Error)
        .collect();
    assert!(
        rule4_errors.is_empty(),
        "Expected no errors for valid outbound_route_type, got: {:?}",
        errors
    );
}

// ============================================================================
// Rule 5 — Action input parameter name keyword collision
// ============================================================================

#[test]
fn test_action_input_named_description_raises_warning() {
    // An action input named "description" shadows the AgentScript keyword and the
    // platform may misparse the intent.  The validator should emit a Warning.
    let source = r#"config:
   agent_name: "Test"

topic main:
   description: "Main"

   actions:
      lookup:
         description: "Looks up a record"
         inputs:
            description: string
               description: "The record description (bad param name)"
         outputs:
            result: string
               description: "Lookup result"
         target: "flow://Lookup"

   reasoning:
      instructions: "Help the user"
"#;
    let ast = parse(source).expect("Should parse successfully");
    let errors = validate_ast(&ast);
    let rule5_warnings: Vec<_> = errors
        .iter()
        .filter(|e| e.severity == Severity::Warning && e.message.contains("description"))
        .collect();
    assert!(
        !rule5_warnings.is_empty(),
        "Expected a warning for action input named 'description', got: {:?}",
        errors
    );
}
