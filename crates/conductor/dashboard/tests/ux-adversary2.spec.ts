// @vitest-environment node
// story:dashboard-ui-loop, adversary cases of the second pass: what the corrected gate (the decision
// rule, the presence check, the theme check, the findings checker) still lets through. Each case
// states what it holds the unit to.
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { chromium, type Browser } from 'playwright';
import { GATING, THEMES, type Present, type Theme, type Viewport } from '../ux/adapter.ts';
import { checkPage, themeMisses, type Themed } from '../ux/contract.ts';
import { fileIn, findingProblems, readFindings } from '../ux/findings.ts';
import { compare, undecided, worseThanMain, writeBaseline, type Baseline, type Metrics } from '../ux/ratchet.ts';

const ROOT = process.cwd();
const VIEWPORT = GATING[1] as Viewport;

function baseline(scenarios: Baseline['scenarios']): Baseline {
  return { about: '', scenarios, tolerances: { counts: 0, steps: 0.05, layout_shift: 0.01 } };
}

describe('the decision rule, second pass', () => {
  const key = 'axe_serious_critical_nodes@1280x800';
  const main = baseline({ repository: { [key]: { value: 4, tolerance: 0, decision: 'R-01' } } });
  const worse: Metrics = { repository: { [key]: { value: 152, tolerance: 0 } } };

  // gate.ts asks only `decision === undefined`; `--update --decision ""` (an unset shell variable)
  // is a decision to it, and to undecided() and worseThanMain() after it.
  it('takes no empty decision id for a hard-gate value that rose', () => {
    const written = writeBaseline(worse, main, '');
    expect(written.scenarios.repository?.[key]?.value).toBe(152);
    expect([...undecided(written), ...worseThanMain(written, main)]).not.toEqual([]);
  });

  // writeBaseline's own comment: "a value that rose carries `decision` or none, never the old id".
  it('never writes a risen value under the id the lower value was held under', () => {
    const written = writeBaseline(worse, main, 'R-01');
    expect(written.scenarios.repository?.[key]?.decision).not.toBe('R-01');
  });

  // compare() reads the tolerance out of the baseline file and worseThanMain() reads only values:
  // a baseline whose tolerance was raised by hand holds any worse measurement, with no decision.
  it('does not let a tolerance raised in the baseline hold a worse value than main accepted', () => {
    const edited = baseline({ repository: { [key]: { value: 4, tolerance: 1000, decision: 'R-01' } } });
    expect([...compare(edited, worse).problems, ...undecided(edited), ...worseThanMain(edited, main)]).not.toEqual([]);
  });
});

