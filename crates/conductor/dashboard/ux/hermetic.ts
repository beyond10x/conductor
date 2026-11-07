// The hermetic server of the scenarios: every request the page makes is answered here, from the
// fixtures the Vitest specs read (src/fixtures, src/views/__tests__/fixtures), or by the built app
// (dist/, served by `vite preview`). Nothing leaves the host.
import type { Page, Route } from '@playwright/test';
import type { Mode } from './adapter';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';

/** A fixture under src/, read from disk (an ES module imports JSON only with an attribute). */
function json(path: string): unknown {
  return JSON.parse(readFileSync(join(import.meta.dirname, '..', 'src', path), 'utf8'));
}

const data = json('fixtures/data.json');
const board = json('views/__tests__/fixtures/overview-board.json');
const repositories = json('views/__tests__/fixtures/overview-repositories.json');
const pullRequests = json('views/__tests__/fixtures/overview-pull-requests.json');
const workflowRuns = json('views/__tests__/fixtures/overview-workflow-runs.json');
const releases = json('views/__tests__/fixtures/overview-releases.json');
const events = json('views/__tests__/fixtures/overview-events.json');
const reviews = json('views/__tests__/fixtures/overview-reviews.json');
const unanswered = json('views/__tests__/fixtures/overview-unanswered.json');
const waitingOn = json('views/__tests__/fixtures/waiting-on.json');
const sessions = json('views/__tests__/fixtures/sessions.json');
const blockers = json('views/__tests__/fixtures/blockers.json');
const specifications = json('views/__tests__/fixtures/specifications.json');
const health = json('views/__tests__/fixtures/health.json');
const decisions = json('views/__tests__/fixtures/decisions.json') as { open: { request_id: string }[] };
const kpis = json('views/__tests__/fixtures/kpis.json');
const series = json('views/__tests__/fixtures/series.json') as {
  list: { series: string; source: string | null; story?: string }[];
  answers: Record<string, unknown>;
};

/** The other instance of the header's selector; its page is a stub this server answers. */
export const OTHER_INSTANCE = 'http://127.0.0.1:7314/';

/** The token the page is served with, as the conductor binary writes it into index.html. */
const TOKEN = 'ux-hermetic-token';

const INSTANCES = {
  instances: [
    { name: 'alpha', dashboard_port: 7313, url: 'http://127.0.0.1:7313/', current: true },
    { name: 'beta', dashboard_port: 7314, url: OTHER_INSTANCE, current: false },
  ],
  errors: [],
};

function clone<T>(value: T): T {
  return structuredClone(value);
}

function loaded(): Record<string, unknown> {
  return {
    '/data.json': clone(data),
    '/api/board': clone(board),
    '/api/repositories': clone(repositories),
    '/api/pull-requests': clone(pullRequests),
    '/api/workflow-runs': clone(workflowRuns),
    '/api/releases': clone(releases),
    '/api/events': clone(events),
    '/api/reviews': clone(reviews),
    '/api/unanswered': clone(unanswered),
    '/api/waiting-on': clone(waitingOn),
    '/api/sessions': clone(sessions),
    '/api/blockers': clone(blockers),
    '/api/specifications': clone(specifications),
    '/api/health': clone(health),
    '/api/decisions': clone(decisions),
    '/api/kpis': clone(kpis),
  };
}

function empty(): Record<string, unknown> {
  const none = { snapshot: null, rows: [] };
  const nothing = clone(data) as unknown as Record<string, unknown> & {
    usage: { rows: unknown[] };
    disk: { rows: unknown[] };
  };
  nothing.sessions = [];
  nothing.decisions = [];
  nothing.awaiting_operator = [];
  nothing.usage.rows = [];
  nothing.disk.rows = [];
  nothing.errors = [];
  nothing.watch = { limits: [], context: [] };
  return {
    '/data.json': nothing,
    '/api/board': none,
    '/api/repositories': none,
    '/api/pull-requests': none,
    '/api/workflow-runs': none,
    '/api/releases': none,
    '/api/events': { rows: [] },
    '/api/reviews': none,
    '/api/unanswered': { count: 0, oldest: null },
    '/api/waiting-on': { rows: [] },
    '/api/sessions': { rows: [] },
    '/api/blockers': none,
    '/api/specifications': none,
    '/api/decisions': { open: [], history: [], errors: [] },
    '/api/kpis': [],
  };
}

