// What the overview (`#/`, story:dashboard-overview) reads, and the pure selections it makes.
// The shapes are what the server answers (crates/conductor/src/dashboard/overview.rs):
// - `/api/board`, `/api/repositories`, `/api/pull-requests`, `/api/workflow-runs`,
//   `/api/releases`: `{snapshot, rows}` of the newest complete snapshot, the snapshot views' fields;
//   the board's deviations are computed by the server, the one place `STATUS.md`'s are, and are
//   never recomputed here;
// - `/api/events?since=<rfc3339>`: `{rows}`, the day's events;
// - `/api/reviews`: `{snapshot, rows}`, the pull requests that wait for review, oldest first, as
//   `STATUS.md`'s "Waiting for review" lists them (story:review-limit);
// - `/api/unanswered`: `{count, oldest}`, the requests escalated as class O or H and not answered,
//   and the one raised first with its age, as `conductor status` counts them
//   (story:decisions-notify);
// - `/data.json`: crates/conductor/src/dashboard.rs.
// The fixtures src/views/__tests__/fixtures/overview-*.json carry these keys, and
// crates/conductor/tests/dashboard_overview.rs holds them to the ones the server answers.

import { gib, tokens } from '../format';
import type { DashboardData, Session } from '../types';

/** The snapshot an endpoint read: the newest complete one. */
export interface SnapshotRef {
  id: string;
  started_at: string;
}

/** An answer read from the newest complete snapshot; `snapshot` is null when there is none. */
export interface FromSnapshot<Row> {
  snapshot: SnapshotRef | null;
  rows: Row[];
}

/** One repository that is not archived, as the board compares it. */
export interface BoardRow {
  repository: string;
  /** In the order `STATUS.md` lists them: `red main`, `failing PR`, `no checkout`, … */
  deviations: string[];
  open_prs: number;
  failing_prs: number;
  non_green_workflows: number;
  open_blockers: number;
  specifications_present: number;
}

