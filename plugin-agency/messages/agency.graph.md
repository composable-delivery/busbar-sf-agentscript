# summary

Render a graph visualization of an AgentScript file.

# description

Parses an AgentScript file and renders its execution flow as a graph. By default, renders colored ASCII art in the terminal. Use --format graphml to export GraphML XML for visualization in external tools like yEd, Gephi, or Cytoscape.

# examples

- Show topic flow (transitions between topics): `sf aslab graph -f agent.agent`
- Show all actions within topics: `sf aslab graph -f agent.agent --view actions`
- Show full graph with all nodes: `sf aslab graph -f agent.agent --view full`
- Export as GraphML for external visualization: `sf aslab graph -f agent.agent --format graphml`
- Export GraphML to a file: `sf aslab graph -f agent.agent --format graphml > agent-graph.graphml`
- Export as Mermaid diagram (pipe-friendly): `sf aslab graph -f agent.agent --format mermaid > graph.md`
- Export as self-contained HTML page: `sf aslab graph -f agent.agent --format html > graph.html`
- Show graph with statistics: `sf aslab graph -f agent.agent --stats`

# flags.file.summary

Path to the AgentScript file to visualize.

# flags.file.description

The AgentScript (.agent) file to render as a graph.

# flags.view.summary

Graph view type (topics, actions, full).

# flags.view.description

Controls what level of detail to show. Use 'topics' for high-level topic transitions, 'actions' for topic and action nodes, or 'full' for all nodes including variables and connections.

# flags.format.summary

Output format (ascii, graphml, mermaid, or html).

# flags.format.description

Output format for the graph. Use 'ascii' for colored terminal output, 'graphml' for XML export, 'mermaid' for Mermaid flowchart syntax (pipe to a .md file), or 'html' for a self-contained HTML page with embedded Mermaid diagram.

# flags.stats.summary

Show graph statistics after rendering.

# flags.stats.description

Display a summary of graph statistics (node counts, edge counts, topic count, etc.) after the main output.

# error.graphFailure

Failed to render graph: %s
