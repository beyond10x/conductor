// @vitest-environment node
// story:dashboard-ui-loop: the ratchet over ux/baseline.json, every value lower is better. It fails
// on a value worse than the baseline, an improvement not taken into it, a missing value, a value
// the baseline does not hold and a baseline made worse than main's without a decision id.
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  compare,
  hardFailures,
  worseThanMain,
  writeBaseline,
  type Baseline,
  type Metrics,
} from '../ux/ratchet.ts';

function baseline(scenarios: Baseline['scenarios']): Baseline {
  return { about: '', scenarios, tolerances: { counts: 0, steps: 0.05, layout_shift: 0.01 } };
}

describe('the ratchet', () => {
  const held = baseline({ decisions: { 'axe_serious_critical_nodes@1280x800': { value: 2, tolerance: 0 } } });

  it('passes the values the baseline holds', () => {
    const now: Metrics = { decisions: { 'axe_serious_critical_nodes@1280x800': { value: 2, tolerance: 0 } } };
    expect(compare(held, now)).toEqual({ ok: true, problems: [] });
  });

  it('fails on a worse value, an improvement not taken, a missing value and an unheld one', () => {
    const worse = compare(held, { decisions: { 'axe_serious_critical_nodes@1280x800': { value: 3, tolerance: 0 } } });
    expect(worse.problems).toEqual(['worse decisions/axe_serious_critical_nodes@1280x800: 3 > baseline 2 (tolerance 0)']);
    const better = compare(held, { decisions: { 'axe_serious_critical_nodes@1280x800': { value: 1, tolerance: 0 } } });
    expect(better.problems).toEqual([
      'improvement not taken into the baseline decisions/axe_serious_critical_nodes@1280x800: 1 < 2; run task ui:ux-gate -- --update',
    ]);
    expect(compare(held, { decisions: {} }).problems).toEqual(['missing value decisions/axe_serious_critical_nodes@1280x800']);
    const unheld = compare(held, {
      decisions: {
        'axe_serious_critical_nodes@1280x800': { value: 2, tolerance: 0 },
        'hscroll_states@1280x800': { value: 0, tolerance: 0 },
      },
    });
    expect(unheld.problems).toEqual(['not in baseline decisions/hscroll_states@1280x800; run task ui:ux-gate -- --update']);
  });

  it('allows a step count its relative tolerance and a layout shift its absolute one', () => {
    const steps = baseline({ decisions: { 'keyboard.tabs@1280x800': { value: 20, tolerance: 0.05 } } });
    expect(compare(steps, { decisions: { 'keyboard.tabs@1280x800': { value: 21, tolerance: 0.05 } } }).ok).toBe(true);
    expect(compare(steps, { decisions: { 'keyboard.tabs@1280x800': { value: 22, tolerance: 0.05 } } }).ok).toBe(false);
    const shift = baseline({ overview: { 'layout_shift_max@1280x800': { value: 0.02, tolerance: 0.01 } } });
    expect(compare(shift, { overview: { 'layout_shift_max@1280x800': { value: 0.029, tolerance: 0.01 } } }).ok).toBe(true);
    expect(compare(shift, { overview: { 'layout_shift_max@1280x800': { value: 0.04, tolerance: 0.01 } } }).ok).toBe(false);
  });

  it('fails on a baseline made worse than main without a decision id', () => {
    const main = baseline({ overview: { 'overflow_states@1280x800': { value: 0, tolerance: 0 } } });
    const raised = baseline({ overview: { 'overflow_states@1280x800': { value: 1, tolerance: 0 } } });
    expect(worseThanMain(raised, main)).toEqual([
      'baseline made worse without a decision id: overview/overflow_states@1280x800 0 -> 1',
    ]);
    const decided = baseline({ overview: { 'overflow_states@1280x800': { value: 1, tolerance: 0, decision: 'R-02' } } });
    expect(worseThanMain(decided, main)).toEqual([]);
    expect(worseThanMain(raised, null)).toEqual([]);
  });

  it('names the hard-gate values above 0', () => {
    expect(
      hardFailures({
        overview: {
          'hscroll_states@1280x800': { value: 1, tolerance: 0 },
          'overflow_states@1280x800': { value: 0, tolerance: 0 },
          'always_visible_violations@1440x900': { value: 2, tolerance: 0 },
          'layout_shift_max@1440x900': { value: 0.3, tolerance: 0.01 },
        },
        decisions: { 'completion_miss@1280x800': { value: 1, tolerance: 0 }, 'keyboard.miss@1280x800': { value: 1, tolerance: 0 } },
      }),
    ).toEqual([
      { s: 'overview', k: 'hscroll_states@1280x800', value: 1 },
      { s: 'overview', k: 'always_visible_violations@1440x900', value: 2 },
      { s: 'decisions', k: 'completion_miss@1280x800', value: 1 },
      { s: 'decisions', k: 'keyboard.miss@1280x800', value: 1 },
    ]);
  });

  it('writes a hard-gate value above 0 with the decision id given, and keeps an old one', () => {
    const metrics: Metrics = {
      overview: { 'hscroll_states@1280x800': { value: 1, tolerance: 0 }, 'axe_rule_cases@1280x800': { value: 0, tolerance: 0 } },
    };
    const written = writeBaseline(metrics, null, 'R-01');
    expect(written.scenarios.overview).toEqual({
      'hscroll_states@1280x800': { value: 1, tolerance: 0, decision: 'R-01' },
      'axe_rule_cases@1280x800': { value: 0, tolerance: 0 },
    });
    expect(writeBaseline(metrics, written, undefined).scenarios.overview?.['hscroll_states@1280x800']).toEqual({
      value: 1,
      tolerance: 0,
      decision: 'R-01',
    });
  });
});

describe('ux/baseline.json', () => {
  const file: Baseline = JSON.parse(readFileSync(join(process.cwd(), 'ux', 'baseline.json'), 'utf8'));

  it('holds every page scenario at both gating viewports', () => {
    expect(Object.keys(file.scenarios).sort()).toEqual(['decisions', 'health', 'kpis', 'overview', 'repository']);
    for (const [scenario, keys] of Object.entries(file.scenarios)) {
      for (const viewport of ['1440x900', '1280x800']) {
        for (const metric of ['hscroll_states', 'overflow_states', 'always_visible_violations', 'axe_serious_critical_nodes']) {
          expect(keys[`${metric}@${viewport}`], `${scenario}/${metric}@${viewport}`).toBeDefined();
        }
      }
    }
    expect(file.scenarios.decisions?.['completion_miss@1280x800']).toBeDefined();
    expect(file.scenarios.decisions?.['false_complete@1440x900']).toBeDefined();
  });

  it('holds no hard-gate value above 0 without a decision id', () => {
    const open = hardFailures(file.scenarios).filter(({ s, k }) => file.scenarios[s]?.[k]?.decision === undefined);
    expect(open).toEqual([]);
  });
});
