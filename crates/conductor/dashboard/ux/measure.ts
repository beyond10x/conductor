// Drives every scenario state by state at each viewport in both themes and reads the contract in
// each state (L2), and plays the screen-only user and the keyboard-only user on the scenarios that
// have a completion state (L3). Hermetic: the page talks only to the loopback server of serve.ts
// and to its store fixture; any other request is refused and counted.
import { chromium, type Browser, type BrowserContext, type Page } from 'playwright';
import { mkdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  GATING,
  NOW,
  THEMES,
  THEME_KEY,
  VIEWPORTS,
  pagePath,
  scenarios,
  type Scenario,
  type State,
  type Step,
  type Theme,
  type Viewport,
} from './adapter.ts';
import { checkPage, type Check } from './contract.ts';
import { FixtureStore } from './fixtures.ts';
import type { Metrics } from './ratchet.ts';
import { startServer } from './serve.ts';

export interface Row extends Check {
  scenario: string;
  state: string;
  network: string[];
  unanswered: string[];
}

export interface Play {
  completed: boolean;
  falseComplete: boolean;
  actions: number;
  looks: number;
  glances: number[];
  log: string[];
}

export interface Keyboard {
  completed: boolean;
  tabs: number;
}

export interface Measurement {
  rows: Row[];
  plays: Record<string, Record<string, { theme: Theme; play: Play; keyboard: Keyboard }[]>>;
  shots: { moments: Shot[]; worst: Shot[] } | null;
  scenarios: Scenario[];
}

export interface Shot {
  file: string;
  scenario: string;
  state: string;
  viewport: string;
  theme: Theme;
  score?: number;
}

interface Opened {
  context: BrowserContext;
  page: Page;
  store: FixtureStore;
  network: string[];
}

/** A context at `viewport` in `theme`, its instance API answered by a store in `state`. */
async function open(browser: Browser, url: string, viewport: Viewport, theme: Theme, state: State): Promise<Opened> {
  const context = await browser.newContext({
    viewport: { width: viewport.width, height: viewport.height },
    colorScheme: theme,
    reducedMotion: 'reduce',
  });
  await context.addInitScript(
    ([key, value]) => {
      try {
        localStorage.setItem(key, value);
      } catch {
        // A page without storage shows the system theme, which the colour scheme sets.
      }
    },
    [THEME_KEY, theme] as [string, string],
  );
  const store = new FixtureStore(state.store);
  const network: string[] = [];
  await context.route('**/*', async (route) => {
    const request = route.request();
    const target = new URL(request.url());
    if (target.origin !== url) {
      network.push(`${request.method()} ${target.origin}${target.pathname}`);
      await route.abort('blockedbyclient');
      return;
    }
    if (!target.pathname.startsWith('/v1/')) {
      await route.continue();
      return;
    }
    const posted = request.postData();
    const answer = store.answer(request.method(), target.pathname + target.search, posted === null ? null : JSON.parse(posted));
    if (answer.hold === true) {
      return;
    }
    await route.fulfill(
      answer.body !== undefined
        ? { status: answer.status, json: answer.body }
        : { status: answer.status, contentType: 'text/plain', body: answer.text ?? '' },
    );
  });
  const page = await context.newPage();
  await page.clock.setFixedTime(new Date(NOW));
  return { context, page, store, network };
}

async function settle(page: Page, held: boolean): Promise<void> {
  await page.waitForLoadState('load');
  await page.evaluate(() => document.fonts.ready);
  if (!held) {
    await page.waitForLoadState('networkidle', { timeout: 5000 }).catch(() => undefined);
  }
  await page.waitForTimeout(held ? 600 : 250);
}

/** Where a control is: 1 in view and uncovered, 3 not in view. */
async function glance(page: Page, step: Step): Promise<{ found: boolean; glance: number }> {
  const scope = step.within === undefined ? page : page.locator(step.within);
  const control = scope.getByRole(step.role, { name: step.name }).first();
  if ((await control.count()) === 0 || !(await control.isVisible())) {
    return { found: false, glance: 3 };
  }
  const inView = await control.evaluate((el) => {
    const box = el.getBoundingClientRect();
    const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
    return (
      box.top >= 0 && box.left >= 0 && box.bottom <= innerHeight && box.right <= innerWidth && hit !== null && (el.contains(hit) || hit.contains(el))
    );
  });
  return { found: true, glance: inView ? 1 : 3 };
}

