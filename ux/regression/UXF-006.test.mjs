import test from 'node:test';
import assert from 'node:assert/strict';
import {withSite, openPage, GATING} from './harness.mjs';

const PAGES = ['/conductor/docs/getting-started', '/conductor/docs/guides/start-conductor', '/conductor/docs/reference/cli'];

test('UXF-006: text in code blocks computes to 14 px or more', async () => {
  await withSite(async ({browser, url}) => {
    for (const vp of GATING) for (const p of PAGES) {
      const {page, close} = await openPage(browser, url, p, vp, 'light');
      const small = await page.evaluate(() => {
        const out = [];
        for (const pre of document.querySelectorAll('.theme-code-block pre')) {
          for (const el of [pre, ...pre.querySelectorAll('.token')]) {
            const fs = parseFloat(getComputedStyle(el).fontSize);
            if (fs < 14) out.push(fs);
          }
        }
        return out;
      });
      assert.deepEqual(small, [], `${p} ${vp.name}: ${small.length} code elements under 14 px`);
      assert.ok(await page.locator('.theme-code-block pre').count() > 0, `${p} has code blocks`);
      await close();
    }
  });
});
