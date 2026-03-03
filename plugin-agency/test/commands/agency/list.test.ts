import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptList from '../../../src/commands/agency/list.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency list', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptList.summary).toBeTypeOf('string');
    expect(AgentscriptList.flags).toHaveProperty('file');
    expect(AgentscriptList.flags).toHaveProperty('path');
    expect(AgentscriptList.flags).toHaveProperty('type');
    expect(AgentscriptList.flags).toHaveProperty('format');
  });

  it('--file is optional', () => {
    expect(AgentscriptList.flags.file.required).toBeFalsy();
  });

  it('lists topics for simple agent', async () => {
    const result = await AgentscriptList.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--type', 'topics'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r).toHaveProperty('file');
    expect(r).toHaveProperty('type');
    expect(r).toHaveProperty('items');
    expect(Array.isArray(r.items)).toBe(true);
    expect(r.type).toBe('topics');
  });

  it('lists variables for simple agent', async () => {
    const result = await AgentscriptList.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--type', 'variables'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.type).toBe('variables');
    expect(Array.isArray(r.items)).toBe(true);
  });

  it('lists actions for simple agent', async () => {
    const result = await AgentscriptList.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--type', 'actions'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.type).toBe('actions');
    expect(Array.isArray(r.items)).toBe(true);
  });

  it('lists messages for simple agent (may be empty)', async () => {
    const result = await AgentscriptList.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--type', 'messages'],
      undefined
    );
    // messages may be absent from simple agents; result may be undefined or have empty items
    if (result) {
      const r = result as any;
      expect(r.type).toBe('messages');
      expect(Array.isArray(r.items)).toBe(true);
    }
  });

  it('returns json format', async () => {
    const result = await AgentscriptList.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--type', 'topics', '--format', 'json'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(Array.isArray(r.items)).toBe(true);
  });

  it('returns array when scanning directory with --path', async () => {
    const result = await AgentscriptList.run(
      ['--path', path.join(FIXTURES, 'agents-dir'), '--type', 'topics'],
      undefined
    );
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBe(2);
  });

  it('each result in multi-file mode has expected shape', async () => {
    const result = await AgentscriptList.run(
      ['--path', path.join(FIXTURES, 'agents-dir'), '--type', 'variables'],
      undefined
    );
    for (const r of result as any[]) {
      expect(r).toHaveProperty('file');
      expect(r).toHaveProperty('type');
      expect(r).toHaveProperty('items');
      expect(Array.isArray(r.items)).toBe(true);
    }
  });
});
