// The hermetic store fixtures every UX scenario drives the pages from (story:dashboard-ui-loop). One
// instance's store, answered as the daemon's instance API answers it through
// `/v1/instances/<instance>/<route>`: the same fixtures the Vitest page specs read
// (src/views/__tests__/fixtures/, src/fixtures/data.json; crates/conductor/tests/dashboard*.rs hold
// them to the served shape), in four states:
//
// - `loading`: every instance read is held unanswered;
// - `error`: every instance read is refused, 503, as by a runtime that is not running;
// - `empty`: every list in every answer is empty;
// - `full`: the fixtures as they are.
//
// The store keeps what is written to it: an answer POSTed to `answers` is recorded as the store
// records one, so the request leaves the open list for the history. No answer reads the network,
// a key or a file outside the app.
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { APP_DIR, INSTANCE, NOW, type StoreState } from './adapter.ts';

const VIEWS = join(APP_DIR, 'src', 'views', '__tests__', 'fixtures');

function fixture(name: string): unknown {
  return JSON.parse(readFileSync(join(VIEWS, `${name}.json`), 'utf8'));
}

/** Every route of the instance API the pages read, by its name below the instance. */
function routes(): Record<string, unknown> {
  return {
    'data.json': JSON.parse(readFileSync(join(APP_DIR, 'src', 'fixtures', 'data.json'), 'utf8')),
    board: fixture('overview-board'),
    repositories: fixture('overview-repositories'),
    'pull-requests': fixture('overview-pull-requests'),
    'workflow-runs': fixture('overview-workflow-runs'),
    releases: fixture('overview-releases'),
    events: fixture('overview-events'),
    reviews: fixture('overview-reviews'),
    unanswered: fixture('overview-unanswered'),
    'waiting-on': fixture('waiting-on'),
    sessions: fixture('sessions'),
    blockers: fixture('blockers'),
    specifications: fixture('specifications'),
    health: fixture('health'),
    decisions: fixture('decisions'),
    kpis: fixture('kpis'),
  };
}

/** The daemon's instances; two, so the header shows its instance selector. */
const INSTANCES = [
  { name: INSTANCE, state: 'Running' },
  { name: 'beta', state: 'Running' },
];

export interface Answer {
  status: number;
  body?: unknown;
  text?: string;
  /** The read is never answered (the `loading` state). */
  hold?: boolean;
}

/** `value` with every list in it emptied. */
function emptied(value: unknown): unknown {
  if (Array.isArray(value)) {
    return [];
  }
  if (value !== null && typeof value === 'object') {
    return Object.fromEntries(Object.entries(value).map(([key, inner]) => [key, emptied(inner)]));
  }
  return value;
}

interface Series {
  list: { series: string; source: string | null; story?: string }[];
  answers: Record<string, unknown>;
}

interface Open {
  request_id: string;
  class: string;
  repository: string;
  question: string;
}

interface Decisions {
  open: Open[];
  history: Record<string, unknown>[];
  errors: string[];
}

export class FixtureStore {
  readonly state: StoreState;
  /** Every request no fixture answers, `<METHOD> <path>`: the gate counts them. */
  readonly unanswered: string[] = [];
  private readonly data: Record<string, unknown>;
  private readonly series: Series;
  private recorded = 0;

  constructor(state: StoreState) {
    this.state = state;
    this.data = routes();
    this.series = fixture('series') as Series;
    if (state === 'empty') {
      this.data = emptied(this.data) as Record<string, unknown>;
      this.data.unanswered = { count: 0, oldest: null };
      this.series = { list: [], answers: {} };
    }
  }

  /** What the store answers to `method` on `url` (path and query, below the origin). */
  answer(method: string, url: string, body: unknown): Answer {
    const parsed = new URL(url, 'http://dashboard.invalid');
    if (parsed.pathname === '/v1/instances') {
      return { status: 200, body: INSTANCES };
    }
    const prefix = `/v1/instances/${INSTANCE}/`;
    if (!parsed.pathname.startsWith(prefix)) {
      return this.miss(method, parsed);
    }
    if (this.state === 'loading') {
      return { status: 0, hold: true };
    }
    if (this.state === 'error') {
      return { status: 503, text: `instance ${INSTANCE}: its runtime is not running\n` };
    }
    const route = parsed.pathname.slice(prefix.length);
    if (route === 'answers' && method === 'POST') {
      return { status: 200, body: this.record(body) };
    }
    if (method !== 'GET') {
      return this.miss(method, parsed);
    }
    if (route === 'series') {
      return this.seriesAnswer(parsed.searchParams);
    }
    return route in this.data ? { status: 200, body: structuredClone(this.data[route]) } : this.miss(method, parsed);
  }

  private miss(method: string, parsed: URL): Answer {
    this.unanswered.push(`${method} ${parsed.pathname}`);
    return { status: 404, text: 'no fixture answers this route\n' };
  }

  /** `/series` as story:dashboard-series-api answers it (src/views/__tests__/series-fixtures.ts). */
  private seriesAnswer(query: URLSearchParams): Answer {
    const name = query.get('series');
    if (name === null) {
      return { status: 200, body: structuredClone(this.series.list) };
    }
    const info = this.series.list.find((entry) => entry.series === name);
    if (info === undefined) {
      return { status: 400, body: { error: `unknown series ${name}` } };
    }
    if (info.source === null) {
      return { status: 404, body: { error: 'no source yet', story: info.story } };
    }
    const by = query.get('by');
    const found = this.series.answers[`${name}|${by ?? ''}`];
    if (found === undefined) {
      return { status: 400, body: { error: `no fixture for ${name} by ${by ?? 'total'}` } };
    }
    return { status: 200, body: { ...(structuredClone(found) as object), days: Number(query.get('days') ?? '30') } };
  }

  /** Records each answer of a batch as the store does, row by row. */
  private record(body: unknown): { results: Record<string, unknown>[] } {
    const decisions = this.data.decisions as Decisions;
    const rows = ((body as { answers?: { request_id: string; choice: string[]; words?: string }[] }).answers ?? []);
    const results = rows.map((row) => {
      const at = decisions.open.findIndex((open) => open.request_id === row.request_id);
      if (at < 0) {
        return {
          request_id: row.request_id,
          outcome: 'refused',
          error: 'RequestStateConflict',
          reason: `request "${row.request_id}" is not escalated; nothing was recorded`,
        };
      }
      const [open] = decisions.open.splice(at, 1) as [Open];
      const id = `DEC-20261013-${String(90 + this.recorded++)}`;
      decisions.history.unshift({
        at: NOW,
        event: 'answered',
        id,
        repo: open.repository,
        class: open.class,
        decided_by: 'operator',
        question: open.question,
        choice: row.choice.join(', '),
        reason: row.words ?? null,
      });
      return { request_id: row.request_id, outcome: 'recorded', decision_id: id };
    });
    return { results };
  }
}
