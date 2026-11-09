import {AxeBuilder, AXE_TAGS} from './lib.mjs';

function inPage([vw, vh, desktopMin]) {
  const out = {violations: [], hscroll: false, overflow: [], innerScroll: 0, truncated: [], smallText: 0};
  const vis = (el) => {
    const r = el.getBoundingClientRect();
    const cs = getComputedStyle(el);
    return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.display !== 'none' && cs.opacity !== '0';
  };
  const inView = (r) => r.left >= -0.5 && r.top >= -0.5 && r.right <= vw + 0.5 && r.bottom <= vh + 0.5;
  const uncovered = (el) => {
    const r = el.getBoundingClientRect();
    const t = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
    return !!t && (el.contains(t) || t.contains(el));
  };
  const sel = (el) => el.tagName.toLowerCase() + (el.id ? '#' + el.id : '') + (typeof el.className === 'string' && el.className ? '.' + el.className.trim().split(/\s+/)[0] : '');
  const need = (name, el) => {
    if (!el) { out.violations.push({kind: name, why: 'missing'}); return; }
    if (!vis(el)) { out.violations.push({kind: name, why: 'not visible'}); return; }
    const r = el.getBoundingClientRect();
    if (!inView(r)) { out.violations.push({kind: name, why: 'outside viewport'}); return; }
    if (!uncovered(el)) { out.violations.push({kind: name, why: 'covered'}); return; }
  };
  need('navbar', document.querySelector('nav.navbar'));
  need('search', document.querySelector('.navbarSearchContainer button, .navbarSearchContainer input, .DocSearch-Button, input[type="search"], [role="search"] input, [role="search"] button'));
  need('theme-toggle', document.querySelector('button[aria-label^="Switch between dark and light mode"]'));
  const sb = document.querySelector('.theme-doc-sidebar-container');
  if (vw >= desktopMin) {
    if (document.querySelector('.theme-doc-markdown, article')) {
      if (!sb || !vis(sb)) out.violations.push({kind: 'sidebar', why: sb ? 'not visible' : 'missing'});
      else {
        const r = sb.getBoundingClientRect();
        const link = sb.querySelector('a.menu__link');
        if (r.left < -0.5 || r.right > vw + 0.5 || !link || !inView(link.getBoundingClientRect()) || !uncovered(link)) out.violations.push({kind: 'sidebar', why: 'outside viewport or covered'});
      }
    }
  }
  const de = document.documentElement;
  out.hscroll = de.scrollWidth > de.clientWidth + 1;
  for (const el of document.querySelectorAll('body *')) {
    if (!(el instanceof HTMLElement) || !vis(el)) continue;
    const cs = getComputedStyle(el);
    if (el.closest('.navbar-sidebar') && !document.querySelector('.navbar-sidebar--show')) continue;
    if (cs.overflowX === 'visible' && cs.display !== 'inline' && el.scrollWidth > el.clientWidth + 1 && el.clientWidth > 0) {
      const er = el.getBoundingClientRect();
      const culprit = [...el.querySelectorAll('*')].find((d) => {
        const r = d.getBoundingClientRect();
        if (r.width <= 0 || r.right <= er.right + 1 || d.closest('.row')) return false;
        for (let q = d.parentElement; q && q !== el; q = q.parentElement) if (getComputedStyle(q).overflowX !== 'visible') return false;
        return true;
      });
      if (culprit) out.overflow.push(sel(el) + ' > ' + sel(culprit));
    }
    if ((cs.overflowX === 'auto' || cs.overflowX === 'scroll') && el.scrollWidth > el.clientWidth + 1) out.innerScroll++;
    if ((cs.textOverflow === 'ellipsis' || el.classList.contains('text--truncate')) && el.scrollWidth > el.clientWidth + 1 && !el.title && !el.getAttribute('aria-label')) {
      out.truncated.push(sel(el) + ':' + (el.textContent || '').trim().slice(0, 40));
    }
  }
  const main = document.querySelector('main');
  if (main) {
    const w = document.createTreeWalker(main, NodeFilter.SHOW_TEXT);
    const seen = new Set();
    for (let n = w.nextNode(); n; n = w.nextNode()) {
      if (!n.textContent.trim()) continue;
      const p = n.parentElement;
      if (seen.has(p) || !vis(p)) continue;
      seen.add(p);
      if (parseFloat(getComputedStyle(p).fontSize) < 14) out.smallText++;
    }
  }
  out.overflow = [...new Set(out.overflow)].slice(0, 8);
  out.truncated = [...new Set(out.truncated)].slice(0, 8);
  return out;
}

export async function checkPage(page, vp, theme, {axe}) {
  const res = {viewport: vp.name, theme, violations: []};
  res.themeApplied = await page.evaluate(() => document.documentElement.dataset.theme);
  const first = await page.evaluate(inPage, [vp.width, vp.height, 997]);
  res.cls = await page.evaluate(() => window.__cls);
  res.hscroll = first.hscroll;
  res.overflow = first.overflow;
  res.innerScroll = first.innerScroll;
  res.truncated = first.truncated;
  res.smallText = first.smallText;
  res.violations.push(...first.violations.map((v) => ({...v, state: 'load'})));
  if (vp.gating) {
    await page.evaluate(() => window.scrollTo(0, document.documentElement.scrollHeight));
    await page.waitForTimeout(250);
    const end = await page.evaluate(inPage, [vp.width, vp.height, 997]);
    for (const v of end.violations) if (v.kind !== 'sidebar' && !res.violations.some((x) => x.kind === v.kind && x.why === v.why)) res.violations.push({...v, state: 'scroll-end'});
    res.hscroll = res.hscroll || end.hscroll;
    await page.evaluate(() => window.scrollTo(0, 0));
  }
  if (axe) {
    const r = await new AxeBuilder({page}).withTags(AXE_TAGS).analyze();
    res.axe = r.violations
      .filter((v) => v.impact === 'serious' || v.impact === 'critical')
      .map((v) => ({id: v.id, impact: v.impact, nodes: v.nodes.length, target: v.nodes[0]?.target?.join(' '), summary: v.nodes[0]?.failureSummary?.split('\n')[1]?.trim()}));
  }
  return res;
}

export async function checkReducedMotion(page) {
  return page.evaluate(() => {
    const running = document.getAnimations().filter((a) => a.playState === 'running').length;
    const smooth = getComputedStyle(document.documentElement).scrollBehavior === 'smooth' ? 1 : 0;
    return running + smooth;
  });
}
