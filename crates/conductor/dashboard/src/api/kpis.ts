// What the KPI panels read (story:dashboard-kpi-panels): `GET /api/kpis`, one row per KPI in the
// order K1–K12, then T1–T4. A measured row carries its value over the last `window_days`, its
// target and which way is better, the `/api/series` series of its daily value, and its source; a
// row not measured yet carries the story that adds its source, and no value. The fixture
// src/views/__tests__/fixtures/kpis.json carries these keys.

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

export interface Kpi {
  id: string;
  title: string;
  definition: string;
  unit: string;
  /** Null until it is set after the first measurement. */
  target: number | null;
  /** Which way is better. */
  direction: 'lower' | 'higher';
  measured: boolean;
  value?: number | null;
  window_days?: number;
  series?: string;
  source?: string;
  story?: string;
}

export async function loadKpis(
  fetcher: Fetch = (input, init) => fetch(input, init),
): Promise<{ kpis: Kpi[]; error: string | null }> {
  try {
    const answer = await fetcher('/api/kpis', { cache: 'no-store' });
    if (!answer.ok) {
      return { kpis: [], error: `could not read /api/kpis: ${answer.status}` };
    }
    return { kpis: (await answer.json()) as Kpi[], error: null };
  } catch (error) {
    return { kpis: [], error: `could not read /api/kpis: ${String(error)}` };
  }
}

export type Standing = 'met' | 'missed' | 'no-target' | 'not-measured';

/** Where a KPI's value stands against its target, read the way `direction` says is better. */
export function standing(kpi: Kpi): Standing {
  if (!kpi.measured || kpi.value === null || kpi.value === undefined) {
    return 'not-measured';
  }
  if (kpi.target === null) {
    return 'no-target';
  }
  const met = kpi.direction === 'higher' ? kpi.value >= kpi.target : kpi.value <= kpi.target;
  return met ? 'met' : 'missed';
}

/**
 * The key of a KPI's series its trend draws: the 90th percentile of a percentile series (K4, K5
 * are p90), the total of any other, else its only key.
 */
export function trendKey(keys: string[]): string | null {
  if (keys.includes('p90')) {
    return 'p90';
  }
  if (keys.includes('total')) {
    return 'total';
  }
  return keys[0] ?? null;
}
