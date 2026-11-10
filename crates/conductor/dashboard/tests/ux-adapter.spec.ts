// @vitest-environment node
// story:dashboard-ui-loop: the dashboard's adapter of the UI iteration loop (the
// ui-iteration-loop skill, § 1). Its scenarios are hermetic store fixtures that drive each page
// state by state; the gating and measured-only viewports, the themes, the always-visible elements
// and the Decisions page's completion state are the story's; and the gate is part of task check
// and of the CI task check job.
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  ALWAYS_VISIBLE,
  AXE_TAGS,
  INSTANCE,
  THEMES,
  VIEWPORTS,
  scenarios,
} from '../ux/adapter.ts';
import { themeMisses } from '../ux/contract.ts';
import { FixtureStore } from '../ux/fixtures.ts';

const REPO = join(process.cwd(), '..', '..', '..');

describe('the adapter', () => {
  it('gates 1440x900 and 1280x800 and measures 1024x768, 768x1024 and 390x844 only', () => {
    expect(VIEWPORTS.filter((viewport) => viewport.gating).map((viewport) => viewport.name)).toEqual([
      '1440x900',
      '1280x800',
    ]);
    expect(VIEWPORTS.filter((viewport) => !viewport.gating).map((viewport) => viewport.name)).toEqual([
      '1024x768',
      '768x1024',
      '390x844',
    ]);
  });

  it('runs both themes the app ships, with the story axe tags', () => {
    expect(THEMES).toEqual(['light', 'dark']);
    const css = readFileSync(join(process.cwd(), 'src', 'style.css'), 'utf8');
    for (const theme of THEMES) {
      expect(css).toContain(`:root[data-theme='${theme}']`);
    }
    expect(AXE_TAGS).toEqual(['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa']);
  });

  it('keeps the header with the instance selector and the page navigation in view', () => {
    expect(ALWAYS_VISIBLE.map((element) => element.kind)).toEqual([
      'header',
      'instance-selector',
      'navigation',
    ]);
  });

  it('drives the overview, repository, decisions, KPIs and health pages state by state', () => {
    const all = scenarios();
    expect(all.map((scenario) => scenario.name)).toEqual(['decisions', 'health', 'kpis', 'overview', 'repository']);
    for (const scenario of all) {
      expect(scenario.states.map((state) => state.name).slice(0, 4), scenario.name).toEqual([
        'loading',
        'error',
        'empty',
        'full',
      ]);
    }
    const decisions = all.find((scenario) => scenario.name === 'decisions');
    expect(decisions?.states.at(-1)?.name).toBe('answered');
    expect(decisions?.completion).toBeDefined();
  });

  it('names what every state must show, so a blank page is a miss', () => {
    for (const scenario of scenarios()) {
      for (const state of scenario.states) {
        const where = `${scenario.name}/${state.name}`;
        expect(state.present.length, where).toBeGreaterThanOrEqual(2);
        expect(state.present, where).toContainEqual({ selector: 'main h1', count: 1 });
        for (const one of state.present) {
          expect(one.selector, where).toMatch(/^main /);
          expect(one.count, where).toBeGreaterThan(0);
        }
      }
    }
  });
});

describe('the theme check', () => {
  const row = (theme: 'light' | 'dark', background: string, themeApplied: string | null = theme) => ({
    scenario: 'health',
    state: 'full',
    viewport: '1280x800',
    theme,
    themeApplied,
    background,
  });

  it('passes two themes that each show their own page background', () => {
    expect(themeMisses([row('light', 'rgb(255, 255, 255)'), row('dark', 'rgb(15, 18, 22)')])).toEqual([]);
  });

  it('misses both cases of a state whose themes show one background: a palette is dead', () => {
    expect(themeMisses([row('light', 'rgb(255, 255, 255)'), row('dark', 'rgb(255, 255, 255)')])).toHaveLength(2);
  });

  it('misses a page that does not carry the theme, and a case with no other theme to compare', () => {
    expect(themeMisses([row('light', 'rgb(255, 255, 255)'), row('dark', 'rgb(15, 18, 22)', null)]).map((one) => one.theme)).toEqual(['dark']);
    expect(themeMisses([row('dark', 'rgb(15, 18, 22)')])).toHaveLength(1);
  });
});

