import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import SummaryTiles from '../overview/SummaryTiles.vue';
import { summary } from '../../api/overview';
import { overview } from './overview-fixtures';

/** The tiles as `label → [value, sub]`, as the strip shows them. */
function shown(wrapper: ReturnType<typeof mount>): Record<string, [string, string]> {
  const tiles: Record<string, [string, string]> = {};
  for (const tile of wrapper.findAll('.tile')) {
    tiles[tile.find('.label').text()] = [tile.find('.val').text(), tile.find('.sub').text()];
  }
  return tiles;
}

describe('SummaryTiles', () => {
  it('shows each figure of the strip from the fixtures', () => {
    const view = overview();
    const wrapper = mount(SummaryTiles, { props: { tiles: summary(view) } });
    expect(shown(wrapper)).toEqual({
      // One awaiting_operator row and two repositories with a red main (alpha, delta).
      'needs you': ['3', '1 decision · 2 red CI'],
      'red CI on main': ['2', 'alpha, delta'],
      // alpha is the one live controller: busy, its last report 5 min before generated_at.
      controllers: ['1', '1 busy · 0 idle · 0 silent'],
      // The board's open_prs and failing_prs, summed.
      'open PRs / MRs': ['3', '1 failing'],
      // alpha v1.1.0 and delta v2.0.0 are published since 00:00 UTC; beta's is of yesterday.
      'shipped today': ['2', '2 releases · 1 merge'],
      context: ['1', 'over 300k tokens'],
      'free disk': ['33.6 GiB', 'of 847.0 GiB'],
      // input + output over the usage rows: 1200 + 3400 + 300 + 900.
      'spend today': ['5.8k tokens', '19 calls'],
    });
  });

  it('counts a live controller whose last report is old as silent', () => {
    const view = overview();
    const alpha = view.data!.sessions.find((session) => session.name === 'alpha')!;
    alpha.dispatch!.event_at = '2026-10-13T08:30:00Z';
    const controllers = summary(view).find((tile) => tile.label === 'controllers')!;
    expect([controllers.value, controllers.sub]).toEqual(['1', '0 busy · 0 idle · 1 silent']);
  });

  it('shows a dash for each figure an endpoint did not answer', () => {
    const view = overview();
    view.data = null;
    view.board = { snapshot: null, rows: [] };
    const tiles = Object.fromEntries(summary(view).map((tile) => [tile.label, tile.value]));
    expect(tiles['free disk']).toBe('—');
    expect(tiles['spend today']).toBe('—');
    expect(tiles['open PRs / MRs']).toBe('—');
  });
});
