//! Error types for graph building and validation.

use super::nodes::Span;
use thiserror::Error;

/// Errors that can occur when building a reference graph from an AST.
#[derive(Debug, Error)]
pub enum GraphBuildError {
    /// The AST is missing required elements
    #[error("Missing required element: {element}")]
    MissingElement { element: String },

    /// A duplicate definition was found
    #[error("Duplicate {kind} definition: {name} at {span:?}")]
    DuplicateDefinition {
        kind: String,
        name: String,
        span: Span,
    },
}

/// Validation errors found in the reference graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// A reference could not be resolved to a definition
    UnresolvedReference {
        /// The reference string (e.g., "@variables.customer_id")
        reference: String,
        /// The namespace of the reference
        namespace: String,
        /// Source location of the reference
        span: Span,
        /// Context where the reference was used
        context: String,
    },

    /// A cycle was detected in topic transitions
    CycleDetected {
        /// The topics involved in the cycle
        path: Vec<String>,
    },

    /// A topic is unreachable from start_agent
    UnreachableTopic {
        /// The unreachable topic name
        name: String,
        /// Source location
        span: Span,
    },

    /// An action definition is never invoked
    UnusedActionDef {
        /// The action name
        name: String,
        /// The parent topic name
        topic: String,
        /// Source location
        span: Span,
    },

    /// A variable is never read
    UnusedVariable {
        /// The variable name
        name: String,
        /// Source location
        span: Span,
    },

    /// A variable is read but never written
    UninitializedVariable {
        /// The variable name
        name: String,
        /// Source location where it's read
        read_span: Span,
    },
}

impl ValidationError {
    /// Get the primary span for this error.
    pub fn span(&self) -> Option<Span> {
        match self {
            ValidationError::UnresolvedReference { span, .. }
            | ValidationError::UnreachableTopic { span, .. }
            | ValidationError::UnusedActionDef { span, .. }
            | ValidationError::UnusedVariable { span, .. }
            | ValidationError::UninitializedVariable {
                read_span: span, ..
            } => Some(*span),
            ValidationError::CycleDetected { .. } => None,
        }
    }

    /// Get a human-readable error message.
    pub fn message(&self) -> String {
        match self {
            ValidationError::UnresolvedReference {
                reference, context, ..
            } => {
                format!("Unresolved reference '{}' in {}", reference, context)
            }
            ValidationError::CycleDetected { path } => {
                format!("Cycle detected in topic transitions: {}", path.join(" -> "))
            }
            ValidationError::UnreachableTopic { name, .. } => {
                format!("Topic '{}' is unreachable from start_agent", name)
            }
            ValidationError::UnusedActionDef { name, topic, .. } => {
                format!("Action '{}' in topic '{}' is never invoked", name, topic)
            }
            ValidationError::UnusedVariable { name, .. } => {
                format!("Variable '{}' is never read", name)
            }
            ValidationError::UninitializedVariable { name, .. } => {
                format!("Variable '{}' is read but never written", name)
            }
        }
    }

    /// Check if this is a reference resolution error.
    pub fn is_unresolved_reference(&self) -> bool {
        matches!(self, ValidationError::UnresolvedReference { .. })
    }

    /// Check if this is a cycle error.
    pub fn is_cycle(&self) -> bool {
        matches!(self, ValidationError::CycleDetected { .. })
    }

    /// Check if this is an unused definition warning.
    pub fn is_unused(&self) -> bool {
        matches!(
            self,
            ValidationError::UnusedActionDef { .. } | ValidationError::UnusedVariable { .. }
        )
    }
}
