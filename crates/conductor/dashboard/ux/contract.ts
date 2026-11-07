// The layout contract and the measurements every scenario state takes (skill § 4, gates 1, 2, 5),
// and how a page is put into a state. The probes run inside the page; each names an element by a
// short CSS path, which is what a known assertion in ux/baseline.json names.
import { createRequire } from 'node:module';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { expect, type Page } from '@playwright/test';
import { ALWAYS_VISIBLE, AXE_TAGS, NOW, THEME_KEY, type Scenario, type State, type Theme } from './adapter';
import { serve, type Served } from './hermetic';

const require = createRequire(import.meta.url);
const AXE = require.resolve('axe-core/axe.min.js');
export const UX = dirname(fileURLToPath(import.meta.url));

/** One failed assertion of the contract: `<gate>:<scenario>/<state>:<what>`, stable across runs. */
export type Assertion = string;

export interface Probe {
  /** How far the page scrolls sideways, in CSS pixels; 0 passes. */
  pageScrollX: number;
  /** Elements whose content runs past their box (overflow visible), innermost only. */
  overflow: string[];
  /** Always-visible elements not fully in view, clipped or covered, each with why. */
  notVisible: string[];
  /** Words of text inside the viewport: the reading load of the state. */
  words: number;
  /** The layout shift (CLS) the page recorded since it loaded. */
  layoutShift: number;
}

/** Puts `page` at `scenario`'s `state` in `theme`: serves the fixtures, loads, waits, interacts. */
export async function enter(page: Page, scenario: Scenario, state: State, theme: Theme): Promise<Served> {
  await page.clock.setFixedTime(NOW);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: 'reduce' });
  await page.addInitScript(
    ([key, value]) => {
      localStorage.setItem(key!, value!);
      (window as unknown as { __shift: number }).__shift = 0;
      new PerformanceObserver((list) => {
        for (const entry of list.getEntries() as unknown as { value: number; hadRecentInput: boolean }[]) {
          if (!entry.hadRecentInput) {
            (window as unknown as { __shift: number }).__shift += entry.value;
          }
        }
      }).observe({ type: 'layout-shift', buffered: true });
    },
    [THEME_KEY, theme],
  );
  const served = await serve(page, state.mode);
  await page.goto(`/${scenario.hash}`);
  await expect(page.locator('html')).toHaveAttribute('data-theme', theme);
  if (state.mode === 'loading') {
    await expect(page.locator('main .state-loading').first()).toBeVisible();
  } else {
    await expect(page.locator('main h1')).toBeVisible();
    await expect(page.locator('main .state-loading')).toHaveCount(0);
    await expect(page.locator('header.top .instance-select')).toBeVisible();
  }
  if (state.then === 'answer') {
    const item = page.locator('article', { hasText: 'Release v2.0 today?' });
    await item.getByRole('button', { name: /^Yes/ }).click();
    await page.getByRole('button', { name: 'Send 1 answer' }).click();
    await expect(page.getByRole('list', { name: 'What was sent' })).toContainText('recorded');
    await page.evaluate(() => window.scrollTo(0, 0));
  }
  // Fonts and the charts' resize observers settle before a probe.
  await page.evaluate(() => document.fonts.ready.then(() => new Promise((done) => requestAnimationFrame(() => done(null)))));
  return served;
}

