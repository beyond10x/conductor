import {settle, median, pct} from './lib.mjs';

const MAX_ACTIONS = 14;

function snapshot() {
  const out = [];
  const vw = innerWidth, vh = innerHeight;
  document.querySelectorAll('[data-ux-i]').forEach((e) => e.removeAttribute('data-ux-i'));
  let i = 0;
  for (const el of document.querySelectorAll('a[href], button')) {
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    const drawer = !!el.closest('.navbar-sidebar');
    if (r.width <= 0 || r.height <= 0 || cs.display === 'none' || (cs.visibility === 'hidden' && !drawer)) continue;
    const name = (el.getAttribute('aria-label') || el.textContent || '').trim().replace(/\s+/g, ' ');
    if (!name) continue;
    const t = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    const inView = r.left >= -0.5 && r.top >= -0.5 && r.right <= vw + 0.5 && r.bottom <= vh + 0.5;
    el.setAttribute('data-ux-i', String(i));
    out.push({
      i: i++, name, tag: el.tagName.toLowerCase(),
      href: el.tagName === 'A' ? new URL(el.href).pathname : null,
      external: el.tagName === 'A' && new URL(el.href).origin !== location.origin,
      inView, uncovered: !!t && (el.contains(t) || t.contains(el)),
      inDrawer: !!el.closest('.navbar-sidebar'),
      inSidebar: !!el.closest('.theme-doc-sidebar-container'),
      zone: el.closest('main, article') ? 'main' : 'chrome',
      expanded: el.getAttribute('aria-expanded'),
      cx: r.left + r.width / 2, cy: r.top + r.height / 2, top: r.top,
    });
  }
  return out;
}

function words(n, k) {
  const esc = k.toLowerCase().replace(/[^a-z0-9 ]/g, (c) => '\\' + c);
  return new RegExp('(^|[^a-z0-9])' + esc + '([^a-z0-9]|$)').test(n);
}

function scoreOf(sc, c) {
  const n = c.name.toLowerCase();
  let s = 0;
  for (const k of sc.keywords) if (words(n, k)) s++;
  for (const l of sc.labels) { if (n === l.toLowerCase()) s += 5; else if (words(n, l)) s += 3; }
  for (const v of sc.via ?? []) if (words(n, v)) s += 1;
  return s;
}

async function onTarget(page, sc) {
  const here = new URL(page.url()).pathname;
  if (here !== sc.target) return null;
  const h1 = await page.evaluate(() => {
    const h = document.querySelector('h1');
    if (!h) return false;
    const r = h.getBoundingClientRect();
    return r.width > 0 && r.top >= 0 && r.bottom <= innerHeight;
  });
  const truth = await page.getByText(sc.truthText).first().isVisible().catch(() => false);
  return {h1, truth};
}

export async function runBot(page, base, sc) {
  await page.goto(base + sc.start);
  await settle(page, await page.evaluate(() => document.documentElement.dataset.theme));
  const res = {actions: 0, looks: 0, steers: 0, glances: [], tta: [], completed: false, falseComplete: false, path: [], log: []};
  const visited = new Set([new URL(page.url()).pathname]);
  let sinceNav = 0;
  while (res.actions < MAX_ACTIONS) {
    res.looks++;
    const done = await onTarget(page, sc);
    if (done?.h1) {
      res.completed = true;
      res.falseComplete = !done.truth;
      break;
    }
    const controls = await page.evaluate(snapshot);
    const toggle = controls.find((c) => c.name === 'Toggle navigation bar' && c.inView && c.uncovered);
    const open = controls.some((c) => c.name === 'Close navigation bar' && c.inView);
    const cands = controls
      .filter((c) => c.tag === 'a' && !c.external && !visited.has(c.href))
      .map((c) => ({...c, score: scoreOf(sc, c)}))
      .filter((c) => c.score > 0);
    const ok = (c) => c.inView && c.uncovered && (!c.inDrawer || open);
    const best = (list) => list.sort((a, b) => b.score - a.score || (a.zone === 'main' ? -1 : 1) - (b.zone === 'main' ? -1 : 1) || a.i - b.i)[0];
    const cost = (c) => (c.inDrawer ? (open ? (c.inView ? 0 : Math.ceil(Math.abs(c.top - 400) / 400)) : 1) : c.inView && c.uncovered ? 0 : Math.ceil(Math.abs(c.top - 400) / 400));
    const nearest = (list) => list.sort((a, b) => cost(a) - cost(b) || b.score - a.score)[0];
    const vis = best(cands.filter(ok));
    const any = nearest(cands.filter((c) => !c.inDrawer || toggle || open));
    let act = null;
    if (process.env.UX_DEBUG) console.error(JSON.stringify({toggle: !!toggle, cands: cands.map((c) => [c.name, c.score, cost(c), c.inDrawer]).slice(0, 8)}));
    if (vis) {
      res.glances.push(vis.zone === 'main' ? 1 : 2);
      act = {kind: 'click', c: vis};
    } else if (any) {
      res.glances.push(3);
      if (any.inDrawer && !open && toggle) act = {kind: 'toggle', c: toggle};
            else act = {kind: 'scroll', c: any};
    } else if (toggle && !open) {
      res.glances.push(3);
      act = {kind: 'toggle', c: toggle};
    }
    if (!act) { res.log.push('no visible way forward'); break; }
    res.actions++;
    sinceNav++;
    res.log.push(`${act.kind} ${act.c.name}`);
    const loc = page.locator(`[data-ux-i="${act.c.i}"]`);
    if (act.kind === 'scroll') {
      const x = act.c.inDrawer ? 190 : act.c.inSidebar ? 150 : 640;
      await page.mouse.move(x, 400);
      await page.mouse.wheel(0, act.c.top > 0 ? 400 : -400);
      await page.waitForTimeout(250);
    } else {
      await loc.click();
      await page.waitForTimeout(450);
      if (act.kind === 'click') {
        const p = new URL(page.url()).pathname;
        visited.add(act.c.href);
        res.tta.push(sinceNav);
        sinceNav = 0;
        res.path.push(p);
        if (p !== sc.target) res.steers++;
      }
    }
  }
  return res;
}

