// story:dashboard-kpi-panels, the app half: one panel per KPI of `/api/kpis` (fixtures/kpis.json),
// a measured KPI with its value against its target, its trend from `/api/series` and its source,
// and a KPI not measured yet with its story and no chart and no number.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import KpiPanel from '../kpi/KpiPanel.vue';
import Kpis from '../Kpis.vue';
import KpiStrip from '../overview/KpiStrip.vue';
import { loadKpis, standing, trendKey, type Kpi } from '../../api/kpis';
import { chartData } from '../../api/series';
import { SERIES_NOW, kpis, seriesAnswer, seriesFetch } from './series-fixtures';

function kpi(id: string): Kpi {
  const found = kpis().find((row) => row.id === id);
  if (found === undefined) {
    throw new Error(`no KPI ${id}`);
  }
  return found;
}

const trend = (series: string) => ({
  state: 'ok' as const,
  data: chartData(seriesAnswer(series, null), SERIES_NOW, 30),
});

describe('reading the KPIs', () => {
  it('answers every row in order, K1–K12 then T1–T4', async () => {
    const read = await loadKpis(seriesFetch());
    expect(read.error).toBeNull();
    expect(read.kpis.map((row) => row.id)).toEqual([
      'K1', 'K2', 'K3', 'K4', 'K5', 'K6', 'K7', 'K8', 'K9', 'K10', 'K11', 'K12', 'T1', 'T2', 'T3', 'T4',
    ]);
  });

  it('names an answer it could not read', async () => {
    const read = await loadKpis(vi.fn(async () => new Response('down', { status: 503 })));
    expect(read).toEqual({ kpis: [], error: 'could not read /api/kpis: 503' });
  });
});

describe('a value against its target', () => {
  it('meets a lower-is-better target at or below it and misses it above', () => {
    expect(standing(kpi('K1'))).toBe('missed');
    expect(standing(kpi('K2'))).toBe('met');
    expect(standing({ ...kpi('K2'), value: 10 })).toBe('met');
  });

  it('reads a higher-is-better target the other way', () => {
    expect(standing({ ...kpi('K2'), direction: 'higher', value: 6 })).toBe('missed');
    expect(standing({ ...kpi('K2'), direction: 'higher', value: 12 })).toBe('met');
  });

  it('has no standing without a target or without a value', () => {
    expect(standing({ ...kpi('K1'), target: null })).toBe('no-target');
    expect(standing(kpi('K11'))).toBe('not-measured');
  });
});

describe('the key of a trend', () => {
  it('is p90 for a percentile series and the total otherwise', () => {
    expect(trendKey(['p50', 'p90'])).toBe('p90');
    expect(trendKey(['total'])).toBe('total');
    expect(trendKey([])).toBeNull();
  });
});

