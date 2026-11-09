import test from 'node:test';
import assert from 'node:assert/strict';
import {withSite, openPage, axeRule, GATING, THEMES} from './harness.mjs';

const PAGES = [
  '/conductor/docs/getting-started',
  '/conductor/docs/guides/start-conductor',
  '/conductor/docs/guides/configure-an-instance',
  '/conductor/docs/security/overview',
  '/conductor/docs/concepts/the-loop',
];

test('UXF-003: links inside running text are distinguishable without colour alone (axe link-in-text-block)', async () => {
  await withSite(async ({browser, url}) => {
    for (const vp of GATING) for (const theme of THEMES) for (const p of PAGES) {
      const {page, close} = await openPage(browser, url, p, vp, theme);
      const bad = await axeRule(page, 'link-in-text-block');
      assert.deepEqual(bad, [], `${p} ${vp.name} ${theme}: ${bad.length} link(s) rely on colour alone`);
      await close();
    }
  });
});
