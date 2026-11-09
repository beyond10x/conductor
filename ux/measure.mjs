import fs from 'node:fs';
import path from 'node:path';
import {chromium, VIEWPORTS, THEMES, sitemapPaths, scenarios, newThemedContext, settle, pool, uxDir} from './lib.mjs';
import {checkPage, checkReducedMotion} from './contract.mjs';
import {runBot, runKeyboard, summarise} from './bot.mjs';
import {startServer} from './serve.mjs';

export const KEY_PAGES = ['/conductor/', '/conductor/docs/getting-started', '/conductor/docs/guides/start-conductor', '/conductor/docs/guides/configure-an-instance', '/conductor/docs/reference/cli'];

export async function measureAll({gatingOnly = false, shotsDir = null} = {}) {
  const {server, url} = await startServer();
  const browser = await chromium.launch();
  const vps = gatingOnly ? VIEWPORTS.filter((v) => v.gating) : VIEWPORTS;
  const pages = sitemapPaths();
  const rows = [];
  const jobs = pages.flatMap((p) => vps.map((vp) => ({p, vp})));
  await pool(jobs, 4, async ({p, vp}) => {
    for (const theme of THEMES) {
      const ctx = await newThemedContext(browser, vp, theme);
      const page = await ctx.newPage();
      try {
        await page.goto(url + p);
        await settle(page, theme);
        const r = await checkPage(page, vp, theme, {axe: vp.gating});
        rows.push({path: p, ...r});
      } finally {
        await ctx.close();
      }
    }
  });
  rows.sort((a, b) => (a.path + a.viewport + a.theme).localeCompare(b.path + b.viewport + b.theme));

  const motion = {};
  for (const vp of vps.filter((v) => v.gating)) {
    const ctx = await newThemedContext(browser, vp, 'light', {reducedMotion: 'reduce'});
    let total = 0;
    for (const p of pages) {
      const page = await ctx.newPage();
      await page.goto(url + p);
      await settle(page, 'light');
      total += await checkReducedMotion(page);
      await page.close();
    }
    motion[vp.name] = total;
    await ctx.close();
  }

  const scs = scenarios();
  const bot = {};
  for (const sc of scs) {
    bot[sc.name] = {};
    for (const vp of vps.filter((v) => v.gating)) {
      const ctx = await newThemedContext(browser, vp, 'light');
      const page = await ctx.newPage();
      const run = await runBot(page, url, sc);
      await ctx.close();
      const ctx2 = await newThemedContext(browser, vp, 'light');
      const page2 = await ctx2.newPage();
      const kb = await runKeyboard(page2, url, sc);
      await ctx2.close();
      bot[sc.name][vp.name] = {run, keyboard: kb, summary: summarise([run])};
    }
  }

  let shots = null;
  if (shotsDir) shots = await takeShots(browser, url, shotsDir, rows);

  await browser.close();
  server.close();
  return {pages, rows, motion, bot, shots, scenarios: scs.map((s) => s.name), scenarioDefs: scs};
}

async function takeShots(browser, url, dir, rows) {
  fs.mkdirSync(dir, {recursive: true});
  const out = {moments: [], worst: []};
  for (const vp of VIEWPORTS.filter((v) => v.gating)) {
    for (const theme of THEMES) {
      const ctx = await newThemedContext(browser, vp, theme);
      for (const p of KEY_PAGES) {
        const page = await ctx.newPage();
        await page.goto(url + p);
        await settle(page, theme);
        await page.waitForTimeout(750);
        const f = `moment-${p.replace(/^\/conductor\/?/, '').replace(/\//g, '_') || 'home'}-${vp.name}-${theme}.png`;
        await page.screenshot({path: path.join(dir, f)});
        out.moments.push({file: f, path: p, viewport: vp.name, theme});
        await page.close();
      }
      await ctx.close();
    }
  }
  const score = new Map();
  for (const r of rows) {
    if (!['1280x800', '390x844'].includes(r.viewport)) continue;
    const k = `${r.path}|${r.viewport}|${r.theme}`;
    score.set(k, (score.get(k) ?? 0) + r.violations.length * 2 + (r.axe?.length ?? 0) + (r.hscroll ? 5 : 0) + r.overflow.length);
  }
  const worst = [...score.entries()].sort((a, b) => b[1] - a[1]).slice(0, 6);
  for (const [k, s] of worst) {
    const [p, v, theme] = k.split('|');
    const vp = VIEWPORTS.find((x) => x.name === v);
    const ctx = await newThemedContext(browser, vp, theme);
    const page = await ctx.newPage();
    await page.goto(url + p);
    await settle(page, theme);
    await page.waitForTimeout(750);
    const f = `worst-${p.replace(/^\/conductor\/?/, '').replace(/\//g, '_') || 'home'}-${v}-${theme}.png`;
    await page.screenshot({path: path.join(dir, f)});
    out.worst.push({file: f, path: p, viewport: v, theme, score: s});
    await ctx.close();
  }
  for (const theme of THEMES) {
    const frames = out.moments.filter((m) => m.theme === theme);
    const html = `<body style="margin:0;background:#888;font:12px sans-serif"><div style="display:grid;grid-template-columns:repeat(5,1fr);gap:6px;padding:6px">${frames
      .map((m) => `<figure style="margin:0"><img style="width:100%;border:1px solid #333" src="data:image/png;base64,${fs.readFileSync(path.join(dir, m.file)).toString('base64')}"><figcaption>${m.path} ${m.viewport}</figcaption></figure>`)
      .join('')}</div></body>`;
    const ctx = await browser.newContext({viewport: {width: 2000, height: 700}});
    const page = await ctx.newPage();
    await page.setContent(html);
    await page.screenshot({path: path.join(dir, `contact-sheet-${theme}.png`), fullPage: true});
    await ctx.close();
  }
  return out;
}

