use crate::ast::AgentFile;
use crate::lexer;

// Re-export the span type
pub use super::primitives::Span;

// Re-export primitives needed by agent_file_parser
use super::primitives::{skip_toplevel_noise, ParserInput};

use chumsky::prelude::*;

use super::config::config_block;
use super::language::language_block;
use super::system::system_block;
use super::topics::{start_agent_block, topic_block};
use super::variables::variables_block;

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::expressions::{expr, reference};
    use super::*;

    #[test]
    fn test_parse_minimal_config() {
        let source = r#"config:
   agent_name: "Test"
"#;
        let result = parse(source);
        assert!(result.is_ok());
        let file = result.unwrap();
        assert!(file.config.is_some());
    }

    #[test]
    fn test_parse_expression() {
        let source = "config:\n   agent_name: \"Test\"";
        let tokens = lexer::lexer().parse(source).unwrap();
        let eoi_span = primitives::Span::new(source.len(), source.len());
        let _token_stream = tokens.as_slice().spanned(eoi_span);

        // Test expression parsing directly
        let expr_tokens = vec![
            (Token::NumberLit(5.0), primitives::Span::new(0, 1)),
            (Token::Plus, primitives::Span::new(2, 3)),
            (Token::NumberLit(3.0), primitives::Span::new(4, 5)),
        ];
        let expr_stream = expr_tokens.as_slice().spanned(primitives::Span::new(5, 5));
        let result = expr().parse(expr_stream);
        assert!(result.has_output());
    }

    #[test]
    fn test_parse_reference() {
        let tokens = vec![
            (Token::At, primitives::Span::new(0, 1)),
            (Token::Variables, primitives::Span::new(1, 10)),
            (Token::Dot, primitives::Span::new(10, 11)),
            (Token::Ident("user_id"), primitives::Span::new(11, 18)),
        ];
        let token_stream = tokens.as_slice().spanned(primitives::Span::new(18, 18));
        let result = reference().parse(token_stream);
        assert!(result.has_output());
        let r = result.output().unwrap();
        assert_eq!(r.namespace, "variables");
    }
}

#[test]
fn test_parse_start_agent_only() {
    let source = r#"config:
    agent_name: "Test"

start_agent topic_selector:
    description: "Welcome new customers"

    reasoning:
        instructions:|
            Select the tool.
        actions:
            begin_onboarding: @utils.transition to @topic.onboarding
                description: "Start onboarding"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok());
    let file = result.unwrap();
    assert!(file.start_agent.is_some());
}

#[test]
fn test_parse_start_agent_and_topic() {
    let source = r#"config:
    agent_name: "Test"

start_agent topic_selector:
    description: "Welcome new customers"

    reasoning:
        instructions:|
            Select the tool.
        actions:
            begin_onboarding: @utils.transition to @topic.onboarding
                description: "Start onboarding"

topic onboarding:
    label: "onboarding"
    description: "Handles onboarding"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok());
    let file = result.unwrap();
    assert!(file.start_agent.is_some());
    assert!(!file.topics.is_empty());
}

#[test]
fn test_parse_topic_with_reasoning() {
    let source = r#"config:
    agent_name: "Test"

topic onboarding:
    label: "onboarding"
    description: "Handles onboarding"

    reasoning:
        instructions: ->
            | Onboard the customer step by step following the below ## Rules.
            | ## Rules:
        actions:
            create_account: @actions.create_account
                available when @variables.step == 1
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok());
}

#[test]
fn test_parse_dynamic_instructions_with_if() {
    let source = r#"config:
    agent_name: "Test"

topic onboarding:
    description: "Handles onboarding"

    reasoning:
        instructions: ->
            | Onboard the customer step by step.
            | ## Rules:
                 a. This is rule a.
                 b. This is rule b.
            if @variables.step == 1:
                | Ask for name and email.
            if @variables.step == 2 and @variables.account_created == True:
                | Collect preferences.
        actions:
            create_account: @actions.create_account
                available when @variables.step == 1
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok());
}

#[test]
fn test_parse_complex_reasoning_actions() {
    let source = r#"config:
    agent_name: "Test"

topic onboarding:
    description: "Handles onboarding"

    reasoning:
        instructions: ->
            | Onboard the customer.
        actions:
            create_account: @actions.create_account
                available when @variables.step == 1
                with email = ...
                with name = ...
                set @variables.customer_id = @outputs.customer_id
                set @variables.account_created = @outputs.success
                run @actions.send_verification
                    with customer_id = @variables.customer_id
                    with email = @variables.customer_email
                    set @variables.verification_token = @outputs.token
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok());
}

