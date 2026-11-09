import {createRequire} from 'node:module';
import fs from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';

export const uxDir = path.dirname(fileURLToPath(import.meta.url));
export const repoRoot = path.resolve(uxDir, '..');
export const websiteDir = path.join(repoRoot, 'website');
const require = createRequire(path.join(websiteDir, 'package.json'));
export const {chromium} = require('playwright');
export const AxeBuilder = require('@axe-core/playwright').default ?? require('@axe-core/playwright').AxeBuilder;

export const VIEWPORTS = [
  {name: '1440x900', width: 1440, height: 900, gating: false},
  {name: '1280x800', width: 1280, height: 800, gating: true},
  {name: '1024x768', width: 1024, height: 768, gating: false},
  {name: '768x1024', width: 768, height: 1024, gating: false},
  {name: '390x844', width: 390, height: 844, gating: true},
];
export const THEMES = ['light', 'dark'];
export const AXE_TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];
export const BASE = '/conductor/';

export function sitemapPaths() {
  const xml = fs.readFileSync(path.join(websiteDir, 'build/sitemap.xml'), 'utf8');
  const out = [...xml.matchAll(/<loc>([^<]+)<\/loc>/g)].map((m) => new URL(m[1]).pathname);
  return out.sort();
}

export function scenarios() {
  const dir = path.join(uxDir, 'scenarios');
  return fs.readdirSync(dir).sort().map((n) => ({name: n, ...JSON.parse(fs.readFileSync(path.join(dir, n, 'scenario.json'), 'utf8'))}));
}

export async function newThemedContext(browser, vp, theme, extra = {}) {
  const ctx = await browser.newContext({
    viewport: {width: vp.width, height: vp.height},
    colorScheme: theme,
    ...extra,
  });
  await ctx.addInitScript((t) => {
    try { localStorage.setItem('theme', t); } catch {}
    window.__cls = 0;
    try {
      new PerformanceObserver((list) => {
        for (const e of list.getEntries()) if (!e.hadRecentInput) window.__cls += e.value;
      }).observe({type: 'layout-shift', buffered: true});
    } catch {}
  }, theme);
  return ctx;
}

export async function settle(page, theme) {
  await page.waitForLoadState('load');
  await page.evaluate(() => document.fonts.ready);
  await page.waitForFunction((t) => document.documentElement.dataset.theme === t, theme, {timeout: 5000}).catch(() => {});
  await page.waitForLoadState('networkidle', {timeout: 5000}).catch(() => {});
  await page.waitForTimeout(300);
}

export async function pool(items, n, fn) {
  const results = new Array(items.length);
  let i = 0;
  await Promise.all(Array.from({length: n}, async () => {
    while (i < items.length) {
      const k = i++;
      results[k] = await fn(items[k], k);
    }
  }));
  return results;
}

export function median(a) {
  if (!a.length) return 0;
  const s = [...a].sort((x, y) => x - y);
  const m = Math.floor(s.length / 2);
  return s.length % 2 ? s[m] : (s[m - 1] + s[m]) / 2;
}
export function pct(a, p) {
  if (!a.length) return 0;
  const s = [...a].sort((x, y) => x - y);
  return s[Math.min(s.length - 1, Math.ceil((p / 100) * s.length) - 1)];
}
