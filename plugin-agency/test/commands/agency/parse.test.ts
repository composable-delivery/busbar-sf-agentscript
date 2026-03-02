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

  it('parses a simple .agent file and returns AST', async () => {
    const result = await AgentscriptParse.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTypeOf('object');
    expect(result).toHaveProperty('config');
  });

  it('parses a minimal .agent file', async () => {
    const result = await AgentscriptParse.run(
      ['--file', path.join(FIXTURES, 'minimal.agent'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTypeOf('object');
    expect(result.config).toBeTruthy();
  });

  it('fails gracefully on a missing file', async () => {
    await expect(
      AgentscriptParse.run(['--file', '/nonexistent/path.agent'], undefined)
    ).rejects.toThrow();
  });
});
