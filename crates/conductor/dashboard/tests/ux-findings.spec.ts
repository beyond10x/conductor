// @vitest-environment node
// story:dashboard-ui-loop: the findings test of the UI iteration loop. Every record under
// ux/findings/ carries the header fields, and a closed S1 or S2 finding names a regression asset
// that exists; the checker is shown to fail on each kind of bad record first.
import { describe, expect, it } from 'vitest';
import { join } from 'node:path';
import { FIELDS, STATUSES, fileIn, findingProblems, readFindings } from '../ux/findings.ts';

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
    const text = record({ status: 'closed', regression_asset: 'tests/ux-uxf-001.spec.ts' });
    expect(findingProblems('UXF-001.md', text, nothing)).toEqual([
      'UXF-001.md: regression asset tests/ux-uxf-001.spec.ts not found',
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

  it('holds the status to its vocabulary, so no other word for closed passes unread', () => {
    expect(STATUSES).toEqual(['open', 'closed', 'duplicate', 'known-loss']);
    for (const status of STATUSES.filter((one) => one !== 'closed')) {
      expect(findingProblems('UXF-001.md', record({ status }), nothing)).toEqual([]);
    }
    for (const status of ['Closed', 'closed in R-02', 'fixed', 'done', 'resolved', 'duplicate of UXF-001']) {
      expect(findingProblems('UXF-001.md', record({ status }), nothing), status).toEqual([
        `UXF-001.md: status ${status} is not one of open, closed, duplicate, known-loss`,
      ]);
    }
  });

  it('takes only a file for a regression asset: no directory, no name without a file in it', () => {
    const here = fileIn(ROOT);
    expect(here('package.json')).toBe(true);
    expect(here('ux')).toBe(false);
    expect(here('ux/no-such-file.ts')).toBe(false);
    for (const regression_asset of ['.', 'ux', 'ux/findings', 'ux/findings/', 'tests']) {
      expect(findingProblems('UXF-001.md', record({ status: 'closed', regression_asset }), () => true), regression_asset).toEqual([
        `UXF-001.md: regression asset ${regression_asset} does not name a file`,
      ]);
    }
    expect(findingProblems('UXF-001.md', record({ status: 'closed', regression_asset: 'none yet; proposed: a case' }), () => true)).toEqual([
      'UXF-001.md: closed S2 without a regression asset',
    ]);
    expect(findingProblems('UXF-001.md', record({ status: 'closed', regression_asset: 'tests/ux-findings.spec.ts#a case' }), here)).toEqual([]);
  });

  it('takes only a test or scenario file for a regression asset, never a record, a shot or a manifest', () => {
    const closed = (regression_asset: string) => findingProblems('UXF-001.md', record({ status: 'closed', regression_asset }), () => true);
    for (const asset of ['tests/ux-findings.spec.ts', 'src/__tests__/App.spec.ts', 'src/views/__tests__/Health.spec.ts', 'ux/scenarios/health/scenario.json', 'ux/breaks.ts']) {
      expect(closed(asset), asset).toEqual([]);
    }
    for (const asset of [
      'ux/findings/UXF-001.md',
      'ux/rounds/R-01/shots/health-full-1280x800-dark.png',
      'package.json',
      'ux/contract.ts',
      'tests/setup.ts',
      'ux/scenarios/../findings/UXF-001.md',
      './tests/ux-findings.spec.ts',
      'node_modules/x/tests/a.spec.ts',
    ]) {
      expect(closed(asset), asset).toEqual([
        `UXF-001.md: regression asset ${asset} is not a test or scenario file (tests/*.spec.ts, src/**/__tests__/*, ux/scenarios/**, ux/breaks.ts)`,
      ]);
    }
  });

  it('holds every .md file in the directory to the record name', () => {
    for (const file of ['UXF-005-header.md', 'UXF-1000.md', 'uxf-006.md', 'README.md']) {
      expect(findingProblems(file, record({ id: file.replace(/\.md$/, '') }), nothing), file).toEqual([`${file}: a record is named UXF-nnn.md`]);
    }
    expect(findingProblems('notes.md', 'no header', nothing)).toEqual(['notes.md: a record is named UXF-nnn.md', 'notes.md: no header']);
    expect(readFindings(join(ROOT, 'tests', 'fixtures-none'))).toEqual([]);
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
      expect(findingProblems(file, text, fileIn(ROOT))).toEqual([]);
    },
  );
});
