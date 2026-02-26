//! Serialization types for graph export.
//!
//! This module contains all the data structures used to serialize RefGraph
//! data for external consumption (JSON, WASM, etc.).

use crate::error::ValidationError;
use crate::{RefGraph, RefNode, ValidationResult};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};

// ============================================================================
// Basic representations
// ============================================================================

/// Serializable representation of a RefGraph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRepr {
    pub nodes: Vec<NodeRepr>,
    pub edges: Vec<EdgeRepr>,
    pub topics: Vec<String>,
    pub variables: Vec<String>,
}

impl From<&RefGraph> for GraphRepr {
    fn from(graph: &RefGraph) -> Self {
        let inner = graph.inner();

        let nodes: Vec<NodeRepr> = inner
            .node_indices()
            .filter_map(|idx| graph.get_node(idx).map(NodeRepr::from))
            .collect();

        let edges: Vec<EdgeRepr> = inner
            .edge_references()
            .map(|e| EdgeRepr {
                source: e.source().index(),
                target: e.target().index(),
                edge_type: e.weight().label().to_string(),
            })
            .collect();

        let topics: Vec<String> = graph.topic_names().map(|s| s.to_string()).collect();
        let variables: Vec<String> = graph.variable_names().map(|s| s.to_string()).collect();

        Self {
            nodes,
            edges,
            topics,
            variables,
        }
    }
}

/// Serializable representation of a RefNode.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRepr {
    pub node_type: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub target: Option<String>,
    pub mutable: Option<bool>,
    pub span_start: usize,
    pub span_end: usize,
}

impl From<&RefNode> for NodeRepr {
    fn from(node: &RefNode) -> Self {
        match node {
            RefNode::StartAgent { span } => NodeRepr {
                node_type: "start_agent".to_string(),
                name: None,
                topic: None,
                target: None,
                mutable: None,
                span_start: span.0,
                span_end: span.1,
            },
            RefNode::Topic { name, span } => NodeRepr {
                node_type: "topic".to_string(),
                name: Some(name.clone()),
                topic: None,
                target: None,
                mutable: None,
                span_start: span.0,
                span_end: span.1,
            },
            RefNode::ActionDef { name, topic, span } => NodeRepr {
                node_type: "action_def".to_string(),
                name: Some(name.clone()),
                topic: Some(topic.clone()),
                target: None,
                mutable: None,
                span_start: span.0,
                span_end: span.1,
            },
            RefNode::ReasoningAction {
                name,
                topic,
                target,
                span,
            } => NodeRepr {
                node_type: "reasoning_action".to_string(),
                name: Some(name.clone()),
                topic: Some(topic.clone()),
                target: target.clone(),
                mutable: None,
                span_start: span.0,
                span_end: span.1,
            },
            RefNode::Variable {
                name,
                mutable,
                span,
            } => NodeRepr {
                node_type: "variable".to_string(),
                name: Some(name.clone()),
                topic: None,
                target: None,
                mutable: Some(*mutable),
                span_start: span.0,
                span_end: span.1,
            },
            RefNode::Connection { name, span } => NodeRepr {
                node_type: "connection".to_string(),
                name: Some(name.clone()),
                topic: None,
                target: None,
                mutable: None,
                span_start: span.0,
                span_end: span.1,
            },
        }
    }
}

/// Serializable representation of a RefEdge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRepr {
    pub source: usize,
    pub target: usize,
    pub edge_type: String,
}

// ============================================================================
// Validation representations
// ============================================================================

/// Serializable representation of ValidationResult.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResultRepr {
    pub errors: Vec<ValidationErrorRepr>,
    pub warnings: Vec<ValidationErrorRepr>,
    pub is_valid: bool,
}

impl From<&ValidationResult> for ValidationResultRepr {
    fn from(result: &ValidationResult) -> Self {
        Self {
            errors: result
                .errors
                .iter()
                .map(ValidationErrorRepr::from)
                .collect(),
            warnings: result
                .warnings
                .iter()
                .map(ValidationErrorRepr::from)
                .collect(),
            is_valid: result.is_ok(),
        }
    }
}

/// Serializable representation of ValidationError.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationErrorRepr {
    pub error_type: String,
    pub message: String,
    pub span_start: Option<usize>,
    pub span_end: Option<usize>,
}

