import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graphLib from '../../wasm-loader.js';
import { resolveTargetFiles } from '../../lib/agent-files.js';

interface GraphExportNode {
  id: number;
  node_type: string;
  name: string | null;
  topic: string | null;
  target: string | null;
}

interface GraphExportEdge {
  source: number;
  target: number;
  edge_type: string;
}

interface TopicExportInfo {
  name: string;
  description: string | null;
  is_entry: boolean;
  transitions_to: string[];
  delegates_to: string[];
}

interface GraphExport {
  nodes: GraphExportNode[];
  edges: GraphExportEdge[];
  topics: TopicExportInfo[];
  variables: string[];
  stats: {
    total_nodes: number;
    total_edges: number;
    topics: number;
    variables: number;
    action_defs: number;
    reasoning_actions: number;
  };
}

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.graph');

interface GraphResult {
  file: string;
  view: string;
  format: string;
  graph: string;
}

export default class AgentscriptGraph extends SfCommand<GraphResult | GraphResult[]> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  public static readonly flags = {
    file: Flags.file({
      char: 'f',
      summary: messages.getMessage('flags.file.summary'),
      description: messages.getMessage('flags.file.description'),
      required: false,
      exists: true,
    }),
    path: Flags.directory({
      summary: 'Directory to scan for agent files (default: current directory).',
      description: 'Recursively searches this directory for .agent files when --file is not specified.',
      default: '.',
    }),
    view: Flags.option({
      char: 'v',
      summary: messages.getMessage('flags.view.summary'),
      description: messages.getMessage('flags.view.description'),
      options: ['topics', 'actions', 'full'] as const,
      default: 'topics',
    })(),
    format: Flags.option({
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['ascii', 'graphml', 'mermaid', 'html'] as const,
      default: 'ascii',
    })(),
    stats: Flags.boolean({
      summary: messages.getMessage('flags.stats.summary'),
      description: messages.getMessage('flags.stats.description'),
      default: false,
    }),
  };

  public async run(): Promise<GraphResult | GraphResult[]> {
    const { flags } = await this.parse(AgentscriptGraph);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    try {
      const files = resolveTargetFiles({
        file: flags.file,
        scanPath: flags.path,
        dataDir: this.config.dataDir,
      });

      const parsed = await Promise.all(files.map(async filePath => {
        const source = fs.readFileSync(filePath, 'utf-8');
        return { filePath, source };
      }));

      const results: GraphResult[] = [];

      for (const { filePath, source } of parsed) {
        const fileName = path.basename(filePath);

        if (files.length > 1) {
          this.log(ansis.bold.dim(`\n─── ${path.relative(process.cwd(), filePath)} ───`));
        }

        // Handle GraphML format separately (raw output for piping)
        const file = path.relative(process.cwd(), filePath);

        if (flags.format === 'graphml') {
          const graphmlOutput = graphLib.export_graphml(source);
          this.log(graphmlOutput);
          results.push({ file, view: flags.view, format: flags.format, graph: graphmlOutput });
          continue;
        }

        // Handle Mermaid format
        if (flags.format === 'mermaid') {
          const mermaidOutput = this.generateMermaid(source, flags.stats);
          this.log(mermaidOutput);
          results.push({ file, view: flags.view, format: flags.format, graph: mermaidOutput });
          continue;
        }

        // Handle HTML format
        if (flags.format === 'html') {
          const agentName = path.basename(filePath, '.agent');
          const htmlOutput = this.generateHtml(source, agentName, flags.stats);
          this.log(htmlOutput);
          results.push({ file, view: flags.view, format: flags.format, graph: htmlOutput });
          continue;
        }

        // ASCII format: Render graph using WASM with timing
        const startTime = performance.now();
        const graphOutput = graphLib.render_graph(source, flags.view);
        const elapsed = (performance.now() - startTime).toFixed(2);

        // Display header
        ux.styledHeader(`${this.getViewTitle(flags.view)} - ${fileName}`);
        this.log('');

        // Display the graph with styling
        this.displayStyledGraph(graphOutput, flags.view);

        this.log('');
        this.log(ansis.dim(`Rendered in ${elapsed}ms`));
        this.log('');

        // Legend
        this.displayLegend(flags.view);

        // Stats footer
        if (flags.stats) {
          this.log('');
          this.displayStats(source);
        }

        results.push({ file, view: flags.view, format: flags.format, graph: graphOutput });
      }

      return files.length === 1 ? results[0] : results;
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.graphFailure', [error.message]);
      }
      throw error;
    }
  }

  private getViewTitle(view: string): string {
    switch (view) {
      case 'topics':
        return 'Topic Flow Graph';
      case 'actions':
        return 'Actions Graph';
      case 'full':
        return 'Full Reference Graph';
      default:
        return 'Graph';
    }
  }

  private displayStyledGraph(graph: string, view: string): void {
    const lines = graph.split('\n');

    for (const line of lines) {
      let styledLine = line;

      if (view === 'full') {
        // Structured topic-centric view
        // Section headers
        if (line.match(/^(VARIABLES|ENTRY POINT|TOPICS):/)) {
          styledLine = ansis.bold.white(line);
        }
        // Topic box headers: ┌─ topic_name ─────
        else if (line.includes('┌─')) {
          styledLine = line.replace(/┌─ ([^\s]+) ─+/, (_, name) =>
            `┌─ ${ansis.bold.cyan(name)} ─────────────────────────────`
          );
        }
        // Actions: │   • action_name
        else if (line.includes('• ')) {
          styledLine = line.replace(/• (.+)$/, (_, name) => `• ${ansis.magenta(name)}`);
        }
        // Reasoning: │   ◆ name → target
        else if (line.includes('◆ ')) {
          styledLine = line.replace(/◆ ([^\s→]+)( → (.+))?$/, (_, name, __, target) => {
            if (target) {
              // Color target based on type
              let coloredTarget = target;
              if (target.includes('@actions.')) {
                coloredTarget = ansis.magenta(target);
              } else if (target.includes('@topic.')) {
                coloredTarget = ansis.cyan(target);
              } else if (target.includes('@utils.')) {
                coloredTarget = ansis.yellow(target);
              }
              return `◆ ${ansis.green(name)} → ${coloredTarget}`;
            }
            return `◆ ${ansis.green(name)}`;
          });
        }
        // Transitions/Delegates lines
        else if (line.includes('Transitions →')) {
          styledLine = line.replace(/Transitions → (.+)$/, (_, topics) =>
            `${ansis.dim('Transitions →')} ${ansis.cyan(topics)}`
          );
        }
        else if (line.includes('Delegates ⇒')) {
          styledLine = line.replace(/Delegates ⇒ (.+)$/, (_, topics) =>
            `${ansis.dim('Delegates ⇒')} ${ansis.blue(topics)}`
          );
        }
        // routes to line
        else if (line.includes('routes to:')) {
          styledLine = line.replace(/routes to: (.+)$/, (_, topics) =>
            `${ansis.dim('routes to:')} ${ansis.cyan(topics)}`
          );
        }
        // Mutable/Linked variable lines
        else if (line.includes('Mutable:')) {
          styledLine = line.replace(/Mutable: (.+)$/, (_, vars) =>
            `${ansis.dim('Mutable:')} ${ansis.green(vars)}`
          );
        }
        else if (line.includes('Linked:')) {
          styledLine = line.replace(/Linked: (.+)$/, (_, vars) =>
            `${ansis.dim('Linked:')}  ${ansis.blue(vars)}`
          );
        }
        // start_agent
        else if (line.includes('start_agent')) {
          styledLine = line.replace(/start_agent/g, ansis.bold.white('start_agent'));
        }
        // Box drawing - dim it
        else if (line.match(/^[│└┌─\s]+$/)) {
          styledLine = ansis.dim(line);
        }
      } else if (view === 'actions') {
        // Actions view has bracketed topics
        styledLine = styledLine
          .replace(/\[([^\]]+)\]/g, ansis.cyan('[$1]'))
          .replace(/\bstart_agent\b/g, ansis.bold.white('start_agent'));
      } else {
        // Topics view - tree structure

        // Color topic names after edge indicators
        styledLine = styledLine
          .replace(/([⊳→⇒]\s*)([a-zA-Z_][a-zA-Z0-9_]*)/g, (_, prefix, name) =>
            `${prefix}${ansis.cyan(name)}`
          );

        // Color standalone topic names (like fraud_review at root), but not start_agent
        if (line.match(/^[a-zA-Z_][a-zA-Z0-9_]*$/) && line !== 'start_agent') {
          styledLine = ansis.cyan(line);
        }

        // Color topic names after tree chars but before ↩
        styledLine = styledLine
          .replace(/(─+\s*)([a-zA-Z_][a-zA-Z0-9_]*)(\s*↩?)$/g, (_, prefix, name, suffix) =>
            `${prefix}${ansis.cyan(name)}${suffix}`
          );

        // start_agent gets bold white (apply last to override any cyan)
        styledLine = styledLine
          .replace(/\bstart_agent\b/g, ansis.bold.white('start_agent'));
      }

      this.log(styledLine);
    }
  }

  private displayLegend(view: string): void {
    if (view === 'full') {
      this.log(ansis.dim('Legend: ') +
        ansis.magenta('•') + ansis.dim(' action | ') +
        ansis.green('◆') + ansis.dim(' reasoning | ') +
        ansis.cyan('→') + ansis.dim(' transition | ') +
        ansis.blue('⇒') + ansis.dim(' delegate'));
    } else if (view === 'actions') {
      this.log(ansis.dim('Legend: ') +
        ansis.bold.white('start_agent') + ansis.dim(' = entry | ') +
        ansis.cyan('[topic]') + ansis.dim(' = topic | ') +
        ansis.dim('action_name = action definition'));
    } else {
      this.log(ansis.dim('Legend: ') +
        ansis.bold.white('start_agent') + ansis.dim(' = entry point | ') +
        ansis.cyan('topic_name') + ansis.dim(' = conversation topic'));
    }
  }

  private displayStats(source: string): void {
    const stats = graphLib.get_graph_stats(source);
    this.log(ansis.dim('Stats: ') +
      ansis.bold(String(stats.topics)) + ansis.dim(' topics | ') +
      ansis.bold(String(stats.variables)) + ansis.dim(' variables | ') +
      ansis.bold(String(stats.action_defs)) + ansis.dim(' action defs | ') +
      ansis.bold(String(stats.reasoning_actions)) + ansis.dim(' reasoning steps'));
  }

  private generateMermaid(source: string, showStats: boolean): string {
    const graphJson: GraphExport = JSON.parse(graphLib.export_graph_json(source));
    const lines: string[] = ['```mermaid', 'flowchart LR'];

    // Define nodes
    for (const topic of graphJson.topics) {
      const safeId = topic.name.replace(/[^a-zA-Z0-9_]/g, '_');
      if (topic.is_entry) {
        lines.push(`  ${safeId}([${topic.name}])`);
      } else {
        lines.push(`  ${safeId}[${topic.name}]`);
      }
    }

    // Define edges from topic info
    for (const topic of graphJson.topics) {
      const srcId = topic.name.replace(/[^a-zA-Z0-9_]/g, '_');
      for (const dest of topic.transitions_to) {
        const destId = dest.replace(/[^a-zA-Z0-9_]/g, '_');
        lines.push(`  ${srcId} --> ${destId}`);
      }
      for (const dest of topic.delegates_to) {
        const destId = dest.replace(/[^a-zA-Z0-9_]/g, '_');
        lines.push(`  ${srcId} ==> ${destId}`);
      }
    }

    lines.push('```');

    if (showStats) {
      const stats = graphLib.get_graph_stats(source);
      lines.push('');
      lines.push(`> Stats: ${stats.topics} topics | ${stats.variables} variables | ${stats.action_defs} action defs | ${stats.reasoning_actions} reasoning steps`);
    }

    return lines.join('\n');
  }

  private generateHtml(source: string, agentName: string, showStats: boolean): string {
    const graphJson: GraphExport = JSON.parse(graphLib.export_graph_json(source));
    const mermaidLines: string[] = ['flowchart LR'];

    for (const topic of graphJson.topics) {
      const safeId = topic.name.replace(/[^a-zA-Z0-9_]/g, '_');
      if (topic.is_entry) {
        mermaidLines.push(`  ${safeId}([${topic.name}])`);
      } else {
        mermaidLines.push(`  ${safeId}[${topic.name}]`);
      }
    }

    for (const topic of graphJson.topics) {
      const srcId = topic.name.replace(/[^a-zA-Z0-9_]/g, '_');
      for (const dest of topic.transitions_to) {
        const destId = dest.replace(/[^a-zA-Z0-9_]/g, '_');
        mermaidLines.push(`  ${srcId} --> ${destId}`);
      }
      for (const dest of topic.delegates_to) {
        const destId = dest.replace(/[^a-zA-Z0-9_]/g, '_');
        mermaidLines.push(`  ${srcId} ==> ${destId}`);
      }
    }

    const mermaidContent = mermaidLines.join('\n');

    let statsHtml = '';
    if (showStats) {
      const stats = graphLib.get_graph_stats(source);
      statsHtml = `
    <div class="stats">
      <strong>Stats:</strong> ${stats.topics} topics &nbsp;|&nbsp; ${stats.variables} variables &nbsp;|&nbsp;
      ${stats.action_defs} action defs &nbsp;|&nbsp; ${stats.reasoning_actions} reasoning steps
    </div>`;
    }

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>${agentName} - Agent Graph</title>
  <script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
  <style>
    body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; margin: 0; padding: 24px; background: #0d1117; color: #c9d1d9; }
    h1 { font-size: 1.4rem; color: #58a6ff; margin-bottom: 8px; }
    .subtitle { font-size: 0.85rem; color: #8b949e; margin-bottom: 24px; }
    .mermaid { background: #161b22; border-radius: 8px; padding: 24px; }
    .stats { margin-top: 16px; font-size: 0.85rem; color: #8b949e; background: #161b22; border-radius: 6px; padding: 12px 16px; }
  </style>
</head>
<body>
  <h1>${agentName}</h1>
  <div class="subtitle">Agent topic flow graph</div>
  <div class="mermaid">
${mermaidContent}
  </div>${statsHtml}
  <script>mermaid.initialize({ startOnLoad: true, theme: 'dark' });</script>
</body>
</html>`;
  }
}
