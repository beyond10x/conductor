// What the charts read (story:dashboard-charts-ui): `/api/series`, as story:dashboard-series-api
// answers it.
// - `GET /api/series`: the list, `[{series, title, unit, by, kpi, source, story?}]`;
// - `GET /api/series?series=<name>&by=<key>&days=<n>`: `{series, by, days, unit, rows}`, a row
//   `{day, key, value}` per UTC day and group (`key` is `total` without `by`); a day without data
//   has no row;
// - a series with no source answers 404 `{error: "no source yet", story}`.
// The fixtures src/views/__tests__/fixtures/series.json carry these keys.

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const defaultFetch: Fetch = (input, init) => fetch(input, init);

export interface SeriesInfo {
  series: string;
  title: string;
  unit: string;
  by: string[];
  kpi: string | null;
  source: string | null;
  story?: string;
}

export interface SeriesRow {
  day: string;
  key: string;
  value: number;
}

export interface SeriesAnswer {
  series: string;
  by: string | null;
  days: number;
  unit: string;
  rows: SeriesRow[];
}

/** One read of a series: its rows, the story that adds its source, or why it could not be read. */
export type SeriesRead =
  | { state: 'ok'; answer: SeriesAnswer }
  | { state: 'no-source'; story: string | null }
  | { state: 'error'; message: string };

/** One line of a chart: a value per day of the range, null where there is none. */
export interface Line {
  key: string;
  label: string;
  values: (number | null)[];
  /** A colour that means something (passed, failed) instead of the series' place. */
  tone?: 'good' | 'bad';
}

export interface ChartData {
  /** The UTC days of the range, oldest first. */
  days: string[];
  unit: string;
  lines: Line[];
}

/** What a chart panel shows. */
export type PanelView =
  | { state: 'loading' }
  | { state: 'no-source'; story: string | null }
  | { state: 'error'; message: string }
  | { state: 'ok'; data: ChartData };

/** The ranges a page offers, in days. */
export const RANGES = [7, 30, 90] as const;

export function seriesUrl(series: string, by: string | null, days: number): string {
  const query = new URLSearchParams({ series });
  if (by !== null) {
    query.set('by', by);
  }
  query.set('days', String(days));
  return `/api/series?${query.toString()}`;
}

export async function loadSeriesList(
  fetcher: Fetch = defaultFetch,
): Promise<{ list: SeriesInfo[]; error: string | null }> {
  try {
    const answer = await fetcher('/api/series', { cache: 'no-cache' });
    if (!answer.ok) {
      return { list: [], error: `could not read /api/series: ${answer.status}` };
    }
    return { list: (await answer.json()) as SeriesInfo[], error: null };
  } catch (error) {
    return { list: [], error: `could not read /api/series: ${String(error)}` };
  }
}

export async function loadSeries(
  series: string,
  by: string | null,
  days: number,
  fetcher: Fetch = defaultFetch,
): Promise<SeriesRead> {
  const url = seriesUrl(series, by, days);
  try {
    const answer = await fetcher(url, { cache: 'no-cache' });
    if (answer.status === 404) {
      let story: string | null = null;
      try {
        const body = (await answer.json()) as { story?: unknown };
        story = typeof body.story === 'string' ? body.story : null;
      } catch {
        story = null;
      }
      return { state: 'no-source', story };
    }
    if (!answer.ok) {
      return { state: 'error', message: `could not read ${url}: ${answer.status}` };
    }
    return { state: 'ok', answer: (await answer.json()) as SeriesAnswer };
  } catch (error) {
    return { state: 'error', message: `could not read ${url}: ${String(error)}` };
  }
}

const DAY_MS = 86_400_000;

/** The `days` UTC days that end on the UTC day of `now`, oldest first. */
export function dayWindow(now: Date, days: number): string[] {
  const today = Date.UTC(now.getUTCFullYear(), now.getUTCMonth(), now.getUTCDate());
  const window: string[] = [];
  for (let back = days - 1; back >= 0; back -= 1) {
    window.push(new Date(today - back * DAY_MS).toISOString().slice(0, 10));
  }
  return window;
}

/** A unit whose absent day is a zero: something counted. Any other is a measure, absent is none. */
function counted(unit: string): boolean {
  return unit === 'count' || unit === 'tokens';
}

