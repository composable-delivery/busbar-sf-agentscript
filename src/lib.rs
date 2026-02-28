#![recursion_limit = "512"]

//! # busbar-sf-agentscript
//!
//! A Rust parser for Salesforce's AgentScript language with graph analysis and WebAssembly support.
//!
//! ## Feature Flags
//!
//! - `graph` - Enable graph analysis, validation, and rendering (brings in `petgraph`)
//! - `wasm` - Enable WebAssembly bindings for browser use
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use busbar_sf_agentscript::{parse, AgentFile};
//!
//! let ast = parse(source).unwrap();
//!
//! // With the `graph` feature:
//! use busbar_sf_agentscript::graph::RefGraph;
//! let graph = RefGraph::from_ast(&ast).unwrap();
//! println!("{} topics", graph.node_count());
//! ```

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod serializer;
pub mod validation;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "graph")]
pub mod graph;

// Re-export commonly used types
pub use ast::{AgentFile, Expr, Reference, Spanned, Type};
pub use error::{AgentScriptError, ErrorReporter};
pub use parser::{parse, parse_with_structured_errors};
pub use serializer::serialize;
pub use validation::validate_ast;

/// Parse AgentScript source code into an AST.
pub fn parse_source(source: &str) -> Result<AgentFile, Vec<String>> {
    let (result, errors) = parser::parse_with_errors(source);
    if !errors.is_empty() {
        return Err(errors);
    }
    result.ok_or_else(|| vec!["Unknown parse error".to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::prelude::*;

    #[test]
    fn test_parse_minimal() {
        let source = r#"
config:
   agent_name: "Test"
"#;
        let result = parse_source(source);
        assert!(result.is_ok());
    }

    #[test]
    fn test_lexer_integration() {
        let source = r#"@variables.user_id != """#;
        let tokens = lexer::lexer().parse(source);
        assert!(tokens.has_output());
    }
}
