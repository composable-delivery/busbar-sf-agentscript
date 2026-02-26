//! GraphML export for graph visualization.
//!
//! GraphML is an XML-based format for graph exchange that is widely supported
//! by graph visualization tools like yEd, Gephi, Cytoscape, etc.

use crate::{RefGraph, RefNode};
use petgraph::visit::EdgeRef;
use std::fmt::Write;

type NodeAttrs<'a> = (
    &'static str,
    Option<&'a str>,
    Option<&'a str>,
    Option<&'a str>,
    Option<bool>,
    (usize, usize),
);

/// Render a RefGraph as GraphML XML.
///
/// The output includes:
/// - Node attributes: node_type, name, topic, target, mutable, span
/// - Edge attributes: edge_type
/// - yEd-compatible metadata keys
pub fn render_graphml(graph: &RefGraph) -> String {
    let inner = graph.inner();
    let mut output = String::new();

    // XML header and GraphML schema
    writeln!(output, r#"<?xml version="1.0" encoding="UTF-8"?>"#).unwrap();
    writeln!(output, r#"<graphml xmlns="http://graphml.graphdrawing.org/xmlns""#).unwrap();
    writeln!(output, r#"         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance""#).unwrap();
    writeln!(output, r#"         xsi:schemaLocation="http://graphml.graphdrawing.org/xmlns"#)
        .unwrap();
    writeln!(output, r#"         http://graphml.graphdrawing.org/xmlns/1.0/graphml.xsd">"#)
        .unwrap();

    // Define node attributes
    writeln!(
        output,
        r#"  <key id="node_type" for="node" attr.name="node_type" attr.type="string"/>"#
    )
    .unwrap();
    writeln!(output, r#"  <key id="name" for="node" attr.name="name" attr.type="string"/>"#)
        .unwrap();
    writeln!(output, r#"  <key id="topic" for="node" attr.name="topic" attr.type="string"/>"#)
        .unwrap();
    writeln!(
        output,
        r#"  <key id="target" for="node" attr.name="target" attr.type="string"/>"#
    )
    .unwrap();
    writeln!(
        output,
        r#"  <key id="mutable" for="node" attr.name="mutable" attr.type="boolean"/>"#
    )
    .unwrap();
    writeln!(
        output,
        r#"  <key id="span_start" for="node" attr.name="span_start" attr.type="int"/>"#
    )
    .unwrap();
    writeln!(
        output,
        r#"  <key id="span_end" for="node" attr.name="span_end" attr.type="int"/>"#
    )
    .unwrap();
    writeln!(output, r#"  <key id="label" for="node" attr.name="label" attr.type="string"/>"#)
        .unwrap();

    // Define edge attributes
    writeln!(
        output,
        r#"  <key id="edge_type" for="edge" attr.name="edge_type" attr.type="string"/>"#
    )
    .unwrap();

    // yEd-specific: node graphics (optional, for better visualization)
    writeln!(output, r#"  <key id="nodegraphics" for="node" yfiles.type="nodegraphics"/>"#)
        .unwrap();
    writeln!(output, r#"  <key id="edgegraphics" for="edge" yfiles.type="edgegraphics"/>"#)
        .unwrap();

    // Start graph
    writeln!(output, r#"  <graph id="G" edgedefault="directed">"#).unwrap();

    // Output nodes
    for idx in inner.node_indices() {
        if let Some(node) = graph.get_node(idx) {
            let id = idx.index();
            let (node_type, name, topic, target, mutable, span) = extract_node_attrs(node);

            let label = node.label();
            let escaped_label = escape_xml(&label);

            writeln!(output, r#"    <node id="n{}">"#, id).unwrap();
            writeln!(output, r#"      <data key="node_type">{}</data>"#, node_type).unwrap();
            writeln!(output, r#"      <data key="label">{}</data>"#, escaped_label).unwrap();

            if let Some(n) = name {
                writeln!(output, r#"      <data key="name">{}</data>"#, escape_xml(n)).unwrap();
            }
            if let Some(t) = topic {
                writeln!(output, r#"      <data key="topic">{}</data>"#, escape_xml(t)).unwrap();
            }
            if let Some(tgt) = target {
                writeln!(output, r#"      <data key="target">{}</data>"#, escape_xml(tgt)).unwrap();
            }
            if let Some(m) = mutable {
                writeln!(output, r#"      <data key="mutable">{}</data>"#, m).unwrap();
            }
            writeln!(output, r#"      <data key="span_start">{}</data>"#, span.0).unwrap();
            writeln!(output, r#"      <data key="span_end">{}</data>"#, span.1).unwrap();

            writeln!(output, r#"    </node>"#).unwrap();
        }
    }

    // Output edges
    for (edge_id, edge) in inner.edge_references().enumerate() {
        let source = edge.source().index();
        let target = edge.target().index();
        let edge_type = edge.weight().label();

        writeln!(
            output,
            r#"    <edge id="e{}" source="n{}" target="n{}">"#,
            edge_id, source, target
        )
        .unwrap();
        writeln!(output, r#"      <data key="edge_type">{}</data>"#, edge_type).unwrap();
        writeln!(output, r#"    </edge>"#).unwrap();
    }

    // Close graph and graphml
    writeln!(output, r#"  </graph>"#).unwrap();
    writeln!(output, r#"</graphml>"#).unwrap();

    output
}

/// Extract node attributes for GraphML output.
fn extract_node_attrs(node: &RefNode) -> NodeAttrs<'_> {
    match node {
        RefNode::StartAgent { span } => ("start_agent", None, None, None, None, *span),
        RefNode::Topic { name, span } => ("topic", Some(name.as_str()), None, None, None, *span),
        RefNode::ActionDef { name, topic, span } => {
            ("action_def", Some(name.as_str()), Some(topic.as_str()), None, None, *span)
        }
        RefNode::ReasoningAction {
            name,
            topic,
            target,
            span,
        } => (
            "reasoning_action",
            Some(name.as_str()),
            Some(topic.as_str()),
            target.as_deref(),
            None,
            *span,
        ),
        RefNode::Variable {
            name,
            mutable,
            span,
        } => ("variable", Some(name.as_str()), None, None, Some(*mutable), *span),
        RefNode::Connection { name, span } => {
            ("connection", Some(name.as_str()), None, None, None, *span)
        }
    }
}

/// Escape special XML characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("hello"), "hello");
        assert_eq!(escape_xml("<tag>"), "&lt;tag&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
        assert_eq!(escape_xml(r#"say "hello""#), "say &quot;hello&quot;");
    }
}
