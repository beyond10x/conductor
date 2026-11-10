// The findings test's checker (the ui-iteration-loop skill, § 5): every .md file in ux/findings/ is
// a record named UXF-nnn.md that carries the header fields, and no closed S1 or S2 finding stands
// without a regression asset: a test or scenario file that exists. tests/ux-findings.spec.ts runs it
// over ux/findings/ in npm test.
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

/** The header fields of a finding record, in the skill's order. */
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

/**
 * What a record's status may read, and nothing else: a status in other words ("Closed", "closed in
 * R-02", "fixed") would pass the closed-finding rule unread.
 */
export const STATUSES = ['open', 'closed', 'duplicate', 'known-loss'] as const;

/**
 * What a regression asset may be, as a path relative to the dashboard app: a file that asserts or
 * that the gate drives. A record, a screenshot or a manifest asserts nothing.
 */
export const ASSETS = [/^tests\/[^/]+\.spec\.ts$/, /^src\/(.+\/)?__tests__\/[^/]+$/, /^ux\/scenarios\/.+$/, /^ux\/breaks\.ts$/];
export const ASSET_KINDS = 'tests/*.spec.ts, src/**/__tests__/*, ux/scenarios/**, ux/breaks.ts';

/** Whether `asset`, relative to `root`, is a file. */
export function fileIn(root: string): (asset: string) => boolean {
  return (asset) => {
    const path = join(root, asset);
    return existsSync(path) && statSync(path).isFile();
  };
}

/** The header of a record: the `key: value` lines between its two `---` lines. */
export function parseFinding(text: string): Record<string, string> | null {
  const match = /^---\n([\s\S]*?)\n---/.exec(text);
  if (match === null) {
    return null;
  }
  const header: Record<string, string> = {};
  for (const line of (match[1] ?? '').split('\n')) {
    const at = line.indexOf(':');
    if (at > 0) {
      header[line.slice(0, at).trim()] = line.slice(at + 1).trim();
    }
  }
  return header;
}

/**
 * What is wrong with the record `file` holding `text`; `isFile` says whether a regression asset, a
 * path relative to the dashboard app, is a file there ([`fileIn`]). A directory is no asset.
 */
export function findingProblems(file: string, text: string, isFile: (asset: string) => boolean): string[] {
  const named = /^UXF-\d{3}\.md$/.test(file) ? [] : [`${file}: a record is named UXF-nnn.md`];
  const header = parseFinding(text);
  if (header === null) {
    return [...named, `${file}: no header`];
  }
  const problems = [...named, ...FIELDS.filter((field) => !(field in header)).map((field) => `${file}: missing ${field}`)];
  if (problems.length > named.length) {
    return problems;
  }
  if (`${header.id}.md` !== file) {
    problems.push(`${file}: id ${header.id} does not name the file`);
  }
  const severity = header.severity ?? '';
  if (!/^S[123]$/.test(severity)) {
    problems.push(`${file}: severity ${severity} is not S1, S2 or S3`);
  }
  const status = header.status ?? '';
  if (!(STATUSES as readonly string[]).includes(status)) {
    problems.push(`${file}: status ${status} is not one of ${STATUSES.join(', ')}`);
  }
  if (status === 'closed' && (severity === 'S1' || severity === 'S2')) {
    const asset = (header.regression_asset ?? '').split('#')[0]?.trim() ?? '';
    if (asset === '' || /^none\b/.test(asset)) {
      problems.push(`${file}: closed ${severity} without a regression asset`);
    } else if (!/(^|\/)[^/]+\.[A-Za-z0-9]+$/.test(asset)) {
      problems.push(`${file}: regression asset ${asset} does not name a file`);
    } else if (asset.split('/').some((part) => part === '..' || part === '.') || !ASSETS.some((kind) => kind.test(asset))) {
      problems.push(`${file}: regression asset ${asset} is not a test or scenario file (${ASSET_KINDS})`);
    } else if (!isFile(asset)) {
      problems.push(`${file}: regression asset ${asset} not found`);
    }
  }
  return problems;
}

/** Every record in `dir`, by name: each .md file, whatever it is called. */
export function readFindings(dir: string): { file: string; text: string }[] {
  if (!existsSync(dir)) {
    return [];
  }
  return readdirSync(dir)
    .filter((file) => file.toLowerCase().endsWith('.md'))
    .sort()
    .map((file) => ({ file, text: readFileSync(join(dir, file), 'utf8') }));
}
