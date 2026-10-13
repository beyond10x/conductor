// The shape of `/data.json`, as `collect` in crates/conductor/src/dashboard.rs writes it. The
// fixture src/fixtures/data.json has these keys, and crates/conductor/tests/dashboard.rs holds
// its keys to the ones the server sends.

export interface Dispatch {
  id: string;
  goal: string | null;
  brief: string;
  event: string | null;
  event_at: string | null;
  event_ago: string | null;
}

export interface Session {
  name: string | null;
  kind: string | null;
  exited: boolean;
  status: string | null;
  state: string | null;
  cwd: string | null;
  started_at: string | null;
  started_ago: string | null;
  dispatch: Dispatch | null;
}

export interface UsageLimit {
  text: string;
  sessions: string[];
}

export interface ContextLine {
  session: string;
  name: string | null;
}

export interface Watch {
  usage_limit: UsageLimit[] | null;
  usage_limit_today: string[] | null;
  context: ContextLine[] | null;
}

export interface RedRun {
  repo: string;
  workflow: string;
  run_id: string;
  conclusion: string;
  time: string;
}

export interface Decision {
  id: string | null;
  class: string | null;
  choice: string | null;
  reason: string | null;
}

export interface Awaiting {
  id: string | null;
  class: string | null;
  question: string | null;
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
  root: string;
  ci_state: string;
  checkouts: string;
}

export interface DashboardData {
  generated_at: string;
  disk: Disk;
  controllers: number;
  build_slot: string;
  sessions: Session[];
  watch: Watch;
  context_handover: number;
  red_ci: RedRun[] | null;
  decisions: Decision[];
  awaiting_operator: Awaiting[];
  usage: Usage;
  reading: Reading;
  errors: string[];
}