#[test]
fn test_parse_topic_with_actions_block() {
    let source = r#"config:
    agent_name: "Test"

topic onboarding:
    description: "Handles onboarding"

    reasoning:
        instructions: ->
            | Onboard the customer.
        actions:
            create_account: @actions.create_account
                available when @variables.step == 1

    actions:
        create_account:
            description: "Creates a new customer account"
            inputs:
                email: string
                    description: "Customer's email address"
                name: string
                    description: "Customer's full name"
            outputs:
                customer_id: string
                    description: "Unique customer ID"
                success: boolean
                    description: "Success flag"
            target: "flow://CreateCustomerAccount"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok());
}

#[test]
fn test_parse_multistep_partial() {
    let source = r#"# Multi-step workflow with actions and instructions and rules to guide the workflow
system:
    instructions: "You are a helpful onboarding assistant. Guide users through account creation step by step."

    messages:
        welcome: "Welcome! Let's get you onboarded step by step to our platform. I'll guide you through the process. Please provide your email and name to begin the onboarding process."
        error: "Something went wrong during onboarding. Let me help you recover."

config:
    agent_name: "MultiStepWorkflows"
    agent_label: "MultiStepWorkflows"
    description: "Handles customer onboarding through a multi-step workflow"

variables:
    customer_email: mutable string = ""
        description: "Customer's email address"
    customer_id: mutable string = ""
        description: "Generated customer ID from account creation"

start_agent topic_selector:
    description: "Welcome new customers and begin onboarding workflow"

    reasoning:
        instructions:|
            Select the tool that best matches the user's message and conversation history. If it's unclear, make your best guess.
        actions:
            begin_onboarding: @utils.transition to @topic.onboarding
                description: "Start the multi-step customer onboarding workflow"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok());
    let file = result.unwrap();
    assert!(file.start_agent.is_some());
}

#[test]
fn test_parse_multistep_with_topic() {
    let source = r#"# Multi-step workflow
system:
    instructions: "You are a helpful assistant."

config:
    agent_name: "MultiStepWorkflows"

start_agent topic_selector:
    description: "Welcome new customers"

    reasoning:
        instructions:|
            Select the tool that best matches.
        actions:
            begin_onboarding: @utils.transition to @topic.onboarding
                description: "Start onboarding"

topic onboarding:
    label: "onboarding"
    description: "Handles onboarding"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok(), "Parse failed");
    let file = result.unwrap();
    assert!(file.start_agent.is_some());
    assert!(!file.topics.is_empty());
}

#[test]
fn test_parse_multistep_with_reasoning() {
    let source = r#"# Multi-step workflow
system:
    instructions: "You are a helpful assistant."

config:
    agent_name: "MultiStepWorkflows"

start_agent topic_selector:
    description: "Welcome new customers"

    reasoning:
        instructions:|
            Select the tool that best matches.
        actions:
            begin_onboarding: @utils.transition to @topic.onboarding
                description: "Start onboarding"

topic onboarding:
    label: "onboarding"
    description: "Handles onboarding"

    reasoning:
        instructions: ->
            | Onboard the customer step by step following the below ## Rules. You are currently on {!variables.step} of Step 4.
            | ## Rules:
                 a. Onboarding is a strictly 4 step process.
                 b. NEVER ask the customer to set password.
            if @variables.step == 1:
                | Ask for name and email to create account.
            if @variables.step == 2 and @variables.account_created == True:
                | Collect preferences.
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok(), "Parse failed");
}

#[test]
fn test_parse_multistep_with_actions() {
    let source = r#"config:
    agent_name: "MultiStepWorkflows"

topic onboarding:
    description: "Handles onboarding"

    reasoning:
        instructions: ->
            | Onboard the customer.
        actions:
            create_account: @actions.create_account
                available when @variables.step == 1
                with email = ...
                with name = ...
                set @variables.customer_id = @outputs.customer_id
                set @variables.account_created = @outputs.success
                set @variables.customer_email = @outputs.customer_email
                set @variables.step = 2
                run @actions.send_verification
                    with customer_id = @variables.customer_id
                    with email = @variables.customer_email
                    set @variables.verification_token = @outputs.token

            setup_profile: @actions.setup_profile
                available when @variables.step == 2
                with customer_id = @variables.customer_id
                with preferences = ...
                set @variables.profile_completed = @outputs.success
                set @variables.step = 3

            configure_settings: @actions.configure_settings
                available when @variables.step == 3
                with customer_id = @variables.customer_id
                with settings = ...
                set @variables.preferences_set = @outputs.success
                set @variables.step = 4

            finalize_onboarding: @actions.finalize_onboarding
                available when @variables.step == 4
                with customer_id = @variables.customer_id
                set @variables.onboarding_complete = @outputs.success
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok(), "Parse failed");
}

