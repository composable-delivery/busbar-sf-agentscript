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
    verbose: Flags.boolean({
      summary: 'Show detailed output with tables for config, variables, system, and topics.',
      default: false,
    }),
  };

  public async run(): Promise<ParseResult | ParseResult[]> {
    const { flags } = await this.parse(AgentscriptParse);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

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

    const results: ParseResult[] = [];
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
        const startTime = performance.now();
        const ast = parser.parse_agent(source);
        const elapsed = (performance.now() - startTime).toFixed(2);

        if (flags.format === 'json') {
          this.log(JSON.stringify({ file, ast }, null, 2));
          results.push({ file, ast });
          continue;
        }

        if (flags.verbose) {
          this.displayVerbose(ux, ast, file, elapsed);
        } else {
          this.displayCompact(ast, file, elapsed);
        }

        results.push({ file, ast });
      } catch (e) {
        fileErrors.push({ file, error: e instanceof Error ? e.message : String(e) });
      }
    }

    this.displayErrors(fileErrors);

    if (results.length === 0 && fileErrors.length > 0) {
      this.exit(1);
    }

    return files.length === 1 ? results[0] : results;
  }

  private displayCompact(ast: ParsedAgentScript, file: string, elapsed: string): void {
    const config = (ast.config?.node || ast.config) as any;
    const agentName = config?.agent_name?.node || config?.agent_name?.value || path.basename(file, '.agent');
    const topicCount = ast.topics
      ? Array.isArray(ast.topics) ? ast.topics.length : Object.keys(ast.topics).length
      : 0;
    const varCount = ast.variables ? Object.keys(ast.variables).length : 0;
    this.log(
      `${ansis.green('✓')} ${ansis.bold(agentName)}  ${ansis.dim('•')}  ` +
      `${ansis.cyan(String(topicCount))} topics  ${ansis.dim('•')}  ` +
      `${ansis.green(String(varCount))} variables  ${ansis.dim(`(${elapsed}ms)`)}`
    );
  }

  private displayVerbose(ux: Ux, ast: ParsedAgentScript, file: string, elapsed: string): void {
    ux.styledHeader(`Parsed: ${path.basename(file)}`);
    this.log('');

    if (ast.config) {
      const config = ast.config.node || ast.config;
      const configData: Array<{ property: string; value: string }> = [];

      if ((config as any).agent_name) {
        const name = (config as any).agent_name.node || (config as any).agent_name.value || (config as any).agent_name;
        configData.push({ property: ansis.cyan('Agent Name'), value: ansis.bold(name) });
      }
      if ((config as any).agent_label) {
        const label = (config as any).agent_label.node || (config as any).agent_label.value || (config as any).agent_label;
        configData.push({ property: ansis.cyan('Agent Label'), value: label });
      }
      if ((config as any).description) {
        const desc = (config as any).description.node || (config as any).description.value || (config as any).description;
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

      const topicsArray = Array.isArray(ast.topics) ? ast.topics : Object.values(ast.topics);
      for (const topic of topicsArray) {
        const topicNode = (topic as any).node || topic;
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
  }

  private displayErrors(fileErrors: Array<{ file: string; error: string }>): void {
    if (fileErrors.length === 0) return;
    this.log('');
    this.log(ansis.red.bold(`${fileErrors.length} file${fileErrors.length === 1 ? '' : 's'} failed to parse:`));
    for (const { file, error } of fileErrors) {
      this.log(`  ${ansis.red('✗')} ${ansis.bold(file)}: ${ansis.dim(error)}`);
    }
  }
}
