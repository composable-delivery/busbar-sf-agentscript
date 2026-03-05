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

#[test]
fn test_roundtrip_system_block_with_messages() {
    // Covers the full `system:` block — both `instructions:` and `messages:`
    // sub-blocks.  No roundtrip test existed for the system block at all.
    let original = r#"config:
   agent_name: "SystemAgent"

system:
   instructions: "You are a helpful assistant."
   messages:
      welcome: "Hello! How can I help you today?"
      error: "Something went wrong. Please try again."

topic main:
   description: "Main"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    assert!(serialized.contains("system:"), "Missing system block header");
    assert!(serialized.contains("messages:"), "Missing messages sub-block");
    assert!(serialized.contains("welcome:"), "Missing welcome entry");

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    let system = reparsed.system.as_ref().expect("system block lost after roundtrip");

    let messages = system.node.messages.as_ref().expect("messages lost after roundtrip");
    assert!(messages.node.welcome.is_some(), "welcome message lost after roundtrip");
    assert_eq!(
        messages.node.welcome.as_ref().unwrap().node,
        "Hello! How can I help you today?"
    );
    assert!(messages.node.error.is_some(), "error message lost after roundtrip");
    assert_eq!(
        messages.node.error.as_ref().unwrap().node,
        "Something went wrong. Please try again."
    );
    assert!(system.node.instructions.is_some(), "instructions lost after roundtrip");
}

#[test]
fn test_roundtrip_multiple_topics() {
    // Tests that two topics are both preserved in the correct order through a
    // parse → serialize → parse cycle.  All existing roundtrip tests have at most 1 topic.
    let original = r#"config:
   agent_name: "MultiTopicAgent"

topic alpha:
   description: "First topic"
   reasoning:
      instructions: "Handle alpha requests"

topic beta:
   description: "Second topic"
   reasoning:
      instructions: "Handle beta requests"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);
    let reparsed = parse(&serialized).expect("Failed to reparse serialized");

    assert_eq!(reparsed.topics.len(), 2, "Expected 2 topics after roundtrip");
    assert_eq!(reparsed.topics[0].node.name.node, "alpha", "First topic name changed");
    assert_eq!(reparsed.topics[1].node.name.node, "beta", "Second topic name changed");
    assert_eq!(
        reparsed.topics[0].node.description.as_ref().unwrap().node,
        "First topic"
    );
    assert_eq!(
        reparsed.topics[1].node.description.as_ref().unwrap().node,
        "Second topic"
    );
}

#[test]
fn test_roundtrip_variable_non_string_types() {
    // Tests that boolean, list[string], and object variable types survive a
    // parse → serialize → parse cycle.  The existing variable roundtrip test
    // only covers `integer` and `string` types.
    let original = r#"config:
   agent_name: "TypedVarsAgent"

variables:
   enabled: mutable boolean = False
      description: "Feature flag"
   tags: mutable list[string] = []
      description: "Collection of tags"
   extra_data: mutable object = None
      description: "Supplementary data"

topic main:
   description: "Main"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    assert!(
        serialized.contains("list[string]"),
        "list[string] type lost during serialization"
    );
    assert!(serialized.contains("boolean"), "boolean type lost during serialization");
    assert!(serialized.contains("object"), "object type lost during serialization");

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    let vars = &reparsed.variables.as_ref().expect("variables block lost").node;
    assert_eq!(vars.variables.len(), 3, "Expected 3 variables after roundtrip");

    let enabled = vars.variables.iter().find(|v| v.node.name.node == "enabled");
    assert!(enabled.is_some(), "enabled variable lost after roundtrip");
    assert!(
        matches!(enabled.unwrap().node.ty.node, busbar_sf_agentscript::Type::Boolean),
        "enabled should have type boolean"
    );

    let tags = vars.variables.iter().find(|v| v.node.name.node == "tags");
    assert!(tags.is_some(), "tags variable lost after roundtrip");
    assert!(
        matches!(
            tags.unwrap().node.ty.node,
            busbar_sf_agentscript::Type::List(_)
        ),
        "tags should have type list[string]"
    );
}

#[test]
fn test_roundtrip_config_optional_fields() {
    // Tests that optional config fields — agent_label, description, agent_type —
    // survive a parse → serialize → parse cycle.  The existing config roundtrip
    // tests only exercise agent_name.
    let original = r#"config:
   agent_name: "FullConfigAgent"
   agent_label: "Full Config Agent"
   description: "An agent with all optional config fields populated"
   agent_type: "customer_service"

topic main:
   description: "Main"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    assert!(
        serialized.contains("agent_label:"),
        "agent_label lost during serialization"
    );
    assert!(
        serialized.contains("Full Config Agent"),
        "agent_label value lost during serialization"
    );

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    let config = reparsed.config.as_ref().expect("config block lost after roundtrip");
    assert_eq!(config.node.agent_name.node, "FullConfigAgent");
    assert_eq!(
        config.node.agent_label.as_ref().unwrap().node,
        "Full Config Agent"
    );
    assert_eq!(
        config.node.description.as_ref().unwrap().node,
        "An agent with all optional config fields populated"
    );
    assert_eq!(
        config.node.agent_type.as_ref().unwrap().node,
        "customer_service"
    );
}