impl From<&ValidationError> for ValidationErrorRepr {
    fn from(error: &ValidationError) -> Self {
        let span = error.span();
        Self {
            error_type: match error {
                ValidationError::UnresolvedReference { .. } => "unresolved_reference",
                ValidationError::CycleDetected { .. } => "cycle_detected",
                ValidationError::UnreachableTopic { .. } => "unreachable_topic",
                ValidationError::UnusedActionDef { .. } => "unused_action_def",
                ValidationError::UnusedVariable { .. } => "unused_variable",
                ValidationError::UninitializedVariable { .. } => "uninitialized_variable",
            }
            .to_string(),
            message: error.message(),
            span_start: span.map(|s| s.0),
            span_end: span.map(|s| s.1),
        }
    }
}

// ============================================================================
// Variable usage representations
// ============================================================================

/// Serializable representation of variable usages.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableUsagesRepr {
    pub readers: Vec<UsageInfoRepr>,
    pub writers: Vec<UsageInfoRepr>,
}

/// Serializable representation of a usage location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfoRepr {
    pub location: String,
    pub node_type: String,
    pub topic: Option<String>,
    pub context: Option<String>,
}

impl UsageInfoRepr {
    pub fn from_node(node: &RefNode) -> Self {
        match node {
            RefNode::ActionDef { name, topic, .. } => UsageInfoRepr {
                location: name.clone(),
                node_type: "action_def".to_string(),
                topic: Some(topic.clone()),
                context: None,
            },
            RefNode::ReasoningAction {
                name,
                topic,
                target,
                ..
            } => UsageInfoRepr {
                location: name.clone(),
                node_type: "reasoning_action".to_string(),
                topic: Some(topic.clone()),
                context: target.clone(),
            },
            RefNode::Topic { name, .. } => UsageInfoRepr {
                location: name.clone(),
                node_type: "topic".to_string(),
                topic: Some(name.clone()),
                context: None,
            },
            RefNode::StartAgent { .. } => UsageInfoRepr {
                location: "start_agent".to_string(),
                node_type: "start_agent".to_string(),
                topic: None,
                context: None,
            },
            RefNode::Variable { name, .. } => UsageInfoRepr {
                location: name.clone(),
                node_type: "variable".to_string(),
                topic: None,
                context: None,
            },
            RefNode::Connection { name, .. } => UsageInfoRepr {
                location: name.clone(),
                node_type: "connection".to_string(),
                topic: None,
                context: None,
            },
        }
    }
}

// ============================================================================
// Full export types (for JSON/GraphQL)
// ============================================================================

/// Full graph export for external consumption.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExport {
    pub version: String,
    pub nodes: Vec<GraphExportNode>,
    pub edges: Vec<GraphExportEdge>,
    pub topics: Vec<TopicExportInfo>,
    pub variables: Vec<String>,
    pub stats: StatsExport,
    pub validation: ValidationExport,
}

/// Node representation for full export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExportNode {
    pub id: usize,
    pub node_type: String,
    pub name: Option<String>,
    pub topic: Option<String>,
    pub target: Option<String>,
    pub mutable: Option<bool>,
    pub span: SpanRepr,
}

/// Edge representation for full export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExportEdge {
    pub source: usize,
    pub target: usize,
    pub edge_type: String,
}

/// Span representation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanRepr {
    pub start: usize,
    pub end: usize,
}

/// Topic information for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopicExportInfo {
    pub name: String,
    pub description: Option<String>,
    pub is_entry: bool,
    pub transitions_to: Vec<String>,
    pub delegates_to: Vec<String>,
    pub actions: Vec<ActionExportInfo>,
}

/// Action information within a topic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExportInfo {
    pub name: String,
    pub target: Option<String>,
}

/// Statistics for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsExport {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub topics: usize,
    pub variables: usize,
    pub action_defs: usize,
    pub reasoning_actions: usize,
}

/// Validation results for export.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationExport {
    pub is_valid: bool,
    pub errors: Vec<ValidationErrorRepr>,
    pub warnings: Vec<ValidationErrorRepr>,
}

// ============================================================================
// Builder for full export
// ============================================================================