#[test]
fn test_parse_set_variable_number() {
    let source = r#"config:
    agent_name: "Test"

topic onboarding:
    description: "Handles onboarding"

    reasoning:
        instructions: ->
            | Onboard the customer.
        actions:
            create_account: @actions.create_account
                available when @variables.step == 1
                set @variables.step = 2
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok(), "Parse failed");
}

#[test]
fn test_parse_two_topics_with_nested_if_in_after_reasoning() {
    // This test reproduces a bug where the parser fails on the second topic
    // when the first topic has a nested `if` inside `after_reasoning`
    let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Selector"
   reasoning:
      instructions: "Route"

topic first:
   description: "First topic"

   reasoning:
      instructions: "Handle"
      actions:
         do_action: @actions.something
            description: "Do something"

   after_reasoning:
      if @variables.count > 3:
         set @variables.flag = True

topic second:
   description: "Second topic"
   reasoning:
      instructions: "Handle second"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok(), "Parse failed - this is the bug we're fixing");
    let file = result.unwrap();
    assert_eq!(file.topics.len(), 2, "Should have 2 topics");
}

#[test]
fn test_parse_chained_run_clauses_in_reasoning_action() {
    // This test reproduces the exact bug from ComprehensiveDemo.agent
    // where chained run clauses inside reasoning actions cause the parser
    // to fail on subsequent topics
    let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Selector"
   reasoning:
      instructions: "Route"

topic first:
   description: "First topic"

   actions:
      do_something:
         description: "An action"
         target: "flow://DoSomething"

   reasoning:
      instructions: "Handle"
      actions:
         complex_action: @actions.do_something
            description: "Complex action with chained runs"
            with param=@variables.foo
            set @variables.result = @outputs.data
            run @actions.do_something
               with param=@variables.bar
            run @actions.do_something
               with param=@variables.baz

   after_reasoning:
      if @variables.count > 3:
         set @variables.flag = True

topic second:
   description: "Second topic"
   reasoning:
      instructions: "Handle second"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(
        result.is_ok(),
        "Parse failed on chained run clauses - this is the bug we're fixing"
    );
    let file = result.unwrap();
    assert_eq!(file.topics.len(), 2, "Should have 2 topics");
}

#[test]
fn test_parse_before_reasoning_with_run_and_set() {
    // Test before_reasoning with both run and set clauses like in ComprehensiveDemo
    let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Selector"
   reasoning:
      instructions: "Route"

topic first:
   description: "First topic"

   actions:
      lookup_policy:
         description: "Look up policy"
         inputs:
            policy_number: string
               description: "Policy number"
         outputs:
            policy: object
               description: "Policy data"
            premium: number
               description: "Premium amount"
         target: "flow://LookupPolicy"

   before_reasoning:
      set @variables.turn_count = @variables.turn_count + 1

      if @variables.policy_number != "":
         run @actions.lookup_policy
            with policy_number=@variables.policy_number
            with include_history=True
            set @variables.current_policy = @outputs.policy
            set @variables.premium_amount = @outputs.premium

   reasoning:
      instructions: "Handle"

   after_reasoning:
      if @variables.retry_count > 3:
         set @variables.fraud_flag = True

topic second:
   description: "Second topic"
   reasoning:
      instructions: "Handle second"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(
        result.is_ok(),
        "Parse failed on before_reasoning with run/set - this is the bug"
    );
    let file = result.unwrap();
    assert_eq!(file.topics.len(), 2, "Should have 2 topics");
}

