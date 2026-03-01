import { SfCommand, Ux } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import * as path from 'path';
import ansis from 'ansis';
// @ts-ignore - WASM module doesn't have TypeScript definitions
import * as parser from 'busbar-sf-agentscript';

// After bundling, __dirname is lib/commands/agentscript-parser/ - go up 3 levels to plugin root
const pluginRoot = path.resolve(__dirname, '..', '..', '..');
Messages.importMessagesDirectory(pluginRoot);
const messages = Messages.loadMessages('sf-plugin-busbar-agency', 'agency.version');

export default class AgentscriptVersion extends SfCommand<{ version: string }> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  public async run(): Promise<{ version: string }> {
    const ux = new Ux({ jsonEnabled: this.jsonEnabled() });

    const version = parser.version();

    ux.styledHeader('AgentScript Parser');
    this.log('');
    this.log(`  ${ansis.cyan('Version:')} ${ansis.bold.green(version)}`);
    this.log(`  ${ansis.cyan('Runtime:')} ${ansis.dim('WebAssembly (WASM)')}`);
    this.log('');

    return { version };
  }
}
