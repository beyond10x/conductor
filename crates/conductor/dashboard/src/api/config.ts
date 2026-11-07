// What the Configuration page reads and writes (story:dashboard-config-page). `GET /api/config`
// answers `conductor.config.ConfigPage`: the effective config of the instance this dashboard
// runs, one field per key `conductor config show` writes, the host's instances and the
// repositories with their activity. `POST /api/config` takes `conductor.config.WriteConfig`, with
// the page's token as the Decisions page's answers: `check: true` answers the server's diff and
// each problem by its YAML path and writes nothing; without it, it keeps a backup, writes the
// file and records one class-C decision by the operator.
//
// Staging lives here, apart from the view, so Vitest holds it: a staged value per path, checked
// as `conductor config validate` checks that kind of key, and the YAML diff of what is staged.

import { pageToken } from './decisions';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

export type Effect = 'WatchTick' | 'Cycle' | 'LoopRestart' | 'Fixed';

export interface ConfigField {
  path: string;
  value: string;
  default: string | null;
  takes_effect: Effect;
}

export interface ConfigInstance {
  name: string;
  dashboard_port: number;
  session_prefix: string | null;
  forge: string;
  current: boolean;
}

export interface RepositoryRow {
  repository: string;
  activity: 'Active' | 'Inactive';
  decided_by: 'Mark' | 'Snapshot';
  marked_by: string | null;
  reason: string | null;
}

export interface ConfigPage {
  file: string;
  instance: string;
  index: number;
  fields: ConfigField[];
  instances: ConfigInstance[];
  repositories: RepositoryRow[];
  errors: string[];
  writable: boolean;
}

export interface Problem {
  path: string;
  message: string;
}

export interface Written {
  diff: string;
  restart: boolean;
  backup: string | null;
  decision_id: string | null;
  problems: Problem[];
}

export const CONFIG_PATH = '/api/config';

/** Reads the page's data; why when it cannot. */
export async function loadConfig(
  fetcher: Fetch = (input, init) => fetch(input, init),
): Promise<{ page: ConfigPage | null; error: string | null }> {
  try {
    const answer = await fetcher(CONFIG_PATH, { cache: 'no-store' });
    if (!answer.ok) {
      return { page: null, error: `could not read ${CONFIG_PATH}: ${answer.status}` };
    }
    return { page: (await answer.json()) as ConfigPage, error: null };
  } catch (error) {
    return { page: null, error: `could not read ${CONFIG_PATH}: ${String(error)}` };
  }
}

// ---------------------------------------------------------------------------------------------
// Staging
// ---------------------------------------------------------------------------------------------

/** A staged value: the YAML text written, or `null` to remove the key so its default applies. */
export type StagedValue = string | null;

/** One staged activity switch, with the operator's reason. */
export interface StagedActivity {
  repository: string;
  activity: 'Active' | 'Inactive';
  reason: string;
  added: boolean;
}

export interface Staging {
  values: Map<string, StagedValue>;
  activity: Map<string, StagedActivity>;
}

export function emptyStaging(): Staging {
  return { values: new Map(), activity: new Map() };
}

/**
 * Stages `value` for `field`; staging the value the file holds again unstages it, as does
 * staging the default of a key the file does not write.
 */
export function stage(staging: Staging, field: ConfigField, value: StagedValue): void {
  const unchanged =
    value === field.value || (value === null && field.default === null);
  if (unchanged) {
    staging.values.delete(field.path);
  } else {
    staging.values.set(field.path, value);
  }
}

/** The value `field` shows with what is staged. */
export function shown(staging: Staging, field: ConfigField): string {
  if (!staging.values.has(field.path)) {
    return field.value;
  }
  const value = staging.values.get(field.path) ?? null;
  return value ?? field.default ?? field.value;
}

/** How many changes are staged. */
export function count(staging: Staging): number {
  return staging.values.size + staging.activity.size;
}

// ---------------------------------------------------------------------------------------------
// Validation, as `conductor config validate` checks each kind of key
// ---------------------------------------------------------------------------------------------

