import fs from 'node:fs';
import path from 'node:path';
import {execFileSync} from 'node:child_process';
import {uxDir, repoRoot} from './lib.mjs';

export function compare(baseline, metrics) {
  const problems = [];
  for (const [s, ks] of Object.entries(baseline.scenarios)) {
    for (const [k, b] of Object.entries(ks)) {
      const cur = metrics[s]?.[k];
      if (!cur) { problems.push(`missing value ${s}/${k}`); continue; }
      const tol = b.tolerance ?? 0;
      const slack = tol < 1 && tol > 0 && b.value !== 0 && !k.startsWith('layout_shift') ? b.value * tol : tol;
      if (cur.value > b.value + slack) problems.push(`worse ${s}/${k}: ${cur.value} > baseline ${b.value} (tolerance ${slack})`);
      else if (cur.value < b.value - slack) problems.push(`improvement not taken into the baseline ${s}/${k}: ${cur.value} < ${b.value}; run with --update`);
    }
  }
  for (const [s, ks] of Object.entries(metrics)) for (const k of Object.keys(ks)) {
    if (!baseline.scenarios[s]?.[k]) problems.push(`not in baseline ${s}/${k}; run with --update`);
  }
  return {ok: problems.length === 0, problems};
}

export function mainBaseline() {
  try {
    const txt = execFileSync('git', ['show', 'origin/main:ux/baseline.json'], {cwd: repoRoot, stdio: ['ignore', 'pipe', 'ignore']}).toString();
    return JSON.parse(txt);
  } catch {
    return null;
  }
}

export function worseThanMain(baseline, main) {
  const problems = [];
  if (!main) return problems;
  for (const [s, ks] of Object.entries(baseline.scenarios)) for (const [k, b] of Object.entries(ks)) {
    const m = main.scenarios?.[s]?.[k];
    if (m && b.value > m.value + (m.tolerance ?? 0) && !b.decision) problems.push(`baseline made worse without a decision id: ${s}/${k} ${m.value} -> ${b.value}`);
  }
  return problems;
}

export const HARD = /^(hscroll_pages|overflow_pages|always_visible|axe_serious|axe_rule|theme_not_applied|completion_miss|false_complete|over_budget|keyboard\.miss)/;

export function hardFailures(metrics) {
  return Object.entries(metrics).flatMap(([s, ks]) => Object.entries(ks).filter(([k, v]) => HARD.test(k) && v.value > 0).map(([k, v]) => ({s, k, value: v.value})));
}

export function writeBaseline(metrics, prev, decision) {
  const scenarios = {};
  for (const [s, ks] of Object.entries(metrics)) {
    scenarios[s] = {};
    for (const [k, v] of Object.entries(ks)) {
      const old = prev?.scenarios?.[s]?.[k];
      const entry = {value: v.value, tolerance: v.tolerance};
      if (old?.decision) entry.decision = old.decision;
      if (decision && ((old && v.value > old.value) || (!old && HARD.test(k) && v.value > 0))) entry.decision = decision;
      scenarios[s][k] = entry;
    }
  }
  return {
    about: 'UX ratchet baseline for the conductor documentation site. All values lower is better. Keys carry @<viewport>; gating viewports are 1280x800 and 390x844. site-contract counts page x theme cases over the sitemap; the other scenarios are screen-only tasks. Rewrite with: node ux/gate.mjs --update.',
    scenarios,
    tolerances: {counts: 0, steps: 0.05, layout_shift: 0.01},
  };
}

export const baselinePath = path.join(uxDir, 'baseline.json');
export function readBaseline() {
  return fs.existsSync(baselinePath) ? JSON.parse(fs.readFileSync(baselinePath, 'utf8')) : null;
}
