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
      // `/api/unanswered`: two requests escalated as O or H; the oldest raised two hours ago.
      'unanswered O/H': ['2', 'oldest 2h · REQ-login'],
      // The same two open items, from awaiting_operator: one count of what asks something of
      // the operator (story:sessions-page-coherent); red CI is its own tile.
      'needs you': ['2', '2 items to answer or do'],
      'red CI on main': ['2', 'alpha, delta'],
      // ex-alpha and ex-beta are the live controllers: ex-alpha busy, ex-beta idle.
      controllers: ['2', '1 busy · 1 idle · 0 silent'],
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
    const alpha = view.data!.sessions.find((session) => session.name === 'ex-alpha')!;
    alpha.dispatch!.event_at = '2026-10-13T08:30:00Z';
    const controllers = summary(view).find((tile) => tile.label === 'controllers')!;
    expect([controllers.value, controllers.sub]).toEqual(['2', '0 busy · 1 idle · 1 silent']);
  });

  it('counts "needs you" and "unanswered O/H" alike, red CI apart', () => {
    const view = overview();
    const tiles = Object.fromEntries(summary(view).map((tile) => [tile.label, tile.value]));
    expect(tiles['needs you']).toBe(tiles['unanswered O/H']);
    view.data!.awaiting_operator = [];
    view.unanswered = { count: 0, oldest: null };
    const none = Object.fromEntries(summary(view).map((tile) => [tile.label, tile.value]));
    expect([none['needs you'], none['unanswered O/H']]).toEqual(['0', '0']);
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

  it('shows no unanswered O/H item as 0 and an unread count as a dash', () => {
    const view = overview();
    view.unanswered = { count: 0, oldest: null };
    const none = summary(view).find((tile) => tile.label === 'unanswered O/H')!;
    expect([none.value, none.sub, none.tone]).toEqual(['0', 'none', 'green']);
    view.unanswered = null;
    const unread = summary(view).find((tile) => tile.label === 'unanswered O/H')!;
    expect(unread.value).toBe('—');
  });
});
