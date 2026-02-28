//! AST Serializer - Convert AgentScript AST back to source text.
//!
//! This module provides functionality to serialize an [`AgentFile`] AST back into
//! valid AgentScript source code. This enables:
//! - Visual editors that modify the AST and regenerate source
//! - Code transformation and refactoring tools
//! - AST-based code generators
//!
//! # Example
//!
//! ```rust
//! use busbar_sf_agentscript::{parse, serialize};
//!
//! let source = r#"
//! config:
//!    agent_name: "Test"
//!
//! topic main:
//!    description: "Main topic"
//! "#;
//!
//! let ast = parse(source).unwrap();
//! let regenerated = serialize(&ast);
//! println!("{}", regenerated);
//! ```
//!
//! # Formatting
//!
//! The serializer produces idiomatic AgentScript with:
//! - 3-space indentation (AgentScript standard)
//! - Consistent spacing and newlines
//! - Proper quoting of strings
//! - Correct reference formatting (`@namespace.path`)

use crate::ast::*;
use std::fmt::Write;

/// Serialize an AgentFile AST to AgentScript source code.
///
/// # Example
///
/// ```rust
/// use busbar_sf_agentscript::{parse, serialize};
///
/// let ast = parse("config:\n   agent_name: \"Test\"\n").unwrap();
/// let source = serialize(&ast);
/// assert!(source.contains("agent_name: \"Test\""));
/// ```
pub fn serialize(agent: &AgentFile) -> String {
    let mut w = Writer::new();
    w.write_agent_file(agent);
    w.finish()
}

/// Internal writer for building output.
struct Writer {
    output: String,
    indent: usize,
}

impl Writer {
    fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    fn finish(self) -> String {
        self.output
    }

