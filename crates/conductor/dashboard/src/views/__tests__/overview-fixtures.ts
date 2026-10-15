// The overview's fixtures, a fresh copy for each test to change. The `overview-*.json` files carry
// the keys the server answers (crates/conductor/tests/dashboard_overview.rs holds them to it);
// `/data.json` is the app's fixture src/fixtures/data.json (held by crates/conductor/tests/dashboard.rs).
import type { DashboardData } from '../../types';
import type {
  BoardRow,
  EventRow,
  FromSnapshot,
  OverviewData,
  PullRequestRow,
  ReleaseRow,
  RepositoryRow,
  ReviewRow,
  Unanswered,
  WorkflowRunRow,
} from '../../api/overview';
import data from '../../fixtures/data.json';
import board from './fixtures/overview-board.json';
import repositories from './fixtures/overview-repositories.json';
import pullRequests from './fixtures/overview-pull-requests.json';
import workflowRuns from './fixtures/overview-workflow-runs.json';
import releases from './fixtures/overview-releases.json';
import events from './fixtures/overview-events.json';
import reviews from './fixtures/overview-reviews.json';
import unanswered from './fixtures/overview-unanswered.json';

/** The instant every overview test runs at: 09:30 UTC of the fixtures' day. */
export const NOW = new Date('2026-10-13T09:30:00Z');

/** 00:00 UTC of that day: the `since` of "today". */
export const SINCE = '2026-10-13T00:00:00Z';

/** What each endpoint answers in the fixtures, by path. */
export function answers(): Record<string, unknown> {
  return {
    '/data.json': structuredClone(data),
    '/api/board': structuredClone(board),
    '/api/repositories': structuredClone(repositories),
    '/api/pull-requests': structuredClone(pullRequests),
    '/api/workflow-runs': structuredClone(workflowRuns),
    '/api/releases': structuredClone(releases),
    '/api/events': structuredClone(events),
    '/api/reviews': structuredClone(reviews),
    '/api/unanswered': structuredClone(unanswered),
  };
}

/** Everything the overview reads, from the fixtures. */
export function overview(): OverviewData {
  const all = answers();
  return {
    data: all['/data.json'] as DashboardData,
    board: all['/api/board'] as FromSnapshot<BoardRow>,
    repositories: all['/api/repositories'] as FromSnapshot<RepositoryRow>,
    pullRequests: all['/api/pull-requests'] as FromSnapshot<PullRequestRow>,
    workflowRuns: all['/api/workflow-runs'] as FromSnapshot<WorkflowRunRow>,
    releases: all['/api/releases'] as FromSnapshot<ReleaseRow>,
    reviews: all['/api/reviews'] as FromSnapshot<ReviewRow>,
    unanswered: all['/api/unanswered'] as Unanswered,
    events: (all['/api/events'] as { rows: EventRow[] }).rows,
    since: SINCE,
    errors: [],
    pending: [],
  };
}

/** A storage that keeps its items in memory, for "mine". */
export function memoryStorage(items: Record<string, string> = {}): Storage {
  const held = new Map(Object.entries(items));
  return {
    get length() {
      return held.size;
    },
    clear: () => held.clear(),
    getItem: (key) => held.get(key) ?? null,
    key: (index) => [...held.keys()][index] ?? null,
    removeItem: (key) => void held.delete(key),
    setItem: (key, value) => void held.set(key, String(value)),
  };
}
