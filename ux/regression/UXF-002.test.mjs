import test from 'node:test';
import assert from 'node:assert/strict';
import {withSite, openPage, GATING, THEMES} from './harness.mjs';

const PAGES = ['/conductor/', '/conductor/docs/getting-started', '/conductor/docs/guides/start-conductor'];

test('UXF-002: the theme toggle is fully in the viewport and not covered at every gating viewport', async () => {
  await withSite(async ({browser, url}) => {
    for (const vp of GATING) for (const theme of THEMES) for (const p of PAGES) {
      const {page, close} = await openPage(browser, url, p, vp, theme);
      const state = await page.evaluate(([vw, vh]) => {
        const el = document.querySelector('button[aria-label^="Switch between dark and light mode"]');
        if (!el) return 'missing';
        const r = el.getBoundingClientRect();
        if (r.width <= 0 || r.left < 0 || r.top < 0 || r.right > vw || r.bottom > vh) return 'outside viewport';
        const t = document.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
        return t && (el.contains(t) || t.contains(el)) ? 'ok' : 'covered';
      }, [vp.width, vp.height]);
      assert.equal(state, 'ok', `${p} ${vp.name} ${theme}: theme toggle ${state}`);
      await close();
    }
  });
});

test('UXF-002: the visible theme toggle switches the theme', async () => {
  await withSite(async ({browser, url}) => {
    const vp = GATING.find((v) => v.name === '390x844');
    const {page, close} = await openPage(browser, url, '/conductor/docs/getting-started', vp, 'light');
    const before = await page.evaluate(() => document.documentElement.dataset.theme);
    let after = before;
    for (let i = 0; i < 3 && after === before; i++) {
      await page.getByRole('button', {name: /Switch between dark and light mode/}).click();
      await page.waitForTimeout(200);
      after = await page.evaluate(() => document.documentElement.dataset.theme);
    }
    assert.notEqual(after, before, 'three presses of the toggle changed nothing');
    await close();
  });
});