export interface RepositoryRow {
  observation_id: string;
  snapshot_id: string;
  repository: string;
  visibility: string;
  archived: boolean;
  local_checkout: boolean;
  main_head: string;
  main_committed_at: string;
  main_commits_7d: number;
  real_commits_7d: number;
  behind_main: number;
  dirty_files: number;
  worktrees: number;
  open_issues: number;
  oldest_open_issue_at: string | null;
  latest_release: string | null;
  latest_release_at: string | null;
  unreleased_commits: number | null;
  planning_store_version: string | null;
  in_catalog: boolean | null;
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

/** One pull request that waits for a person's review: open, not draft, no failing check. */
export interface ReviewRow {
  repository: string;
  number: number;
  title: string;
  opened_at: string;
  url: string | null;
}

export type EventKind = 'dispatch' | 'report' | 'decision' | 'release' | 'merge';

/** One event of the day. */
export interface EventRow {
  at: string;
  repository: string | null;
  kind: EventKind;
  text: string;
  link: string | null;
}

/** The oldest unanswered operator item: a request escalated as class O or H. */
export interface UnansweredItem {
  request_id: string;
  class: string;
  raised_at: string;
  /** How long ago it was raised when the server answered; null when its time does not read. */
  age_seconds: number | null;
}

/**
 * `/api/unanswered`: the count of decision requests escalated as class O or H and not answered,
 * and the one raised first (story:decisions-notify).
 */
export interface Unanswered {
  count: number;
  oldest: UnansweredItem | null;
}

/** Everything the overview reads, each endpoint that could not be read named in `errors`. */
export interface OverviewData {
  data: DashboardData | null;
  board: FromSnapshot<BoardRow>;
  repositories: FromSnapshot<RepositoryRow>;
  pullRequests: FromSnapshot<PullRequestRow>;
  workflowRuns: FromSnapshot<WorkflowRunRow>;
  releases: FromSnapshot<ReleaseRow>;
  reviews: FromSnapshot<ReviewRow>;
  /** Null when `/api/unanswered` could not be read. */
  unanswered: Unanswered | null;
  events: EventRow[];
  /** The `since` the events were asked with: 00:00 UTC of the day. */
  since: string;
  errors: string[];
  /** The endpoints of the first read that have not answered yet; their panels say they read. */
  pending: string[];
}

/** The endpoints the overview reads, by path, as `pending` and `errors` name them. */
export const OVERVIEW_PATHS = [
  '/data.json',
  '/api/board',
  '/api/repositories',
  '/api/pull-requests',
  '/api/workflow-runs',
  '/api/releases',
  '/api/events',
  '/api/reviews',
  '/api/unanswered',
] as const;

/** The deviation of a repository whose `main` has a failing workflow, as the board names it. */
export const RED_MAIN = 'red main';

/** A live controller whose last report is older than this many minutes is silent. */
export const SILENT_MINUTES = 30;

/** The `localStorage` key that keeps the repositories "mine" names. */
export const MINE_KEY = 'conductor-dashboard-mine';

/** 00:00 UTC of the day `now` is in, as RFC 3339: "today", pinned to UTC in every zone. */
export function startOfDayUtc(now: Date): string {
  return `${now.toISOString().slice(0, 10)}T00:00:00Z`;
}

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

/**
 * The JSON `path` answers, or the reason it could not be read. `no-cache` keeps the answer and
 * asks again with its `ETag`, which the server answers `304` while nothing changed.
 */
async function read<T>(fetcher: Fetch, path: string): Promise<T | string> {
  try {
    const answer = await fetcher(path, { cache: 'no-cache' });
    if (!answer.ok) {
      return `${path.split('?')[0]}: ${answer.status}`;
    }
    return (await answer.json()) as T;
  } catch (error) {
    return `${path.split('?')[0]}: ${String(error)}`;
  }
}

/** The overview before anything answered: every part empty, every endpoint pending. */
export function emptyOverview(since: string): OverviewData {
  const none = { snapshot: null, rows: [] };
  return {
    data: null,
    board: none,
    repositories: none,
    pullRequests: none,
    workflowRuns: none,
    releases: none,
    reviews: none,
    unanswered: null,
    events: [],
    since,
    errors: [],
    pending: [...OVERVIEW_PATHS],
  };
}

/**
 * Reads every endpoint of the overview at `now`, at once, and gives the view to `onPart` each
 * time one answers, so each panel shows as its endpoint arrives; one that fails leaves its part
 * empty and is named in `errors`. A reload starts from `previous`, whose parts stay until their
 * endpoint answers again. Resolves with the view once every endpoint answered.
 */
export async function loadOverview(
  now: Date,
  fetcher: Fetch = fetch,
  onPart: (view: OverviewData) => void = () => {},
  previous: OverviewData | null = null,
): Promise<OverviewData> {
  const since = startOfDayUtc(now);
  const none = { snapshot: null, rows: [] };
  let view: OverviewData =
    previous === null ? emptyOverview(since) : { ...previous, since, pending: [] };

  async function settle<T>(
    path: (typeof OVERVIEW_PATHS)[number],
    url: string,
    empty: T,
    put: (view: OverviewData, value: T) => OverviewData,
  ): Promise<void> {
    const answer = await read<T>(fetcher, url);
    const errors = view.errors.filter((error) => !error.startsWith(`${path}:`));
    let value = empty;
    if (typeof answer === 'string') {
      errors.push(answer);
    } else {
      value = answer;
    }
    view = {
      ...put(view, value),
      errors,
      pending: view.pending.filter((pending) => pending !== path),
    };
    onPart(view);
  }

  await Promise.all([
    settle<DashboardData | null>('/data.json', '/data.json', null, (v, data) => ({ ...v, data })),
    settle<FromSnapshot<BoardRow>>('/api/board', '/api/board', none, (v, board) => ({
      ...v,
      board,
    })),
    settle<FromSnapshot<RepositoryRow>>('/api/repositories', '/api/repositories', none, (v, repositories) => ({
      ...v,
      repositories,
    })),
    settle<FromSnapshot<PullRequestRow>>('/api/pull-requests', '/api/pull-requests', none, (v, pullRequests) => ({
      ...v,
      pullRequests,
    })),
    settle<FromSnapshot<WorkflowRunRow>>('/api/workflow-runs', '/api/workflow-runs', none, (v, workflowRuns) => ({
      ...v,
      workflowRuns,
    })),
    settle<FromSnapshot<ReleaseRow>>('/api/releases', '/api/releases', none, (v, releases) => ({
      ...v,
      releases,
    })),
    settle<{ rows: EventRow[] }>(
      '/api/events',
      `/api/events?since=${encodeURIComponent(since)}`,
      { rows: [] },
      (v, events) => ({ ...v, events: events.rows }),
    ),
    settle<FromSnapshot<ReviewRow>>('/api/reviews', '/api/reviews', none, (v, reviews) => ({
      ...v,
      reviews,
    })),
    settle<Unanswered | null>('/api/unanswered', '/api/unanswered', null, (v, unanswered) => ({
      ...v,
      unanswered,
    })),
  ]);
  return view;
}

// ---------------------------------------------------------------------------------------------
// Selections
// ---------------------------------------------------------------------------------------------

/** The pull requests that wait for review, oldest first. */
export function waitingForReview(view: OverviewData): ReviewRow[] {
  return [...view.reviews.rows].sort(
    (left, right) => Date.parse(left.opened_at) - Date.parse(right.opened_at),
  );
}

/**
 * How long a pull request opened at `openedAt` had been open at `at`, in its largest whole unit:
 * `2d`, `18h`, `40m`; `openedAt` as written when either does not read.
 */
export function age(openedAt: string, at: string | null): string {
  const open = at === null ? Number.NaN : Date.parse(at) - Date.parse(openedAt);
  if (Number.isNaN(open)) {
    return openedAt;
  }
  const minutes = Math.max(0, Math.floor(open / 60_000));
  if (minutes >= 24 * 60) {
    return `${Math.floor(minutes / (24 * 60))}d`;
  }
  if (minutes >= 60) {
    return `${Math.floor(minutes / 60)}h`;
  }
  return `${minutes}m`;
}

/** A span of `seconds` in its largest whole unit, `2d`, `18h`, `40m`; `?` when unknown. */
export function span(seconds: number | null): string {
  if (seconds === null) {
    return '?';
  }
  const minutes = Math.max(0, Math.floor(seconds / 60));
  if (minutes >= 24 * 60) {
    return `${Math.floor(minutes / (24 * 60))}d`;
  }
  if (minutes >= 60) {
    return `${Math.floor(minutes / 60)}h`;
  }
  return `${minutes}m`;
}

/** The repositories of the board whose `main` is red, in the board's order. */
export function redMain(view: OverviewData): string[] {
  return view.board.rows
    .filter((row) => row.deviations.includes(RED_MAIN))
    .map((row) => row.repository);
}

/** A controller's state on the overview. */
export type ControllerState = 'busy' | 'idle' | 'silent';

/**
 * The live controller sessions of `data`: each controller row of the instance's records that a
 * live session holds (story:sessions-page-coherent), wherever it works.
 */
export function controllers(data: DashboardData): Session[] {
  return data.sessions.filter((session) => session.role === 'controller' && !session.exited);
}

/**
 * A live controller's state: `silent` when its last report is older than [SILENT_MINUTES] at
 * `data.generated_at`, else `busy` when the session list says so, else `idle`.
 */
export function controllerState(session: Session, generatedAt: string): ControllerState {
  const at = session.dispatch?.event_at;
  if (at && Date.parse(generatedAt) - Date.parse(at) > SILENT_MINUTES * 60_000) {
    return 'silent';
  }
  return session.status === 'busy' ? 'busy' : 'idle';
}

/** The live controller session of the repository `name`, by its record's repository. */
export function controllerOf(data: DashboardData | null, name: string): Session | null {
  if (data === null) {
    return null;
  }
  return controllers(data).find((session) => session.repository === name) ?? null;
}

/** One tile of the summary strip. */
export interface Tile {
  label: string;
  value: string;
  sub: string;
  /** The tone the tile's value is shown in. */
  tone: 'amber' | 'red' | 'green' | 'text';
}

/** `count` and the singular or plural of `noun`. */
function counted(count: number, noun: string): string {
  return `${count} ${noun}${count === 1 ? '' : 's'}`;
}

/** A count of tokens as the strip shows it: `950`, `5.8k`, `6.2M`. */
export function compactTokens(count: number): string {
  if (count >= 1_000_000) {
    return `${(count / 1_000_000).toFixed(1)}M`;
  }
  if (count >= 1_000) {
    return `${(count / 1_000).toFixed(1)}k`;
  }
  return String(count);
}

/** The tokens spent today: input and output over every repository's usage row; null unmeasured. */
export function spentToday(data: DashboardData | null): { tokens: number; calls: number } | null {
  if (data === null || data.usage.measured_at === null) {
    return null;
  }
  return data.usage.rows.reduce(
    (sum, row) => ({ tokens: sum.tokens + row.input + row.output, calls: sum.calls + row.calls }),
    { tokens: 0, calls: 0 },
  );
}

/** The summary strip of `view`; a figure an endpoint did not answer is `—`. */
export function summary(view: OverviewData): Tile[] {
  const data = view.data;
  const red = redMain(view);
  const awaiting = data?.awaiting_operator.length ?? 0;
  const live = data === null ? [] : controllers(data);
  const states = live.map((session) => controllerState(session, data!.generated_at));
  const count = (state: ControllerState) => states.filter((one) => one === state).length;
  const snapshot = view.board.snapshot !== null;
  const open = view.board.rows.reduce((sum, row) => sum + row.open_prs, 0);
  const failing = view.board.rows.reduce((sum, row) => sum + row.failing_prs, 0);
  const since = Date.parse(view.since);
  const shipped = view.releases.rows.filter((row) => Date.parse(row.published_at) >= since);
  const merges = view.events.filter((event) => event.kind === 'merge').length;
  const context = data?.watch.context ?? null;
  const spent = spentToday(data);
  const unanswered = view.unanswered;
  const oldest = unanswered?.oldest ?? null;
  return [
    {
      label: 'unanswered O/H',
      value: unanswered === null ? '—' : String(unanswered.count),
      sub:
        oldest === null
          ? unanswered === null
            ? ''
            : 'none'
          : `oldest ${span(oldest.age_seconds)} · ${oldest.request_id}`,
      tone: unanswered === null ? 'text' : unanswered.count > 0 ? 'amber' : 'green',
    },
    {
      // The open items that ask something of the operator, by the rule `unanswered O/H` counts
      // them (story:sessions-page-coherent); red CI has its own tile.
      label: 'needs you',
      value: data === null ? '—' : String(awaiting),
      sub: data === null ? '' : `${counted(awaiting, 'item')} to answer or do`,
      tone: awaiting > 0 ? 'amber' : 'green',
    },
    {
      label: 'red CI on main',
      value: snapshot ? String(red.length) : '—',
      sub: red.length > 0 ? red.join(', ') : 'none',
      tone: red.length > 0 ? 'red' : 'green',
    },
    {
      label: 'controllers',
      value: data === null ? '—' : String(live.length),
      sub: `${count('busy')} busy · ${count('idle')} idle · ${count('silent')} silent`,
      tone: 'text',
    },
    {
      label: 'open PRs / MRs',
      value: snapshot ? String(open) : '—',
      sub: `${failing} failing`,
      tone: failing > 0 ? 'red' : 'text',
    },
    {
      label: 'shipped today',
      value: view.releases.snapshot === null ? '—' : String(shipped.length),
      sub: `${counted(shipped.length, 'release')} · ${counted(merges, 'merge')}`,
      tone: 'green',
    },
    {
      label: 'context',
      value: context === null ? '—' : String(context.length),
      sub: `over ${tokens(data?.context_handover ?? null)} tokens`,
      tone: context !== null && context.length > 0 ? 'amber' : 'text',
    },
    {
      label: 'free disk',
      value: data === null ? '—' : gib(data.disk.free_bytes),
      sub: data === null ? '' : `of ${gib(data.disk.size_bytes)}`,
      tone: 'text',
    },
    {
      label: 'spend today',
      value: spent === null ? '—' : `${compactTokens(spent.tokens)} tokens`,
      sub: spent === null ? 'not measured' : counted(spent.calls, 'call'),
      tone: 'text',
    },
  ];
}

/** One card of "needs you". */
export interface NeedCard {
  /** What kind of card: a decision for the operator, one for their hands, or a red `main`. */
  kind: 'operator' | 'hands' | 'red-ci';
  pill: string;
  /** The repository it is about, if any. */
  repository: string | null;
  text: string;
  /** What to do: an open item's action (`answer: …`, `do: …`), a red `main`'s command. */
  command: string;
  /** The open item's place on the Decisions page; null for a red `main`. */
  link: string | null;
}

/** The workflows of `name` whose latest `main` run failed, by name. */
function failingWorkflows(view: OverviewData, name: string): string[] {
  return view.workflowRuns.rows
    .filter((row) => row.repository === name && row.conclusion === 'Failure')
    .map((row) => row.workflow);
}

/**
 * "Needs you": one card per `awaiting_operator` row (class O for the operator's decision, H for
 * their hands), with the one action it asks and its link on the Decisions page
 * (story:sessions-page-coherent); then one per repository whose
 * `main` is red, with the command that lists its runs in its checkout.
 */
export function needsYou(view: OverviewData): NeedCard[] {
  const cards: NeedCard[] = [];
  for (const row of view.data?.awaiting_operator ?? []) {
    const hands = row.class === 'H';
    cards.push({
      kind: hands ? 'hands' : 'operator',
      pill: `${hands ? 'hands' : 'operator'} · class ${row.class} · ${row.id}`,
      repository: row.repository,
      text: row.question,
      command: row.action,
      link: row.link,
    });
  }
  const checkouts = (view.data?.reading.checkouts ?? '~').replace(/\/+$/, '');
  for (const name of redMain(view)) {
    const workflows = failingWorkflows(view, name);
    const workflow = workflows[0];
    cards.push({
      kind: 'red-ci',
      pill: 'red CI · main',
      repository: name,
      text:
        workflows.length > 0
          ? `${workflows.join(', ')} failed on main`
          : 'a workflow failed on main',
      command:
        `cd ${checkouts}/${name} && gh run list --branch main` +
        (workflow === undefined ? '' : ` --workflow '${workflow.replace(/'/g, "'\\''")}'`) +
        ' --limit 5',
      link: null,
    });
  }
  return cards;
}

/** One row of the repository table. */
export interface RepositoryLine {
  name: string;
  /** The board's deviations; `ok` when there are none. */
  chips: string[];
  red: boolean;
  prs: string;
  main: 'red' | 'not green' | 'green';
  release: string;
  controller: ControllerState | 'none';
  last: string;
}

/** One line per repository of the board, in its order, joined with its repository row and its
 * controller session. */
export function repositoryRows(view: OverviewData): RepositoryLine[] {
  return view.board.rows.map((row) => {
    const repository = view.repositories.rows.find((held) => held.repository === row.repository);
    const session = controllerOf(view.data, row.repository);
    const red = row.deviations.includes(RED_MAIN);
    const release = repository?.latest_release ?? null;
    return {
      name: row.repository,
      chips: row.deviations.length > 0 ? row.deviations : ['ok'],
      red,
      prs: `${row.open_prs} open · ${row.failing_prs} failing`,
      main: red ? 'red' : row.non_green_workflows > 0 ? 'not green' : 'green',
      release:
        release === null
          ? '—'
          : `${release} · +${repository?.unreleased_commits ?? '?'}`,
      controller:
        session === null || view.data === null
          ? 'none'
          : controllerState(session, view.data.generated_at),
      last: session?.dispatch?.event_ago ? `${session.dispatch.event_ago} ago` : '—',
    };
  });
}

/** The table's filters. */
export type Filter = 'all' | 'mine' | 'red';

/** The lines `filter` shows, `mine` naming the repositories "mine" keeps. */
export function filtered(lines: RepositoryLine[], filter: Filter, mine: string[]): RepositoryLine[] {
  switch (filter) {
    case 'mine':
      return lines.filter((line) => mine.includes(line.name));
    case 'red':
      return lines.filter((line) => line.red);
    default:
      return lines;
  }
}

/** The names `?mine=a,b` gives in `search`, trimmed, the empty ones aside. */
function names(text: string): string[] {
  return text
    .split(',')
    .map((name) => name.trim())
    .filter((name) => name !== '');
}

/**
 * The repositories "mine" names: those of `?mine=` in the address's `search`, which are kept in
 * `storage`; without `?mine=`, those `storage` keeps. No identity is read.
 */
export function mineFrom(search: string, storage: Storage | null): string[] {
  const given = new URLSearchParams(search).get('mine');
  if (given !== null) {
    const mine = names(given);
    try {
      storage?.setItem(MINE_KEY, mine.join(','));
    } catch {
      // A browser without storage keeps "mine" for this page only.
    }
    return mine;
  }
  try {
    return names(storage?.getItem(MINE_KEY) ?? '');
  } catch {
    return [];
  }
}

/** The events newest first. */
export function newestFirst(events: EventRow[]): EventRow[] {
  return [...events].sort((left, right) => Date.parse(right.at) - Date.parse(left.at));
}

/** The time of an instant as the timeline shows it, in UTC: `09:10Z`. */
export function clock(at: string): string {
  const instant = new Date(at);
  return Number.isNaN(instant.getTime()) ? at : `${instant.toISOString().slice(11, 16)}Z`;
}
