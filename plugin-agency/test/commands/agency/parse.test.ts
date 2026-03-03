import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptParse from '../../../src/commands/agency/parse.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency parse', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptParse.summary).toBeTypeOf('string');
    expect(AgentscriptParse.flags).toHaveProperty('file');
    expect(AgentscriptParse.flags).toHaveProperty('format');
  });

  it('parses a simple .agent file and returns ParseResult with file + ast', async () => {
    const result = await AgentscriptParse.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTypeOf('object');
    expect(result).toHaveProperty('file');
    expect(result).toHaveProperty('ast');
    expect((result as any).ast).toHaveProperty('config');
  });

  it('parses a minimal .agent file', async () => {
    const result = await AgentscriptParse.run(
      ['--file', path.join(FIXTURES, 'minimal.agent'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTypeOf('object');
    expect((result as any).ast.config).toBeTruthy();
  });

  it('fails gracefully on a missing file', async () => {
    await expect(
      AgentscriptParse.run(['--file', '/nonexistent/path.agent'], undefined)
    ).rejects.toThrow();
  });

  it('renders default pretty format without error', async () => {
    const result = await AgentscriptParse.run(
      ['--file', path.join(FIXTURES, 'simple.agent')],
      undefined
    );
    expect(result).toBeTypeOf('object');
    expect(result).toHaveProperty('file');
    expect(result).toHaveProperty('ast');
  });

  it('runs with --verbose flag', async () => {
    const result = await AgentscriptParse.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--verbose'],
      undefined
    );
    expect(result).toBeTypeOf('object');
    expect(result).toHaveProperty('ast');
  });

  it('returns array when scanning directory with --path', async () => {
    const result = await AgentscriptParse.run(
      ['--path', path.join(FIXTURES, 'agents-dir'), '--format', 'json'],
      undefined
    );
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBe(2);
    for (const r of result as any[]) {
      expect(r).toHaveProperty('file');
      expect(r).toHaveProperty('ast');
    }
  });

  it('each result in multi-file mode has correct shape', async () => {
    const result = await AgentscriptParse.run(
      ['--path', path.join(FIXTURES, 'agents-dir'), '--format', 'json'],
      undefined
    );
    for (const r of result as any[]) {
      expect(r.ast).toHaveProperty('config');
    }
  });
});
