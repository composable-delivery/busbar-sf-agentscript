import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graphLib from 'busbar-sf-agentscript';

// After bundling, __dirname is lib/commands/agency/ - go up 3 levels to plugin root
const pluginRoot = path.resolve(__dirname, '..', '..', '..');
Messages.importMessagesDirectory(pluginRoot);
const messages = Messages.loadMessages('sf-plugin-busbar-agency', 'agency.graph');

interface GraphResult {
  view: string;
  format: string;
  graph: string;
}

export default class AgentscriptGraph extends SfCommand<GraphResult> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  public static readonly flags = {
    file: Flags.file({
      char: 'f',
      summary: messages.getMessage('flags.file.summary'),
      description: messages.getMessage('flags.file.description'),
      required: true,
      exists: true,
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
      options: ['ascii', 'graphml'] as const,
      default: 'ascii',
    })(),
  };

  public async run(): Promise<GraphResult> {
    const { flags } = await this.parse(AgentscriptGraph);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    try {
      const filePath = path.resolve(flags.file as string);
      const source = fs.readFileSync(filePath, 'utf-8');
      const fileName = path.basename(filePath);

      // Handle GraphML format separately (raw output for piping)
      if (flags.format === 'graphml') {
        const graphmlOutput = graphLib.export_graphml(source);
        this.log(graphmlOutput);
        return { view: flags.view, format: flags.format, graph: graphmlOutput };
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

      return { view: flags.view, format: flags.format, graph: graphOutput };
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
}