/** Whether the scenario's completion state is on screen: its element in view, with its text. */
async function complete(page: Page, scenario: Scenario): Promise<boolean> {
  const completion = scenario.completion;
  if (completion === undefined) {
    return false;
  }
  return page.evaluate(
    ([selector, text]) => {
      const el = document.querySelector(selector);
      if (el === null || !(el.textContent ?? '').includes(text)) {
        return false;
      }
      const box = el.getBoundingClientRect();
      return box.height > 0 && box.top >= 0 && box.bottom <= innerHeight;
    },
    [completion.selector, completion.text] as [string, string],
  );
}

/** The screen-only user: each step through the visible control its role and name find. */
async function play(page: Page, scenario: Scenario, steps: Step[]): Promise<Play> {
  const out: Play = { completed: false, falseComplete: await complete(page, scenario), actions: 0, looks: 0, glances: [], log: [] };
  for (const step of steps) {
    const seen = await glance(page, step);
    out.glances.push(seen.glance);
    if (!seen.found) {
      out.log.push(`no ${step.role} "${step.name}"`);
      return out;
    }
    const scope = step.within === undefined ? page : page.locator(step.within);
    const control = scope.getByRole(step.role, { name: step.name }).first();
    if (seen.glance > 1) {
      out.looks++;
      await control.scrollIntoViewIfNeeded();
    }
    await control.click();
    out.actions++;
    out.log.push(`${step.role} "${step.name}"`);
    await page.waitForLoadState('networkidle', { timeout: 5000 }).catch(() => undefined);
    await page.waitForTimeout(150);
  }
  await page.waitForTimeout(250);
  out.completed = await complete(page, scenario);
  return out;
}

/** The keyboard-only user: Tab to each step's control and press Enter on it. */
async function keyboard(page: Page, scenario: Scenario, steps: Step[]): Promise<Keyboard> {
  let tabs = 0;
  for (const step of steps) {
    let reached = false;
    while (!reached && tabs < 200) {
      await page.keyboard.press('Tab');
      tabs++;
      reached = await page.evaluate(
        ([within, name]) => {
          const el = document.activeElement;
          if (el === null || el === document.body || (within !== '' && el.closest(within) === null)) {
            return false;
          }
          const label = (el.getAttribute('aria-label') ?? (el as HTMLElement).innerText ?? '').trim();
          return label.startsWith(name);
        },
        [step.within ?? '', step.name] as [string, string],
      );
    }
    if (!reached) {
      return { completed: false, tabs };
    }
    await page.keyboard.press('Enter');
    await page.waitForLoadState('networkidle', { timeout: 5000 }).catch(() => undefined);
    await page.waitForTimeout(150);
  }
  await page.waitForTimeout(250);
  return { completed: await complete(page, scenario), tabs };
}

async function pool<T, R>(items: T[], size: number, run: (item: T) => Promise<R>): Promise<R[]> {
  const results: R[] = new Array(items.length);
  let next = 0;
  await Promise.all(
    Array.from({ length: size }, async () => {
      while (next < items.length) {
        const at = next++;
        results[at] = await run(items[at] as T);
      }
    }),
  );
  return results;
}

function shotName(scenario: string, state: string, viewport: string, theme: Theme): string {
  return `${scenario}-${state}-${viewport}-${theme}.png`;
}

/**
 * Every scenario, every state, at the gating viewports (`gatingOnly`) or all of them, in both
 * themes; with `shotsDir`, a shot of each scenario's last state at 1280x800 and of the worst cases.
 */
