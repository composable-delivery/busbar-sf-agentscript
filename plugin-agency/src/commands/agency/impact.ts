import { SfCommand, Flags, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as fs from 'fs';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as graphLib from '../../wasm-loader.js';
import { findAgentFiles } from '../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.impact');

interface DependencyReport {
  flows: string[];
  apex_classes: string[];
  sobjects: string[];
  prompt_templates: string[];
}

interface ImpactMatch {
  file: string;
  dep_type: string;
}

interface ImpactResult {
  resource: string;
  matches: ImpactMatch[];
  total_scanned: number;
}

export default class AgencyImpact extends SfCommand<ImpactResult> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  public static readonly flags = {
    resource: Flags.string({
      char: 'r',
      summary: messages.getMessage('flags.resource.summary'),
      description: messages.getMessage('flags.resource.description'),
      required: true,
    }),
    type: Flags.option({
      char: 't',
      summary: messages.getMessage('flags.type.summary'),
      description: messages.getMessage('flags.type.description'),
      options: ['flow', 'apex', 'sobject', 'prompt', 'all'] as const,
      default: 'all',
    })(),
    path: Flags.directory({
      char: 'd',
      summary: messages.getMessage('flags.path.summary'),
      description: messages.getMessage('flags.path.description'),
      default: '.',
    }),
    format: Flags.option({
      summary: messages.getMessage('flags.format.summary'),
      description: messages.getMessage('flags.format.description'),
      options: ['json', 'pretty'] as const,
      default: 'pretty',
    })(),
  };

  public async run(): Promise<ImpactResult> {
    const { flags } = await this.parse(AgencyImpact);
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    try {
      const scanDir = path.resolve(flags.path as string);
      const resourceName = flags.resource as string;
      const resourceType = flags.type as string;

      // Recursively find all .agent files
      const agentFiles = findAgentFiles(scanDir);

      const matches: ImpactMatch[] = [];

      for (const agentFile of agentFiles) {
        const source = fs.readFileSync(agentFile, 'utf-8');

        let found = false;
        let matchType = '';

        try {
          if (resourceType === 'flow' || resourceType === 'all') {
            if (graphLib.uses_flow(source, resourceName)) {
              found = true;
              matchType = 'flow';
            }
          }
          if (!found && (resourceType === 'apex' || resourceType === 'all')) {
            if (graphLib.uses_apex_class(source, resourceName)) {
              found = true;
              matchType = 'apex';
            }
          }
          if (!found && (resourceType === 'sobject' || resourceType === 'all')) {
            if (graphLib.uses_sobject(source, resourceName)) {
              found = true;
              matchType = 'sobject';
            }
          }
          if (!found && (resourceType === 'prompt' || resourceType === 'all')) {
            // Fall back to dependency report for prompts
            const report: DependencyReport = graphLib.extract_dependencies(source);
            if (report.prompt_templates.includes(resourceName)) {
              found = true;
              matchType = 'prompt';
            }
          }
        } catch {
          // Skip unparseable files
        }

        if (found) {
          matches.push({ file: path.relative(scanDir, agentFile), dep_type: matchType });
        }
      }

      const result: ImpactResult = {
        resource: resourceName,
        matches,
        total_scanned: agentFiles.length,
      };

      if (flags.format === 'json') {
        this.log(JSON.stringify(result, null, 2));
      } else {
        this.displayPretty(ux, result);
      }

      return result;
    } catch (error) {
      if (error instanceof Error) {
        throw messages.createError('error.impactFailure', [error.message]);
      }
      throw error;
    }
  }

  private displayPretty(ux: Ux, result: ImpactResult): void {
    ux.styledHeader(`Impact: ${result.resource}`);
    this.log('');
    this.log(`  ${ansis.dim('Scanned:')} ${ansis.bold(String(result.total_scanned))} agent files`);
    this.log('');

    if (result.matches.length === 0) {
      this.log(ansis.green('  No agent files depend on this resource.'));
    } else {
      this.log(ansis.bold(`${result.matches.length} of ${result.total_scanned} agent files depend on this resource:`));
      this.log('');

      const tableData = result.matches.map(m => ({
        file: ansis.bold(m.file),
        dep_type: ansis.yellow(m.dep_type),
      }));
      ux.table({
        data: tableData,
        columns: [
          { key: 'file', name: 'Agent File' },
          { key: 'dep_type', name: 'Dependency Type' },
        ],
      });
    }
    this.log('');
  }
}
