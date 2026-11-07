// story:dashboard-ui-loop: no S1 or S2 finding under ux/findings/ is closed without a regression
// asset (skill § 5), and each record carries every header field.
import { describe, expect, it } from 'vitest';
import { join } from 'node:path';
import { parse, problems, readFindings } from '../ux/findings';

const DIR = join(process.cwd(), 'ux', 'findings');

describe('the UI loop findings', () => {
  it('has at least one record to check', () => {
    expect(readFindings(DIR).length).toBeGreaterThan(0);
  });

  it('closes no S1 or S2 finding without a regression asset', () => {
    const found = readFindings(DIR).flatMap(([name, finding]) => problems(name, finding));
    expect(found).toEqual([]);
  });

  it('fails on a closed S2 record with no regression asset', () => {
    const record = parse(
      [
        '---',
        'id: UXF-999',
        'title: a header that wraps',
        'class: layout',
        'severity: S2',
        'metric: layout.failed',
        'status: closed',
        'found_by: L2',
        'evidence_recording: none',
        'evidence_screenshot: shots/x.png',
        'evidence_lines: -',
        'regression_asset: none',
        '---',
        '',
      ].join('\n'),
    );
    expect(problems('UXF-999.md', record)).toEqual(['UXF-999.md: a closed S2 finding with no regression asset']);
    expect(problems('UXF-999.md', { ...record, severity: 'S3' })).toEqual([]);
    expect(problems('UXF-999.md', { ...record, regression_asset: 'ux/layout.spec.ts' })).toEqual([]);
  });
});