#[test]
fn test_parse_comprehensive_demo_structure() {
    // Reproduce the EXACT structure from ComprehensiveDemo.agent policy_management topic
    let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Selector"
   reasoning:
      instructions: "Route"

topic policy_management:
   description: "Handles policy"

   system:
      instructions:|
         You are in Policy Management context.
         Focus on helping customers.

   actions:
      lookup_policy:
         description: "Look up policy"
         inputs:
            policy_number: string
               description: "The policy number"
               is_required: True
         outputs:
            policy: object
               description: "Policy details"
            premium: number
               description: "Premium amount"
         target: "flow://LookupPolicy"

      update_coverage:
         description: "Update coverage"
         inputs:
            policy_record_id: id
               description: "Policy ID"
               is_required: True
         outputs:
            updated_policy: object
               description: "Updated policy"
         target: "flow://UpdateCoverage"

   before_reasoning:
      set @variables.turn_count = @variables.turn_count + 1

      if @variables.policy_number != "":
         run @actions.lookup_policy
            with policy_number=@variables.policy_number
            set @variables.current_policy = @outputs.policy
            set @variables.premium_amount = @outputs.premium

   reasoning:
      instructions:->
         | Policy Management Dashboard

         if @variables.policy_number == "":
            | Please provide your policy number.

         if @variables.policy_number != "":
            | Policy: {!@variables.policy_number}

            if @variables.is_policy_active:
               | Status: Active
            else:
               | Status: Requires attention

      actions:
         set_policy_info: @utils.setVariables
            description: "Store policy number"
            with policy_number=...

         view_policy: @actions.lookup_policy
            description: "Look up policy"
            with policy_number=@variables.policy_number
            set @variables.current_policy = @outputs.policy
            set @variables.premium_amount = @outputs.premium

         update_my_coverage: @actions.update_coverage
            description: "Modify coverage"
            available when @variables.is_policy_active == True and @variables.policy_record_id != None
            with policy_record_id=@variables.policy_record_id
            set @variables.current_policy = @outputs.updated_policy
            run @actions.lookup_policy
               with policy_number=@variables.policy_number
            run @actions.lookup_policy
               with policy_number=@variables.policy_number

         file_a_claim: @utils.transition to @topic.claims_processing
            description: "File a claim"
            available when @variables.is_policy_active == True

         speak_to_agent: @utils.escalate
            description: "Speak to agent"
            available when @variables.retry_count > 2 or @variables.fraud_flag == True

   after_reasoning:
      if @variables.retry_count > 3:
         set @variables.fraud_flag = True

topic claims_processing:
   description: "Claims topic"
   reasoning:
      instructions: "Handle claims"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok(), "Parse failed on comprehensive structure - THIS IS THE BUG");
    let file = result.unwrap();
    assert_eq!(file.topics.len(), 2, "Should have 2 topics");
}

