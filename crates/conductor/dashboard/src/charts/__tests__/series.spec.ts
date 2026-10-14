// story:dashboard-charts-ui: how the app reads `/api/series` and turns an answer into what a chart
// draws, over the fixtures of the contract (src/views/__tests__/fixtures/series.json).
import { describe, expect, it } from 'vitest';
import {
  chartData,
  chartKind,
  dayWindow,
  formatValue,
  loadSeries,
  loadSeriesList,
  passRate,
  seriesUrl,
} from '../../api/series';
import { SERIES_NOW, seriesAnswer, seriesFetch } from '../../views/__tests__/series-fixtures';

describe('the series URL', () => {
  it('names the series, the group and the days, and leaves the group out for the total', () => {
    expect(seriesUrl('prs_merged', 'repository', 30)).toBe(
      '/api/series?series=prs_merged&by=repository&days=30',
    );
    expect(seriesUrl('operator_touches', null, 7)).toBe('/api/series?series=operator_touches&days=7');
  });
});

describe('reading a series', () => {
  it('lists the series', async () => {
    const read = await loadSeriesList(seriesFetch());
    expect(read.error).toBeNull();
    expect(read.list.map((info) => info.series)).toContain('tokens_out');
  });

  it('answers the rows of a series with a source', async () => {
    const read = await loadSeries('prs_merged', 'repository', 30, seriesFetch());
    expect(read.state).toBe('ok');
    expect(read.state === 'ok' && read.answer.rows).toHaveLength(5);
  });

  it('names the story of a series that answers 404', async () => {
    expect(await loadSeries('tokens_out', null, 30, seriesFetch())).toEqual({
      state: 'no-source',
      story: 'story:kpi-transcripts',
    });
  });

  it('names an answer it could not read', async () => {
    const read = await loadSeries('prs_merged', 'workflow', 30, seriesFetch());
    expect(read).toEqual({
      state: 'error',
      message: 'could not read /api/series?series=prs_merged&by=workflow&days=30: 400',
    });
  });
});

describe('the days of a range', () => {
  it('ends on the UTC day of now, oldest first', () => {
    expect(dayWindow(SERIES_NOW, 7)).toEqual([
      '2026-10-07',
      '2026-10-08',
      '2026-10-09',
      '2026-10-10',
      '2026-10-11',
      '2026-10-12',
      '2026-10-13',
    ]);
    expect(dayWindow(new Date('2026-10-13T23:30:00-05:00'), 1)).toEqual(['2026-10-14']);
    expect(dayWindow(SERIES_NOW, 90)).toHaveLength(90);
  });
});

describe('what a chart draws', () => {
  it('fills the days without a row with zero for a count, one line per key', () => {
    const data = chartData(seriesAnswer('prs_merged', 'repository'), SERIES_NOW, 7);
    expect(data.unit).toBe('count');
    expect(data.lines.map((line) => line.key)).toEqual(['alpha', 'beta']);
    expect(data.lines[0]?.values).toEqual([0, 0, 2, 0, 0, 4, 2]);
    expect(data.lines[1]?.values).toEqual([0, 0, 1, 0, 0, 0, 3]);
  });

  it('keeps only the keys asked for', () => {
    const data = chartData(seriesAnswer('prs_merged', 'repository'), SERIES_NOW, 7, (key) => key === 'beta');
    expect(data.lines.map((line) => line.key)).toEqual(['beta']);
  });

  it('leaves a day without a row empty for hours: no dispatch is not zero hours', () => {
    const data = chartData(seriesAnswer('dispatch_lead_hours', null), SERIES_NOW, 4);
    expect(data.lines.map((line) => line.key)).toEqual(['p50', 'p90']);
    expect(data.lines[0]?.values).toEqual([null, 1.5, 0.5, 2]);
  });

  it('computes the CI pass rate from the runs passed and failed, empty where none ran', () => {
    const passed = chartData(seriesAnswer('ci_passed', null), SERIES_NOW, 4);
    const failed = chartData(seriesAnswer('ci_failed', null), SERIES_NOW, 4);
    const rate = passRate(passed, failed);
    expect(rate.unit).toBe('percent');
    expect(rate.lines).toHaveLength(1);
    expect(rate.lines[0]?.values).toEqual([null, 100, 80, 80]);
  });

  it('draws counts as stacked bars and rates and hours as lines', () => {
    expect(chartKind('count')).toBe('bars');
    expect(chartKind('tokens')).toBe('bars');
    expect(chartKind('hours')).toBe('line');
    expect(chartKind('percent')).toBe('line');
  });
});

describe('a value, per unit', () => {
  it('writes counts whole, hours in hours or minutes, tokens compact, rates in percent', () => {
    expect(formatValue(1234, 'count')).toBe('1,234');
    expect(formatValue(6.25, 'hours')).toBe('6.3 h');
    expect(formatValue(0.5, 'hours')).toBe('30 min');
    expect(formatValue(950, 'tokens')).toBe('950');
    expect(formatValue(12_340, 'tokens')).toBe('12.3k');
    expect(formatValue(2_500_000, 'tokens')).toBe('2.5M');
    expect(formatValue(80, 'percent')).toBe('80%');
    expect(formatValue(null, 'count')).toBe('—');
  });
});
