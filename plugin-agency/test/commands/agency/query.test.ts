import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptQuery from '../../../src/commands/agency/query.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency query', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptQuery.summary).toBeTypeOf('string');
    expect(AgentscriptQuery.flags).toHaveProperty('file');
    expect(AgentscriptQuery.flags).toHaveProperty('path');
    expect(AgentscriptQuery.flags).toHaveProperty('format');
    expect(AgentscriptQuery.args).toHaveProperty('queryPath');
  });

  it('--file is optional', () => {
    expect(AgentscriptQuery.flags.file.required).toBeFalsy();
  });

  describe('/topics/<name>', () => {
    it('returns topic result with incoming/outgoing', async () => {
      const result = await AgentscriptQuery.run(
        ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json', '/topics/someTopic'],
        undefined
      );
      expect(result).toHaveProperty('file');
      expect(result).toHaveProperty('queryPath', '/topics/someTopic');
      expect(result).toHaveProperty('result');
      const r = (result as any).result;
      expect(r).toHaveProperty('topic', 'someTopic');
      expect(r).toHaveProperty('incoming');
      expect(r).toHaveProperty('outgoing');
      expect(Array.isArray(r.incoming)).toBe(true);
      expect(Array.isArray(r.outgoing)).toBe(true);
    });

    it('returns empty arrays for nonexistent topic', async () => {
      const result = await AgentscriptQuery.run(
        ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json', '/topics/nonexistent_topic_xyz'],
        undefined
      );
      const r = (result as any).result;
      expect(r.incoming).toHaveLength(0);
      expect(r.outgoing).toHaveLength(0);
    });

    it('returns array in multi-file mode', async () => {
      const result = await AgentscriptQuery.run(
        ['--path', path.join(FIXTURES, 'agents-dir'), '--format', 'json', '/topics/someTopic'],
        undefined
      );
      expect(Array.isArray(result)).toBe(true);
      const arr = result as any[];
      expect(arr.length).toBeGreaterThan(0);
      expect(arr[0]).toHaveProperty('file');
      expect(arr[0]).toHaveProperty('queryPath', '/topics/someTopic');
    });
  });

  describe('/variables/<name>', () => {
    it('returns variable result with readers/writers', async () => {
      const result = await AgentscriptQuery.run(
        ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json', '/variables/someVar'],
        undefined
      );
      expect(result).toHaveProperty('queryPath', '/variables/someVar');
      const r = (result as any).result;
      expect(r).toHaveProperty('variable', 'someVar');
      expect(r).toHaveProperty('readers');
      expect(r).toHaveProperty('writers');
    });
  });

  describe('/actions/<name>', () => {
    it('throws when action not found in single-file mode', async () => {
      await expect(
        AgentscriptQuery.run(
          ['--file', path.join(FIXTURES, 'simple.agent'), '/actions/nonexistent_action'],
          undefined
        )
      ).rejects.toThrow();
    });

    it('skips missing actions in multi-file mode', async () => {
      const result = await AgentscriptQuery.run(
        ['--path', path.join(FIXTURES, 'agents-dir'), '--format', 'json', '/actions/nonexistent_action'],
        undefined
      );
      // All files skip, return empty array
      expect(Array.isArray(result)).toBe(true);
      expect((result as any[]).length).toBe(0);
    });
  });

  describe('raw AST traversal', () => {
    it('queries dot-notation path', async () => {
      const result = await AgentscriptQuery.run(
        ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'json', 'config'],
        undefined
      );
      expect(result).toHaveProperty('file');
      expect(result).toHaveProperty('queryPath', 'config');
      expect(result).toHaveProperty('result');
    });

    it('throws for invalid path', async () => {
      await expect(
        AgentscriptQuery.run(
          ['--file', path.join(FIXTURES, 'simple.agent'), 'nonexistent.deep.path'],
          undefined
        )
      ).rejects.toThrow();
    });
  });
});
