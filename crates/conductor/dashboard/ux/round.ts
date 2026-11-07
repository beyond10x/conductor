// The round command (skill § 3.2; story:dashboard-ui-loop): from what `UX_ROUND=1 playwright test`
// measured (ux/.out/), writes ux/rounds/<round>/report.md, round.json and shots/, and one row of
// ux/trend.md. Deterministic: the same measurements give the same files.
//
//   node ux/round.ts R-01
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { measure } from './ratchet.ts';
import { readBaseline } from './baseline.ts';
import { readFindings } from './findings.ts';

const UX = import.meta.dirname;
const OUT = join(UX, '.out');

interface LayoutRecord {
  kind: 'layout';
  scenario: string;
  state: string;
  viewport: string;
  theme: string;
  gated: boolean;
  probe: { pageScrollX: number; overflow: string[]; notVisible: string[]; words: number; layoutShift: number };
  axe: { id: string; impact: string; targets: string[] }[];
  failed: string[];
}

function records(): LayoutRecord[] {
  const dir = join(OUT, 'measure');
  return readdirSync(dir)
    .filter((f) => f.startsWith('layout-'))
    .sort()
    .map((f) => JSON.parse(readFileSync(join(dir, f), 'utf8')) as LayoutRecord)
    .filter((r) => r.kind === 'layout');
}