/** `/api/series` as src/views/__tests__/series-fixtures.ts answers it. */
function seriesAnswer(url: URL, mode: Mode): { status: number; body: unknown } {
  const name = url.searchParams.get('series');
  if (name === null) {
    return { status: 200, body: mode === 'empty' ? [] : series.list };
  }
  const info = series.list.find((entry) => entry.series === name);
  if (info === undefined) {
    return { status: 400, body: { error: `unknown series ${name}` } };
  }
  if (info.source === null) {
    return { status: 404, body: { error: 'no source yet', story: info.story } };
  }
  const by = url.searchParams.get('by');
  const answer = (series.answers as Record<string, unknown>)[`${name}|${by ?? ''}`];
  if (answer === undefined) {
    return { status: 400, body: { error: `no fixture for ${name} by ${by ?? 'total'}` } };
  }
  return { status: 200, body: { ...(answer as object), days: Number(url.searchParams.get('days') ?? '30') } };
}

/** What the hermetic server saw: the answers the page sent. */
export interface Served {
  answers: unknown[];
}

/**
 * Answers every request of `page` in `mode`. An answer sent to `/api/answers` is recorded and
 * taken out of the open decisions, as the server does.
 */
export async function serve(page: Page, mode: Mode): Promise<Served> {
  const served: Served = { answers: [] };
  let state = mode === 'empty' ? empty() : loaded();
  await page.route('**/*', async (route: Route) => {
    const request = route.request();
    const url = new URL(request.url());
    if (url.origin === new URL(OTHER_INSTANCE).origin) {
      await route.fulfill({
        contentType: 'text/html',
        body: '<!doctype html><html lang="en"><title>beta</title><main><h1>the beta instance</h1></main></html>',
      });
      return;
    }
    const path = url.pathname;
    if (path === '/' || path === '/index.html') {
      const original = await route.fetch();
      const html = (await original.text()).replace(
        '</head>',
        `<meta name="conductor-token" content="${TOKEN}">\n</head>`,
      );
      await route.fulfill({ response: original, body: html });
      return;
    }
    if (path === '/api/instances') {
      await route.fulfill({ json: INSTANCES });
      return;
    }
    if (path !== '/data.json' && !path.startsWith('/api/')) {
      await route.continue();
      return;
    }
    if (mode === 'loading') {
      // Never answered: the page stays in its reading state.
      return;
    }
    if (mode === 'error') {
      await route.fulfill({ status: 500, body: 'nope' });
      return;
    }
    if (path === '/api/series') {
      const answer = seriesAnswer(url, mode);
      await route.fulfill({ status: answer.status, json: answer.body });
      return;
    }
    if (path === '/api/answers' && request.method() === 'POST') {
      const body = request.postDataJSON() as { answers: { request_id: string }[] };
      served.answers.push(...body.answers);
      const open = state['/api/decisions'] as typeof decisions;
      const ids = new Set(body.answers.map((row) => row.request_id));
      state = {
        ...state,
        '/api/decisions': { ...open, open: open.open.filter((item) => !ids.has(item.request_id)) },
      };
      await route.fulfill({
        json: {
          results: body.answers.map((row, at) => ({
            request_id: row.request_id,
            outcome: 'recorded',
            decision_id: `D-ux-${at + 1}`,
          })),
        },
      });
      return;
    }
    if (path in state) {
      await route.fulfill({ json: state[path] });
      return;
    }
    await route.fulfill({ status: 404, body: 'no fixture' });
  });
  return served;
}
