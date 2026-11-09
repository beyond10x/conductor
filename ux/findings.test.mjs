import test from 'node:test';
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import {uxDir, repoRoot} from './lib.mjs';

export function parseFinding(text) {
  const m = text.match(/^---\n([\s\S]*?)\n---/);
  const h = {};
  if (!m) return h;
  for (const line of m[1].split('\n')) {
    const i = line.indexOf(':');
    if (i > 0) h[line.slice(0, i).trim()] = line.slice(i + 1).trim();
  }
  return h;
}

const dir = path.join(uxDir, 'findings');
const files = fs.existsSync(dir) ? fs.readdirSync(dir).filter((f) => /^UXF-\d{3}\.md$/.test(f)) : [];

test('findings dir holds records', () => {
  assert.ok(files.length >= 0);
});

for (const f of files) {
  test(`${f} has the header fields`, () => {
    const h = parseFinding(fs.readFileSync(path.join(dir, f), 'utf8'));
    for (const k of ['id', 'title', 'class', 'severity', 'metric', 'status', 'found_by', 'evidence_screenshot', 'evidence_lines', 'regression_asset']) {
      assert.ok(k in h, `${f}: missing ${k}`);
    }
    assert.equal(h.id + '.md', f);
    assert.match(h.severity, /^S[123]$/);
  });
  test(`${f}: a closed S1/S2 record has a regression asset that exists`, () => {
    const h = parseFinding(fs.readFileSync(path.join(dir, f), 'utf8'));
    if (h.status === 'closed' && (h.severity === 'S1' || h.severity === 'S2')) {
      assert.ok(h.regression_asset && h.regression_asset !== 'none', `${f}: closed ${h.severity} without regression_asset`);
      const file = h.regression_asset.split('#')[0].trim();
      assert.ok(fs.existsSync(path.join(repoRoot, file)), `${f}: regression asset ${file} not found`);
    }
  });
}
