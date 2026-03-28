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
    // Covers the `system:` block containing both `messages:` (welcome + error) and
    // `instructions:`. No prior roundtrip test exercised the messages sub-block.
    let original = r#"config:
   agent_name: "MsgAgent"

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

    assert!(serialized.contains("system:"), "Missing system block in serialized output");
    assert!(serialized.contains("messages:"), "Missing messages sub-block");
    assert!(serialized.contains("welcome:"), "Missing welcome entry");
    assert!(serialized.contains("error:"), "Missing error entry");

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    let sys = reparsed.system.as_ref().expect("system block lost after roundtrip");
    assert!(sys.node.instructions.is_some(), "system instructions lost after roundtrip");
    let msgs = sys.node.messages.as_ref().expect("messages block lost after roundtrip");
    assert!(msgs.node.welcome.is_some(), "welcome message lost after roundtrip");
    assert!(msgs.node.error.is_some(), "error message lost after roundtrip");
}

#[test]
fn test_roundtrip_linked_variable() {
    // Covers a `linked` variable declaration with a `source:` reference binding.
    // Previous roundtrip tests only covered `mutable` variables.
    use busbar_sf_agentscript::ast::VariableKind;

    let original = r#"config:
   agent_name: "LinkedAgent"

variables:
   user_email: linked string
      description: "Email of the logged-in user"
      source: @messagingSession.userEmail

topic main:
   description: "Main"
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    assert!(serialized.contains("linked"), "Missing 'linked' keyword in serialized output");

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    let vars = reparsed.variables.as_ref().expect("variables block lost after roundtrip");
    assert_eq!(vars.node.variables.len(), 1, "Expected one variable after roundtrip");
    let var = &vars.node.variables[0].node;
    assert_eq!(var.name.node, "user_email");
    assert!(
        matches!(var.kind, VariableKind::Linked),
        "Variable kind changed from Linked after roundtrip"
    );
    assert!(var.source.is_some(), "source reference lost after roundtrip");
}

#[test]
fn test_roundtrip_topic_with_system_override() {
    // Covers a `topic` block that contains a `system:` override block. The override
    // sets topic-scoped instructions that differ from the global system block.
    // No prior roundtrip test exercised the per-topic system override.
    let original = r#"config:
   agent_name: "SysOverrideAgent"

topic support:
   description: "Support topic"

   system:
      instructions: "You are now in support mode. Focus on resolving customer issues quickly."

   reasoning:
      instructions: "Help the user resolve their issue."
"#;

    let ast = parse(original).expect("Failed to parse original");
    let serialized = serialize(&ast);

    assert!(
        serialized.contains("system:"),
        "Missing system override in serialized output"
    );
    assert!(
        serialized.contains("support mode"),
        "Missing system override instruction text"
    );

    let reparsed = parse(&serialized).expect("Failed to reparse serialized");
    assert_eq!(reparsed.topics.len(), 1);
    let topic = &reparsed.topics[0].node;
    assert!(topic.system.is_some(), "topic system override lost after roundtrip");
    assert!(
        topic.system.as_ref().unwrap().node.instructions.is_some(),
        "system override instructions lost after roundtrip"
    );
}
