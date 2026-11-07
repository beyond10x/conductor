// L3, the screen-only user (skill § 7; story:dashboard-ui-loop): deterministic code, no model. From
// the landing page it reaches each task's truth using only what is visible: a control fully inside
// the viewport and not covered at its centre, found by its accessible role and name. A click, a key
// press or a choice in a select is one action; a scroll is a look, not an action.
//
// Glance cost of each control it uses: 1 in the header or the top half of the viewport, 2
// elsewhere in view, 3 not in view (it scrolls one viewport at a time until it is).
// `actions_per_fix` counts the actions from the task's page to its completion (the answer sent,
// the selector changed); a task with no fix records 0.
import { test, expect, type Locator, type Page } from '@playwright/test';
import { NOW, THEME_KEY } from './adapter';
import { serve, OTHER_INSTANCE } from './hermetic';
import { record } from './contract';

class Bot {
  actions = 0;
  looks = 0;
  glances: number[] = [];
  private fixFrom: number | null = null;
  fixActions: number[] = [];

  constructor(readonly page: Page) {}

  /** Finds `target`, scrolling until it is fully in view, and notes the glance. */
  async see(target: Locator, what: string): Promise<Locator> {
    const one = target.first();
    for (let scrolls = 0; scrolls <= 12; scrolls += 1) {
      this.looks += 1;
      await expect(one, `${what} exists`).toBeAttached();
      const place = await one.evaluate((el) => {
        const box = el.getBoundingClientRect();
        const vw = document.documentElement.clientWidth;
        const vh = document.documentElement.clientHeight;
        const inView = box.width > 0 && box.left >= 0 && box.top >= 0 && box.right <= vw && box.bottom <= vh;
        const hit = document.elementFromPoint(box.left + box.width / 2, box.top + box.height / 2);
        const uncovered = hit !== null && (hit === el || el.contains(hit));
        const header = el.closest('header.top') !== null;
        return { inView: inView && uncovered, glance: header || box.bottom <= vh / 2 ? 1 : 2 };
      });
      if (place.inView) {
        this.glances.push(scrolls > 0 ? 3 : place.glance);
        return one;
      }
      await this.page.mouse.wheel(0, (this.page.viewportSize()?.height ?? 800) * 0.8);
      await this.page.evaluate(() => new Promise((done) => requestAnimationFrame(() => done(null))));
    }
    throw new Error(`${what}: never fully in view`);
  }

  /** Clicks the centre of a visible control. */
  async click(target: Locator, what: string): Promise<void> {
    const one = await this.see(target, what);
    const box = (await one.boundingBox())!;
    await this.page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
    this.actions += 1;
  }

  /** Opens a visible select and chooses `label`: two actions. */
  async choose(target: Locator, label: string, what: string): Promise<void> {
    const one = await this.see(target, what);
    await one.focus();
    this.actions += 1;
    await one.selectOption({ label });
    this.actions += 1;
  }

  fixStarts(): void {
    this.fixFrom = this.actions;
  }

  fixEnds(): void {
    this.fixActions.push(this.actions - (this.fixFrom ?? this.actions));
    this.fixFrom = null;
  }

  metrics(): Record<string, number> {
    const sorted = [...this.glances].sort((a, b) => a - b);
    const at = (q: number) => sorted[Math.min(sorted.length - 1, Math.ceil(q * sorted.length) - 1)] ?? 0;
    const median = sorted.length === 0 ? 0 : sorted.length % 2 === 1 ? sorted[(sorted.length - 1) / 2]! : (sorted[sorted.length / 2 - 1]! + sorted[sorted.length / 2]!) / 2;
    return {
      'screen_only.actions': this.actions,
      'screen_only.looks': this.looks,
      'glance_cost.median': median,
      'glance_cost.p95': at(0.95),
      'actions_per_fix.max': Math.max(0, ...this.fixActions),
    };
  }
}

async function land(page: Page, theme: 'light' | 'dark' = 'light') {
  await page.setViewportSize({ width: 1280, height: 800 });
  await page.clock.setFixedTime(NOW);
  await page.emulateMedia({ colorScheme: theme, reducedMotion: 'reduce' });
  await page.addInitScript(([key, value]) => localStorage.setItem(key!, value!), [THEME_KEY, theme]);
  const served = await serve(page, 'loaded');
  await page.goto('/#/');
  await expect(page.locator('main h1')).toBeVisible();
  await expect(page.locator('main .state-loading')).toHaveCount(0);
  return { bot: new Bot(page), served };
}

test('task-answer-decision: answer one pending decision on the Decisions page', async ({ page }) => {
  const { bot, served } = await land(page);
  const nav = page.getByRole('navigation', { name: 'Views' });
  await bot.click(nav.getByRole('link', { name: 'Decisions' }), 'the Decisions link');
  await expect(page.getByRole('heading', { level: 1, name: 'Decisions' })).toBeVisible();
  bot.fixStarts();
  const item = page.getByRole('article').filter({ hasText: 'Release v2.0 today?' });
  await bot.click(item.getByRole('button', { name: /^Yes/ }), 'Yes on the release request');
  await bot.click(page.getByRole('button', { name: 'Send 1 answer' }), 'the send button');
  bot.fixEnds();
  // Completion: the page says the answer was recorded, and the server got it.
  await bot.see(page.getByRole('list', { name: 'What was sent' }).getByText(/REQ-release: recorded/), 'the recorded answer');
  expect(served.answers).toEqual([expect.objectContaining({ request_id: 'REQ-release', choice: ['Y'] })]);
  record('screen-only-task-answer-decision', { kind: 'screen-only', scenario: 'task-answer-decision', metrics: bot.metrics() });
});

test('task-failing-check: open one repository and find its failing check', async ({ page }) => {
  const { bot } = await land(page);
  await bot.click(page.getByRole('main').getByRole('link', { name: 'alpha', exact: true }), 'the alpha repository');
  await expect(page.getByRole('heading', { level: 1 })).toContainText('alpha');
  // Truth: alpha's newest CI run on main failed (overview-workflow-runs.json).
  await bot.see(page.locator('.main-ci').getByText('red', { exact: true }), 'main is red');
  record('screen-only-task-failing-check', { kind: 'screen-only', scenario: 'task-failing-check', metrics: bot.metrics() });
});

test('task-switch-instance: switch the instance with the header selector', async ({ page }) => {
  const { bot } = await land(page);
  bot.fixStarts();
  await bot.choose(page.getByRole('combobox', { name: 'Instance' }), 'beta', 'the instance selector');
  bot.fixEnds();
  await page.waitForURL(`${OTHER_INSTANCE}#/`);
  await expect(page.getByRole('heading', { name: 'the beta instance' })).toBeVisible();
  record('screen-only-task-switch-instance', { kind: 'screen-only', scenario: 'task-switch-instance', metrics: bot.metrics() });
});

test("task-kpi-target: read one KPI's target and whether it is met", async ({ page }) => {
  const { bot } = await land(page);
  const nav = page.getByRole('navigation', { name: 'Views' });
  await bot.click(nav.getByRole('link', { name: 'KPIs' }), 'the KPIs link');
  const k1 = page.getByRole('article', { name: /K1 · Operator touches per merged PR/ });
  // Truth: K1 targets 0.1 or lower and reads 0.245, so it misses its target (kpis.json).
  const target = await bot.see(k1.locator('.target'), "K1's target");
  await expect(target).toContainText('0.1');
  await bot.see(k1.getByText('misses target'), 'whether K1 is met');
  record('screen-only-task-kpi-target', { kind: 'screen-only', scenario: 'task-kpi-target', metrics: bot.metrics() });
});