const DURATIONS = new Set([
  'cadence.cycle',
  'cadence.watch',
  'cadence.ci',
  'thresholds.watchdog_every',
  'thresholds.silent_session',
  'thresholds.stalled_session',
  'thresholds.build_queued',
  'thresholds.tree_stale',
  'thresholds.short_lane_max',
]);
const SIZES = new Set([
  'thresholds.disk_low',
  'thresholds.disk_clear',
  'thresholds.disk_admit',
  'thresholds.build_slot',
  'thresholds.build_size',
  'thresholds.gate_stop',
  'thresholds.gate_resume',
  'thresholds.repo_cache_size',
]);
/** Whole numbers and the least each takes. */
const COUNTS: Record<string, number> = {
  'controllers.max_working': 1,
  'controllers.max_subagents': 1,
  'controllers.max_open_reviews': 0,
  'thresholds.cpu_reserve': 0,
  'thresholds.build_slots': 1,
  'thresholds.short_lane_slots': 0,
  'retention.snapshots': 1,
};
/** Keys that take one of a few words. */
export const CHOICES: Record<string, string[]> = {
  'thresholds.lean_debuginfo': ['true', 'false'],
  'thresholds.repo_cache': ['on', 'off'],
  'authority.class_c': ['conductor', 'operator'],
  'authority.class_o': ['conductor', 'operator'],
};

function gib(text: string): number {
  return Number.parseInt(text, 10);
}

/**
 * The problem of the value `value` staged for `path`, or null; `others` gives a key's value as
 * staged, for the checks between two keys.
 */
export function problemOf(
  path: string,
  value: StagedValue,
  others: (path: string) => string | undefined = () => undefined,
): string | null {
  if (value === null) {
    return null;
  }
  const text = value.trim();
  if (text === '') {
    return 'must not be empty';
  }
  if (DURATIONS.has(path) && !/^\d+[smhd]$/.test(text)) {
    return 'a duration: a whole number with s, m, h or d, such as 30m';
  }
  if (SIZES.has(path) && !/^\d+G$/.test(text)) {
    return 'a size in GiB: a whole number with G, such as 200G';
  }
  if (path === 'cadence.daily' && !/^([01]\d|2[0-3]):[0-5]\d$/.test(text)) {
    return 'a UTC time of day, HH:MM, such as 08:07';
  }
  if (path === 'thresholds.context_handover' && !/^\d+k?$/.test(text)) {
    return 'a count of tokens: a whole number, or thousands with k, such as 300k';
  }
  const least = COUNTS[path];
  if (least !== undefined) {
    if (!/^\d+$/.test(text)) {
      return 'must be a whole number';
    }
    if (Number(text) < least) {
      return `must be at least ${least}`;
    }
  }
  const choices = CHOICES[path] ?? (/^roles\[\d+\]\.harness$/.test(path) ? ['claude', 'codex'] : undefined);
  if (choices !== undefined && !choices.includes(text)) {
    return `one of ${choices.join(', ')}`;
  }
  const pairs: [string, string][] = [
    ['thresholds.disk_clear', 'thresholds.disk_low'],
    ['thresholds.gate_resume', 'thresholds.gate_stop'],
  ];
  for (const [upper, lower] of pairs) {
    if (path === upper) {
      const other = others(lower);
      if (other !== undefined && SIZES.has(lower) && gib(text) <= gib(other)) {
        return `must be above ${lower.split('.')[1]} (${other})`;
      }
    }
    if (path === lower) {
      const other = others(upper);
      if (other !== undefined && gib(other) <= gib(text)) {
        return `must be below ${upper.split('.')[1]} (${other})`;
      }
    }
  }
  return null;
}

/** Every problem of what is staged, each named by its YAML path under the instance. */
export function problems(staging: Staging, fields: ConfigField[]): Problem[] {
  const byPath = new Map(fields.map((field) => [field.path, field]));
  const others = (path: string): string | undefined => {
    const field = byPath.get(path);
    return field === undefined ? undefined : shown(staging, field);
  };
  const found: Problem[] = [];
  for (const [path, value] of staging.values) {
    if (byPath.get(path)?.takes_effect === 'Fixed') {
      found.push({ path, message: 'the page does not change this key; config migrate moves it' });
      continue;
    }
    const message = problemOf(path, value, others);
    if (message !== null) {
      found.push({ path, message });
    }
  }
  for (const switched of staging.activity.values()) {
    if (switched.reason.trim() === '') {
      found.push({
        path: `repositories.${switched.repository}.reason`,
        message: 'required when its activity changes',
      });
    }
  }
  return found;
}

