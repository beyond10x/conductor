// How the dashboard writes a value, as the page `conductor dashboard serve` served before the app.

const KIB = 1024;
const MIB = KIB * 1024;
const GIB = MIB * 1024;

/** A value as a table cell shows it: a string as it is, null as `—`, anything else as JSON. */
export function cell(value: unknown): string {
  if (value === null || value === undefined) {
    return '—';
  }
  if (typeof value === 'string') {
    return value;
  }
  return JSON.stringify(value);
}

/** Bytes in GiB with one decimal, or `—`. */
export function gib(bytes: number | null | undefined): string {
  return typeof bytes === 'number' ? `${(bytes / GIB).toFixed(1)} GiB` : '—';
}

/** Bytes in GiB, MiB or KiB, whichever is the largest below them, or `—`. */
export function size(bytes: number | null | undefined): string {
  if (typeof bytes !== 'number') {
    return '—';
  }
  if (bytes >= GIB) {
    return `${(bytes / GIB).toFixed(1)} GiB`;
  }
  if (bytes >= MIB) {
    return `${(bytes / MIB).toFixed(1)} MiB`;
  }
  return `${(bytes / KIB).toFixed(0)} KiB`;
}

/** A count of tokens as the config writes it: in thousands with `k` when it is whole thousands. */
export function tokens(count: number | null | undefined): string {
  if (typeof count !== 'number') {
    return cell(count);
  }
  return count !== 0 && count % 1000 === 0 ? `${count / 1000}k` : String(count);
}

/** A percentage with one decimal, as the server rounds it. */
export function percent(share: number | null | undefined): string {
  return typeof share === 'number' ? share.toFixed(1) : '—';
}