describe('the store fixtures', () => {
  const instance = `/v1/instances/${INSTANCE}`;

  it('answer every route each page reads from the fixtures, and nothing else', () => {
    const store = new FixtureStore('full');
    for (const route of [
      '/data.json',
      '/board',
      '/repositories',
      '/pull-requests',
      '/workflow-runs',
      '/releases',
      '/events?since=2026-10-13T00:00:00Z',
      '/reviews',
      '/unanswered',
      '/waiting-on',
      '/sessions',
      '/blockers',
      '/specifications',
      '/health',
      '/decisions',
      '/kpis',
      '/series',
      '/series?series=prs_merged&days=30',
    ]) {
      expect(store.answer('GET', `${instance}${route}`, null).status, route).toBe(200);
    }
    expect(store.answer('GET', '/v1/instances', null).status).toBe(200);
    expect(store.answer('GET', `${instance}/no-such-route`, null).status).toBe(404);
    expect(store.unanswered).toEqual([`GET ${instance}/no-such-route`]);
  });

  it('hold every read in the loading state, fail it in the error state and empty its rows', () => {
    expect(new FixtureStore('loading').answer('GET', `${instance}/board`, null)).toEqual({ status: 0, hold: true });
    expect(new FixtureStore('loading').answer('GET', '/v1/instances', null).status).toBe(200);
    expect(new FixtureStore('error').answer('GET', `${instance}/board`, null).status).toBe(503);
    const empty = new FixtureStore('empty');
    expect((empty.answer('GET', `${instance}/board`, null).body as { rows: unknown[] }).rows).toEqual([]);
    expect((empty.answer('GET', `${instance}/decisions`, null).body as { open: unknown[] }).open).toEqual([]);
    expect(empty.answer('GET', `${instance}/kpis`, null).body).toEqual([]);
  });

  it('records an answer as the store does: the request leaves the open list for the history', () => {
    const store = new FixtureStore('full');
    const before = store.answer('GET', `${instance}/decisions`, null).body as { open: { request_id: string }[] };
    expect(before.open.map((open) => open.request_id)).toContain('REQ-release');
    const sent = store.answer('POST', `${instance}/answers`, {
      answers: [{ request_id: 'REQ-release', choice: ['Y'] }],
    });
    expect(sent.body).toEqual({
      results: [{ request_id: 'REQ-release', outcome: 'recorded', decision_id: 'DEC-20261013-90' }],
    });
    const after = store.answer('GET', `${instance}/decisions`, null).body as {
      open: { request_id: string }[];
      history: { id: string | null }[];
    };
    expect(after.open.map((open) => open.request_id)).not.toContain('REQ-release');
    expect(after.history[0]?.id).toBe('DEC-20261013-90');
  });
});

describe('the gate', () => {
  it('is part of task check, after the committed dist/ is held to a fresh build', () => {
    const taskfile = readFileSync(join(REPO, 'Taskfile.yml'), 'utf8');
    const check = taskfile.slice(taskfile.indexOf('\n  check:\n'), taskfile.indexOf('\n  docs-generate:\n'));
    const dist = check.indexOf('git diff --exit-code -- crates/conductor/dashboard/dist');
    const gate = check.indexOf('task: ui:ux-gate');
    expect(dist).toBeGreaterThan(0);
    expect(gate).toBeGreaterThan(dist);
  });

  it('has a Chromium in the CI task check job before task check runs', () => {
    const workflow = readFileSync(join(REPO, '.github', 'workflows', 'check.yml'), 'utf8');
    const install = workflow.indexOf('playwright install --with-deps chromium');
    expect(install).toBeGreaterThan(0);
    expect(workflow.indexOf('run: task check')).toBeGreaterThan(install);
  });

  it('has the main branch in the CI checkout before task check runs, for the worse-than-main check', () => {
    const workflow = readFileSync(join(REPO, '.github', 'workflows', 'check.yml'), 'utf8');
    const fetch = workflow.indexOf('git fetch --no-tags --depth=1 origin +refs/heads/main:refs/remotes/origin/main');
    expect(fetch).toBeGreaterThan(0);
    expect(workflow.indexOf('run: task check')).toBeGreaterThan(fetch);
  });
});
