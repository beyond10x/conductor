// What the overview (`#/`, story:dashboard-overview) reads, and the pure selections it makes.
// The shapes are what the server answers (crates/conductor/src/dashboard/overview.rs):
// - `/api/board`, `/api/repositories`, `/api/pull-requests`, `/api/workflow-runs`,
//   `/api/releases`: `{snapshot, rows}` of the newest complete snapshot, the snapshot views' fields;
//   the board's deviations are computed by the server, the one place `STATUS.md`'s are, and are
//   never recomputed here;
// - `/api/events?since=<rfc3339>`: `{rows}`, the day's events;
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

export type EventKind = 'dispatch' | 'report' | 'decision' | 'release' | 'merge';

/** One event of the day. */
export interface EventRow {
  at: string;
  repository: string | null;
  kind: EventKind;
  text: string;
  link: string | null;
}

/** Everything the overview reads, each endpoint that could not be read named in `errors`. */
export interface OverviewData {
  data: DashboardData | null;
  board: FromSnapshot<BoardRow>;
  repositories: FromSnapshot<RepositoryRow>;
  pullRequests: FromSnapshot<PullRequestRow>;
  workflowRuns: FromSnapshot<WorkflowRunRow>;
  releases: FromSnapshot<ReleaseRow>;
  events: EventRow[];
  /** The `since` the events were asked with: 00:00 UTC of the day. */
  since: string;
  errors: string[];
}

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

/** The JSON `path` answers, or the reason it could not be read. */
async function read<T>(fetcher: Fetch, path: string): Promise<T | string> {
  try {
    const answer = await fetcher(path, { cache: 'no-store' });
    if (!answer.ok) {
      return `${path.split('?')[0]}: ${answer.status}`;
    }
    return (await answer.json()) as T;
  } catch (error) {
    return `${path.split('?')[0]}: ${String(error)}`;
  }
}

/** Reads every endpoint of the overview at `now`, at once; one that fails leaves its part empty. */
export async function loadOverview(now: Date, fetcher: Fetch = fetch): Promise<OverviewData> {
  const since = startOfDayUtc(now);
  const none = { snapshot: null, rows: [] };
  const [data, board, repositories, pullRequests, workflowRuns, releases, events] =
    await Promise.all([
      read<DashboardData>(fetcher, '/data.json'),
      read<FromSnapshot<BoardRow>>(fetcher, '/api/board'),
      read<FromSnapshot<RepositoryRow>>(fetcher, '/api/repositories'),
      read<FromSnapshot<PullRequestRow>>(fetcher, '/api/pull-requests'),
      read<FromSnapshot<WorkflowRunRow>>(fetcher, '/api/workflow-runs'),
      read<FromSnapshot<ReleaseRow>>(fetcher, '/api/releases'),
      read<{ rows: EventRow[] }>(fetcher, `/api/events?since=${encodeURIComponent(since)}`),
    ]);
  const errors: string[] = [];
  function part<T>(answer: T | string, empty: T): T {
    if (typeof answer === 'string') {
      errors.push(answer);
      return empty;
    }
    return answer;
  }
  return {
    data: part<DashboardData | null>(data, null),
    board: part(board, none),
    repositories: part(repositories, none),
    pullRequests: part(pullRequests, none),
    workflowRuns: part(workflowRuns, none),
    releases: part(releases, none),
    events: part(events, { rows: [] }).rows,
    since,
    errors,
  };
}

// ---------------------------------------------------------------------------------------------
// Selections
// ---------------------------------------------------------------------------------------------

/** The repositories of the board whose `main` is red, in the board's order. */
export function redMain(view: OverviewData): string[] {
  return view.board.rows
    .filter((row) => row.deviations.includes(RED_MAIN))
    .map((row) => row.repository);
}

/** A controller's state on the overview. */
export type ControllerState = 'busy' | 'idle' | 'silent';

/**
 * The live controller sessions of `data`: each live session working directly under the checkouts
 * root, conductor's own (in the records root) aside.
 */
export function controllers(data: DashboardData): Session[] {
  const root = data.reading.checkouts.replace(/\/+$/, '');
  return data.sessions.filter((session) => {
    if (session.exited || session.cwd === null || session.cwd === data.reading.root) {
      return false;
    }
    const parent = session.cwd.replace(/\/+$/, '').replace(/\/[^/]+$/, '');
    return parent === root;
  });
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

/** The controller session of the repository `name`: the one in its checkout. */
export function controllerOf(data: DashboardData | null, name: string): Session | null {
  if (data === null) {
    return null;
  }
  return (
    controllers(data).find((session) => session.cwd?.replace(/\/+$/, '').endsWith(`/${name}`)) ??
    null
  );
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
  return [
    {
      label: 'needs you',
      value: String(awaiting + red.length),
      sub: `${counted(awaiting, 'decision')} · ${red.length} red CI`,
      tone: 'amber',
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
  /** The command to copy. */
  command: string;
}

/** The workflows of `name` whose latest `main` run failed, by name. */
function failingWorkflows(view: OverviewData, name: string): string[] {
  return view.workflowRuns.rows
    .filter((row) => row.repository === name && row.conclusion === 'Failure')
    .map((row) => row.workflow);
}

/**
 * "Needs you": one card per `awaiting_operator` row (class O for the operator's decision, H for
 * their hands), answered with the decision line to send conductor; then one per repository whose
 * `main` is red, with the command that lists its runs in its checkout.
 */
export function needsYou(view: OverviewData): NeedCard[] {
  const cards: NeedCard[] = [];
  for (const row of view.data?.awaiting_operator ?? []) {
    const hands = row.class === 'H';
    cards.push({
      kind: hands ? 'hands' : 'operator',
      pill: `${hands ? 'hands' : 'operator'} · class ${row.class ?? '?'}`,
      repository: null,
      text: row.question ?? '',
      command: `[DECISION ${row.id ?? '?'}] <option> <reason>`,
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