describe('a KPI panel', () => {
  it('shows a measured KPI that misses its target: value, target, the miss, trend and source', () => {
    const wrapper = mount(KpiPanel, { props: { kpi: kpi('K1'), trend: trend('operator_touches_per_pr') } });
    expect(wrapper.find('h3').text()).toContain('K1');
    expect(wrapper.find('h3').text()).toContain('Operator touches per merged PR');
    expect(wrapper.find('.value').text()).toBe('0.25');
    expect(wrapper.find('.target').text()).toBe('target ≤ 0.10');
    expect(wrapper.find('.standing').text()).toBe('misses target');
    expect(wrapper.classes()).toContain('missed');
    expect(wrapper.find('svg').exists()).toBe(true);
    expect(wrapper.findAll('path.line')).toHaveLength(1);
    expect(wrapper.find('.source').text()).toBe('source: decision store + forge snapshot · last 7 days');
  });

  it('shows a measured KPI that meets its target', () => {
    const wrapper = mount(KpiPanel, {
      props: { kpi: kpi('K2'), trend: trend('unnecessary_escalations_pct') },
    });
    expect(wrapper.find('.value').text()).toBe('6%');
    expect(wrapper.find('.target').text()).toBe('target ≤ 10%');
    expect(wrapper.find('.standing').text()).toBe('meets target');
    expect(wrapper.classes()).toContain('met');
  });

  it('writes hours in hours and a target not set yet', () => {
    const wrapper = mount(KpiPanel, {
      props: { kpi: { ...kpi('K4'), target: null }, trend: trend('dispatch_lead_hours') },
    });
    expect(wrapper.find('.value').text()).toBe('9.5 h');
    expect(wrapper.find('.target').text()).toBe('target set after first measurement');
    expect(wrapper.find('.standing').exists()).toBe(false);
    // The trend of a percentile series draws its p90 only.
    expect(wrapper.findAll('path.line')).toHaveLength(1);
  });

  it('says its trend is being read, or names why it could not be', () => {
    const reading = mount(KpiPanel, { props: { kpi: kpi('K10'), trend: { state: 'loading' } } });
    expect(reading.find('.value').text()).toBe('3');
    expect(reading.find('.state-loading').exists()).toBe(true);
    const failed = mount(KpiPanel, {
      props: { kpi: kpi('K10'), trend: { state: 'error', message: 'could not read x: 500' } },
    });
    expect(failed.find('.state-error').text()).toBe('could not read x: 500');
    const none = mount(KpiPanel, {
      props: { kpi: kpi('K10'), trend: { state: 'no-source', story: 'story:dashboard-series-api' } },
    });
    expect(none.find('.state-no-source').text()).toBe('no source yet · story:dashboard-series-api');
  });

  it('shows a KPI not measured yet with its story, and no chart and no number', () => {
    const wrapper = mount(KpiPanel, { props: { kpi: kpi('K11'), trend: null } });
    expect(wrapper.find('.not-measured').text()).toBe('not measured yet · story:kpi-transcripts');
    expect(wrapper.find('svg').exists()).toBe(false);
    expect(wrapper.find('.value').exists()).toBe(false);
    expect(wrapper.find('.standing').exists()).toBe(false);
    expect(wrapper.text()).not.toMatch(/\d+(\.\d+)?\s*(tokens|%|h)\b/);
  });
});

describe('the KPI page', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(SERIES_NOW);
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('shows one panel per KPI and reads the series of each measured one', async () => {
    const asked: string[] = [];
    vi.stubGlobal('fetch', seriesFetch({}, asked));
    const wrapper = mount(Kpis);
    expect(wrapper.find('.state-loading').exists()).toBe(true);
    await flushPromises();
    expect(wrapper.find('h1').text()).toBe('KPIs');
    expect(wrapper.findAll('.kpi')).toHaveLength(16);
    expect(wrapper.findAll('.kpi.not-measured-kpi')).toHaveLength(9);
    expect(wrapper.findAll('.kpi svg')).toHaveLength(7);
    expect(asked).toContain('/api/series?series=dispatch_lead_hours&days=30');
    expect(asked.filter((url) => url.startsWith('/api/series?'))).toHaveLength(7);
    wrapper.unmount();
  });

  it('names /api/kpis when it could not be read', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('down', { status: 503 })));
    const wrapper = mount(Kpis);
    await flushPromises();
    expect(wrapper.find('.state-error').text()).toBe('could not read /api/kpis: 503');
    wrapper.unmount();
  });

  it('says so when there is no KPI', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => Response.json([])));
    const wrapper = mount(Kpis);
    await flushPromises();
    expect(wrapper.find('.state-empty').text()).toBe('no KPI answered');
    wrapper.unmount();
  });
});

describe('the KPI strip of the overview', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(SERIES_NOW);
  });
  afterEach(() => {
    vi.useRealTimers();
    vi.unstubAllGlobals();
  });

  it('shows one compact panel per KPI and links the KPI page', async () => {
    vi.stubGlobal('fetch', seriesFetch());
    const wrapper = mount(KpiStrip);
    await flushPromises();
    expect(wrapper.findAll('.kpi')).toHaveLength(16);
    expect(wrapper.findAll('.kpi.compact')).toHaveLength(16);
    expect(wrapper.find('a[href="#/kpis"]').exists()).toBe(true);
    wrapper.unmount();
  });
});
