// @vitest-environment node
// story:dashboard-ui-loop, adversary cases: what the ratchet, the findings test and the CI job let
// through. Each case states the acceptance line it holds the unit to.
import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { findingProblems } from '../ux/findings.ts';
import { worseThanMain, writeBaseline, type Baseline, type Metrics } from '../ux/ratchet.ts';

const ROOT = process.cwd();
const REPO = join(ROOT, '..', '..', '..');

function baseline(scenarios: Baseline['scenarios']): Baseline {
  return { about: '', scenarios, tolerances: { counts: 0, steps: 0.05, layout_shift: 0.01 } };
}

function record(fields: Record<string, string>): string {
  const all: Record<string, string> = {
    id: 'UXF-001',
    title: 'A finding',
    class: 'layout',
    severity: 'S1',
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

describe('the ratchet, a value already held under a decision', () => {
  // "the ratchet never worse": R-01 accepted 6, not whatever a later --update measures.
  it('does not rise in an update that names no decision', () => {
    const key = 'always_visible_violations@1280x800';
    const main = baseline({ decisions: { [key]: { value: 6, tolerance: 0, decision: 'R-01' } } });
    const now: Metrics = { decisions: { [key]: { value: 60, tolerance: 0 } } };
    const written = writeBaseline(now, main, undefined);
    expect(written.scenarios.decisions?.[key]?.value).toBe(60);
    expect(worseThanMain(written, main)).toEqual([
      `baseline made worse without a decision id: decisions/${key} 6 -> 60`,
    ]);
  });
});

describe('the findings checker, a closed S1 without a regression asset', () => {
  const nothing = () => false;

  // The status field is free text, as the committed records' other fields are.
  it.each(['Closed', 'closed in R-02', 'closed (R-02)', 'fixed', 'done'])('fails when its status reads "%s"', (status) => {
    expect(findingProblems('UXF-001.md', record({ status }), nothing)).not.toEqual([]);
  });

  it('fails when the asset it names is a directory, not an assertion', () => {
    const exists = (asset: string) => existsSync(join(ROOT, asset));
    for (const regression_asset of ['.', 'ux', 'ux/findings']) {
      expect(findingProblems('UXF-001.md', record({ status: 'closed', regression_asset }), exists), regression_asset).not.toEqual([]);
    }
  });
});

describe('the CI task check job', () => {
  // gate.ts reads `git show origin/main:.../baseline.json` and skips the worse-than-main check
  // with a note when that ref is absent; a default checkout of a pull request has no origin/main.
  it('gives the gate the baseline of main to compare with', () => {
    const workflow = readFileSync(join(REPO, '.github', 'workflows', 'check.yml'), 'utf8');
    const job = workflow.slice(0, workflow.indexOf('run: task check'));
    expect(job).toMatch(/fetch-depth: 0|git fetch [^\n]*origin [^\n]*main/);
  });
});
