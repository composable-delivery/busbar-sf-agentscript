use crate::ast::{
    ActionDef, AgentFile, ConnectionEntry, Expr, LanguageEntry, Type, VariableDecl, VariableKind,
};
use serde::Serialize;
use std::ops::Range;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SemanticError {
    pub message: String,
    pub span: Option<Range<usize>>,
    pub severity: Severity,
    pub hint: Option<String>,
}

pub fn validate_ast(ast: &AgentFile) -> Vec<SemanticError> {
    let mut errors = Vec::new();

    // Rule 1 & 2: Variables
    if let Some(vars_block) = &ast.variables {
        for var in &vars_block.node.variables {
            validate_variable(&var.node, &mut errors);
        }
    }

    // Rule 3: Locale Code Validation
    if let Some(lang_block) = &ast.language {
        for entry in &lang_block.node.entries {
            validate_language_entry(&entry.node, &mut errors);
        }
    }

    // Rule 4: Outbound Route Type Validation
    for conn_block in &ast.connections {
        for entry in &conn_block.node.entries {
            validate_connection_entry(&entry.node, &mut errors);
        }
    }

    // Rule 5: Action Input Keyword Collision
    // Actions can be in start_agent and topics
    if let Some(start_agent) = &ast.start_agent {
        if let Some(actions) = &start_agent.node.actions {
            for action in &actions.node.actions {
                validate_action_def(&action.node, &mut errors);
            }
        }
    }

    for topic in &ast.topics {
        if let Some(actions) = &topic.node.actions {
            for action in &actions.node.actions {
                validate_action_def(&action.node, &mut errors);
            }
        }
    }

    errors
}

fn validate_variable(var: &VariableDecl, errors: &mut Vec<SemanticError>) {
    // Rule 1: Mutable Variable Type Restrictions
    if let VariableKind::Mutable = var.kind {
        match var.ty.node {
            Type::Integer | Type::Long | Type::Datetime | Type::Time => {
                errors.push(SemanticError {
                    message: format!(
                        "Variable '{}' with type {:?} is not supported for mutable variables. This may be supported in the future.",
                        var.name.node, var.ty.node
                    ),
                    span: Some(var.ty.span.clone()),
                    severity: Severity::Error,
                    hint: Some("Allowed mutable types: String, Boolean, Number, Currency, Date, Id, Object, Timestamp".to_string()),
                });
            }
            _ => {}
        }
    }

    // Rule 2: Context Variable Object Type
    // Linked variables (source starts with @context.) cannot be Object type.
    if let VariableKind::Linked = var.kind {
        if let Some(source) = &var.source {
            if source.node.namespace == "context" {
                if let Type::Object = var.ty.node {
                    errors.push(SemanticError {
                        message: format!(
                            "Context variable '{}' cannot be an object type",
                            var.name.node
                        ),
                        span: Some(var.ty.span.clone()),
                        severity: Severity::Error,
                        hint: None,
                    });
                }
            }
        }
    }
}

fn validate_language_entry(entry: &LanguageEntry, errors: &mut Vec<SemanticError>) {
    // Rule 3: Locale Code Validation
    if entry.name.node == "additional_locales" {
        if let Expr::String(ref s) = entry.value.node {
            let valid_locales = [
                "ar", "bg", "ca", "cs", "da", "de", "el", "en_AU", "en_GB", "en_US", "es", "es_MX",
                "et", "fi", "fr", "fr_CA", "hi", "hr", "hu", "in", "it", "iw", "ja", "ko", "nl_NL",
                "no", "pl", "pt_BR", "pt_PT", "ro", "sv", "th", "tl", "tr", "vi", "zh_CN", "zh_TW",
            ];

            let codes: Vec<&str> = s.split(',').map(|s| s.trim()).collect();
            for code in codes {
                if !valid_locales.contains(&code) {
                    errors.push(SemanticError {
                        message: format!("Invalid additional_locale '{}'.", code),
                        span: Some(entry.value.span.clone()),
                        severity: Severity::Error,
                        hint: Some(format!("Valid locales are: {}", valid_locales.join(", "))),
                    });
                }
            }
        }
    }
}

fn validate_connection_entry(entry: &ConnectionEntry, errors: &mut Vec<SemanticError>) {
    // Rule 4: Outbound Route Type Validation
    if entry.name.node == "outbound_route_type" && entry.value.node != "OmniChannelFlow" {
        errors.push(SemanticError {
            message: format!(
                "invalid outbound_route_type, found '{}' expected 'OmniChannelFlow'",
                entry.value.node
            ),
            span: Some(entry.value.span.clone()),
            severity: Severity::Error,
            hint: None,
        });
    }
}

fn validate_action_def(action: &ActionDef, errors: &mut Vec<SemanticError>) {
    // Rule 5: Action Input Keyword Collision
    if let Some(inputs) = &action.inputs {
        for param in &inputs.node {
            let name = &param.node.name.node;
            match name.as_str() {
                "description" | "label" | "target" | "inputs" | "outputs" => {
                    errors.push(SemanticError {
                        message: format!(
                            "Action input parameter '{}' collides with keyword '{}' and may cause platform parse errors",
                            name, name
                        ),
                        span: Some(param.node.name.span.clone()),
                        severity: Severity::Warning,
                        hint: None,
                    });
                }
                _ => {}
            }
        }
    }
}
