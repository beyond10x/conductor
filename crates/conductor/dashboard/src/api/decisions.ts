// What the Decisions page reads (story:decisions-page): `GET /api/decisions`, every open class-O
// and class-H request with what the operator chooses from, oldest first, and the records'
// decision log, newest first. The server computes it once per write and answers with an `ETag`;
// a reader that sends it back in `If-None-Match` is answered 304 and keeps what it had. The
// fixture src/views/__tests__/fixtures/decisions.json carries these keys
// (crates/conductor/tests/dashboard_overview.rs holds it to the served shape).

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

export type DecisionKind = 'yes-no' | 'one-of' | 'several-of';

export interface DecisionOption {
  letter: string;
  label: string;
  cost: string;
}

export interface OpenDecision {
  request_id: string;
  class: 'O' | 'H';
  repository: string;
  question: string;
  /** `one-of` for a request escalated before its kind was recorded. */
  kind: DecisionKind;
  /** Empty for a request escalated before its options were recorded. */
  options: DecisionOption[];
  recommended: string[];
  default: string[];
  deadline: string | null;
  raised_at: string;
  reason: string | null;
  /** The request's own words, as the controller raised it. */
  options_text: string;
  recommendation_text: string;
}

export interface HistoryRow {
  at: string | null;
  event: string | null;
  id: string | null;
  repo: string | null;
  class: string | null;
  decided_by: string | null;
  question: string | null;
  choice: string | null;
  reason: string | null;
}

export interface DecisionsAnswer {
  open: OpenDecision[];
  history: HistoryRow[];
  errors: string[];
}

export interface DecisionsRead {
  answer: DecisionsAnswer | null;
  etag: string | null;
  error: string | null;
}

const PATH = '/api/decisions';

/**
 * Reads `/api/decisions`. Given the last read, it sends that read's `ETag` and keeps its answer
 * when the server says nothing changed.
 */
export async function loadDecisions(
  fetcher: Fetch = (input, init) => fetch(input, init),
  last: DecisionsRead | null = null,
): Promise<DecisionsRead> {
  const headers: Record<string, string> = {};
  if (last?.etag && last.answer !== null) {
    headers['If-None-Match'] = last.etag;
  }
  try {
    const answer = await fetcher(PATH, { cache: 'no-store', headers });
    if (answer.status === 304 && last !== null && last.answer !== null) {
      return { answer: last.answer, etag: last.etag, error: null };
    }
    if (!answer.ok) {
      return { answer: null, etag: null, error: `could not read ${PATH}: ${answer.status}` };
    }
    return {
      answer: (await answer.json()) as DecisionsAnswer,
      etag: answer.headers.get('ETag'),
      error: null,
    };
  } catch (error) {
    return { answer: null, etag: null, error: `could not read ${PATH}: ${String(error)}` };
  }
}

/** The open items, the one raised first first, then by id. */
export function oldestFirst(open: OpenDecision[]): OpenDecision[] {
  return [...open].sort(
    (left, right) =>
      Date.parse(left.raised_at) - Date.parse(right.raised_at) ||
      left.request_id.localeCompare(right.request_id),
  );
}

/** The history, newest first; a row whose instant does not read last. */
export function newestFirst(history: HistoryRow[]): HistoryRow[] {
  const instant = (row: HistoryRow) => (row.at === null ? Number.NaN : Date.parse(row.at));
  return [...history].sort((left, right) => {
    const [a, b] = [instant(left), instant(right)];
    if (Number.isNaN(a) || Number.isNaN(b)) {
      return Number.isNaN(a) ? (Number.isNaN(b) ? 0 : 1) : -1;
    }
    return b - a;
  });
}

/** What the answer to a request of this kind is, as its control says it. */
export function kindNote(kind: DecisionKind): string {
  switch (kind) {
    case 'yes-no':
      return 'yes or no';
    case 'one-of':
      return 'choose one';
    case 'several-of':
      return 'choose any, at least one';
  }
}

/** An instant as the page writes it, in UTC: `2026-10-13 18:00Z`. */
export function stamp(at: string): string {
  const instant = new Date(at);
  return Number.isNaN(instant.getTime())
    ? at
    : `${instant.toISOString().slice(0, 10)} ${instant.toISOString().slice(11, 16)}Z`;
}
