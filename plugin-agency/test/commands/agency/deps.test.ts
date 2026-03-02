import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptDeps from '../../../src/commands/agency/deps.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency deps', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptDeps.summary).toBeTypeOf('string');
    expect(AgentscriptDeps.flags).toHaveProperty('file');
    expect(AgentscriptDeps.flags).toHaveProperty('format');
    expect(AgentscriptDeps.flags).toHaveProperty('type');
    expect(AgentscriptDeps.flags).toHaveProperty('retrieve');
    expect(AgentscriptDeps.flags).toHaveProperty('path');
  });

  it('--file is optional', () => {
    expect(AgentscriptDeps.flags.file.required).toBeFalsy();
  });

  it('extracts dependencies from simple agent', async () => {
    const result = await AgentscriptDeps.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r).toHaveProperty('report');
    expect(r).toHaveProperty('interfaces');
    expect(r).toHaveProperty('summary');
  });

  it('returns summary format', async () => {
    const result = await AgentscriptDeps.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'summary'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.summary).toHaveProperty('total');
    expect(r.summary).toHaveProperty('by_category');
  });

  it('returns multiple results when scanning directory', async () => {
    const result = await AgentscriptDeps.run(
      ['--path', path.join(FIXTURES, 'agents-dir'), '--format', 'json'],
      undefined
    );
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBe(2);
  });
});