// ---------------------------------------------------------------------------------------------
// The diff
// ---------------------------------------------------------------------------------------------

export interface DiffLine {
  kind: 'section' | 'removed' | 'added';
  text: string;
}

/** The section of `path`, the key below it and the key's indent. */
function split(path: string): [string, string] {
  const at = path.search(/[.[]/);
  if (at < 0) {
    return ['', path];
  }
  const rest = path.slice(at);
  return [path.slice(0, at), rest.startsWith('.') ? rest.slice(1) : rest];
}

/** The YAML diff of what is staged, section by section, in the order the fields list the keys. */
export function diff(staging: Staging, fields: ConfigField[]): DiffLine[] {
  const lines: DiffLine[] = [];
  let section: string | null = null;
  for (const field of fields) {
    if (!staging.values.has(field.path)) {
      continue;
    }
    const [head, key] = split(field.path);
    if (head !== section) {
      section = head;
      if (head !== '') {
        lines.push({ kind: 'section', text: `  ${head}:` });
      }
    }
    const value = staging.values.get(field.path) ?? null;
    lines.push({ kind: 'removed', text: `-   ${key}: ${field.value}` });
    lines.push({
      kind: 'added',
      text:
        value === null
          ? `+   ${key}: ${field.default ?? ''}  # the default`
          : `+   ${key}: ${value}`,
    });
  }
  if (staging.activity.size > 0) {
    lines.push({ kind: 'section', text: '  repositories:' });
    for (const switched of staging.activity.values()) {
      const reason = switched.reason.trim() === '' ? '<reason missing>' : switched.reason.trim();
      lines.push({
        kind: 'added',
        text: `+   ${switched.repository}: ${switched.activity.toLowerCase()}  # ${reason}${switched.added ? ' (added)' : ''}`,
      });
    }
  }
  return lines;
}

/** Whether a staged key takes effect only when the loop starts again. */
export function needsRestart(staging: Staging, fields: ConfigField[]): boolean {
  return fields.some(
    (field) => staging.values.has(field.path) && field.takes_effect === 'LoopRestart',
  );
}

/** The body of the write. */
export function writeBody(staging: Staging, reason: string, check: boolean): object {
  return {
    changes: [...staging.values].map(([path, value]) =>
      value === null ? { path } : { path, value },
    ),
    activity: [...staging.activity.values()].map((switched) => ({
      repository: switched.repository,
      activity: switched.activity,
      reason: switched.reason.trim(),
    })),
    reason,
    check,
  };
}

/** Sends the staged changes, as a check or a write; the answer, or why there is none. */
export async function sendConfig(
  staging: Staging,
  reason: string,
  check: boolean,
  fetcher: Fetch = (input, init) => fetch(input, init),
  token: string | null = pageToken(),
): Promise<{ written: Written | null; problems: Problem[]; error: string | null }> {
  if (token === null || token === '') {
    return { written: null, problems: [], error: 'the page carries no token; reload it' };
  }
  try {
    const answer = await fetcher(CONFIG_PATH, {
      method: 'POST',
      cache: 'no-store',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', 'X-Conductor-Token': token },
      body: JSON.stringify(writeBody(staging, reason, check)),
    });
    const text = await answer.text();
    if (!answer.ok) {
      try {
        const refused = JSON.parse(text) as { detail: string; problems: Problem[] };
        return { written: null, problems: refused.problems, error: refused.detail };
      } catch {
        return { written: null, problems: [], error: `${answer.status} ${text.trim()}` };
      }
    }
    const written = JSON.parse(text) as Written;
    return { written, problems: written.problems, error: null };
  } catch (error) {
    return { written: null, problems: [], error: String(error) };
  }
}
