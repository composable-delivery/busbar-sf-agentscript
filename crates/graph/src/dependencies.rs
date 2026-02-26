//! Dependency extraction and analysis for AgentScript files.
//!
//! This module identifies all external dependencies on Salesforce org configuration:
//! - **Objects/Fields**: Referenced via record actions (create://, read://, query://, etc.)
//! - **Flows**: Referenced via flow://FlowName
//! - **Apex Classes**: Referenced via apex://ClassName
//! - **Knowledge Bases**: Referenced in knowledge block
//! - **Connections**: Referenced for escalation routing
//!
//! This enables offline analysis of agent dependencies without round-tripping to the org.

use busbar_sf_agentscript_parser::ast::{ActionDef, ConnectionBlock, KnowledgeBlock};
use busbar_sf_agentscript_parser::AgentFile;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Type of Salesforce org dependency.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DependencyType {
    /// Salesforce Object (e.g., Account, Contact, custom__c)
    SObject(String),
    /// Salesforce Field on an object (Object.Field)
    Field { object: String, field: String },
    /// Flow (flow://FlowName)
    Flow(String),
    /// Apex Class (apex://ClassName)
    ApexClass(String),
    /// Apex Method (apex://ClassName.methodName)
    ApexMethod { class: String, method: String },
    /// Knowledge Base
    KnowledgeBase(String),
    /// Connection for escalation
    Connection(String),
    /// Prompt Template
    PromptTemplate(String),
    /// External Service
    ExternalService(String),
    /// Unknown/Custom target
    Custom(String),
}

impl DependencyType {
    /// Get the category of this dependency.
    pub fn category(&self) -> &'static str {
        match self {
            DependencyType::SObject(_) => "sobject",
            DependencyType::Field { .. } => "field",
            DependencyType::Flow(_) => "flow",
            DependencyType::ApexClass(_) => "apex_class",
            DependencyType::ApexMethod { .. } => "apex_method",
            DependencyType::KnowledgeBase(_) => "knowledge",
            DependencyType::Connection(_) => "connection",
            DependencyType::PromptTemplate(_) => "prompt_template",
            DependencyType::ExternalService(_) => "external_service",
            DependencyType::Custom(_) => "custom",
        }
    }

    /// Get the name of this dependency.
    pub fn name(&self) -> String {
        match self {
            DependencyType::SObject(name) => name.clone(),
            DependencyType::Field { object, field } => format!("{}.{}", object, field),
            DependencyType::Flow(name) => name.clone(),
            DependencyType::ApexClass(name) => name.clone(),
            DependencyType::ApexMethod { class, method } => format!("{}.{}", class, method),
            DependencyType::KnowledgeBase(name) => name.clone(),
            DependencyType::Connection(name) => name.clone(),
            DependencyType::PromptTemplate(name) => name.clone(),
            DependencyType::ExternalService(name) => name.clone(),
            DependencyType::Custom(target) => target.clone(),
        }
    }
}

/// A single dependency with its source location.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    /// The type and name of the dependency
    pub dep_type: DependencyType,
    /// Where this dependency is used (topic name or "start_agent")
    pub used_in: String,
    /// The action name that references this dependency
    pub action_name: String,
    /// Source span (start, end)
    pub span: (usize, usize),
}

/// Complete dependency report for an AgentScript file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyReport {
    /// All unique SObjects referenced
    pub sobjects: HashSet<String>,
    /// All unique fields referenced (Object.Field)
    pub fields: HashSet<String>,
    /// All Flows referenced
    pub flows: HashSet<String>,
    /// All Apex classes referenced
    pub apex_classes: HashSet<String>,
    /// All Knowledge bases referenced
    pub knowledge_bases: HashSet<String>,
    /// All Connections referenced
    pub connections: HashSet<String>,
    /// All Prompt Templates referenced
    pub prompt_templates: HashSet<String>,
    /// External services referenced
    pub external_services: HashSet<String>,
    /// All dependencies with full details
    pub all_dependencies: Vec<Dependency>,
    /// Dependencies grouped by type
    pub by_type: HashMap<String, Vec<Dependency>>,
    /// Dependencies grouped by topic
    pub by_topic: HashMap<String, Vec<Dependency>>,
}

impl DependencyReport {
    /// Check if a specific SObject is used.
    pub fn uses_sobject(&self, name: &str) -> bool {
        self.sobjects.contains(name)
    }

    /// Check if a specific Flow is used.
    pub fn uses_flow(&self, name: &str) -> bool {
        self.flows.contains(name)
    }

