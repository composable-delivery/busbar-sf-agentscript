import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptValidate from '../../../src/commands/agency/validate.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency validate', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptValidate.summary).toBeTypeOf('string');
    expect(AgentscriptValidate.flags).toHaveProperty('file');
    expect(AgentscriptValidate.flags).toHaveProperty('path');
    expect(AgentscriptValidate.flags).toHaveProperty('verbose');
  });

  it('--file is optional', () => {
    expect(AgentscriptValidate.flags.file.required).toBeFalsy();
  });

  it('validates a valid agent file and returns valid result', async () => {
    const result = await AgentscriptValidate.run(
      ['--file', path.join(FIXTURES, 'simple.agent')],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.file).toBeTypeOf('string');
    expect(r.valid).toBe(true);
    expect(Array.isArray(r.issues)).toBe(true);
  });

  it('validates a minimal agent file', async () => {
    const result = await AgentscriptValidate.run(
      ['--file', path.join(FIXTURES, 'minimal.agent')],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r).toHaveProperty('valid');
    expect(r).toHaveProperty('issues');
  });

  it('returns array when scanning directory with --path', async () => {
    const result = await AgentscriptValidate.run(
      ['--path', path.join(FIXTURES, 'agents-dir')],
      undefined
    );
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBe(2);
  });

  it('each result has expected shape', async () => {
    const result = await AgentscriptValidate.run(
      ['--path', path.join(FIXTURES, 'agents-dir')],
      undefined
    );
    for (const r of result as any[]) {
      expect(r).toHaveProperty('file');
      expect(r).toHaveProperty('valid');
      expect(r).toHaveProperty('issues');
      expect(Array.isArray(r.issues)).toBe(true);
    }
  });

  it('runs with --verbose flag', async () => {
    const result = await AgentscriptValidate.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--verbose'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r).toHaveProperty('valid');
  });
});
