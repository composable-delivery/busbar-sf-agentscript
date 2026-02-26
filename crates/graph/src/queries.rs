//! Query operations on the reference graph.

use crate::edges::RefEdge;
use crate::nodes::RefNode;
use crate::RefGraph;
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use petgraph::visit::EdgeRef;
use petgraph::Direction;

/// Result of a query operation.
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// The nodes matching the query
    pub nodes: Vec<NodeIndex>,
}

impl QueryResult {
    /// Check if the query returned any results.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Get the number of results.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

impl RefGraph {
    /// Find all nodes that use (reference) the given node.
    ///
    /// This returns nodes that have outgoing edges pointing to the target.
    pub fn find_usages(&self, target: NodeIndex) -> QueryResult {
        let nodes = self
            .graph
            .edges_directed(target, Direction::Incoming)
            .map(|e| e.source())
            .collect();

        QueryResult { nodes }
    }

    /// Find all nodes that the given node depends on.
    ///
    /// This returns nodes that the source has outgoing edges pointing to.
    pub fn find_dependencies(&self, source: NodeIndex) -> QueryResult {
        let nodes = self
            .graph
            .edges_directed(source, Direction::Outgoing)
            .map(|e| e.target())
            .collect();

        QueryResult { nodes }
    }

    /// Find all topics that transition to the given topic.
    pub fn find_incoming_transitions(&self, topic: NodeIndex) -> QueryResult {
        let nodes = self
            .graph
            .edges_directed(topic, Direction::Incoming)
            .filter(|e| {
                matches!(e.weight(), RefEdge::TransitionsTo | RefEdge::Delegates | RefEdge::Routes)
            })
            .map(|e| e.source())
            .collect();

        QueryResult { nodes }
    }

    /// Find all topics that the given topic transitions to.
    pub fn find_outgoing_transitions(&self, topic: NodeIndex) -> QueryResult {
        let nodes = self
            .graph
            .edges_directed(topic, Direction::Outgoing)
            .filter(|e| matches!(e.weight(), RefEdge::TransitionsTo | RefEdge::Delegates))
            .map(|e| e.target())
            .collect();

        QueryResult { nodes }
    }

    /// Find all reasoning actions that invoke the given action definition.
    pub fn find_action_invokers(&self, action_def: NodeIndex) -> QueryResult {
        let nodes = self
            .graph
            .edges_directed(action_def, Direction::Incoming)
            .filter(|e| matches!(e.weight(), RefEdge::Invokes))
            .map(|e| e.source())
            .collect();

        QueryResult { nodes }
    }

    /// Find all actions that read the given variable.
    pub fn find_variable_readers(&self, variable: NodeIndex) -> QueryResult {
        let nodes = self
            .graph
            .edges_directed(variable, Direction::Incoming)
            .filter(|e| matches!(e.weight(), RefEdge::Reads))
            .map(|e| e.source())
            .collect();

        QueryResult { nodes }
    }

    /// Find all actions that write to the given variable.
    pub fn find_variable_writers(&self, variable: NodeIndex) -> QueryResult {
        let nodes = self
            .graph
            .edges_directed(variable, Direction::Incoming)
            .filter(|e| matches!(e.weight(), RefEdge::Writes))
            .map(|e| e.source())
            .collect();

        QueryResult { nodes }
    }

    /// Get a topological ordering of topics (for execution order).
    ///
    /// Returns None if there are cycles.
    pub fn topic_execution_order(&self) -> Option<Vec<NodeIndex>> {
        // Build a subgraph of just topics and transition edges
        let topic_indices: Vec<_> = self.topics.values().copied().collect();

        // Use toposort on the full graph, then filter to just topics
        toposort(&self.graph, None).ok().map(|sorted| {
            sorted
                .into_iter()
                .filter(|idx| topic_indices.contains(idx))
                .collect()
        })
    }

    /// Get all reasoning actions in a topic.
    pub fn get_topic_reasoning_actions(&self, topic_name: &str) -> Vec<NodeIndex> {
        self.reasoning_actions
            .iter()
            .filter_map(|((t, _), &idx)| if t == topic_name { Some(idx) } else { None })
            .collect()
    }

    /// Get all action definitions in a topic.
    pub fn get_topic_action_defs(&self, topic_name: &str) -> Vec<NodeIndex> {
        self.action_defs
            .iter()
            .filter_map(|((t, _), &idx)| if t == topic_name { Some(idx) } else { None })
            .collect()
    }

    /// Get summary statistics about the graph.
    pub fn stats(&self) -> GraphStats {
        let mut stats = GraphStats::default();

        for idx in self.graph.node_indices() {
            match self.graph.node_weight(idx) {
                Some(RefNode::Topic { .. }) => stats.topics += 1,
                Some(RefNode::ActionDef { .. }) => stats.action_defs += 1,
                Some(RefNode::ReasoningAction { .. }) => stats.reasoning_actions += 1,
                Some(RefNode::Variable { .. }) => stats.variables += 1,
                Some(RefNode::StartAgent { .. }) => stats.has_start_agent = true,
                Some(RefNode::Connection { .. }) => stats.connections += 1,
                None => {}
            }
        }

        for edge in self.graph.edge_references() {
            match edge.weight() {
                RefEdge::TransitionsTo | RefEdge::Delegates => stats.transitions += 1,
                RefEdge::Invokes => stats.invocations += 1,
                RefEdge::Reads => stats.reads += 1,
                RefEdge::Writes => stats.writes += 1,
                _ => {}
            }
        }

        stats
    }
}

/// Summary statistics about the graph.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct GraphStats {
    pub topics: usize,
    pub action_defs: usize,
    pub reasoning_actions: usize,
    pub variables: usize,
    pub connections: usize,
    pub has_start_agent: bool,
    pub transitions: usize,
    pub invocations: usize,
    pub reads: usize,
    pub writes: usize,
}

impl GraphStats {
    /// Total number of definitions.
    pub fn total_definitions(&self) -> usize {
        self.topics + self.action_defs + self.reasoning_actions + self.variables + self.connections
    }

    /// Total number of edges.
    pub fn total_edges(&self) -> usize {
        self.transitions + self.invocations + self.reads + self.writes
    }
}
