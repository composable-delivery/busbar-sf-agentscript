//! Node types for the reference graph.

use serde::{Deserialize, Serialize};

/// A span in the source code (start, end byte offsets).
pub type Span = (usize, usize);

/// A node in the reference graph representing a definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RefNode {
    /// The start_agent entry point
    StartAgent {
        /// Source location
        span: Span,
    },

    /// A topic definition
    Topic {
        /// Topic name
        name: String,
        /// Source location
        span: Span,
    },

    /// An action definition within a topic
    ActionDef {
        /// Action name
        name: String,
        /// Parent topic name
        topic: String,
        /// Source location
        span: Span,
    },

    /// A reasoning action within a topic
    ReasoningAction {
        /// Reasoning action name
        name: String,
        /// Parent topic name
        topic: String,
        /// The target action or topic this reasoning action invokes
        target: Option<String>,
        /// Source location
        span: Span,
    },

    /// A variable definition
    Variable {
        /// Variable name
        name: String,
        /// Whether this is a mutable variable
        mutable: bool,
        /// Source location
        span: Span,
    },

    /// A connection/escalation definition
    Connection {
        /// Connection name
        name: String,
        /// Source location
        span: Span,
    },
}

impl RefNode {
    /// Get a human-readable label for this node.
    pub fn label(&self) -> String {
        match self {
            RefNode::StartAgent { .. } => "start_agent".to_string(),
            RefNode::Topic { name, .. } => format!("topic:{}", name),
            RefNode::ActionDef { name, topic, .. } => format!("action:{}:{}", topic, name),
            RefNode::ReasoningAction { name, topic, .. } => {
                format!("reasoning:{}:{}", topic, name)
            }
            RefNode::Variable { name, .. } => format!("variable:{}", name),
            RefNode::Connection { name, .. } => format!("connection:{}", name),
        }
    }

    /// Get the source span for this node.
    pub fn span(&self) -> Span {
        match self {
            RefNode::StartAgent { span }
            | RefNode::Topic { span, .. }
            | RefNode::ActionDef { span, .. }
            | RefNode::ReasoningAction { span, .. }
            | RefNode::Variable { span, .. }
            | RefNode::Connection { span, .. } => *span,
        }
    }

    /// Get the name of this node (if applicable).
    pub fn name(&self) -> Option<&str> {
        match self {
            RefNode::StartAgent { .. } => None,
            RefNode::Topic { name, .. }
            | RefNode::ActionDef { name, .. }
            | RefNode::ReasoningAction { name, .. }
            | RefNode::Variable { name, .. }
            | RefNode::Connection { name, .. } => Some(name),
        }
    }

    /// Check if this node is a topic.
    pub fn is_topic(&self) -> bool {
        matches!(self, RefNode::Topic { .. })
    }

    /// Check if this node is an action definition.
    pub fn is_action_def(&self) -> bool {
        matches!(self, RefNode::ActionDef { .. })
    }

    /// Check if this node is a reasoning action.
    pub fn is_reasoning_action(&self) -> bool {
        matches!(self, RefNode::ReasoningAction { .. })
    }

    /// Check if this node is a variable.
    pub fn is_variable(&self) -> bool {
        matches!(self, RefNode::Variable { .. })
    }
}
