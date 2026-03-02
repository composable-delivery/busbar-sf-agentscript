import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgencyPaths from '../../../src/commands/agency/paths.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency paths', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgencyPaths.summary).toBeTypeOf('string');
    expect(AgencyPaths.flags).toHaveProperty('file');
    expect(AgencyPaths.flags).toHaveProperty('format');
    expect(AgencyPaths.flags).toHaveProperty('max-depth');
    expect(AgencyPaths.flags).toHaveProperty('path');
  });

  it('--file is optional', () => {
    expect(AgencyPaths.flags.file.required).toBeFalsy();
  });

  it('returns paths result for simple agent', async () => {
    const result = await AgencyPaths.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r).toHaveProperty('paths');
    expect(r).toHaveProperty('total_paths');
    expect(r).toHaveProperty('unreachable');
    expect(Array.isArray(r.paths)).toBe(true);
  });

  it('respects max-depth flag', async () => {
    const result = await AgencyPaths.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--max-depth', '1'],
      undefined
    );
    expect(result).toBeTruthy();
  });

  it('returns multiple results when scanning directory', async () => {
    const result = await AgencyPaths.run(
      ['--path', path.join(FIXTURES, 'agents-dir')],
      undefined
    );
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBe(2);
  });
});
