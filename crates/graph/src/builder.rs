//! Builder for constructing a RefGraph from an AST.

use crate::edges::RefEdge;
use crate::error::{GraphBuildError, ValidationError};
use crate::nodes::RefNode;
use crate::RefGraph;
use busbar_sf_agentscript_parser::ast::{
    Expr, InstructionPart, Instructions, ReasoningAction, ReasoningActionTarget, Reference,
    VariableKind,
};
use busbar_sf_agentscript_parser::AgentFile;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Builder for constructing a reference graph from an AST.
pub struct RefGraphBuilder {
    graph: DiGraph<RefNode, RefEdge>,
    topics: HashMap<String, NodeIndex>,
    action_defs: HashMap<(String, String), NodeIndex>,
    reasoning_actions: HashMap<(String, String), NodeIndex>,
    variables: HashMap<String, NodeIndex>,
    start_agent: Option<NodeIndex>,
    unresolved_references: Vec<ValidationError>,
}

impl RefGraphBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            topics: HashMap::new(),
            action_defs: HashMap::new(),
            reasoning_actions: HashMap::new(),
            variables: HashMap::new(),
            start_agent: None,
            unresolved_references: Vec::new(),
        }
    }

    /// Build a RefGraph from an AgentFile AST.
    pub fn build(mut self, ast: &AgentFile) -> Result<RefGraph, GraphBuildError> {
        // Phase 1: Add all definition nodes
        self.add_variables(ast)?;
        self.add_start_agent(ast)?;
        self.add_topics(ast)?;

        // Phase 2: Add all reference edges
        self.add_start_agent_edges(ast)?;
        self.add_topic_edges(ast)?;

        Ok(RefGraph {
            graph: self.graph,
            topics: self.topics,
            action_defs: self.action_defs,
            reasoning_actions: self.reasoning_actions,
            variables: self.variables,
            start_agent: self.start_agent,
            unresolved_references: self.unresolved_references,
        })
    }

    /// Add variable definition nodes.
    fn add_variables(&mut self, ast: &AgentFile) -> Result<(), GraphBuildError> {
        if let Some(variables) = &ast.variables {
            for var in &variables.node.variables {
                let name = var.node.name.node.clone();
                let mutable = matches!(var.node.kind, VariableKind::Mutable);
                let span = (var.span.start, var.span.end);

                let node = RefNode::Variable {
                    name: name.clone(),
                    mutable,
                    span,
                };

                let idx = self.graph.add_node(node);
                self.variables.insert(name, idx);
            }
        }
        Ok(())
    }

    /// Add the start_agent node.
    fn add_start_agent(&mut self, ast: &AgentFile) -> Result<(), GraphBuildError> {
        if let Some(start) = &ast.start_agent {
            let span = (start.span.start, start.span.end);
            let node = RefNode::StartAgent { span };
            let idx = self.graph.add_node(node);
            self.start_agent = Some(idx);

            // Add action definitions from start_agent
            if let Some(actions) = &start.node.actions {
                for action in &actions.node.actions {
                    let action_name = action.node.name.node.clone();
                    let action_span = (action.span.start, action.span.end);

                    let action_node = RefNode::ActionDef {
                        name: action_name.clone(),
                        topic: "start_agent".to_string(),
                        span: action_span,
                    };
                    let action_idx = self.graph.add_node(action_node);
                    self.action_defs
                        .insert(("start_agent".to_string(), action_name), action_idx);
                }
            }

            // Add reasoning actions from start_agent
            if let Some(reasoning) = &start.node.reasoning {
                if let Some(actions) = &reasoning.node.actions {
                    for action in &actions.node {
                        let action_name = action.node.name.node.clone();
                        let action_span = (action.span.start, action.span.end);
                        let target = Self::extract_target(&action.node.target.node);

                        let reasoning_node = RefNode::ReasoningAction {
                            name: action_name.clone(),
                            topic: "start_agent".to_string(),
                            target,
                            span: action_span,
                        };
                        let reasoning_idx = self.graph.add_node(reasoning_node);
                        self.reasoning_actions
                            .insert(("start_agent".to_string(), action_name), reasoning_idx);
                    }
                }
            }
        }
        Ok(())
    }

    /// Add topic nodes and their child action nodes.
    fn add_topics(&mut self, ast: &AgentFile) -> Result<(), GraphBuildError> {
        for topic in &ast.topics {
            let topic_name = topic.node.name.node.clone();
            let span = (topic.span.start, topic.span.end);

            // Add topic node
            let topic_node = RefNode::Topic {
                name: topic_name.clone(),
                span,
            };
            let topic_idx = self.graph.add_node(topic_node);
            self.topics.insert(topic_name.clone(), topic_idx);

            // Add action definition nodes
            if let Some(actions) = &topic.node.actions {
                for action in &actions.node.actions {
                    let action_name = action.node.name.node.clone();
                    let action_span = (action.span.start, action.span.end);

                    let action_node = RefNode::ActionDef {
                        name: action_name.clone(),
                        topic: topic_name.clone(),
                        span: action_span,
                    };
                    let action_idx = self.graph.add_node(action_node);
                    self.action_defs
                        .insert((topic_name.clone(), action_name), action_idx);
                }
            }

            // Add reasoning action nodes
            if let Some(reasoning) = &topic.node.reasoning {
                if let Some(actions) = &reasoning.node.actions {
                    for action in &actions.node {
                        let action_name = action.node.name.node.clone();
                        let action_span = (action.span.start, action.span.end);
                        let target = Self::extract_target(&action.node.target.node);

                        let reasoning_node = RefNode::ReasoningAction {
                            name: action_name.clone(),
                            topic: topic_name.clone(),
                            target,
                            span: action_span,
                        };
                        let reasoning_idx = self.graph.add_node(reasoning_node);
                        self.reasoning_actions
                            .insert((topic_name.clone(), action_name), reasoning_idx);
                    }
                }
            }
        }
        Ok(())
    }

    /// Extract the target string from a ReasoningActionTarget.
    fn extract_target(target: &ReasoningActionTarget) -> Option<String> {
        match target {
            ReasoningActionTarget::Action(reference) => Some(reference.full_path()),
            ReasoningActionTarget::TransitionTo(reference) => Some(reference.full_path()),
            ReasoningActionTarget::TopicDelegate(reference) => Some(reference.full_path()),
            ReasoningActionTarget::Escalate => Some("@utils.escalate".to_string()),
            ReasoningActionTarget::SetVariables => Some("@utils.setVariables".to_string()),
        }
    }

    /// Add edges from start_agent to routed topics.
    fn add_start_agent_edges(&mut self, ast: &AgentFile) -> Result<(), GraphBuildError> {
        let start_idx = match self.start_agent {
            Some(idx) => idx,
            None => return Ok(()),
        };

        if let Some(start) = &ast.start_agent {
            // Extract topic transitions from reasoning actions
            if let Some(reasoning) = &start.node.reasoning {
                if let Some(instructions) = &reasoning.node.instructions {
                    self.scan_instructions(start_idx, &instructions.node);
                }

                if let Some(actions) = &reasoning.node.actions {
                    for action in &actions.node {
                        // Check for transition targets
                        if let ReasoningActionTarget::TransitionTo(ref reference) =
                            action.node.target.node
                        {
                            if let Some(topic_name) = Self::extract_topic_from_ref(reference) {
                                if let Some(&topic_idx) = self.topics.get(&topic_name) {
                                    self.graph.add_edge(start_idx, topic_idx, RefEdge::Routes);
                                } else {
                                    self.unresolved_references.push(
                                        ValidationError::UnresolvedReference {
                                            reference: reference.full_path(),
                                            namespace: "topic".to_string(),
                                            span: (
                                                action.node.target.span.start,
                                                action.node.target.span.end,
                                            ),
                                            context: "start_agent".to_string(),
                                        },
                                    );
                                }
                            }
                        }
                        if let ReasoningActionTarget::TopicDelegate(ref reference) =
                            action.node.target.node
                        {
                            if let Some(topic_name) = Self::extract_topic_from_ref(reference) {
                                if let Some(&topic_idx) = self.topics.get(&topic_name) {
                                    self.graph.add_edge(start_idx, topic_idx, RefEdge::Routes);
                                } else {
                                    self.unresolved_references.push(
                                        ValidationError::UnresolvedReference {
                                            reference: reference.full_path(),
                                            namespace: "topic".to_string(),
                                            span: (
                                                action.node.target.span.start,
                                                action.node.target.span.end,
                                            ),
                                            context: "start_agent".to_string(),
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }

    /// Add edges within and between topics.
    fn add_topic_edges(&mut self, ast: &AgentFile) -> Result<(), GraphBuildError> {
        for topic in &ast.topics {
            let topic_name = &topic.node.name.node;
            let topic_idx = self.topics[topic_name];

            // Add edges from reasoning actions to their targets
            if let Some(reasoning) = &topic.node.reasoning {
                if let Some(instructions) = &reasoning.node.instructions {
                    self.scan_instructions(topic_idx, &instructions.node);
                }

                if let Some(actions) = &reasoning.node.actions {
                    self.add_reasoning_action_edges(topic_name, topic_idx, &actions.node)?;
                }
            }
        }
        Ok(())
    }

    /// Add edges for reasoning actions in a topic.
    fn add_reasoning_action_edges(
        &mut self,
        topic_name: &str,
        topic_idx: NodeIndex,
        actions: &[busbar_sf_agentscript_parser::Spanned<ReasoningAction>],
    ) -> Result<(), GraphBuildError> {
        for action in actions {
            let action_name = &action.node.name.node;
            let reasoning_idx =
                self.reasoning_actions[&(topic_name.to_string(), action_name.clone())];

            match &action.node.target.node {
                ReasoningActionTarget::Action(reference) => {
                    // Reasoning action invokes an action definition
                    if let Some(action_ref) = Self::extract_action_name(reference) {
                        if let Some(&target_idx) = self
                            .action_defs
                            .get(&(topic_name.to_string(), action_ref.clone()))
                        {
                            self.graph
                                .add_edge(reasoning_idx, target_idx, RefEdge::Invokes);
                        } else {
                            self.unresolved_references
                                .push(ValidationError::UnresolvedReference {
                                    reference: reference.full_path(),
                                    namespace: "actions".to_string(),
                                    span: (
                                        action.node.target.span.start,
                                        action.node.target.span.end,
                                    ),
                                    context: format!("topic {}", topic_name),
                                });
                        }
                    }
                }
                ReasoningActionTarget::TransitionTo(reference) => {
                    // Transition to another topic
                    if let Some(target_topic) = Self::extract_topic_from_ref(reference) {
                        if let Some(&target_idx) = self.topics.get(&target_topic) {
                            self.graph
                                .add_edge(topic_idx, target_idx, RefEdge::TransitionsTo);
                        } else {
                            self.unresolved_references
                                .push(ValidationError::UnresolvedReference {
                                    reference: reference.full_path(),
                                    namespace: "topic".to_string(),
                                    span: (
                                        action.node.target.span.start,
                                        action.node.target.span.end,
                                    ),
                                    context: format!("topic {}", topic_name),
                                });
                        }
                    }
                }
                ReasoningActionTarget::TopicDelegate(reference) => {
                    // Delegate to another topic
                    if let Some(target_topic) = Self::extract_topic_from_ref(reference) {
                        if let Some(&target_idx) = self.topics.get(&target_topic) {
                            self.graph
                                .add_edge(topic_idx, target_idx, RefEdge::Delegates);
                        } else {
                            self.unresolved_references
                                .push(ValidationError::UnresolvedReference {
                                    reference: reference.full_path(),
                                    namespace: "topic".to_string(),
                                    span: (
                                        action.node.target.span.start,
                                        action.node.target.span.end,
                                    ),
                                    context: format!("topic {}", topic_name),
                                });
                        }
                    }
                }
                ReasoningActionTarget::Escalate | ReasoningActionTarget::SetVariables => {
                    // Built-in utilities, no edges to add
                }
            }

            // Add edges for with_clauses (reading variables)
            for clause in &action.node.with_clauses {
                self.add_with_value_edges(reasoning_idx, &clause.node.value);
            }

            // Add edges for set_clauses (writing variables)
            for clause in &action.node.set_clauses {
                let target_ref = &clause.node.target.node;
                if target_ref.namespace == "variables" {
                    let var_name = target_ref.path.join(".");
                    if let Some(&var_idx) = self.variables.get(&var_name) {
                        self.graph.add_edge(reasoning_idx, var_idx, RefEdge::Writes);
                    } else {
                        self.unresolved_references
                            .push(ValidationError::UnresolvedReference {
                                reference: target_ref.full_path(),
                                namespace: "variables".to_string(),
                                span: (clause.node.target.span.start, clause.node.target.span.end),
                                context: format!("set clause in topic {}", topic_name),
                            });
                    }
                }
            }
        }
        Ok(())
    }

    /// Scan instructions for variable and action references.
    fn scan_instructions(&mut self, node_idx: NodeIndex, instructions: &Instructions) {
        match instructions {
            Instructions::Simple(_) | Instructions::Static(_) => {
                // Simple/static instructions don't contain references
            }
            Instructions::Dynamic(parts) => {
                for part in parts {
                    self.scan_instruction_part(node_idx, part);
                }
            }
        }
    }

    fn scan_instruction_part(
        &mut self,
        node_idx: NodeIndex,
        part: &busbar_sf_agentscript_parser::Spanned<InstructionPart>,
    ) {
        match &part.node {
            InstructionPart::Text(_) => {}
            InstructionPart::Interpolation(expr) => {
                let spanned_expr = busbar_sf_agentscript_parser::Spanned {
                    node: expr.clone(),
                    span: part.span.clone(),
                };
                self.add_expression_edges(node_idx, &spanned_expr);
            }
            InstructionPart::Conditional {
                condition,
                then_parts,
                else_parts,
            } => {
                self.add_expression_edges(node_idx, condition);
                for p in then_parts {
                    self.scan_instruction_part(node_idx, p);
                }
                if let Some(parts) = else_parts {
                    for p in parts {
                        self.scan_instruction_part(node_idx, p);
                    }
                }
            }
        }
    }

    /// Add edges for variable reads within a with clause value.
    fn add_with_value_edges(
        &mut self,
        from_idx: NodeIndex,
        value: &busbar_sf_agentscript_parser::Spanned<busbar_sf_agentscript_parser::ast::WithValue>,
    ) {
        match &value.node {
            busbar_sf_agentscript_parser::ast::WithValue::Expr(expr) => {
                let spanned_expr = busbar_sf_agentscript_parser::Spanned {
                    node: expr.clone(),
                    span: value.span.clone(),
                };
                self.add_expression_edges(from_idx, &spanned_expr);
            }
        }
    }

    /// Add edges for variable reads within an expression.
    fn add_expression_edges(
        &mut self,
        from_idx: NodeIndex,
        expr: &busbar_sf_agentscript_parser::Spanned<Expr>,
    ) {
        match &expr.node {
            Expr::Reference(reference) => {
                if reference.namespace == "variables" {
                    let var_name = reference.path.join(".");
                    if let Some(&var_idx) = self.variables.get(&var_name) {
                        self.graph.add_edge(from_idx, var_idx, RefEdge::Reads);
                    } else {
                        self.unresolved_references
                            .push(ValidationError::UnresolvedReference {
                                reference: reference.full_path(),
                                namespace: "variables".to_string(),
                                span: (expr.span.start, expr.span.end),
                                context: "variable read".to_string(),
                            });
                    }
                } else if reference.namespace == "actions" {
                    let topic_name = match self.graph.node_weight(from_idx) {
                        Some(RefNode::Topic { name, .. }) => Some(name.clone()),
                        Some(RefNode::StartAgent { .. }) => Some("start_agent".to_string()),
                        Some(RefNode::ReasoningAction { topic, .. }) => Some(topic.clone()),
                        _ => None,
                    };

                    if let Some(topic_name) = topic_name {
                        if let Some(action_ref) = Self::extract_action_name(reference) {
                            if let Some(&action_idx) = self
                                .action_defs
                                .get(&(topic_name.clone(), action_ref.clone()))
                            {
                                self.graph.add_edge(from_idx, action_idx, RefEdge::Invokes);
                            } else {
                                self.unresolved_references.push(
                                    ValidationError::UnresolvedReference {
                                        reference: reference.full_path(),
                                        namespace: "actions".to_string(),
                                        span: (expr.span.start, expr.span.end),
                                        context: format!("topic {}", topic_name),
                                    },
                                );
                            }
                        }
                    }
                }
            }
            Expr::BinOp { left, right, .. } => {
                self.add_expression_edges(from_idx, left);
                self.add_expression_edges(from_idx, right);
            }
            Expr::UnaryOp { operand, .. } => {
                self.add_expression_edges(from_idx, operand);
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                self.add_expression_edges(from_idx, condition);
                self.add_expression_edges(from_idx, then_expr);
                self.add_expression_edges(from_idx, else_expr);
            }
            Expr::List(items) => {
                for item in items {
                    self.add_expression_edges(from_idx, item);
                }
            }
            Expr::Object(entries) => {
                for (_, value) in entries {
                    self.add_expression_edges(from_idx, value);
                }
            }
            Expr::Property { object, .. } => {
                self.add_expression_edges(from_idx, object);
            }
            Expr::Index { object, index } => {
                self.add_expression_edges(from_idx, object);
                self.add_expression_edges(from_idx, index);
            }
            // Literals don't have references
            Expr::String(_) | Expr::Number(_) | Expr::Bool(_) | Expr::None => {}
        }
    }

    /// Extract topic name from a @topic.name reference.
    fn extract_topic_from_ref(reference: &Reference) -> Option<String> {
        if reference.namespace == "topic" && !reference.path.is_empty() {
            Some(reference.path[0].clone())
        } else {
            None
        }
    }

    /// Extract action name from a @actions.name reference.
    fn extract_action_name(reference: &Reference) -> Option<String> {
        if reference.namespace == "actions" && !reference.path.is_empty() {
            Some(reference.path[0].clone())
        } else {
            None
        }
    }
}

impl Default for RefGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}
