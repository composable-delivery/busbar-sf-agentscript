import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from 'busbar-sf-agentscript';

// After bundling, __dirname is lib/commands/agentscript-parser/ - go up 3 levels to plugin root
const pluginRoot = path.resolve(__dirname, '..', '..', '..');
Messages.importMessagesDirectory(pluginRoot);
const messages = Messages.loadMessages('sf-plugin-busbar-agency', 'agency.query');

interface QueryResult {
  data: unknown;
  path: string;
}

export default class AgentscriptQuery extends SfCommand<QueryResult> {
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
    path: Flags.string({
      char: 'p',
      summary: messages.getMessage('flags.path.summary'),
      description: messages.getMessage('flags.path.description'),
      required: true,
    }),
    format: Flags.option({
      char: 'o',
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['json', 'pretty'] as const,
      default: 'pretty',
    })(),
  };

  public async run(): Promise<QueryResult> {
    const { flags } = await this.parse(AgentscriptQuery);

    try {
      // Read and parse the AgentScript file
      const filePath = path.resolve(flags.file as string);
      const source = fs.readFileSync(filePath, 'utf-8');
      const ast = parser.parse_agent(source);

      // Query the AST using the provided path
      const queryPath = flags.path as string;
      const result = this.queryAst(ast, queryPath);

      if (flags.format === 'json') {
        this.log(JSON.stringify(result, null, 2));
      } else {
        this.displayPretty(queryPath, result);
      }

      return { data: result, path: queryPath };
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.queryFailure', [error.message]);
      }
      throw error;
    }
  }

  private queryAst(ast: any, queryPath: string): unknown {
    const parts = queryPath.split('.').filter(p => p);
    let current = ast;

    for (const part of parts) {
      // Handle array indices
      if (/^\d+$/.test(part)) {
        const index = parseInt(part, 10);
        if (Array.isArray(current)) {
          current = current[index];
        } else {
          throw new Error(`Cannot use array index '${part}' on non-array`);
        }
      } else {
        // Handle object keys
        if (current && typeof current === 'object') {
          // Check for .node wrapper
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

  private displayPretty(queryPath: string, data: unknown): void {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

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
        return {
          index: ansis.dim(`[${index}]`),
          value: preview,
        };
      });

      ux.table(tableData, {
        index: { header: 'Index' },
        value: { header: 'Value' },
      });

      if (data.length > 10) {
        this.log(ansis.dim(`  ... and ${data.length - 10} more items`));
      }
      return;
    }

    if (type === 'object') {
      const obj = data as Record<string, unknown>;

      // Check if this is a spanned node
      if (obj.node && obj.span) {
        this.log(`  ${ansis.cyan('Type:')} ${ansis.yellow('spanned node')}`);
        this.log(`  ${ansis.cyan('Value:')} ${ansis.green(JSON.stringify(obj.node))}`);
        this.log(`  ${ansis.cyan('Location:')} ${ansis.dim(`${(obj.span as any).start}-${(obj.span as any).end}`)}`);
        return;
      }

      // Display object keys
      const keys = Object.keys(obj);
      this.log(`  ${ansis.cyan('Type:')} ${ansis.yellow('object')}`);
      this.log(`  ${ansis.cyan('Properties:')} ${ansis.bold(String(keys.length))}`);
      this.log('');

      const tableData = keys.map(key => {
        const value = obj[key];
        const valueStr = typeof value === 'object' && value !== null
          ? ansis.dim('[object]')
          : String(value);
        return {
          property: ansis.bold(key),
          value: valueStr,
        };
      });

      ux.table(tableData, {
        property: { header: 'Property' },
        value: { header: 'Value' },
      });
    }
  }
}
