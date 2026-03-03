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
  actions?: string[];
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
  outputFile?: string;
}

interface AgentHtmlVariable {
  name: string;
  type: string;
  mutable: boolean;
  linked: boolean;
}

interface AgentHtmlTopic {
  safeId: string;
  name: string;
  description: string | null;
  is_entry: boolean;
  actions: string[];
  transitions_to: string[];
  delegates_to: string[];
  var_reads: string[];
  var_writes: string[];
}

interface AgentHtmlData {
  id: string;
  agentName: string;
  agentLabel?: string;
  agentDescription?: string;
  file: string;
  mermaidContent: string;
  stats?: {
    topics: number;
    variables: number;
    action_defs: number;
    reasoning_actions: number;
  };
  variables: AgentHtmlVariable[];
  topics: AgentHtmlTopic[];
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
    output: Flags.string({
      summary: 'Path to write the HTML report file. Defaults to agents-report.html (or <name>.html for a single agent).',
      helpValue: 'FILE',
    }),
  };

  public async run(): Promise<GraphResult | GraphResult[]> {
    const { flags } = await this.parse(AgentscriptGraph);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    const files = resolveTargetFiles({
      file: flags.file,
      scanPath: flags.path,
      dataDir: this.config.dataDir,
    });

    // Read all files in parallel
    const fileReads = await Promise.all(files.map(async filePath => {
      try {
        const source = await fs.promises.readFile(filePath, 'utf-8');
        return { filePath, source, ok: true as const };
      } catch (e) {
        return { filePath, source: '', ok: false as const, error: e instanceof Error ? e.message : String(e) };
      }
    }));

    const results: GraphResult[] = [];
    const fileErrors: Array<{ file: string; error: string }> = [];

    // HTML: collect all agents first, then write one combined report
    if (flags.format === 'html') {
      const agentDataList: AgentHtmlData[] = [];

      for (const fileRead of fileReads) {
        const file = path.relative(process.cwd(), fileRead.filePath);
        if (!fileRead.ok) {
          fileErrors.push({ file, error: fileRead.error });
          continue;
        }
        try {
          const fileAgentName = path.basename(fileRead.filePath, '.agent');
          const id = fileAgentName.replace(/[^a-zA-Z0-9_-]/g, '_');
          const graphJson: GraphExport = JSON.parse(graphLib.export_graph_json(fileRead.source));

          // Extract per-topic variable reads/writes from graph nodes
          const topicVarReads = new Map<string, Set<string>>();
          const topicVarWrites = new Map<string, Set<string>>();
          for (const node of graphJson.nodes) {
            if (node.topic && node.name) {
              if (node.node_type === 'variable_read') {
                const s = topicVarReads.get(node.topic) ?? new Set();
                s.add(node.name);
                topicVarReads.set(node.topic, s);
              } else if (node.node_type === 'variable_write') {
                const s = topicVarWrites.get(node.topic) ?? new Set();
                s.add(node.name);
                topicVarWrites.set(node.topic, s);
              }
            }
          }

          // Build per-topic detail objects
          const topics: AgentHtmlTopic[] = graphJson.topics.map(t => ({
            safeId: t.name.replace(/[^a-zA-Z0-9_]/g, '_'),
            name: t.name,
            description: t.description,
            is_entry: t.is_entry,
            actions: t.actions ?? [],
            transitions_to: t.transitions_to,
            delegates_to: t.delegates_to,
            var_reads: [...(topicVarReads.get(t.name) ?? [])],
            var_writes: [...(topicVarWrites.get(t.name) ?? [])],
          }));

          // Extract config and variables from AST
          let agentLabel: string | undefined;
          let agentDescription: string | undefined;
          let agentDisplayName = fileAgentName;
          const variables: AgentHtmlVariable[] = [];
          try {
            const ast = graphLib.parse_agent(fileRead.source);
            const config = ast?.config?.node ?? ast?.config;
            if (config) {
              const name = config.agent_name?.node || config.agent_name?.value;
              if (name) agentDisplayName = name;
              const label = config.agent_label?.node || config.agent_label?.value;
              if (label) agentLabel = label;
              const desc = config.description?.node || config.description?.value;
              if (desc) agentDescription = desc;
            }
            if (ast?.variables) {
              for (const [varName, varVal] of Object.entries(ast.variables as Record<string, any>)) {
                const varNode = varVal?.node ?? varVal;
                variables.push({
                  name: varName,
                  type: varNode?.var_type?.node || varNode?.var_type?.value || varNode?.var_type || 'unknown',
                  mutable: Boolean(varNode?.is_mutable),
                  linked: Boolean(varNode?.is_linked),
                });
              }
            }
          } catch {
            // AST parse failed — use graph data only
          }

          // Build Mermaid content with click handlers
          const mermaidLines: string[] = ['flowchart LR'];
          for (const topic of graphJson.topics) {
            const safeId = topic.name.replace(/[^a-zA-Z0-9_]/g, '_');
            mermaidLines.push(`  ${safeId}${topic.is_entry ? `([${topic.name}])` : `[${topic.name}]`}`);
          }
          for (const topic of graphJson.topics) {
            const srcId = topic.name.replace(/[^a-zA-Z0-9_]/g, '_');
            for (const dest of topic.transitions_to) {
              mermaidLines.push(`  ${srcId} --> ${dest.replace(/[^a-zA-Z0-9_]/g, '_')}`);
            }
            for (const dest of topic.delegates_to) {
              mermaidLines.push(`  ${srcId} ==> ${dest.replace(/[^a-zA-Z0-9_]/g, '_')}`);
            }
          }
          // Add click handlers after edges
          for (const topic of graphJson.topics) {
            const safeId = topic.name.replace(/[^a-zA-Z0-9_]/g, '_');
            mermaidLines.push(`  click ${safeId} onNodeClick`);
          }

          const stats = graphLib.get_graph_stats(fileRead.source);
          agentDataList.push({
            id,
            agentName: agentDisplayName,
            agentLabel,
            agentDescription,
            file,
            mermaidContent: mermaidLines.join('\n'),
            stats,
            variables,
            topics,
          });
          results.push({ file, view: flags.view, format: flags.format, graph: mermaidLines.join('\n') });
        } catch (e) {
          fileErrors.push({ file, error: e instanceof Error ? e.message : String(e) });
        }
      }

      // Sort agents alphabetically by display name
      agentDataList.sort((a, b) => a.agentName.localeCompare(b.agentName));

      if (agentDataList.length > 0) {
        const defaultName = files.length === 1
          ? `${path.basename(files[0], '.agent')}.html`
          : 'agents-report.html';
        const outputPath = path.resolve(flags.output ?? defaultName);
        const html = this.generateCombinedHtml(agentDataList);
        fs.writeFileSync(outputPath, html, 'utf-8');
        this.log(`${ansis.green('✓')} Report written to ${ansis.bold(path.relative(process.cwd(), outputPath))}`);
        results[0] = { ...results[0], outputFile: outputPath };
      }

      if (fileErrors.length > 0) {
        this.log('');
        this.log(ansis.red.bold(`${fileErrors.length} file${fileErrors.length === 1 ? '' : 's'} failed:`));
        for (const { file, error } of fileErrors) {
          this.log(`  ${ansis.red('✗')} ${ansis.bold(file)}: ${ansis.dim(error)}`);
        }
      }

      return files.length === 1 ? results[0] : results;
    }

    // Non-HTML formats
    for (const fileRead of fileReads) {
      const file = path.relative(process.cwd(), fileRead.filePath);

      if (!fileRead.ok) {
        fileErrors.push({ file, error: fileRead.error });
        continue;
      }

      if (files.length > 1) {
        this.log(ansis.bold.dim(`\n─── ${file} ───`));
      }

      try {
        const { filePath, source } = fileRead;
        const fileName = path.basename(filePath);

        if (flags.format === 'graphml') {
          const graphmlOutput = graphLib.export_graphml(source);
          this.log(graphmlOutput);
          results.push({ file, view: flags.view, format: flags.format, graph: graphmlOutput });
          continue;
        }

        if (flags.format === 'mermaid') {
          const mermaidOutput = this.generateMermaid(source, flags.stats);
          this.log(mermaidOutput);
          results.push({ file, view: flags.view, format: flags.format, graph: mermaidOutput });
          continue;
        }

        // ASCII format
        const startTime = performance.now();
        const graphOutput = graphLib.render_graph(source, flags.view);
        const elapsed = (performance.now() - startTime).toFixed(2);

        ux.styledHeader(`${this.getViewTitle(flags.view)} - ${fileName}`);
        this.log('');
        this.displayStyledGraph(graphOutput, flags.view);
        this.log('');
        this.log(ansis.dim(`Rendered in ${elapsed}ms`));
        this.log('');
        this.displayLegend(flags.view);

        if (flags.stats) {
          this.log('');
          this.displayStats(source);
        }

        results.push({ file, view: flags.view, format: flags.format, graph: graphOutput });
      } catch (e) {
        fileErrors.push({ file, error: e instanceof Error ? e.message : String(e) });
      }
    }

    if (fileErrors.length > 0) {
      this.log('');
      this.log(ansis.red.bold(`${fileErrors.length} file${fileErrors.length === 1 ? '' : 's'} failed:`));
      for (const { file, error } of fileErrors) {
        this.log(`  ${ansis.red('✗')} ${ansis.bold(file)}: ${ansis.dim(error)}`);
      }
    }

    return files.length === 1 ? results[0] : results;
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

  private generateCombinedHtml(agents: AgentHtmlData[]): string {
    const navItems = agents.map(a => `
      <div class="nav-item" id="nav-${a.id}" data-id="${a.id}" onclick="showAgent('${a.id}')">
        <span class="nav-name">${a.agentName}</span>
        ${a.stats ? `<span class="badge">${a.stats.topics}T</span>` : ''}
      </div>`).join('');

    const sections = agents.map(a => {
      const statsBar = a.stats ? `
        <div class="stats-bar">
          <span><strong>${a.stats.topics}</strong> topics</span>
          <span><strong>${a.stats.variables}</strong> variables</span>
          <span><strong>${a.stats.action_defs}</strong> action defs</span>
          <span><strong>${a.stats.reasoning_actions}</strong> reasoning steps</span>
        </div>` : '';

      return `
      <div class="agent-section" id="section-${a.id}">
        <div class="agent-header">
          <h1 class="agent-name">${a.agentName}</h1>
          ${a.agentLabel ? `<div class="agent-label">${a.agentLabel}</div>` : ''}
          <div class="agent-file">${a.file}</div>
        </div>
        <div class="graph-container">
          <div class="mermaid">${a.mermaidContent}</div>
        </div>
        ${statsBar}
        <div class="legend">→ transition &nbsp;|&nbsp; ⇒ delegate &nbsp;|&nbsp; ([name]) = entry point &nbsp;|&nbsp; <em>click a node for details</em></div>
        <div class="detail-panel" id="detail-${a.id}"></div>
      </div>`;
    }).join('');

    // Embed all agent data as JSON for the JS detail panel.
    // Escape </script> to prevent the HTML parser from closing the script block early.
    const allDataJson = JSON.stringify(
      Object.fromEntries(agents.map(a => [a.id, {
        agentName: a.agentName,
        agentLabel: a.agentLabel,
        agentDescription: a.agentDescription,
        variables: a.variables,
        topics: a.topics,
      }]))
    ).replace(/<\/script>/gi, '<\\/script>');

    const firstId = agents[0]?.id ?? '';

    return `<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>AgentScript Report</title>
  <script src="https://cdn.jsdelivr.net/npm/mermaid@10/dist/mermaid.min.js"></script>
  <style>
    *, *::before, *::after { box-sizing: border-box; }
    body { margin: 0; background: #0d1117; color: #c9d1d9; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; display: flex; min-height: 100vh; }
    #sidebar { width: 240px; min-height: 100vh; background: #161b22; border-right: 1px solid #30363d; position: fixed; top: 0; left: 0; bottom: 0; overflow-y: auto; display: flex; flex-direction: column; }
    .sidebar-header { padding: 20px 16px 16px; border-bottom: 1px solid #30363d; }
    .sidebar-title { font-size: 0.75rem; font-weight: 600; color: #8b949e; text-transform: uppercase; letter-spacing: 0.05em; }
    .sidebar-count { font-size: 0.7rem; color: #484f58; margin-top: 2px; }
    .nav-item { padding: 8px 16px; cursor: pointer; color: #c9d1d9; font-size: 0.875rem; border-left: 3px solid transparent; display: flex; align-items: center; gap: 8px; transition: background 0.1s; }
    .nav-item:hover { background: #21262d; }
    .nav-item.active { color: #58a6ff; border-left-color: #58a6ff; background: rgba(88,166,255,0.05); }
    .nav-name { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
    .badge { font-size: 0.65rem; color: #484f58; background: #21262d; border-radius: 10px; padding: 1px 6px; flex-shrink: 0; }
    .nav-item.active .badge { background: rgba(88,166,255,0.15); color: #58a6ff; }
    #main { margin-left: 240px; padding: 32px; flex: 1; min-width: 0; }
    .agent-section { display: none; }
    .agent-section.active { display: block; }
    .agent-header { margin-bottom: 20px; }
    .agent-name { font-size: 1.4rem; color: #58a6ff; margin: 0 0 2px 0; font-weight: 600; }
    .agent-label { font-size: 0.85rem; color: #8b949e; margin-bottom: 2px; }
    .agent-file { font-size: 0.75rem; color: #484f58; font-family: monospace; }
    .graph-container { background: #161b22; border-radius: 8px; padding: 24px; border: 1px solid #30363d; overflow: auto; cursor: pointer; }
    .stats-bar { margin-top: 10px; font-size: 0.8rem; color: #8b949e; background: #161b22; border-radius: 6px; padding: 9px 16px; border: 1px solid #30363d; display: flex; gap: 20px; flex-wrap: wrap; }
    .stats-bar strong { color: #c9d1d9; }
    .legend { margin-top: 8px; font-size: 0.72rem; color: #484f58; }
    .mermaid svg { max-width: 100%; height: auto; }

    /* Detail panel */
    .detail-panel { margin-top: 24px; }
    .detail-card { background: #161b22; border: 1px solid #30363d; border-radius: 8px; padding: 20px; }
    .detail-card + .detail-card { margin-top: 16px; }
    .detail-back { font-size: 0.78rem; color: #58a6ff; cursor: pointer; margin-bottom: 14px; display: inline-flex; align-items: center; gap: 4px; }
    .detail-back:hover { text-decoration: underline; }
    .detail-title { font-size: 1rem; font-weight: 600; color: #c9d1d9; margin: 0 0 4px 0; display: flex; align-items: center; gap: 8px; }
    .detail-desc { font-size: 0.82rem; color: #8b949e; margin: 0 0 16px 0; }
    .entry-badge { font-size: 0.65rem; background: rgba(88,166,255,0.15); color: #58a6ff; border: 1px solid rgba(88,166,255,0.3); border-radius: 10px; padding: 1px 7px; font-weight: 500; }
    .detail-section { margin-bottom: 14px; }
    .detail-section-title { font-size: 0.7rem; font-weight: 600; color: #8b949e; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 6px; }
    .tag-list { display: flex; flex-wrap: wrap; gap: 6px; }
    .tag { font-size: 0.78rem; padding: 2px 10px; border-radius: 12px; font-family: monospace; }
    .tag-action { background: rgba(188,135,255,0.12); color: #c8a4ff; border: 1px solid rgba(188,135,255,0.25); }
    .tag-topic { background: rgba(88,166,255,0.1); color: #79b8ff; border: 1px solid rgba(88,166,255,0.2); }
    .tag-var-read { background: rgba(57,211,83,0.1); color: #7ee787; border: 1px solid rgba(57,211,83,0.2); }
    .tag-var-write { background: rgba(255,166,77,0.1); color: #ffb347; border: 1px solid rgba(255,166,77,0.2); }
    .tag-delegate { background: rgba(88,118,255,0.1); color: #7080ff; border: 1px solid rgba(88,118,255,0.2); }
    .var-table { width: 100%; border-collapse: collapse; font-size: 0.82rem; }
    .var-table th { color: #8b949e; font-weight: 500; text-align: left; padding: 4px 8px 8px 0; font-size: 0.7rem; text-transform: uppercase; letter-spacing: 0.04em; }
    .var-table td { padding: 4px 8px 4px 0; color: #c9d1d9; border-top: 1px solid #21262d; font-family: monospace; }
    .var-type { color: #7ee787; }
    .var-mod { font-size: 0.7rem; color: #8b949e; }
    .empty-msg { font-size: 0.8rem; color: #484f58; font-style: italic; }
    .overview-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
    @media (max-width: 700px) { .overview-grid { grid-template-columns: 1fr; } }
  </style>
</head>
<body>
  <nav id="sidebar">
    <div class="sidebar-header">
      <div class="sidebar-title">AgentScript</div>
      <div class="sidebar-count">${agents.length} agent${agents.length === 1 ? '' : 's'}</div>
    </div>
    ${navItems}
  </nav>
  <main id="main">
    ${sections}
  </main>
  <script>
    const allData = ${allDataJson};

    mermaid.initialize({ startOnLoad: false, theme: 'dark' });
    const rendered = new Set();
    let currentAgentId = null;

    function showAgent(id) {
      document.querySelectorAll('.agent-section').forEach(el => el.classList.remove('active'));
      document.querySelectorAll('.nav-item').forEach(el => el.classList.remove('active'));
      const section = document.getElementById('section-' + id);
      const nav = document.getElementById('nav-' + id);
      if (!section) return;
      section.classList.add('active');
      if (nav) nav.classList.add('active');
      currentAgentId = id;
      if (!rendered.has(id)) {
        rendered.add(id);
        mermaid.run({ nodes: section.querySelectorAll('.mermaid') });
      }
      history.replaceState(null, '', '#' + id);
      showOverview(id);
    }

    function showOverview(agentId) {
      const data = allData[agentId];
      const panel = document.getElementById('detail-' + agentId);
      if (!panel || !data) return;

      const descHtml = data.agentDescription
        ? '<p class="detail-desc">' + esc(data.agentDescription) + '</p>' : '';

      const varRows = data.variables.length
        ? data.variables.map(v => {
            const mods = [v.mutable ? 'mutable' : '', v.linked ? 'linked' : ''].filter(Boolean).join(' ');
            return '<tr><td>' + esc(v.name) + '</td><td class="var-type">' + esc(v.type) + '</td><td class="var-mod">' + esc(mods) + '</td></tr>';
          }).join('')
        : '<tr><td colspan="3" class="empty-msg">No variables defined</td></tr>';

      panel.innerHTML = '<div class="detail-card">' +
        '<div class="detail-title">Agent Overview</div>' +
        descHtml +
        '<div class="overview-grid">' +
        '<div class="detail-section"><div class="detail-section-title">Variables (' + data.variables.length + ')</div>' +
        '<table class="var-table"><thead><tr><th>Name</th><th>Type</th><th>Modifiers</th></tr></thead><tbody>' + varRows + '</tbody></table></div>' +
        '<div class="detail-section"><div class="detail-section-title">Topics (' + data.topics.length + ')</div>' +
        '<div class="tag-list">' + data.topics.map(t =>
          '<span class="tag tag-topic">' + esc(t.name) + (t.is_entry ? ' ★' : '') + '</span>'
        ).join('') + '</div></div>' +
        '</div></div>';
    }

    function onNodeClick(nodeId) {
      if (!currentAgentId) return;
      const data = allData[currentAgentId];
      if (!data) return;
      const topic = data.topics.find(t => t.safeId === nodeId);
      if (!topic) return;

      const panel = document.getElementById('detail-' + currentAgentId);
      if (!panel) return;

      const descHtml = topic.description
        ? '<p class="detail-desc">' + esc(topic.description) + '</p>' : '';

      const actionsHtml = topic.actions.length
        ? '<div class="tag-list">' + topic.actions.map(a => '<span class="tag tag-action">' + esc(a) + '</span>').join('') + '</div>'
        : '<span class="empty-msg">None</span>';

      const transHtml = topic.transitions_to.length
        ? '<div class="tag-list">' + topic.transitions_to.map(t => '<span class="tag tag-topic">' + esc(t) + '</span>').join('') + '</div>'
        : '<span class="empty-msg">None (terminal topic)</span>';

      const delegHtml = topic.delegates_to.length
        ? '<div class="tag-list">' + topic.delegates_to.map(t => '<span class="tag tag-delegate">' + esc(t) + '</span>').join('') + '</div>'
        : '';

      const readsHtml = topic.var_reads.length
        ? '<div class="tag-list">' + topic.var_reads.map(v => '<span class="tag tag-var-read">' + esc(v) + '</span>').join('') + '</div>'
        : '<span class="empty-msg">None</span>';

      const writesHtml = topic.var_writes.length
        ? '<div class="tag-list">' + topic.var_writes.map(v => '<span class="tag tag-var-write">' + esc(v) + '</span>').join('') + '</div>'
        : '<span class="empty-msg">None</span>';

      panel.innerHTML =
        '<div class="detail-back" onclick="showOverview(currentAgentId)">← back to overview</div>' +
        '<div class="detail-card">' +
        '<div class="detail-title">' + esc(topic.name) + (topic.is_entry ? ' <span class="entry-badge">entry</span>' : '') + '</div>' +
        descHtml +
        (topic.actions.length ? '<div class="detail-section"><div class="detail-section-title">Actions (' + topic.actions.length + ')</div>' + actionsHtml + '</div>' : '') +
        '<div class="detail-section"><div class="detail-section-title">Transitions to</div>' + transHtml + '</div>' +
        (topic.delegates_to.length ? '<div class="detail-section"><div class="detail-section-title">Delegates to</div>' + delegHtml + '</div>' : '') +
        '<div class="detail-section"><div class="detail-section-title">Variables read</div>' + readsHtml + '</div>' +
        '<div class="detail-section"><div class="detail-section-title">Variables written</div>' + writesHtml + '</div>' +
        '</div>';
    }

    function esc(str) {
      if (!str) return '';
      return String(str).replace(/&/g,'&amp;').replace(/</g,'&lt;').replace(/>/g,'&gt;').replace(/"/g,'&quot;');
    }

    const hash = location.hash.slice(1);
    const firstId = '${firstId}';
    showAgent(hash && document.getElementById('section-' + hash) ? hash : firstId);
  </script>
</body>
</html>`;
  }
}
