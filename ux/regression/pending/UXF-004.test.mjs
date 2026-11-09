import test from 'node:test';
import assert from 'node:assert/strict';
import {withSite, openPage, axeRule, GATING, THEMES} from '../harness.mjs';

test('UXF-004: every checklist checkbox has an accessible name (axe label)', async () => {
  await withSite(async ({browser, url}) => {
    for (const vp of GATING) for (const theme of THEMES) {
      const {page, close} = await openPage(browser, url, '/conductor/docs/security/hardening', vp, theme);
      const bad = await axeRule(page, 'label');
      assert.deepEqual(bad, [], `${vp.name} ${theme}: unlabelled form controls`);
      const n = await page.locator('input[type=checkbox]').count();
      assert.ok(n > 0, 'the checklist page still has checkboxes');
      await close();
    }
  });
});
