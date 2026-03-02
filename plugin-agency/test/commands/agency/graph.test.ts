import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgentscriptGraph from '../../../src/commands/agency/graph.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency graph', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgentscriptGraph.summary).toBeTypeOf('string');
    expect(AgentscriptGraph.flags).toHaveProperty('file');
    expect(AgentscriptGraph.flags).toHaveProperty('format');
    expect(AgentscriptGraph.flags).toHaveProperty('view');
    expect(AgentscriptGraph.flags).toHaveProperty('stats');
    expect(AgentscriptGraph.flags).toHaveProperty('path');
  });

  it('--file is optional', () => {
    expect(AgentscriptGraph.flags.file.required).toBeFalsy();
  });

  it('renders ascii graph for simple agent', async () => {
    const result = await AgentscriptGraph.run(
      ['--file', path.join(FIXTURES, 'simple.agent')],
      undefined
    );
    expect(result).toBeTruthy();
    expect((result as any).format).toBe('ascii');
  });

  it('renders mermaid format', async () => {
    const result = await AgentscriptGraph.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'mermaid'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.format).toBe('mermaid');
    expect(r.graph).toContain('flowchart LR');
  });

  it('renders html format', async () => {
    const result = await AgentscriptGraph.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'html'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.format).toBe('html');
    expect(r.graph).toContain('<!DOCTYPE html>');
    expect(r.graph).toContain('mermaid');
  });

  it('renders graphml format', async () => {
    const result = await AgentscriptGraph.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--format', 'graphml'],
      undefined
    );
    expect(result).toBeTruthy();
    const r = result as any;
    expect(r.format).toBe('graphml');
  });

  it('renders with stats flag', async () => {
    const result = await AgentscriptGraph.run(
      ['--file', path.join(FIXTURES, 'simple.agent'), '--stats'],
      undefined
    );
    expect(result).toBeTruthy();
  });

  it('renders multiple files when using --path', async () => {
    const result = await AgentscriptGraph.run(
      ['--path', path.join(FIXTURES, 'agents-dir')],
      undefined
    );
    expect(Array.isArray(result)).toBe(true);
    expect((result as any[]).length).toBe(2);
  });
});