impl GraphExport {
    /// Build a full export from a RefGraph.
    pub fn from_graph(graph: &RefGraph) -> Self {
        let inner = graph.inner();

        // Build nodes
        let nodes: Vec<GraphExportNode> = inner
            .node_indices()
            .filter_map(|idx| {
                graph.get_node(idx).map(|node| {
                    let repr = NodeRepr::from(node);
                    GraphExportNode {
                        id: idx.index(),
                        node_type: repr.node_type,
                        name: repr.name,
                        topic: repr.topic,
                        target: repr.target,
                        mutable: repr.mutable,
                        span: SpanRepr {
                            start: repr.span_start,
                            end: repr.span_end,
                        },
                    }
                })
            })
            .collect();

        // Build edges
        let edges: Vec<GraphExportEdge> = inner
            .edge_references()
            .map(|e| GraphExportEdge {
                source: e.source().index(),
                target: e.target().index(),
                edge_type: e.weight().label().to_string(),
            })
            .collect();

        // Build topic info
        let mut topic_info: Vec<TopicExportInfo> = Vec::new();

        // Add start_agent first
        topic_info.push(TopicExportInfo {
            name: "start_agent".to_string(),
            description: None,
            is_entry: true,
            transitions_to: Vec::new(),
            delegates_to: Vec::new(),
            actions: Vec::new(),
        });

        // Collect topic information
        for topic_name in graph.topic_names() {
            let mut transitions = Vec::new();
            let mut delegates = Vec::new();
            let mut actions = Vec::new();

            // Find topic's actions and transitions from edges
            for edge in inner.edge_references() {
                let edge_type = edge.weight().label();
                if let (Some(src), Some(tgt)) =
                    (graph.get_node(edge.source()), graph.get_node(edge.target()))
                {
                    match (src, edge_type) {
                        (RefNode::Topic { name: src_name, .. }, "transitions_to")
                            if src_name == topic_name =>
                        {
                            if let RefNode::Topic { name: tgt_name, .. } = tgt {
                                transitions.push(tgt_name.clone());
                            }
                        }
                        (RefNode::Topic { name: src_name, .. }, "delegates")
                            if src_name == topic_name =>
                        {
                            if let RefNode::Topic { name: tgt_name, .. } = tgt {
                                delegates.push(tgt_name.clone());
                            }
                        }
                        _ => {}
                    }
                }
            }

            // Find actions defined in this topic
            for idx in inner.node_indices() {
                if let Some(RefNode::ReasoningAction {
                    name,
                    topic,
                    target,
                    ..
                }) = graph.get_node(idx)
                {
                    if topic == topic_name {
                        actions.push(ActionExportInfo {
                            name: name.clone(),
                            target: target.clone(),
                        });
                    }
                }
            }

            topic_info.push(TopicExportInfo {
                name: topic_name.to_string(),
                description: None,
                is_entry: false,
                transitions_to: transitions,
                delegates_to: delegates,
                actions,
            });
        }

        // Update start_agent transitions
        for edge in inner.edge_references() {
            if edge.weight().label() == "routes" {
                if let Some(RefNode::StartAgent { .. }) = graph.get_node(edge.source()) {
                    if let Some(RefNode::Topic { name, .. }) = graph.get_node(edge.target()) {
                        if let Some(start) = topic_info.get_mut(0) {
                            start.transitions_to.push(name.clone());
                        }
                    }
                }
            }
        }

        // Get stats and validation
        let stats = graph.stats();
        let validation = graph.validate();

        GraphExport {
            version: env!("CARGO_PKG_VERSION").to_string(),
            nodes,
            edges,
            topics: topic_info,
            variables: graph.variable_names().map(|s| s.to_string()).collect(),
            stats: StatsExport {
                total_nodes: stats.total_definitions(),
                total_edges: stats.total_edges(),
                topics: stats.topics,
                variables: stats.variables,
                action_defs: stats.action_defs,
                reasoning_actions: stats.reasoning_actions,
            },
            validation: ValidationExport {
                is_valid: validation.is_ok(),
                errors: validation
                    .errors
                    .iter()
                    .map(ValidationErrorRepr::from)
                    .collect(),
                warnings: validation
                    .warnings
                    .iter()
                    .map(ValidationErrorRepr::from)
                    .collect(),
            },
        }
    }
}
