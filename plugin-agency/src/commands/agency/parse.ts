import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from '../../wasm-loader.js';
import { resolveTargetFiles } from '../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.parse');

interface ParseResult {
  file: string;
  ast: ParsedAgentScript;
}

// Type definition for parsed AgentScript AST
interface ParsedAgentScript {
  config?: {
    node?: {
      agent_name?: { node?: string; value?: string };
      agent_label?: { node?: string; value?: string };
      description?: { node?: string; value?: string };
    };
    span?: { start: number; end: number };
  };
  variables?: Record<string, unknown>;
  system?: {
    node?: {
      messages?: unknown;
      instructions?: unknown;
    };
    span?: { start: number; end: number };
  };
  topics?: Record<string, unknown>;
}

export default class AgentscriptParse extends SfCommand<ParseResult | ParseResult[]> {
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
      char: 'o',
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['json', 'pretty'] as const,
      default: 'pretty',
    })(),
  };

  public async run(): Promise<ParseResult | ParseResult[]> {
    const { flags } = await this.parse(AgentscriptParse);

    try {
      const files = resolveTargetFiles({
        file: flags.file,
        scanPath: flags.path,
        dataDir: this.config.dataDir,
      });

      const results: ParseResult[] = [];

      for (const filePath of files) {
        if (files.length > 1) {
          this.log(ansis.bold.dim(`\n─── ${path.relative(process.cwd(), filePath)} ───`));
        }

        const source = fs.readFileSync(filePath, 'utf-8');
        const file = path.relative(process.cwd(), filePath);
        const startTime = performance.now();
        const ast = parser.parse_agent(source);
        const elapsed = (performance.now() - startTime).toFixed(2);

        if (flags.format === 'json') {
          this.log(JSON.stringify({ file, ast }, null, 2));
          results.push({ file, ast });
          continue;
        }

        {
        const ux = new Ux({ jsonEnabled: this.jsonEnabled() });
        ux.styledHeader(`Parsed: ${path.basename(filePath)}`);
        this.log('');

        if (ast.config) {
          const config = ast.config.node || ast.config;
          const configData: Array<{ property: string; value: string }> = [];

          if (config.agent_name) {
            const name = config.agent_name.node || config.agent_name.value || config.agent_name;
            configData.push({ property: ansis.cyan('Agent Name'), value: ansis.bold(name) });
          }
          if (config.agent_label) {
            const label = config.agent_label.node || config.agent_label.value || config.agent_label;
            configData.push({ property: ansis.cyan('Agent Label'), value: label });
          }
          if (config.description) {
            const desc = config.description.node || config.description.value || config.description;
            configData.push({ property: ansis.cyan('Description'), value: ansis.dim(desc) });
          }

          if (configData.length > 0) {
            ux.table({
              data: configData,
              columns: [
                { key: 'property', name: 'Configuration' },
                { key: 'value', name: 'Value' },
              ],
            });
            this.log('');
          }
        }

        if (ast.variables) {
          ux.styledHeader('Variables');
          const varData: Array<{ name: string; type: string; modifiers: string }> = [];

          for (const [name, variable] of Object.entries(ast.variables as Record<string, any>)) {
            const varNode = variable.node || variable;
            const varType = varNode.var_type?.node || varNode.var_type?.value || 'unknown';
            const mods: string[] = [];
            if (varNode.is_mutable) mods.push(ansis.yellow('mutable'));
            if (varNode.is_linked) mods.push(ansis.magenta('linked'));
            varData.push({
              name: ansis.bold(name),
              type: ansis.green(varType),
              modifiers: mods.join(', ') || ansis.dim('-'),
            });
          }

          ux.table({
            data: varData,
            columns: [
              { key: 'name', name: 'Name' },
              { key: 'type', name: 'Type' },
              { key: 'modifiers', name: 'Modifiers' },
            ],
          });
          this.log('');
        }

        if (ast.system) {
          ux.styledHeader('System');
          const sys = (ast.system.node || ast.system) as any;
          if (sys.messages) {
            const msgs = sys.messages.node || sys.messages;
            this.log(`  ${ansis.cyan('Messages:')} ${Object.keys(msgs).length} defined`);
          }
          if (sys.instructions) {
            this.log(`  ${ansis.cyan('Instructions:')} ${ansis.green('defined')}`);
          }
          this.log('');
        }

        if (ast.topics) {
          ux.styledHeader('Topics');
          const topicData: Array<{ name: string; description: string }> = [];

          // Topics can be an array or object - handle both
          const topicsArray = Array.isArray(ast.topics) ? ast.topics : Object.values(ast.topics);
          for (const topic of topicsArray) {
            const topicNode = topic.node || topic;
            const topicName = topicNode.name?.node || topicNode.name || 'unknown';
            const desc = topicNode.description?.node || topicNode.description || '';
            topicData.push({
              name: ansis.cyanBright(topicName),
              description: desc ? ansis.dim(desc) : ansis.dim('-'),
            });
          }

          ux.table({
            data: topicData,
            columns: [
              { key: 'name', name: 'Topic' },
              { key: 'description', name: 'Description' },
            ],
          });
          this.log('');
        }

        this.log(ansis.green('✓') + ' Parse successful ' + ansis.dim(`(${elapsed}ms)`));
        results.push({ file, ast });
        }
      }

      return files.length === 1 ? results[0] : results;

    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.parseFailure', [error.message]);
      }
      throw error;
    }
  }
}
