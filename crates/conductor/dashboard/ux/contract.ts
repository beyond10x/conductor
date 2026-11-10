// The layout contract (the ui-iteration-loop skill, § 4, hard gates 1, 2 and 5), the dashboard's
// port of the documentation site's ux/contract.mjs. In one page state it reads: a horizontal page
// scroll, any element running past its box, the always-visible elements of the adapter wholly in
// the viewport and uncovered, the elements the state must show (a blank page breaks nothing else),
// the theme applied and its palette's page background, text cut off without its full value, and
// axe-core's serious and critical violations under the story's tags. breaks.ts proves each check
// can fail.
import type { Page } from 'playwright';
import { AxeBuilder } from '@axe-core/playwright';
import { ALWAYS_VISIBLE, AXE_TAGS, THEMES, type Present, type Theme, type Viewport } from './adapter.ts';

export interface Violation {
  kind: string;
  why: string;
}

export interface InPage {
  violations: Violation[];
  hscroll: boolean;
  overflow: string[];
  innerScroll: number;
  truncated: string[];
  smallText: number;
  theme: string | null;
  /** The computed background of the page, which each theme's palette sets. */
  background: string;
  /** Each element the state must show and does not, `<selector>: <shown> of <count>`. */
  absent: string[];
  /**
   * On a page that pins the theme the system scheme is not: the background the page shows without
   * the pin, which is the system palette's.
   */
  system: { scheme: string; background: string } | null;
  motion: number;
}

export interface AxeFinding {
  id: string;
  impact: string;
  nodes: number;
  target: string;
  summary: string;
}

export interface Check extends InPage {
  viewport: string;
  theme: Theme;
  /**
   * The theme the page carries; null when it carries none, and when the page without its pin shows
   * the same background under the other system scheme: that scheme's palette is dead.
   */
  themeApplied: string | null;
  axe: AxeFinding[] | null;
}

/** Runs in the page: everything the contract reads from the laid-out document. */
function inPage([width, height, visible, present]: [number, number, { kind: string; selector: string }[], { selector: string; count: number; text?: boolean }[]]): InPage {
  const out: InPage = {
    violations: [],
    hscroll: false,
    overflow: [],
    innerScroll: 0,
    truncated: [],
    smallText: 0,
    theme: document.documentElement.getAttribute('data-theme'),
    background: getComputedStyle(document.body).backgroundColor,
    absent: [],
    system: null,
    motion: document.getAnimations().filter((animation) => animation.playState === 'running').length,
  };
  const shown = (el: Element): boolean => {
    const box = el.getBoundingClientRect();
    // With its ancestors: none without a box, none hidden, none at opacity 0.
    return box.width > 0 && box.height > 0 && el.checkVisibility({ opacityProperty: true, visibilityProperty: true, contentVisibilityAuto: true });
  };
  /** Whether `el` holds text that is on screen: a text node with a box, in a visible element. */
  const hasText = (el: Element): boolean => {
    const walker = document.createTreeWalker(el, NodeFilter.SHOW_TEXT);
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
      const parent = node.parentElement;
      if (parent === null || (node.textContent ?? '').trim() === '' || !parent.checkVisibility({ opacityProperty: true, visibilityProperty: true, contentVisibilityAuto: true })) {
        continue;
      }
      const range = document.createRange();
      range.selectNodeContents(node);
      const box = range.getBoundingClientRect();
      if (box.width > 0 && box.height > 0) {
        return true;
      }
    }
    return false;
  };
  const inView = (box: DOMRect): boolean =>
    box.left >= -0.5 && box.top >= -0.5 && box.right <= width + 0.5 && box.bottom <= height + 0.5;
  const uncovered = (el: Element): boolean => {
    const box = el.getBoundingClientRect();
    const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    return hit !== null && (el.contains(hit) || hit.contains(el));
  };
  const name = (el: Element): string =>
    el.tagName.toLowerCase() +
    (el.id ? `#${el.id}` : '') +
    (typeof el.className === 'string' && el.className.trim() !== '' ? `.${el.className.trim().split(/\s+/)[0]}` : '');
  const need = (kind: string, el: Element | null): void => {
    if (el === null) {
      out.violations.push({ kind, why: 'missing' });
    } else if (!shown(el)) {
      out.violations.push({ kind, why: 'not visible' });
    } else if (!inView(el.getBoundingClientRect())) {
      out.violations.push({ kind, why: 'outside viewport' });
    } else if (!uncovered(el)) {
      out.violations.push({ kind, why: 'covered' });
    }
  };
  for (const { kind, selector } of visible) {
    const el = document.querySelector(selector);
    need(kind, el);
    // A region is in view only when each control in it is: every link and button it holds.
    if (el !== null && out.violations.every((violation) => violation.kind !== kind)) {
      for (const control of el.querySelectorAll('a[href], button, select')) {
        if (shown(control) && (!inView(control.getBoundingClientRect()) || !uncovered(control))) {
          out.violations.push({ kind, why: `control ${name(control)} outside viewport or covered` });
          break;
        }
      }
    }
  }
  for (const { selector, count, text } of present) {
    const filled = (el: Element): boolean => (text === false ? [...el.querySelectorAll('*')].some(shown) : hasText(el));
    const found = [...document.querySelectorAll(selector)].filter((el) => shown(el) && filled(el)).length;
    if (found !== count) {
      out.absent.push(`${selector}: ${found} of ${count}`);
    }
  }
  const root = document.documentElement;
  out.hscroll = root.scrollWidth > root.clientWidth + 1;
  for (const el of document.querySelectorAll('body *')) {
    if (!(el instanceof HTMLElement) || !shown(el)) {
      continue;
    }
    const style = getComputedStyle(el);
    if (style.overflowX === 'visible' && style.display !== 'inline' && el.clientWidth > 0 && el.scrollWidth > el.clientWidth + 1) {
      const edge = el.getBoundingClientRect().right;
      const culprit = [...el.querySelectorAll('*')].find((inner) => {
        const box = inner.getBoundingClientRect();
        if (box.width <= 0 || box.right <= edge + 1) {
          return false;
        }
        for (let up = inner.parentElement; up !== null && up !== el; up = up.parentElement) {
          if (getComputedStyle(up).overflowX !== 'visible') {
            return false;
          }
        }
        return true;
      });
      if (culprit !== undefined) {
        out.overflow.push(`${name(el)} > ${name(culprit)}`);
      }
    }
    if ((style.overflowX === 'auto' || style.overflowX === 'scroll') && el.scrollWidth > el.clientWidth + 1) {
      out.innerScroll++;
    }
    if (style.textOverflow === 'ellipsis' && el.scrollWidth > el.clientWidth + 1 && !el.title && !el.getAttribute('aria-label')) {
      out.truncated.push(`${name(el)}:${(el.textContent ?? '').trim().slice(0, 40)}`);
    }
  }
  const main = document.querySelector('main');
  if (main !== null) {
    const walker = document.createTreeWalker(main, NodeFilter.SHOW_TEXT);
    const seen = new Set<Element>();
    for (let node = walker.nextNode(); node !== null; node = walker.nextNode()) {
      const parent = node.parentElement;
      if (parent === null || (node.textContent ?? '').trim() === '' || seen.has(parent) || !shown(parent)) {
        continue;
      }
      seen.add(parent);
      if (parseFloat(getComputedStyle(parent).fontSize) < 14) {
        out.smallText++;
      }
    }
  }
  // Last, since it takes the pin off for one read: what the page shows when it follows the system.
  const scheme = matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light';
  if (out.theme !== null && out.theme !== scheme) {
    root.removeAttribute('data-theme');
    out.system = { scheme, background: getComputedStyle(document.body).backgroundColor };
    root.setAttribute('data-theme', out.theme);
  }
  out.overflow = [...new Set(out.overflow)].slice(0, 8);
  out.truncated = [...new Set(out.truncated)].slice(0, 8);
  return out;
}

