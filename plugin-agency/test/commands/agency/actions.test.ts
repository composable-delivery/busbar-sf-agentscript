import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptActions from '../../../src/commands/agency/actions.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency actions', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptActions.summary).toBeTypeOf('string');
    expect(AgentscriptActions.flags).toHaveProperty('file');
    expect(AgentscriptActions.flags).toHaveProperty('path');
    expect(AgentscriptActions.flags).toHaveProperty('format');
    expect(AgentscriptActions.flags).toHaveProperty('target');
    expect(AgentscriptActions.flags).toHaveProperty('verbose');
  });

  it('--file is optional', () => {
    expect(AgentscriptActions.flags.file.required).toBeFalsy();
  });

  it('returns actions result for simple agent', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'simple.agent')],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r).toHaveProperty('file');
    expect(r).toHaveProperty('actions');
    expect(r).toHaveProperty('summary');
    expect(Array.isArray(r.actions)).toBe(true);
  });

  it('summary has expected shape', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'simple.agent')],
      undefined
    );
    const r = result as any;
    expect(r.summary).toHaveProperty('total');
    expect(r.summary).toHaveProperty('byTargetType');
    expect(r.summary).toHaveProperty('byLocation');
    expect(typeof r.summary.total).toBe('number');
  });

  it('returns json format', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.format ?? 'json').toBeTypeOf('string');
  });

  it('returns markdown format', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'markdown'],
      undefined
    );
    expect(result).toBeTruthy();
  });

  it('returns typescript format', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'typescript'],
      undefined
    );
    expect(result).toBeTruthy();
  });

  it('filters by target type', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--target', 'flow'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(Array.isArray(r.actions)).toBe(true);
    // All returned actions should be flow type
    for (const action of r.actions) {
      expect(action.targetType).toBe('flow');
    }
  });

  it('runs with --verbose flag', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--verbose'],
      undefined
    );
    expect(result).toBeTruthy();
  });

  it('returns array when scanning directory with --path', async () => {
    const result = await AgentscriptActions.run(
      ['--path', path.join(FIXTURES, 'agents-dir')],
      undefined
    );
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBe(2);
  });

  it('each result has expected shape in multi-file mode', async () => {
    const result = await AgentscriptActions.run(
      ['--path', path.join(FIXTURES, 'agents-dir')],
      undefined
    );
    for (const r of result as any[]) {
      expect(r).toHaveProperty('file');
      expect(r).toHaveProperty('actions');
      expect(r).toHaveProperty('summary');
    }
  });

  it('extracts flow actions from rich agent', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'rich.agent'), '--format', 'json'],
      undefined
    );
    const r = result as any;
    expect(Array.isArray(r.actions)).toBe(true);
    expect(r.actions.length).toBeGreaterThan(0);
    const flowActions = r.actions.filter((a: any) => a.targetType === 'flow');
    expect(flowActions.length).toBeGreaterThan(0);
  });

  it('renders table format with actions (default)', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'rich.agent')],
      undefined
    );
    const r = result as any;
    expect(r.actions.length).toBeGreaterThan(0);
    expect(r.summary.total).toBeGreaterThan(0);
  });

  it('renders verbose table with action details', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'rich.agent'), '--verbose'],
      undefined
    );
    const r = result as any;
    expect(r.actions.length).toBeGreaterThan(0);
  });

  it('renders markdown with real actions', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'rich.agent'), '--format', 'markdown'],
      undefined
    );
    const r = result as any;
    expect(r.actions.length).toBeGreaterThan(0);
  });

  it('renders typescript with real actions', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'rich.agent'), '--format', 'typescript'],
      undefined
    );
    const r = result as any;
    expect(r.actions.length).toBeGreaterThan(0);
  });

  it('filters by apex target type', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'rich.agent'), '--target', 'apex', '--format', 'json'],
      undefined
    );
    const r = result as any;
    expect(Array.isArray(r.actions)).toBe(true);
    for (const action of r.actions) {
      expect(action.targetType).toBe('apex');
    }
  });

  it('summary byTargetType and byLocation are populated', async () => {
    const result = await AgentscriptActions.run(
      ['--file', path.join(FIXTURES, 'rich.agent'), '--format', 'json'],
      undefined
    );
    const r = result as any;
    expect(r.summary.total).toBeGreaterThan(0);
    expect(Object.keys(r.summary.byTargetType).length).toBeGreaterThan(0);
    expect(Object.keys(r.summary.byLocation).length).toBeGreaterThan(0);
  });
});
