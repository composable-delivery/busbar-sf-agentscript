//! Edge types for the reference graph.

use serde::{Deserialize, Serialize};

/// An edge in the reference graph representing a relationship between nodes.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RefEdge {
    /// StartAgent routes to a topic
    Routes,

    /// Topic transitions to another topic (via `transition_to`)
    TransitionsTo,

    /// Topic delegates to another topic (via `topic_delegate`)
    Delegates,

    /// Reasoning action invokes an action definition
    Invokes,

    /// Action reads a variable (via `with` clause or condition)
    Reads,

    /// Action writes to a variable (via `set` clause)
    Writes,

    /// Action chains to another action (via `run` clause)
    Chains,

    /// Escalation routes to a connection
    Escalates,
}

impl RefEdge {
    /// Get a human-readable label for this edge type.
    pub fn label(&self) -> &'static str {
        match self {
            RefEdge::Routes => "routes",
            RefEdge::TransitionsTo => "transitions_to",
            RefEdge::Delegates => "delegates",
            RefEdge::Invokes => "invokes",
            RefEdge::Reads => "reads",
            RefEdge::Writes => "writes",
            RefEdge::Chains => "chains",
            RefEdge::Escalates => "escalates",
        }
    }

    /// Check if this is a control flow edge (affects execution path).
    pub fn is_control_flow(&self) -> bool {
        matches!(
            self,
            RefEdge::Routes
                | RefEdge::TransitionsTo
                | RefEdge::Delegates
                | RefEdge::Invokes
                | RefEdge::Chains
                | RefEdge::Escalates
        )
    }

    /// Check if this is a data flow edge (affects data).
    pub fn is_data_flow(&self) -> bool {
        matches!(self, RefEdge::Reads | RefEdge::Writes)
    }
}