export async function measureAll({ gatingOnly = false, shotsDir = null }: { gatingOnly?: boolean; shotsDir?: string | null } = {}): Promise<Measurement> {
  const { server, url } = await startServer();
  const browser = await chromium.launch();
  const all = scenarios();
  const viewports = gatingOnly ? GATING : VIEWPORTS;
  const rows: Row[] = [];
  const plays: Measurement['plays'] = {};
  const jobs = all.flatMap((scenario) => viewports.flatMap((viewport) => THEMES.map((theme) => ({ scenario, viewport, theme }))));
  const moments: Shot[] = [];
  if (shotsDir !== null) {
    mkdirSync(shotsDir, { recursive: true });
  }
  try {
    await pool(jobs, 4, async ({ scenario, viewport, theme }) => {
      let last: Opened | null = null;
      for (const state of scenario.states) {
        let opened: Opened;
        if (state.act !== undefined && last !== null) {
          opened = last;
          const played = await play(opened.page, scenario, state.act);
          if (viewport.gating) {
            const fresh = await open(browser, url, viewport, theme, state);
            await fresh.page.goto(url + pagePath(scenario));
            await settle(fresh.page, false);
            const keys = await keyboard(fresh.page, scenario, state.act);
            await fresh.context.close();
            ((plays[scenario.name] ??= {})[viewport.name] ??= []).push({ theme, play: played, keyboard: keys });
          }
        } else {
          if (last !== null) {
            await last.context.close();
          }
          opened = await open(browser, url, viewport, theme, state);
          await opened.page.goto(url + pagePath(scenario));
          await settle(opened.page, state.store === 'loading');
        }
        last = opened;
        const check = await checkPage(opened.page, viewport, theme, { axe: viewport.gating });
        rows.push({ ...check, scenario: scenario.name, state: state.name, network: [...opened.network], unanswered: [...opened.store.unanswered] });
        const keyState = state.act !== undefined || (state.store === 'full' && scenario.states.at(-1) === state);
        if (shotsDir !== null && viewport.name === '1280x800' && keyState) {
          const file = shotName(scenario.name, state.name, viewport.name, theme);
          await opened.page.waitForTimeout(750);
          await opened.page.screenshot({ path: join(shotsDir, file) });
          moments.push({ file, scenario: scenario.name, state: state.name, viewport: viewport.name, theme });
        }
      }
      if (last !== null) {
        await last.context.close();
      }
    });
    rows.sort((a, b) => `${a.scenario} ${a.viewport} ${a.theme} ${a.state}`.localeCompare(`${b.scenario} ${b.viewport} ${b.theme} ${b.state}`));
    let shots: Measurement['shots'] = null;
    if (shotsDir !== null) {
      shots = { moments: moments.sort((a, b) => a.file.localeCompare(b.file)), worst: await worstShots(browser, url, shotsDir, rows, all) };
      await contactSheets(browser, shotsDir, shots.moments);
    }
    return { rows, plays, shots, scenarios: all };
  } finally {
    await browser.close();
    server.close();
  }
}

function score(row: Row): number {
  return row.violations.length * 2 + (row.axe ?? []).reduce((sum, finding) => sum + finding.nodes, 0) + (row.hscroll ? 5 : 0) + row.overflow.length;
}

async function worstShots(browser: Browser, url: string, dir: string, rows: Row[], all: Scenario[]): Promise<Shot[]> {
  const worst = rows
    .filter((row) => GATING.some((viewport) => viewport.name === row.viewport) && score(row) > 0)
    .sort((a, b) => score(b) - score(a))
    .slice(0, 4);
  const out: Shot[] = [];
  for (const row of worst) {
    const scenario = all.find((one) => one.name === row.scenario) as Scenario;
    const viewport = VIEWPORTS.find((one) => one.name === row.viewport) as Viewport;
    const at = scenario.states.findIndex((state) => state.name === row.state);
    const state = scenario.states[at] as State;
    const base = state.act !== undefined ? (scenario.states[at - 1] as State) : state;
    const opened = await open(browser, url, viewport, row.theme, base);
    await opened.page.goto(url + pagePath(scenario));
    await settle(opened.page, base.store === 'loading');
    if (state.act !== undefined) {
      await play(opened.page, scenario, state.act);
    }
    await opened.page.waitForTimeout(750);
    const file = `worst-${shotName(row.scenario, row.state, row.viewport, row.theme)}`;
    await opened.page.screenshot({ path: join(dir, file) });
    out.push({ file, scenario: row.scenario, state: row.state, viewport: row.viewport, theme: row.theme, score: score(row) });
    await opened.context.close();
  }
  return out;
}

