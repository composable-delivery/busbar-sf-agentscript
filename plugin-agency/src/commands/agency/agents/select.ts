import { SfCommand, Flags } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as path from 'path';
import ansis from 'ansis';
import { findAgentFiles, getRepoRoot, saveAgentState, AgentState } from '../../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.agents.select');

interface SelectResult {
  selected: string[];
  total: number;
}

export default class AgencyAgentsSelect extends SfCommand<SelectResult> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  public static readonly flags = {
    path: Flags.directory({
      summary: messages.getMessage('flags.path.summary'),
      description: messages.getMessage('flags.path.description'),
      default: '.',
    }),
    all: Flags.boolean({
      summary: messages.getMessage('flags.all.summary'),
      description: messages.getMessage('flags.all.description'),
      default: false,
    }),
    none: Flags.boolean({
      summary: messages.getMessage('flags.none.summary'),
      description: messages.getMessage('flags.none.description'),
      default: false,
    }),
  };

  public async run(): Promise<SelectResult> {
    const { flags } = await this.parse(AgencyAgentsSelect);

    try {
      const scanDir = path.resolve(flags.path as string);
      const agentFiles = findAgentFiles(scanDir);

      if (agentFiles.length === 0 && !flags.none) {
        throw messages.createError('error.noAgentsFound', [scanDir]);
      }

      const repoRoot = getRepoRoot();
      let selected: string[] = [];

      if (flags.none) {
        // Clear selection
        selected = [];
        if (!this.jsonEnabled()) {
          this.log(ansis.green('✓') + ' Selection cleared.');
        }
      } else if (flags.all) {
        // Select all
        selected = agentFiles.map(f => path.relative(repoRoot, f));
        if (!this.jsonEnabled()) {
          this.log(ansis.green('✓') + ` Selected ${selected.length} agent files.`);
          for (const rel of selected) {
            this.log(`  ${ansis.dim('•')} ${rel}`);
          }
        }
      } else {
        // Interactive mode
        if (!process.stdout.isTTY || this.jsonEnabled()) {
          throw messages.createError('error.requireAllOrNone');
        }

        const { checkbox } = await import('@inquirer/prompts');
        const choices = agentFiles.map(f => ({
          name: path.relative(process.cwd(), f),
          value: path.relative(repoRoot, f),
        }));

        const picked = await checkbox({
          message: 'Select agent files to include by default:',
          choices,
        });

        selected = picked;

        if (!this.jsonEnabled()) {
          this.log('');
          this.log(ansis.green('✓') + ` Selected ${selected.length} of ${agentFiles.length} agent files.`);
        }
      }

      const state: AgentState = { repoRoot, selected };
      saveAgentState(this.config.dataDir, state);

      return { selected, total: agentFiles.length };
    } catch (error) {
      if (error instanceof Error && error.message.startsWith('Failed')) {
        throw error;
      }
      if (error instanceof Error) {
        throw messages.createError('error.selectFailure', [error.message]);
      }
      throw error;
    }
  }
}
