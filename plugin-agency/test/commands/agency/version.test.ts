import { describe, it, expect, beforeAll } from 'vitest';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptVersion from '../../../src/commands/agency/version.js';

describe('agency version', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptVersion.summary).toBeTypeOf('string');
    expect(AgentscriptVersion.summary.length).toBeGreaterThan(0);
    expect(AgentscriptVersion.description).toBeTypeOf('string');
    expect(AgentscriptVersion.examples).toBeInstanceOf(Array);
    expect(AgentscriptVersion.examples.length).toBeGreaterThan(0);
  });

  it('returns a version string', async () => {
    const result = await AgentscriptVersion.run([]);
    expect(result).toHaveProperty('version');
    expect(result.version).toBeTypeOf('string');
    expect(result.version).toMatch(/^\d+\.\d+\.\d+/);
  });
});
