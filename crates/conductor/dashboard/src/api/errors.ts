// Which of a page's read errors belong to a panel: the ones naming an endpoint the panel reads.
// The overview names an error `<path>: <why>`, the repository page `could not read <path>: <why>`;
// a panel shows either as `could not read <path>: <why>`.

/** The errors of `errors` that name one of `paths`, as one line, or null when there is none. */
export function errorFor(errors: string[], paths: string[]): string | null {
  const mine = errors
    .map((error) => error.replace(/^could not read /, ''))
    .filter((error) => paths.some((path) => error.startsWith(`${path}:`) || error.startsWith(`${path}?`)))
    .map((error) => `could not read ${error}`);
  return mine.length === 0 ? null : mine.join(' · ');
}
