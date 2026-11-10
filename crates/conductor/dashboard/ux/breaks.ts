// Break-tests of the layout contract (the skill's "break-tests that prove each check can fail"):
// before the gate reads the dashboard, it runs the contract over pages built to break one check
// each, and fails when a check does not fire. A contract that cannot fail gates nothing.
import type { Browser } from 'playwright';
import { GATING, type Theme } from './adapter.ts';
import { checkPage, type Check } from './contract.ts';

const HEADER = `<header class="top"><select aria-label="Instance"><option>alpha</option><option>beta</option></select>
<nav class="views" aria-label="Views"><a href="#/">Overview</a> <a href="#/decisions">Decisions</a></nav></header>`;

function page(header: string, main: string, theme: Theme): string {
  return `<!doctype html><html lang="en" data-theme="${theme}"><head><title>break</title><style>header select, header a { display: inline-block; min-height: 24px; min-width: 24px; padding: 4px; }</style></head><body>${header}<main><h1>A page</h1>${main}</main></body></html>`;
}

interface Case {
  name: string;
  html: string;
  /** Whether the contract read this page as it should: the break found, or the control clean. */
  fired: (check: Check) => boolean;
}

const CASES: Case[] = [
  // The control: a page that breaks nothing passes every check, so each case below fails on its break.
  { name: 'a clean page passes every check', html: page(HEADER, '<p>text</p>', 'light'), fired: (check) => clean(check) },
  { name: 'a horizontal page scroll', html: page(HEADER, '<div style="width:3000px">wide</div>', 'light'), fired: (check) => check.hscroll },
  {
    name: 'an element past its box',
    html: page(HEADER, '<div style="width:120px"><span style="display:inline-block;white-space:nowrap">a line much longer than its box can hold</span></div>', 'light'),
    fired: (check) => check.overflow.length > 0,
  },
  { name: 'the instance selector missing', html: page(HEADER.replace(/<select[\s\S]*<\/select>/, ''), '', 'light'), fired: (check) => has(check, 'instance-selector', 'missing') },
  {
    name: 'the header covered',
    html: page(HEADER, '<div style="position:fixed;inset:0;background:#fff"></div>', 'light'),
    fired: (check) => has(check, 'header', 'covered'),
  },
  {
    name: 'the navigation below the viewport',
    html: page(HEADER.replace('<nav class="views"', '<nav class="views" style="position:absolute;top:5000px"'), '', 'light'),
    fired: (check) => has(check, 'navigation', 'outside viewport'),
  },
  {
    name: 'a navigation link outside the viewport',
    html: page(HEADER.replace('<a href="#/decisions">', '<a style="position:relative;left:4000px" href="#/decisions">'), '', 'light'),
    fired: (check) => check.violations.some((violation) => violation.kind === 'navigation' && violation.why.startsWith('control')),
  },
  { name: 'the theme not applied', html: page(HEADER, '', 'light').replace('data-theme="light"', ''), fired: (check) => check.themeApplied !== 'light' },
  {
    name: 'a serious axe violation',
    html: page(HEADER, '<p style="color:#bbb;background:#fff">low contrast</p><button></button>', 'light'),
    fired: (check) => (check.axe ?? []).length > 0,
  },
];

function clean(check: Check): boolean {
  return check.violations.length === 0 && !check.hscroll && check.overflow.length === 0 && (check.axe ?? []).length === 0 && check.themeApplied === 'light';
}

function has(check: Check, kind: string, why: string): boolean {
  return check.violations.some((violation) => violation.kind === kind && violation.why === why);
}

/** The cases whose check did not fire, by name; empty when the contract can fail on each. */
export async function breakTests(browser: Browser): Promise<{ ran: number; silent: string[] }> {
  const viewport = GATING[1] ?? GATING[0];
  if (viewport === undefined) {
    return { ran: 0, silent: ['no gating viewport'] };
  }
  const silent: string[] = [];
  for (const one of CASES) {
    const context = await browser.newContext({ viewport: { width: viewport.width, height: viewport.height } });
    const tab = await context.newPage();
    await tab.setContent(one.html);
    const check = await checkPage(tab, viewport, 'light', { axe: true });
    if (!one.fired(check)) {
      silent.push(one.name);
    }
    await context.close();
  }
  return { ran: CASES.length, silent };
}
