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
const messages = Messages.loadMessages('sf-plugin-busbar-agency', 'agency.list');

interface ListResult {
  type: string;
  items: string[];
}

export default class AgentscriptList extends SfCommand<ListResult> {
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
    type: Flags.option({
      char: 't',
      summary: messages.getMessage('flags.type.summary'),
      description: messages.getMessage('flags.type.description'),
      options: ['topics', 'variables', 'actions', 'messages'] as const,
      required: true,
    })(),
    format: Flags.option({
      char: 'o',
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['json', 'pretty'] as const,
      default: 'pretty',
    })(),
  };

  public async run(): Promise<ListResult> {
    const { flags } = await this.parse(AgentscriptList);

    try {
      // Read and parse the AgentScript file
      const filePath = path.resolve(flags.file as string);
      const source = fs.readFileSync(filePath, 'utf-8');
      const ast = parser.parse_agent(source);

      // List the requested type
      const items = this.listItems(ast, flags.type);

      if (flags.format === 'json') {
        this.log(JSON.stringify({ type: flags.type, items }, null, 2));
      } else {
        this.displayPretty(flags.type, items);
      }

      return { type: flags.type, items };
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.listFailure', [error.message]);
      }
      throw error;
    }
  }

  private listItems(ast: any, type: string): string[] {
    switch (type) {
      case 'topics':
        return this.listTopics(ast);
      case 'variables':
        return this.listVariables(ast);
      case 'actions':
        return this.listActions(ast);
      case 'messages':
        return this.listMessages(ast);
      default:
        return [];
    }
  }

  private listTopics(ast: any): string[] {
    const topics: string[] = [];
    
    // Add start_agent if it exists
    if (ast.start_agent) {
      const startNode = ast.start_agent.node || ast.start_agent;
      const name = startNode.name?.node || startNode.name || 'topic_selector';
      topics.push(`start_agent: ${name}`);
    }

    // Add regular topics - handle array or object
    if (ast.topics) {
      const topicsArray = Array.isArray(ast.topics) ? ast.topics : Object.values(ast.topics);
      for (const topic of topicsArray) {
        const topicNode = (topic as any).node || topic;
        const topicName = topicNode.name?.node || topicNode.name || 'unknown';
        const desc = topicNode.description?.node || topicNode.description || '';
        topics.push(`${topicName}${desc ? ': ' + desc : ''}`);
      }
    }

    return topics;
  }

  private listVariables(ast: any): string[] {
    const variables: string[] = [];
    
    if (ast.variables) {
      const varsObj = ast.variables;
      for (const [key, value] of Object.entries(varsObj)) {
        const varNode = (value as any).node || value;
        const varType = varNode.var_type?.node || varNode.var_type || 'unknown';
        const isMutable = varNode.is_mutable ? 'mutable ' : '';
        const isLinked = varNode.is_linked ? 'linked ' : '';
        variables.push(`${key}: ${isMutable}${isLinked}${varType}`);
      }
    }

    return variables;
  }

  private listActions(ast: any): string[] {
    const actions: string[] = [];
    
    // Collect from start_agent
    if (ast.start_agent) {
      const startNode = ast.start_agent.node || ast.start_agent;
      if (startNode.reasoning) {
        const reasoning = startNode.reasoning.node || startNode.reasoning;
        if (reasoning.actions) {
          const actionsObj = reasoning.actions.node || reasoning.actions;
          for (const key of Object.keys(actionsObj)) {
            actions.push(`start_agent.${key}`);
          }
        }
      }
    }

    // Collect from topics
    if (ast.topics) {
      for (const [topicName, topicValue] of Object.entries(ast.topics)) {
        const topicNode = (topicValue as any).node || topicValue;
        
        // Actions in reasoning
        if (topicNode.reasoning) {
          const reasoning = topicNode.reasoning.node || topicNode.reasoning;
          if (reasoning.actions) {
            const actionsObj = reasoning.actions.node || reasoning.actions;
            for (const key of Object.keys(actionsObj)) {
              actions.push(`${topicName}.reasoning.${key}`);
            }
          }
        }

        // Actions as direct topic property
        if (topicNode.actions) {
          const actionsObj = topicNode.actions.node || topicNode.actions;
          for (const key of Object.keys(actionsObj)) {
            actions.push(`${topicName}.actions.${key}`);
          }
        }
      }
    }

    return actions;
  }

  private listMessages(ast: any): string[] {
    const messages: string[] = [];
    
    if (ast.system) {
      const systemNode = ast.system.node || ast.system;
      if (systemNode.messages) {
        const messagesObj = systemNode.messages.node || systemNode.messages;
        for (const [key, value] of Object.entries(messagesObj)) {
          const msgNode = (value as any).node || value;
          const text = typeof msgNode === 'string' ? msgNode : (msgNode.text || JSON.stringify(msgNode));
          messages.push(`${key}: ${text}`);
        }
      }
    }

    return messages;
  }

  private displayPretty(type: string, items: string[]): void {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    const typeLabels: Record<string, string> = {
      topics: 'Topics',
      variables: 'Variables',
      actions: 'Actions',
      messages: 'Messages',
    };

    const typeColors: Record<string, (s: string) => string> = {
      topics: ansis.cyanBright,
      variables: ansis.green,
      actions: ansis.magenta,
      messages: ansis.yellow,
    };

    const colorFn = typeColors[type] || ansis.white;

    ux.styledHeader(`${typeLabels[type]} (${items.length})`);
    this.log('');

    if (items.length === 0) {
      this.log(`  ${ansis.dim('(none)')}`);
      this.log('');
      return;
    }

    items.forEach(item => {
      // Parse the item to separate name from description/type
      const colonIdx = item.indexOf(':');
      if (colonIdx > 0) {
        const name = item.substring(0, colonIdx);
        const rest = item.substring(colonIdx + 1).trim();
        this.log(`  ${ansis.green('•')} ${colorFn(name)}${ansis.dim(':')} ${rest}`);
      } else {
        this.log(`  ${ansis.green('•')} ${colorFn(item)}`);
      }
    });

    this.log('');
  }
}
