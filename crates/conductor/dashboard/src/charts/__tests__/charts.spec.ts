// story:dashboard-charts-ui: each chart, drawn as inline SVG from the fixture series, with its
// axis labels, its legend and a tooltip on hover and on focus.
import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import LineChart from '../LineChart.vue';
import StackedBars from '../StackedBars.vue';
import ChartPanel from '../ChartPanel.vue';
import { chartData, passRate } from '../../api/series';
import { niceScale } from '../scale';
import { SERIES_NOW, seriesAnswer, seriesList } from '../../views/__tests__/series-fixtures';

const merged = () => chartData(seriesAnswer('prs_merged', 'repository'), SERIES_NOW, 7);
const lead = () => chartData(seriesAnswer('dispatch_lead_hours', null), SERIES_NOW, 7);

describe('the scale', () => {
  it('rounds the top up to a whole step, whole steps for counts, 100 for a rate', () => {
    expect(niceScale(7, 'count')).toEqual({ max: 8, ticks: [0, 2, 4, 6, 8] });
    expect(niceScale(0, 'count')).toEqual({ max: 4, ticks: [0, 1, 2, 3, 4] });
    expect(niceScale(9.5, 'hours').max).toBe(10);
    expect(niceScale(80, 'percent')).toEqual({ max: 100, ticks: [0, 25, 50, 75, 100] });
  });

  it('reaches past 100 % for a share that can exceed it: a run over its reservation', () => {
    expect(niceScale(100, 'percent')).toEqual({ max: 100, ticks: [0, 25, 50, 75, 100] });
    expect(niceScale(150, 'percent')).toEqual({ max: 150, ticks: [0, 50, 100, 150] });
    const disk = chartData(
      seriesAnswer('disk_peak_vs_reserved', 'repository'),
      SERIES_NOW,
      7,
    );
    const wrapper = mount(LineChart, { props: { data: disk, label: 'Disk peak against reserved' } });
    for (const dot of wrapper.findAll('circle.dot')) {
      expect(Number(dot.attributes('cy'))).toBeGreaterThanOrEqual(0);
    }
    expect(wrapper.findAll('.y-axis text').map((text) => text.text())).toContain('150%');
    wrapper.unmount();
  });
});

describe('every series of the fixtures', () => {
  it('draws, as bars when it counts and as lines otherwise', () => {
    for (const info of seriesList()) {
      if (info.source === null) {
        continue;
      }
      const by = info.by.includes('repository') ? 'repository' : (info.by[0] ?? null);
      const data = chartData(seriesAnswer(info.series, by), SERIES_NOW, 30);
      const wrapper = mount(ChartPanel, {
        props: { title: info.title, view: { state: 'ok', data } },
      });
      const svg = wrapper.find('svg');
      expect(svg.exists(), info.series).toBe(true);
      const drawn = info.unit === 'count' ? 'rect.bar' : 'path.line';
      expect(wrapper.findAll(drawn).length, info.series).toBeGreaterThan(0);
    }
  });
});

describe('the stacked bars', () => {
  it('stack one bar per key and day with a value, under a legend naming each key', () => {
    const wrapper = mount(StackedBars, { props: { data: merged(), label: 'PRs merged' } });
    // alpha on three days, beta on two.
    expect(wrapper.findAll('rect.bar')).toHaveLength(5);
    expect(wrapper.findAll('.legend li').map((item) => item.text())).toEqual(['alpha', 'beta']);
    expect(wrapper.find('svg').attributes('role')).toBe('img');
    expect(wrapper.find('svg').attributes('aria-label')).toContain('PRs merged');
  });

  it('labels the axes: the days below, the counts beside', () => {
    const wrapper = mount(StackedBars, { props: { data: merged(), label: 'PRs merged' } });
    const x = wrapper.findAll('.x-axis text').map((text) => text.text());
    expect(x[0]).toBe('Oct 7');
    expect(x[x.length - 1]).toBe('Oct 13');
    // The highest day stacks 5; the axis rounds up to 6.
    expect(wrapper.findAll('.y-axis text').map((text) => text.text())).toEqual([
      '0',
      '2',
      '4',
      '6',
    ]);
  });

  it('shows the day, each key and the total on hover', async () => {
    const wrapper = mount(StackedBars, { props: { data: merged(), label: 'PRs merged' } });
    expect(wrapper.find('.tip').exists()).toBe(false);
    await wrapper.findAll('rect.hit')[6]!.trigger('mouseenter');
    const tip = wrapper.find('.tip').text();
    expect(tip).toContain('Oct 13');
    expect(tip).toContain('alpha: 2');
    expect(tip).toContain('beta: 3');
    expect(tip).toContain('total: 5');
    await wrapper.find('svg').trigger('mouseleave');
    expect(wrapper.find('.tip').exists()).toBe(false);
  });

  it('shows the newest day on focus and steps through the days with the arrow keys', async () => {
    const wrapper = mount(StackedBars, { props: { data: merged(), label: 'PRs merged' } });
    const svg = wrapper.find('svg');
    expect(svg.attributes('tabindex')).toBe('0');
    await svg.trigger('focus');
    expect(wrapper.find('.tip').text()).toContain('Oct 13');
    await svg.trigger('keydown', { key: 'ArrowLeft' });
    expect(wrapper.find('.tip').text()).toContain('Oct 12');
    expect(wrapper.find('.tip').text()).toContain('alpha: 4');
    await svg.trigger('keydown', { key: 'Home' });
    expect(wrapper.find('.tip').text()).toContain('Oct 7');
    // What the tooltip says is said to a screen reader too.
    expect(wrapper.find('[aria-live="polite"]').text()).toContain('Oct 7');
    await svg.trigger('blur');
    expect(wrapper.find('.tip').exists()).toBe(false);
  });
});

