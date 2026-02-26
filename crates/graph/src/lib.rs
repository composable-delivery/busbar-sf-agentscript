//! # sf-agentscript-graph
//!
//! Graph-based analysis and validation for AgentScript ASTs.
//!
//! This crate provides tools to build a reference graph from a parsed AgentScript AST,
//! enabling validation, analysis, and querying of relationships between definitions.
//!
//! ## Features
//!
//! - **Reference Resolution**: Validate that all `@variables.*`, `@actions.*`, `@topic.*` references resolve
//! - **Cycle Detection**: Ensure topic transitions form a DAG (no cycles)
//! - **Reachability Analysis**: Find unreachable topics from `start_agent`
//! - **Usage Queries**: Find all usages of a definition, or all dependencies of a node
//! - **Dead Code Detection**: Identify unused actions and variables
//!
//! ## Example
//!
//! ```ignore
//! use busbar_sf_agentscript_parser::parse;
//! use busbar_sf_agentscript_graph::RefGraph;
//!
//! let source = r#"
//! config:
//!    agent_name: "MyAgent"
//! "#;
//!
//! let ast = parse(source).unwrap();
//! let graph = RefGraph::from_ast(&ast).unwrap();
//!
//! // Validate all references
//! let errors = graph.validate();
//! for error in errors {
//!     println!("Validation error: {:?}", error);
//! }
//!
//! // Check for cycles
//! if let Some(cycle) = graph.find_cycles().first() {
//!     println!("Cycle detected: {:?}", cycle);
//! }
//! ```

mod builder;
pub mod dependencies;
mod edges;
mod error;
pub mod export;
mod nodes;
mod queries;
pub mod render;
mod validation;

#[cfg(feature = "wasm")]
pub mod wasm;

pub use builder::RefGraphBuilder;
pub use dependencies::{extract_dependencies, Dependency, DependencyReport, DependencyType};
pub use edges::RefEdge;
pub use error::{GraphBuildError, ValidationError};
pub use export::{EdgeRepr, GraphExport, GraphRepr, NodeRepr, ValidationResultRepr};
pub use nodes::RefNode;
pub use queries::QueryResult;
pub use render::{render_actions_view, render_full_view, render_graphml, render_topic_flow};
pub use validation::ValidationResult;

use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// A reference graph built from an AgentScript AST.
///
/// The graph represents relationships between definitions (topics, actions, variables)
/// and can be used for validation, analysis, and querying.
#[derive(Debug)]
pub struct RefGraph {
    /// The underlying directed graph
    graph: DiGraph<RefNode, RefEdge>,

    /// Index of topic nodes by name
    topics: HashMap<String, NodeIndex>,

    /// Index of action definition nodes by (topic_name, action_name)
    action_defs: HashMap<(String, String), NodeIndex>,

    /// Index of reasoning action nodes by (topic_name, action_name)
    reasoning_actions: HashMap<(String, String), NodeIndex>,

    /// Index of variable nodes by name
    variables: HashMap<String, NodeIndex>,

    /// The start_agent node index (if present)
    start_agent: Option<NodeIndex>,

    /// References that could not be resolved during build
    unresolved_references: Vec<ValidationError>,
}

impl RefGraph {
    /// Build a reference graph from a parsed AgentScript AST.
    ///
    /// This traverses the AST and builds nodes for all definitions,
    /// then creates edges for all references between them.
    pub fn from_ast(ast: &busbar_sf_agentscript_parser::AgentFile) -> Result<Self, GraphBuildError> {
        RefGraphBuilder::new().build(ast)
    }

    /// Get the underlying petgraph for advanced operations.
    pub fn inner(&self) -> &DiGraph<RefNode, RefEdge> {
        &self.graph
    }

    /// Get a node by its index.
    pub fn get_node(&self, index: NodeIndex) -> Option<&RefNode> {
        self.graph.node_weight(index)
    }

    /// Look up a topic node by name.
    pub fn get_topic(&self, name: &str) -> Option<NodeIndex> {
        self.topics.get(name).copied()
    }

    /// Look up an action definition node by topic and action name.
    pub fn get_action_def(&self, topic: &str, action: &str) -> Option<NodeIndex> {
        self.action_defs
            .get(&(topic.to_string(), action.to_string()))
            .copied()
    }

    /// Look up a reasoning action node by topic and action name.
    pub fn get_reasoning_action(&self, topic: &str, action: &str) -> Option<NodeIndex> {
        self.reasoning_actions
            .get(&(topic.to_string(), action.to_string()))
            .copied()
    }

    /// Look up a variable node by name.
    pub fn get_variable(&self, name: &str) -> Option<NodeIndex> {
        self.variables.get(name).copied()
    }

    /// Get the start_agent node index.
    pub fn get_start_agent(&self) -> Option<NodeIndex> {
        self.start_agent
    }

    /// Get all topic names in the graph.
    pub fn topic_names(&self) -> impl Iterator<Item = &str> {
        self.topics.keys().map(|s| s.as_str())
    }

    /// Get all variable names in the graph.
    pub fn variable_names(&self) -> impl Iterator<Item = &str> {
        self.variables.keys().map(|s| s.as_str())
    }

    /// Get the number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Get the number of edges in the graph.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_graph() {
        // Minimal valid AgentScript
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
        let ast = busbar_sf_agentscript_parser::parse(source).unwrap();
        let graph = RefGraph::from_ast(&ast).unwrap();

        assert!(graph.node_count() > 0);
        assert!(graph.get_topic("help").is_some());
        assert!(graph.get_start_agent().is_some());
    }
}
