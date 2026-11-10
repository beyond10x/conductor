// One round's Measure step (the ui-iteration-loop skill, § 3.2) for the dashboard: every scenario
// state at every viewport in both themes into ux/rounds/R-nn/ (report.md, round.json, shots/), and
// one row of ux/trend.md.
//
//   node ux/round.ts R-nn [--no-shots]
import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { APP_DIR, GATING, THEMES, UX_DIR, VIEWPORTS } from './adapter.ts';
import { systemMisses, themeMisses } from './contract.ts';
import { flatten, measureAll, measuredOnly } from './measure.ts';
import { compare, readBaseline } from './ratchet.ts';

const args = process.argv.slice(2);
const round = args[0] ?? '';
if (!/^R-\d\d$/.test(round)) {
  console.error('usage: node ux/round.ts R-nn [--no-shots]');
  process.exit(2);
}
const rounds = join(UX_DIR, 'rounds');
const dir = join(rounds, round);
mkdirSync(dir, { recursive: true });

const started = Date.now();
const measured = await measureAll({ shotsDir: args.includes('--no-shots') ? null : join(dir, 'shots') });
const metrics = flatten(measured);
const baseline = readBaseline();
const ratchet = baseline === null ? null : compare(baseline, metrics);
const git = (...rest: string[]) => execFileSync('git', rest, { cwd: APP_DIR }).toString();
const commit = git('rev-parse', '--short', 'HEAD').trim();
const dirty = git('status', '--porcelain', '--', '.').split('\n').filter(Boolean).map((line) => line.trim());
const earlier = existsSync(rounds) ? readdirSync(rounds).filter((name) => /^R-\d\d$/.test(name) && name < round).sort() : [];
const lastName = earlier.at(-1);
const last = lastName === undefined ? null : (JSON.parse(readFileSync(join(rounds, lastName, 'round.json'), 'utf8')) as { gating_metrics: typeof metrics });

const details: Record<string, unknown> = {};
for (const viewport of VIEWPORTS) {
  const rows = measured.rows.filter((row) => row.viewport === viewport.name);
  const violations: Record<string, string[]> = {};
  for (const row of rows) {
    for (const violation of row.violations) {
      (violations[`${violation.kind}: ${violation.why}`] ??= []).push(`${row.scenario}/${row.state}[${row.theme}]`);
    }
  }
  const axe: Record<string, { cases: string[]; nodes: number; target: string }> = {};
  for (const row of rows) {
    for (const finding of row.axe ?? []) {
      const key = `${finding.id} (${finding.impact}), ${finding.summary}`;
      axe[key] ??= { cases: [], nodes: 0, target: finding.target };
      axe[key].cases.push(`${row.scenario}/${row.state}[${row.theme}]`);
      axe[key].nodes += finding.nodes;
    }
  }
  details[viewport.name] = {
    violations,
    axe,
    overflow: [...new Set(rows.flatMap((row) => row.overflow.map((one) => `${row.scenario}/${row.state}: ${one}`)))].slice(0, 12),
    truncated: [...new Set(rows.flatMap((row) => row.truncated))].slice(0, 12),
    absent: rows.flatMap((row) => row.absent.map((one) => `${row.scenario}/${row.state}[${row.theme}]: ${one}`)),
    theme_misses: [
      ...themeMisses(rows).map((row) => `${row.scenario}/${row.state}[${row.theme}] background ${row.background}`),
      ...systemMisses(measured.system.filter((page) => page.viewport === viewport.name), rows).map((page) => `${page.scenario}/${page.state}[system ${page.scheme}] background ${page.background}`),
    ],
    network: [...new Set(rows.flatMap((row) => row.network))],
    unanswered: [...new Set(rows.flatMap((row) => row.unanswered))],
    small_text_states: rows.filter((row) => row.smallText > 0).length,
  };
}
const date = new Date().toISOString();
const seconds = Math.round((Date.now() - started) / 1000);
const states = measured.scenarios.reduce((sum, scenario) => sum + scenario.states.length, 0);
const summary = {
  round,
  date,
  commit,
  tree_dirty: dirty,
  scenarios: measured.scenarios.map((scenario) => ({ name: scenario.name, page: scenario.page, anchor: scenario.anchor, states: scenario.states.map((state) => state.name) })),
  viewports: VIEWPORTS.map((viewport) => ({ name: viewport.name, gating: viewport.gating })),
  themes: THEMES,
  levels: ['L1', 'L2', 'L3'],
  skipped_levels: {
    L0: 'no guidance cue on the dashboard',
    L4: 'paid, not requested',
    L5: 'paid, not requested',
    L6: 'no people in this round',
    L7: 'the review agents are run by the session after the round command',
    L8: 'paid, not requested',
  },
  gating_metrics: metrics,
  measured_only: measuredOnly(measured),
  details,
  plays: measured.plays,
  system: measured.system,
  rows: measured.rows.map((row) => ({
    scenario: row.scenario,
    state: row.state,
    viewport: row.viewport,
    theme: row.theme,
    violations: row.violations,
    hscroll: row.hscroll,
    overflow: row.overflow,
    truncated: row.truncated,
    absent: row.absent,
    theme_applied: row.themeApplied,
    background: row.background,
    axe: row.axe,
    small_text: row.smallText,
  })),
  ratchet: ratchet,
  duration_s: seconds,
  shots: measured.shots,
};
writeFileSync(join(dir, 'round.json'), `${JSON.stringify(summary, null, 2)}\n`);

