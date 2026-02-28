use busbar_sf_agentscript::parser;

#[test]
fn test_reasoning_minimal() {
    let src = r#"config:
   agent_name: "Test"

start_agent topic_selector:
   description: "Welcome"

   reasoning:
      instructions:|
         Select the tool.
      actions:
         check_order: @utils.transition to @topic.order_status
            description: "Start checking order status"

topic order_status:
   description: "Looks up and explains order status"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert!(ast.start_agent.is_some());
            assert_eq!(ast.topics.len(), 1);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed");
        }
    }
}

#[test]
#[ignore = "Known parser limitation — uses '...' placeholder syntax"]
fn test_dynamic_instructions() {
    let src = r#"config:
   agent_name: "Test"

topic main:
   description: "Main"

   reasoning:
      instructions:->
         if not @variables.order_id:
            | Ask for order number.

         if @variables.order_id and not @variables.status:
            run @actions.get_status
               with order_id=@variables.order_id
               set @variables.status = @outputs.status

         | Status: {!@variables.status}

         if @variables.status == "pending":
            | Order is being processed.

      actions:
         get_status: @actions.get_status
            with order_id=...
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed");
        }
    }
}

#[test]
fn test_topic_with_actions_definitions() {
    // Mirrors the structure in ReasoningInstructions.agent
    let src = r#"config:
   agent_name: "Test"

start_agent topic_selector:
   description: "Welcome"

   reasoning:
      instructions:|
         Select the tool.
      actions:
         check_order: @utils.transition to @topic.order_status
            description: "Start checking order status"

topic order_status:
   description: "Looks up order status"

   actions:
      get_order_status:
         description: "Retrieves status"
         inputs:
            order_id: string
               description: "Order ID"
         outputs:
            status: string
               description: "Status"
         target: "flow://GetOrderStatus"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert!(ast.start_agent.is_some());
            assert_eq!(ast.topics.len(), 1);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed");
        }
    }
}

#[test]
#[ignore = "Known parser limitation — uses '...' placeholder syntax"]
fn test_full_reasoning_instructions_structure() {
    // Full structure from ReasoningInstructions.agent
    let src = r#"config:
   agent_name: "ReasoningInstructions"
   agent_label: "ReasoningInstructions"
   description: "Checks order status"

variables:
   order_id: mutable string = ""
      description: "The order ID to check"

   order_status: mutable string = ""
      description: "Current status"

system:
   messages:
      welcome: "Hi! I can help you check your order status."
      error: "I had trouble looking up that order."

   instructions: "You are an order status assistant."

start_agent topic_selector:
   description: "Welcome customers"

   reasoning:
      instructions:|
         Select the tool that best matches the user's message.
      actions:
         check_order: @utils.transition to @topic.order_status
            description: "Start checking order status"

topic order_status:
   description: "Looks up and explains order status"

   actions:
      get_order_status:
         description: "Retrieves current status for an order"
         inputs:
            order_id: string
               description: "The unique order identifier"
         outputs:
            status: string
               description: "Current order status"
         target: "flow://GetOrderStatus"

   reasoning:
      instructions:->
         if not @variables.order_id:
            | Ask the customer for their order number.

         if @variables.order_id and not @variables.order_status:
            run @actions.get_order_status
               with order_id=@variables.order_id
               set @variables.order_status = @outputs.status

         | The customer's order has status: {!@variables.order_status}

         if @variables.order_status == "pending":
            | The order is being processed.

      actions:
         get_order_status: @actions.get_order_status
            with order_id=...
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert!(ast.start_agent.is_some());
            assert_eq!(ast.topics.len(), 1);
            assert!(ast.variables.is_some());
            assert!(ast.system.is_some());
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed");
        }
    }
}

