import { describe, it, expect } from 'vitest';
import * as path from 'path';
import { fileURLToPath } from 'url';
import AgencyImpact from '../../../src/commands/agency/impact.js';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const FIXTURES = path.resolve(__dirname, 'fixtures');

describe('agency impact', () => {
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
});
