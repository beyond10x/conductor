// The shape of `/data.json`, as `collect` in crates/conductor/src/dashboard.rs writes it. The
// fixture src/fixtures/data.json has these keys, and crates/conductor/tests/dashboard.rs holds
// its keys to the ones the server sends.

export interface Dispatch {
  id: string;
  /** Whether it is open: its last event is none of `done`, `cancelled`, `failed`. */
  open: boolean;
  goal: string | null;
  brief: string;
  event: string | null;
  event_at: string | null;
  event_ago: string | null;
}

/** The role a session plays for the instance (story:sessions-page-coherent). */
export type Role = 'conductor' | 'maintainer' | 'controller';

/**
 * One of the instance's sessions, from its records: its conductor, its maintainer, each
 * controller the store runs. A record no live session holds reads `exited`, state `not running`.
 */
export interface Session {
  name: string | null;
  role: Role;
  /** The controller's repository; null for the conductor and the maintainer. */
  repository: string | null;
  kind: string | null;
  exited: boolean;
  status: string | null;
  state: string | null;
  cwd: string | null;
  started_at: string | null;
  started_ago: string | null;
  dispatch: Dispatch | null;
}

/** A usage limit in force: its text, when it resets, and the sessions it was found in. */
export interface UsageLimit {
  text: string;
  resets_at: string | null;
  sessions: string[];
}

export interface ContextLine {
  session: string;
  name: string | null;
}

export interface Watch {
  limits: UsageLimit[] | null;
  context: ContextLine[] | null;
}

export interface Decision {
  id: string;
  at: string | null;
  class: string | null;
  choice: string;
  reason: string | null;
}

/**
 * An open item that asks something of the operator, as the Decisions page lists it: `action` is
 * one line, `answer: …` (class O, with its options) or `do: …` (class H, with the command), and
 * `link` its place on the Decisions page.
 */
export interface Awaiting {
  id: string;
  class: string;
  repository: string;
  question: string;
  action: string;
  link: string;
}

/** The newest complete snapshot and its age; `stale` when older than the instance's cycle. */
export interface SnapshotAge {
  id: string;
  started_at: string;
  age_seconds: number | null;
  age: string | null;
  cadence_seconds: number | null;
  stale: boolean;
}

export interface WasteRow {
  repository: string | null;
  what: string;
  path: string | null;
  count: number | null;
  bytes: number | null;
  owner: string | null;
  age: string | null;
  state: string;
  reclaimable: boolean;
  gc: string;
}

export interface Disk {
  path: string;
  free_bytes: number | null;
  size_bytes: number | null;
  measured_at: string | null;
  measured_ago: string | null;
  measuring: boolean;
  total_bytes: number | null;
  reclaimable_bytes: number | null;
  rows: WasteRow[];
}

export interface UsageRow {
  repository: string;
  sessions: number;
  calls: number;
  input: number;
  output: number;
  cache_read: number;
  cache_write: number;
  cache_read_percent: number;
}

export interface Usage {
  since: string | null;
  measured_at: string | null;
  measured_ago: string | null;
  measuring: boolean;
  rows: UsageRow[];
  errors: string[];
}

export interface Reading {
  sessions: string;
  /** When the server last read the session list, which it reads every 10 s off the request. */
  sessions_read_at: string | null;
  root: string;
  ci_state: string;
  checkouts: string;
}

export interface DashboardData {
  generated_at: string;
  disk: Disk;
  /** The controller rows a live session holds. */
  controllers: number;
  snapshot: SnapshotAge | null;
  build_slot: string;
  sessions: Session[];
  watch: Watch;
  context_handover: number;
  decisions: Decision[];
  awaiting_operator: Awaiting[];
  usage: Usage;
  reading: Reading;
  errors: string[];
}
