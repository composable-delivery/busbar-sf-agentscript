//! Tests for `validate_ast` semantic validation rules.
//!
//! The `validate_ast` function enforces five rules on a parsed AgentScript AST.
//! These tests cover every rule to ensure violations are reported with the
//! correct severity and that valid inputs produce no errors.

use busbar_sf_agentscript::{parse, validate_ast, validation::Severity};

// ---------------------------------------------------------------------------
// Rule 1 & 2 – Mutable variable type restrictions
// ---------------------------------------------------------------------------

/// Valid mutable types (string, number, boolean, object) should not produce
/// any validation errors.
#[test]
fn test_validate_valid_mutable_variable_types() {
    let src = r#"config:
   agent_name: "TestAgent"

variables:
   name: mutable string = ""
      description: "Name"
   score: mutable number = 0.0
      description: "Score"
   active: mutable boolean = False
      description: "Active flag"
   data: mutable object = {}
      description: "Data blob"

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert!(
        errors.is_empty(),
        "Expected no validation errors, got: {:?}",
        errors
    );
}

/// A `mutable integer` variable violates Rule 1 and must produce an Error.
#[test]
fn test_validate_mutable_integer_produces_error() {
    let src = r#"config:
   agent_name: "TestAgent"

variables:
   retry_count: mutable integer = 0
      description: "Retry counter"

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert_eq!(errors.len(), 1, "Expected exactly one validation error");
    assert!(
        matches!(errors[0].severity, Severity::Error),
        "Expected Error severity"
    );
    assert!(
        errors[0].message.contains("retry_count"),
        "Error message should name the offending variable"
    );
}

/// A `mutable long` variable also violates Rule 1 and must produce an Error.
#[test]
fn test_validate_mutable_long_produces_error() {
    let src = r#"config:
   agent_name: "TestAgent"

variables:
   big_number: mutable long = 0
      description: "A long value"

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert_eq!(errors.len(), 1, "Expected exactly one validation error");
    assert!(
        matches!(errors[0].severity, Severity::Error),
        "Expected Error severity for mutable long"
    );
}

/// Multiple disallowed mutable types in one file each produce their own Error.
#[test]
fn test_validate_multiple_disallowed_mutable_types() {
    // `integer`, `long`, `datetime`, and `time` are all forbidden for mutable.
    let src = r#"config:
   agent_name: "TestAgent"

variables:
   a: mutable integer = 0
   b: mutable long = 0

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert_eq!(
        errors.len(),
        2,
        "Expected one error per disallowed mutable type; got: {:?}",
        errors
    );
    assert!(errors.iter().all(|e| matches!(e.severity, Severity::Error)));
}

// ---------------------------------------------------------------------------
// Rule 3 – Locale code validation
// ---------------------------------------------------------------------------

/// A recognised locale code in `additional_locales` must not produce errors.
#[test]
fn test_validate_valid_additional_locale() {
    let src = r#"config:
   agent_name: "TestAgent"

language:
   locale: "en_US"
   additional_locales: "es_MX"

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert!(
        errors.is_empty(),
        "Valid locale should produce no errors; got: {:?}",
        errors
    );
}

/// An unrecognised locale code in `additional_locales` must produce an Error.
#[test]
fn test_validate_invalid_additional_locale_produces_error() {
    let src = r#"config:
   agent_name: "TestAgent"

language:
   additional_locales: "xx_INVALID"

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert_eq!(errors.len(), 1, "Expected exactly one locale error; got: {:?}", errors);
    assert!(
        matches!(errors[0].severity, Severity::Error),
        "Locale error should have Error severity"
    );
    assert!(
        errors[0].message.contains("xx_INVALID"),
        "Error should mention the invalid locale code"
    );
}

// ---------------------------------------------------------------------------
// Rule 4 – Outbound route type validation
// ---------------------------------------------------------------------------

/// `outbound_route_type: "OmniChannelFlow"` is the only valid value – no error.
#[test]
fn test_validate_valid_outbound_route_type() {
    let src = r#"config:
   agent_name: "TestAgent"

connection messaging:
   escalation_message: "Connecting you now."
   outbound_route_type: "OmniChannelFlow"
   outbound_route_name: "Queue1"

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert!(
        errors.is_empty(),
        "Valid outbound_route_type should produce no errors; got: {:?}",
        errors
    );
}

/// Any value other than `"OmniChannelFlow"` must produce an Error.
#[test]
fn test_validate_invalid_outbound_route_type_produces_error() {
    let src = r#"config:
   agent_name: "TestAgent"

connection messaging:
   escalation_message: "Connecting you now."
   outbound_route_type: "DirectTransfer"
   outbound_route_name: "Queue1"

topic main:
   description: "Main"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert_eq!(
        errors.len(),
        1,
        "Expected one outbound_route_type error; got: {:?}",
        errors
    );
    assert!(
        matches!(errors[0].severity, Severity::Error),
        "outbound_route_type error should have Error severity"
    );
    assert!(
        errors[0].message.contains("DirectTransfer"),
        "Error should quote the invalid value"
    );
}

// ---------------------------------------------------------------------------
// Rule 5 – Action input keyword collision
// ---------------------------------------------------------------------------

/// An action input parameter named `description` collides with a keyword and
/// must produce a Warning.
#[test]
fn test_validate_action_input_keyword_collision_produces_warning() {
    let src = r#"config:
   agent_name: "TestAgent"

topic main:
   description: "Main"
   actions:
      do_thing:
         description: "Does a thing"
         inputs:
            description: string
               description: "Collides with keyword"
         outputs:
            result: string
               description: "Result"
         target: "flow://DoThing"
   reasoning:
      instructions: "Help the user"
"#;
    let ast = parse(src).expect("parse failed");
    let errors = validate_ast(&ast);
    assert_eq!(
        errors.len(),
        1,
        "Expected one keyword-collision warning; got: {:?}",
        errors
    );
    assert!(
        matches!(errors[0].severity, Severity::Warning),
        "Keyword collision should be a Warning, not an Error"
    );
    assert!(
        errors[0].message.contains("description"),
        "Warning should name the colliding parameter"
    );
}
