// ux/baseline.json, read (skill § 6). Kept free of syntax Node cannot strip, so the ratchet runs
// it with plain `node`.
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export interface Entry {
  value: number;
  tolerance: number;
  /** The decision that let this value get worse than main's. */
  decision?: string;
}

export interface Baseline {
  about: string;
  scenarios: Record<string, Record<string, Entry>>;
  tolerances: Record<string, number>;
  /**
   * The contract's assertions that fail on today's app because of a recorded finding: the
   * assertion (`*` matches within one `/`-, `@`- or `:`-separated part) and the finding's id.
   */
  known: Record<string, string>;
}

export const BASELINE = join(dirname(fileURLToPath(import.meta.url)), 'baseline.json');

export function readBaseline(file: string = BASELINE): Baseline {
  try {
    return JSON.parse(readFileSync(file, 'utf8')) as Baseline;
  } catch {
    return { about: '', scenarios: {}, tolerances: {}, known: {} };
  }
}

/** Whether `pattern` names `assertion`. */
export function matches(pattern: string, assertion: string): boolean {
  const escaped = pattern.replace(/[.+?^${}()|[\]\\]/g, '\\$&').replace(/\*/g, '[^/@:]*');
  return new RegExp(`^${escaped}$`).test(assertion);
}

/** The finding a known `assertion` is recorded under, or null. */
export function known(assertion: string, baseline: Baseline = readBaseline()): string | null {
  for (const [pattern, finding] of Object.entries(baseline.known)) {
    if (matches(pattern, assertion)) {
      return finding;
    }
  }
  return null;
}
