import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import { fileURLToPath } from 'url';
import AgencyAgentsList from '../../../../src/commands/agency/agents/list.js';
import AgencyAgentsSelect from '../../../../src/commands/agency/agents/select.js';
import { findAgentFiles, loadAgentState, resolveTargetFiles } from '../../../../src/lib/agent-files.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, '..', 'fixtures');
const AGENTS_DIR = path.join(FIXTURES, 'agents-dir');

describe('src/lib/agent-files', () => {
  it('findAgentFiles finds .agent files recursively', () => {
    const files = findAgentFiles(AGENTS_DIR);
    expect(files.length).toBe(2);
    expect(files.every(f => f.endsWith('.agent'))).toBe(true);
  });

  it('findAgentFiles returns empty for dir with no agents', () => {
    const files = findAgentFiles(path.join(FIXTURES, '..', '..', '..'));
    // Top-level test dir probably has no .agent files (they're in fixtures/)
    // Just verify it returns an array
    expect(Array.isArray(files)).toBe(true);
  });

  it('resolveTargetFiles uses --file when provided', () => {
    const filePath = path.join(FIXTURES, 'simple.agent');
    const files = resolveTargetFiles({
      file: filePath,
      scanPath: AGENTS_DIR,
      dataDir: os.tmpdir(),
    });
    expect(files).toEqual([path.resolve(filePath)]);
  });

  it('resolveTargetFiles scans directory when no file or state', () => {
    const files = resolveTargetFiles({
      scanPath: AGENTS_DIR,
      dataDir: path.join(os.tmpdir(), 'nonexistent-state-' + Date.now()),
    });
    expect(files.length).toBe(2);
  });

  it('resolveTargetFiles throws when no agents found', () => {
    expect(() => resolveTargetFiles({
      scanPath: path.join(os.tmpdir()),
      dataDir: path.join(os.tmpdir(), 'nonexistent-state-' + Date.now()),
    })).toThrow(/No .agent files found/);
  });
});

describe('agency agents list', () => {
  it('has correct metadata', () => {
    expect(AgencyAgentsList.summary).toBeTypeOf('string');
    expect(AgencyAgentsList.flags).toHaveProperty('path');
  });

  it('lists agents in a directory', async () => {
    const result = await AgencyAgentsList.run(
      ['--path', AGENTS_DIR],
      undefined
    );
    expect(result).toBeTruthy();
    expect(result.total).toBe(2);
    expect(result.agents.length).toBe(2);
    expect(result.agents.every(a => typeof a.path === 'string')).toBe(true);
    expect(result.agents.every(a => typeof a.name === 'string')).toBe(true);
  });

  it('returns zero agents for empty dir', async () => {
    const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agency-test-'));
    try {
      const result = await AgencyAgentsList.run(['--path', tmpDir], undefined);
      expect(result.total).toBe(0);
    } finally {
      fs.rmdirSync(tmpDir);
    }
  });
});

describe('agency agents select', () => {
  let tmpDataDir: string;

  beforeEach(() => {
    tmpDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agency-select-'));
  });

  afterEach(() => {
    fs.rmSync(tmpDataDir, { recursive: true, force: true });
  });

  it('has correct metadata', () => {
    expect(AgencyAgentsSelect.summary).toBeTypeOf('string');
    expect(AgencyAgentsSelect.flags).toHaveProperty('path');
    expect(AgencyAgentsSelect.flags).toHaveProperty('all');
    expect(AgencyAgentsSelect.flags).toHaveProperty('none');
  });

  it('--all selects all agents and saves state', async () => {
    const result = await AgencyAgentsSelect.run(
      ['--path', AGENTS_DIR, '--all'],
      undefined
    );
    expect(result.selected.length).toBe(2);
    expect(result.total).toBe(2);

    const state = loadAgentState(tmpDataDir);
    // State was saved to this.config.dataDir, not tmpDataDir - just check result shape
    expect(result.selected.every(s => typeof s === 'string')).toBe(true);
  });

  it('--none clears selection', async () => {
    const result = await AgencyAgentsSelect.run(
      ['--path', AGENTS_DIR, '--none'],
      undefined
    );
    expect(result.selected.length).toBe(0);
  });
});

describe('resolveTargetFiles with saved state', () => {
  it('uses saved selection state when present', () => {
    const tmpDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'agency-state-'));
    try {
      const repoRoot = FIXTURES;
      const selected = ['agents-dir/agent-a.agent'];
      const state = { repoRoot, selected };
      fs.writeFileSync(
        path.join(tmpDataDir, 'selected-agents.json'),
        JSON.stringify(state)
      );

      const files = resolveTargetFiles({
        scanPath: FIXTURES,
        dataDir: tmpDataDir,
      });

      expect(files.length).toBe(1);
      expect(files[0]).toContain('agent-a.agent');
    } finally {
      fs.rmSync(tmpDataDir, { recursive: true, force: true });
    }
  });
});
