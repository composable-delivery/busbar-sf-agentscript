import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import { execSync } from 'child_process';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graph from '../../wasm-loader.js';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from '../../wasm-loader.js';
import { resolveTargetFiles } from '../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.deps');

interface ActionParameter {
  name: string;
  type: string;
  description?: string;
  isRequired?: boolean;
}

interface ActionInterface {
  name: string;
  description?: string;
  targetType: 'flow' | 'apex' | 'prompt' | 'unknown';
  targetName: string;
  location: string;
  inputs: ActionParameter[];
  outputs: ActionParameter[];
}

interface DependencyReport {
  sobjects: string[];
  fields: string[];
  flows: string[];
  apex_classes: string[];
  knowledge_bases: string[];
  connections: string[];
  prompt_templates: string[];
  external_services: string[];
  all_dependencies: Array<{
    dep_type: { [key: string]: string };
    used_in: string;
    action_name: string;
    span: [number, number];
  }>;
  by_type: { [key: string]: unknown[] };
  by_topic: { [key: string]: unknown[] };
}

interface DepsResult {
  file: string;
  report: DependencyReport;
  interfaces: ActionInterface[];
  summary: {
    total: number;
    by_category: { [key: string]: number };
  };
}

interface GroupedDepEntry {
  dependency: string;
  type: string;
  agents: string[];
}

