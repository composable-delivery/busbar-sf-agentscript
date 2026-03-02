import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graphLib from '../../wasm-loader.js';
import { resolveTargetFiles } from '../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.paths');

interface TopicExportInfo {
  name: string;
  is_entry: boolean;
  transitions_to: string[];
  delegates_to: string[];
}

interface GraphExport {
  topics: TopicExportInfo[];
}

interface PathEntry {
  nodes: string[];
  edge_types: string[];  // parallel array: edge_types[i] = how we got from nodes[i] to nodes[i+1]
  has_cycle: boolean;
}

interface PathsResult {
  file: string;
  paths: PathEntry[];
  unreachable: string[];
  total_paths: number;
}

export default class AgencyPaths extends SfCommand<PathsResult | PathsResult[]> {
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
    format: Flags.option({
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['json', 'pretty'] as const,
      default: 'pretty',
    })(),
    'max-depth': Flags.integer({
      summary: messages.getMessage('flags.max-depth.summary'),
      description: messages.getMessage('flags.max-depth.description'),
      default: 20,
    }),
  };

  public async run(): Promise<PathsResult | PathsResult[]> {
    const { flags } = await this.parse(AgencyPaths);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    try {
      const files = resolveTargetFiles({
        file: flags.file,
        scanPath: flags.path,
        dataDir: this.config.dataDir,
      });

      const results: PathsResult[] = [];

      for (const filePath of files) {
        if (files.length > 1) {
          this.log(ansis.bold.dim(`\n─── ${path.relative(process.cwd(), filePath)} ───`));
        }

        const source = fs.readFileSync(filePath, 'utf-8');
        const result = this.computePaths(source, flags['max-depth'], flags.format, ux, filePath);
        results.push(result);
      }

      return files.length === 1 ? results[0] : results;
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.pathsFailure', [error.message]);
      }
      throw error;
    }
  }

  private computePaths(source: string, maxDepth: number, format: string, ux: Ux, filePath: string): PathsResult {
    const graphJson: GraphExport = JSON.parse(graphLib.export_graph_json(source));

    const adjacency = new Map<string, { transitions: string[]; delegates: string[] }>();
    for (const topic of graphJson.topics) {
      adjacency.set(topic.name, {
        transitions: topic.transitions_to,
        delegates: topic.delegates_to,
      });
    }

    const allTopicNames = new Set(graphJson.topics.map(t => t.name));
    const entryTopics = graphJson.topics.filter(t => t.is_entry);
    const startName = entryTopics.length > 0 ? entryTopics[0].name : 'start_agent';

    const paths: PathEntry[] = [];

    const dfs = (current: string, currentPath: string[], edgeTypes: string[], depth: number): void => {
      if (depth > maxDepth) return;

      const adj = adjacency.get(current);
      const outgoing: Array<{ name: string; type: string }> = [];

      if (adj) {
        for (const dest of adj.transitions) {
          outgoing.push({ name: dest, type: 'transitions' });
        }
        for (const dest of adj.delegates) {
          outgoing.push({ name: dest, type: 'delegates' });
        }
      }

      if (outgoing.length === 0) {
        paths.push({ nodes: [...currentPath], edge_types: [...edgeTypes], has_cycle: false });
        return;
      }

      for (const { name: next, type: edgeType } of outgoing) {
        if (currentPath.includes(next)) {
          paths.push({
            nodes: [...currentPath, next],
            edge_types: [...edgeTypes, edgeType],
            has_cycle: true,
          });
        } else {
          dfs(next, [...currentPath, next], [...edgeTypes, edgeType], depth + 1);
        }
      }
    };

    dfs(startName, [startName], [], 0);

    const reachable = new Set<string>();
    for (const p of paths) {
      for (const n of p.nodes) reachable.add(n);
    }
    const unreachable = [...allTopicNames].filter(n => !reachable.has(n) && n !== startName);

    const file = path.relative(process.cwd(), filePath);
    const result: PathsResult = { file, paths, unreachable, total_paths: paths.length };

    if (format === 'json') {
      this.log(JSON.stringify(result, null, 2));
    } else {
      this.displayPretty(ux, result);
    }

    return result;
  }

  private displayPretty(ux: Ux, result: PathsResult): void {
    ux.styledHeader(`Execution Paths (${result.total_paths} total)`);
    this.log('');

    for (const p of result.paths) {
      const parts: string[] = [];
      for (let i = 0; i < p.nodes.length; i++) {
        const node = p.nodes[i];
        if (i === 0) {
          parts.push(ansis.bold.white(node));
        } else {
          const edgeType = p.edge_types[i - 1];
          const arrow = edgeType === 'delegates' ? ansis.blue(' ⇒ ') : ansis.dim(' → ');
          parts.push(arrow + ansis.cyan(node));
        }
      }
      const line = parts.join('');
      if (p.has_cycle) {
        this.log(`  ${line} ${ansis.yellow('↩ (cycle)')}`);
      } else {
        this.log(`  ${line}`);
      }
    }

    if (result.unreachable.length > 0) {
      this.log('');
      this.log(ansis.yellow.bold('Unreachable topics:'));
      for (const t of result.unreachable) {
        this.log(`  ${ansis.yellow('!')} ${ansis.cyan(t)}`);
      }
    }

    this.log('');
    this.log(ansis.dim('Legend: ') + ansis.dim('→') + ansis.dim(' transition | ') + ansis.blue('⇒') + ansis.dim(' delegate | ') + ansis.yellow('↩') + ansis.dim(' cycle'));
  }
}
