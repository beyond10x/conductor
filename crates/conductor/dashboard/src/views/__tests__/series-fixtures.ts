// `/api/series` as the contract of story:dashboard-series-api answers it, from fixtures/series.json:
// the list at `/api/series`, one answer per `series` and `by`, and 404 with the story for a series
// that has no source yet.
import { vi } from 'vitest';
import raw from './fixtures/series.json';
import kpiRows from './fixtures/kpis.json';
import type { SeriesAnswer, SeriesInfo } from '../../api/series';
import type { Kpi } from '../../api/kpis';

/** `/api/kpis` as the contract of story:dashboard-kpi-panels answers it. */
export function kpis(): Kpi[] {
  return structuredClone(kpiRows) as Kpi[];
}

/** The instant the chart tests run at: the window of every range ends on 2026-10-13 UTC. */
export const SERIES_NOW = new Date('2026-10-13T09:30:00Z');

export function seriesList(): SeriesInfo[] {
  return structuredClone(raw.list) as SeriesInfo[];
}

export function seriesAnswer(series: string, by: string | null): SeriesAnswer {
  const answer = (raw.answers as Record<string, unknown>)[`${series}|${by ?? ''}`];
  if (answer === undefined) {
    throw new Error(`no fixture for ${series} by ${by ?? 'total'}`);
  }
  return structuredClone(answer) as SeriesAnswer;
}

/** What `/api/series` answers for `url`, or null when the fixtures do not cover it. */
export function seriesResponse(url: string): Response | null {
  const parsed = new URL(url, 'http://dashboard.invalid');
  if (parsed.pathname === '/api/kpis') {
    return Response.json(kpiRows);
  }
  if (parsed.pathname !== '/api/series') {
    return null;
  }
  const series = parsed.searchParams.get('series');
  if (series === null) {
    return Response.json(raw.list);
  }
  const info = raw.list.find((entry) => entry.series === series);
  if (info === undefined) {
    return Response.json({ error: `unknown series ${series}` }, { status: 400 });
  }
  if (info.source === null) {
    return Response.json({ error: 'no source yet', story: info.story }, { status: 404 });
  }
  const by = parsed.searchParams.get('by');
  const answer = (raw.answers as Record<string, unknown>)[`${series}|${by ?? ''}`];
  if (answer === undefined) {
    return Response.json({ error: `no fixture for ${series} by ${by ?? 'total'}` }, { status: 400 });
  }
  const days = Number(parsed.searchParams.get('days') ?? '30');
  return Response.json({ ...(answer as object), days });
}

/** A `fetch` answering `/api/series` from the fixtures, `others` by path, 500 for the rest. */
export function seriesFetch(others: Record<string, unknown> = {}, asked: string[] = []) {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    asked.push(url);
    const series = seriesResponse(url);
    if (series !== null) {
      return series;
    }
    if (url in others) {
      return Response.json(others[url]);
    }
    return new Response('nope', { status: 500 });
  });
}