export default class AgentscriptDeps extends SfCommand<DepsResult | DepsResult[] | GroupedDepEntry[]> {
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
      options: ['json', 'table', 'summary'] as const,
      default: 'table',
    })(),
    type: Flags.option({
      char: 't',
      summary: messages.getMessage('flags.type.summary'),
      description: messages.getMessage('flags.type.description'),
      options: ['all', 'sobjects', 'flows', 'apex', 'knowledge', 'connections'] as const,
      default: 'all',
    })(),
    group: Flags.option({
      summary: messages.getMessage('flags.group.summary'),
      description: messages.getMessage('flags.group.description'),
      options: ['file', 'dependency'] as const,
      default: 'file',
    })(),
    retrieve: Flags.boolean({
      summary: messages.getMessage('flags.retrieve.summary'),
      description: messages.getMessage('flags.retrieve.description'),
      default: false,
    }),
    'target-org': Flags.optionalOrg({
      summary: messages.getMessage('flags.target-org.summary'),
      description: messages.getMessage('flags.target-org.description'),
    }),
  };

  public async run(): Promise<DepsResult | DepsResult[] | GroupedDepEntry[]> {
    const { flags } = await this.parse(AgentscriptDeps);

    try {
      const files = resolveTargetFiles({
        file: flags.file,
        scanPath: flags.path,
        dataDir: this.config.dataDir,
      });

      const results: DepsResult[] = [];

      for (const filePath of files) {
        if (files.length > 1 && flags.group !== 'dependency') {
          this.log(ansis.bold.dim(`\n─── ${path.relative(process.cwd(), filePath)} ───`));
        }

        const file = path.relative(process.cwd(), filePath);
        const source = fs.readFileSync(filePath, 'utf-8');

        const report = graph.extract_dependencies(source) as DependencyReport;
        const ast = parser.parse_agent(source);
        const interfaces = this.extractActionInterfaces(ast);

        const summary = {
          total:
            report.sobjects.length +
            report.fields.length +
            report.flows.length +
            report.apex_classes.length +
            report.knowledge_bases.length +
            report.connections.length +
            report.prompt_templates.length +
            report.external_services.length,
          by_category: {
            sobjects: report.sobjects.length,
            fields: report.fields.length,
            flows: report.flows.length,
            apex_classes: report.apex_classes.length,
            knowledge_bases: report.knowledge_bases.length,
            connections: report.connections.length,
            prompt_templates: report.prompt_templates.length,
            external_services: report.external_services.length,
          },
        };

        if (flags.group !== 'dependency') {
          if (flags.format === 'json') {
            this.log(JSON.stringify({ file, report, interfaces, summary }, null, 2));
          } else if (flags.format === 'summary') {
            this.displaySummary(summary);
          } else {
            this.displayTable(report, interfaces, flags.type as string);
          }
        }

        if (flags.retrieve) {
          if (!flags['target-org']) {
            throw messages.createError('error.retrieveRequiresOrg');
          }
          const org = flags['target-org'];
          const orgAlias = org.getUsername() ?? '';
          this.runRetrieval(report, orgAlias);
        }

        results.push({ file, report, interfaces, summary });
      }

      if (flags.group === 'dependency') {
        const grouped = this.groupByDependency(results);
        if (flags.format === 'json') {
          this.log(JSON.stringify(grouped, null, 2));
        } else {
          this.displayGrouped(grouped);
        }
        return grouped;
      }

      return files.length === 1 ? results[0] : results;
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.extractionFailure', [error.message]);
      }
      throw error;
    }
  }

  private groupByDependency(results: DepsResult[]): GroupedDepEntry[] {
    const map = new Map<string, { type: string; agents: Set<string> }>();

    const addDeps = (deps: string[], type: string, file: string) => {
      for (const dep of deps) {
        const key = `${type}:${dep}`;
        if (!map.has(key)) {
          map.set(key, { type, agents: new Set() });
        }
        map.get(key)!.agents.add(file);
      }
    };

    for (const result of results) {
      const { report, file } = result;
      addDeps(report.sobjects, 'sobject', file);
      addDeps(report.fields, 'field', file);
      addDeps(report.flows, 'flow', file);
      addDeps(report.apex_classes, 'apex_class', file);
      addDeps(report.knowledge_bases, 'knowledge_base', file);
      addDeps(report.connections, 'connection', file);
      addDeps(report.prompt_templates, 'prompt_template', file);
      addDeps(report.external_services, 'external_service', file);
    }

    return Array.from(map.entries())
      .map(([key, val]) => ({
        dependency: key.slice(key.indexOf(':') + 1),
        type: val.type,
        agents: Array.from(val.agents).sort(),
      }))
      .sort((a, b) => a.type.localeCompare(b.type) || a.dependency.localeCompare(b.dependency));
  }

  private displayGrouped(grouped: GroupedDepEntry[]): void {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });
    ux.styledHeader(`Dependencies by Resource (${grouped.length} unique)`);
    this.log('');

    const byType = new Map<string, GroupedDepEntry[]>();
    for (const entry of grouped) {
      const list = byType.get(entry.type) ?? [];
      list.push(entry);
      byType.set(entry.type, list);
    }

    const typeColors: Record<string, (s: string) => string> = {
      sobject: ansis.cyan,
      field: ansis.blue,
      flow: ansis.cyanBright,
      apex_class: ansis.magentaBright,
      knowledge_base: ansis.yellow,
      connection: ansis.green,
      prompt_template: ansis.greenBright,
      external_service: ansis.red,
    };

    for (const [type, entries] of byType) {
      const colorFn = typeColors[type] ?? ansis.white;
      this.log(colorFn.bold(type.replace(/_/g, ' ').replace(/\b\w/g, c => c.toUpperCase())) + ansis.dim(` (${entries.length})`));
      for (const entry of entries) {
        this.log(`  ${ansis.green('▸')} ${ansis.bold(entry.dependency)}`);
        for (const agent of entry.agents) {
          this.log(`      ${ansis.dim('•')} ${ansis.dim(agent)}`);
        }
      }
      this.log('');
    }
  }

  private runRetrieval(report: DependencyReport, orgAlias: string): void {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    const metadataArgs: string[] = [];
    for (const flow of report.flows) {
      metadataArgs.push(`Flow:${flow}`);
    }
    for (const cls of report.apex_classes) {
      metadataArgs.push(`ApexClass:${cls}`);
    }
    for (const prompt of report.prompt_templates) {
      metadataArgs.push(`LightningComponentBundle:${prompt}`);
    }

    if (metadataArgs.length === 0) {
      if (!this.jsonEnabled()) {
        this.log(ansis.dim('  No retrievable metadata found (flows, apex classes, or prompt templates).'));
      }
      return;
    }

    if (!this.jsonEnabled()) {
      ux.styledHeader('Retrieving Metadata');
      this.log('');
      for (const m of metadataArgs) {
        this.log(`  ${ansis.green('•')} ${m}`);
      }
      this.log('');
    }

    const metadataFlags = metadataArgs.map(m => `"${m}"`).join(' ');
    const cmd = `sf project retrieve start --metadata ${metadataFlags} --target-org ${orgAlias} --json`;

    try {
      const output = execSync(cmd, {
        encoding: 'utf-8',
        stdio: ['pipe', 'pipe', 'pipe'],
      });

      const json = JSON.parse(output);
      if (!this.jsonEnabled()) {
        if (json.status === 0) {
          this.log(ansis.green('  ✓ Retrieval complete'));
          if (json.result?.files) {
            for (const f of json.result.files) {
              this.log(`    ${ansis.dim(f.filePath ?? f)}`);
            }
          }
        } else {
          this.log(ansis.yellow('  ! Retrieval completed with warnings'));
        }
      }
    } catch (error: unknown) {
      const execError = error as { stdout?: string; message?: string };
      const msg = execError.stdout
        ? (() => { try { return JSON.parse(execError.stdout).message; } catch { return execError.message; } })()
        : execError.message;
      if (!this.jsonEnabled()) {
        this.log(ansis.red(`  ✗ Retrieval failed: ${msg ?? 'Unknown error'}`));
      }
    }
  }

  private extractActionInterfaces(ast: any): ActionInterface[] {
    const interfaces: ActionInterface[] = [];

    // Extract from start_agent
    if (ast.start_agent) {
      const startNode = ast.start_agent.node || ast.start_agent;
      if (startNode.actions) {
        const actionsBlock = startNode.actions.node || startNode.actions;
        // Actions are in actionsBlock.actions array
        const actionsArray = actionsBlock.actions || actionsBlock;
        if (Array.isArray(actionsArray)) {
          for (const actionDef of actionsArray) {
            const iface = this.parseActionDef(actionDef, 'start_agent');
            if (iface) interfaces.push(iface);
          }
        }
      }
    }

    // Extract from topics
    if (ast.topics) {
      for (const topic of ast.topics) {
        const topicNode = topic.node || topic;
        const topicName = topicNode.name?.node || topicNode.name || 'unknown';

        if (topicNode.actions) {
          const actionsBlock = topicNode.actions.node || topicNode.actions;
          // Actions are in actionsBlock.actions array
          const actionsArray = actionsBlock.actions || actionsBlock;
          if (Array.isArray(actionsArray)) {
            for (const actionDef of actionsArray) {
              const iface = this.parseActionDef(actionDef, `topic.${topicName}`);
              if (iface) interfaces.push(iface);
            }
          }
        }
      }
    }

    return interfaces;
  }

  private parseActionDef(def: any, location: string): ActionInterface | null {
    const node = def.node || def;

    // Get the action name
    const name = node.name?.node || node.name;
    if (!name) return null;

    // Get the target - skip if no target (not a flow/apex/prompt action)
    if (!node.target) return null;

    const targetStr = node.target.node || node.target;
    if (typeof targetStr !== 'string') return null;

    const { targetType, targetName } = this.parseTarget(targetStr);

    return {
      name,
      description: node.description?.node || node.description,
      targetType,
      targetName,
      location,
      inputs: this.parseParameters(node.inputs),
      outputs: this.parseParameters(node.outputs),
    };
  }

  private parseTarget(target: string): { targetType: 'flow' | 'apex' | 'prompt' | 'unknown'; targetName: string } {
    if (target.startsWith('flow://')) {
      return { targetType: 'flow', targetName: target.substring(7) };
    } else if (target.startsWith('apex://')) {
      return { targetType: 'apex', targetName: target.substring(7) };
    } else if (target.startsWith('prompt://')) {
      return { targetType: 'prompt', targetName: target.substring(9) };
    }
    return { targetType: 'unknown', targetName: target };
  }

  private parseParameters(params: any): ActionParameter[] {
    if (!params) return [];

    const paramsNode = params.node || params;
    if (!Array.isArray(paramsNode)) return [];

    return paramsNode.map((p: any) => {
      const param = p.node || p;
      return {
        name: param.name?.node || param.name || 'unknown',
        type: this.normalizeType(param.ty?.node || param.ty),
        description: param.description?.node || param.description,
        isRequired: param.is_required?.node ?? param.is_required,
      };
    });
  }

  private normalizeType(ty: any): string {
    if (typeof ty === 'string') return ty.toLowerCase();
    if (ty && typeof ty === 'object') {
      if (ty.List) return `list[${this.normalizeType(ty.List)}]`;
      return JSON.stringify(ty).toLowerCase();
    }
    return 'unknown';
  }

  private displaySummary(summary: { total: number; by_category: { [key: string]: number } }): void {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    ux.styledHeader(`Dependency Summary (${summary.total} total)`);
    this.log('');

    const categories = [
      { name: 'SObjects', count: summary.by_category.sobjects, color: ansis.cyan },
      { name: 'Fields', count: summary.by_category.fields, color: ansis.blue },
      { name: 'Flows', count: summary.by_category.flows, color: ansis.cyanBright },
      { name: 'Apex Classes', count: summary.by_category.apex_classes, color: ansis.magentaBright },
      { name: 'Knowledge Bases', count: summary.by_category.knowledge_bases, color: ansis.yellow },
      { name: 'Connections', count: summary.by_category.connections, color: ansis.green },
      { name: 'Prompt Templates', count: summary.by_category.prompt_templates, color: ansis.greenBright },
      { name: 'External Services', count: summary.by_category.external_services, color: ansis.red },
    ];

    const tableData = categories
      .filter(cat => cat.count > 0)
      .map(cat => ({
        category: cat.color(cat.name),
        count: ansis.bold(String(cat.count)),
      }));

    if (tableData.length > 0) {
      ux.table({
        data: tableData,
        columns: [
          { key: 'category', name: 'Category' },
          { key: 'count', name: 'Count' },
        ],
      });
    } else {
      this.log(`  ${ansis.dim('No dependencies found')}`);
    }
    this.log('');
  }

  private displayTable(report: DependencyReport, interfaces: ActionInterface[], type: string): void {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    ux.styledHeader('Salesforce Org Dependencies');
    this.log('');

    if (type === 'all' || type === 'sobjects') {
      if (report.sobjects.length > 0) {
        this.log(`${ansis.cyan.bold('SObjects')}`);
        for (const obj of report.sobjects) {
          this.log(`  ${ansis.green('•')} ${obj}`);
        }
        this.log('');
      }

      if (report.fields.length > 0) {
        this.log(`${ansis.blue.bold('Fields')}`);
        for (const field of report.fields) {
          this.log(`  ${ansis.green('•')} ${field}`);
        }
        this.log('');
      }
    }

    if (type === 'all' || type === 'flows') {
      const flowInterfaces = interfaces.filter(i => i.targetType === 'flow');
      if (flowInterfaces.length > 0) {
        this.log(`${ansis.cyanBright.bold('Flows')} ${ansis.dim(`(${flowInterfaces.length})`)}`);
        this.log('');
        for (const flow of flowInterfaces) {
          this.log(`  ${ansis.cyanBright('▸')} ${ansis.bold(flow.targetName)}`);
          if (flow.description) {
            this.log(`    ${ansis.dim(flow.description)}`);
          }
          this.displayParams(flow.inputs, flow.outputs);
          this.log('');
        }
      }
    }

    if (type === 'all' || type === 'apex') {
      const apexInterfaces = interfaces.filter(i => i.targetType === 'apex');
      if (apexInterfaces.length > 0) {
        this.log(`${ansis.magentaBright.bold('Apex Classes')} ${ansis.dim(`(${apexInterfaces.length})`)}`);
        this.log('');
        for (const apex of apexInterfaces) {
          this.log(`  ${ansis.magentaBright('▸')} ${ansis.bold(apex.targetName)}`);
          if (apex.description) {
            this.log(`    ${ansis.dim(apex.description)}`);
          }
          this.displayParams(apex.inputs, apex.outputs);
          this.log('');
        }
      }
    }

    if (type === 'all' || type === 'knowledge') {
      if (report.knowledge_bases.length > 0) {
        this.log(`${ansis.yellow.bold('Knowledge Bases')}`);
        for (const kb of report.knowledge_bases) {
          this.log(`  ${ansis.green('•')} ${kb}`);
        }
        this.log('');
      }
    }

    if (type === 'all' || type === 'connections') {
      if (report.connections.length > 0) {
        this.log(`${ansis.green.bold('Connections')}`);
        for (const conn of report.connections) {
          this.log(`  ${ansis.green('•')} ${conn}`);
        }
        this.log('');
      }
    }

    if (type === 'all') {
      const promptInterfaces = interfaces.filter(i => i.targetType === 'prompt');
      if (promptInterfaces.length > 0) {
        this.log(`${ansis.greenBright.bold('Prompt Templates')} ${ansis.dim(`(${promptInterfaces.length})`)}`);
        this.log('');
        for (const prompt of promptInterfaces) {
          this.log(`  ${ansis.greenBright('▸')} ${ansis.bold(prompt.targetName)}`);
          if (prompt.description) {
            this.log(`    ${ansis.dim(prompt.description)}`);
          }
          this.displayParams(prompt.inputs, prompt.outputs);
          this.log('');
        }
      }

      if (report.external_services.length > 0) {
        this.log(`${ansis.red.bold('External Services')}`);
        for (const svc of report.external_services) {
          this.log(`  ${ansis.green('•')} ${svc}`);
        }
        this.log('');
      }
    }

    // Legend
    this.log(ansis.dim('Legend: ') + ansis.red('*') + ansis.dim(' = required'));
  }

  private displayParams(inputs: ActionParameter[], outputs: ActionParameter[]): void {
    if (inputs.length > 0) {
      this.log(`    ${ansis.green('Inputs:')}`);
      for (const input of inputs) {
        const required = input.isRequired ? ansis.red('*') : ' ';
        const type = ansis.yellow(input.type);
        const desc = input.description ? ansis.dim(` - ${input.description}`) : '';
        this.log(`      ${required} ${input.name}: ${type}${desc}`);
      }
    }
    if (outputs.length > 0) {
      this.log(`    ${ansis.magenta('Outputs:')}`);
      for (const output of outputs) {
        const type = ansis.yellow(output.type);
        const desc = output.description ? ansis.dim(` - ${output.description}`) : '';
        this.log(`        ${output.name}: ${type}${desc}`);
      }
    }
  }
}
