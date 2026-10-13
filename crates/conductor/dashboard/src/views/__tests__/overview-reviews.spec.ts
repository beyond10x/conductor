// story:review-limit: the overview's first panel lists the pull requests that wait for review,
// oldest first, with their links, from `/api/reviews`.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Overview from '../Overview.vue';
import WaitingForReview from '../overview/WaitingForReview.vue';
import { age, loadOverview, waitingForReview } from '../../api/overview';
import { NOW, answers, overview } from './overview-fixtures';

/** A `fetch` answering the fixtures by path. */
function fakeFetch(all: Record<string, unknown> = answers()) {
  return vi.fn(async (input: RequestInfo | URL) => {
    const path = String(input).split('?')[0]!;
    if (!(path in all)) {
      return new Response('nope', { status: 500 });
    }
    return new Response(JSON.stringify(all[path]), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
  });
}

describe('loadOverview', () => {
  it('reads the pull requests that wait for review from /api/reviews', async () => {
    const asked: string[] = [];
    const fetcher = fakeFetch();
    const view = await loadOverview(NOW, async (input) => {
      asked.push(String(input));
      return fetcher(input);
    });
    expect(asked).toContain('/api/reviews');
    expect(view.reviews.rows.map((row) => `${row.repository} #${row.number}`)).toEqual([
      'beta #3',
      'alpha #9',
    ]);
  });
});

describe('waitingForReview', () => {
  it('is oldest first, whatever order the rows come in', () => {
    const view = overview();
    view.reviews.rows.reverse();
    expect(waitingForReview(view).map((row) => row.number)).toEqual([3, 9]);
  });
});

describe('age', () => {
  it('is the largest whole unit between opening and the snapshot', () => {
    expect(age('2026-10-11T09:00:00Z', '2026-10-13T09:00:00Z')).toBe('2d');
    expect(age('2026-10-12T15:00:00Z', '2026-10-13T09:00:00Z')).toBe('18h');
    expect(age('2026-10-13T08:20:00Z', '2026-10-13T09:00:00Z')).toBe('40m');
    expect(age('2026-10-13T08:20:00Z', null)).toBe('2026-10-13T08:20:00Z');
  });
});

describe('WaitingForReview', () => {
  it('lists each pull request oldest first, linked where it has a link', () => {
    const view = overview();
    view.reviews.rows.reverse();
    const wrapper = mount(WaitingForReview, {
      props: { rows: waitingForReview(view), at: view.reviews.snapshot!.started_at },
    });
    const items = wrapper.findAll('li');
    expect(items.map((item) => item.text())).toEqual([
      'beta #3 Read the beta feed — 2d',
      'alpha #9 Name the tiles — 18h',
    ]);
    expect(items[0]!.find('a').exists()).toBe(false);
    expect(items[1]!.find('a').attributes('href')).toBe(
      'https://forge.example.org/example-org/alpha/pull/9',
    );
    expect(wrapper.find('h2').text()).toBe('Waiting for review · 2');
  });

  it('says so when nothing waits', () => {
    const wrapper = mount(WaitingForReview, { props: { rows: [], at: null } });
    expect(wrapper.findAll('li')).toHaveLength(0);
    expect(wrapper.text()).toContain('nothing waits for review');
  });
});

describe('Overview', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(NOW);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('opens with the waiting-for-review panel, from the endpoint, in age order', async () => {
    const all = answers();
    (all['/api/reviews'] as { rows: unknown[] }).rows.reverse();
    vi.stubGlobal('fetch', fakeFetch(all));
    const wrapper = mount(Overview);
    await flushPromises();
    const first = wrapper.find('section');
    expect(first.attributes('aria-labelledby')).toBe('reviews');
    expect(first.findAll('li').map((item) => item.text())).toEqual([
      'beta #3 Read the beta feed — 2d',
      'alpha #9 Name the tiles — 18h',
    ]);
  });
});
