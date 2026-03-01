//! Integration test for AST serialization (parse → serialize → parse).
//!
//! This test verifies that:
//! 1. We can parse AgentScript source to AST
//! 2. We can serialize AST back to AgentScript source
//! 3. Re-parsing the serialized source produces equivalent AST

use busbar_sf_agentscript::{parse, serialize};

#[test]
fn test_roundtrip_minimal_config() {
    let original = r#"config:
   agent_name: "TestAgent"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);
    let reparsed = parse(&serialized).expect("Failed to reparse serialized");

    // Check key properties are preserved
    assert_eq!(
        ast.config.as_ref().unwrap().node.agent_name.node,
        reparsed.config.as_ref().unwrap().node.agent_name.node
    );
}

#[test]
fn test_roundtrip_config_and_topic() {
    let original = r#"config:
   agent_name: "Test"

topic main:
   description: "Main topic"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);
    let reparsed = parse(&serialized).expect("Failed to reparse serialized");

    assert_eq!(reparsed.topics.len(), 1);
    assert_eq!(reparsed.topics[0].node.name.node, "main");
}

#[test]
fn test_serialize_preserves_structure() {
    let original = r#"config:
   agent_name: "OrderAgent"

topic main:
   description: "Main topic"
   actions:
      create_order:
         description: "Create a new order"
         inputs:
            customer_id: string
               description: "Customer identifier"
         outputs:
            order_id: string
               description: "Created order ID"
         target: "flow://CreateOrder"
   reasoning:
      instructions: "Help customer place orders"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    // Check that serialized output contains expected sections
    assert!(serialized.contains("config:"));
    assert!(serialized.contains("actions:"));
    assert!(serialized.contains("topic main:"));
    assert!(serialized.contains("create_order:"));
    assert!(serialized.contains("inputs:"));
    assert!(serialized.contains("outputs:"));
    assert!(serialized.contains("target: \"flow://CreateOrder\""));

    // Reparse to ensure it's valid
    let reparsed = parse(&serialized).expect("Failed to reparse serialized output");
    assert_eq!(reparsed.topics.len(), 1);
}

#[test]
fn test_roundtrip_with_variables() {
    let original = r#"config:
   agent_name: "TestAgent"

variables:
   counter: mutable integer = 0
   name: mutable string = ""

topic test:
   description: "Test"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);
    let reparsed = parse(&serialized).expect("Failed to reparse serialized");

    let vars = &reparsed.variables.as_ref().unwrap().node;
    assert_eq!(vars.variables.len(), 2);
}

#[test]
fn test_roundtrip_static_instructions() {
    let original = r#"config:
   agent_name: "TestAgent"

topic support:
   description: "Support topic"
   reasoning:
      instructions: "Help the customer resolve their issue"
      actions:
         check_status: @actions.check_status
            description: "Check order status"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);
    let reparsed = parse(&serialized).expect("Failed to reparse serialized");

    assert_eq!(reparsed.topics.len(), 1);
    let topic = &reparsed.topics[0].node;
    assert_eq!(topic.name.node, "support");
    assert!(topic.reasoning.is_some());
}

#[test]
fn test_roundtrip_connection_block() {
    // Covers the `connection <name>:` block — the serializer writes it but no roundtrip
    // test existed.  Ensures escalation routing metadata survives a parse → serialize →
    // parse cycle.
    let original = r#"config:
   agent_name: "EscalationAgent"

connection live_agent:
   escalation_message: "Connecting you with a specialist."
   outbound_route_type: "OmniChannelFlow"
   outbound_route_name: "SpecialistQueue"

topic main:
   description: "Main"
   reasoning:
      instructions: "Help or escalate"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    // Serialized output must contain connection block markers
    assert!(serialized.contains("connection live_agent:"), "Missing connection block header");
    assert!(serialized.contains("SpecialistQueue"), "Missing outbound_route_name value");

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    assert_eq!(reparsed.connections.len(), 1);
    assert_eq!(reparsed.connections[0].node.name.node, "live_agent");
    assert_eq!(reparsed.connections[0].node.entries.len(), 3);
}

#[test]
fn test_roundtrip_language_block() {
    // Covers the `language:` block — the serializer writes it but no roundtrip test
    // existed.  Ensures locale settings survive a parse → serialize → parse cycle.
    let original = r#"config:
   agent_name: "LocaleAgent"

language:
   locale: "en_US"

topic main:
   description: "Main"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    assert!(serialized.contains("language:"), "Missing language block");
    assert!(serialized.contains("locale"), "Missing locale entry");

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    assert!(reparsed.language.is_some(), "language block lost after roundtrip");
    assert_eq!(reparsed.language.as_ref().unwrap().node.entries.len(), 1);
}

#[test]
fn test_roundtrip_before_and_after_reasoning() {
    // Covers `before_reasoning:` and `after_reasoning:` directive blocks inside a topic.
    // The serializer handles these blocks but no roundtrip test existed.
    let original = r#"config:
   agent_name: "DirectiveAgent"

variables:
   turn_count: mutable integer = 0

topic main:
   description: "Main"

   before_reasoning:
      set @variables.turn_count = @variables.turn_count + 1

   reasoning:
      instructions: "Help the user"

   after_reasoning:
      set @variables.turn_count = @variables.turn_count + 1
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    assert!(serialized.contains("before_reasoning:"), "Missing before_reasoning block");
    assert!(serialized.contains("after_reasoning:"), "Missing after_reasoning block");

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    assert_eq!(reparsed.topics.len(), 1);
    let topic = &reparsed.topics[0].node;
    assert!(topic.before_reasoning.is_some(), "before_reasoning lost after roundtrip");
    assert!(topic.after_reasoning.is_some(), "after_reasoning lost after roundtrip");
}
