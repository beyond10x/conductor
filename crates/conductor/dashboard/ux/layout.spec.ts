// L2, the layout contract and axe-core (skill § 4, gates 1, 2 and 5; story:dashboard-ui-loop): at
// each gating viewport, in each theme, over every state of every scenario. A failed assertion
// fails the merge unless ux/baseline.json lists it as known, with the finding that records it; the
// ratchet (ux/ratchet.ts) fails when a known one no longer fails, so the list only shrinks.
// The measured-only viewports run when UX_ROUND is set (the round command), recorded, not gated.
import { test, expect } from '@playwright/test';
import { GATING, MEASURED, SCENARIOS, THEMES } from './adapter';
import { assertions, axe, enter, probe, record } from './contract';
import { known } from './baseline';

const round = process.env.UX_ROUND !== undefined;
const viewports = [...GATING.map((v) => ({ ...v, gated: true })), ...(round ? MEASURED.map((v) => ({ ...v, gated: false })) : [])];

for (const scenario of SCENARIOS) {
  for (const state of scenario.states) {
    for (const viewport of viewports) {
      for (const theme of THEMES) {
        const where = `${scenario.name}/${state.name}@${viewport.name}/${theme}`;
        test(`layout ${where}`, async ({ page }) => {
          await page.setViewportSize({ width: viewport.width, height: viewport.height });
          await enter(page, scenario, state, theme);
          const measured = await probe(page);
          const violations = await axe(page);
          const failed = assertions(where, measured, violations);
          // The round's shots: every state at 1280x800, of which the round command keeps a few.
          if (round && viewport.name === '1280x800') {
            await page.screenshot({ path: `ux/.out/shots/${scenario.name}-${state.name}-${theme}.png` });
          }
          record(`layout-${where}`, {
            kind: 'layout',
            scenario: scenario.name,
            state: state.name,
            viewport: viewport.name,
            theme,
            gated: viewport.gated,
            probe: measured,
            axe: violations,
            failed,
          });
          if (viewport.gated) {
            const unknown = failed.filter((assertion) => known(assertion) === null);
            expect(unknown, `${where}: the layout contract`).toEqual([]);
          }
        });
      }
    }
  }
}
