//! Graph rendering utilities.
//!
//! This module provides various output formats for visualizing RefGraph structures:
//! - ASCII tree rendering for terminal display
//! - GraphML export for external visualization tools

mod ascii;
mod graphml;

pub use ascii::{render_topic_flow, render_actions_view, render_full_view, render_ascii_tree};
pub use graphml::render_graphml;
