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
const messages = Messages.loadMessages('sf-plugin-busbar-agency', 'agency.actions');

interface ActionParameter {
  name: string;
  type: string;
  description?: string;
  label?: string;
  isRequired?: boolean;
  isDisplayable?: boolean;
  filterFromAgent?: boolean;
}

interface ActionInterface {
  name: string;
  description?: string;
  label?: string;
  target: string;
  targetType: 'flow' | 'apex' | 'prompt' | 'unknown';
  targetName: string;
  location: string; // topic or start_agent where it's defined
  requireUserConfirmation?: boolean;
  includeProgressIndicator?: boolean;
  progressIndicatorMessage?: string;
  inputs: ActionParameter[];
  outputs: ActionParameter[];
}

interface ActionsResult {
  actions: ActionInterface[];
  summary: {
    total: number;
    byTargetType: { [key: string]: number };
    byLocation: { [key: string]: number };
  };
}

export default class AgentscriptActions extends SfCommand<ActionsResult> {
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
    format: Flags.option({
      char: 'o',
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['json', 'table', 'typescript', 'markdown'] as const,
      default: 'table',
    })(),
    target: Flags.option({
      char: 't',
      summary: messages.getMessage('flags.target.summary'),
      description: messages.getMessage('flags.target.description'),
      options: ['all', 'flow', 'apex', 'prompt'] as const,
      default: 'all',
    })(),
  };

  public async run(): Promise<ActionsResult> {
    const { flags } = await this.parse(AgentscriptActions);

    try {
      const filePath = path.resolve(flags.file as string);
      const source = fs.readFileSync(filePath, 'utf-8');
      const ast = parser.parse_agent(source);

      const actions = this.extractActions(ast);

      // Filter by target type if specified
      const filteredActions = flags.target === 'all'
        ? actions
        : actions.filter(a => a.targetType === flags.target);

      const summary = {
        total: filteredActions.length,
        byTargetType: this.countBy(filteredActions, 'targetType'),
        byLocation: this.countBy(filteredActions, 'location'),
      };

      // Format output
      switch (flags.format) {
        case 'json':
          this.log(JSON.stringify({ actions: filteredActions, summary }, null, 2));
          break;
        case 'typescript':
          this.outputTypeScript(filteredActions);
          break;
        case 'markdown':
          this.outputMarkdown(filteredActions, summary);
          break;
        default:
          this.outputTable(filteredActions, summary);
      }

      return { actions: filteredActions, summary };
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.extractionFailure', [error.message]);
      }
      throw error;
    }
  }

  private extractActions(ast: any): ActionInterface[] {
    const actions: ActionInterface[] = [];

    // Extract from start_agent
    if (ast.start_agent) {
      const startNode = ast.start_agent.node || ast.start_agent;
      if (startNode.actions) {
        const actionsBlock = startNode.actions.node || startNode.actions;
        // Actions are in actionsBlock.actions array
        const actionsArray = actionsBlock.actions || actionsBlock;
        if (Array.isArray(actionsArray)) {
          for (const actionDef of actionsArray) {
            const action = this.parseActionDef(actionDef, 'start_agent');
            if (action) actions.push(action);
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
              const action = this.parseActionDef(actionDef, `topic.${topicName}`);
              if (action) actions.push(action);
            }
          }
        }
      }
    }

    return actions;
  }

  private parseActionDef(def: any, location: string): ActionInterface | null {
    const node = def.node || def;

    // Get the action name
    const name = node.name?.node || node.name;
    if (!name) return null;

    // Skip if no target (might be a reasoning action reference, not a definition)
    if (!node.target) return null;

    const targetStr = node.target.node || node.target;

    // Only process string targets (flow://, apex://, prompt://)
    if (typeof targetStr !== 'string') return null;

    const { targetType, targetName } = this.parseTarget(targetStr);

    return {
      name,
      description: node.description?.node || node.description,
      label: node.label?.node || node.label,
      target: targetStr,
      targetType,
      targetName,
      location,
      requireUserConfirmation: node.require_user_confirmation?.node ?? node.require_user_confirmation,
      includeProgressIndicator: node.include_in_progress_indicator?.node ?? node.include_in_progress_indicator,
      progressIndicatorMessage: node.progress_indicator_message?.node || node.progress_indicator_message,
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
        label: param.label?.node || param.label,
        isRequired: param.is_required?.node ?? param.is_required,
        isDisplayable: param.is_displayable?.node ?? param.is_displayable,
        filterFromAgent: param.filter_from_agent?.node ?? param.filter_from_agent,
      };
    });
  }

  private normalizeType(ty: any): string {
    if (typeof ty === 'string') return ty.toLowerCase();
    if (ty && typeof ty === 'object') {
      // Handle List[T] types
      if (ty.List) return `list[${this.normalizeType(ty.List)}]`;
      // Handle other object types
      return JSON.stringify(ty).toLowerCase();
    }
    return 'unknown';
  }

  private countBy(items: ActionInterface[], key: keyof ActionInterface): { [k: string]: number } {
    const counts: { [k: string]: number } = {};
    for (const item of items) {
      const value = String(item[key]);
      counts[value] = (counts[value] || 0) + 1;
    }
    return counts;
  }

  private outputTable(actions: ActionInterface[], summary: { total: number; byTargetType: { [key: string]: number }; byLocation: { [key: string]: number } }): void {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    // Summary header
    ux.styledHeader(`Action Interfaces (${summary.total} total)`);
    this.log('');

    // Summary table by target type
    const summaryData = Object.entries(summary.byTargetType).map(([type, count]) => ({
      type: this.colorizeTargetType(type),
      count: count.toString(),
    }));

    ux.table(summaryData, {
      type: { header: 'Target Type' },
      count: { header: 'Count' },
    });

    this.log('');

    // Group by location
    const byLocation = new Map<string, ActionInterface[]>();
    for (const action of actions) {
      const list = byLocation.get(action.location) || [];
      list.push(action);
      byLocation.set(action.location, list);
    }

    for (const [location, locationActions] of byLocation) {
      ux.styledHeader(this.formatLocation(location));

      // Actions table for this location
      const tableData = locationActions.map(action => ({
        action: ansis.bold(action.name),
        target: this.colorizeTargetType(action.targetType) + ' ' + ansis.dim(action.targetName),
        inputs: this.formatParamsSummary(action.inputs),
        outputs: this.formatParamsSummary(action.outputs),
      }));

      ux.table(tableData, {
        action: { header: 'Action' },
        target: { header: 'Target' },
        inputs: { header: 'Inputs' },
        outputs: { header: 'Outputs' },
      }, { 'no-truncate': true });

      this.log('');

      // Show detailed parameter info for each action
      for (const action of locationActions) {
        if (action.inputs.length > 0 || action.outputs.length > 0) {
          this.log(`  ${ansis.cyan.bold(action.name)}`);
          if (action.description) {
            this.log(`  ${ansis.dim(action.description)}`);
          }

          if (action.inputs.length > 0) {
            this.log(`  ${ansis.green('Inputs:')}`);
            for (const input of action.inputs) {
              const required = input.isRequired ? ansis.red('*') : ' ';
              const type = ansis.yellow(input.type);
              const desc = input.description ? ansis.dim(` - ${input.description}`) : '';
              this.log(`    ${required} ${input.name}: ${type}${desc}`);
            }
          }

          if (action.outputs.length > 0) {
            this.log(`  ${ansis.magenta('Outputs:')}`);
            for (const output of action.outputs) {
              const type = ansis.yellow(output.type);
              const desc = output.description ? ansis.dim(` - ${output.description}`) : '';
              this.log(`      ${output.name}: ${type}${desc}`);
            }
          }
          this.log('');
        }
      }
    }

    // Legend
    this.log(ansis.dim('Legend: ') + ansis.red('*') + ansis.dim(' = required'));
  }

  private colorizeTargetType(type: string): string {
    switch (type) {
      case 'flow':
        return ansis.cyanBright('flow');
      case 'apex':
        return ansis.magentaBright('apex');
      case 'prompt':
        return ansis.greenBright('prompt');
      default:
        return ansis.dim(type);
    }
  }

  private formatLocation(location: string): string {
    if (location === 'start_agent') {
      return 'Start Agent';
    }
    if (location.startsWith('topic.')) {
      return `Topic: ${location.substring(6)}`;
    }
    return location;
  }

  private formatParamsSummary(params: ActionParameter[]): string {
    if (params.length === 0) return ansis.dim('-');
    const required = params.filter(p => p.isRequired).length;
    const optional = params.length - required;
    const parts: string[] = [];
    if (required > 0) parts.push(`${required} req`);
    if (optional > 0) parts.push(`${optional} opt`);
    return parts.join(', ');
  }

  private outputTypeScript(actions: ActionInterface[]): void {
    this.log('// Auto-generated TypeScript interfaces from AgentScript\n');

    for (const action of actions) {
      // Input interface
      if (action.inputs.length > 0) {
        this.log(`interface ${this.toPascalCase(action.name)}Input {`);
        for (const input of action.inputs) {
          const optional = input.isRequired ? '' : '?';
          const comment = input.description ? ` // ${input.description}` : '';
          this.log(`  ${input.name}${optional}: ${this.toTsType(input.type)};${comment}`);
        }
        this.log('}\n');
      }

      // Output interface
      if (action.outputs.length > 0) {
        this.log(`interface ${this.toPascalCase(action.name)}Output {`);
        for (const output of action.outputs) {
          const comment = output.description ? ` // ${output.description}` : '';
          this.log(`  ${output.name}: ${this.toTsType(output.type)};${comment}`);
        }
        this.log('}\n');
      }
    }

    // Flow registry type
    this.log('// Flow registry mapping action names to their interfaces');
    this.log('interface FlowRegistry {');
    for (const action of actions) {
      if (action.targetType === 'flow') {
        const inputType = action.inputs.length > 0 ? `${this.toPascalCase(action.name)}Input` : 'void';
        const outputType = action.outputs.length > 0 ? `${this.toPascalCase(action.name)}Output` : 'void';
        this.log(`  '${action.targetName}': { input: ${inputType}; output: ${outputType}; };`);
      }
    }
    this.log('}');
  }

  private outputMarkdown(actions: ActionInterface[], summary: { total: number; byTargetType: { [key: string]: number } }): void {
    this.log('# Action Interface Reference\n');

    this.log('## Summary\n');
    this.log(`Total actions: ${summary.total}\n`);
    this.log('| Target Type | Count |');
    this.log('|-------------|-------|');
    for (const [type, count] of Object.entries(summary.byTargetType)) {
      this.log(`| ${type} | ${count} |`);
    }
    this.log('');

    // Group by target type
    const flows = actions.filter(a => a.targetType === 'flow');
    const apex = actions.filter(a => a.targetType === 'apex');
    const prompts = actions.filter(a => a.targetType === 'prompt');

    if (flows.length > 0) {
      this.log('## Flows\n');
      for (const action of flows) {
        this.outputMarkdownAction(action);
      }
    }

    if (apex.length > 0) {
      this.log('## Apex Classes\n');
      for (const action of apex) {
        this.outputMarkdownAction(action);
      }
    }

    if (prompts.length > 0) {
      this.log('## Prompt Templates\n');
      for (const action of prompts) {
        this.outputMarkdownAction(action);
      }
    }
  }

  private outputMarkdownAction(action: ActionInterface): void {
    this.log(`### ${action.targetName}\n`);
    this.log(`**Action:** \`${action.name}\`  `);
    this.log(`**Location:** ${action.location}  `);
    if (action.description) {
      this.log(`**Description:** ${action.description}  `);
    }
    this.log('');

    if (action.inputs.length > 0) {
      this.log('#### Inputs\n');
      this.log('| Name | Type | Required | Description |');
      this.log('|------|------|----------|-------------|');
      for (const input of action.inputs) {
        const required = input.isRequired ? 'Yes' : 'No';
        const desc = input.description || '-';
        this.log(`| ${input.name} | ${input.type} | ${required} | ${desc} |`);
      }
      this.log('');
    }

    if (action.outputs.length > 0) {
      this.log('#### Outputs\n');
      this.log('| Name | Type | Description |');
      this.log('|------|------|-------------|');
      for (const output of action.outputs) {
        const desc = output.description || '-';
        this.log(`| ${output.name} | ${output.type} | ${desc} |`);
      }
      this.log('');
    }
  }

  private toPascalCase(str: string): string {
    return str
      .split(/[_-]/)
      .map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase())
      .join('');
  }

  private toTsType(agentType: string): string {
    const lower = agentType.toLowerCase();
    if (lower === 'string') return 'string';
    if (lower === 'number' || lower === 'integer' || lower === 'long' || lower === 'currency') return 'number';
    if (lower === 'boolean') return 'boolean';
    if (lower === 'object') return 'Record<string, unknown>';
    if (lower === 'date' || lower === 'datetime' || lower === 'time' || lower === 'timestamp') return 'string';
    if (lower === 'id') return 'string';
    if (lower.startsWith('list[')) {
      const inner = lower.slice(5, -1);
      return `${this.toTsType(inner)}[]`;
    }
    return 'unknown';
  }
}
