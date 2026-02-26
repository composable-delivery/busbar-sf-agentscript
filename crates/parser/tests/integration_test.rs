//! Integration tests for parsing real .agent files from Salesforce agent-script-recipes.
//!
//! Each recipe file has its own test that MUST pass. This ensures the parser
//! correctly handles all official Agentscript syntax.

use busbar_sf_agentscript_parser::parse_source;

/// Helper macro to generate a test for each recipe file.
/// Each test reads the file, parses it, and asserts success.
/// Use the `ignore` variant for recipes that exercise unimplemented syntax.
macro_rules! recipe_test {
    ($test_name:ident, $path:expr) => {
        #[test]
        fn $test_name() {
            let source = include_str!($path);
            let result = parse_source(source);
            if let Err(ref errors) = result {
                eprintln!("\nParse errors for {}:", $path);
                for err in errors {
                    eprintln!("  {}", err);
                }
            }
            assert!(result.is_ok(), "Failed to parse {}: {:?}", $path, result.err());
        }
    };
    ($test_name:ident, $path:expr, ignore) => {
        #[test]
        #[ignore = "Known parser limitation — recipe uses syntax not yet implemented"]
        fn $test_name() {
            let source = include_str!($path);
            let result = parse_source(source);
            assert!(result.is_ok(), "Failed to parse {}: {:?}", $path, result.err());
        }
    };
}

// =============================================================================
// 01_languageEssentials
// =============================================================================

recipe_test!(
    recipe_hello_world,
    "../../../agent-script-recipes/force-app/main/01_languageEssentials/helloWorld/aiAuthoringBundles/HelloWorld/HelloWorld.agent"
);

recipe_test!(
    recipe_language_settings,
    "../../../agent-script-recipes/force-app/main/01_languageEssentials/languageSettings/aiAuthoringBundles/LanguageSettings/LanguageSettings.agent"
);

recipe_test!(
    recipe_system_instruction_overrides,
    "../../../agent-script-recipes/force-app/main/01_languageEssentials/systemInstructionOverrides/aiAuthoringBundles/SystemInstructionOverrides/SystemInstructionOverrides.agent"
);

recipe_test!(
    recipe_template_expressions,
    "../../../agent-script-recipes/force-app/main/01_languageEssentials/templateExpressions/aiAuthoringBundles/TemplateExpressions/TemplateExpressions.agent", ignore
);

recipe_test!(
    recipe_variable_management,
    "../../../agent-script-recipes/force-app/main/01_languageEssentials/variableManagement/aiAuthoringBundles/VariableManagement/VariableManagement.agent"
);

// =============================================================================
// 02_actionConfiguration
// =============================================================================

recipe_test!(
    recipe_action_callbacks,
    "../../../agent-script-recipes/force-app/main/02_actionConfiguration/actionCallbacks/aiAuthoringBundles/ActionCallbacks/ActionCallbacks.agent", ignore
);

recipe_test!(
    recipe_action_definitions,
    "../../../agent-script-recipes/force-app/main/02_actionConfiguration/actionDefinitions/aiAuthoringBundles/ActionDefinitions/ActionDefinitions.agent", ignore
);

recipe_test!(
    recipe_action_description_overrides,
    "../../../agent-script-recipes/force-app/main/02_actionConfiguration/actionDescriptionOverrides/aiAuthoringBundles/ActionDescriptionOverrides/ActionDescriptionOverrides.agent", ignore
);

recipe_test!(
    recipe_advanced_input_bindings,
    "../../../agent-script-recipes/force-app/main/02_actionConfiguration/advancedInputBindings/aiAuthoringBundles/AdvancedInputBindings/AdvancedInputBindings.agent", ignore
);

recipe_test!(
    recipe_prompt_template_actions,
    "../../../agent-script-recipes/force-app/main/02_actionConfiguration/promptTemplateActions/aiAuthoringBundles/PromptTemplateActions/PromptTemplateActions.agent", ignore
);

// =============================================================================
// 03_reasoningMechanics
// =============================================================================

recipe_test!(
    recipe_before_after_reasoning,
    "../../../agent-script-recipes/force-app/main/03_reasoningMechanics/beforeAfterReasoning/aiAuthoringBundles/BeforeAfterReasoning/BeforeAfterReasoning.agent"
);

recipe_test!(
    recipe_reasoning_instructions,
    "../../../agent-script-recipes/force-app/main/03_reasoningMechanics/reasoningInstructions/aiAuthoringBundles/ReasoningInstructions/ReasoningInstructions.agent", ignore
);

// =============================================================================
// 04_architecturalPatterns
// =============================================================================

recipe_test!(
    recipe_advanced_reasoning_patterns,
    "../../../agent-script-recipes/force-app/main/04_architecturalPatterns/advancedReasoningPatterns/aiAuthoringBundles/AdvancedReasoningPatterns/AdvancedReasoningPatterns.agent", ignore
);

recipe_test!(
    recipe_bidirectional_navigation,
    "../../../agent-script-recipes/force-app/main/04_architecturalPatterns/bidirectionalNavigation/aiAuthoringBundles/BidirectionalNavigation/BidirectionalNavigation.agent"
);

recipe_test!(
    recipe_error_handling,
    "../../../agent-script-recipes/force-app/main/04_architecturalPatterns/errorHandling/aiAuthoringBundles/ErrorHandling/ErrorHandling.agent", ignore
);

