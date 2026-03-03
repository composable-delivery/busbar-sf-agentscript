import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import { Args } from '@oclif/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from '../../wasm-loader.js';
import { resolveTargetFiles } from '../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.query');

interface NodeRepr {
  node_type: string;
  name: string | null;
  topic: string | null;
  target: string | null;
}

interface UsageInfoRepr {
  location: string;
  node_type: string;
  topic: string | null;
  context: string | null;
}

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

interface GraphExport {
  nodes: GraphExportNode[];
  edges: GraphExportEdge[];
}

interface QueryResult {
  file: string;
  queryPath: string;
  result: unknown;
}

export default class AgentscriptQuery extends SfCommand<QueryResult | QueryResult[]> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  public static readonly args = {
    queryPath: Args.string({
      required: true,
      description: 'Query path: /topics/<name>, /variables/<name>, /actions/<name>, or dot.notation for raw AST access',
    }),
  };

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
      char: 'o',
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['json', 'pretty'] as const,
      default: 'pretty',
    })(),
  };

  public async run(): Promise<QueryResult | QueryResult[]> {
    const { flags, args } = await this.parse(AgentscriptQuery);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });
    const queryPath = args.queryPath as string;

    const files = resolveTargetFiles({
      file: flags.file,
      scanPath: flags.path,
      dataDir: this.config.dataDir,
    });

    // Read all files in parallel
    const fileReads = await Promise.all(
      files.map(async (filePath) => {
        try {
          const source = await fs.promises.readFile(filePath, 'utf-8');
          return { filePath, source, ok: true as const };
        } catch (e) {
          return { filePath, source: '', ok: false as const, error: e instanceof Error ? e.message : String(e) };
        }
      })
    );

    const results: QueryResult[] = [];
    const fileErrors: Array<{ file: string; error: string }> = [];

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
        const { source } = fileRead;
        let result: unknown;
        let skipped = false;

        if (queryPath.startsWith('/topics/')) {
          const name = queryPath.slice(8);
          result = this.queryTopic(source, name, ux, flags.format);
        } else if (queryPath.startsWith('/variables/')) {
          const name = queryPath.slice(11);
          result = this.queryVariable(source, name, ux, flags.format);
        } else if (queryPath.startsWith('/actions/')) {
          const name = queryPath.slice(9);
          const actionResult = this.queryAction(source, name, ux, flags.format, files.length > 1);
          if (actionResult === undefined) {
            skipped = true;
          } else {
            result = actionResult;
          }
        } else {
          const ast = parser.parse_agent(source);
          result = this.traverseAst(ast, queryPath);
          if (flags.format === 'json') {
            this.log(JSON.stringify({ file, queryPath, result }, null, 2));
          } else {
            this.displayAstResult(ux, queryPath, result);
          }
        }

        if (!skipped) {
          if (queryPath.startsWith('/topics/') || queryPath.startsWith('/variables/') || queryPath.startsWith('/actions/')) {
            if (flags.format === 'json') {
              this.log(JSON.stringify({ file, queryPath, result }, null, 2));
            }
          }
          results.push({ file, queryPath, result });
        }
      } catch (e) {
        if (files.length === 1) {
          // Single-file: preserve original throw behavior
          if (e instanceof Error) {
            throw messages.createError('error.queryFailure', [e.message]);
          }
          throw e;
        }
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

  private queryTopic(source: string, name: string, ux: Ux, format: string): { topic: string; incoming: NodeRepr[]; outgoing: NodeRepr[] } {
    let incoming: NodeRepr[] = [];
    let outgoing: NodeRepr[] = [];
    try {
      incoming = parser.find_topic_usages(source, name);
      outgoing = parser.find_topic_transitions(source, name);
    } catch {
      // Topic not found — leave as empty
    }
    const result = { topic: name, incoming, outgoing };
    if (format === 'pretty') {
      this.displayTopic(ux, name, incoming, outgoing);
    }
    return result;
  }

  private queryVariable(source: string, name: string, ux: Ux, format: string): { variable: string; readers: UsageInfoRepr[]; writers: UsageInfoRepr[] } {
    let readers: UsageInfoRepr[] = [];
    let writers: UsageInfoRepr[] = [];
    try {
      const usagesJson: string = parser.find_variable_usages(source, name);
      const usages = JSON.parse(usagesJson);
      readers = usages.readers;
      writers = usages.writers;
    } catch {
      // Variable not found — leave as empty
    }
    const result = { variable: name, readers, writers };
    if (format === 'pretty') {
      this.displayVariable(ux, name, readers, writers);
    }
    return result;
  }

  private queryAction(
    source: string,
    name: string,
    ux: Ux,
    format: string,
    multiFile: boolean,
  ): { action: string; target: string | null; invocations: { reasoning_action: string; topic: string }[] } | undefined {
    const graphJson: GraphExport = JSON.parse(parser.export_graph_json(source));
    const actionNode = graphJson.nodes.find(n => n.node_type === 'action_def' && n.name === name);

    if (!actionNode) {
      if (multiFile) {
        this.log(ansis.dim(`  (action '${name}' not found in this file)`));
        return undefined;
      }
      throw new Error(`Action '${name}' not found in agent file`);
    }

    const invocations: { reasoning_action: string; topic: string }[] = [];
    for (const edge of graphJson.edges) {
      if (edge.edge_type === 'invokes' && edge.target === actionNode.id) {
        const srcNode = graphJson.nodes.find(n => n.id === edge.source);
        if (srcNode && srcNode.node_type === 'reasoning_action' && srcNode.name) {
          invocations.push({ reasoning_action: srcNode.name, topic: srcNode.topic ?? '-' });
        }
      }
    }

    const result = { action: name, target: actionNode.target, invocations };
    if (format === 'pretty') {
      this.displayAction(ux, result);
    }
    return result;
  }

  private traverseAst(ast: any, queryPath: string): unknown {
    // Support both dot-notation (topics.name) and slash-notation (/ast/topics/name)
    const normalizedPath = queryPath.startsWith('/')
      ? queryPath.slice(1).replace(/\//g, '.')
      : queryPath;
    const parts = normalizedPath.split('.').filter(p => p);
    let current = ast;

    for (const part of parts) {
      if (/^\d+$/.test(part)) {
        const index = parseInt(part, 10);
        if (Array.isArray(current)) {
          current = current[index];
        } else {
          throw new Error(`Cannot use array index '${part}' on non-array`);
        }
      } else {
        if (current && typeof current === 'object') {
          if (current.node && !current[part]) {
            current = current.node;
          }
          current = current[part];
        } else {
          throw new Error(`Cannot access property '${part}' on ${typeof current}`);
        }
      }

      if (current === undefined) {
        throw new Error(`Path '${queryPath}' not found in AST`);
      }
    }

    return current;
  }

  private displayTopic(ux: Ux, name: string, incoming: NodeRepr[], outgoing: NodeRepr[]): void {
    ux.styledHeader(`Topic: ${name}`);
    this.log('');

    this.log(ansis.bold.cyan('Incoming') + ansis.dim(` (${incoming.length} references)`));
    if (incoming.length === 0) {
      this.log(ansis.dim('  No incoming references — this may be an entry point.'));
    } else {
      ux.table({
        data: incoming.map(n => ({
          type: ansis.yellow(n.node_type),
          name: ansis.bold(n.name ?? '-'),
          topic: ansis.cyan(n.topic ?? '-'),
        })),
        columns: [
          { key: 'type', name: 'Type' },
          { key: 'name', name: 'Name' },
          { key: 'topic', name: 'From Topic' },
        ],
      });
    }
    this.log('');

    this.log(ansis.bold.cyan('Outgoing') + ansis.dim(` (${outgoing.length} transitions)`));
    if (outgoing.length === 0) {
      this.log(ansis.dim('  No outgoing transitions — this topic does not transition elsewhere.'));
    } else {
      ux.table({
        data: outgoing.map(n => ({
          name: ansis.bold(n.name ?? '-'),
          type: ansis.yellow(n.node_type),
        })),
        columns: [
          { key: 'name', name: 'Topic' },
          { key: 'type', name: 'Type' },
        ],
      });
    }
    this.log('');
  }

  private displayVariable(ux: Ux, name: string, readers: UsageInfoRepr[], writers: UsageInfoRepr[]): void {
    ux.styledHeader(`Variable: ${name}`);
    this.log('');

    this.log(ansis.bold.green('Reads') + ansis.dim(` (${readers.length})`));
    if (readers.length === 0) {
      this.log(ansis.dim('  No readers found.'));
    } else {
      ux.table({
        data: readers.map(u => ({
          topic: ansis.cyan(u.topic ?? '-'),
          action: ansis.bold(u.location),
          type: ansis.yellow(u.node_type),
          context: ansis.dim(u.context ?? '-'),
        })),
        columns: [
          { key: 'topic', name: 'Topic' },
          { key: 'action', name: 'Action' },
          { key: 'type', name: 'Type' },
          { key: 'context', name: 'Context' },
        ],
      });
    }
    this.log('');

    this.log(ansis.bold.magenta('Writes') + ansis.dim(` (${writers.length})`));
    if (writers.length === 0) {
      this.log(ansis.dim('  No writers found.'));
    } else {
      ux.table({
        data: writers.map(u => ({
          topic: ansis.cyan(u.topic ?? '-'),
          action: ansis.bold(u.location),
          type: ansis.yellow(u.node_type),
          context: ansis.dim(u.context ?? '-'),
        })),
        columns: [
          { key: 'topic', name: 'Topic' },
          { key: 'action', name: 'Action' },
          { key: 'type', name: 'Type' },
          { key: 'context', name: 'Context' },
        ],
      });
    }
    this.log('');
  }

  private displayAction(
    ux: Ux,
    result: { action: string; target: string | null; invocations: { reasoning_action: string; topic: string }[] },
  ): void {
    ux.styledHeader(`Action: ${result.action}`);
    this.log('');
    this.log(`  ${ansis.dim('Target:')} ${result.target ? ansis.bold(result.target) : ansis.dim('(none)')}`);
    this.log(`  ${ansis.dim('Topic:')} ${ansis.cyan(result.invocations[0]?.topic ?? '-')}`);
    this.log('');

    this.log(ansis.bold.cyan('Invoked by') + ansis.dim(` (${result.invocations.length} reasoning steps)`));
    if (result.invocations.length === 0) {
      this.log(ansis.dim('  No reasoning steps invoke this action.'));
    } else {
      ux.table({
        data: result.invocations.map(inv => ({
          reasoning: ansis.bold(inv.reasoning_action),
          topic: ansis.cyan(inv.topic),
        })),
        columns: [
          { key: 'reasoning', name: 'Reasoning Step' },
          { key: 'topic', name: 'Topic' },
        ],
      });
    }
    this.log('');
  }

  private displayAstResult(ux: Ux, queryPath: string, data: unknown): void {
    ux.styledHeader('Query Result');
    this.log('');
    this.log(`  ${ansis.cyan('Path:')} ${ansis.bold(queryPath)}`);
    this.log('');

    if (data === null || data === undefined) {
      this.log(`  ${ansis.cyan('Result:')} ${ansis.dim('null')}`);
      return;
    }

    const type = typeof data;

    if (type === 'string' || type === 'number' || type === 'boolean') {
      this.log(`  ${ansis.cyan('Type:')} ${ansis.yellow(type)}`);
      this.log(`  ${ansis.cyan('Value:')} ${ansis.green(String(data))}`);
      return;
    }

    if (Array.isArray(data)) {
      this.log(`  ${ansis.cyan('Type:')} ${ansis.yellow('array')}`);
      this.log(`  ${ansis.cyan('Length:')} ${ansis.bold(String(data.length))}`);
      this.log('');

      const tableData = data.slice(0, 10).map((item, index) => {
        const preview = typeof item === 'object' && item !== null
          ? JSON.stringify(item).substring(0, 60) + (JSON.stringify(item).length > 60 ? '...' : '')
          : String(item);
        return { index: ansis.dim(`[${index}]`), value: preview };
      });

      ux.table({
        data: tableData,
        columns: [
          { key: 'index', name: 'Index' },
          { key: 'value', name: 'Value' },
        ],
      });

      if (data.length > 10) {
        this.log(ansis.dim(`  ... and ${data.length - 10} more items`));
      }
      return;
    }

    if (type === 'object') {
      const obj = data as Record<string, unknown>;

      if (obj.node && obj.span) {
        this.log(`  ${ansis.cyan('Type:')} ${ansis.yellow('spanned node')}`);
        this.log(`  ${ansis.cyan('Value:')} ${ansis.green(JSON.stringify(obj.node))}`);
        this.log(`  ${ansis.cyan('Location:')} ${ansis.dim(`${(obj.span as any).start}-${(obj.span as any).end}`)}`);
        return;
      }

      const keys = Object.keys(obj);
      this.log(`  ${ansis.cyan('Type:')} ${ansis.yellow('object')}`);
      this.log(`  ${ansis.cyan('Properties:')} ${ansis.bold(String(keys.length))}`);
      this.log('');

      ux.table({
        data: keys.map(key => ({
          property: ansis.bold(key),
          value: typeof obj[key] === 'object' && obj[key] !== null ? ansis.dim('[object]') : String(obj[key]),
        })),
        columns: [
          { key: 'property', name: 'Property' },
          { key: 'value', name: 'Value' },
        ],
      });
    }
  }
}
