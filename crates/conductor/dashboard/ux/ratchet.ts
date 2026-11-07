// The ratchet over ux/baseline.json (skill § 6; story:dashboard-ui-loop). Reads what the browser
// gates measured (ux/.out/measure/, written by `npm run ux:gates`) and exits 1 on:
// - a value worse than the baseline beyond its tolerance;
// - an improvement not taken into the baseline (an improved value is the new baseline in the same
//   merge: `npm run ux:ratchet -- --update`);
// - a missing value, or a measured value the baseline does not hold;
// - a known assertion that no longer fails (it is taken out of `known` with --update), or one whose
//   finding has no record under ux/findings/;
// - a baseline made worse than the one on `UX_BASE_REF` (default `main`) without a decision id.
//
// Run with plain `node` (Node strips the types). `--update [--decision <id>]` rewrites the baseline
// from the measurements; a worse value is written only with --decision, which it keeps beside it.
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join, relative } from 'node:path';
import { execFileSync } from 'node:child_process';
import { BASELINE, matches, readBaseline, type Baseline, type Entry } from './baseline.ts';

const UX = import.meta.dirname;
const MEASURE = join(UX, '.out', 'measure');
const FINDINGS = join(UX, 'findings');

/** The tolerances by kind: counts 0, steps 5 % (relative), layout shift 0.01 (absolute). */
export const TOLERANCES = { counts: 0, steps: 0.05, layout_shift: 0.01 };

function kindOf(key: string): keyof typeof TOLERANCES {
  if (key.startsWith('layout_shift.')) {
    return 'layout_shift';
  }
  if (key.startsWith('glance_cost.') || key.startsWith('reading_load.')) {
    return 'steps';
  }
  return 'counts';
}

/** The allowed distance from `value` for `key`. */
function allowance(key: string, value: number): number {
  const kind = kindOf(key);
  return kind === 'steps' ? Math.abs(value) * TOLERANCES.steps : TOLERANCES[kind];
}

interface LayoutRecord {
  kind: 'layout';
  scenario: string;
  state: string;
  gated: boolean;
  probe: { words: number; layoutShift: number };
  axe: unknown[];
  failed: string[];
}
interface ScreenRecord {
  kind: 'screen-only';
  scenario: string;
  metrics: Record<string, number>;
}

function median(values: number[]): number {
  const sorted = [...values].sort((a, b) => a - b);
  if (sorted.length === 0) {
    return 0;
  }
  const mid = Math.floor(sorted.length / 2);
  return sorted.length % 2 === 1 ? sorted[mid]! : (sorted[mid - 1]! + sorted[mid]!) / 2;
}

export interface Measured {
  scenarios: Record<string, Record<string, number>>;
  failed: string[];
}

/** The per-scenario values of the gated measurements in `dir`. */
export function measure(dir: string = MEASURE): Measured {
  const layout = new Map<string, LayoutRecord[]>();
  const scenarios: Record<string, Record<string, number>> = {};
  const failed: string[] = [];
  if (!existsSync(dir)) {
    return { scenarios, failed };
  }
  for (const file of readdirSync(dir).filter((f) => f.endsWith('.json')).sort()) {
    const one = JSON.parse(readFileSync(join(dir, file), 'utf8')) as LayoutRecord | ScreenRecord;
    if (one.kind === 'screen-only') {
      scenarios[one.scenario] = { ...one.metrics };
    } else if (one.gated) {
      layout.set(one.scenario, [...(layout.get(one.scenario) ?? []), one]);
      failed.push(...one.failed);
    }
  }
  for (const [name, records] of layout) {
    const words = records.map((r) => r.probe.words);
    scenarios[name] = {
      'layout.failed': records.reduce((n, r) => n + r.failed.filter((a) => !a.startsWith('axe:')).length, 0),
      'axe.serious_critical': records.reduce((n, r) => n + r.axe.length, 0),
      'layout_shift.max': Math.max(...records.map((r) => r.probe.layoutShift)),
      'reading_load.max': Math.max(...words),
      'reading_load.median': median(words),
    };
  }
  return { scenarios, failed: failed.sort() };
}

