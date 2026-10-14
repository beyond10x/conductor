// story:dashboard-charts-ui: the Metrics page, `#/metrics`, one chart per series of `/api/series`
// (fixtures/series.json), filtered by repository and over 7, 30 or 90 days.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Metrics from '../Metrics.vue';
import App from '../../App.vue';
import { SERIES_NOW, seriesFetch, seriesList } from './series-fixtures';

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
  vi.setSystemTime(SERIES_NOW);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  window.location.hash = '';
});

/** The chart panel titled `title`. */
function panel(wrapper: ReturnType<typeof mount>, title: string) {
  const found = wrapper.findAll('.chart-panel').find((at) => at.find('h2').text() === title);
  if (found === undefined) {
    throw new Error(`no panel ${title}`);
  }
  return found;
}

describe('the Metrics page', () => {
  it('draws one chart per series with a source, and names the story of one without', async () => {
    const asked: string[] = [];
    vi.stubGlobal('fetch', seriesFetch({}, asked));
    const wrapper = mount(Metrics);
    expect(wrapper.find('.state-loading').exists()).toBe(true);
    await flushPromises();
    expect(wrapper.find('h1').text()).toBe('Metrics');
    expect(wrapper.findAll('.chart-panel')).toHaveLength(seriesList().length);
    expect(asked).toContain('/api/series');
    expect(asked).toContain('/api/series?series=prs_merged&by=repository&days=30');
    expect(asked).toContain('/api/series?series=ci_passed&by=repository&days=30');
    expect(asked).toContain('/api/series?series=guard_denials&by=rule&days=30');
    expect(asked).toContain('/api/series?series=operator_touches&days=30');
    expect(panel(wrapper, 'PRs merged').findAll('rect.bar').length).toBeGreaterThan(0);
    expect(panel(wrapper, 'Dispatch sent to done').findAll('path.line')).toHaveLength(2);
    expect(panel(wrapper, 'Output tokens').find('.state-no-source').text()).toBe(
      'no source yet · story:kpi-transcripts',
    );
    expect(panel(wrapper, 'Output tokens').find('svg').exists()).toBe(false);
    wrapper.unmount();
  });

  it('draws the runs\' disk peak against reserved, a line per repository (story:dashboard-disk-series)', async () => {
    const asked: string[] = [];
    vi.stubGlobal('fetch', seriesFetch({}, asked));
    const wrapper = mount(Metrics);
    await flushPromises();
    expect(asked).toContain('/api/series?series=disk_peak_vs_reserved&by=repository&days=30');
    const disk = panel(wrapper, 'Disk peak against reserved');
    expect(disk.find('.scope').text()).toBe('by repository');
    expect(disk.find('.source').text()).toBe(
      'source: host ledger: released runs, recorded peak over bytes reserved',
    );
    expect(disk.findAll('path.line')).toHaveLength(2);
    expect(disk.findAll('.legend li').map((item) => item.text())).toEqual(['alpha', 'beta']);
    expect(disk.find('rect.bar').exists()).toBe(false);
    await wrapper.find('select#metrics-repository').setValue('beta');
    expect(panel(wrapper, 'Disk peak against reserved').findAll('path.line')).toHaveLength(1);
    wrapper.unmount();
  });

  it('reads every series again over the range chosen', async () => {
    const asked: string[] = [];
    vi.stubGlobal('fetch', seriesFetch({}, asked));
    const wrapper = mount(Metrics);
    await flushPromises();
    const ranges = wrapper.findAll('.range button');
    expect(ranges.map((button) => button.text())).toEqual(['7 days', '30 days', '90 days']);
    expect(ranges[1]?.attributes('aria-pressed')).toBe('true');
    await ranges[0]!.trigger('click');
    await flushPromises();
    expect(asked).toContain('/api/series?series=prs_merged&by=repository&days=7');
    expect(wrapper.findAll('.range button')[0]?.attributes('aria-pressed')).toBe('true');
    // Seven days, seven columns to hover.
    expect(panel(wrapper, 'PRs merged').findAll('rect.hit')).toHaveLength(7);
    await wrapper.findAll('.range button')[2]!.trigger('click');
    await flushPromises();
    expect(panel(wrapper, 'PRs merged').findAll('rect.hit')).toHaveLength(90);
    wrapper.unmount();
  });

  it('keeps the repository chosen in each chart grouped by repository', async () => {
    vi.stubGlobal('fetch', seriesFetch());
    const wrapper = mount(Metrics);
    await flushPromises();
    const select = wrapper.find('select#metrics-repository');
    expect(select.findAll('option').map((option) => option.text())).toEqual([
      'all repositories',
      'alpha',
      'beta',
    ]);
    expect(panel(wrapper, 'PRs merged').findAll('.legend li').map((item) => item.text())).toEqual([
      'alpha',
      'beta',
    ]);
    await select.setValue('beta');
    expect(panel(wrapper, 'PRs merged').findAll('.legend li').map((item) => item.text())).toEqual([
      'beta',
    ]);
    expect(panel(wrapper, 'Operator touches').find('.scope').text()).toBe('all repositories');
    wrapper.unmount();
  });

  it('names the list it could not read', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('down', { status: 503 })));
    const wrapper = mount(Metrics);
    await flushPromises();
    expect(wrapper.find('.state-error').text()).toBe('could not read /api/series: 503');
    wrapper.unmount();
  });

  it('says so when no series is listed', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => Response.json([])));
    const wrapper = mount(Metrics);
    await flushPromises();
    expect(wrapper.find('.state-empty').text()).toBe('no series answered');
    wrapper.unmount();
  });

  it('is in the navigation and routed at #/metrics', async () => {
    window.location.hash = '#/metrics';
    vi.stubGlobal('fetch', seriesFetch());
    const wrapper = mount(App);
    await flushPromises();
    const link = wrapper.find('nav a[href="#/metrics"]');
    expect(link.text()).toBe('Metrics');
    expect(link.attributes('aria-current')).toBe('page');
    expect(wrapper.find('nav a[href="#/kpis"]').text()).toBe('KPIs');
    expect(wrapper.findComponent(Metrics).exists()).toBe(true);
    wrapper.unmount();
  });
});