const lines: string[] = [];
lines.push(
  `# ${round} report: the dashboard`,
  '',
  `Commit ${commit} (${dirty.length} changed paths under crates/conductor/dashboard, listed in round.json), ${date}. ${measured.scenarios.length} scenarios (${measured.scenarios.map((scenario) => scenario.name).join(', ')}), ${states} states, viewports ${VIEWPORTS.map((viewport) => viewport.name + (viewport.gating ? '*' : '')).join(', ')} (* gating), themes ${THEMES.join(' and ')}: ${measured.rows.length} state cases. Levels L1 to L3; L0 (no guidance cue), L4, L5, L6, L7 and L8 are not run by this command.`,
  '',
  '## Gating metrics (lower is better)',
  '',
  '| scenario | metric | value | baseline | last round |',
  '|---|---|---|---|---|',
);
for (const [scenario, keys] of Object.entries(metrics)) {
  for (const [key, value] of Object.entries(keys)) {
    lines.push(`| ${scenario} | ${key} | ${value.value} | ${baseline?.scenarios[scenario]?.[key]?.value ?? 'none'} | ${last?.gating_metrics[scenario]?.[key]?.value ?? 'none'} |`);
  }
}
lines.push('', '## Measured only (not gated)', '', '| viewport | hscroll states | overflow states | always-visible violations | truncated states | theme not on screen | presence misses | states with a scrolling block | states with text under 14 px |', '|---|---|---|---|---|---|---|---|---|');
for (const [viewport, values] of Object.entries(summary.measured_only)) {
  lines.push(`| ${viewport} | ${values.hscroll_states} | ${values.overflow_states} | ${values.always_visible_violations} | ${values.truncated_states} | ${values.theme_not_applied} | ${values.presence_misses} | ${values.scrolling_block_states} | ${values.small_text_states} |`);
}
for (const viewport of VIEWPORTS) {
  const detail = details[viewport.name] as {
    violations: Record<string, string[]>;
    axe: Record<string, { cases: string[]; nodes: number; target: string }>;
    overflow: string[];
    truncated: string[];
    absent: string[];
    theme_misses: string[];
    network: string[];
    unanswered: string[];
    small_text_states: number;
  };
  lines.push('', `## Detail at ${viewport.name}${viewport.gating ? ' (gating)' : ''}`, '');
  lines.push('Always-visible defects (kind: why: state cases):');
  for (const [key, cases] of Object.entries(detail.violations)) {
    lines.push(`- ${key}: ${cases.length} (${cases.slice(0, 6).join(', ')}${cases.length > 6 ? ', ...' : ''})`);
  }
  if (Object.keys(detail.violations).length === 0) {
    lines.push('- none');
  }
  if (viewport.gating) {
    lines.push('', 'axe serious or critical, by rule (state cases, nodes, first target):');
    for (const [key, found] of Object.entries(detail.axe)) {
      lines.push(`- ${key}: ${found.cases.length} cases, ${found.nodes} nodes, \`${found.target}\` (${found.cases.slice(0, 4).join(', ')}${found.cases.length > 4 ? ', ...' : ''})`);
    }
    if (Object.keys(detail.axe).length === 0) {
      lines.push('- none');
    }
  }
  lines.push(
    '',
    `Elements past their box: ${detail.overflow.join('; ') || 'none'}`,
    `Text cut off without its full value: ${detail.truncated.join(' | ') || 'none'}`,
    `Elements a state names and does not show (shown of named): ${detail.absent.join('; ') || 'none'}`,
    `State cases whose theme is not on screen (not pinned, the background of the other theme, or a system page that is not its scheme's theme): ${detail.theme_misses.join('; ') || 'none'}`,
    `Requests off the loopback origin: ${detail.network.join(', ') || 'none'}`,
    `Requests no fixture answers: ${detail.unanswered.join(', ') || 'none'}`,
    `State cases with text under 14 px in main: ${detail.small_text_states} of ${measured.rows.filter((row) => row.viewport === viewport.name).length}`,
  );
}
lines.push('', '## Screen-only and keyboard-only users (L3)', '', '| scenario | viewport | theme | completed | actions | looks | path | keyboard tabs | keyboard completed |', '|---|---|---|---|---|---|---|---|---|');
for (const [scenario, byViewport] of Object.entries(measured.plays)) {
  for (const [viewport, runs] of Object.entries(byViewport)) {
    for (const run of runs) {
      lines.push(`| ${scenario} | ${viewport} | ${run.theme} | ${run.play.completed ? 'yes' : 'NO'} | ${run.play.actions} | ${run.play.looks} | ${run.play.log.join(' > ')} | ${run.keyboard.tabs} | ${run.keyboard.completed ? 'yes' : 'NO'} |`);
    }
  }
}
if (measured.shots !== null) {
  lines.push('', '## Worst moments', '');
  for (const shot of measured.shots.worst) {
    lines.push(`- ${shot.scenario}/${shot.state} ${shot.viewport} ${shot.theme}: score ${shot.score}, shots/${shot.file}`);
  }
  if (measured.shots.worst.length === 0) {
    lines.push('- none scored above 0');
  }
  lines.push('', `Key moments (each scenario's last state at 1280x800): ${measured.shots.moments.map((shot) => `shots/${shot.file}`).join(', ')}`, '', 'Contact sheets: shots/contact-sheet-light.png, shots/contact-sheet-dark.png');
}
lines.push(
  '',
  '## Ratchet',
  '',
  ratchet === null ? 'no baseline yet' : ratchet.ok ? 'ok' : ratchet.problems.map((problem) => `- ${problem}`).join('\n'),
  '',
  '## Missing inputs',
  '',
  '- questionnaire scores: no people in this round',
  '- L5 probe misses: not run (paid)',
);
writeFileSync(join(dir, 'report.md'), `${lines.join('\n')}\n`);

const trend = join(UX_DIR, 'trend.md');
const head = '| round | date | commit | state cases | hscroll | overflow | always-visible | axe nodes | completion_miss | keyboard.miss | duration_s |\n|---|---|---|---|---|---|---|---|---|---|---|\n';
const sum = (prefix: string) =>
  Object.values(metrics).reduce((total, keys) => total + Object.entries(keys).filter(([key]) => key.startsWith(prefix)).reduce((inner, [, value]) => inner + value.value, 0), 0);
const kept = existsSync(trend) ? readFileSync(trend, 'utf8').split('\n').filter((line) => line.startsWith('| R-') && !line.startsWith(`| ${round} |`)) : [];
const row = `| ${round} | ${date.slice(0, 10)} | ${commit} | ${measured.rows.filter((one) => GATING.some((viewport) => viewport.name === one.viewport)).length} | ${sum('hscroll_states')} | ${sum('overflow_states')} | ${sum('always_visible')} | ${sum('axe_serious')} | ${sum('completion_miss')} | ${sum('keyboard.miss')} | ${seconds} |`;
writeFileSync(trend, `${head}${[...kept, row].join('\n')}\n`);
console.log(`wrote ${dir} and a row of ${trend} in ${seconds}s`);
