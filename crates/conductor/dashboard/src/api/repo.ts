// What the repository page (`#/repo/<name>`, story:dashboard-repo-view) reads, and the pure
// selections it makes. The shapes are what the server answers:
// - `/api/waiting-on`, `/api/sessions?repository=`, `/api/blockers`, `/api/specifications`:
//   crates/conductor/src/dashboard/repo.rs;
// - `/api/pull-requests`, `/api/workflow-runs`, `/api/releases`, `/api/repositories` and
//   `/api/events` (story:dashboard-overview): the snapshot views' fields, `{snapshot, rows}`;
// - `/data.json`: crates/conductor/src/dashboard.rs.
// The fixtures under src/views/__tests__/fixtures/ carry these keys, and
// crates/conductor/tests/dashboard_repo.rs holds them to the rows the server answers.

import type { DashboardData, Session } from '../types';

/** The snapshot an endpoint read: the newest complete one, or none. */
export interface SnapshotRef {
  id: string;
  started_at: string;
}

/** An answer read from the newest complete snapshot. */
export interface FromSnapshot<Row> {
  snapshot: SnapshotRef | null;
  rows: Row[];
}

/** One cause a repository waits on. `on` names the repository an `another-repository` waits for. */
export interface Cause {
  cause: 'operator' | 'ci' | 'build-slot' | 'another-repository' | 'usage-limit' | 'silent';
  detail: string;
  on?: string;
}

export interface WaitingRow {
  repository: string;
  causes: Cause[];
}

export interface PullRequestRow {
  observation_id: string;
  snapshot_id: string;
  repository: string;
  number: number;
  title: string;
  draft: boolean;
  mergeability: string;
  failing_checks: number;
  opened_at: string;
  updated_at: string;
  url: string | null;
}

export interface WorkflowRunRow {
  observation_id: string;
  snapshot_id: string;
  repository: string;
  workflow: string;
  conclusion: string;
  run_at: string;
}

export interface ReleaseRow {
  observation_id: string;
  snapshot_id: string;
  repository: string;
  tag: string;
  published_at: string;
  url: string;
}

/** The fields of a repository row the page reads. */
export interface RepositoryRow {
  repository: string;
  unreleased_commits: number | null;
  latest_release: string | null;
  latest_release_at: string | null;
}

export interface BlockerRow {
  observation_id: string;
  snapshot_id: string;
  repository: string;
  reference: string;
  blocker_kind: string;
  title: string;
}

export interface SpecificationRow {
  observation_id: string;
  snapshot_id: string;
  repository: string;
  presence: string;
  path: string | null;
  format: string | null;
  required_ess: string | null;
  validation: string;
  validation_refusals: number;
  scenarios: number | null;
  synthesis_refusals: number | null;
  conformance_status: string | null;
}

export interface SpendToday {
  calls: number;
  input: number;
  output: number;
  cache_read: number;
  cache_write: number;
}

export interface RepoSession {
  name: string | null;
  repository: string;
  state: string | null;
  started_at: string | null;
  dispatch: { id: string; brief: string } | null;
  transcript: string | null;
  spend_today: SpendToday | null;
}

export interface EventRow {
  at: string;
  repository: string | null;
  kind: 'dispatch' | 'report' | 'decision' | 'release' | 'merge';
  text: string;
  link?: string | null;
}

/** Everything the repository page shows of one repository, each part already selected. */
export interface RepoView {
  name: string;
  waiting: Cause[];
  working: Session | null;
  main: WorkflowRunRow | null;
  release: { latest: ReleaseRow | null; since: number | null };
  pullRequests: PullRequestRow[];
  blockers: Map<string, BlockerRow[]>;
  specification: SpecificationRow | null;
  sessions: RepoSession[];
  events: EventRow[];
  /** Each endpoint that could not be read, with why. */
  errors: string[];
}

/** 00:00 UTC of the day `now` is in, as RFC 3339: "today" in every zone the browser runs in. */
export function todaySince(now: Date): string {
  return `${now.toISOString().slice(0, 10)}T00:00:00Z`;
}

/** The causes `/api/waiting-on` answers for `name`. */
export function waitingFor(rows: WaitingRow[], name: string): Cause[] {
  return rows.find((row) => row.repository === name)?.causes ?? [];
}

/** The repositories `/api/waiting-on` answers at least one cause for: the overview's "waiting". */
export function waitingRepositories(rows: WaitingRow[]): Set<string> {
  return new Set(rows.filter((row) => row.causes.length > 0).map((row) => row.repository));
}

/** Of `names`, those the overview's filter "waiting" shows: each with a cause, in their order. */
export function filterWaiting(names: string[], rows: WaitingRow[]): string[] {
  const waiting = waitingRepositories(rows);
  return names.filter((name) => waiting.has(name));
}

/**
 * The controller session of `name` in `/data.json`: the session whose working directory is its
 * checkout, `<checkouts>/<name>`. Its dispatch is the newest to it, with its newest report.
 */
export function workingOn(data: DashboardData | null, name: string): Session | null {
  if (data === null) {
    return null;
  }
  const checkout = `${data.reading.checkouts.replace(/\/+$/, '')}/${name}`;
  return data.sessions.find((session) => session.cwd === checkout && !session.exited) ?? null;
}

