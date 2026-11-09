import {chromium, AxeBuilder, VIEWPORTS, THEMES, newThemedContext, settle} from '../lib.mjs';
import {startServer} from '../serve.mjs';

export const GATING = VIEWPORTS.filter((v) => v.gating);
export {THEMES};

export async function withSite(fn) {
  const {server, url} = await startServer();
  const browser = await chromium.launch();
  try {
    return await fn({browser, url});
  } finally {
    await browser.close();
    server.close();
  }
}

export async function openPage(browser, url, path, vp, theme) {
  const ctx = await newThemedContext(browser, vp, theme);
  const page = await ctx.newPage();
  await page.goto(url + path);
  await settle(page, theme);
  return {page, close: () => ctx.close()};
}

export async function axeRule(page, rule) {
  const r = await new AxeBuilder({page}).withRules([rule]).analyze();
  return r.violations.flatMap((v) => v.nodes.map((n) => n.target.join(' ')));
}
