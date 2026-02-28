//! Validation and analysis of reference graphs.

use crate::edges::RefEdge;
use crate::error::ValidationError;
use crate::nodes::RefNode;
use crate::RefGraph;
use petgraph::algo::{is_cyclic_directed, tarjan_scc};
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;
use std::collections::HashSet;

/// Result of validating a reference graph.
#[derive(Debug, Default)]
pub struct ValidationResult {
    /// All validation errors found
    pub errors: Vec<ValidationError>,
    /// All validation warnings found
    pub warnings: Vec<ValidationError>,
}

impl ValidationResult {
    /// Check if validation passed (no errors).
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }

    /// Check if there are any issues (errors or warnings).
    pub fn has_issues(&self) -> bool {
        !self.errors.is_empty() || !self.warnings.is_empty()
    }

    /// Get all issues (errors and warnings combined).
    pub fn all_issues(&self) -> impl Iterator<Item = &ValidationError> {
        self.errors.iter().chain(self.warnings.iter())
    }
}

impl RefGraph {
    /// Perform full validation of the reference graph.
    ///
    /// Returns errors for issues that would cause runtime failures,
    /// and warnings for issues that may indicate problems.
    pub fn validate(&self) -> ValidationResult {
        let mut result = ValidationResult::default();

        // Report unresolved references found during graph build
        result.errors.extend(self.unresolved_references.clone());

        // Check for cycles
        result.errors.extend(self.find_cycles());

        // Check for unreachable topics
        result.warnings.extend(self.find_unreachable_topics());

        // Check for unused definitions
        result.warnings.extend(self.find_unused_actions());
        result.warnings.extend(self.find_unused_variables());

        result
    }

    /// Find cycles in topic transitions.
    ///
    /// Topic transitions should form a DAG. Cycles indicate infinite loops.
    pub fn find_cycles(&self) -> Vec<ValidationError> {
        if !is_cyclic_directed(&self.graph) {
            return vec![];
        }

        // Find strongly connected components to identify cycles
        let sccs = tarjan_scc(&self.graph);
        let mut errors = Vec::new();

        for scc in sccs {
            // A SCC with more than one node indicates a cycle
            if scc.len() > 1 {
                let path: Vec<String> = scc
                    .iter()
                    .filter_map(|&idx| {
                        if let Some(RefNode::Topic { name, .. }) = self.graph.node_weight(idx) {
                            Some(name.clone())
                        } else {
                            None
                        }
                    })
                    .collect();

                if !path.is_empty() {
                    errors.push(ValidationError::CycleDetected { path });
                }
            }
        }

        errors
    }

