// story:dashboard-charts-ui: the small trend charts of the overview (merges, CI pass rate, operator
// touches) and of the repository page (its merges, CI runs passed and failed, releases).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Trends from '../overview/Trends.vue';
import RepoTrends from '../repo/RepoTrends.vue';
import { SERIES_NOW, seriesFetch } from './series-fixtures';

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
  vi.setSystemTime(SERIES_NOW);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

function titles(wrapper: ReturnType<typeof mount>): string[] {
  return wrapper.findAll('.chart-panel h3').map((heading) => heading.text());
}

describe('the overview trends', () => {
  it('draw merges and operator touches as bars and the CI pass rate as a line, over 30 days', async () => {
    const asked: string[] = [];
    vi.stubGlobal('fetch', seriesFetch({}, asked));
    const wrapper = mount(Trends);
    expect(wrapper.findAll('.state-loading')).toHaveLength(3);
    await flushPromises();
    expect(wrapper.find('h2').text()).toBe('Trends · 30 days');
    expect(titles(wrapper)).toEqual(['PRs merged', 'CI pass rate', 'Operator touches']);
    expect(asked.sort()).toEqual([
      '/api/series?series=ci_failed&days=30',
      '/api/series?series=ci_passed&days=30',
      '/api/series?series=operator_touches&days=30',
      '/api/series?series=prs_merged&days=30',
    ]);
    const panels = wrapper.findAll('.chart-panel');
    expect(panels[0]?.findAll('rect.bar')).toHaveLength(3);
    expect(panels[1]?.findAll('path.line')).toHaveLength(1);
    expect(panels[2]?.findAll('rect.bar')).toHaveLength(3);
    expect(wrapper.find('a[href="#/metrics"]').exists()).toBe(true);
    wrapper.unmount();
  });

  it('names a series it could not read on its own chart, and draws the others', async () => {
    const fetch = seriesFetch();
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) =>
        String(input).includes('operator_touches')
          ? new Response('nope', { status: 500 })
          : fetch(input),
      ),
    );
    const wrapper = mount(Trends);
    await flushPromises();
    const panels = wrapper.findAll('.chart-panel');
    expect(panels[2]?.find('.state-error').text()).toBe(
      'could not read /api/series?series=operator_touches&days=30: 500',
    );
    expect(panels[0]?.find('svg').exists()).toBe(true);
    wrapper.unmount();
  });

  it('shows no pass rate when either half has no source', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) =>
        String(input).includes('ci_')
          ? Response.json({ error: 'no source yet', story: 'story:dashboard-history' }, { status: 404 })
          : seriesFetch()(input),
      ),
    );
    const wrapper = mount(Trends);
    await flushPromises();
    expect(wrapper.findAll('.chart-panel')[1]?.find('.state-no-source').text()).toBe(
      'no source yet · story:dashboard-history',
    );
    wrapper.unmount();
  });
});

describe('the repository trends', () => {
  it("draw that repository's merges, CI runs passed and failed, and releases", async () => {
    const asked: string[] = [];
    vi.stubGlobal('fetch', seriesFetch({}, asked));
    const wrapper = mount(RepoTrends, { props: { name: 'alpha' } });
    await flushPromises();
    expect(titles(wrapper)).toEqual(['PRs merged', 'CI runs', 'Releases']);
    expect(asked.sort()).toEqual([
      '/api/series?series=ci_failed&by=repository&days=30',
      '/api/series?series=ci_passed&by=repository&days=30',
      '/api/series?series=prs_merged&by=repository&days=30',
      '/api/series?series=releases&by=repository&days=30',
    ]);
    const panels = wrapper.findAll('.chart-panel');
    // alpha merged on three days; beta's merges are not alpha's.
    expect(panels[0]?.findAll('rect.bar')).toHaveLength(3);
    expect(panels[1]?.findAll('.legend li').map((item) => item.text())).toEqual(['passed', 'failed']);
    // alpha: passed on Oct 11, 12, 13; failed on Oct 12.
    expect(panels[1]?.findAll('rect.bar')).toHaveLength(4);
    expect(panels[2]?.findAll('rect.bar')).toHaveLength(2);
    wrapper.unmount();
  });

  it('says a repository with nothing in the range has nothing to draw', async () => {
    vi.stubGlobal('fetch', seriesFetch());
    const wrapper = mount(RepoTrends, { props: { name: 'gamma' } });
    await flushPromises();
    expect(wrapper.findAll('.state-empty')).toHaveLength(3);
    wrapper.unmount();
  });
});
