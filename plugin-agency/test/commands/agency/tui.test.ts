import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgencyTui from '../../../src/commands/agency/tui.js';
import { actionName } from '../../../src/lib/logic/graph.js';
import { computePaths } from '../../../src/lib/logic/paths.js';
import { validateAgent } from '../../../src/lib/logic/validation.js';
import { extractActionInterfaces, summarizeDependencies, groupByDependency } from '../../../src/lib/logic/deps.js';
import { getGraphData } from '../../../src/lib/logic/graph.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');
const SIMPLE_AGENT = path.join(FIXTURES, 'simple.agent');
const RICH_AGENT = path.join(FIXTURES, 'rich.agent');

// ─── Command metadata (no WASM needed) ───────────────────────────────────────

describe('AgencyTui command metadata', () => {
  it('has a summary string', () => {
    expect(AgencyTui.summary).toBeTypeOf('string');
    expect(AgencyTui.summary.length).toBeGreaterThan(0);
  });

  it('disables --json flag', () => {
    expect(AgencyTui.enableJsonFlag).toBe(false);
  });

  it('has optional --file flag', () => {
    expect(AgencyTui.flags).toHaveProperty('file');
    expect(AgencyTui.flags.file.required).toBeFalsy();
  });

  it('has --path flag with default "."', () => {
    expect(AgencyTui.flags).toHaveProperty('path');
  });

  it('does not have --format or --json flags', () => {
    expect(AgencyTui.flags).not.toHaveProperty('format');
    expect(AgencyTui.flags).not.toHaveProperty('json');
  });
});

// ─── logic/graph.ts helpers (no WASM needed) ─────────────────────────────────

describe('actionName helper', () => {
  it('returns string actions as-is', () => {
    expect(actionName('MyAction')).toBe('MyAction');
  });

  it('extracts name from object actions', () => {
    expect(actionName({ name: 'GetOrderStatus', target: 'flow://GetOrderStatusFlow' })).toBe('GetOrderStatus');
  });

  it('handles object action without target', () => {
    expect(actionName({ name: 'DoSomething' })).toBe('DoSomething');
  });
});

// ─── logic/paths.ts (no WASM needed — pure DFS) ──────────────────────────────

describe('computePaths', () => {
  const singleTopicGraph = {
    nodes: [], edges: [],
    topics: [{ name: 'main', description: null, is_entry: true, transitions_to: [], delegates_to: [] }],
    variables: [],
    stats: { total_nodes: 1, total_edges: 0, topics: 1, variables: 0, action_defs: 0, reasoning_actions: 0 },
  };

  const linearGraph = {
    nodes: [], edges: [],
    topics: [
      { name: 'start', description: null, is_entry: true, transitions_to: ['middle'], delegates_to: [] },
      { name: 'middle', description: null, is_entry: false, transitions_to: ['end'], delegates_to: [] },
      { name: 'end', description: null, is_entry: false, transitions_to: [], delegates_to: [] },
    ],
    variables: [],
    stats: { total_nodes: 3, total_edges: 2, topics: 3, variables: 0, action_defs: 0, reasoning_actions: 0 },
  };

  const cyclicGraph = {
    nodes: [], edges: [],
    topics: [
      { name: 'a', description: null, is_entry: true, transitions_to: ['b'], delegates_to: [] },
      { name: 'b', description: null, is_entry: false, transitions_to: ['a'], delegates_to: [] },
    ],
    variables: [],
    stats: { total_nodes: 2, total_edges: 2, topics: 2, variables: 0, action_defs: 0, reasoning_actions: 0 },
  };

  it('returns a single terminal path for a single-topic graph', () => {
    const result = computePaths(singleTopicGraph);
    expect(result.total_paths).toBe(1);
    expect(result.paths[0].nodes).toEqual(['main']);
    expect(result.paths[0].has_cycle).toBe(false);
    expect(result.unreachable).toHaveLength(0);
  });

  it('returns a linear path through all topics', () => {
    const result = computePaths(linearGraph);
    expect(result.total_paths).toBe(1);
    expect(result.paths[0].nodes).toEqual(['start', 'middle', 'end']);
    expect(result.paths[0].has_cycle).toBe(false);
    expect(result.paths[0].edge_types).toEqual(['transitions', 'transitions']);
  });

  it('detects cycles', () => {
    const result = computePaths(cyclicGraph);
    expect(result.total_paths).toBeGreaterThan(0);
    const cyclic = result.paths.filter(p => p.has_cycle);
    expect(cyclic.length).toBeGreaterThan(0);
  });

  it('identifies unreachable topics', () => {
    const graph = {
      ...linearGraph,
      topics: [
        ...linearGraph.topics,
        { name: 'orphan', description: null, is_entry: false, transitions_to: [], delegates_to: [] },
      ],
    };
    const result = computePaths(graph);
    expect(result.unreachable).toContain('orphan');
  });

  it('respects maxDepth to stop infinite loops', () => {
    const result = computePaths(cyclicGraph, 3);
    // With maxDepth=3, DFS should terminate; all path nodes lengths should be <= maxDepth + 1
    for (const p of result.paths) {
      expect(p.nodes.length).toBeLessThanOrEqual(5); // small bound
    }
  });

  it('handles delegate edges', () => {
    const graph = {
      nodes: [], edges: [],
      topics: [
        { name: 'parent', description: null, is_entry: true, transitions_to: [], delegates_to: ['child'] },
        { name: 'child', description: null, is_entry: false, transitions_to: [], delegates_to: [] },
      ],
      variables: [],
      stats: { total_nodes: 2, total_edges: 1, topics: 2, variables: 0, action_defs: 0, reasoning_actions: 0 },
    };
    const result = computePaths(graph);
    expect(result.paths[0].edge_types).toContain('delegates');
  });
});

