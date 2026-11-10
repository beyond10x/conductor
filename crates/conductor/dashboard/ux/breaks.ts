// Break-tests of the layout contract (the skill's "break-tests that prove each check can fail"):
// before the gate reads the dashboard, it runs the contract over pages built to break one check
// each, and fails when a check does not fire. A contract that cannot fail gates nothing.
import type { Browser } from 'playwright';
import { GATING, THEMES, type Present, type Theme, type Viewport } from './adapter.ts';
import { checkPage, themeMisses, type Check, type Themed } from './contract.ts';

const HEADER = `<header class="top"><select aria-label="Instance"><option>alpha</option><option>beta</option></select>
<nav class="views" aria-label="Views"><a href="#/">Overview</a> <a href="#/decisions">Decisions</a></nav></header>`;

/** A palette as the app writes its own (src/style.css): the system's scheme, and the theme a page pins. */
const PALETTE = `
:root, :root[data-theme='light'] { --bg: #ffffff; --text: #111111; }
@media (prefers-color-scheme: dark) { :root:not([data-theme='light']) { --bg: #101010; --text: #eeeeee; } }
:root[data-theme='dark'] { --bg: #101010; --text: #eeeeee; }
body { background: var(--bg); color: var(--text); }`;

/** What every break page must show. */
const PRESENT: Present[] = [
  { selector: 'main h1', count: 1 },
  { selector: 'main p', count: 1 },
];

function page(header: string, main: string, theme: Theme): string {
  return `<!doctype html><html lang="en" data-theme="${theme}"><head><title>break</title><style>header select, header a { display: inline-block; min-height: 24px; min-width: 24px; padding: 4px; }${PALETTE}</style></head><body>${header}<main><h1>A page</h1>${main}</main></body></html>`;
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
  { name: 'a horizontal page scroll', html: page(HEADER, '<p>text</p><div style="width:3000px">wide</div>', 'light'), fired: (check) => check.hscroll },
  {
    name: 'an element past its box',
    html: page(HEADER, '<p>text</p><div style="width:120px"><span style="display:inline-block;white-space:nowrap">a line much longer than its box can hold</span></div>', 'light'),
    fired: (check) => check.overflow.length > 0,
  },
  { name: 'the instance selector missing', html: page(HEADER.replace(/<select[\s\S]*<\/select>/, ''), '<p>text</p>', 'light'), fired: (check) => has(check, 'instance-selector', 'missing') },
  {
    name: 'the header covered',
    html: page(HEADER, '<p>text</p><div style="position:fixed;inset:0;background:#fff"></div>', 'light'),
    fired: (check) => has(check, 'header', 'covered'),
  },
  {
    name: 'the navigation below the viewport',
    html: page(HEADER.replace('<nav class="views"', '<nav class="views" style="position:absolute;top:5000px"'), '<p>text</p>', 'light'),
    fired: (check) => has(check, 'navigation', 'outside viewport'),
  },
  {
    name: 'a navigation link outside the viewport',
    html: page(HEADER.replace('<a href="#/decisions">', '<a style="position:relative;left:4000px" href="#/decisions">'), '<p>text</p>', 'light'),
    fired: (check) => check.violations.some((violation) => violation.kind === 'navigation' && violation.why.startsWith('control')),
  },
  { name: 'the theme not applied', html: page(HEADER, '<p>text</p>', 'light').replace('data-theme="light"', ''), fired: (check) => check.themeApplied !== 'light' },
  {
    name: 'a serious axe violation',
    html: page(HEADER, '<p style="color:#bbb;background:#fff">low contrast</p><button></button>', 'light'),
    fired: (check) => (check.axe ?? []).length > 0,
  },
  {
    name: 'a blank page: everything in main hidden',
    html: page(HEADER, '<p>text</p><style>main > * { display: none !important; }</style>', 'light'),
    fired: (check) => check.absent.length === 2,
  },
  {
    name: 'a page that shows one element fewer than its state names',
    html: page(HEADER, '', 'light'),
    fired: (check) => check.absent.length === 1 && check.absent[0] === 'main p: 0 of 1',
  },
];

function clean(check: Check): boolean {
  return check.violations.length === 0 && !check.hscroll && check.overflow.length === 0 && (check.axe ?? []).length === 0 && check.themeApplied === 'light' && check.absent.length === 0;
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
    const check = await checkPage(tab, viewport, 'light', { axe: true, present: PRESENT });
    if (!one.fired(check)) {
      silent.push(one.name);
    }
    await context.close();
  }
  for (const one of PALETTES) {
    const misses = themeMisses(await themed(browser, viewport, one.css)).length;
    if (!one.fired(misses)) {
      silent.push(one.name);
    }
  }
  return { ran: CASES.length + PALETTES.length, silent };
}

/** The palette rules a theme's colours come from, killed one way each: the selector never matches. */
const PALETTES: { name: string; css: (palette: string) => string; fired: (misses: number) => boolean }[] = [
  // The control: with both palettes live, each theme shows its own background.
  { name: 'both palettes live pass the theme check', css: (palette) => palette, fired: (misses) => misses === 0 },
  {
    name: 'the dark palette rule dead',
    css: (palette) => palette.replace(":root[data-theme='dark']", ":root[data-theme='dark-gone']"),
    fired: (misses) => misses === THEMES.length,
  },
  {
    name: 'every dark palette rule dead',
    css: (palette) => palette.replace(":root[data-theme='dark']", ":root[data-theme='dark-gone']").replace('prefers-color-scheme: dark', 'prefers-color-scheme: dark-gone'),
    fired: (misses) => misses === THEMES.length,
  },
  {
    name: 'the light palette lost under a dark system',
    css: (palette) => palette.replace(":root, :root[data-theme='light']", ':root').replace(":root:not([data-theme='light'])", ':root'),
    fired: (misses) => misses === THEMES.length,
  },
];

/** One page in each theme, as measure.ts opens it: the theme pinned, the system on the other scheme. */
async function themed(browser: Browser, viewport: Viewport, css: (palette: string) => string): Promise<Themed[]> {
  const rows: Themed[] = [];
  for (const theme of THEMES) {
    const context = await browser.newContext({
      viewport: { width: viewport.width, height: viewport.height },
      colorScheme: theme === 'dark' ? 'light' : 'dark',
    });
    const tab = await context.newPage();
    await tab.setContent(page(HEADER, '<p>text</p>', theme).replace(PALETTE, css(PALETTE)));
    const check = await checkPage(tab, viewport, theme, { axe: false, present: PRESENT });
    rows.push({ scenario: 'break', state: 'page', viewport: viewport.name, theme, themeApplied: check.themeApplied, background: check.background });
    await context.close();
  }
  return rows;
}
