// The layout contract (the ui-iteration-loop skill, § 4, hard gates 1, 2 and 5), the dashboard's
// port of the documentation site's ux/contract.mjs. In one page state it reads: a horizontal page
// scroll, any element running past its box, the always-visible elements of the adapter wholly in
// the viewport and uncovered, the theme applied, text cut off without its full value, and axe-core's
// serious and critical violations under the story's tags. breaks.ts proves each check can fail.
import type { Page } from 'playwright';
import { AxeBuilder } from '@axe-core/playwright';
import { ALWAYS_VISIBLE, AXE_TAGS, type Theme, type Viewport } from './adapter.ts';

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
  themeApplied: string | null;
  axe: AxeFinding[] | null;
}

/** Runs in the page: everything the contract reads from the laid-out document. */
function inPage([width, height, visible]: [number, number, { kind: string; selector: string }[]]): InPage {
  const out: InPage = {
    violations: [],
    hscroll: false,
    overflow: [],
    innerScroll: 0,
    truncated: [],
    smallText: 0,
    theme: document.documentElement.getAttribute('data-theme'),
    motion: document.getAnimations().filter((animation) => animation.playState === 'running').length,
  };
  const shown = (el: Element): boolean => {
    const box = el.getBoundingClientRect();
    const style = getComputedStyle(el);
    return box.width > 0 && box.height > 0 && style.visibility !== 'hidden' && style.display !== 'none' && style.opacity !== '0';
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
  out.overflow = [...new Set(out.overflow)].slice(0, 8);
  out.truncated = [...new Set(out.truncated)].slice(0, 8);
  return out;
}

/** The contract over the page as it stands; axe-core too when `axe` is set. */
export async function checkPage(page: Page, viewport: Viewport, theme: Theme, { axe }: { axe: boolean }): Promise<Check> {
  const read = await page.evaluate(inPage, [viewport.width, viewport.height, ALWAYS_VISIBLE] as [
    number,
    number,
    { kind: string; selector: string }[],
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
  return { ...read, viewport: viewport.name, theme, themeApplied: read.theme, axe: found };
}
