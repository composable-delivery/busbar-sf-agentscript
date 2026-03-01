//! WebAssembly bindings for the AgentScript graph analysis library.
//!
//! This module provides thin JavaScript-accessible wrappers around the core
//! graph functionality. All actual logic lives in other modules:
//! - `render/` - ASCII and GraphML rendering
//! - `export` - Serialization types
//! - Core crate - Graph building, validation, queries

use super::{export, render, RefGraph};
use wasm_bindgen::prelude::*;

// ============================================================================
// Graph building
// ============================================================================

/// Build a reference graph from an AgentScript AST.
#[wasm_bindgen]
pub fn build_graph(ast: JsValue) -> Result<JsValue, JsValue> {
    let agent: crate::AgentFile = serde_wasm_bindgen::from_value(ast)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize AST: {}", e)))?;

    let graph = RefGraph::from_ast(&agent)
        .map_err(|e| JsValue::from_str(&format!("Failed to build graph: {}", e)))?;

    let repr = export::GraphRepr::from(&graph);
    serde_wasm_bindgen::to_value(&repr)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Build a reference graph from AgentScript source code.
#[wasm_bindgen]
pub fn build_graph_from_source(source: &str) -> Result<JsValue, JsValue> {
    let graph = parse_and_build(source)?;
    let repr = export::GraphRepr::from(&graph);
    serde_wasm_bindgen::to_value(&repr)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

// ============================================================================
// Validation
// ============================================================================

/// Validate a reference graph and return any errors/warnings.
#[wasm_bindgen]
pub fn validate_graph(source: &str) -> Result<JsValue, JsValue> {
    let graph = parse_and_build(source)?;
    let result = graph.validate();
    let repr = export::ValidationResultRepr::from(&result);
    serde_wasm_bindgen::to_value(&repr)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

// ============================================================================
// Statistics
// ============================================================================

/// Get statistics about a reference graph.
#[wasm_bindgen]
pub fn get_graph_stats(source: &str) -> Result<JsValue, JsValue> {
    let graph = parse_and_build(source)?;
    let stats = graph.stats();
    serde_wasm_bindgen::to_value(&stats)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

// ============================================================================
// Queries
// ============================================================================

/// Find all usages of a topic by name.
#[wasm_bindgen]
pub fn find_topic_usages(source: &str, topic_name: &str) -> Result<JsValue, JsValue> {
    let graph = parse_and_build(source)?;

    let topic_idx = graph
        .get_topic(topic_name)
        .ok_or_else(|| JsValue::from_str(&format!("Topic '{}' not found", topic_name)))?;

    let usages = graph.find_usages(topic_idx);
    let nodes: Vec<export::NodeRepr> = usages
        .nodes
        .iter()
        .filter_map(|&idx| graph.get_node(idx).map(export::NodeRepr::from))
        .collect();

    serde_wasm_bindgen::to_value(&nodes)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Find all topics that a given topic transitions to.
#[wasm_bindgen]
pub fn find_topic_transitions(source: &str, topic_name: &str) -> Result<JsValue, JsValue> {
    let graph = parse_and_build(source)?;

    let topic_idx = graph
        .get_topic(topic_name)
        .ok_or_else(|| JsValue::from_str(&format!("Topic '{}' not found", topic_name)))?;

    let transitions = graph.find_outgoing_transitions(topic_idx);
    let nodes: Vec<export::NodeRepr> = transitions
        .nodes
        .iter()
        .filter_map(|&idx| graph.get_node(idx).map(export::NodeRepr::from))
        .collect();

    serde_wasm_bindgen::to_value(&nodes)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Find all usages (readers and writers) of a variable by name.
#[wasm_bindgen]
pub fn find_variable_usages(source: &str, var_name: &str) -> Result<String, JsValue> {
    let graph = parse_and_build(source)?;

    let var_idx = graph
        .get_variable(var_name)
        .ok_or_else(|| JsValue::from_str(&format!("Variable '{}' not found", var_name)))?;

    let readers = graph.find_variable_readers(var_idx);
    let writers = graph.find_variable_writers(var_idx);

    let result = export::VariableUsagesRepr {
        readers: readers
            .nodes
            .iter()
            .filter_map(|&idx| graph.get_node(idx).map(export::UsageInfoRepr::from_node))
            .collect(),
        writers: writers
            .nodes
            .iter()
            .filter_map(|&idx| graph.get_node(idx).map(export::UsageInfoRepr::from_node))
            .collect(),
    };

    serde_json::to_string(&result)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

// ============================================================================
// Rendering (ASCII)
// ============================================================================

/// Render the topic flow graph as ASCII art.
#[wasm_bindgen]
pub fn render_topic_flow(source: &str) -> Result<String, JsValue> {
    let graph = parse_and_build(source)?;
    Ok(render::render_topic_flow(&graph))
}

/// Render a detailed execution graph as ASCII art.
#[wasm_bindgen]
pub fn render_graph(source: &str, view: &str) -> Result<String, JsValue> {
    let graph = parse_and_build(source)?;

    match view {
        "topics" => Ok(render::render_topic_flow(&graph)),
        "actions" => Ok(render::render_actions_view(&graph)),
        "full" => Ok(render::render_full_view(&graph)),
        _ => Err(JsValue::from_str("Invalid view type. Use 'topics', 'actions', or 'full'")),
    }
}

// ============================================================================
// Export (JSON)
// ============================================================================

/// Export the graph structure as JSON for visualization/GraphQL consumption.
#[wasm_bindgen]
pub fn export_graph_json(source: &str) -> Result<String, JsValue> {
    let graph = parse_and_build(source)?;
    let export = export::GraphExport::from_graph(&graph);
    serde_json::to_string_pretty(&export)
        .map_err(|e| JsValue::from_str(&format!("JSON serialization error: {}", e)))
}

/// Compact JSON export (no pretty printing).
#[wasm_bindgen]
pub fn export_graph_json_compact(source: &str) -> Result<String, JsValue> {
    let graph = parse_and_build(source)?;
    let repr = export::GraphRepr::from(&graph);
    serde_json::to_string(&repr)
        .map_err(|e| JsValue::from_str(&format!("JSON serialization error: {}", e)))
}

// ============================================================================
// Export (GraphML)
// ============================================================================

/// Export the reference graph as GraphML format.
///
/// GraphML is an XML-based format supported by yEd, Gephi, Cytoscape, etc.
#[wasm_bindgen]
pub fn export_graphml(source: &str) -> Result<String, JsValue> {
    let graph = parse_and_build(source)?;
    Ok(render::render_graphml(&graph))
}

// ============================================================================
// Dependencies
// ============================================================================

/// Extract all Salesforce org dependencies from AgentScript source.
#[wasm_bindgen]
pub fn extract_dependencies(source: &str) -> Result<JsValue, JsValue> {
    let agent = crate::parse(source).map_err(|errs| JsValue::from_str(&errs.join("\n")))?;

    let report = super::dependencies::extract_dependencies(&agent);
    serde_wasm_bindgen::to_value(&report)
        .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e)))
}

/// Check if a specific SObject is used in the source.
#[wasm_bindgen]
pub fn uses_sobject(source: &str, sobject_name: &str) -> Result<bool, JsValue> {
    let agent = crate::parse(source).map_err(|errs| JsValue::from_str(&errs.join("\n")))?;
    let report = super::dependencies::extract_dependencies(&agent);
    Ok(report.uses_sobject(sobject_name))
}

/// Check if a specific Flow is used in the source.
#[wasm_bindgen]
pub fn uses_flow(source: &str, flow_name: &str) -> Result<bool, JsValue> {
    let agent = crate::parse(source).map_err(|errs| JsValue::from_str(&errs.join("\n")))?;
    let report = super::dependencies::extract_dependencies(&agent);
    Ok(report.uses_flow(flow_name))
}

/// Check if a specific Apex class is used in the source.
#[wasm_bindgen]
pub fn uses_apex_class(source: &str, class_name: &str) -> Result<bool, JsValue> {
    let agent = crate::parse(source).map_err(|errs| JsValue::from_str(&errs.join("\n")))?;
    let report = super::dependencies::extract_dependencies(&agent);
    Ok(report.uses_apex_class(class_name))
}

// ============================================================================
// Utility
// ============================================================================

/// Get the version of the graph library.
#[wasm_bindgen]
pub fn graph_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ============================================================================
// Internal helpers
// ============================================================================

/// Parse source and build graph - common helper to reduce duplication.
fn parse_and_build(source: &str) -> Result<RefGraph, JsValue> {
    let agent = crate::parse(source).map_err(|errs| JsValue::from_str(&errs.join("\n")))?;

    RefGraph::from_ast(&agent)
        .map_err(|e| JsValue::from_str(&format!("Failed to build graph: {}", e)))
}