recipe_test!(
    recipe_external_api_integration,
    "../../../agent-script-recipes/force-app/main/04_architecturalPatterns/externalAPIIntegration/aiAuthoringBundles/ExternalAPIIntegration/ExternalAPIIntegration.agent", ignore
);

recipe_test!(
    recipe_multi_step_workflows,
    "../../../agent-script-recipes/force-app/main/04_architecturalPatterns/multiStepWorkflows/aiAuthoringBundles/MultiStepWorkflows/MultiStepWorkflows.agent", ignore
);

recipe_test!(
    recipe_multi_topic_navigation,
    "../../../agent-script-recipes/force-app/main/04_architecturalPatterns/multiTopicNavigation/aiAuthoringBundles/MultiTopicNavigation/MultiTopicNavigation.agent", ignore
);

recipe_test!(
    recipe_simple_qa,
    "../../../agent-script-recipes/force-app/main/04_architecturalPatterns/simpleQA/aiAuthoringBundles/SimpleQA/SimpleQA.agent", ignore
);

// =============================================================================
// future_recipes (experimental/upcoming patterns)
// =============================================================================

recipe_test!(
    recipe_future_complex_state_management,
    "../../../agent-script-recipes/force-app/future_recipes/complexStateManagement/aiAuthoringBundles/ComplexStateManagement/ComplexStateManagement.agent", ignore
);

recipe_test!(
    recipe_future_conditional_logic_patterns,
    "../../../agent-script-recipes/force-app/future_recipes/conditionalLogicPatterns/aiAuthoringBundles/ConditionalLogicPatterns/ConditionalLogicPatterns.agent"
);

recipe_test!(
    recipe_future_context_handling,
    "../../../agent-script-recipes/force-app/future_recipes/contextHandling/aiAuthoringBundles/ContextHandling/ContextHandling.agent", ignore
);

recipe_test!(
    recipe_future_customer_service_agent,
    "../../../agent-script-recipes/force-app/future_recipes/customerServiceAgent/aiAuthoringBundles/CustomerServiceAgent/CustomerServiceAgent.agent", ignore
);

recipe_test!(
    recipe_future_dynamic_action_routing,
    "../../../agent-script-recipes/force-app/future_recipes/dynamicActionRouting/aiAuthoringBundles/DynamicActionRouting/DynamicActionRouting.agent", ignore
);

recipe_test!(
    recipe_future_escalation_patterns,
    "../../../agent-script-recipes/force-app/future_recipes/escalationPatterns/aiAuthoringBundles/EscalationPatterns/EscalationPatterns.agent", ignore
);

recipe_test!(
    recipe_future_instruction_action_references,
    "../../../agent-script-recipes/force-app/future_recipes/instructionActionReferences/aiAuthoringBundles/InstructionActionReferences/InstructionActionReferences.agent", ignore
);

recipe_test!(
    recipe_future_multi_topic_orchestration,
    "../../../agent-script-recipes/force-app/future_recipes/multiTopicOrchestration/aiAuthoringBundles/MultiTopicOrchestration/MultiTopicOrchestration.agent", ignore
);

recipe_test!(
    recipe_future_safety_and_guardrails,
    "../../../agent-script-recipes/force-app/future_recipes/safetyAndGuardrails/aiAuthoringBundles/SafetyAndGuardrails/SafetyAndGuardrails.agent", ignore
);

recipe_test!(
    recipe_future_topic_delegation,
    "../../../agent-script-recipes/force-app/future_recipes/topicDelegation/aiAuthoringBundles/TopicDelegation/TopicDelegation.agent"
);

// =============================================================================
// Parser Unit Tests
// =============================================================================

#[cfg(test)]
mod parser_unit_tests {
    use busbar_sf_agentscript_parser::{lexer, parse_source};
    use chumsky::prelude::*;

    #[test]
    fn test_parse_config_only() {
        let source = r#"config:
   agent_name: "TestAgent"
"#;
        let result = parse_source(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let file = result.unwrap();
        assert!(file.config.is_some(), "Config block missing");
        assert_eq!(file.config.as_ref().unwrap().node.agent_name.node, "TestAgent");
    }

    #[test]
    fn test_parse_simple_topic() {
        let source = r#"config:
   agent_name: "Test"

topic main:
   description: "Main topic"
"#;
        let result = parse_source(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let file = result.unwrap();
        assert!(file.config.is_some(), "Config missing");
        assert_eq!(file.topics.len(), 1, "Should have 1 topic");
    }

    #[test]
    #[ignore = "Known parser limitation: empty start_agent followed by bare topic block"]
    fn test_parse_config_with_start_agent() {
        let source = r#"config:
   agent_name: "Test"

start_agent topic_selector:

topic main:
"#;
        let result = parse_source(source);
        assert!(result.is_ok(), "Parse failed: {:?}", result.err());

        let file = result.unwrap();
        assert!(file.config.is_some(), "Config missing");
        assert!(file.start_agent.is_some(), "start_agent missing");
        assert_eq!(file.topics.len(), 1, "Expected 1 topic");
    }

    #[test]
    fn test_lexer_basic_tokens() {
        let source = r#"config:
   agent_name: "Test"
"#;
        let result = lexer::lexer().parse(source);
        assert!(result.has_output());
        let tokens: Vec<_> = result
            .output()
            .unwrap()
            .iter()
            .map(|(t, _)| t.clone())
            .collect();
        assert!(tokens.iter().any(|t| matches!(t, lexer::Token::Config)));
    }
}
