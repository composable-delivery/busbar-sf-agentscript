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

# flags.file.summary

Path to the AgentScript file to visualize.

# flags.file.description

The AgentScript (.agent) file to render as a graph.

# flags.view.summary

Graph view type (topics, actions, full).

# flags.view.description

Controls what level of detail to show. Use 'topics' for high-level topic transitions, 'actions' for topic and action nodes, or 'full' for all nodes including variables and connections.

# flags.format.summary

Output format (ascii or graphml).

# flags.format.description

Output format for the graph. Use 'ascii' for colored terminal output, or 'graphml' for XML export compatible with yEd, Gephi, Cytoscape, and other graph visualization tools.

# error.graphFailure

Failed to render graph: %s
