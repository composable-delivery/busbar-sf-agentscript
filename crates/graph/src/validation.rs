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
}
