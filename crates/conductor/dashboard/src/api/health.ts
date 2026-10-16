// What the health view (`#/health`, story:dashboard-health) reads: `/api/health`, which
// crates/conductor/src/dashboard/health.rs answers. The fixture
// src/views/__tests__/fixtures/health.json carries these keys, and
// crates/conductor/tests/dashboard.rs holds them to the ones the server sends.

/** `conductor doctor`'s rows and verdict, from the server's cache. */
export interface DoctorHealth {
  /** When the rows were taken; null before the first run ends. */
  taken_at: string | null;
  /** Whether the next run is being taken. */
  taking: boolean;
  /** What `conductor doctor` prints but its verdict, one line each. */
  rows: string[];
  /** `ready`, or `not ready: ` and what failed; null before the first run ends. */
  verdict: string | null;
  ready: boolean | null;
  /** Why doctor could not run, if it could not. */
  error: string | null;
}

/** The newest snapshot, in any state. */
export interface SnapshotHealth {
  snapshot_id: string;
  /** `Collecting`, `Complete` or `Failed`. */
  state: string;
  started_at: string;
  disk_free_bytes: number;
  failure_reason: string | null;
}

/** One collector's run in that snapshot. */
export interface CollectorRun {
  collector: string;
  started_at: string;
  duration_ms: number | null;
  /** How the run ended, as the specification names it: `Completed`, `Failed`, `OverBudget`. */
  outcome: string;
  /** The bytes the snapshot read and wrote while it ran; `null` where they were not recorded. */
  read_bytes: number | null;
  written_bytes: number | null;
}

/** The class of a member failure's first line, as `/api/health` names it. */
export type FailureClass = 'needs login' | 'unreachable host' | 'timed out' | 'old store' | 'other';

/** One member a collector could not read in that snapshot, with what to do about it. */
export interface MemberFailure {
  member: string;
  collector: string;
  error_line: string;
  class: FailureClass;
  fix: string;
}

/** One disk-waste row, as the overview's disk panel shows it. */
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

/** `ok` above `thresholds.disk_low`, `low` at or below it, `admit-only` at or below `disk_admit`. */
export type DiskState = 'ok' | 'low' | 'admit-only';

/** The free disk against the instance's floors, and every disk-waste row. */
export interface DiskHealth {
  path: string;
  free_bytes: number | null;
  disk_low_bytes: number;
  disk_admit_bytes: number;
  state: DiskState | null;
  measured_at: string | null;
  measured_ago: string | null;
  measuring: boolean;
  total_bytes: number | null;
  reclaimable_bytes: number | null;
  rows: WasteRow[];
}

/**
 * What the dashboard's process holds in memory (story:dashboard-memory-bound):
 * `conductor.observation.DashboardMemory`.
 */
export interface MemoryHealth {
  /** Its resident set; null where the system does not report it. */
  rss_bytes: number | null;
  store_handles: number;
  store_replays: number;
  /** The most replays of one tree alive at once since start: 1 when bounded. */
  store_replays_peak: number;
  store_bytes: number;
  memo_entries: number;
  memo_entries_peak_endpoint: number;
  memo_bytes: number;
}

/** `/api/health`. */
export interface HealthData {
  generated_at: string;
  doctor: DoctorHealth;
  snapshot: SnapshotHealth | null;
  collector_runs: CollectorRun[];
  member_failures: MemberFailure[];
  disk: DiskHealth;
  memory: MemoryHealth;
  errors: string[];
}

/** What one read of `/api/health` gave: the data, or why there is none. */
export interface HealthRead {
  health: HealthData | null;
  error: string | null;
}

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

/** Reads `/api/health` through `fetcher`. */
export async function loadHealth(
  fetcher: Fetch = (input, init) => fetch(input, init),
): Promise<HealthRead> {
  try {
    const answer = await fetcher('/api/health', { cache: 'no-cache' });
    if (!answer.ok) {
      return { health: null, error: `could not read /api/health: ${answer.status}` };
    }
    return { health: (await answer.json()) as HealthData, error: null };
  } catch (error) {
    return { health: null, error: `could not read /api/health: ${String(error)}` };
  }
}

/** A duration in milliseconds as the view shows it: `41s`, `2m03s`, `1h02m`; `—` for none. */
export function duration(milliseconds: number | null | undefined): string {
  if (typeof milliseconds !== 'number') {
    return '—';
  }
  const seconds = Math.floor(milliseconds / 1000);
  const two = (count: number) => String(count).padStart(2, '0');
  if (seconds < 60) {
    return `${seconds}s`;
  }
  if (seconds < 3600) {
    return `${Math.floor(seconds / 60)}m${two(seconds % 60)}s`;
  }
  return `${Math.floor(seconds / 3600)}h${two(Math.floor((seconds % 3600) / 60))}m`;
}

/** The time of day of an RFC 3339 instant, `HH:MMZ`; the text itself when it is none. */
export function clock(at: string | null): string {
  if (at === null) {
    return '—';
  }
  const parsed = new Date(at);
  return Number.isNaN(parsed.getTime()) ? at : `${parsed.toISOString().slice(11, 16)}Z`;
}

/** A specification's state name in words: `OverBudget` is `over budget`. */
export function words(name: string): string {
  return name.replace(/([a-z])([A-Z])/g, '$1 $2').toLowerCase();
}
