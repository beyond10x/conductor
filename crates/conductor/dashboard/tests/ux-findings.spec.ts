// @vitest-environment node
// story:dashboard-ui-loop: the findings test of the UI iteration loop. Every record under
// ux/findings/ carries the header fields, and a closed S1 or S2 finding names a regression asset
// that exists; the checker is shown to fail on each kind of bad record first.
import { describe, expect, it } from 'vitest';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { FIELDS, findingProblems, readFindings } from '../ux/findings.ts';

const ROOT = process.cwd();

function record(fields: Record<string, string>): string {
  const all: Record<string, string> = {
    id: 'UXF-001',
    title: 'A finding',
    class: 'layout',
    severity: 'S2',
    metric: 'overflow_states@1280x800',
    status: 'open',
    found_by: 'round R-01',
    evidence_recording: 'none',
    evidence_screenshot: 'ux/rounds/R-01/shots/x.png',
    evidence_lines: 'ux/rounds/R-01/report.md:1',
    regression_asset: 'none',
    ...fields,
  };
  return `---\n${Object.entries(all)
    .map(([key, value]) => `${key}: ${value}`)
    .join('\n')}\n---\n\nbody\n`;
}

const nothing = () => false;

describe('the findings checker', () => {
  it('fails on a closed S1 or S2 finding with no regression asset', () => {
    for (const severity of ['S1', 'S2']) {
      expect(findingProblems('UXF-001.md', record({ severity, status: 'closed' }), nothing)).toEqual([
        `UXF-001.md: closed ${severity} without a regression asset`,
      ]);
      expect(
        findingProblems('UXF-001.md', record({ severity, status: 'closed', regression_asset: '' }), nothing),
      ).toEqual([`UXF-001.md: closed ${severity} without a regression asset`]);
    }
  });

  it('fails on a closed S1 or S2 finding whose regression asset does not exist', () => {
    const text = record({ status: 'closed', regression_asset: 'ux/regression/UXF-001.ts' });
    expect(findingProblems('UXF-001.md', text, nothing)).toEqual([
      'UXF-001.md: regression asset ux/regression/UXF-001.ts not found',
    ]);
    expect(findingProblems('UXF-001.md', text, () => true)).toEqual([]);
  });

  it('lets an open finding and a closed S3 finding stand without an asset', () => {
    expect(findingProblems('UXF-001.md', record({ severity: 'S1' }), nothing)).toEqual([]);
    expect(findingProblems('UXF-001.md', record({ severity: 'S3', status: 'closed' }), nothing)).toEqual([]);
  });

  it('fails on a missing header field, a wrong id and a severity outside S1 to S3', () => {
    const text = record({}).replace('found_by: round R-01\n', '');
    expect(findingProblems('UXF-001.md', text, nothing)).toEqual(['UXF-001.md: missing found_by']);
    expect(findingProblems('UXF-002.md', record({}), nothing)).toEqual([
      'UXF-002.md: id UXF-001 does not name the file',
    ]);
    expect(findingProblems('UXF-001.md', record({ severity: 'S4' }), nothing)).toEqual([
      'UXF-001.md: severity S4 is not S1, S2 or S3',
    ]);
    expect(findingProblems('UXF-001.md', 'no header', nothing)).toEqual(['UXF-001.md: no header']);
  });

  it('names every header field of the skill', () => {
    expect(FIELDS).toEqual([
      'id',
      'title',
      'class',
      'severity',
      'metric',
      'status',
      'found_by',
      'evidence_recording',
      'evidence_screenshot',
      'evidence_lines',
      'regression_asset',
    ]);
  });
});

describe('ux/findings/', () => {
  const found = readFindings(join(ROOT, 'ux', 'findings'));

  it('holds a record per confirmed finding of round 1', () => {
    expect(found.length).toBeGreaterThan(0);
  });

  it.each(found.map((finding) => [finding.file, finding.text] as const))(
    '%s passes the findings checker',
    (file, text) => {
      expect(findingProblems(file, text, (asset) => existsSync(join(ROOT, asset)))).toEqual([]);
    },
  );
});
