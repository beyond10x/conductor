// The finding records under ux/findings/ (skill § 5): a header of `key: value` lines between `---`
// lines, then the text. No S1 or S2 finding closes without a regression asset.
import { readdirSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

export const FIELDS = [
  'id',
  'title',
  'class',
  'severity',
  'metric',
  'status',
  'found_by',
  'evidence_recording',
  'evidence_screenshot',
  'evidence_lines',
  'regression_asset',
] as const;

export type Finding = Record<(typeof FIELDS)[number], string>;

/** The header of one record. */
export function parse(text: string): Partial<Finding> {
  const match = /^---\n([\s\S]*?)\n---\n/.exec(text);
  const header: Record<string, string> = {};
  for (const line of (match?.[1] ?? '').split('\n')) {
    const at = line.indexOf(':');
    if (at > 0) {
      header[line.slice(0, at).trim()] = line.slice(at + 1).trim();
    }
  }
  return header as Partial<Finding>;
}

/** What is wrong with one record: a missing field, or a closed S1/S2 with no regression asset. */
export function problems(name: string, finding: Partial<Finding>): string[] {
  const found: string[] = [];
  for (const field of FIELDS) {
    if (finding[field] === undefined || finding[field] === '') {
      found.push(`${name}: no ${field}`);
    }
  }
  const asset = (finding.regression_asset ?? '').trim().toLowerCase();
  const closed = (finding.status ?? '').toLowerCase() === 'closed';
  const serious = finding.severity === 'S1' || finding.severity === 'S2';
  if (closed && serious && (asset === '' || asset === 'none' || asset === '-')) {
    found.push(`${name}: a closed ${finding.severity} finding with no regression asset`);
  }
  return found;
}

/** Every record of `dir`, by file name. */
export function readFindings(dir: string): [string, Partial<Finding>][] {
  return readdirSync(dir)
    .filter((file) => /^UXF-\d{3}\.md$/.test(file))
    .sort()
    .map((file) => [file, parse(readFileSync(join(dir, file), 'utf8'))]);
}
