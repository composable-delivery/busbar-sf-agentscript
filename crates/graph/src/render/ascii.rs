//! ASCII rendering for graph visualization.
//!
//! Provides terminal-friendly tree and diagram output for RefGraph structures.

use std::collections::{HashMap, HashSet};
use petgraph::visit::EdgeRef;
use crate::{RefGraph, RefNode};

/// Render the topic flow graph as ASCII art.
///
/// Shows how topics connect to each other via transitions, delegations, and routing.
pub fn render_topic_flow(graph: &RefGraph) -> String {
    let inner = graph.inner();

    // Collect topic names and edges
    let mut topics: Vec<String> = vec!["start_agent".to_string()];
    let mut topic_idx: HashMap<String, usize> = HashMap::new();
    topic_idx.insert("start_agent".to_string(), 0);

    for name in graph.topic_names() {
        topic_idx.insert(name.to_string(), topics.len());
        topics.push(name.to_string());
    }

    // Collect edges by source
    let mut edges: HashMap<usize, Vec<(usize, String)>> = HashMap::new();

    for edge in inner.edge_references() {
        let edge_type = edge.weight().label();
        if edge_type == "transitions_to" || edge_type == "delegates" || edge_type == "routes" {
            let source_node = graph.get_node(edge.source());
            let target_node = graph.get_node(edge.target());

            if let (Some(src), Some(tgt)) = (source_node, target_node) {
                let src_name = get_topic_name(src);
                let tgt_name = get_topic_name(tgt);

                if let (Some(&src_id), Some(&tgt_id)) =
                    (topic_idx.get(&src_name), topic_idx.get(&tgt_name))
                {
                    edges.entry(src_id).or_default().push((tgt_id, edge_type.to_string()));
                }
            }
        }
    }

    render_ascii_tree(&topics, &edges)
}

/// Render the actions graph as ASCII art.
///
/// Shows topics with their action definitions and reasoning actions.
pub fn render_actions_view(graph: &RefGraph) -> String {
    let inner = graph.inner();
    let mut labels: Vec<String> = Vec::new();
    let mut node_map: HashMap<usize, usize> = HashMap::new();

    for idx in inner.node_indices() {
        if let Some(node) = graph.get_node(idx) {
            let label = match node {
                RefNode::StartAgent { .. } => Some("start_agent".to_string()),
                RefNode::Topic { name, .. } => Some(format!("[{}]", name)),
                RefNode::ActionDef { name, .. } => Some(name.clone()),
                RefNode::ReasoningAction { name, target, .. } => {
                    if let Some(t) = target {
                        Some(format!("{}→{}", name, t.split("://").last().unwrap_or(t)))
                    } else {
                        Some(name.clone())
                    }
                }
                _ => None,
            };

            if let Some(lbl) = label {
                node_map.insert(idx.index(), labels.len());
                labels.push(lbl);
            }
        }
    }

    // Collect edges
    let mut edges: HashMap<usize, Vec<(usize, String)>> = HashMap::new();

    for edge in inner.edge_references() {
        let edge_type = edge.weight().label();
        if edge_type == "invokes" || edge_type == "transitions_to" || edge_type == "delegates" {
            if let (Some(&src_id), Some(&tgt_id)) = (
                node_map.get(&edge.source().index()),
                node_map.get(&edge.target().index()),
            ) {
                edges.entry(src_id).or_default().push((tgt_id, edge_type.to_string()));
            }
        }
    }

    render_ascii_tree(&labels, &edges)
}