    /// Write indentation at current level (3 spaces per level).
    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("   ");
        }
    }

    /// Increase indentation level.
    fn indent(&mut self) {
        self.indent += 1;
    }

    /// Decrease indentation level.
    fn dedent(&mut self) {
        if self.indent > 0 {
            self.indent -= 1;
        }
    }

    /// Write a line with current indentation.
    fn writeln(&mut self, text: &str) {
        self.write_indent();
        writeln!(self.output, "{}", text).unwrap();
    }

    /// Write without indentation or newline.
    #[allow(dead_code)]
    fn write(&mut self, text: &str) {
        write!(self.output, "{}", text).unwrap();
    }

    /// Write a newline.
    fn newline(&mut self) {
        self.output.push('\n');
    }

    // ========================================================================
    // Top-Level File Structure
    // ========================================================================

    fn write_agent_file(&mut self, agent: &AgentFile) {
        // Write blocks in standard order
        if let Some(config) = &agent.config {
            self.write_config_block(&config.node);
            self.newline();
        }

        if let Some(variables) = &agent.variables {
            self.write_variables_block(&variables.node);
            self.newline();
        }

        if let Some(system) = &agent.system {
            self.write_system_block(&system.node);
            self.newline();
        }

        for connection in &agent.connections {
            self.write_connection_block(&connection.node);
            self.newline();
        }

        if let Some(knowledge) = &agent.knowledge {
            self.write_knowledge_block(&knowledge.node);
            self.newline();
        }

        if let Some(language) = &agent.language {
            self.write_language_block(&language.node);
            self.newline();
        }

        if let Some(start_agent) = &agent.start_agent {
            self.write_start_agent_block(&start_agent.node);
            self.newline();
        }

        for topic in &agent.topics {
            self.write_topic_block(&topic.node);
            self.newline();
        }
    }

    // ========================================================================
    // Config Block
    // ========================================================================

    fn write_config_block(&mut self, config: &ConfigBlock) {
        self.writeln("config:");
        self.indent();

        self.write_indent();
        write!(self.output, "agent_name: \"{}\"", config.agent_name.node).unwrap();
        self.newline();

        if let Some(label) = &config.agent_label {
            self.write_indent();
            write!(self.output, "agent_label: \"{}\"", label.node).unwrap();
            self.newline();
        }

        if let Some(desc) = &config.description {
            self.write_indent();
            write!(self.output, "description: \"{}\"", escape_string(&desc.node)).unwrap();
            self.newline();
        }

        if let Some(agent_type) = &config.agent_type {
            self.write_indent();
            write!(self.output, "agent_type: \"{}\"", agent_type.node).unwrap();
            self.newline();
        }

        if let Some(user) = &config.default_agent_user {
            self.write_indent();
            write!(self.output, "default_agent_user: \"{}\"", user.node).unwrap();
            self.newline();
        }

        self.dedent();
    }

    // ========================================================================
    // Variables Block
    // ========================================================================

    fn write_variables_block(&mut self, vars: &VariablesBlock) {
        self.writeln("variables:");
        self.indent();

        for var in &vars.variables {
            self.write_variable_decl(&var.node);
        }

        self.dedent();
    }

    fn write_variable_decl(&mut self, var: &VariableDecl) {
        self.write_indent();
        write!(self.output, "{}: ", var.name.node).unwrap();

        // Write kind and type
        match var.kind {
            VariableKind::Mutable => {
                write!(self.output, "mutable {}", self.type_to_string(&var.ty.node)).unwrap();
                if let Some(default) = &var.default {
                    write!(self.output, " = {}", self.expr_to_string(&default.node)).unwrap();
                }
            }
            VariableKind::Linked => {
                write!(self.output, "linked {}", self.type_to_string(&var.ty.node)).unwrap();
            }
        }
        self.newline();

        // Write metadata
        self.indent();
        if let Some(desc) = &var.description {
            self.write_indent();
            write!(self.output, "description: \"{}\"", escape_string(&desc.node)).unwrap();
            self.newline();
        }

        if let Some(source) = &var.source {
            self.write_indent();
            write!(self.output, "source: {}", self.reference_to_string(&source.node)).unwrap();
            self.newline();
        }
        self.dedent();
    }

    // ========================================================================
    // System Block
    // ========================================================================

    fn write_system_block(&mut self, system: &SystemBlock) {
        self.writeln("system:");
        self.indent();

        if let Some(instructions) = &system.instructions {
            self.write_indent();
            write!(self.output, "instructions:").unwrap();
            self.write_instructions(&instructions.node);
        }

        if let Some(messages) = &system.messages {
            self.writeln("messages:");
            self.indent();

            if let Some(welcome) = &messages.node.welcome {
                self.write_indent();
                write!(self.output, "welcome: \"{}\"", escape_string(&welcome.node)).unwrap();
                self.newline();
            }

            if let Some(error) = &messages.node.error {
                self.write_indent();
                write!(self.output, "error: \"{}\"", escape_string(&error.node)).unwrap();
                self.newline();
            }

            self.dedent();
        }

        self.dedent();
    }

    // ========================================================================
    // Connection Block
    // ========================================================================

    fn write_connection_block(&mut self, connection: &ConnectionBlock) {
        // Write: connection <name>:
        self.write_indent();
        write!(self.output, "connection {}:", connection.name.node).unwrap();
        self.newline();

        self.indent();
        for entry in &connection.entries {
            self.write_indent();
            write!(
                self.output,
                "{}: \"{}\"",
                entry.node.name.node,
                escape_string(&entry.node.value.node)
            )
            .unwrap();
            self.newline();
        }
        self.dedent();
    }

    // ========================================================================
    // Knowledge Block
    // ========================================================================

    fn write_knowledge_block(&mut self, knowledge: &KnowledgeBlock) {
        self.writeln("knowledge:");
        self.indent();

        for entry in &knowledge.entries {
            self.write_indent();
            write!(
                self.output,
                "{}: {}",
                entry.node.name.node,
                self.expr_to_string(&entry.node.value.node)
            )
            .unwrap();
            self.newline();
        }

        self.dedent();
    }

    // ========================================================================
    // Language Block
    // ========================================================================

    fn write_language_block(&mut self, language: &LanguageBlock) {
        self.writeln("language:");
        self.indent();

        for entry in &language.entries {
            self.write_indent();
            write!(
                self.output,
                "{}: {}",
                entry.node.name.node,
                self.expr_to_string(&entry.node.value.node)
            )
            .unwrap();
            self.newline();
        }

        self.dedent();
    }

    // ========================================================================
    // Start Agent Block
    // ========================================================================

    fn write_start_agent_block(&mut self, start_agent: &StartAgentBlock) {
        self.write_indent();
        write!(self.output, "start_agent {}:", start_agent.name.node).unwrap();
        self.newline();

        self.indent();

        if let Some(desc) = &start_agent.description {
            self.write_indent();
            write!(self.output, "description: \"{}\"", escape_string(&desc.node)).unwrap();
            self.newline();
        }

        if let Some(system) = &start_agent.system {
            self.write_topic_system_override(&system.node);
        }

        if let Some(actions) = &start_agent.actions {
            self.write_actions_block(&actions.node);
        }

        if let Some(before) = &start_agent.before_reasoning {
            self.writeln("before_reasoning:");
            self.indent();
            self.write_directive_block(&before.node);
            self.dedent();
        }

        if let Some(reasoning) = &start_agent.reasoning {
            self.write_reasoning_block(&reasoning.node);
        }

        if let Some(after) = &start_agent.after_reasoning {
            self.writeln("after_reasoning:");
            self.indent();
            self.write_directive_block(&after.node);
            self.dedent();
        }

        self.dedent();
    }

    // ========================================================================
    // Topic Block
    // ========================================================================

    fn write_topic_block(&mut self, topic: &TopicBlock) {
        self.write_indent();
        write!(self.output, "topic {}:", topic.name.node).unwrap();
        self.newline();

        self.indent();

        if let Some(desc) = &topic.description {
            self.write_indent();
            write!(self.output, "description: \"{}\"", escape_string(&desc.node)).unwrap();
            self.newline();
        }

        if let Some(system) = &topic.system {
            self.write_topic_system_override(&system.node);
        }

        if let Some(actions) = &topic.actions {
            self.write_actions_block(&actions.node);
        }

        if let Some(before) = &topic.before_reasoning {
            self.writeln("before_reasoning:");
            self.indent();
            self.write_directive_block(&before.node);
            self.dedent();
        }

        if let Some(reasoning) = &topic.reasoning {
            self.write_reasoning_block(&reasoning.node);
        }

        if let Some(after) = &topic.after_reasoning {
            self.writeln("after_reasoning:");
            self.indent();
            self.write_directive_block(&after.node);
            self.dedent();
        }

        self.dedent();
    }

    fn write_topic_system_override(&mut self, system: &TopicSystemOverride) {
        if let Some(instructions) = &system.instructions {
            self.writeln("system:");
            self.indent();
            self.write_indent();
            write!(self.output, "instructions:").unwrap();
            self.write_instructions(&instructions.node);
            self.dedent();
        }
    }

    // ========================================================================
    // Actions Block
    // ========================================================================

    fn write_actions_block(&mut self, actions: &ActionsBlock) {
        self.writeln("actions:");
        self.indent();

        for action in &actions.actions {
            self.write_action_def(&action.node);
        }

        self.dedent();
    }

    fn write_action_def(&mut self, action: &ActionDef) {
        self.write_indent();
        write!(self.output, "{}:", action.name.node).unwrap();
        self.newline();

        self.indent();

        if let Some(desc) = &action.description {
            self.write_indent();
            write!(self.output, "description: \"{}\"", escape_string(&desc.node)).unwrap();
            self.newline();
        }

        if let Some(label) = &action.label {
            self.write_indent();
            write!(self.output, "label: \"{}\"", escape_string(&label.node)).unwrap();
            self.newline();
        }

        if let Some(target) = &action.target {
            self.write_indent();
            write!(self.output, "target: \"{}\"", escape_string(&target.node)).unwrap();
            self.newline();
        }

        if let Some(confirm) = &action.require_user_confirmation {
            self.write_indent();
            write!(
                self.output,
                "require_user_confirmation: {}",
                if confirm.node { "True" } else { "False" }
            )
            .unwrap();
            self.newline();
        }

        if let Some(progress) = &action.include_in_progress_indicator {
            self.write_indent();
            write!(
                self.output,
                "include_in_progress_indicator: {}",
                if progress.node { "True" } else { "False" }
            )
            .unwrap();
            self.newline();
        }

        if let Some(msg) = &action.progress_indicator_message {
            self.write_indent();
            write!(self.output, "progress_indicator_message: \"{}\"", escape_string(&msg.node))
                .unwrap();
            self.newline();
        }

        if let Some(inputs) = &action.inputs {
            self.writeln("inputs:");
            self.indent();
            for param in &inputs.node {
                self.write_param_def(&param.node);
            }
            self.dedent();
        }

        if let Some(outputs) = &action.outputs {
            self.writeln("outputs:");
            self.indent();
            for param in &outputs.node {
                self.write_param_def(&param.node);
            }
            self.dedent();
        }

        self.dedent();
    }

    fn write_param_def(&mut self, param: &ParamDef) {
        self.write_indent();
        write!(self.output, "{}: {}", param.name.node, self.type_to_string(&param.ty.node))
            .unwrap();
        self.newline();

        self.indent();

        if let Some(desc) = &param.description {
            self.write_indent();
            write!(self.output, "description: \"{}\"", escape_string(&desc.node)).unwrap();
            self.newline();
        }

        if let Some(label) = &param.label {
            self.write_indent();
            write!(self.output, "label: \"{}\"", escape_string(&label.node)).unwrap();
            self.newline();
        }

        if let Some(required) = &param.is_required {
            self.write_indent();
            write!(self.output, "is_required: {}", if required.node { "True" } else { "False" })
                .unwrap();
            self.newline();
        }

        if let Some(filter) = &param.filter_from_agent {
            self.write_indent();
            write!(
                self.output,
                "filter_from_agent: {}",
                if filter.node { "True" } else { "False" }
            )
            .unwrap();
            self.newline();
        }

        if let Some(displayable) = &param.is_displayable {
            self.write_indent();
            write!(
                self.output,
                "is_displayable: {}",
                if displayable.node { "True" } else { "False" }
            )
            .unwrap();
            self.newline();
        }

        if let Some(complex) = &param.complex_data_type_name {
            self.write_indent();
            write!(self.output, "complex_data_type_name: \"{}\"", escape_string(&complex.node))
                .unwrap();
            self.newline();
        }

        self.dedent();
    }

    // ========================================================================
    // Directive Block (before_reasoning, after_reasoning)
    // ========================================================================

    fn write_directive_block(&mut self, block: &DirectiveBlock) {
        for stmt in &block.statements {
            self.write_statement(&stmt.node, false);
        }
    }

    fn write_statement(&mut self, stmt: &Stmt, _in_reasoning: bool) {
        match stmt {
            Stmt::Set { target, value } => {
                self.write_indent();
                write!(
                    self.output,
                    "set {} = {}",
                    self.reference_to_string(&target.node),
                    self.expr_to_string(&value.node)
                )
                .unwrap();
                self.newline();
            }
            Stmt::Run {
                action,
                with_clauses,
                set_clauses,
            } => {
                self.write_indent();
                write!(self.output, "run {}", self.reference_to_string(&action.node)).unwrap();
                self.newline();

                self.indent();
                for with_clause in with_clauses {
                    self.write_with_clause(&with_clause.node);
                }
                for set_clause in set_clauses {
                    self.write_set_clause(&set_clause.node);
                }
                self.dedent();
            }
            Stmt::If {
                condition,
                then_block,
                else_block,
            } => {
                self.write_indent();
                write!(self.output, "if {}:", self.expr_to_string(&condition.node)).unwrap();
                self.newline();

                self.indent();
                for then_stmt in then_block {
                    self.write_statement(&then_stmt.node, _in_reasoning);
                }
                self.dedent();

                if let Some(else_stmts) = else_block {
                    self.writeln("else:");
                    self.indent();
                    for else_stmt in else_stmts {
                        self.write_statement(&else_stmt.node, _in_reasoning);
                    }
                    self.dedent();
                }
            }
            Stmt::Transition { target } => {
                self.write_indent();
                write!(self.output, "transition to {}", self.reference_to_string(&target.node))
                    .unwrap();
                self.newline();
            }
        }
    }

    // ========================================================================
    // Reasoning Block
    // ========================================================================

    fn write_reasoning_block(&mut self, reasoning: &ReasoningBlock) {
        self.writeln("reasoning:");
        self.indent();

        if let Some(instructions) = &reasoning.instructions {
            self.write_indent();
            write!(self.output, "instructions:").unwrap();
            self.write_instructions(&instructions.node);
        }

        if let Some(actions) = &reasoning.actions {
            self.writeln("actions:");
            self.indent();
            for action in &actions.node {
                self.write_reasoning_action(&action.node);
            }
            self.dedent();
        }

        self.dedent();
    }

    fn write_reasoning_action(&mut self, action: &ReasoningAction) {
        self.write_indent();
        write!(
            self.output,
            "{}: {}",
            action.name.node,
            self.reasoning_action_target_to_string(&action.target.node)
        )
        .unwrap();
        self.newline();

        self.indent();

        if let Some(desc) = &action.description {
            self.write_indent();
            write!(self.output, "description: \"{}\"", escape_string(&desc.node)).unwrap();
            self.newline();
        }

        if let Some(available) = &action.available_when {
            self.write_indent();
            write!(self.output, "available when {}", self.expr_to_string(&available.node)).unwrap();
            self.newline();
        }

        for with_clause in &action.with_clauses {
            self.write_with_clause(&with_clause.node);
        }

        for set_clause in &action.set_clauses {
            self.write_set_clause(&set_clause.node);
        }

        for run_clause in &action.run_clauses {
            self.write_run_clause(&run_clause.node);
        }

        for if_clause in &action.if_clauses {
            self.write_if_clause(&if_clause.node);
        }

        if let Some(transition) = &action.transition {
            self.write_indent();
            write!(self.output, "transition to {}", self.reference_to_string(&transition.node))
                .unwrap();
            self.newline();
        }

        self.dedent();
    }

    fn write_with_clause(&mut self, with: &WithClause) {
        self.write_indent();
        write!(self.output, "with {} = ", with.param.node).unwrap();
        match &with.value.node {
            WithValue::Expr(expr) => {
                write!(self.output, "{}", self.expr_to_string(expr)).unwrap();
            }
        }
        self.newline();
    }

    fn write_set_clause(&mut self, set: &SetClause) {
        self.write_indent();
        write!(
            self.output,
            "set {} = {}",
            self.reference_to_string(&set.target.node),
            self.expr_to_string(&set.source.node)
        )
        .unwrap();
        self.newline();
    }

    fn write_run_clause(&mut self, run: &RunClause) {
        self.write_indent();
        write!(self.output, "run {}", self.reference_to_string(&run.action.node)).unwrap();
        self.newline();

        self.indent();
        for with_clause in &run.with_clauses {
            self.write_with_clause(&with_clause.node);
        }
        for set_clause in &run.set_clauses {
            self.write_set_clause(&set_clause.node);
        }
        self.dedent();
    }

    fn write_if_clause(&mut self, if_clause: &IfClause) {
        self.write_indent();
        write!(self.output, "if {}:", self.expr_to_string(&if_clause.condition.node)).unwrap();
        self.newline();

        if let Some(transition) = &if_clause.transition {
            self.indent();
            self.write_indent();
            write!(self.output, "transition to {}", self.reference_to_string(&transition.node))
                .unwrap();
            self.newline();
            self.dedent();
        }
    }

    // ========================================================================
    // Instructions
    // ========================================================================

    fn write_instructions(&mut self, instructions: &Instructions) {
        match instructions {
            Instructions::Simple(text) => {
                // Simple string on same line
                write!(self.output, " \"{}\"", escape_string(text)).unwrap();
                self.newline();
            }
            Instructions::Static(lines) => {
                // Static multiline: caller already wrote "instructions:", we add "|"
                write!(self.output, "|").unwrap();
                self.newline();
                self.indent();
                for line in lines {
                    self.write_indent();
                    write!(self.output, "{}", line.node).unwrap();
                    self.newline();
                }
                self.dedent();
            }
            Instructions::Dynamic(parts) => {
                // Dynamic multiline: caller already wrote "instructions:", we add "->"
                write!(self.output, "->").unwrap();
                self.newline();
                self.indent();
                for part in parts {
                    self.write_instruction_part(&part.node);
                }
                self.dedent();
            }
        }
    }

    fn write_instruction_part(&mut self, part: &InstructionPart) {
        match part {
            InstructionPart::Text(text) => {
                // Text may contain newlines - first line gets |, continuation lines get extra indent
                let lines: Vec<&str> = text.split('\n').collect();
                for (i, line) in lines.iter().enumerate() {
                    self.write_indent();
                    if i == 0 {
                        write!(self.output, "| {}", line).unwrap();
                    } else {
                        // Continuation line - extra indent, no pipe
                        write!(self.output, "  {}", line).unwrap();
                    }
                    self.newline();
                }
            }
            InstructionPart::Interpolation(expr) => {
                self.write_indent();
                write!(self.output, "{{!{}}}", self.expr_to_string(expr)).unwrap();
                self.newline();
            }
            InstructionPart::Conditional {
                condition,
                then_parts,
                else_parts,
            } => {
                self.write_indent();
                write!(self.output, "if {}:", self.expr_to_string(&condition.node)).unwrap();
                self.newline();

                self.indent();
                for then_part in then_parts {
                    self.write_instruction_part(&then_part.node);
                }
                self.dedent();

                if let Some(else_ps) = else_parts {
                    self.writeln("else:");
                    self.indent();
                    for else_part in else_ps {
                        self.write_instruction_part(&else_part.node);
                    }
                    self.dedent();
                }
            }
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    #[allow(clippy::only_used_in_recursion)]
    fn type_to_string(&self, ty: &Type) -> String {
        match ty {
            Type::String => "string".to_string(),
            Type::Number => "number".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::Object => "object".to_string(),
            Type::Date => "date".to_string(),
            Type::Timestamp => "timestamp".to_string(),
            Type::Currency => "currency".to_string(),
            Type::Id => "id".to_string(),
            Type::Datetime => "datetime".to_string(),
            Type::Time => "time".to_string(),
            Type::Integer => "integer".to_string(),
            Type::Long => "long".to_string(),
            Type::List(inner) => format!("list[{}]", self.type_to_string(inner)),
        }
    }

    fn reference_to_string(&self, reference: &Reference) -> String {
        reference.full_path()
    }

    fn reasoning_action_target_to_string(&self, target: &ReasoningActionTarget) -> String {
        match target {
            ReasoningActionTarget::Action(r) => self.reference_to_string(r),
            ReasoningActionTarget::TransitionTo(r) => {
                format!("@utils.transition to {}", self.reference_to_string(r))
            }
            ReasoningActionTarget::Escalate => "@utils.escalate".to_string(),
            ReasoningActionTarget::SetVariables => "@utils.setVariables".to_string(),
            ReasoningActionTarget::TopicDelegate(r) => self.reference_to_string(r),
        }
    }

    fn expr_to_string(&self, expr: &Expr) -> String {
        match expr {
            Expr::Reference(r) => self.reference_to_string(r),
            Expr::String(s) => format!("\"{}\"", escape_string(s)),
            Expr::Number(n) => {
                // Format numbers nicely
                if n.fract() == 0.0 && n.is_finite() {
                    format!("{:.0}", n)
                } else {
                    format!("{}", n)
                }
            }
            Expr::Bool(b) => {
                if *b {
                    "True".to_string()
                } else {
                    "False".to_string()
                }
            }
            Expr::None => "None".to_string(),
            Expr::List(items) => {
                let items_str: Vec<_> =
                    items.iter().map(|i| self.expr_to_string(&i.node)).collect();
                format!("[{}]", items_str.join(", "))
            }
            Expr::Object(map) => {
                let pairs: Vec<_> = map
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, self.expr_to_string(&v.node)))
                    .collect();
                format!("{{{}}}", pairs.join(", "))
            }
            Expr::BinOp { left, op, right } => {
                format!(
                    "{} {} {}",
                    self.expr_to_string(&left.node),
                    self.binop_to_string(op),
                    self.expr_to_string(&right.node)
                )
            }
            Expr::UnaryOp { op, operand } => {
                format!("{} {}", self.unaryop_to_string(op), self.expr_to_string(&operand.node))
            }
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
            } => {
                format!(
                    "{} if {} else {}",
                    self.expr_to_string(&then_expr.node),
                    self.expr_to_string(&condition.node),
                    self.expr_to_string(&else_expr.node)
                )
            }
            Expr::Property { object, field } => {
                format!("{}.{}", self.expr_to_string(&object.node), field.node)
            }
            Expr::Index { object, index } => {
                format!(
                    "{}[{}]",
                    self.expr_to_string(&object.node),
                    self.expr_to_string(&index.node)
                )
            }
        }
    }

    fn binop_to_string(&self, op: &BinOp) -> &'static str {
        match op {
            BinOp::Eq => "==",
            BinOp::Ne => "!=",
            BinOp::Lt => "<",
            BinOp::Gt => ">",
            BinOp::Le => "<=",
            BinOp::Ge => ">=",
            BinOp::Is => "is",
            BinOp::IsNot => "is not",
            BinOp::And => "and",
            BinOp::Or => "or",
            BinOp::Add => "+",
            BinOp::Sub => "-",
        }
    }

    fn unaryop_to_string(&self, op: &UnaryOp) -> &'static str {
        match op {
            UnaryOp::Not => "not",
            UnaryOp::Neg => "-",
        }
    }
}