    /// Check if a specific Apex class is used.
    pub fn uses_apex_class(&self, name: &str) -> bool {
        self.apex_classes.contains(name)
    }

    /// Get all dependencies of a specific type.
    pub fn get_by_type(&self, category: &str) -> Vec<&Dependency> {
        self.by_type
            .get(category)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Get all dependencies used in a specific topic.
    pub fn get_by_topic(&self, topic: &str) -> Vec<&Dependency> {
        self.by_topic
            .get(topic)
            .map(|deps| deps.iter().collect())
            .unwrap_or_default()
    }

    /// Get total count of unique dependencies.
    pub fn unique_count(&self) -> usize {
        self.sobjects.len()
            + self.fields.len()
            + self.flows.len()
            + self.apex_classes.len()
            + self.knowledge_bases.len()
            + self.connections.len()
            + self.prompt_templates.len()
            + self.external_services.len()
    }
}

/// Extract all Salesforce org dependencies from an AgentScript AST.
pub fn extract_dependencies(ast: &AgentFile) -> DependencyReport {
    let mut report = DependencyReport::default();

    // Extract from knowledge block
    if let Some(knowledge) = &ast.knowledge {
        extract_from_knowledge(&knowledge.node, &mut report);
    }

    // Extract from connection blocks
    for connection in &ast.connections {
        extract_from_connection(&connection.node, &mut report);
    }

    // Extract from start_agent actions
    if let Some(start) = &ast.start_agent {
        if let Some(actions) = &start.node.actions {
            for action in &actions.node.actions {
                extract_from_action(
                    &action.node,
                    "start_agent",
                    (action.span.start, action.span.end),
                    &mut report,
                );
            }
        }
    }

    // Extract from topic actions
    for topic in &ast.topics {
        let topic_name = &topic.node.name.node;
        if let Some(actions) = &topic.node.actions {
            for action in &actions.node.actions {
                extract_from_action(
                    &action.node,
                    topic_name,
                    (action.span.start, action.span.end),
                    &mut report,
                );
            }
        }
    }

    // Build grouped views
    for dep in &report.all_dependencies {
        let category = dep.dep_type.category().to_string();
        report
            .by_type
            .entry(category)
            .or_default()
            .push(dep.clone());

        report
            .by_topic
            .entry(dep.used_in.clone())
            .or_default()
            .push(dep.clone());
    }

    report
}

/// Parse an action target and extract dependencies.
fn extract_from_action(
    action: &ActionDef,
    topic: &str,
    span: (usize, usize),
    report: &mut DependencyReport,
) {
    let action_name = action.name.node.clone();

    if let Some(target) = &action.target {
        let target_str = &target.node;
        let dep_type = parse_action_target(target_str);

        // Add to appropriate set
        match &dep_type {
            DependencyType::SObject(name) => {
                report.sobjects.insert(name.clone());
            }
            DependencyType::Field { object, field } => {
                report.sobjects.insert(object.clone());
                report.fields.insert(format!("{}.{}", object, field));
            }
            DependencyType::Flow(name) => {
                report.flows.insert(name.clone());
            }
            DependencyType::ApexClass(name) => {
                report.apex_classes.insert(name.clone());
            }
            DependencyType::ApexMethod { class, .. } => {
                report.apex_classes.insert(class.clone());
            }
            DependencyType::PromptTemplate(name) => {
                report.prompt_templates.insert(name.clone());
            }
            DependencyType::ExternalService(name) => {
                report.external_services.insert(name.clone());
            }
            _ => {}
        }

        report.all_dependencies.push(Dependency {
            dep_type,
            used_in: topic.to_string(),
            action_name,
            span,
        });
    }
}

/// Parse an action target string into a dependency type.
fn parse_action_target(target: &str) -> DependencyType {
    if let Some(name) = target.strip_prefix("flow://") {
        return DependencyType::Flow(name.to_string());
    }

    if let Some(name) = target.strip_prefix("apex://") {
        if let Some((class, method)) = name.split_once('.') {
            return DependencyType::ApexMethod {
                class: class.to_string(),
                method: method.to_string(),
            };
        }
        return DependencyType::ApexClass(name.to_string());
    }

    if let Some(name) = target.strip_prefix("prompt://") {
        return DependencyType::PromptTemplate(name.to_string());
    }

    if let Some(name) = target.strip_prefix("service://") {
        return DependencyType::ExternalService(name.to_string());
    }

    // Record operations: create://, read://, update://, delete://, query://
    for op in &["create://", "read://", "update://", "delete://", "query://"] {
        if let Some(rest) = target.strip_prefix(op) {
            // Check for field access (Object.Field)
            if let Some((object, field)) = rest.split_once('.') {
                return DependencyType::Field {
                    object: object.to_string(),
                    field: field.to_string(),
                };
            }
            return DependencyType::SObject(rest.to_string());
        }
    }

    DependencyType::Custom(target.to_string())
}

/// Extract dependencies from knowledge block.
fn extract_from_knowledge(knowledge: &KnowledgeBlock, report: &mut DependencyReport) {
    for entry in &knowledge.entries {
        let name = entry.node.name.node.clone();
        report.knowledge_bases.insert(name.clone());
        report.all_dependencies.push(Dependency {
            dep_type: DependencyType::KnowledgeBase(name.clone()),
            used_in: "knowledge".to_string(),
            action_name: name,
            span: (entry.span.start, entry.span.end),
        });
    }
}

/// Extract dependencies from a connection block.
fn extract_from_connection(connection: &ConnectionBlock, report: &mut DependencyReport) {
    // Register the connection name
    let connection_name = connection.name.node.clone();
    report.connections.insert(connection_name.clone());

    // Extract dependencies from entries
    for entry in &connection.entries {
        let name = entry.node.name.node.clone();
        report.all_dependencies.push(Dependency {
            dep_type: DependencyType::Connection(connection_name.clone()),
            used_in: format!("connection:{}", connection_name),
            action_name: name,
            span: (entry.span.start, entry.span.end),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_flow_target() {
        let dep = parse_action_target("flow://Get_Customer_Details");
        assert!(matches!(dep, DependencyType::Flow(name) if name == "Get_Customer_Details"));
    }

    #[test]
    fn test_parse_apex_target() {
        let dep = parse_action_target("apex://OrderService");
        assert!(matches!(dep, DependencyType::ApexClass(name) if name == "OrderService"));

        let dep = parse_action_target("apex://OrderService.createOrder");
        assert!(matches!(dep, DependencyType::ApexMethod { class, method }
            if class == "OrderService" && method == "createOrder"));
    }

    #[test]
    fn test_parse_record_target() {
        let dep = parse_action_target("query://Account");
        assert!(matches!(dep, DependencyType::SObject(name) if name == "Account"));

        let dep = parse_action_target("read://Contact.Email");
        assert!(matches!(dep, DependencyType::Field { object, field }
            if object == "Contact" && field == "Email"));
    }

    #[test]
    fn test_parse_prompt_template() {
        let dep = parse_action_target("prompt://Customer_Greeting");
        assert!(matches!(dep, DependencyType::PromptTemplate(name) if name == "Customer_Greeting"));
    }

    #[test]
    fn test_parse_external_service() {
        let dep = parse_action_target("service://WeatherAPI");
        assert!(matches!(dep, DependencyType::ExternalService(name) if name == "WeatherAPI"));
    }

    #[test]
    #[ignore = "Recipe file uses {} empty object literal which is not valid AgentScript"]
    fn test_full_dependency_extraction() {
        // Load and parse a real recipe from the submodule
        let source = include_str!("../../../agent-script-recipes/force-app/future_recipes/customerServiceAgent/aiAuthoringBundles/CustomerServiceAgent/CustomerServiceAgent.agent");
        let ast = busbar_sf_agentscript_parser::parse(source).unwrap();
        let report = extract_dependencies(&ast);

        // Check flows (multiple flow targets in this recipe)
        assert!(report.uses_flow("FetchCustomer"));
        assert!(report.uses_flow("SearchKnowledgeBase"));
        assert!(report.uses_flow("CreateCase"));
        assert!(report.uses_flow("UpdateCase"));
        assert!(report.uses_flow("EscalateCase"));
        assert!(report.uses_flow("SendSatisfactionSurvey"));

        // Check count of flows
        assert!(report.flows.len() >= 6, "Expected at least 6 flows, got {}", report.flows.len());

        // Check apex (IssueClassifier is called via apex://)
        assert!(report.uses_apex_class("IssueClassifier"));

        // Check grouping by topic - triage topic has many actions
        let triage_deps = report.get_by_topic("triage");
        assert!(!triage_deps.is_empty(), "Expected dependencies in triage topic");

        // Check grouping by type
        let flow_deps = report.get_by_type("flow");
        assert!(!flow_deps.is_empty());

        // Verify we can get a summary
        assert!(report.unique_count() > 0);
    }
}