/// Render a full structured view of the graph.
///
/// Shows a topic-centric view with variables, actions, and transitions.
pub fn render_full_view(graph: &RefGraph) -> String {
    let inner = graph.inner();
    let mut output = String::new();

    struct TopicInfo {
        actions: Vec<String>,
        reasoning: Vec<String>,
        transitions: Vec<String>,
        delegates: Vec<String>,
    }

    let mut topics: HashMap<String, TopicInfo> = HashMap::new();
    let mut start_routes: Vec<String> = Vec::new();
    let mut variables: Vec<(String, bool)> = Vec::new();

    // First pass: collect all nodes
    for idx in inner.node_indices() {
        if let Some(node) = graph.get_node(idx) {
            match node {
                RefNode::Variable { name, mutable, .. } => {
                    variables.push((name.clone(), *mutable));
                }
                RefNode::Topic { name, .. } => {
                    topics.entry(name.clone()).or_insert(TopicInfo {
                        actions: Vec::new(),
                        reasoning: Vec::new(),
                        transitions: Vec::new(),
                        delegates: Vec::new(),
                    });
                }
                RefNode::ActionDef { name, topic, .. } => {
                    if let Some(t) = topics.get_mut(topic) {
                        t.actions.push(name.clone());
                    }
                }
                RefNode::ReasoningAction { name, topic, target, .. } => {
                    if let Some(t) = topics.get_mut(topic) {
                        let desc = if let Some(tgt) = target {
                            format!("{} → {}", name, tgt.split("://").last().unwrap_or(tgt))
                        } else {
                            name.clone()
                        };
                        t.reasoning.push(desc);
                    }
                }
                _ => {}
            }
        }
    }

    // Second pass: collect edges for transitions
    for edge in inner.edge_references() {
        let edge_type = edge.weight().label();
        let source_node = graph.get_node(edge.source());
        let target_node = graph.get_node(edge.target());

        if let (Some(src), Some(tgt)) = (source_node, target_node) {
            match (src, tgt, edge_type) {
                (RefNode::StartAgent { .. }, RefNode::Topic { name, .. }, "routes") => {
                    start_routes.push(name.clone());
                }
                (RefNode::Topic { name: src_name, .. }, RefNode::Topic { name: tgt_name, .. }, "transitions_to") => {
                    if let Some(t) = topics.get_mut(src_name) {
                        if !t.transitions.contains(tgt_name) {
                            t.transitions.push(tgt_name.clone());
                        }
                    }
                }
                (RefNode::Topic { name: src_name, .. }, RefNode::Topic { name: tgt_name, .. }, "delegates") => {
                    if let Some(t) = topics.get_mut(src_name) {
                        if !t.delegates.contains(tgt_name) {
                            t.delegates.push(tgt_name.clone());
                        }
                    }
                }
                _ => {}
            }
        }
    }

    // Render output
    output.push_str("┌─────────────────────────────────────────────────────────────┐\n");
    output.push_str("│  AGENT EXECUTION FLOW                                       │\n");
    output.push_str("└─────────────────────────────────────────────────────────────┘\n\n");

    // Variables summary
    if !variables.is_empty() {
        output.push_str("VARIABLES:\n");
        let mutable: Vec<_> = variables.iter().filter(|(_, m)| *m).map(|(n, _)| n.as_str()).collect();
        let linked: Vec<_> = variables.iter().filter(|(_, m)| !*m).map(|(n, _)| n.as_str()).collect();
        if !mutable.is_empty() {
            output.push_str(&format!("  Mutable: {}\n", mutable.join(", ")));
        }
        if !linked.is_empty() {
            output.push_str(&format!("  Linked:  {}\n", linked.join(", ")));
        }
        output.push('\n');
    }

    // Entry point
    output.push_str("ENTRY POINT:\n");
    output.push_str("  start_agent\n");
    if !start_routes.is_empty() {
        output.push_str(&format!("    routes to: {}\n", start_routes.join(", ")));
    }
    output.push('\n');

    // Topics
    output.push_str("TOPICS:\n");
    for (name, info) in &topics {
        output.push_str(&format!("\n  ┌─ {} ─────────────────────────────\n", name));

        if !info.actions.is_empty() {
            output.push_str("  │ Actions:\n");
            for action in &info.actions {
                output.push_str(&format!("  │   • {}\n", action));
            }
        }

        if !info.reasoning.is_empty() {
            output.push_str("  │ Reasoning:\n");
            for r in &info.reasoning {
                output.push_str(&format!("  │   ◆ {}\n", r));
            }
        }

        if !info.transitions.is_empty() {
            output.push_str(&format!("  │ Transitions → {}\n", info.transitions.join(", ")));
        }

        if !info.delegates.is_empty() {
            output.push_str(&format!("  │ Delegates ⇒ {}\n", info.delegates.join(", ")));
        }

        output.push_str("  └────────────────────────────────────────\n");
    }

    output
}

