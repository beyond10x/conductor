// The value axis of a chart: from zero to a round top, in round steps; whole steps for counts and
// 0–100 for a percentage.

export interface Scale {
  max: number;
  ticks: number[];
}

const STEPS = [1, 2, 2.5, 5, 10];

function whole(unit: string): boolean {
  return unit === 'count' || unit === 'tokens';
}

/** A round step near a quarter of `max`. */
function step(max: number, unit: string): number {
  const rough = max / 4;
  const power = 10 ** Math.floor(Math.log10(rough));
  for (const candidate of STEPS) {
    const size = candidate * power;
    if (size >= rough - 1e-9 && (!whole(unit) || Number.isInteger(size))) {
      return whole(unit) ? Math.max(1, size) : size;
    }
  }
  return 10 * power;
}

export function niceScale(max: number, unit: string): Scale {
  if (unit === 'percent') {
    return { max: 100, ticks: [0, 25, 50, 75, 100] };
  }
  if (!(max > 0)) {
    const size = whole(unit) ? 1 : 0.25;
    return { max: size * 4, ticks: [0, size, size * 2, size * 3, size * 4] };
  }
  const size = step(max, unit);
  const top = Math.ceil(max / size - 1e-9) * size;
  const ticks: number[] = [];
  for (let at = 0; at <= top + 1e-9; at += size) {
    ticks.push(Math.round(at * 1e6) / 1e6);
  }
  return { max: top, ticks };
}