/** The layout contract's probe of the page as it is. */
export async function probe(page: Page): Promise<Probe> {
  return page.evaluate((always) => {
    const root = document.documentElement;
    const vw = root.clientWidth;
    const vh = root.clientHeight;
    function path(el: Element): string {
      const parts: string[] = [];
      let at: Element | null = el;
      while (at !== null && at !== document.body && parts.length < 4) {
        const classes = [...at.classList].filter((c) => !c.startsWith('data-v-')).slice(0, 2);
        const id = at.id !== '' ? `#${at.id}` : '';
        parts.unshift(`${at.tagName.toLowerCase()}${id}${classes.map((c) => `.${c}`).join('')}`);
        at = at.parentElement;
      }
      return parts.join(' > ');
    }
    // Gate 1: content past its box. An element whose own overflow is visible and whose content is
    // wider than it is; a scroll container (overflow auto or scroll) is allowed.
    const flagged: Element[] = [];
    for (const el of document.body.querySelectorAll('*')) {
      if (el instanceof SVGElement || !(el instanceof HTMLElement)) {
        continue;
      }
      const style = getComputedStyle(el);
      if (style.display === 'inline' || style.display === 'contents' || el.clientWidth === 0) {
        continue;
      }
      if (style.overflowX === 'visible' && el.scrollWidth > el.clientWidth + 1) {
        flagged.push(el);
      }
    }
    const innermost = flagged.filter((el) => !flagged.some((other) => other !== el && el.contains(other)));
    // Gate 2: the always-visible elements.
    const notVisible: string[] = [];
    for (const { name, selector } of always) {
      const found = [...document.querySelectorAll(selector)];
      if (found.length === 0) {
        notVisible.push(`${name}: missing`);
      }
      for (const el of found) {
        const box = el.getBoundingClientRect();
        const label = `${name} ${(el.textContent ?? '').trim().slice(0, 20) || path(el)}`.trim();
        if (box.width === 0 || box.height === 0) {
          notVisible.push(`${label}: no size`);
          continue;
        }
        if (box.left < -0.5 || box.top < -0.5 || box.right > vw + 0.5 || box.bottom > vh + 0.5) {
          notVisible.push(`${label}: outside the viewport`);
          continue;
        }
        let clipped = false;
        for (let up = el.parentElement; up !== null && up !== document.body; up = up.parentElement) {
          const style = getComputedStyle(up);
          if (style.overflowX !== 'visible' || style.overflowY !== 'visible') {
            const outer = up.getBoundingClientRect();
            if (box.left < outer.left - 0.5 || box.right > outer.right + 0.5 || box.top < outer.top - 0.5 || box.bottom > outer.bottom + 0.5) {
              clipped = true;
            }
          }
        }
        if (clipped) {
          notVisible.push(`${label}: clipped`);
          continue;
        }
        const points = [
          [box.left + box.width / 2, box.top + box.height / 2],
          [box.left + 2, box.top + 2],
          [box.right - 2, box.top + 2],
          [box.left + 2, box.bottom - 2],
          [box.right - 2, box.bottom - 2],
        ];
        for (const [x, y] of points) {
          const hit = document.elementFromPoint(x!, y!);
          if (hit !== null && hit !== el && !el.contains(hit)) {
            notVisible.push(`${label}: covered by ${path(hit)}`);
            break;
          }
        }
      }
    }
    // Reading load: the words of each text node whose box is inside the viewport.
    let words = 0;
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
      const text = (node.textContent ?? '').trim();
      if (text === '' || node.parentElement === null) {
        continue;
      }
      const range = document.createRange();
      range.selectNodeContents(node);
      const box = range.getBoundingClientRect();
      if (box.width > 0 && box.bottom > 0 && box.top < vh && box.right > 0 && box.left < vw) {
        words += text.split(/\s+/).length;
      }
    }
    return {
      pageScrollX: Math.max(0, root.scrollWidth - vw),
      overflow: innermost.map(path),
      notVisible,
      words,
      layoutShift: Math.round(((window as unknown as { __shift?: number }).__shift ?? 0) * 1000) / 1000,
    };
  }, ALWAYS_VISIBLE);
}

export interface AxeViolation {
  id: string;
  impact: string;
  targets: string[];
}

/** axe-core's serious and critical violations for the gate's tags. */
export async function axe(page: Page): Promise<AxeViolation[]> {
  await page.addScriptTag({ path: AXE });
  const found = await page.evaluate(async (tags) => {
    const run = (window as unknown as { axe: { run: (c: Document, o: object) => Promise<{ violations: { id: string; impact: string | null; nodes: { target: string[] }[] }[] }> } }).axe;
    const result = await run.run(document, { runOnly: { type: 'tag', values: tags }, resultTypes: ['violations'] });
    return result.violations.map((v) => ({ id: v.id, impact: v.impact ?? 'minor', targets: v.nodes.map((n) => n.target.join(' ')) }));
  }, AXE_TAGS);
  return found.filter((v) => v.impact === 'serious' || v.impact === 'critical');
}

/** The contract's failed assertions for one state, as stable ids. */
export function assertions(where: string, measured: Probe, violations: AxeViolation[]): Assertion[] {
  const failed: Assertion[] = [];
  if (measured.pageScrollX > 0) {
    failed.push(`no-scroll:${where}:page scrolls sideways`);
  }
  for (const el of measured.overflow) {
    failed.push(`no-overflow:${where}:${el}`);
  }
  for (const why of measured.notVisible) {
    failed.push(`always-visible:${where}:${why}`);
  }
  for (const v of violations) {
    failed.push(`axe:${where}:${v.id}`);
  }
  return failed;
}

/** Where a spec's measurements go, one file per test, read by the ratchet and the round command. */
export const OUT = join(UX, '.out', 'measure');

export function record(name: string, value: unknown): void {
  mkdirSync(OUT, { recursive: true });
  writeFileSync(join(OUT, `${name.replace(/[^a-z0-9-]+/gi, '_')}.json`), `${JSON.stringify(value, null, 2)}\n`);
}
