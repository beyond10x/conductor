// The ratchet over ux/baseline.json (the ui-iteration-loop skill, § 6), the dashboard's port of the
// documentation site's ux/ratchet.mjs: every value is lower-is-better, and the gate fails on a value
// worse than the baseline, an improvement not taken into it, a missing value, a value the baseline
// does not hold, a tolerance in the baseline that is not the gate's own, and a baseline made worse
// than main's (a higher value or a higher tolerance) without a decision id of its own.
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

/** Whether `id` is a decision id: an empty or blank one (an unset shell variable) is none. */
export function decided(id: string | undefined): id is string {
  return id !== undefined && id.trim() !== '';
}

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
      // The tolerance is the gate's own, measured with the value: one raised by hand in the file
      // would hold any worse value.
      if ((held.tolerance ?? 0) !== now.tolerance) {
        problems.push(`tolerance of ${scenario}/${key} is ${held.tolerance} in the baseline, the gate's is ${now.tolerance}; ${UPDATE}`);
      }
      const allowed = slack(key, { ...held, tolerance: now.tolerance });
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

/**
 * What `origin/main` holds: its baseline; `absent` where main's tree has no baseline file yet;
 * `no-ref` where this checkout has no `origin/main` at all, or its baseline does not read.
 */
export type MainRead = { kind: 'baseline'; baseline: Baseline } | { kind: 'absent' } | { kind: 'no-ref'; why: string };

export function readMain(): MainRead {
  const git = (...args: string[]) => execFileSync('git', args, { cwd: APP_DIR, stdio: ['ignore', 'pipe', 'ignore'] }).toString();
  try {
    git('rev-parse', '--verify', '--quiet', 'origin/main^{commit}');
  } catch {
    return { kind: 'no-ref', why: 'this checkout has no origin/main' };
  }
  try {
    git('cat-file', '-e', `origin/main:${BASELINE_IN_REPO}`);
  } catch {
    return { kind: 'absent' };
  }
  try {
    return { kind: 'baseline', baseline: JSON.parse(git('show', `origin/main:${BASELINE_IN_REPO}`)) as Baseline };
  } catch (error) {
    return { kind: 'no-ref', why: `origin/main:${BASELINE_IN_REPO} does not read: ${String(error)}` };
  }
}

/**
 * What the gate says of a main it could not compare with. In CI (`ci`) a checkout without
 * `origin/main` is a failure, since the worse-than-main check would be skipped on every pull
 * request; elsewhere, and where main holds no baseline yet, it is a note.
 */
export function mainProblems(main: MainRead, ci: boolean): { problems: string[]; notes: string[] } {
  if (main.kind === 'baseline') {
    return { problems: [], notes: [] };
  }
  if (main.kind === 'absent') {
    return { problems: [], notes: [`origin/main has no ${BASELINE_IN_REPO} yet; the worse-than-main check is skipped`] };
  }
  const text = `the worse-than-main check cannot run: ${main.why}`;
  return ci ? { problems: [`${text}; fetch origin main before the gate`], notes: [] } : { problems: [], notes: [`${text}; it is skipped`] };
}

/**
 * The values of `baseline` that are worse than main's without a decision of their own, in value or
 * in tolerance: no decision, a blank one, or the one main already held the key under.
 */
export function worseThanMain(baseline: Baseline, main: Baseline | null): string[] {
  const problems: string[] = [];
  if (main === null) {
    return problems;
  }
  for (const [scenario, keys] of Object.entries(baseline.scenarios)) {
    for (const [key, held] of Object.entries(keys)) {
      const before = main.scenarios[scenario]?.[key];
      if (before === undefined || (decided(held.decision) && held.decision !== before.decision)) {
        continue;
      }
      if (held.value > before.value + (before.tolerance ?? 0)) {
        problems.push(`baseline made worse without a decision id: ${scenario}/${key} ${before.value} -> ${held.value}`);
      }
      if ((held.tolerance ?? 0) > (before.tolerance ?? 0)) {
        problems.push(`baseline tolerance raised without a decision id: ${scenario}/${key} ${before.tolerance ?? 0} -> ${held.tolerance}`);
      }
    }
  }
  return problems;
}

/** The hard gates of the skill's § 4: any value above 0 is a failure the baseline holds only with a decision. */
export const HARD =
  /^(hscroll_states|overflow_states|always_visible|axe_serious|axe_rule|theme_not_applied|completion_miss|false_complete|over_budget|keyboard\.miss|network_requests|unanswered_requests|presence_misses)/;

export function hardFailures(metrics: Metrics): { s: string; k: string; value: number }[] {
  return Object.entries(metrics).flatMap(([s, keys]) =>
    Object.entries(keys)
      .filter(([k, v]) => HARD.test(k) && v.value > 0)
      .map(([k, v]) => ({ s, k, value: v.value })),
  );
}

/**
 * The hard-gate values `--update` may not write with `decision`: one above 0 the baseline does not
 * hold, and one above the value it holds, unless `decision` is an id ([`decided`]) and not the one
 * the lower value was held under.
 */
export function needsDecision(metrics: Metrics, baseline: Baseline | null, decision?: string): { s: string; k: string; value: number }[] {
  return hardFailures(metrics).filter(({ s, k, value }) => {
    const held = baseline?.scenarios[s]?.[k];
    const rose = held === undefined || value > held.value;
    return rose && !(decided(decision) && decision !== held?.decision);
  });
}

/** The hard-gate values above 0 a baseline holds under no decision. */
export function undecided(baseline: Baseline): string[] {
  return hardFailures(baseline.scenarios)
    .filter(({ s, k }) => !decided(baseline.scenarios[s]?.[k]?.decision))
    .map(({ s, k, value }) => `hard-gate value above 0 held without a decision id: ${s}/${k} = ${value}`);
}

/**
 * The baseline of `metrics`. A decision is an acceptance of one value: a value that did not rise
 * keeps the decision it was held under, a value that rose carries `decision` or none, never the
 * old id, also when `decision` repeats it; a blank `decision` is none.
 */
export function writeBaseline(metrics: Metrics, previous: Baseline | null, decision: string | undefined): Baseline {
  const scenarios: Metrics = {};
  for (const [scenario, keys] of Object.entries(metrics)) {
    const out: Record<string, Value> = {};
    scenarios[scenario] = out;
    for (const [key, now] of Object.entries(keys)) {
      const old = previous?.scenarios[scenario]?.[key];
      const entry: Value = { value: now.value, tolerance: now.tolerance };
      const rose = old === undefined ? HARD.test(key) && now.value > 0 : now.value > old.value;
      const fresh = decided(decision) && decision !== old?.decision ? decision : undefined;
      const kept = rose ? fresh : now.value > 0 ? old?.decision : undefined;
      if (decided(kept)) {
        entry.decision = kept;
      }
      out[key] = entry;
    }
  }
  return {
    about:
      'UX ratchet baseline for the conductor dashboard (crates/conductor/dashboard). All values lower is better. Keys carry @<viewport>; the gating viewports are 1440x900 and 1280x800. Each scenario is one page driven state by state from the store fixtures (ux/fixtures.ts) in both themes; the *_states counts are state x theme cases. Rewrite with: task ui:ux-gate -- --update (a hard-gate value that enters above 0 or rises needs --decision <id>, a new id).',
    scenarios,
    tolerances: { counts: 0, steps: 0.05, layout_shift: 0.01 },
  };
}

export function readBaseline(): Baseline | null {
  return existsSync(baselinePath) ? (JSON.parse(readFileSync(baselinePath, 'utf8')) as Baseline) : null;
}
