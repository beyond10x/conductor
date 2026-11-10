// The ratchet over ux/baseline.json (the ui-iteration-loop skill, § 6), the dashboard's port of the
// documentation site's ux/ratchet.mjs: every value is lower-is-better, and the gate fails on a value
// worse than the baseline, an improvement not taken into it, a missing value, a value the baseline
// does not hold, and a baseline made worse than main's without a decision id.
import { execFileSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import { UX_DIR, APP_DIR } from './adapter.ts';

export interface Value {
  value: number;
  tolerance: number;
  decision?: string;
}

/** The measured values, by scenario and key (`<metric>@<viewport>`). */
export type Metrics = Record<string, Record<string, Value>>;

export interface Baseline {
  about: string;
  scenarios: Metrics;
  tolerances: { counts: number; steps: number; layout_shift: number };
}

/** The path of the baseline in the repository, for `git show`. */
export const BASELINE_IN_REPO = 'crates/conductor/dashboard/ux/baseline.json';
export const baselinePath = join(UX_DIR, 'baseline.json');

const UPDATE = 'run task ui:ux-gate -- --update';

/** The slack a key's tolerance allows around `value`: relative below 1, absolute for a shift. */
function slack(key: string, held: Value): number {
  const tolerance = held.tolerance ?? 0;
  return tolerance > 0 && tolerance < 1 && held.value !== 0 && !key.startsWith('layout_shift')
    ? held.value * tolerance
    : tolerance;
}

export function compare(baseline: Baseline, metrics: Metrics): { ok: boolean; problems: string[] } {
  const problems: string[] = [];
  for (const [scenario, keys] of Object.entries(baseline.scenarios)) {
    for (const [key, held] of Object.entries(keys)) {
      const now = metrics[scenario]?.[key];
      if (now === undefined) {
        problems.push(`missing value ${scenario}/${key}`);
        continue;
      }
      const allowed = slack(key, held);
      if (now.value > held.value + allowed) {
        problems.push(`worse ${scenario}/${key}: ${now.value} > baseline ${held.value} (tolerance ${allowed})`);
      } else if (now.value < held.value - allowed) {
        problems.push(`improvement not taken into the baseline ${scenario}/${key}: ${now.value} < ${held.value}; ${UPDATE}`);
      }
    }
  }
  for (const [scenario, keys] of Object.entries(metrics)) {
    for (const key of Object.keys(keys)) {
      if (baseline.scenarios[scenario]?.[key] === undefined) {
        problems.push(`not in baseline ${scenario}/${key}; ${UPDATE}`);
      }
    }
  }
  return { ok: problems.length === 0, problems };
}

/** The baseline on `origin/main`, or null where main has none yet. */
export function mainBaseline(): Baseline | null {
  try {
    const text = execFileSync('git', ['show', `origin/main:${BASELINE_IN_REPO}`], {
      cwd: APP_DIR,
      stdio: ['ignore', 'pipe', 'ignore'],
    }).toString();
    return JSON.parse(text) as Baseline;
  } catch {
    return null;
  }
}

export function worseThanMain(baseline: Baseline, main: Baseline | null): string[] {
  const problems: string[] = [];
  if (main === null) {
    return problems;
  }
  for (const [scenario, keys] of Object.entries(baseline.scenarios)) {
    for (const [key, held] of Object.entries(keys)) {
      const before = main.scenarios[scenario]?.[key];
      if (before !== undefined && held.value > before.value + (before.tolerance ?? 0) && held.decision === undefined) {
        problems.push(`baseline made worse without a decision id: ${scenario}/${key} ${before.value} -> ${held.value}`);
      }
    }
  }
  return problems;
}

/** The hard gates of the skill's § 4: any value above 0 is a failure the baseline holds only with a decision. */
export const HARD =
  /^(hscroll_states|overflow_states|always_visible|axe_serious|axe_rule|theme_not_applied|completion_miss|false_complete|over_budget|keyboard\.miss|network_requests|unanswered_requests)/;

export function hardFailures(metrics: Metrics): { s: string; k: string; value: number }[] {
  return Object.entries(metrics).flatMap(([s, keys]) =>
    Object.entries(keys)
      .filter(([k, v]) => HARD.test(k) && v.value > 0)
      .map(([k, v]) => ({ s, k, value: v.value })),
  );
}

export function writeBaseline(metrics: Metrics, previous: Baseline | null, decision: string | undefined): Baseline {
  const scenarios: Metrics = {};
  for (const [scenario, keys] of Object.entries(metrics)) {
    const out: Record<string, Value> = {};
    scenarios[scenario] = out;
    for (const [key, now] of Object.entries(keys)) {
      const old = previous?.scenarios[scenario]?.[key];
      const entry: Value = { value: now.value, tolerance: now.tolerance };
      if (old?.decision !== undefined) {
        entry.decision = old.decision;
      }
      if (decision !== undefined && ((old !== undefined && now.value > old.value) || (old === undefined && HARD.test(key) && now.value > 0))) {
        entry.decision = decision;
      }
      out[key] = entry;
    }
  }
  return {
    about:
      'UX ratchet baseline for the conductor dashboard (crates/conductor/dashboard). All values lower is better. Keys carry @<viewport>; the gating viewports are 1440x900 and 1280x800. Each scenario is one page driven state by state from the store fixtures (ux/fixtures.ts) in both themes; the *_states counts are state x theme cases. Rewrite with: task ui:ux-gate -- --update (a hard-gate value above 0 needs --decision <id>).',
    scenarios,
    tolerances: { counts: 0, steps: 0.05, layout_shift: 0.01 },
  };
}

export function readBaseline(): Baseline | null {
  return existsSync(baselinePath) ? (JSON.parse(readFileSync(baselinePath, 'utf8')) as Baseline) : null;
}