/** How a chart draws a unit: counts stack as bars, rates and durations are lines. */
export function chartKind(unit: string): 'bars' | 'line' {
  return counted(unit) ? 'bars' : 'line';
}

/**
 * What a chart draws of `answer` over the `days` ending today: one line per key (sorted, `keep`
 * choosing which), a value per day, zero for a count on a day without a row and none for a measure.
 */
export function chartData(
  answer: SeriesAnswer,
  now: Date,
  days: number,
  keep: (key: string) => boolean = () => true,
): ChartData {
  const window = dayWindow(now, days);
  const at = new Map(window.map((day, index) => [day, index]));
  const fill = counted(answer.unit) ? 0 : null;
  const lines = new Map<string, (number | null)[]>();
  for (const row of answer.rows) {
    const index = at.get(row.day);
    if (index === undefined || !keep(row.key)) {
      continue;
    }
    let values = lines.get(row.key);
    if (values === undefined) {
      values = window.map(() => fill);
      lines.set(row.key, values);
    }
    values[index] = (values[index] ?? 0) + row.value;
  }
  return {
    days: window,
    unit: answer.unit,
    lines: [...lines.keys()]
      .sort((one, two) => one.localeCompare(two))
      .map((key) => ({ key, label: key, values: lines.get(key)! })),
  };
}

/** The sum of every line on each day. */
function totals(data: ChartData): number[] {
  return data.days.map((_, index) =>
    data.lines.reduce((sum, line) => sum + (line.values[index] ?? 0), 0),
  );
}

/** The CI pass rate per day, in percent, from the runs passed and failed; none where none ran. */
export function passRate(passed: ChartData, failed: ChartData): ChartData {
  const good = totals(passed);
  const bad = totals(failed);
  return {
    days: passed.days,
    unit: 'percent',
    lines: [
      {
        key: 'pass rate',
        label: 'pass rate',
        values: good.map((count, index) => {
          const runs = count + (bad[index] ?? 0);
          return runs === 0 ? null : Math.round((count / runs) * 1000) / 10;
        }),
      },
    ],
  };
}

/** Whether a chart has anything to draw: one value that is not empty, and for counts not zero. */
export function hasValues(data: ChartData): boolean {
  return data.lines.some((line) =>
    line.values.some((value) => value !== null && (!counted(data.unit) || value !== 0)),
  );
}

const COUNT = new Intl.NumberFormat('en', { maximumFractionDigits: 0 });

/** A value as its unit writes it. */
export function formatValue(value: number | null | undefined, unit: string): string {
  if (value === null || value === undefined || !Number.isFinite(value)) {
    return '—';
  }
  switch (unit) {
    case 'count':
      return COUNT.format(value);
    case 'tokens':
      if (Math.abs(value) >= 1_000_000) {
        return `${trim(value / 1_000_000)}M`;
      }
      if (Math.abs(value) >= 1000) {
        return `${trim(value / 1000)}k`;
      }
      return COUNT.format(value);
    case 'hours':
      return Math.abs(value) < 1 && value !== 0
        ? `${Math.round(value * 60)} min`
        : `${value.toFixed(1)} h`;
    case 'percent':
      return `${Number.isInteger(value) ? value : value.toFixed(1)}%`;
    case 'ratio':
      return value.toFixed(2);
    case 'USD':
      return `$${value.toFixed(2)}`;
    default:
      return String(Math.round(value * 100) / 100);
  }
}

/** One decimal, without a trailing `.0`. */
function trim(value: number): string {
  return value.toFixed(1).replace(/\.0$/, '');
}

/** A UTC day as an axis writes it: `Oct 13`. */
export function shortDay(day: string): string {
  const [year, month, date] = day.split('-').map(Number);
  const at = new Date(Date.UTC(year ?? 1970, (month ?? 1) - 1, date ?? 1));
  return at.toLocaleDateString('en', { month: 'short', day: 'numeric', timeZone: 'UTC' });
}

/** The view a chart panel shows for a read, `draw` turning its answer into a chart. */
export function panelView(read: SeriesRead | null, draw: (answer: SeriesAnswer) => ChartData): PanelView {
  if (read === null) {
    return { state: 'loading' };
  }
  if (read.state === 'ok') {
    return { state: 'ok', data: draw(read.answer) };
  }
  return read;
}
