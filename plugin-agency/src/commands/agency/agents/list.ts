import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
import { findAgentFiles, loadAgentState } from '../../../lib/agent-files.js';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from '../../../wasm-loader.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.agents.list');

interface AgentListEntry {
  path: string;
  name: string;
  selected: boolean;
}

interface AgentsListResult {
  agents: AgentListEntry[];
  total: number;
  selectedCount: number;
}

export default class AgencyAgentsList extends SfCommand<AgentsListResult> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  public static readonly flags = {
    path: Flags.directory({
      summary: messages.getMessage('flags.path.summary'),
      description: messages.getMessage('flags.path.description'),
      default: '.',
    }),
  };

  public async run(): Promise<AgentsListResult> {
    const { flags } = await this.parse(AgencyAgentsList);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    try {
      const scanDir = path.resolve(flags.path as string);
      const agentFiles = findAgentFiles(scanDir);

      // Load selection state
      const state = loadAgentState(this.config.dataDir);
      const selectedSet = new Set<string>();
      if (state) {
        for (const rel of state.selected) {
          selectedSet.add(path.resolve(state.repoRoot, rel));
        }
      }

      const agents: AgentListEntry[] = agentFiles.map(filePath => {
        const relPath = path.relative(process.cwd(), filePath);
        const agentName = extractAgentName(filePath);
        const isSelected = selectedSet.has(filePath);
        return { path: relPath, name: agentName, selected: isSelected };
      });

      const selectedCount = agents.filter(a => a.selected).length;
      const result: AgentsListResult = { agents, total: agents.length, selectedCount };

      if (!this.jsonEnabled()) {
        ux.styledHeader(`Agent Files (${agents.length} found)`);
        this.log('');

        if (agents.length === 0) {
          this.log(ansis.dim('  No .agent files found.'));
        } else {
          const tableData = agents.map(a => ({
            selected: a.selected ? ansis.green('✓') : ansis.dim('-'),
            path: ansis.bold(a.path),
            name: ansis.cyan(a.name),
          }));
          ux.table({
            data: tableData,
            columns: [
              { key: 'selected', name: '' },
              { key: 'path', name: 'File' },
              { key: 'name', name: 'Agent Name' },
            ],
          });
        }
        this.log('');
        if (selectedCount > 0) {
          this.log(ansis.dim(`${selectedCount} of ${agents.length} selected`));
        } else {
          this.log(ansis.dim('No selection saved — all commands default to scanning this directory.'));
        }
        this.log('');
      }

      return result;
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.listFailure', [error.message]);
      }
      throw error;
    }
  }
}

function extractAgentName(filePath: string): string {
  try {
    const content = fs.readFileSync(filePath, 'utf-8');
    const match = content.match(/agent_name:\s*"([^"]+)"/);
    return match?.[1] ?? path.basename(filePath, '.agent');
  } catch {
    return path.basename(filePath, '.agent');
  }
}