async function keyboardPass(page, base, sc, openMenu) {
  await page.goto(base + sc.start);
  await settle(page, await page.evaluate(() => document.documentElement.dataset.theme));
  const res = {tabs: 0, hops: 0, completed: false};
  const visited = new Set([new URL(page.url()).pathname]);
  await page.evaluate(() => document.activeElement?.blur());
  for (let n = 0; n < 80; n++) {
    if ((await onTarget(page, sc))?.h1) { res.completed = true; break; }
    await page.keyboard.press('Tab');
    res.tabs++;
    const f = await page.evaluate(() => {
      const e = document.activeElement;
      if (!e || e === document.body) return null;
      const r = e.getBoundingClientRect();
      const t = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
      return {
        tag: e.tagName.toLowerCase(),
        name: (e.getAttribute('aria-label') || e.textContent || '').trim().replace(/\s+/g, ' '),
        href: e.tagName === 'A' ? new URL(e.href).pathname : null,
        external: e.tagName === 'A' && new URL(e.href).origin !== location.origin,
        expanded: e.getAttribute('aria-expanded'),
        shown: r.width > 0 && r.left >= -0.5 && r.right <= innerWidth + 0.5 && r.top >= -0.5 && r.bottom <= innerHeight + 0.5,
        uncovered: !!t && (e.contains(t) || t.contains(e)),
      };
    });
    if (!f) continue;
    if (openMenu && f.name === 'Toggle navigation bar' && f.expanded === 'false') {
      await page.keyboard.press('Enter');
      res.tabs++;
      await page.waitForTimeout(300);
      continue;
    }
    if (f.tag === 'a' && !f.external && !visited.has(f.href) && f.shown && f.uncovered && scoreOf(sc, f) > 0) {
      visited.add(f.href);
      await page.keyboard.press('Enter');
      res.hops++;
      res.tabs++;
      await page.waitForTimeout(450);
      await page.evaluate(() => document.activeElement?.blur());
    }
  }
  return res;
}

export async function runKeyboard(page, base, sc) {
  const a = await keyboardPass(page, base, sc, false);
  if (a.completed) return a;
  const b = await keyboardPass(page, base, sc, true);
  return {...b, tabs: a.tabs + b.tabs, hops: a.hops + b.hops};
}

export function summarise(runs) {
  const all = runs.flatMap((r) => r.glances);
  return {
    actions: runs.reduce((a, r) => a + r.actions, 0),
    looks: runs.reduce((a, r) => a + r.looks, 0),
    steers: runs.reduce((a, r) => a + r.steers, 0),
    time_to_act_steps_max: Math.max(0, ...runs.flatMap((r) => r.tta)),
    glance_median: median(all),
    glance_p95: pct(all, 95),
    completion_miss: runs.filter((r) => !r.completed).length,
    false_complete: runs.filter((r) => r.falseComplete).length,
  };
}
