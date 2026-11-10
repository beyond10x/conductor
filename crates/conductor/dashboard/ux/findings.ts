// The findings test's checker (the ui-iteration-loop skill, § 5): every record ux/findings/UXF-nnn.md
// carries the header fields, and no closed S1 or S2 finding stands without a regression asset that
// exists. tests/ux-findings.spec.ts runs it over ux/findings/ in npm test.
import { readFileSync, readdirSync, existsSync } from 'node:fs';
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
 * What is wrong with the record `file` holding `text`; `exists` says whether a regression asset,
 * a path relative to the dashboard app, is there.
 */
export function findingProblems(file: string, text: string, exists: (asset: string) => boolean): string[] {
  const header = parseFinding(text);
  if (header === null) {
    return [`${file}: no header`];
  }
  const problems = FIELDS.filter((field) => !(field in header)).map((field) => `${file}: missing ${field}`);
  if (problems.length > 0) {
    return problems;
  }
  if (`${header.id}.md` !== file) {
    problems.push(`${file}: id ${header.id} does not name the file`);
  }
  const severity = header.severity ?? '';
  if (!/^S[123]$/.test(severity)) {
    problems.push(`${file}: severity ${severity} is not S1, S2 or S3`);
  }
  if (header.status === 'closed' && (severity === 'S1' || severity === 'S2')) {
    const asset = (header.regression_asset ?? '').split('#')[0]?.trim() ?? '';
    if (asset === '' || asset === 'none' || asset.startsWith('none ')) {
      problems.push(`${file}: closed ${severity} without a regression asset`);
    } else if (!exists(asset)) {
      problems.push(`${file}: regression asset ${asset} not found`);
    }
  }
  return problems;
}

/** Every record `UXF-nnn.md` in `dir`, by name. */
export function readFindings(dir: string): { file: string; text: string }[] {
  if (!existsSync(dir)) {
    return [];
  }
  return readdirSync(dir)
    .filter((file) => /^UXF-\d{3}\.md$/.test(file))
    .sort()
    .map((file) => ({ file, text: readFileSync(join(dir, file), 'utf8') }));
}
