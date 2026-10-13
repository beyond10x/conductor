import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Overview from '../Overview.vue';
import { loadOverview, startOfDayUtc } from '../../api/overview';
import { NOW, SINCE, answers } from './overview-fixtures';

/** A `fetch` answering the fixtures by path, recording each URL it is asked. */
function fakeFetch(asked: string[], failing: string[] = []) {
  const all = answers();
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    asked.push(url);
    const path = url.split('?')[0]!;
    if (failing.includes(path) || !(path in all)) {
      return new Response('nope', { status: 500 });
    }
    return new Response(JSON.stringify(all[path]), {
      status: 200,
      headers: { 'Content-Type': 'application/json' },
    });
  });
}

describe('startOfDayUtc', () => {
  it('is 00:00 UTC of the day in UTC, whatever the local zone', () => {
    expect(startOfDayUtc(new Date('2026-10-13T23:59:59Z'))).toBe(SINCE);
    expect(startOfDayUtc(new Date('2026-10-13T01:30:00+02:00'))).toBe('2026-10-12T00:00:00Z');
  });
});

describe('loadOverview', () => {
  it('reads each endpoint, the events since 00:00 UTC', async () => {
    const asked: string[] = [];
    const view = await loadOverview(NOW, fakeFetch(asked));
    expect(asked).toContain(`/api/events?since=${encodeURIComponent(SINCE)}`);
    for (const path of [
      '/data.json',
      '/api/board',
      '/api/repositories',
      '/api/pull-requests',
      '/api/workflow-runs',
      '/api/releases',
    ]) {
      expect(asked).toContain(path);
    }
    expect(view.since).toBe(SINCE);
    expect(view.board.rows).toHaveLength(4);
    expect(view.events).toHaveLength(5);
    expect(view.errors).toEqual([]);
  });

  it('names an endpoint that could not be read and keeps the others', async () => {
    const view = await loadOverview(NOW, fakeFetch([], ['/api/board']));
    expect(view.errors).toEqual(['/api/board: 500']);
    expect(view.board).toEqual({ snapshot: null, rows: [] });
    expect(view.releases.rows).toHaveLength(3);
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

  it('shows the summary, needs-you cards, repositories, today, and disk and spend collapsed', async () => {
    const asked: string[] = [];
    vi.stubGlobal('fetch', fakeFetch(asked));
    const wrapper = mount(Overview);
    await flushPromises();

    expect(asked).toContain('/api/events?since=2026-10-13T00%3A00%3A00Z');
    expect(wrapper.findAll('.tiles .tile')).toHaveLength(8);
    expect(wrapper.findAll('.needs .card')).toHaveLength(3);
    expect(wrapper.findAll('table tbody tr')).toHaveLength(4);
    expect(wrapper.findAll('ol.timeline li')).toHaveLength(5);
    const collapsed = wrapper.find('details.disk-spend');
    expect(collapsed.exists()).toBe(true);
    expect(collapsed.attributes('open')).toBeUndefined();
    expect(collapsed.find('summary').text()).toBe('Disk & spend · 33.6 GiB free · 5.8k tokens today');
    expect(wrapper.text()).toContain('snapshot of 2026-10-13T09:00:00Z');
  });

  it('names what could not be read', async () => {
    vi.stubGlobal('fetch', fakeFetch([], ['/api/events']));
    const wrapper = mount(Overview);
    await flushPromises();
    expect(wrapper.find('.read-errors').text()).toContain('/api/events: 500');
  });
});