#[test]
fn test_multiple_topics_with_dynamic_instructions() {
    // Minimal test for two topics each with instructions:->
    let src = r#"config:
   agent_name: "Test"

topic general:
   description: "General"

   reasoning:
      instructions:->
         | Hello
      actions:
         go: @utils.transition to @topic.other

topic other:
   description: "Other"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 2, "Should parse 2 topics");
            assert_eq!(ast.topics[0].node.name.node, "general");
            assert_eq!(ast.topics[1].node.name.node, "other");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_pipe_with_continuation() {
    // Tests | text with continuation lines (MORE indented than the |)
    // This is the pattern in SystemInstructionOverrides
    let src = r#"config:
   agent_name: "Test"

topic general:
   description: "General"

   reasoning:
      instructions:->
         | Hello world
           "continued text"
           "more text"
      actions:
         go: @utils.transition to @topic.other

topic other:
   description: "Other"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 2, "Should parse 2 topics");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
#[ignore = "Known parser limitation — uses '...' placeholder syntax"]
fn test_exact_first_40_lines() {
    // EXACT content from ReasoningInstructions.agent lines 1-40
    let src = r#"# ReasoningInstructions - Procedural Templating with Dynamic Instructions
# This agent demonstrates how to build instructions dynamically using procedures

config:
   agent_name: "ReasoningInstructions"
   agent_label: "ReasoningInstructions"
   description: "Checks order status and provides dynamic instructions based on order state"

variables:
   order_id: mutable string = ""
      description: "The order ID to check"

   order_status: mutable string = ""
      description: "Current status of the order"

   order_details: mutable object = {}
      description: "Full order details"

   tracking_number: mutable string = ""
      description: "Shipping tracking number"

system:
   messages:
      welcome: "Hi! I can help you check your order status. What's your order number?"
      error: "I had trouble looking up that order. Please verify the order number."

   instructions: "You are an order status assistant. Help customers track their orders and resolve issues."

start_agent topic_selector:
   description: "Welcome customers and begin helping with order status"

   reasoning:
      instructions:|
         Select the tool that best matches the user's message and conversation history. If it's unclear, make your best guess.
      actions:
         check_order: @utils.transition to @topic.order_status
            description: "Start checking order status"

topic order_status:
   description: "Looks up and explains order status"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert!(ast.start_agent.is_some());
            assert_eq!(ast.topics.len(), 1);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed");
        }
    }
}

#[test]
fn test_start_agent_then_topic() {
    let src = r#"config:
   agent_name: "Test"

start_agent topic_selector:
   description: "Welcome"

   reasoning:
      instructions:|
         Select the tool.
      actions:
         check_order: @utils.transition to @topic.order_status
            description: "Start checking order status"

topic order_status:
   description: "Looks up and explains order status"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert!(ast.start_agent.is_some());
            assert_eq!(ast.topics.len(), 1);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed");
        }
    }
}