/// Render nodes and edges as an ASCII tree structure.
pub fn render_ascii_tree(
    labels: &[String],
    edges: &HashMap<usize, Vec<(usize, String)>>,
) -> String {
    let mut output = String::new();
    let mut visited: HashSet<usize> = HashSet::new();

    // Find root nodes (nodes with no incoming edges)
    let mut has_incoming: HashSet<usize> = HashSet::new();
    for targets in edges.values() {
        for (target, _) in targets {
            has_incoming.insert(*target);
        }
    }

    let roots: Vec<usize> = (0..labels.len())
        .filter(|i| !has_incoming.contains(i))
        .collect();

    // If no roots found (everything has incoming), start from node 0
    let start_nodes = if roots.is_empty() {
        vec![0]
    } else {
        roots
    };

    for (i, &root) in start_nodes.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        render_node(
            &mut output,
            labels,
            edges,
            root,
            "",
            true,
            &mut visited,
        );
    }

    output
}

fn render_node(
    output: &mut String,
    labels: &[String],
    edges: &HashMap<usize, Vec<(usize, String)>>,
    node: usize,
    prefix: &str,
    is_last: bool,
    visited: &mut HashSet<usize>,
) {
    let connector = if prefix.is_empty() {
        ""
    } else if is_last {
        "└── "
    } else {
        "├── "
    };

    let label = labels.get(node).map(|s| s.as_str()).unwrap_or("?");

    // Check if this is a back-edge (cycle)
    if visited.contains(&node) {
        output.push_str(prefix);
        output.push_str(connector);
        output.push_str(label);
        output.push_str(" ↩\n");
        return;
    }

    output.push_str(prefix);
    output.push_str(connector);
    output.push_str(label);
    output.push('\n');

    visited.insert(node);

    if let Some(children) = edges.get(&node) {
        let new_prefix = if prefix.is_empty() {
            "".to_string()
        } else if is_last {
            format!("{}    ", prefix)
        } else {
            format!("{}│   ", prefix)
        };

        for (i, (child, edge_type)) in children.iter().enumerate() {
            let child_is_last = i == children.len() - 1;

            // Show edge type for non-trivial edges
            let edge_indicator = match edge_type.as_str() {
                "transitions_to" => "→ ",
                "delegates" => "⇒ ",
                "routes" => "⊳ ",
                "invokes" => "◆ ",
                "reads" => "◇ ",
                "writes" => "◈ ",
                _ => "",
            };

            if !edge_indicator.is_empty() {
                output.push_str(&new_prefix);
                output.push_str(if child_is_last { "└" } else { "├" });
                output.push_str(edge_indicator);

                let child_label = labels.get(*child).map(|s| s.as_str()).unwrap_or("?");
                if visited.contains(child) {
                    output.push_str(child_label);
                    output.push_str(" ↩\n");
                } else {
                    output.push_str(child_label);
                    output.push('\n');

                    // Recurse for this child's children
                    let deeper_prefix = if child_is_last {
                        format!("{}    ", new_prefix)
                    } else {
                        format!("{}│   ", new_prefix)
                    };

                    visited.insert(*child);
                    if let Some(grandchildren) = edges.get(child) {
                        for (j, (grandchild, _gc_edge)) in grandchildren.iter().enumerate() {
                            let gc_is_last = j == grandchildren.len() - 1;
                            render_node(output, labels, edges, *grandchild, &deeper_prefix, gc_is_last, visited);
                        }
                    }
                }
            } else {
                render_node(output, labels, edges, *child, &new_prefix, child_is_last, visited);
            }
        }
    }
}

fn get_topic_name(node: &RefNode) -> String {
    match node {
        RefNode::StartAgent { .. } => "start_agent".to_string(),
        RefNode::Topic { name, .. } => name.clone(),
        _ => "unknown".to_string(),
    }
}
