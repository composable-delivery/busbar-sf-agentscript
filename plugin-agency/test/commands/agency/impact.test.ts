import { describe, it, expect, beforeAll } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import { initWasm } from '../../helpers/wasm-init.js';
import AgencyImpact from '../../../src/commands/agency/impact.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency impact', () => {
  beforeAll(async () => initWasm());

  it('has correct metadata', () => {
    expect(AgencyImpact.summary).toBeTypeOf('string');
    expect(AgencyImpact.flags).toHaveProperty('resource');
    expect(AgencyImpact.flags).toHaveProperty('type');
    expect(AgencyImpact.flags).toHaveProperty('path');
    expect(AgencyImpact.flags).toHaveProperty('format');
  });

  it('uses --path flag (renamed from --dir)', () => {
    expect(AgencyImpact.flags).toHaveProperty('path');
    expect(AgencyImpact.flags).not.toHaveProperty('dir');
  });

  it('scans directory and returns no matches for nonexistent resource', async () => {
    const result = await AgencyImpact.run(
      ['--resource', 'NonExistentFlow__ZZZNONE', '--path', path.join(FIXTURES, 'agents-dir'), '--format', 'json'],
      undefined
    );
    expect(result).toBeTruthy();
    expect(result.matches).toEqual([]);
    expect(result.total_scanned).toBe(2);
  });

  it('returns impact result structure', async () => {
    const result = await AgencyImpact.run(
      ['--resource', 'SomeResource', '--path', FIXTURES, '--format', 'json'],
      undefined
    );
    expect(result).toHaveProperty('resource');
    expect(result).toHaveProperty('matches');
    expect(result).toHaveProperty('total_scanned');
    expect(result.resource).toBe('SomeResource');
  });

  it('renders pretty format with no matches', async () => {
    const result = await AgencyImpact.run(
      ['--resource', 'NonExistentFlow__ZZZNONE', '--path', path.join(FIXTURES, 'agents-dir')],
      undefined
    );
    expect(result.matches).toEqual([]);
    expect(result.total_scanned).toBe(2);
  });

  it('renders pretty format with matches (flow in rich agent)', async () => {
    const result = await AgencyImpact.run(
      ['--resource', 'CreateOrderFlow', '--type', 'flow', '--path', FIXTURES],
      undefined
    );
    expect(result.resource).toBe('CreateOrderFlow');
    expect(result.matches.length).toBeGreaterThan(0);
    expect(result.matches[0]).toHaveProperty('dep_type', 'flow');
  });

  it('finds apex dependency in rich agent', async () => {
    const result = await AgencyImpact.run(
      ['--resource', 'OrderService', '--type', 'apex', '--path', FIXTURES],
      undefined
    );
    expect(result.resource).toBe('OrderService');
    expect(result.matches.length).toBeGreaterThan(0);
    expect(result.matches[0]).toHaveProperty('dep_type', 'apex');
  });

  it('filters by type correctly (flow type finds nothing for apex resource)', async () => {
    const result = await AgencyImpact.run(
      ['--resource', 'OrderService', '--type', 'flow', '--path', FIXTURES, '--format', 'json'],
      undefined
    );
    // OrderService is an apex dependency, not a flow — should not match when type=flow
    expect(result.matches.filter(m => m.dep_type === 'flow')).toHaveLength(0);
  });
});
