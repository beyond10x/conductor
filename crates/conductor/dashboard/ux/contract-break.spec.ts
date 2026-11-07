// The layout contract's break-tests: each check of ux/contract.ts fails on a page broken on
// purpose, so a green run of ux/layout.spec.ts is a pass and not a probe that sees nothing.
import { test, expect } from '@playwright/test';
import { SCENARIOS } from './adapter';
import { assertions, axe, enter, probe } from './contract';

const overview = SCENARIOS.find((s) => s.name === 'overview')!;
const loaded = overview.states.find((s) => s.name === 'loaded')!;

test.beforeEach(async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 800 });
  await enter(page, overview, loaded, 'light');
});

test('the intact overview passes the layout checks', async ({ page }) => {
  const measured = await probe(page);
  expect(measured.pageScrollX).toBe(0);
  expect(measured.overflow).toEqual([]);
  expect(measured.notVisible).toEqual([]);
  expect(measured.words).toBeGreaterThan(50);
});

test('a block wider than the page scrolls it sideways and runs past its box', async ({ page }) => {
  await page.evaluate(() => {
    const wide = document.createElement('div');
    wide.className = 'ux-break-wide';
    wide.style.width = '3000px';
    wide.style.height = '10px';
    document.querySelector('main')!.appendChild(wide);
  });
  const measured = await probe(page);
  expect(measured.pageScrollX).toBeGreaterThan(0);
  expect(measured.overflow.some((path) => path.startsWith('main'))).toBe(true);
  expect(assertions('break', measured, []).some((a) => a.startsWith('no-scroll:break:'))).toBe(true);
});

test('a layer over the navigation covers it', async ({ page }) => {
  await page.evaluate(() => {
    const layer = document.createElement('div');
    layer.className = 'ux-break-layer';
    Object.assign(layer.style, { position: 'fixed', inset: '0 0 auto 0', height: '120px', zIndex: '99' });
    document.body.appendChild(layer);
  });
  const measured = await probe(page);
  expect(measured.notVisible.some((why) => why.startsWith('page navigation') && why.includes('covered by div.ux-break-layer'))).toBe(true);
});

test('a header pushed down is outside the viewport', async ({ page }) => {
  await page.evaluate(() => window.scrollTo(0, 400));
  const measured = await probe(page);
  expect(measured.notVisible.some((why) => why.startsWith('header') && why.endsWith('outside the viewport'))).toBe(true);
});

test('a missing instance selector is reported missing', async ({ page }) => {
  await page.evaluate(() => document.querySelector('header.top .instance-select')!.remove());
  const measured = await probe(page);
  expect(measured.notVisible).toContain('instance selector: missing');
});

test('axe-core reports a serious violation on an unlabelled button', async ({ page }) => {
  await page.evaluate(() => {
    const button = document.createElement('button');
    button.type = 'button';
    document.querySelector('main')!.appendChild(button);
  });
  const violations = await axe(page);
  expect(violations.map((v) => v.id)).toContain('button-name');
});