/// Escape special characters in strings.
fn escape_string(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_minimal_config() {
        let mut agent = AgentFile::new();
        agent.config = Some(Spanned::new(
            ConfigBlock {
                agent_name: Spanned::new("TestAgent".to_string(), 0..10),
                agent_label: None,
                description: None,
                agent_type: None,
                default_agent_user: None,
            },
            0..10,
        ));

        let output = serialize(&agent);
        assert!(output.contains("config:"));
        assert!(output.contains("agent_name: \"TestAgent\""));
    }

    #[test]
    fn test_serialize_variable() {
        let mut agent = AgentFile::new();
        agent.variables = Some(Spanned::new(
            VariablesBlock {
                variables: vec![Spanned::new(
                    VariableDecl {
                        name: Spanned::new("test_var".to_string(), 0..8),
                        kind: VariableKind::Mutable,
                        ty: Spanned::new(Type::String, 0..6),
                        default: Some(Spanned::new(Expr::String("default".to_string()), 0..7)),
                        description: Some(Spanned::new("Test variable".to_string(), 0..13)),
                        source: None,
                    },
                    0..50,
                )],
            },
            0..50,
        ));

        let output = serialize(&agent);
        assert!(output.contains("variables:"));
        assert!(output.contains("test_var: mutable string"));
        assert!(output.contains("description: \"Test variable\""));
    }

    #[test]
    fn test_serialize_topic() {
        let mut agent = AgentFile::new();
        agent.topics = vec![Spanned::new(
            TopicBlock {
                name: Spanned::new("main".to_string(), 0..4),
                description: Some(Spanned::new("Main topic".to_string(), 0..10)),
                system: None,
                actions: None,
                before_reasoning: None,
                reasoning: None,
                after_reasoning: None,
            },
            0..50,
        )];

        let output = serialize(&agent);
        assert!(output.contains("topic main:"));
        assert!(output.contains("description: \"Main topic\""));
    }

    #[test]
    fn test_escape_string() {
        assert_eq!(escape_string("hello"), "hello");
        assert_eq!(escape_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_string("line1\nline2"), "line1\\nline2");
        assert_eq!(escape_string("tab\there"), "tab\\there");
    }

    #[test]
    fn test_type_to_string() {
        let w = Writer::new();
        assert_eq!(w.type_to_string(&Type::String), "string");
        assert_eq!(w.type_to_string(&Type::Number), "number");
        assert_eq!(w.type_to_string(&Type::Boolean), "boolean");
        assert_eq!(w.type_to_string(&Type::List(Box::new(Type::String))), "list[string]");
    }

    #[test]
    fn test_expr_to_string() {
        let w = Writer::new();
        assert_eq!(w.expr_to_string(&Expr::String("test".to_string())), "\"test\"");
        assert_eq!(w.expr_to_string(&Expr::Number(42.0)), "42");
        assert_eq!(w.expr_to_string(&Expr::Number(3.15)), "3.15");
        assert_eq!(w.expr_to_string(&Expr::Bool(true)), "True");
        assert_eq!(w.expr_to_string(&Expr::Bool(false)), "False");
        assert_eq!(w.expr_to_string(&Expr::None), "None");
    }
}
