#![recursion_limit = "512"]

//! # sf-agentscript
//!
//! A Rust parser for Salesforce's AgentScript language with WebAssembly support.
//!
//! AgentScript is an indentation-sensitive, YAML-like language for defining
//! AI agent behavior in Salesforce Agentforce. This crate provides:
//!
//! - **Complete parsing** of AgentScript source into a typed AST
//! - **Span tracking** for precise error reporting and IDE integration
//! - **WASM bindings** for browser-based parsing and tooling
//! - **JSON serialization** for interoperability with other tools
//!
//! ## Quick Start
//!
//! Parse an AgentScript file and inspect the result:
//!
//! ```rust
//! use busbar_sf_agentscript_parser::{parse, AgentFile};
//!
//! let source = r#"
//! config:
//!    agent_name: "MyAgent"
//!    description: "A helpful assistant"
//!
//! system:
//!    instructions: "Be helpful and concise."
//!    messages:
//!       welcome: "Hello! How can I help?"
//!       error: "Sorry, something went wrong."
//!
//! variables:
//!    user_name: mutable string = ""
//!       description: "The user's name"
//!
//! start_agent topic_selector:
//!    description: "Route to appropriate topic"
//!    reasoning:
//!       instructions:|
//!          Determine the best topic for the user's request.
//!       actions:
//!          go_main: @utils.transition to @topic.main
//!             description: "Go to main topic"
//!
//! topic main:
//!    description: "Main conversation topic"
//!    reasoning:
//!       instructions:|
//!          Help the user with their request.
//! "#;
//!
//! match busbar_sf_agentscript_parser::parse(source) {
//!     Ok(agent) => {
//!         println!("Agent: {:?}", agent.config.map(|c| c.node.agent_name.node));
//!         println!("Topics: {}", agent.topics.len());
//!     }
//!     Err(errors) => {
//!         for err in errors {
//!             eprintln!("Parse error: {}", err);
//!         }
//!     }
//! }
//! ```
//!
//! ## AgentScript Language Overview
//!
//! AgentScript uses **3-space indentation** and has these main blocks:
//!
//! | Block | Purpose |
//! |-------|---------|
//! | `config:` | Agent metadata (name, description) |
//! | `system:` | Global instructions and messages |
//! | `variables:` | State variables (mutable or linked) |
//! | `start_agent:` | Entry point with initial routing |
//! | `topic:` | Conversation topics with reasoning and actions |
//! | `actions:` | Action definitions with inputs/outputs |
//!
//! ### Example Blocks
//!
//! **Variables with types:**
//! ```text
//! variables:
//!    customer_id: mutable string = ""
//!       description: "Customer identifier"
//!    order_total: linked number
//!       source: @context.order.total
//! ```
//!
//! **Topic with reasoning and actions:**
//! ```text
//! topic support:
//!    description: "Handle support requests"
//!    reasoning:
//!       instructions: ->
//!          | Help the customer resolve their issue.
//!          if @variables.is_premium:
//!             | Provide priority support.
//!       actions:
//!          lookup_order: @actions.lookup_order
//!             with order_id = @variables.order_id
//!             set @variables.order_status = @outputs.status
//! ```
//!
//! ## Module Overview
//!
//! - [`ast`] - Abstract Syntax Tree types representing parsed AgentScript ([AST Reference](https://github.com/composable-delivery/sf-agentscript/blob/main/AST_REFERENCE.md))
//! - [`lexer`] - Lexical analysis with indentation-aware tokenization
//! - [`parser`] - Recursive descent parser using chumsky combinators
//! - [`error`] - Error types with pretty printing via ariadne
//!
//! ## Feature Flags
//!
//! - `wasm` - Enable WebAssembly bindings for browser use
//!
//! ## JSON Serialization
//!
//! All AST types implement `Serialize` and `Deserialize`:
//!
//! ```rust
//! # use busbar_sf_agentscript_parser::parse;
//! let source = r#"config:
//!    agent_name: "Test"
//! "#;
//!
//! if let Ok(agent) = parse(source) {
//!     let json = serde_json::to_string_pretty(&agent).unwrap();
//!     println!("{}", json);
//! }
//! ```
//!
//! ## Error Handling
//!
//! The parser returns detailed error information including source spans:
//!
//! ```rust
//! use busbar_sf_agentscript_parser::{parse_source, ErrorReporter};
//!
//! let source = "config:\n   agent_name: missing_quotes";
//!
//! match parse_source(source) {
//!     Ok(agent) => println!("Parsed successfully"),
//!     Err(errors) => {
//!         for err in &errors {
//!             eprintln!("{}", err);
//!         }
//!     }
//! }
//! ```

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod serializer;
pub mod validation;

#[cfg(feature = "wasm")]
pub mod wasm;

// Re-export commonly used types
pub use ast::{AgentFile, Expr, Reference, Spanned, Type};
pub use error::{AgentScriptError, ErrorReporter};
pub use parser::{parse, parse_with_structured_errors};
pub use serializer::serialize;
pub use validation::validate_ast;

/// Parse AgentScript source code into an AST.
///
/// This is a convenience function that wraps the parser module.
///
/// # Example
///
/// ```rust
/// let source = r#"
/// config:
///    agent_name: "Test"
/// "#;
///
/// match busbar_sf_agentscript_parser::parse_source(source) {
///     Ok(agent) => println!("Parsed agent: {:?}", agent.config),
///     Err(errors) => {
///         for err in errors {
///             eprintln!("Error: {}", err);
///         }
///     }
/// }
/// ```
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