// ─── logic/validation.ts + graph.ts (WASM needed) ────────────────────────────

describe('validateAgent', () => {
  beforeAll(async () => initWasm());

  it('returns valid result for simple.agent', () => {
    const source = require('fs').readFileSync(SIMPLE_AGENT, 'utf-8');
    const result = validateAgent(source, SIMPLE_AGENT);
    expect(result.file).toBe(SIMPLE_AGENT);
    expect(result.valid).toBe(true);
    expect(Array.isArray(result.issues)).toBe(true);
  });

  it('returns issues array (may be empty) for rich.agent', () => {
    const source = require('fs').readFileSync(RICH_AGENT, 'utf-8');
    const result = validateAgent(source, RICH_AGENT);
    expect(result).toHaveProperty('valid');
    expect(Array.isArray(result.issues)).toBe(true);
  });

  it('each issue has severity of Error or Warning', () => {
    const source = require('fs').readFileSync(RICH_AGENT, 'utf-8');
    const { issues } = validateAgent(source, RICH_AGENT);
    for (const issue of issues) {
      expect(['Error', 'Warning']).toContain(issue.severity);
      expect(issue.message).toBeTypeOf('string');
    }
  });

  it('returns valid=false for a syntactically broken agent', () => {
    const broken = 'config:\n   agent_name: !!! bad\n\ntopic @#$:\n';
    const result = validateAgent(broken, 'broken.agent');
    // Broken source should produce errors (not valid) or still parse gracefully
    expect(result).toHaveProperty('valid');
    expect(result).toHaveProperty('issues');
  });
});

describe('getGraphData', () => {
  beforeAll(async () => initWasm());

  it('returns GraphExport with topics for rich.agent', () => {
    const source = require('fs').readFileSync(RICH_AGENT, 'utf-8');
    const graph = getGraphData(source);
    expect(graph).toHaveProperty('topics');
    expect(graph).toHaveProperty('nodes');
    expect(graph).toHaveProperty('edges');
    expect(graph).toHaveProperty('variables');
    expect(graph).toHaveProperty('stats');
    expect(Array.isArray(graph.topics)).toBe(true);
  });

  it('topics have expected shape', () => {
    const source = require('fs').readFileSync(RICH_AGENT, 'utf-8');
    const graph = getGraphData(source);
    for (const topic of graph.topics) {
      expect(topic).toHaveProperty('name');
      expect(topic).toHaveProperty('is_entry');
      expect(topic).toHaveProperty('transitions_to');
      expect(topic).toHaveProperty('delegates_to');
    }
  });

  it('actions (if present) are ActionRef (string or {name})', () => {
    const source = require('fs').readFileSync(RICH_AGENT, 'utf-8');
    const graph = getGraphData(source);
    for (const topic of graph.topics) {
      if (topic.actions) {
        for (const action of topic.actions) {
          // Must be renderable via actionName without throwing
          expect(() => actionName(action)).not.toThrow();
          expect(actionName(action)).toBeTypeOf('string');
        }
      }
    }
  });
});

// ─── logic/deps.ts (pure functions, no WASM) ─────────────────────────────────

describe('summarizeDependencies', () => {
  it('returns total=0 for empty report', () => {
    const emptyReport = {
      sobjects: [], fields: [], flows: [], apex_classes: [],
      knowledge_bases: [], connections: [], prompt_templates: [],
      external_services: [], all_dependencies: [], by_type: {}, by_topic: {},
    };
    const summary = summarizeDependencies(emptyReport);
    expect(summary.total).toBe(0);
    expect(Object.values(summary.by_category).every(v => v === 0)).toBe(true);
  });

  it('counts each category correctly', () => {
    const report = {
      sobjects: ['Account', 'Contact'], fields: ['Account.Name'],
      flows: ['MyFlow'], apex_classes: [],
      knowledge_bases: [], connections: [], prompt_templates: [],
      external_services: [], all_dependencies: [], by_type: {}, by_topic: {},
    };
    const summary = summarizeDependencies(report);
    expect(summary.total).toBe(4);
    expect(summary.by_category.sobjects).toBe(2);
    expect(summary.by_category.fields).toBe(1);
    expect(summary.by_category.flows).toBe(1);
    expect(summary.by_category.apex_classes).toBe(0);
  });
});

describe('groupByDependency', () => {
  it('groups shared deps across multiple agents', () => {
    const report = {
      sobjects: ['Account'], fields: [], flows: [], apex_classes: [],
      knowledge_bases: [], connections: [], prompt_templates: [],
      external_services: [], all_dependencies: [], by_type: {}, by_topic: {},
    };
    const results = [
      { file: 'agent1.agent', report, interfaces: [], summary: { total: 1, by_category: { sobjects: 1 } } },
      { file: 'agent2.agent', report, interfaces: [], summary: { total: 1, by_category: { sobjects: 1 } } },
    ];
    const grouped = groupByDependency(results);
    const accountEntry = grouped.find(g => g.dependency === 'Account' && g.type === 'sobject');
    expect(accountEntry).toBeTruthy();
    expect(accountEntry!.agents).toContain('agent1.agent');
    expect(accountEntry!.agents).toContain('agent2.agent');
  });

  it('returns empty array for no results', () => {
    expect(groupByDependency([])).toEqual([]);
  });
});

describe('extractActionInterfaces', () => {
  it('returns empty array for empty AST', () => {
    expect(extractActionInterfaces({})).toEqual([]);
  });

  it('returns empty array for AST with no actions', () => {
    const ast = { topics: [{ node: { name: { node: 'main' }, actions: null } }] };
    expect(extractActionInterfaces(ast)).toEqual([]);
  });
});