const sc0 = (m, name) => m.scenarioDefs.find((s) => s.name === name);

export function flatten(m) {
  const gating = VIEWPORTS.filter((v) => v.gating).map((v) => v.name);
  const metrics = {'site-contract': {}};
  const site = metrics['site-contract'];
  for (const v of gating) {
    const rs = m.rows.filter((r) => r.viewport === v);
    const cnt = (f) => rs.filter(f).length;
    site[`hscroll_pages@${v}`] = {value: cnt((r) => r.hscroll), tolerance: 0};
    site[`overflow_pages@${v}`] = {value: cnt((r) => r.overflow.length > 0), tolerance: 0};
    site[`always_visible_violations@${v}`] = {value: rs.reduce((a, r) => a + r.violations.length, 0), tolerance: 0};
    site[`axe_serious_critical_nodes@${v}`] = {value: rs.reduce((a, r) => a + (r.axe ?? []).reduce((b, x) => b + x.nodes, 0), 0), tolerance: 0};
    site[`axe_rule_cases@${v}`] = {value: rs.reduce((a, r) => a + (r.axe?.length ?? 0), 0), tolerance: 0};
    site[`always_visible_defects@${v}`] = {value: new Set(rs.flatMap((r) => r.violations.map((x) => x.kind + ':' + x.why))).size, tolerance: 0};
    site[`theme_not_applied@${v}`] = {value: rs.filter((r) => r.themeApplied !== r.theme).length, tolerance: 0};
    site[`truncated_pages@${v}`] = {value: cnt((r) => r.truncated.length > 0), tolerance: 0};
    site[`layout_shift_max@${v}`] = {value: Math.round(Math.max(0, ...rs.map((r) => r.cls)) * 1000) / 1000, tolerance: 0.01};
    site[`reduced_motion_violations@${v}`] = {value: m.motion[v] ?? 0, tolerance: 0};
  }
  for (const name of m.scenarios) {
    metrics[name] = {};
    for (const v of gating) {
      const b = m.bot[name][v];
      const s = b.summary;
      const put = (k, value, tolerance = 0) => (metrics[name][`${k}@${v}`] = {value, tolerance});
      put('screen_only.actions', s.actions);
      put('screen_only.looks', s.looks);
      put('screen_only.steers', s.steers);
      put('screen_only.time_to_act_steps.max', s.time_to_act_steps_max);
      put('glance_cost.median', s.glance_median);
      put('glance_cost.p95', s.glance_p95);
      put('completion_miss', s.completion_miss);
      put('false_complete', s.false_complete);
      put('over_budget', b.run.actions > sc0(m, name).expectations.maxActions ? 1 : 0);
      put('keyboard.tabs', b.keyboard.tabs, 0.05);
      put('keyboard.miss', b.keyboard.completed ? 0 : 1);
    }
  }
  return metrics;
}

export function measuredOnly(m) {
  const out = {};
  for (const vp of VIEWPORTS.filter((v) => !v.gating)) {
    const rs = m.rows.filter((r) => r.viewport === vp.name);
    out[vp.name] = {
      hscroll_pages: rs.filter((r) => r.hscroll).length,
      overflow_pages: rs.filter((r) => r.overflow.length).length,
      always_visible_violations: rs.reduce((a, r) => a + r.violations.length, 0),
      truncated_pages: rs.filter((r) => r.truncated.length).length,
      layout_shift_max: Math.round(Math.max(0, ...rs.map((r) => r.cls)) * 1000) / 1000,
      scrollable_block_cases: rs.filter((r) => r.innerScroll > 0).length,
    };
  }
  return out;
}
