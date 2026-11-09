import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {VIEWPORTS, uxDir, repoRoot} from './lib.mjs';
import {measureAll, flatten, measuredOnly} from './measure.mjs';
import {compare} from './ratchet.mjs';

const args = process.argv.slice(2);
const round = args[0];
if (!/^R-\d\d$/.test(round ?? '')) {
  console.error('usage: node ux/round.mjs R-nn [--no-shots]');
  process.exit(2);
}
const noShots = args.includes('--no-shots');
const dir = path.join(uxDir, 'rounds', round);
fs.mkdirSync(dir, {recursive: true});

const t0 = Date.now();
const m = await measureAll({shotsDir: noShots ? null : path.join(dir, 'shots')});
const metrics = flatten(m);
const baselinePath = path.join(uxDir, 'baseline.json');
const baseline = fs.existsSync(baselinePath) ? JSON.parse(fs.readFileSync(baselinePath, 'utf8')) : null;
const cmp = baseline ? compare(baseline, metrics) : null;

const commit = execFileSync('git', ['rev-parse', '--short', 'HEAD'], {cwd: repoRoot}).toString().trim();
const dirty = execFileSync('git', ['status', '--porcelain'], {cwd: repoRoot}).toString().split('\n').filter(Boolean).map((l) => l.trim());
const prevRounds = fs.readdirSync(path.join(uxDir, 'rounds')).filter((r) => /^R-\d\d$/.test(r) && r < round).sort();
const prev = prevRounds.length ? JSON.parse(fs.readFileSync(path.join(uxDir, 'rounds', prevRounds.at(-1), 'round.json'), 'utf8')) : null;

const gateKeys = (k) => k.includes('@');
const details = {};
for (const vp of VIEWPORTS.filter((v) => v.gating)) {
  const rs = m.rows.filter((r) => r.viewport === vp.name);
  const agg = {};
  for (const r of rs) for (const v of r.violations) {
    const k = `${v.kind}: ${v.why} (${v.state})`;
    (agg[k] ??= new Set()).add(`${r.path}[${r.theme}]`);
  }
  const axe = {};
  for (const r of rs) for (const a of r.axe ?? []) {
    const k = `${a.id} (${a.impact}), ${a.summary ?? ''}`;
    axe[k] ??= {pages: new Set(), nodes: 0};
    axe[k].pages.add(r.path);
    axe[k].nodes += a.nodes;
  }
  details[vp.name] = {
    violations: Object.fromEntries(Object.entries(agg).map(([k, v]) => [k, v.size])),
    axe: Object.fromEntries(Object.entries(axe).map(([k, v]) => [k, {pages: v.pages.size, nodes: v.nodes}])),
    overflow: [...new Set(rs.flatMap((r) => r.overflow))].slice(0, 10),
    truncated: [...new Set(rs.flatMap((r) => r.truncated))].slice(0, 10),
    smallTextPages: new Set(rs.filter((r) => r.smallText > 0).map((r) => r.path)).size,
  };
}

const roundJson = {
  round,
  date: new Date().toISOString(),
  commit,
  tree_dirty: dirty,
  pages: m.pages.length,
  viewports: VIEWPORTS.map((v) => ({name: v.name, gating: v.gating})),
  themes: ['light', 'dark'],
  levels: ['L1', 'L2', 'L3', 'L7'],
  skipped_levels: {L0: 'no guidance cue in a docs site', L4: 'paid, not requested', L5: 'paid, not requested', L6: 'no people in this round', L8: 'paid, not requested'},
  gating_metrics: metrics,
  measured_only: measuredOnly(m),
  details,
  motion_reduced_violations: m.motion,
  bot: Object.fromEntries(Object.entries(m.bot).map(([s, v]) => [s, Object.fromEntries(Object.entries(v).map(([vp, x]) => [vp, {summary: x.summary, path: x.run.path, log: x.run.log, keyboard: x.keyboard}]))])),
  ratchet: cmp ? {ok: cmp.ok, problems: cmp.problems} : null,
  duration_s: Math.round((Date.now() - t0) / 1000),
  shots: m.shots,
};
fs.writeFileSync(path.join(dir, 'round.json'), JSON.stringify(roundJson, null, 2) + '\n');