/** The contract over the page as it stands, with the elements `present` says it must show; axe-core too when `axe` is set. */
export async function checkPage(page: Page, viewport: Viewport, theme: Theme, { axe, present }: { axe: boolean; present: Present[] }): Promise<Check> {
  const read = await page.evaluate(inPage, [viewport.width, viewport.height, ALWAYS_VISIBLE, present] as [
    number,
    number,
    { kind: string; selector: string }[],
    Present[],
  ]);
  let found: AxeFinding[] | null = null;
  if (axe) {
    const result = await new AxeBuilder({ page }).withTags(AXE_TAGS).analyze();
    found = result.violations
      .filter((violation) => violation.impact === 'serious' || violation.impact === 'critical')
      .map((violation) => ({
        id: violation.id,
        impact: violation.impact ?? '',
        nodes: violation.nodes.length,
        target: violation.nodes[0]?.target.join(' ') ?? '',
        summary: violation.nodes[0]?.failureSummary?.split('\n')[1]?.trim() ?? '',
      }));
  }
  const systemDead = read.system !== null && read.system.background === read.background;
  return { ...read, viewport: viewport.name, theme, themeApplied: systemDead ? null : read.theme, axe: found };
}

/** What one state case shows of its theme. */
export interface Themed {
  scenario: string;
  state: string;
  viewport: string;
  theme: Theme;
  themeApplied: string | null;
  background: string;
}

/**
 * The state cases whose theme is not on screen: the page does not carry the theme pinned, or its
 * background is the one the same state shows in another theme, so the palette rules of one of the
 * two are dead. A case with no counterpart in the other theme is a miss too: nothing compares it.
 */
export function themeMisses(rows: Themed[]): Themed[] {
  return rows.filter((row) => {
    if (row.themeApplied !== row.theme) {
      return true;
    }
    return THEMES.filter((theme) => theme !== row.theme).some((theme) => {
      const other = rows.find((one) => one.scenario === row.scenario && one.state === row.state && one.viewport === row.viewport && one.theme === theme);
      return other === undefined || other.background === row.background;
    });
  });
}

/** What a state shows on the page that pins no theme, under the system scheme `scheme`. */
export interface Unpinned {
  scenario: string;
  state: string;
  viewport: string;
  scheme: Theme;
  /** The theme the page carries, which is none. */
  themeApplied: string | null;
  background: string;
}

/**
 * The unpinned pages that do not follow the system: the page carries a theme, or its background is
 * not the one the same state shows with the theme of that name pinned, so the system palette (the
 * prefers-color-scheme block) of that scheme is dead. A page with no pinned one to compare misses.
 */
export function systemMisses(system: Unpinned[], rows: Themed[]): Unpinned[] {
  return system.filter((page) => {
    const pinned = rows.find((row) => row.scenario === page.scenario && row.state === page.state && row.viewport === page.viewport && row.theme === page.scheme);
    return page.themeApplied !== null || pinned === undefined || pinned.background !== page.background;
  });
}