describe('the findings checker, second pass', () => {
  function record(fields: Record<string, string>): string {
    const all: Record<string, string> = {
      id: 'UXF-001',
      title: 'A finding',
      class: 'layout',
      severity: 'S1',
      metric: 'overflow_states@1280x800',
      status: 'closed',
      found_by: 'round R-01',
      evidence_recording: 'none',
      evidence_screenshot: 'ux/rounds/R-01/shots/x.png',
      evidence_lines: 'ux/rounds/R-01/report.md:1',
      regression_asset: 'none',
      ...fields,
    };
    return `---\n${Object.entries(all)
      .map(([key, value]) => `${key}: ${value}`)
      .join('\n')}\n---\n\nbody\n`;
  }

  // readFindings() keeps only `UXF-nnn.md`; any other record name in ux/findings/ is never checked.
  it('reads a closed S1 record whatever its file is called in ux/findings/', () => {
    const dir = join(ROOT, 'node_modules', `.ux-adversary2-${process.pid}`);
    mkdirSync(dir, { recursive: true });
    try {
      const names = ['UXF-005-header.md', 'UXF-1000.md', 'uxf-006.md'];
      for (const name of names) {
        writeFileSync(join(dir, name), record({ id: name.replace(/\.md$/, '') }));
      }
      const problems = readFindings(dir).flatMap((finding) => findingProblems(finding.file, finding.text, fileIn(ROOT)));
      for (const name of names) {
        expect(problems.some((problem) => problem.startsWith(`${name}:`)), name).toBe(true);
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  // "A test fails on a closed S1 or S2 finding with no regression asset": the record itself, its
  // evidence screenshot and the package manifest are files, and none of them asserts anything.
  it('takes no file that asserts nothing for a regression asset', () => {
    for (const regression_asset of ['ux/findings/UXF-001.md', 'ux/rounds/R-01/shots/health-full-1280x800-dark.png', 'package.json']) {
      expect(findingProblems('UXF-001.md', record({ regression_asset }), fileIn(ROOT)), regression_asset).not.toEqual([]);
    }
  });
});

describe('the contract in a browser, second pass', () => {
  let browser: Browser;
  beforeAll(async () => {
    browser = await chromium.launch();
  }, 60_000);
  afterAll(async () => {
    await browser?.close();
  });

  const HEADER = `<header class="top"><select aria-label="Instance"><option>alpha</option></select>
<nav class="views" aria-label="Views"><a href="#/">Overview</a></nav></header>`;
  const doc = (main: string, style = '', theme: Theme | null = 'light'): string =>
    `<!doctype html><html lang="en"${theme === null ? '' : ` data-theme="${theme}"`}><head><title>t</title><style>td { padding: 6px; }${style}</style></head><body>${HEADER}<main>${main}</main></body></html>`;

  async function absent(html: string, present: Present[]): Promise<string[]> {
    const context = await browser.newContext({ viewport: { width: VIEWPORT.width, height: VIEWPORT.height } });
    const tab = await context.newPage();
    await tab.setContent(html);
    const check = await checkPage(tab, VIEWPORT, 'light', { axe: false, present });
    await context.close();
    return check.absent;
  }

  const ROWS = '<h1>A page</h1><table><tbody><tr><td>api</td><td>3 open</td></tr><tr><td>web</td><td>1 open</td></tr></tbody></table>';
  const PRESENT: Present[] = [
    { selector: 'main h1', count: 1 },
    { selector: 'main tbody tr', count: 2 },
  ];

  it('control: the page with its rows shown has no presence miss', async () => {
    expect(await absent(doc(ROWS), PRESENT)).toEqual([]);
  }, 60_000);

  // shown() reads the named element's own opacity: a wrapper left at opacity 0 (a fade that never
  // ends) blanks the page and every count still matches.
  it('misses a page whose main is transparent', async () => {
    expect(await absent(doc(ROWS, 'main { opacity: 0; }'), PRESENT)).not.toEqual([]);
  }, 60_000);

  // The rows are counted, not read: every cell hidden leaves two rows "shown".
  it('misses rows whose every cell is hidden', async () => {
    expect(await absent(doc(ROWS, 'main td { visibility: hidden; }'), PRESENT)).not.toEqual([]);
  }, 60_000);

  // A render that loses the values (a renamed field) still draws the rows, padded and empty.
  it('misses rows that hold no text', async () => {
    const empty = '<h1>A page</h1><table><tbody><tr><td></td><td></td></tr><tr><td></td><td></td></tr></tbody></table>';
    expect(await absent(doc(empty), PRESENT)).not.toEqual([]);
  }, 60_000);

  // The app's palette as src/style.css writes it, with the system rule dead: a page that pins no
  // theme (the default) is light on a dark system. The gate opens only pinned pages, each under the
  // other system scheme, so its two rows show two backgrounds.
  const PALETTE_SYSTEM_DEAD = `
:root, :root[data-theme='light'] { --bg: #f6f7f9; --text: #1a1f26; }
@media (prefers-color-scheme: dark-gone) { :root:not([data-theme='light']) { --bg: #0f1216; --text: #e6e9ee; } }
:root[data-theme='dark'] { --bg: #0f1216; --text: #e6e9ee; }
body { background: var(--bg); color: var(--text); }`;

  async function background(theme: Theme | null, colorScheme: 'light' | 'dark', css: string): Promise<{ applied: string | null; background: string }> {
    const context = await browser.newContext({ viewport: { width: VIEWPORT.width, height: VIEWPORT.height }, colorScheme });
    const tab = await context.newPage();
    await tab.setContent(doc('<h1>A page</h1><p>text</p>', css, theme));
    const check = await checkPage(tab, VIEWPORT, theme ?? 'light', { axe: false, present: [] });
    await context.close();
    return { applied: check.themeApplied, background: check.background };
  }

  it('misses the dark palette dead for the page that follows the system', async () => {
    const unpinned = await background(null, 'dark', PALETTE_SYSTEM_DEAD);
    // What the operator on a dark system sees: the light page.
    expect(unpinned.background).toBe('rgb(246, 247, 249)');
    const rows: Themed[] = [];
    for (const theme of THEMES) {
      const seen = await background(theme, theme === 'dark' ? 'light' : 'dark', PALETTE_SYSTEM_DEAD);
      rows.push({ scenario: 'break', state: 'page', viewport: VIEWPORT.name, theme, themeApplied: seen.applied, background: seen.background });
    }
    expect(themeMisses(rows)).not.toEqual([]);
  }, 60_000);
});