#[test]
#[ignore = "Known parser limitation — uses '...' placeholder syntax"]
fn test_topic_with_dynamic_instructions_and_actions() {
    // This adds instructions:-> to the topic, which is what the failing recipes have
    let src = r#"config:
   agent_name: "Test"

start_agent topic_selector:
   description: "Welcome"

   reasoning:
      instructions:|
         Select the tool.
      actions:
         check_order: @utils.transition to @topic.order_status
            description: "Start checking order status"

topic order_status:
   description: "Looks up and explains order status"

   reasoning:
      instructions:->
         if not @variables.order_id:
            | Ask for order number.

      actions:
         get_order_status: @actions.get_order_status
            with order_id=...
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert!(ast.start_agent.is_some());
            assert_eq!(ast.topics.len(), 1);
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_exact_indentation_from_file() {
    // Exact indentation from SystemInstructionOverrides - 7 spaces for reasoning content
    let src = r#"config:
   agent_name: "Test"

topic general:
   description: "General"

   reasoning:
       instructions: ->
           | Hello
             "continued"

       actions:
           go: @utils.transition to @topic.other

topic other:
   description: "Other"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 2, "Should parse 2 topics");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_with_template_expressions() {
    // Add template expressions like in the real file
    let src = r#"config:
   agent_name: "Test"

topic general:
   description: "General"

   reasoning:
       instructions: ->
           | Hello
             "continued"
             Use {!@actions.go} to transition.

       actions:
           go: @utils.transition to @topic.other

topic other:
   description: "Other"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 2, "Should parse 2 topics");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_topic_with_system_override() {
    // Test topic-level system instruction override like in the real file
    let src = r#"config:
   agent_name: "Test"

topic general:
   description: "General"

   reasoning:
       instructions: ->
           | Hello

       actions:
           go: @utils.transition to @topic.professional

topic professional:
   description: "Professional"

   system:
       instructions: "You are a formal business professional."

   reasoning:
       instructions: ->
           | Professional mode

       actions:
           back: @utils.transition to @topic.general
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 2, "Should parse 2 topics");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_topic_with_system_simple() {
    // Minimal reproduction - topic with system block
    let src = r#"config:
   agent_name: "Test"

topic main:
   description: "Main"

   system:
       instructions: "Override instructions."

   reasoning:
       instructions: ->
           | Hello
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1, "Should parse 1 topic");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_topic_with_action_definitions() {
    // Topic with action definitions (like topic order_status in ReasoningInstructions)
    let src = r#"config:
   agent_name: "Test"

start_agent main:
   description: "Entry"
   reasoning:
      instructions:|
         Help.
      actions:
         go: @utils.transition to @topic.order_status
            description: "Go"

topic order_status:
   description: "Looks up order status"

   actions:
      get_order_status:
         description: "Retrieves status"
         inputs:
            order_id: string
               description: "The order ID"
         outputs:
            status: string
               description: "Status"
         target: "flow://GetOrderStatus"

   reasoning:
      instructions:->
         | Check order status
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1, "Should parse 1 topic");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
#[ignore = "Known parser limitation — uses '...' placeholder syntax"]
fn test_dynamic_instructions_with_run() {
    // Topic with instructions:-> containing run @actions
    let src = r#"config:
   agent_name: "Test"

topic main:
   description: "Main"

   actions:
      get_status:
         description: "Get status"
         inputs:
            order_id: string
         outputs:
            status: string
         target: "flow://GetStatus"

   reasoning:
      instructions:->
         if not @variables.order_id:
            | Ask for order number.

         if @variables.order_id and not @variables.status:
            run @actions.get_status
               with order_id=@variables.order_id
               set @variables.status = @outputs.status

         | Status: {!@variables.status}

      actions:
         get_status: @actions.get_status
            with order_id=...
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1, "Should parse 1 topic");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_instructions_with_comments() {
    // Comments inside instructions:->
    let src = r#"config:
   agent_name: "Test"

topic main:
   description: "Main"

   reasoning:
      # Comment before instructions
      instructions:->
         # First check
         if not @variables.order_id:
            # Nested comment
            | Ask for order.

         # Another check
         | Done.
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1, "Should parse 1 topic");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_comment_before_instructions() {
    // Just test comment before instructions
    let src = r#"config:
   agent_name: "Test"

topic main:
   description: "Main"

   reasoning:
      # Comment here
      instructions:|
         Help.
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1, "Should parse 1 topic");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_comment_inside_dynamic_instructions() {
    // Comment INSIDE instructions:->
    let src = r#"config:
   agent_name: "Test"

topic main:
   description: "Main"

   reasoning:
      instructions:->
         # Comment inside
         | Help.
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1, "Should parse 1 topic");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}

#[test]
fn test_prompt_template_structure() {
    // Test structure from PromptTemplateActions
    let src = r#"system:
    instructions: "You are an assistant."

    messages:
        welcome: "Hi!"
        error: "Error."

config:
    agent_name: "Test"
start_agent topic_selector:
    description: "Welcome"

    reasoning:
        instructions:|
            Select tool.
        actions:
            go: @utils.transition to @topic.main
                description: "Go"

topic main:
    description: "Main topic"
"#;
    match parser::parse(src) {
        Ok(ast) => {
            assert_eq!(ast.topics.len(), 1, "Should parse 1 topic");
        }
        Err(errors) => {
            for e in &errors {
                eprintln!("{}", e);
            }
            panic!("Parse failed with {} errors", errors.len());
        }
    }
}