describe('the line chart', () => {
  it('draws one line per key, broken where a day has no value', () => {
    const wrapper = mount(LineChart, { props: { data: lead(), label: 'Dispatch sent to done' } });
    const lines = wrapper.findAll('path.line');
    expect(lines).toHaveLength(2);
    // Oct 7..10 have no dispatch: the line starts on Oct 11 with a move, not a line from zero.
    expect(lines[0]?.attributes('d')?.startsWith('M')).toBe(true);
    expect(lines[0]?.attributes('d')?.match(/M/g)).toHaveLength(1);
    expect(wrapper.findAll('.legend li').map((item) => item.text())).toEqual(['p50', 'p90']);
  });

  it('labels the hours axis in hours and writes the tooltip per unit', async () => {
    const wrapper = mount(LineChart, { props: { data: lead(), label: 'Dispatch sent to done' } });
    expect(wrapper.findAll('.y-axis text').map((text) => text.text())).toContain('10.0 h');
    await wrapper.findAll('rect.hit')[5]!.trigger('mouseenter');
    const tip = wrapper.find('.tip').text();
    expect(tip).toContain('Oct 12');
    expect(tip).toContain('p50: 30 min');
    expect(tip).toContain('p90: 3.0 h');
    await wrapper.findAll('rect.hit')[0]!.trigger('mouseenter');
    expect(wrapper.find('.tip').text()).toContain('p50: —');
  });

  it('draws the CI pass rate in percent', () => {
    const passed = chartData(seriesAnswer('ci_passed', null), SERIES_NOW, 7);
    const failed = chartData(seriesAnswer('ci_failed', null), SERIES_NOW, 7);
    const wrapper = mount(LineChart, { props: { data: passRate(passed, failed), label: 'CI pass rate' } });
    expect(wrapper.findAll('path.line')).toHaveLength(1);
    expect(wrapper.findAll('.y-axis text').map((text) => text.text())).toEqual([
      '0%',
      '25%',
      '50%',
      '75%',
      '100%',
    ]);
  });
});

describe('a chart panel', () => {
  it('says it is reading, names a series without a source and its story, and names an error', () => {
    const reading = mount(ChartPanel, { props: { title: 'PRs merged', view: { state: 'loading' } } });
    expect(reading.find('.state-loading').text()).toContain('reading');
    const none = mount(ChartPanel, {
      props: { title: 'Output tokens', view: { state: 'no-source', story: 'story:kpi-transcripts' } },
    });
    expect(none.find('.state-no-source').text()).toBe('no source yet · story:kpi-transcripts');
    expect(none.find('svg').exists()).toBe(false);
    const failed = mount(ChartPanel, {
      props: { title: 'PRs merged', view: { state: 'error', message: 'could not read x: 500' } },
    });
    expect(failed.find('.state-error').text()).toBe('could not read x: 500');
    expect(failed.find('.state-error').attributes('role')).toBe('alert');
  });

  it('says so when the range has no value at all', () => {
    const data = chartData(seriesAnswer('prs_merged', 'repository'), SERIES_NOW, 7, () => false);
    const empty = mount(ChartPanel, { props: { title: 'PRs merged', view: { state: 'ok', data } } });
    expect(empty.find('.state-empty').text()).toBe('nothing in this range');
    const zeros = chartData({ ...seriesAnswer('prs_merged', 'repository'), rows: [] }, SERIES_NOW, 7);
    expect(
      mount(ChartPanel, { props: { title: 'PRs merged', view: { state: 'ok', data: zeros } } })
        .find('.state-empty')
        .exists(),
    ).toBe(true);
  });
});
