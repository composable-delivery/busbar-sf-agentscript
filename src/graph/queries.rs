//! Query operations on the reference graph.

use super::edges::RefEdge;
use super::nodes::RefNode;
use super::RefGraph;
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

#[cfg(test)]
mod tests {
    use crate::RefGraph;

    fn parse_and_build(source: &str) -> RefGraph {
        let ast = busbar_sf_agentscript_parser::parse(source).expect("Failed to parse");
        RefGraph::from_ast(&ast).expect("Failed to build graph")
    }

    /// Source with two topics and a one-way transition: start → topic_a → topic_b.
    fn two_topic_source() -> &'static str {
        r#"config:
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
"#
    }

    #[test]
    fn test_find_outgoing_transitions_from_topic_a() {
        // topic_a transitions to topic_b via @utils.transition, so
        // find_outgoing_transitions(topic_a) should return [topic_b].
        let graph = parse_and_build(two_topic_source());
        let topic_a_idx = graph.get_topic("topic_a").expect("topic_a not found");
        let topic_b_idx = graph.get_topic("topic_b").expect("topic_b not found");

        let result = graph.find_outgoing_transitions(topic_a_idx);
        assert_eq!(result.len(), 1, "Expected exactly 1 outgoing transition from topic_a");
        assert_eq!(
            result.nodes[0], topic_b_idx,
            "Expected transition target to be topic_b"
        );
    }

    #[test]
    fn test_find_incoming_transitions_to_topic_b() {
        // topic_b is only reachable from topic_a, so find_incoming_transitions(topic_b)
        // should return [topic_a].
        let graph = parse_and_build(two_topic_source());
        let topic_a_idx = graph.get_topic("topic_a").expect("topic_a not found");
        let topic_b_idx = graph.get_topic("topic_b").expect("topic_b not found");

        let result = graph.find_incoming_transitions(topic_b_idx);
        assert_eq!(result.len(), 1, "Expected exactly 1 incoming transition to topic_b");
        assert_eq!(
            result.nodes[0], topic_a_idx,
            "Expected transition source to be topic_a"
        );
    }

    #[test]
    fn test_find_outgoing_transitions_empty_for_leaf_topic() {
        // topic_b has no outgoing transitions — it is a leaf node.
        let graph = parse_and_build(two_topic_source());
        let topic_b_idx = graph.get_topic("topic_b").expect("topic_b not found");

        let result = graph.find_outgoing_transitions(topic_b_idx);
        assert!(
            result.is_empty(),
            "Expected no outgoing transitions from leaf topic_b"
        );
    }

    #[test]
    fn test_topic_execution_order_for_acyclic_graph() {
        // An acyclic start → topic_a → topic_b graph should yield a valid topological
        // ordering where topic_a appears before topic_b.
        let graph = parse_and_build(two_topic_source());
        let order = graph.topic_execution_order();
        assert!(
            order.is_some(),
            "Expected a valid topological order for an acyclic graph"
        );

        let order = order.unwrap();
        let topic_a_pos = order
            .iter()
            .position(|&idx| idx == graph.get_topic("topic_a").unwrap());
        let topic_b_pos = order
            .iter()
            .position(|&idx| idx == graph.get_topic("topic_b").unwrap());

        assert!(topic_a_pos.is_some(), "topic_a should appear in execution order");
        assert!(topic_b_pos.is_some(), "topic_b should appear in execution order");
        assert!(
            topic_a_pos.unwrap() < topic_b_pos.unwrap(),
            "topic_a should come before topic_b in topological order"
        );
    }

    #[test]
    fn test_stats_counts_nodes_correctly() {
        // Verify that stats() correctly counts topics, action defs, and variables.
        let source = r#"config:
   agent_name: "Test"

variables:
   order_id: mutable string = ""
      description: "Order ID"

start_agent selector:
   description: "Route"
   reasoning:
      instructions: "Select"
      actions:
         go_main: @utils.transition to @topic.main
            description: "Go to main"

topic main:
   description: "Main topic"

   actions:
      get_order:
         description: "Gets an order"
         inputs:
            id: string
               description: "Order identifier"
         outputs:
            status: string
               description: "Order status"
         target: "flow://GetOrder"

   reasoning:
      instructions: "Help"
"#;
        let graph = parse_and_build(source);
        let stats = graph.stats();

        assert_eq!(stats.topics, 1, "Expected 1 topic");
        assert!(stats.has_start_agent, "Expected start_agent to be present");
        assert_eq!(stats.action_defs, 1, "Expected 1 action def (get_order)");
        assert_eq!(stats.variables, 1, "Expected 1 variable (order_id)");
        // At least one edge should exist (the Routes edge from start_agent → main)
        assert!(
            graph.edge_count() > 0,
            "Expected at least one edge in the graph"
        );
    }
}