function main(round: string): void {
  const dir = join(UX, 'rounds', round);
  const shots = join(dir, 'shots');
  mkdirSync(shots, { recursive: true });
  const all = records();
  const now = measure();
  const baseline = readBaseline();
  const findings = readFindings(join(UX, 'findings'));

  // Per viewport: states checked and assertions failed, gated or measured only.
  const byViewport = new Map<string, { gated: boolean; states: number; failed: number; axe: number }>();
  for (const r of all) {
    const v = byViewport.get(r.viewport) ?? { gated: r.gated, states: 0, failed: 0, axe: 0 };
    v.states += 1;
    v.failed += r.failed.filter((a) => !a.startsWith('axe:')).length;
    v.axe += r.axe.length;
    byViewport.set(r.viewport, v);
  }
  // Worst moments: the gated states with the most failed assertions, one per scenario.
  const worst = all
    .filter((r) => r.gated && r.failed.length > 0 && r.viewport === '1280x800' && r.theme === 'light')
    .sort((a, b) => b.failed.length - a.failed.length || a.scenario.localeCompare(b.scenario))
    .filter((r, at, list) => list.findIndex((o) => o.scenario === r.scenario) === at)
    .slice(0, 3);
  const keep = [
    'overview-loaded-light',
    'overview-loaded-dark',
    'decisions-answered-light',
    ...worst.map((r) => `${r.scenario}-${r.state}-light`),
  ].filter((name, at, list) => list.indexOf(name) === at);
  const kept: string[] = [];
  for (const name of keep) {
    const from = join(OUT, 'shots', `${name}.png`);
    if (existsSync(from)) {
      copyFileSync(from, join(shots, `${name}.png`));
      kept.push(name);
    }
  }

  const gatedStates = all.filter((r) => r.gated).length;
  const scenarioCount = new Set(all.map((r) => r.scenario)).size;
  const stateCount = new Set(all.map((r) => `${r.scenario}/${r.state}`)).size;
  const roundJson = {
    round,
    checked: { scenarios: scenarioCount, states: stateCount, gated_checks: gatedStates, all_checks: all.length },
    viewports: Object.fromEntries(byViewport),
    scenarios: now.scenarios,
    failed: now.failed,
    known: baseline.known,
    findings: findings.map(([, f]) => ({ id: f.id, severity: f.severity, status: f.status, title: f.title })),
    shots: kept.map((name) => `shots/${name}.png`),
  };
  writeFileSync(join(dir, 'round.json'), `${JSON.stringify(roundJson, null, 2)}\n`);

  const lines: string[] = [];
  lines.push(`# ${round}: the dashboard's UI loop, round report`, '');
  lines.push(
    `Hermetic run of ${scenarioCount} scenarios, ${stateCount} states; ${gatedStates} gated checks (2 viewports × 2 themes) and ${all.length - gatedStates} measured-only. No recording, no questionnaire and no L5 probe this round: the trial has no people and no paid level (round 1 is the set-up).`,
    '',
  );
  lines.push('## Viewports', '', '| viewport | gated | states | layout assertions failed | axe serious/critical |', '|---|---|---|---|---|');
  for (const [name, v] of byViewport) {
    lines.push(`| ${name} | ${v.gated ? 'yes' : 'no'} | ${v.states} | ${v.failed} | ${v.axe} |`);
  }
  lines.push('', '## Metrics against the baseline', '', '| scenario | key | value | baseline | bar |', '|---|---|---|---|---|');
  for (const [name, keys] of Object.entries(now.scenarios).sort()) {
    for (const [key, value] of Object.entries(keys).sort()) {
      const held = baseline.scenarios[name]?.[key];
      const bar = key === 'axe.serious_critical' || key === 'layout.failed' ? '0 (hard gate)' : 'lower is better';
      lines.push(`| ${name} | ${key} | ${value} | ${held?.value ?? '—'} | ${bar} |`);
    }
  }
  lines.push('', '## Worst moments', '');
  if (worst.length === 0) {
    lines.push('None: no gated state failed an assertion.');
  }
  for (const r of worst) {
    lines.push(`- ${r.scenario}/${r.state} at ${r.viewport}, ${r.theme}: ${r.failed.length} failed assertions; shots/${r.scenario}-${r.state}-light.png`);
    for (const a of r.failed.slice(0, 5)) {
      lines.push(`  - \`${a}\``);
    }
  }
  const measuredOnly = all.filter((r) => !r.gated && r.failed.length > 0);
  lines.push('', '## Measured-only viewports', '');
  const seen = new Map<string, number>();
  for (const r of measuredOnly) {
    for (const a of r.failed) {
      const kind = `${a.split(':')[0]} at ${r.viewport}`;
      seen.set(kind, (seen.get(kind) ?? 0) + 1);
    }
  }
  if (seen.size === 0) {
    lines.push('No assertion failed at a measured-only viewport.');
  }
  for (const [kind, count] of [...seen].sort()) {
    lines.push(`- ${kind}: ${count} assertions`);
  }
  lines.push('', '## Known assertions', '');
  for (const [pattern, finding] of Object.entries(baseline.known)) {
    lines.push(`- \`${pattern}\`: ${finding}`);
  }
  if (Object.keys(baseline.known).length === 0) {
    lines.push('None.');
  }
  lines.push('', '## Findings', '', '| id | severity | status | title |', '|---|---|---|---|');
  for (const [, f] of findings) {
    lines.push(`| ${f.id} | ${f.severity} | ${f.status} | ${f.title} |`);
  }
  lines.push('', '## Missing inputs', '', '- L4 keyed replay, L5 probe, L6 live sessions and the questionnaire: not run in round 1.', '');
  writeFileSync(join(dir, 'report.md'), lines.join('\n'));

  const trend = join(UX, 'trend.md');
  const head = '# The dashboard UI loop, one row per round\n\n| round | scenarios | states | gated checks | layout failed | axe serious/critical | known | findings open |\n|---|---|---|---|---|---|---|---|\n';
  const rows = existsSync(trend) ? readFileSync(trend, 'utf8').split('\n').filter((l) => l.startsWith('| R-')) : [];
  const total = (key: string) => Object.values(now.scenarios).reduce((n, s) => n + (s[key] ?? 0), 0);
  const open = findings.filter(([, f]) => f.status !== 'closed').length;
  const row = `| ${round} | ${scenarioCount} | ${stateCount} | ${gatedStates} | ${total('layout.failed')} | ${total('axe.serious_critical')} | ${Object.keys(baseline.known).length} | ${open} |`;
  const kept2 = rows.filter((l) => !l.startsWith(`| ${round} |`));
  writeFileSync(trend, `${head}${[...kept2, row].join('\n')}\n`);
  process.stdout.write(`ux round ${round}: ${all.length} checks, ${kept.length} shots\n`);
}

const round = process.argv[2];
if (round === undefined || !/^R-\d{2}$/.test(round)) {
  process.stderr.write('usage: node ux/round.ts R-nn\n');
  process.exit(2);
}
main(round);
