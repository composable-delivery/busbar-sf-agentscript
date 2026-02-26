//! # busbar-sf-agentscript
//!
//! AgentScript parser, graph analysis, and LSP for Salesforce Agentforce.
//!
//! ## Features
//!
//! | Feature | Crate | Description |
//! |---------|-------|-------------|
//! | `parser` | [`busbar-sf-agentscript-parser`] | Lexer, AST, serializer, validator |
//! | `graph`  | [`busbar-sf-agentscript-graph`]  | Reference graph, cycle detection, dead code |
//!
//! `default = ["full"]` enables all of the above.
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! busbar-sf-agentscript = "0.1"
//! ```
//!
//! ```rust,ignore
//! use busbar_sf_agentscript::parse;
//! use busbar_sf_agentscript::graph::RefGraph;
//!
//! let ast = parse(source).unwrap();
//! let graph = RefGraph::from_ast(&ast).unwrap();
//! println!("{} topics", graph.topic_count());
//! ```

#[cfg(feature = "parser")]
pub use busbar_sf_agentscript_parser as parser;

#[cfg(feature = "parser")]
pub use busbar_sf_agentscript_parser::{parse, serialize, validate_ast, AgentFile, Spanned};

#[cfg(feature = "graph")]
pub use busbar_sf_agentscript_graph as graph;