#[test]
fn test_parse_full_file_with_all_blocks() {
    // Include all the blocks from ComprehensiveDemo to find what triggers failure
    let source = r#"config:
   agent_name: "ApexInsuranceAgent"
   agent_label: "Apex Insurance Service Agent"
   description: "A comprehensive agent"
   agent_type: "AgentforceServiceAgent"
   default_agent_user: "agent@test.com"

language:
   default_locale: "en_US"
   additional_locales: "es_MX,fr_CA"

connections:
   messaging:
      escalation_message: "Connecting you with a specialist."
      outbound_route_type: "OmniChannelFlow"
      outbound_route_name: "SpecialistQueue"

variables:
   customer_name: mutable string = ""
      description: "Full name of the customer"
   policy_number: mutable string = ""
      description: "Policy number"
   premium_amount: mutable number = 0.0
      description: "Premium amount"
   is_policy_active: mutable boolean = False
      description: "Whether policy is active"
   current_policy: mutable object = {}
      description: "Current policy data"
   retry_count: mutable integer = 0
      description: "Retry count"
   turn_count: mutable integer = 0
      description: "Turn count"
   policy_record_id: mutable id = None
      description: "Policy record ID"
   fraud_flag: mutable boolean = False
      description: "Fraud flag"
   logged_in_user_email: linked string
      description: "Email of logged-in user"
      source: @messagingSession.userEmail

system:
   messages:
      welcome: "Welcome!"
      error: "Error occurred."
   instructions: "You are an AI agent."

start_agent topic_selector:
   description: "Main entry point"

   system:
      instructions:|
         As the topic selector, understand what customer needs.

   actions:
      log_session_start:
         description: "Logs session start"
         inputs:
            user_email: string
               description: "Email"
         outputs:
            session_logged: boolean
               description: "Whether logged"
         target: "flow://LogSessionStart"

   before_reasoning:
      set @variables.turn_count = @variables.turn_count + 1
      if @variables.turn_count == 1:
         run @actions.log_session_start
            with user_email=@variables.logged_in_user_email

   reasoning:
      instructions:->
         | Analyze and route.

         if @variables.logged_in_user_email != "":
            | Welcome back!
         else:
            | No account found.

         | Route to appropriate topic.

      actions:
         go_to_policy: @utils.transition to @topic.policy_management
            description: "Route to policy"
         go_to_claims: @utils.transition to @topic.claims_processing
            description: "Route to claims"

   after_reasoning:
      set @variables.turn_count = @variables.turn_count + 1

topic policy_management:
   description: "Handles policy-related inquiries"

   system:
      instructions:|
         You are in Policy Management.
         Focus on helping customers.

   actions:
      lookup_policy:
         description: "Look up policy"
         inputs:
            policy_number: string
               description: "Policy number"
               is_required: True
            include_history: boolean
               description: "Include history"
               is_required: False
         outputs:
            policy: object
               description: "Policy details"
               is_displayable: True
               complex_data_type_name: "InsurancePolicyType"
            status: string
               description: "Policy status"
            premium: number
               description: "Premium amount"
            coverage_details: list[object]
               description: "Coverage items"
         target: "flow://LookupPolicy"

      update_coverage:
         description: "Update coverage"
         inputs:
            policy_record_id: id
               description: "Policy ID"
               is_required: True
            coverage_type: string
               description: "Coverage type"
               is_required: True
            new_limit: number
               description: "New limit"
               is_required: True
            effective_date: date
               description: "Effective date"
               is_required: True
         outputs:
            updated_policy: object
               description: "Updated policy"
            new_premium: number
               description: "New premium"
            confirmation_number: string
               description: "Confirmation"
         target: "flow://UpdatePolicyCoverage"

      calculate_premium:
         description: "Calculate premium"
         inputs:
            policy_record_id: id
               description: "Policy ID"
            proposed_changes: object
               description: "Changes"
         outputs:
            current_premium: number
               description: "Current premium"
            new_premium: number
               description: "New premium"
            difference: number
               description: "Difference"
         target: "flow://CalculatePremium"

      send_policy_documents:
         description: "Send documents"
         inputs:
            policy_record_id: id
               description: "Policy ID"
            document_types: list[string]
               description: "Document types"
            email: string
               description: "Email"
         outputs:
            sent: boolean
               description: "Whether sent"
            document_urls: list[string]
               description: "URLs"
         target: "flow://SendPolicyDocuments"

   before_reasoning:
      set @variables.turn_count = @variables.turn_count + 1

      if @variables.policy_number != "":
         run @actions.lookup_policy
            with policy_number=@variables.policy_number
            with include_history=True
            set @variables.current_policy = @outputs.policy
            set @variables.premium_amount = @outputs.premium

   reasoning:
      instructions:->
         | Policy Management Dashboard

         if @variables.policy_number == "":
            | Please provide your policy number.
              Format: POL-XXXXXX

         if @variables.policy_number != "":
            | Policy: {!@variables.policy_number}
              Monthly Premium: ${!@variables.premium_amount}

            if @variables.is_policy_active:
               | Status: Active
            else:
               | Status: Requires attention

            | I can help you with:
              1. View coverage
              2. Make changes
              3. Request documents
              4. File a claim

      actions:
         set_policy_info: @utils.setVariables
            description: "Store policy number"
            with policy_number=...

         view_policy: @actions.lookup_policy
            description: "Look up policy"
            with policy_number=@variables.policy_number
            with include_history=True
            set @variables.current_policy = @outputs.policy
            set @variables.premium_amount = @outputs.premium

         update_my_coverage: @actions.update_coverage
            description: "Modify coverage"
            available when @variables.is_policy_active == True and @variables.policy_record_id != None
            with policy_record_id=@variables.policy_record_id
            with coverage_type=...
            with new_limit=...
            with effective_date=...
            set @variables.current_policy = @outputs.updated_policy
            set @variables.premium_amount = @outputs.new_premium
            run @actions.calculate_premium
               with policy_record_id=@variables.policy_record_id
               with proposed_changes=@variables.current_policy
            run @actions.send_policy_documents
               with policy_record_id=@variables.policy_record_id
               with document_types=["coverage_change_confirmation"]
               with email=@variables.logged_in_user_email

         send_documents: @actions.send_policy_documents
            description: "Send policy documents"
            available when @variables.policy_record_id != None
            with policy_record_id=@variables.policy_record_id
            with document_types=...
            with email=@variables.logged_in_user_email

         file_a_claim: @utils.transition to @topic.claims_processing
            description: "File a claim"
            available when @variables.is_policy_active == True

         back_to_main: @utils.transition to @topic.general_support
            description: "Return to main menu"

         speak_to_agent: @utils.escalate
            description: "Speak to agent"
            available when @variables.retry_count > 2 or @variables.fraud_flag == True

   after_reasoning:
      if @variables.retry_count > 3:
         set @variables.fraud_flag = True

topic claims_processing:
   description: "Handles claims lifecycle"
   reasoning:
      instructions: "Handle claims"
"#;
    let result = parse(source);
    if let Err(ref errs) = result {
        for err in errs {
            eprintln!("Error: {}", err);
        }
    }
    assert!(result.is_ok(), "Parse failed with all blocks - THIS IS THE BUG");
    let file = result.unwrap();
    assert_eq!(file.topics.len(), 2, "Should have 2 topics");
}