async function contactSheets(browser: Browser, dir: string, moments: Shot[]): Promise<void> {
  for (const theme of THEMES) {
    const frames = moments.filter((moment) => moment.theme === theme);
    const html = `<body style="margin:0;background:#888;font:12px sans-serif"><div style="display:grid;grid-template-columns:repeat(3,1fr);gap:6px;padding:6px">${frames
      .map(
        (moment) =>
          `<figure style="margin:0"><img style="width:100%;border:1px solid #333" src="data:image/png;base64,${readFileSync(join(dir, moment.file)).toString('base64')}"><figcaption>${moment.scenario} ${moment.state} ${moment.viewport}</figcaption></figure>`,
      )
      .join('')}</div></body>`;
    const context = await browser.newContext({ viewport: { width: 1800, height: 600 } });
    const page = await context.newPage();
    await page.setContent(html);
    await page.screenshot({ path: join(dir, `contact-sheet-${theme}.png`), fullPage: true });
    await context.close();
  }
}

/** The gated values, by scenario: counts over its states and both themes, per gating viewport. */
export function flatten(measured: Measurement): Metrics {
  const metrics: Metrics = {};
  for (const scenario of measured.scenarios) {
    const out: Metrics[string] = {};
    metrics[scenario.name] = out;
    for (const viewport of GATING) {
      const rows = measured.rows.filter((row) => row.scenario === scenario.name && row.viewport === viewport.name);
      const put = (key: string, value: number, tolerance = 0) => {
        out[`${key}@${viewport.name}`] = { value, tolerance };
      };
      const count = (test: (row: Row) => boolean) => rows.filter(test).length;
      put('hscroll_states', count((row) => row.hscroll));
      put('overflow_states', count((row) => row.overflow.length > 0));
      put('always_visible_violations', rows.reduce((sum, row) => sum + row.violations.length, 0));
      put('axe_serious_critical_nodes', rows.reduce((sum, row) => sum + (row.axe ?? []).reduce((inner, finding) => inner + finding.nodes, 0), 0));
      put('axe_rule_cases', rows.reduce((sum, row) => sum + (row.axe?.length ?? 0), 0));
      put('theme_not_applied', count((row) => row.themeApplied !== row.theme));
      put('truncated_states', count((row) => row.truncated.length > 0));
      put('reduced_motion_violations', rows.reduce((sum, row) => sum + row.motion, 0));
      put('network_requests', rows.reduce((sum, row) => sum + row.network.length, 0));
      put('unanswered_requests', rows.reduce((sum, row) => sum + row.unanswered.length, 0));
      if (scenario.completion !== undefined) {
        const runs = measured.plays[scenario.name]?.[viewport.name] ?? [];
        put('completion_miss', THEMES.length - runs.filter((run) => run.play.completed).length);
        put('false_complete', runs.filter((run) => run.play.falseComplete).length);
        put('over_budget', runs.filter((run) => run.play.actions > scenario.expectations.maxActions).length);
        // Looks and glances turn on where the fold falls, which moves with the host's fonts; a round
        // reports them (round.json `plays`) and the gate holds only what the DOM decides.
        put('screen_only.actions', runs.reduce((sum, run) => sum + run.play.actions, 0));
        put('keyboard.tabs', runs.reduce((sum, run) => sum + run.keyboard.tabs, 0), 0.05);
        put('keyboard.miss', THEMES.length - runs.filter((run) => run.keyboard.completed).length);
      }
    }
  }
  return metrics;
}

/** The values reported and not gated, per measured-only viewport. */
export function measuredOnly(measured: Measurement): Record<string, Record<string, number>> {
  const out: Record<string, Record<string, number>> = {};
  for (const viewport of VIEWPORTS.filter((one) => !one.gating)) {
    const rows = measured.rows.filter((row) => row.viewport === viewport.name);
    out[viewport.name] = {
      hscroll_states: rows.filter((row) => row.hscroll).length,
      overflow_states: rows.filter((row) => row.overflow.length > 0).length,
      always_visible_violations: rows.reduce((sum, row) => sum + row.violations.length, 0),
      truncated_states: rows.filter((row) => row.truncated.length > 0).length,
      scrolling_block_states: rows.filter((row) => row.innerScroll > 0).length,
      small_text_states: rows.filter((row) => row.smallText > 0).length,
    };
  }
  return out;
}

