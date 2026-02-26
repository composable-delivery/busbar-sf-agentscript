/**
 * Graph Webview Provider
 *
 * Renders an interactive DAG visualization of the AgentScript topic flow
 * in a VS Code webview panel. Uses D3.js and dagre for layout.
 *
 * The graph data comes from the LSP via a custom request that returns
 * the GraphRepr JSON from the Rust graph crate.
 */

import * as vscode from "vscode";
import type { LanguageClient } from "vscode-languageclient/node";

/** Mirrors GraphRepr from the Rust graph crate */
interface GraphRepr {
  nodes: NodeRepr[];
  edges: EdgeRepr[];
  topics: string[];
  variables: string[];
}

interface NodeRepr {
  node_type: string;
  name: string | null;
  topic: string | null;
  target: string | null;
  mutable: boolean | null;
  span_start: number;
  span_end: number;
}

interface EdgeRepr {
  source: number;
  target: number;
  edge_type: string;
}

export class GraphWebviewProvider {
  private panel: vscode.WebviewPanel | undefined;
  private readonly extensionUri: vscode.Uri;
  private client: LanguageClient | undefined;
  private disposables: vscode.Disposable[] = [];

  constructor(extensionUri: vscode.Uri) {
    this.extensionUri = extensionUri;
  }

  setClient(client: LanguageClient): void {
    this.client = client;
  }

  async show(): Promise<void> {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "agentscript") {
      vscode.window.showWarningMessage(
        "Open an AgentScript (.agent) file first.",
      );
      return;
    }

    if (this.panel) {
      this.panel.reveal(vscode.ViewColumn.Beside);
    } else {
      this.panel = vscode.window.createWebviewPanel(
        "agentscriptGraph",
        "AgentScript: Topic Graph",
        vscode.ViewColumn.Beside,
        {
          enableScripts: true,
          retainContextWhenHidden: true,
        },
      );

      this.panel.onDidDispose(
        () => {
          this.panel = undefined;
          this.disposables.forEach((d) => d.dispose());
          this.disposables = [];
        },
        null,
        this.disposables,
      );

      // Handle messages from webview (click-to-navigate)
      this.panel.webview.onDidReceiveMessage(
        (msg) => this.handleWebviewMessage(msg),
        null,
        this.disposables,
      );
    }

    await this.updateGraph(editor.document);

    // Re-render on document change
    const changeListener = vscode.workspace.onDidChangeTextDocument((e) => {
      if (
        this.panel &&
        e.document.languageId === "agentscript" &&
        e.contentChanges.length > 0
      ) {
        this.updateGraph(e.document);
      }
    });
    this.disposables.push(changeListener);