    /// Find topics that are unreachable from start_agent.
    pub fn find_unreachable_topics(&self) -> Vec<ValidationError> {
        let start_idx = match self.start_agent {
            Some(idx) => idx,
            None => return vec![], // No start_agent to check from
        };

        // Find all topics reachable from start_agent
        let reachable = self.find_reachable_from(start_idx);

        // Check each topic
        self.topics
            .iter()
            .filter_map(|(name, &idx)| {
                if !reachable.contains(&idx) {
                    if let Some(RefNode::Topic { span, .. }) = self.graph.node_weight(idx) {
                        Some(ValidationError::UnreachableTopic {
                            name: name.clone(),
                            span: *span,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find action definitions that are never invoked.
    pub fn find_unused_actions(&self) -> Vec<ValidationError> {
        self.action_defs
            .iter()
            .filter_map(|((topic, name), &idx)| {
                // Check if any edge points to this action
                let has_incoming = self
                    .graph
                    .edges_directed(idx, Direction::Incoming)
                    .any(|e| matches!(e.weight(), RefEdge::Invokes));

                if !has_incoming {
                    if let Some(RefNode::ActionDef { span, .. }) = self.graph.node_weight(idx) {
                        Some(ValidationError::UnusedActionDef {
                            name: name.clone(),
                            topic: topic.clone(),
                            span: *span,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find variables that are never read.
    pub fn find_unused_variables(&self) -> Vec<ValidationError> {
        self.variables
            .iter()
            .filter_map(|(name, &idx)| {
                // Check if any edge reads from this variable
                let has_readers = self
                    .graph
                    .edges_directed(idx, Direction::Incoming)
                    .any(|e| matches!(e.weight(), RefEdge::Reads));

                if !has_readers {
                    if let Some(RefNode::Variable { span, .. }) = self.graph.node_weight(idx) {
                        Some(ValidationError::UnusedVariable {
                            name: name.clone(),
                            span: *span,
                        })
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    /// Find all nodes reachable from a starting node.
    fn find_reachable_from(&self, start: NodeIndex) -> HashSet<NodeIndex> {
        let mut reachable = HashSet::new();
        let mut stack = vec![start];

        while let Some(idx) = stack.pop() {
            if reachable.insert(idx) {
                // Add all outgoing neighbors
                for edge in self.graph.edges_directed(idx, Direction::Outgoing) {
                    stack.push(edge.target());
                }
            }
        }

        reachable
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_and_build(source: &str) -> RefGraph {
        let ast = busbar_sf_agentscript_parser::parse(source).expect("Failed to parse");
        RefGraph::from_ast(&ast).expect("Failed to build graph")
    }

    #[test]
    fn test_no_cycles() {
        let source = r#"config:
   agent_name: "Test"

start_agent topic_selector:
   description: "Route to topics"
   reasoning:
      instructions: "Select the best topic"
      actions:
         go_help: @utils.transition to @topic.help
            description: "Go to help topic"

topic help:
   description: "Help topic"
   reasoning:
      instructions: "Provide help"
"#;
        let graph = parse_and_build(source);
        let result = graph.validate();
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_cycle_detected_between_two_topics() {
        // topic_a transitions to topic_b and topic_b transitions back to topic_a,
        // forming a cycle that should be detected.
        let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Route"
   reasoning:
      instructions: "Select"
      actions:
         go_a: @utils.transition to @topic.topic_a
            description: "Go to A"

topic topic_a:
   description: "Topic A"
   reasoning:
      instructions: "In A"
      actions:
         go_b: @utils.transition to @topic.topic_b
            description: "Go to B"

topic topic_b:
   description: "Topic B"
   reasoning:
      instructions: "In B"
      actions:
         go_a: @utils.transition to @topic.topic_a
            description: "Back to A"
"#;
        let graph = parse_and_build(source);
        let cycles = graph.find_cycles();
        assert!(
            !cycles.is_empty(),
            "Expected a cycle between topic_a and topic_b"
        );
        let cycle_names: Vec<_> = cycles
            .iter()
            .flat_map(|e| {
                if let ValidationError::CycleDetected { path } = e {
                    path.clone()
                } else {
                    vec![]
                }
            })
            .collect();
        assert!(
            cycle_names.contains(&"topic_a".to_string())
                || cycle_names.contains(&"topic_b".to_string()),
            "Cycle should involve topic_a and/or topic_b, got: {:?}",
            cycle_names
        );
    }

    #[test]
    fn test_unreachable_topic_detected() {
        // topic_orphan is never the target of any transition, so it is unreachable
        // from start_agent and should be reported as a warning.
        let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Route"
   reasoning:
      instructions: "Select"
      actions:
         go_help: @utils.transition to @topic.help
            description: "Go to help"

topic help:
   description: "Help topic"
   reasoning:
      instructions: "Provide help"

topic orphan:
   description: "This topic is never reached by any transition"
   reasoning:
      instructions: "Orphan"
"#;
        let graph = parse_and_build(source);
        let unreachable = graph.find_unreachable_topics();
        assert!(
            !unreachable.is_empty(),
            "Expected 'orphan' to be detected as unreachable"
        );
        let unreachable_names: Vec<_> = unreachable
            .iter()
            .filter_map(|e| {
                if let ValidationError::UnreachableTopic { name, .. } = e {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            unreachable_names.contains(&"orphan".to_string()),
            "Expected 'orphan' in unreachable topics, got: {:?}",
            unreachable_names
        );
        // 'help' IS reachable so it should not appear
        assert!(
            !unreachable_names.contains(&"help".to_string()),
            "'help' should be reachable"
        );
    }

    #[test]
    fn test_unused_action_def_detected() {
        // get_data is defined in the actions block but no reasoning action invokes it,
        // so it should be reported as an unused action definition.
        let source = r#"config:
   agent_name: "Test"

topic main:
   description: "Main topic"

   actions:
      get_data:
         description: "Retrieves data from backend"
         inputs:
            record_id: string
               description: "Record identifier"
         outputs:
            result: string
               description: "Query result"
         target: "flow://GetData"

   reasoning:
      instructions: "Help the user with their request"
"#;
        let graph = parse_and_build(source);
        let unused = graph.find_unused_actions();
        assert!(
            !unused.is_empty(),
            "Expected 'get_data' to be detected as unused"
        );
        let unused_names: Vec<_> = unused
            .iter()
            .filter_map(|e| {
                if let ValidationError::UnusedActionDef { name, topic, .. } = e {
                    Some((topic.clone(), name.clone()))
                } else {
                    None
                }
            })
            .collect();
        assert!(
            unused_names.contains(&("main".to_string(), "get_data".to_string())),
            "Expected ('main', 'get_data') in unused actions, got: {:?}",
            unused_names
        );
    }

    #[test]
    fn test_unused_variable_detected() {
        // customer_name is declared in the variables block but is never read by
        // any reasoning action, so it should be reported as an unused variable.
        let source = r#"config:
   agent_name: "Test"

variables:
   customer_name: mutable string = ""
      description: "The customer's name — declared but never read"

topic main:
   description: "Main topic"
   reasoning:
      instructions: "Help the user"
"#;
        let graph = parse_and_build(source);
        let unused = graph.find_unused_variables();
        assert!(
            !unused.is_empty(),
            "Expected 'customer_name' to be detected as unused"
        );
        let unused_names: Vec<_> = unused
            .iter()
            .filter_map(|e| {
                if let ValidationError::UnusedVariable { name, .. } = e {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            unused_names.contains(&"customer_name".to_string()),
            "Expected 'customer_name' in unused variables, got: {:?}",
            unused_names
        );
    }

    #[test]
    fn test_unresolved_topic_reference_detected() {
        // The start_agent transitions to @topic.nonexistent which is never defined,
        // so the reference should surface as an error during validation.
        let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Route"
   reasoning:
      instructions: "Select"
      actions:
         go_missing: @utils.transition to @topic.nonexistent
            description: "Go to a topic that does not exist"

topic real_topic:
   description: "The only real topic"
   reasoning:
      instructions: "Real"
"#;
        let graph = parse_and_build(source);
        let result = graph.validate();
        // Unresolved references are collected as errors
        let unresolved: Vec<_> = result
            .errors
            .iter()
            .filter(|e| matches!(e, ValidationError::UnresolvedReference { .. }))
            .collect();
        assert!(
            !unresolved.is_empty(),
            "Expected an unresolved reference error for @topic.nonexistent"
        );
    }

    #[test]
    fn test_validate_returns_ok_for_fully_connected_graph() {
        // All topics reachable, all action defs invoked, no cycles — validate() should
        // return no errors and no warnings.
        let source = r#"config:
   agent_name: "Test"

start_agent selector:
   description: "Route to main"
   reasoning:
      instructions: "Select"
      actions:
         go_main: @utils.transition to @topic.main
            description: "Enter main"

topic main:
   description: "Main topic"

   actions:
      lookup:
         description: "Look up a record"
         inputs:
            id: string
               description: "Record ID"
         outputs:
            name: string
               description: "Record name"
         target: "flow://Lookup"

   reasoning:
      instructions: "Help"
      actions:
         do_lookup: @actions.lookup
            description: "Perform the lookup"
"#;
        let graph = parse_and_build(source);
        let result = graph.validate();
        assert!(
            result.errors.is_empty(),
            "Expected no errors, got: {:?}",
            result.errors
        );
        // The action is invoked, so no unused-action warnings expected
        let unused_action_warns: Vec<_> = result
            .warnings
            .iter()
            .filter(|w| matches!(w, ValidationError::UnusedActionDef { .. }))
            .collect();
        assert!(
            unused_action_warns.is_empty(),
            "Expected no unused-action warnings, got: {:?}",
            unused_action_warns
        );
    }
}