/** Every problem of `now` against `baseline`, and of `baseline` against `base` (main's). */
export function check(baseline: Baseline, now: Measured, base: Baseline | null, findings: string[]): string[] {
  const problems: string[] = [];
  const names = new Set([...Object.keys(baseline.scenarios), ...Object.keys(now.scenarios)]);
  for (const name of [...names].sort()) {
    const held = baseline.scenarios[name] ?? {};
    const got = now.scenarios[name] ?? {};
    for (const key of [...new Set([...Object.keys(held), ...Object.keys(got)])].sort()) {
      const entry = held[key];
      const value = got[key];
      if (value === undefined) {
        problems.push(`${name} ${key}: missing value (the baseline holds ${entry?.value})`);
      } else if (entry === undefined) {
        problems.push(`${name} ${key}: ${value} is not in the baseline`);
      } else if (value > entry.value + entry.tolerance + 1e-9) {
        problems.push(`${name} ${key}: ${value} is worse than the baseline ${entry.value} (tolerance ${entry.tolerance})`);
      } else if (value < entry.value - entry.tolerance - 1e-9 || (entry.tolerance === 0 && value < entry.value)) {
        problems.push(`${name} ${key}: ${value} improves on ${entry.value}; take it into the baseline in this merge`);
      }
    }
  }
  for (const [pattern, finding] of Object.entries(baseline.known)) {
    if (!now.failed.some((assertion) => matches(pattern, assertion))) {
      problems.push(`known ${pattern} (${finding}) no longer fails; take it out of the baseline`);
    }
    if (!findings.includes(finding)) {
      problems.push(`known ${pattern}: ${finding} has no record under ux/findings/`);
    }
  }
  if (base !== null) {
    for (const [name, keys] of Object.entries(base.scenarios)) {
      for (const [key, was] of Object.entries(keys)) {
        const is = baseline.scenarios[name]?.[key];
        if (is === undefined) {
          problems.push(`${name} ${key}: dropped from the baseline that main holds`);
        } else if (is.value > was.value && is.decision === undefined) {
          problems.push(`${name} ${key}: the baseline went from ${was.value} to ${is.value} without a decision id`);
        }
      }
    }
  }
  return problems;
}

/** The baseline `now` makes, keeping `decision` on every value worse than `old`'s. */
export function update(old: Baseline, now: Measured, decision: string | null): { baseline: Baseline; refused: string[] } {
  const refused: string[] = [];
  const scenarios: Record<string, Record<string, Entry>> = {};
  for (const [name, values] of Object.entries(now.scenarios)) {
    scenarios[name] = {};
    for (const [key, value] of Object.entries(values)) {
      const was = old.scenarios[name]?.[key];
      const entry: Entry = { value, tolerance: Math.round(allowance(key, value) * 1000) / 1000 };
      if (was !== undefined && value > was.value + was.tolerance) {
        if (decision === null) {
          refused.push(`${name} ${key}: ${was.value} → ${value} needs --decision <id>`);
          scenarios[name]![key] = was;
          continue;
        }
        entry.decision = decision;
      } else if (was?.decision !== undefined && value >= was.value) {
        entry.decision = was.decision;
      }
      scenarios[name]![key] = entry;
    }
  }
  const known = Object.fromEntries(
    Object.entries(old.known).filter(([pattern]) => now.failed.some((assertion) => matches(pattern, assertion))),
  );
  return {
    baseline: {
      about:
        old.about !== ''
          ? old.about
          : 'The UI loop baseline of the conductor dashboard (story:dashboard-ui-loop): lower is better for every value; written by npm run ux:ratchet -- --update.',
      scenarios,
      tolerances: { ...TOLERANCES },
      known,
    },
    refused,
  };
}

function findingIds(): string[] {
  return existsSync(FINDINGS)
    ? readdirSync(FINDINGS).filter((f) => /^UXF-\d{3}\.md$/.test(f)).map((f) => f.replace(/\.md$/, ''))
    : [];
}

/** Main's baseline, or null when the ref or the file is not there (a shallow CI checkout). */
function baseBaseline(): Baseline | null {
  const ref = process.env.UX_BASE_REF ?? 'main';
  try {
    const top = execFileSync('git', ['rev-parse', '--show-toplevel'], { encoding: 'utf8' }).trim();
    const path = relative(top, BASELINE);
    return JSON.parse(execFileSync('git', ['show', `${ref}:${path}`], { encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'] })) as Baseline;
  } catch {
    process.stderr.write(`ux ratchet: no baseline at ${ref}; the comparison with it is skipped\n`);
    return null;
  }
}

function main(argv: string[]): number {
  const now = measure();
  if (Object.keys(now.scenarios).length === 0) {
    process.stderr.write('ux ratchet: nothing measured under ux/.out/measure; run npm run ux:gates first\n');
    return 1;
  }
  const old = readBaseline();
  if (argv.includes('--update')) {
    const at = argv.indexOf('--decision');
    const decision = at >= 0 ? (argv[at + 1] ?? null) : null;
    const { baseline, refused } = update(old, now, decision);
    writeFileSync(BASELINE, `${JSON.stringify(baseline, null, 2)}\n`);
    for (const line of refused) {
      process.stderr.write(`ux ratchet: kept ${line}\n`);
    }
    return refused.length === 0 ? 0 : 1;
  }
  const problems = check(old, now, baseBaseline(), findingIds());
  for (const line of problems) {
    process.stderr.write(`ux ratchet: ${line}\n`);
  }
  const keys = Object.values(now.scenarios).reduce((n, s) => n + Object.keys(s).length, 0);
  process.stdout.write(`ux ratchet: ${Object.keys(now.scenarios).length} scenarios, ${keys} values, ${problems.length} problems\n`);
  return problems.length === 0 ? 0 : 1;
}

if (process.argv[1] !== undefined && import.meta.filename === process.argv[1]) {
  process.exit(main(process.argv.slice(2)));
}