const L = [];
L.push(`# ${round} report`, '', `Commit ${commit} (tree dirty: ${dirty.length} paths, listed in round.json), ${roundJson.date}, ${m.pages.length} pages from the sitemap, viewports ${VIEWPORTS.map((v) => v.name + (v.gating ? '*' : '')).join(', ')} (* gating), themes light and dark. Levels L1-L3 and L7 only; L0 (no guidance cue), L4, L5, L6 and L8 are not run.`, '');
L.push('## Gating metrics (lower is better)', '', '| scenario | metric | value | baseline | last round |', '|---|---|---|---|---|');
for (const [s, ks] of Object.entries(metrics)) for (const [k, v] of Object.entries(ks)) {
  L.push(`| ${s} | ${k} | ${v.value} | ${baseline?.scenarios?.[s]?.[k]?.value ?? 'none'} | ${prev?.gating_metrics?.[s]?.[k]?.value ?? 'none'} |`);
}
L.push('', '## Measured only (not gated)', '', '| viewport | hscroll pages | overflow pages | always-visible violations | truncated pages | layout shift max | cases with a scrolling code block or table |', '|---|---|---|---|---|---|---|');
for (const [v, x] of Object.entries(roundJson.measured_only)) L.push(`| ${v} | ${x.hscroll_pages} | ${x.overflow_pages} | ${x.always_visible_violations} | ${x.truncated_pages} | ${x.layout_shift_max} | ${x.scrollable_block_cases} |`);
for (const v of Object.keys(details)) {
  L.push('', `## Detail at ${v}`, '', 'Always-visible defects (kind: why (first state): page x theme cases of ' + m.pages.length * 2 + '):');
  for (const [k, n] of Object.entries(details[v].violations)) L.push(`- ${k}: ${n}`);
  L.push('', 'axe serious or critical, by rule (pages affected of ' + m.pages.length + ', failing nodes over both themes):');
  for (const [k, n] of Object.entries(details[v].axe)) L.push(`- ${k}: ${n.pages} pages, ${n.nodes} nodes`);
  L.push('', `Overflowing elements: ${details[v].overflow.join(', ') || 'none'}`, `Truncated without title: ${details[v].truncated.join(' | ') || 'none'}`, `Pages with main-content text under 14 px: ${details[v].smallTextPages}`);
}
L.push('', '## Screen-only user (L3)', '', '| scenario | viewport | completed | actions | path | keyboard tabs | keyboard completed |', '|---|---|---|---|---|---|---|');
for (const [s, v] of Object.entries(m.bot)) for (const [vp, x] of Object.entries(v)) L.push(`| ${s} | ${vp} | ${x.run.completed ? 'yes' : 'NO'} | ${x.run.actions} | ${x.run.log.join(' > ')} | ${x.keyboard.tabs} | ${x.keyboard.completed ? 'yes' : 'NO'} |`);
L.push('', 'The bot has no search path: the site has no search control (UXF-001). Cards use the site\'s own link text, so a scenario can pass by label coincidence (configure-an-instance passes in 1 action through the navbar Guides link). The sitemap lists 36 pages; redirect stubs and 404.html are not tested.');
if (m.shots) {
  L.push('', '## Worst moments', '');
  for (const w of m.shots.worst) L.push(`- ${w.path} ${w.viewport} ${w.theme}: score ${w.score}, shots/${w.file}`);
  L.push('', 'Contact sheets: shots/contact-sheet-light.png, shots/contact-sheet-dark.png');
}
L.push('', '## Ratchet', '', cmp ? (cmp.ok ? 'ok' : cmp.problems.map((p) => `- ${p}`).join('\n')) : 'no baseline yet', '', '## Missing inputs', '', '- questionnaire scores: no people in this round', '- L5 probe misses: not run (paid)');
fs.writeFileSync(path.join(dir, 'report.md'), L.join('\n') + '\n');

const trendPath = path.join(uxDir, 'rounds', 'trend.md');
if (!fs.existsSync(trendPath)) fs.writeFileSync(trendPath, '| round | date | commit | hscroll | overflow | always-visible | axe | completion_miss | duration_s |\n|---|---|---|---|---|---|---|---|---|\n');
const sum = (pre) => Object.entries(metrics['site-contract']).filter(([k]) => k.startsWith(pre)).reduce((a, [, v]) => a + v.value, 0);
const cm = Object.entries(metrics).filter(([s]) => s !== 'site-contract').flatMap(([, ks]) => Object.entries(ks)).filter(([k]) => k.startsWith('completion_miss')).reduce((a, [, v]) => a + v.value, 0);
const trend = fs.readFileSync(trendPath, 'utf8').split('\n').filter((l) => !l.startsWith(`| ${round} |`));
fs.writeFileSync(trendPath, trend.filter((l) => l !== '').join('\n') + `\n| ${round} | ${roundJson.date.slice(0, 10)} | ${commit} | ${sum('hscroll_pages')} | ${sum('overflow_pages')} | ${sum('always_visible')} | ${sum('axe')} | ${cm} | ${roundJson.duration_s} |\n`);
console.log(`wrote ${dir} in ${roundJson.duration_s}s`);