    // Re-render when switching editors
    const editorListener = vscode.window.onDidChangeActiveTextEditor((e) => {
      if (this.panel && e && e.document.languageId === "agentscript") {
        this.updateGraph(e.document);
      }
    });
    this.disposables.push(editorListener);
  }

  private async updateGraph(document: vscode.TextDocument): Promise<void> {
    if (!this.panel) return;

    const graphData = await this.getGraphData(document);
    if (!graphData) {
      this.panel.webview.html = this.getErrorHtml(
        "Failed to parse agent file. Check for syntax errors.",
      );
      return;
    }

    this.panel.webview.html = this.getWebviewHtml(graphData);
  }

  private async getGraphData(
    document: vscode.TextDocument,
  ): Promise<GraphRepr | null> {
    if (!this.client) return null;

    try {
      const result = await this.client.sendRequest(
        "agentscript/getGraph",
        { uri: document.uri.toString() },
      );
      return result as GraphRepr;
    } catch {
      // LSP request not available yet — fall back to null
      return null;
    }
  }

  private handleWebviewMessage(msg: {
    type: string;
    spanStart?: number;
    spanEnd?: number;
  }): void {
    if (msg.type === "navigateToSpan" && msg.spanStart !== undefined) {
      const editor = vscode.window.activeTextEditor;
      if (!editor || editor.document.languageId !== "agentscript") return;

      const startPos = editor.document.positionAt(msg.spanStart);
      const endPos = editor.document.positionAt(msg.spanEnd ?? msg.spanStart);
      const range = new vscode.Range(startPos, endPos);
      editor.revealRange(range, vscode.TextEditorRevealType.InCenter);
      editor.selection = new vscode.Selection(startPos, startPos);
    }
  }

  private getErrorHtml(message: string): string {
    return `<!DOCTYPE html>
<html><body style="display:flex;align-items:center;justify-content:center;height:100vh;font-family:var(--vscode-font-family);color:var(--vscode-errorForeground);">
<p>${this.escapeHtml(message)}</p>
</body></html>`;
  }

  private escapeHtml(text: string): string {
    return text
      .replace(/&/g, "&amp;")
      .replace(/</g, "&lt;")
      .replace(/>/g, "&gt;")
      .replace(/"/g, "&quot;");
  }

  private getWebviewHtml(graph: GraphRepr): string {
    const graphJson = JSON.stringify(graph);
    return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>AgentScript Topic Graph</title>
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    background: var(--vscode-editor-background, #1e1e1e);
    color: var(--vscode-editor-foreground, #d4d4d4);
    font-family: var(--vscode-font-family, 'Segoe UI', sans-serif);
    font-size: var(--vscode-font-size, 13px);
    overflow: hidden;
    width: 100vw;
    height: 100vh;
  }
  #toolbar {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 12px;
    background: var(--vscode-titleBar-activeBackground, #3c3c3c);
    border-bottom: 1px solid var(--vscode-panel-border, #444);
  }
  #toolbar button {
    background: var(--vscode-button-background, #0e639c);
    color: var(--vscode-button-foreground, #fff);
    border: none;
    border-radius: 3px;
    padding: 4px 10px;
    cursor: pointer;
    font-size: 12px;
  }
  #toolbar button:hover {
    background: var(--vscode-button-hoverBackground, #1177bb);
  }
  #toolbar .stats {
    margin-left: auto;
    opacity: 0.7;
    font-size: 11px;
  }
  svg {
    width: 100%;
    height: calc(100vh - 34px);
  }
  .node-group { cursor: pointer; }
  .node-group:hover rect { stroke-width: 2.5; }
  .node-rect {
    rx: 6; ry: 6;
    stroke-width: 1.5;
  }
  .node-label {
    fill: var(--vscode-editor-foreground, #d4d4d4);
    font-size: 12px;
    font-weight: 600;
    text-anchor: middle;
    dominant-baseline: central;
    pointer-events: none;
  }
  .node-sublabel {
    fill: var(--vscode-descriptionForeground, #999);
    font-size: 10px;
    text-anchor: middle;
    dominant-baseline: central;
    pointer-events: none;
  }
  .edge-line {
    fill: none;
    stroke-width: 1.5;
  }
  .edge-arrow {
    fill: var(--vscode-editor-foreground, #d4d4d4);
  }
  .edge-label {
    fill: var(--vscode-descriptionForeground, #999);
    font-size: 9px;
    text-anchor: middle;
  }
  /* Node type colors */
  .node-start_agent rect { fill: #2d7d46; stroke: #3da85a; }
  .node-topic rect { fill: #1a5276; stroke: #2980b9; }
  .node-action_def rect { fill: #7d3c98; stroke: #a569bd; }
  .node-reasoning_action rect { fill: #b7950b; stroke: #d4ac0d; }
  .node-variable rect { fill: #6c3483; stroke: #8e44ad; }
  .node-connection rect { fill: #a04000; stroke: #d35400; }
  /* Edge type colors */
  .edge-transition { stroke: #2ecc71; }
  .edge-delegates_to { stroke: #3498db; stroke-dasharray: 6,3; }
  .edge-escalates { stroke: #e74c3c; }
  .edge-reads { stroke: #9b59b6; opacity: 0.5; }
  .edge-writes { stroke: #e67e22; opacity: 0.5; }
  .edge-invokes { stroke: #1abc9c; }
  .edge-default { stroke: #95a5a6; }
</style>
</head>
<body>
<div id="toolbar">
  <button onclick="resetZoom()">Reset View</button>
  <button onclick="toggleVars()">Toggle Variables</button>
  <span class="stats" id="stats"></span>
</div>
<svg id="graph"></svg>

<script>
const vscode = acquireVsCodeApi();
const graph = ${graphJson};
let showVars = false;

// Simple dagre-like layout algorithm (no external dependency)
function layoutGraph(graph, showVariables) {
  const LAYER_GAP = 120;
  const NODE_GAP = 40;
  const NODE_W = 180;
  const NODE_H = 50;

  // Filter nodes: only show topic-level nodes (start_agent, topic, connection) + optionally variables
  const visibleTypes = new Set(['start_agent', 'topic', 'connection']);
  if (showVariables) visibleTypes.add('variable');

  const visibleNodes = graph.nodes
    .map((n, i) => ({ ...n, originalIndex: i }))
    .filter(n => visibleTypes.has(n.node_type));

  const nodeIndexMap = new Map();
  visibleNodes.forEach((n, i) => nodeIndexMap.set(n.originalIndex, i));

  // Filter edges to only those between visible nodes
  const visibleEdges = graph.edges.filter(
    e => nodeIndexMap.has(e.source) && nodeIndexMap.has(e.target)
  );

  // Build adjacency for topological sort
  const adj = new Map();
  const inDegree = new Map();
  visibleNodes.forEach((_, i) => { adj.set(i, []); inDegree.set(i, 0); });

  visibleEdges.forEach(e => {
    const s = nodeIndexMap.get(e.source);
    const t = nodeIndexMap.get(e.target);
    if (s !== undefined && t !== undefined) {
      adj.get(s).push(t);
      inDegree.set(t, (inDegree.get(t) || 0) + 1);
    }
  });

  // Topological layers (BFS)
  const layers = [];
  const queue = [];
  const layerOf = new Map();

  visibleNodes.forEach((n, i) => {
    if (n.node_type === 'start_agent' || inDegree.get(i) === 0) {
      queue.push(i);
      layerOf.set(i, 0);
    }
  });

  while (queue.length > 0) {
    const curr = queue.shift();
    const layer = layerOf.get(curr);
    if (!layers[layer]) layers[layer] = [];
    layers[layer].push(curr);

    for (const next of adj.get(curr) || []) {
      if (!layerOf.has(next)) {
        layerOf.set(next, layer + 1);
        queue.push(next);
      }
    }
  }

  // Place any unvisited nodes in a final layer
  visibleNodes.forEach((_, i) => {
    if (!layerOf.has(i)) {
      const lastLayer = layers.length;
      if (!layers[lastLayer]) layers[lastLayer] = [];
      layers[lastLayer].push(i);
      layerOf.set(i, lastLayer);
    }
  });

  // Assign positions
  const positioned = visibleNodes.map((n, i) => {
    const layer = layerOf.get(i) || 0;
    const layerNodes = layers[layer] || [i];
    const posInLayer = layerNodes.indexOf(i);
    const layerWidth = layerNodes.length * (NODE_W + NODE_GAP) - NODE_GAP;
    return {
      ...n,
      x: posInLayer * (NODE_W + NODE_GAP) - layerWidth / 2 + NODE_W / 2,
      y: layer * LAYER_GAP,
      w: NODE_W,
      h: NODE_H,
      layoutIndex: i,
    };
  });

  const mappedEdges = visibleEdges.map(e => ({
    ...e,
    sourceIdx: nodeIndexMap.get(e.source),
    targetIdx: nodeIndexMap.get(e.target),
  })).filter(e => e.sourceIdx !== undefined && e.targetIdx !== undefined);

  return { nodes: positioned, edges: mappedEdges, nodeIndexMap };
}

function edgeClass(type) {
  const t = type.toLowerCase().replace(/ /g, '_');
  if (t.includes('transition')) return 'edge-transition';
  if (t.includes('delegate')) return 'edge-delegates_to';
  if (t.includes('escalat')) return 'edge-escalates';
  if (t.includes('read')) return 'edge-reads';
  if (t.includes('write')) return 'edge-writes';
  if (t.includes('invoke')) return 'edge-invokes';
  return 'edge-default';
}

function render() {
  const { nodes, edges } = layoutGraph(graph, showVars);
  const svg = document.getElementById('graph');

  // Compute viewBox from node positions
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  nodes.forEach(n => {
    minX = Math.min(minX, n.x - n.w / 2);
    minY = Math.min(minY, n.y - n.h / 2);
    maxX = Math.max(maxX, n.x + n.w / 2);
    maxY = Math.max(maxY, n.y + n.h / 2);
  });
  const pad = 60;
  if (nodes.length === 0) {
    svg.innerHTML = '<text x="50%" y="50%" text-anchor="middle" fill="#999">No graph data</text>';
    return;
  }
  const vbW = maxX - minX + pad * 2;
  const vbH = maxY - minY + pad * 2;
  svg.setAttribute('viewBox', (minX - pad) + ' ' + (minY - pad) + ' ' + vbW + ' ' + vbH);

  // Arrow marker
  let html = '<defs><marker id="arrow" viewBox="0 0 10 6" refX="10" refY="3" markerWidth="8" markerHeight="6" orient="auto-start-reverse"><path d="M 0 0 L 10 3 L 0 6 z" class="edge-arrow"/></marker></defs>';

  // Edges
  edges.forEach(e => {
    const src = nodes[e.sourceIdx];
    const tgt = nodes[e.targetIdx];
    if (!src || !tgt) return;
    const cls = edgeClass(e.edge_type);
    html += '<line class="edge-line ' + cls + '" x1="' + src.x + '" y1="' + (src.y + src.h / 2) + '" x2="' + tgt.x + '" y2="' + (tgt.y - tgt.h / 2) + '" marker-end="url(#arrow)"/>';
    // Edge label
    const mx = (src.x + tgt.x) / 2;
    const my = (src.y + src.h / 2 + tgt.y - tgt.h / 2) / 2;
    const shortLabel = e.edge_type.replace(/_/g, ' ');
    html += '<text class="edge-label" x="' + mx + '" y="' + (my - 4) + '">' + shortLabel + '</text>';
  });

  // Nodes
  nodes.forEach(n => {
    const cls = 'node-' + n.node_type;
    const label = n.name || n.node_type;
    const sublabel = n.node_type === 'variable' && n.mutable !== null
      ? (n.mutable ? 'mutable' : 'linked')
      : n.node_type.replace(/_/g, ' ');

    html += '<g class="node-group ' + cls + '" onclick="nodeClick(' + n.span_start + ',' + n.span_end + ')">';
    html += '<rect class="node-rect" x="' + (n.x - n.w / 2) + '" y="' + (n.y - n.h / 2) + '" width="' + n.w + '" height="' + n.h + '"/>';
    html += '<text class="node-label" x="' + n.x + '" y="' + (n.y - 4) + '">' + escHtml(label) + '</text>';
    html += '<text class="node-sublabel" x="' + n.x + '" y="' + (n.y + 12) + '">' + escHtml(sublabel) + '</text>';
    html += '</g>';
  });

  svg.innerHTML = html;

  // Stats
  document.getElementById('stats').textContent =
    nodes.length + ' nodes · ' + edges.length + ' edges · ' +
    graph.topics.length + ' topics · ' + graph.variables.length + ' variables';
}

function escHtml(s) {
  return s.replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;');
}

function nodeClick(spanStart, spanEnd) {
  vscode.postMessage({ type: 'navigateToSpan', spanStart, spanEnd });
}

function resetZoom() { render(); }
function toggleVars() { showVars = !showVars; render(); }

// Pan & zoom via mouse wheel/drag
let viewBox = null;
let isPanning = false;
let panStart = { x: 0, y: 0 };

document.getElementById('graph').addEventListener('wheel', (e) => {
  e.preventDefault();
  const svg = document.getElementById('graph');
  const vb = svg.viewBox.baseVal;
  const scale = e.deltaY > 0 ? 1.1 : 0.9;
  const cx = vb.x + vb.width / 2;
  const cy = vb.y + vb.height / 2;
  const newW = vb.width * scale;
  const newH = vb.height * scale;
  svg.setAttribute('viewBox', (cx - newW / 2) + ' ' + (cy - newH / 2) + ' ' + newW + ' ' + newH);
}, { passive: false });

document.getElementById('graph').addEventListener('mousedown', (e) => {
  if (e.target.closest('.node-group')) return;
  isPanning = true;
  panStart = { x: e.clientX, y: e.clientY };
});

document.addEventListener('mousemove', (e) => {
  if (!isPanning) return;
  const svg = document.getElementById('graph');
  const vb = svg.viewBox.baseVal;
  const rect = svg.getBoundingClientRect();
  const dx = (e.clientX - panStart.x) * (vb.width / rect.width);
  const dy = (e.clientY - panStart.y) * (vb.height / rect.height);
  svg.setAttribute('viewBox', (vb.x - dx) + ' ' + (vb.y - dy) + ' ' + vb.width + ' ' + vb.height);
  panStart = { x: e.clientX, y: e.clientY };
});

document.addEventListener('mouseup', () => { isPanning = false; });

render();
</script>
</body>
</html>`;
  }

  dispose(): void {
    this.panel?.dispose();
    this.disposables.forEach((d) => d.dispose());
  }
}