/** The newest of `rows` by `at`; none when there are none. */
function newest<Row>(rows: Row[], at: (row: Row) => string): Row | null {
  let best: Row | null = null;
  for (const row of rows) {
    if (best === null || Date.parse(at(row)) > Date.parse(at(best))) {
      best = row;
    }
  }
  return best;
}

/** The newest `main` workflow run of `name`. */
export function mainCi(rows: WorkflowRunRow[], name: string): WorkflowRunRow | null {
  return newest(
    rows.filter((row) => row.repository === name),
    (row) => row.run_at,
  );
}

/** The newest release of `name`, and the commits on `main` since it. */
export function releaseOf(
  releases: ReleaseRow[],
  repositories: RepositoryRow[],
  name: string,
): { latest: ReleaseRow | null; since: number | null } {
  const latest = newest(
    releases.filter((row) => row.repository === name),
    (row) => row.published_at,
  );
  const since = repositories.find((row) => row.repository === name)?.unreleased_commits ?? null;
  return { latest, since };
}

/** The open pull and merge requests of `name`, newest first. */
export function pullRequestsOf(rows: PullRequestRow[], name: string): PullRequestRow[] {
  return rows
    .filter((row) => row.repository === name)
    .sort((a, b) => Date.parse(b.opened_at) - Date.parse(a.opened_at));
}

/**
 * A pull request's pipeline as the page names it: `failing` with a failing check, else `passing`.
 * The snapshot records failing checks only, so a pipeline still running reads `passing`.
 */
export function pipeline(row: PullRequestRow): 'failing' | 'passing' {
  return row.failing_checks > 0 ? 'failing' : 'passing';
}

/** The open blockers of `name`, by kind, kinds in name order. */
export function blockersByKind(rows: BlockerRow[], name: string): Map<string, BlockerRow[]> {
  const kinds = new Map<string, BlockerRow[]>();
  for (const row of rows.filter((row) => row.repository === name)) {
    const list = kinds.get(row.blocker_kind) ?? [];
    list.push(row);
    kinds.set(row.blocker_kind, list);
  }
  return new Map([...kinds.entries()].sort(([a], [b]) => a.localeCompare(b)));
}

/** The specification of `name`. */
export function specificationOf(rows: SpecificationRow[], name: string): SpecificationRow | null {
  return rows.find((row) => row.repository === name) ?? null;
}

/** `rows` newest first. */
export function newestFirst(rows: EventRow[]): EventRow[] {
  return [...rows].sort((a, b) => Date.parse(b.at) - Date.parse(a.at));
}

/** The time of day of an RFC 3339 instant in UTC, `HH:MMZ`. */
export function clock(at: string): string {
  const parsed = new Date(at);
  return Number.isNaN(parsed.getTime()) ? at : `${parsed.toISOString().slice(11, 16)}Z`;
}

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

/** Reads every endpoint the page shows for `name`, at `now`, through `fetcher`. */
export async function loadRepo(
  name: string,
  now: Date = new Date(),
  fetcher: Fetch = (input, init) => fetch(input, init),
): Promise<RepoView> {
  const errors: string[] = [];
  const repository = encodeURIComponent(name);
  async function read<T>(path: string, empty: T): Promise<T> {
    try {
      const answer = await fetcher(path, { cache: 'no-store' });
      if (!answer.ok) {
        errors.push(`could not read ${path}: ${answer.status}`);
        return empty;
      }
      return (await answer.json()) as T;
    } catch (error) {
      errors.push(`could not read ${path}: ${String(error)}`);
      return empty;
    }
  }
  const none = <Row>(): FromSnapshot<Row> => ({ snapshot: null, rows: [] });
  const [
    data,
    waiting,
    pulls,
    runs,
    releases,
    repositories,
    blockers,
    specifications,
    sessions,
    events,
  ] = await Promise.all([
    read<DashboardData | null>('/data.json', null),
    read<{ rows: WaitingRow[] }>('/api/waiting-on', { rows: [] }),
    read<FromSnapshot<PullRequestRow>>('/api/pull-requests', none()),
    read<FromSnapshot<WorkflowRunRow>>('/api/workflow-runs', none()),
    read<FromSnapshot<ReleaseRow>>('/api/releases', none()),
    read<FromSnapshot<RepositoryRow>>('/api/repositories', none()),
    read<FromSnapshot<BlockerRow>>('/api/blockers', none()),
    read<FromSnapshot<SpecificationRow>>('/api/specifications', none()),
    read<{ rows: RepoSession[] }>(`/api/sessions?repository=${repository}`, { rows: [] }),
    read<{ rows: EventRow[] }>(
      `/api/events?since=${encodeURIComponent(todaySince(now))}&repository=${repository}`,
      { rows: [] },
    ),
  ]);
  return {
    name,
    waiting: waitingFor(waiting.rows, name),
    working: workingOn(data, name),
    main: mainCi(runs.rows, name),
    release: releaseOf(releases.rows, repositories.rows, name),
    pullRequests: pullRequestsOf(pulls.rows, name),
    blockers: blockersByKind(blockers.rows, name),
    specification: specificationOf(specifications.rows, name),
    sessions: sessions.rows,
    events: newestFirst(events.rows.filter((row) => row.repository === name)),
    errors,
  };
}
