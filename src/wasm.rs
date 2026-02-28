//! WebAssembly bindings for the AgentScript parser.
//!
//! This module provides JavaScript-accessible functions for parsing
//! AgentScript source code and working with the resulting AST.
//!
//! # Example (JavaScript)
//!
//! ```javascript
//! import init, { parse_agent, parse_agent_to_json, serialize_agent } from './sf_agentscript.js';
//!
//! await init();
//!
//! // Parse to JS object
//! const ast = parse_agent(source);
//! console.log(ast.config.agent_name);
//!
//! // Or parse to JSON string
//! const json = parse_agent_to_json(source);
//!
//! // Serialize AST back to source
//! const regenerated = serialize_agent(ast);
//! ```

use crate::validation::Severity;
use serde::Serialize;
use wasm_bindgen::prelude::*;

#[derive(Serialize)]
struct ValidationReport {
    errors: Vec<crate::validation::SemanticError>,
    warnings: Vec<crate::validation::SemanticError>,
}

/// Initialize panic hook for better error messages in the browser console.
#[wasm_bindgen(start)]
pub fn init() {
    #[cfg(feature = "wasm")]
    console_error_panic_hook::set_once();
}

/// Parse AgentScript source code and return the AST as a JavaScript object.
///
/// # Arguments
/// * `source` - The AgentScript source code to parse
///
/// # Returns
/// * `Ok(JsValue)` - The parsed AST as a JavaScript object
/// * `Err(JsValue)` - Error message if parsing fails
#[wasm_bindgen]
pub fn parse_agent(source: &str) -> Result<JsValue, JsValue> {
    match crate::parse(source) {
        Ok(ast) => serde_wasm_bindgen::to_value(&ast)
            .map_err(|e| JsValue::from_str(&format!("Serialization error: {}", e))),
        Err(errs) => Err(JsValue::from_str(&errs.join("\n"))),
    }
}

/// Parse AgentScript source code and return the AST as a JSON string.
///
/// This is useful when you need to pass the AST to another system
/// or want to inspect it as text.
///
/// # Arguments
/// * `source` - The AgentScript source code to parse
///
/// # Returns
/// * `Ok(String)` - The parsed AST as a JSON string
/// * `Err(JsValue)` - Error message if parsing fails
#[wasm_bindgen]
pub fn parse_agent_to_json(source: &str) -> Result<String, JsValue> {
    match crate::parse(source) {
        Ok(ast) => serde_json::to_string_pretty(&ast)
            .map_err(|e| JsValue::from_str(&format!("JSON serialization error: {}", e))),
        Err(errs) => Err(JsValue::from_str(&errs.join("\n"))),
    }
}

/// Validate AgentScript source code without returning the full AST.
///
/// Returns `true` if the source is valid, or throws an error with
/// the parse errors if invalid.
///
/// # Arguments
/// * `source` - The AgentScript source code to validate
///
/// # Returns
/// * `Ok(true)` - The source is valid AgentScript
/// * `Err(JsValue)` - Error messages if parsing fails
#[wasm_bindgen]
pub fn validate_agent(source: &str) -> Result<bool, JsValue> {
    match crate::parse(source) {
        Ok(ast) => {
            let issues = crate::validate_ast(&ast);
            let errors: Vec<String> = issues
                .into_iter()
                .filter(|i| i.severity == Severity::Error)
                .map(|i| i.message)
                .collect();

            if errors.is_empty() {
                Ok(true)
            } else {
                Err(JsValue::from_str(&errors.join("\n")))
            }
        }
        Err(errs) => Err(JsValue::from_str(&errs.join("\n"))),
    }
}

/// Validate AgentScript source code and return structured errors and warnings.
///
/// # Arguments
/// * `source` - The AgentScript source code to validate
///
/// # Returns
/// * `Ok(JsValue)` - Object with `errors` and `warnings` arrays
/// * `Err(JsValue)` - Error message if serialization fails
#[wasm_bindgen]
pub fn validate_agent_semantic(source: &str) -> Result<JsValue, JsValue> {
    let (errors, warnings) = match crate::parse(source) {
        Ok(ast) => {
            let issues = crate::validate_ast(&ast);
            issues
                .into_iter()
                .partition(|i| i.severity == Severity::Error)
        }
        Err(parse_errs) => {
            let errors = parse_errs
                .into_iter()
                .map(|msg| crate::validation::SemanticError {
                    message: msg,
                    span: None,
                    severity: Severity::Error,
                    hint: None,
                })
                .collect();
            (errors, vec![])
        }
    };

    let report = ValidationReport { errors, warnings };
    serde_wasm_bindgen::to_value(&report).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Get the version of the parser.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Serialize an AST back to AgentScript source code.
///
/// Takes a JavaScript object representing an AST (as returned by `parse_agent`)
/// and converts it back to AgentScript source code.
///
/// # Arguments
/// * `ast` - The AST as a JavaScript object
///
/// # Returns
/// * `Ok(String)` - The serialized AgentScript source code
/// * `Err(JsValue)` - Error message if serialization fails
#[wasm_bindgen]
pub fn serialize_agent(ast: JsValue) -> Result<String, JsValue> {
    let agent: crate::ast::AgentFile = serde_wasm_bindgen::from_value(ast)
        .map_err(|e| JsValue::from_str(&format!("Failed to deserialize AST: {}", e)))?;
    Ok(crate::serialize(&agent))
}

/// Parse AgentScript source, then serialize it back.
///
/// This is useful for formatting/normalizing AgentScript code.
///
/// # Arguments
/// * `source` - The AgentScript source code to parse and reserialize
///
/// # Returns
/// * `Ok(String)` - The normalized AgentScript source code
/// * `Err(JsValue)` - Error message if parsing or serialization fails
#[wasm_bindgen]
pub fn normalize_agent(source: &str) -> Result<String, JsValue> {
    match crate::parse(source) {
        Ok(ast) => Ok(crate::serialize(&ast)),
        Err(errs) => Err(JsValue::from_str(&errs.join("\n"))),
    }
}
