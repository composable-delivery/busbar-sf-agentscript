import { SfCommand, Flags } from '@salesforce/sf-plugins-core';
import { Messages } from '@salesforce/core';
import { render } from 'ink';
import React from 'react';
import * as path from 'path';
import { App } from '../../components/App.js';
import { resolveTargetFiles } from '../../lib/agent-files.js';

Messages.importMessagesDirectoryFromMetaUrl(import.meta.url);
const messages = Messages.loadMessages('@muselab/sf-plugin-busbar-agency', 'agency.tui');

export default class AgencyTui extends SfCommand<void> {
  public static readonly summary = messages.getMessage('summary');
  public static readonly description = messages.getMessage('description');
  public static readonly examples = messages.getMessages('examples');

  // TUI does not support --json
  public static readonly enableJsonFlag = false;

  public static readonly flags = {
    file: Flags.file({
      char: 'f',
      summary: messages.getMessage('flags.file.summary'),
      description: messages.getMessage('flags.file.description'),
      required: false,
      exists: true,
    }),
    path: Flags.directory({
      summary: messages.getMessage('flags.path.summary'),
      description: messages.getMessage('flags.path.description'),
      default: '.',
    }),
  };

  public async run(): Promise<void> {
    const { flags } = await this.parse(AgencyTui);

    let files: string[];
    let initialFileIdx = 0;

    try {
      files = resolveTargetFiles({
        file: flags.file,
        scanPath: flags.path,
        dataDir: this.config.dataDir,
      });
    } catch (e) {
      this.error(e instanceof Error ? e.message : String(e));
    }

    // If a specific file was given, open directly to it
    if (flags.file) {
      const absFile = path.resolve(flags.file);
      const idx = files.findIndex(f => f === absFile);
      if (idx >= 0) initialFileIdx = idx;
    }

    const { waitUntilExit } = render(
      React.createElement(App, { files, initialFileIdx })
    );

    await waitUntilExit();
  }
}
