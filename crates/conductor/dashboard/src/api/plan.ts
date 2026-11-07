// What the Plan page reads and writes (story:dashboard-plan-board). `GET /api/plan` answers
// `conductor.direction.PlanBoard`: the checkouts that hold a planning store, the one shown and its
// stories as cards, each in its column and lane. `GET /api/plan/story` answers one story as the
// drawer shows it (`conductor.direction.PlanStory`). `POST /api/plan` takes
// `conductor.direction.WritePlan` with the page's token, through the dashboard's one write path
// (the Configuration page's): the server runs the one `aep plan artifact` verb the action needs,
// and records it as the operator's decision; a move the store's lifecycle refuses comes back
// `MoveRefused` with the store's own reason, and nothing changed.
//
// The grouping and the labels live here, apart from the view, so Vitest holds them.

import { pageToken } from './decisions';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

export type Column = 'Draft' | 'Proposed' | 'Accepted' | 'InProgress' | 'Done';

export interface PlanCard {
  id: string;
  title: string;
  status: string;
  column: Column;
  lane: string;
  important: boolean;
  started: boolean;
}

export interface PlanBoard {
  repositories: string[];
  repository: string;
  cards: PlanCard[];
  errors: string[];
}

export interface AcceptanceLine {
  text: string;
  evidence: boolean;
}

export interface PlanEvent {
  at: string;
  actor: string;
  what: string;
}

export interface PlanStory {
  id: string;
  title: string;
  status: string;
  column: Column | null;
  why: string;
  acceptance: AcceptanceLine[];
  scope: string[];
  history: PlanEvent[];
  moves: string[];
  comments: string;
  important: boolean;
  started: boolean;
}

export type Action = 'Move' | 'Start' | 'Important' | 'Unimportant' | 'Comment';

export interface PlanWrite {
  repository: string;
  id: string;
  action: Action;
  to?: string;
  text?: string;
}

export interface PlanWritten {
  id: string;
  status: string;
  decision_id: string;
  detail: string;
}

export const PLAN_PATH = '/api/plan';
export const STORY_PATH = '/api/plan/story';

/** The columns, in board order: a name, and what a story there is. */
export const COLUMNS: { column: Column; name: string; hint: string }[] = [
  { column: 'Draft', name: 'Draft', hint: 'ideas, not yet asked for' },
  { column: 'Proposed', name: 'Proposed', hint: 'waiting for your yes' },
  { column: 'Accepted', name: 'Accepted · next', hint: 'approved, not started' },
  { column: 'InProgress', name: 'In progress', hint: 'being worked on' },
  { column: 'Done', name: 'Done', hint: 'implemented' },
];

/** The columns whose cards are grouped by wave. */
const LANED: Column[] = ['Proposed', 'Accepted', 'InProgress'];

/** The order of lanes: `next`, then each wave by number, then `unplanned`. */
function laneRank(lane: string): number {
  if (lane === 'next') return 0;
  const wave = /^wave (\d+)$/.exec(lane);
  if (wave !== null) return Number(wave[1]);
  return Number.MAX_SAFE_INTEGER;
}

/** The cards of `column` that match `query` (title or id), important ones first. */
export function cardsOf(board: PlanBoard, column: Column, query = ''): PlanCard[] {
  const wanted = query.trim().toLowerCase();
  return board.cards
    .filter((card) => card.column === column)
    .filter((card) => wanted === '' || `${card.title} ${card.id}`.toLowerCase().includes(wanted))
    .sort((a, b) => Number(b.important) - Number(a.important));
}

/** The cards of `column` in lanes, when the column is grouped and grouping is on; one unnamed lane else. */
export function lanesOf(
  board: PlanBoard,
  column: Column,
  query = '',
  grouped = true,
): { lane: string | null; cards: PlanCard[] }[] {
  const cards = cardsOf(board, column, query);
  if (!grouped || !LANED.includes(column)) {
    return [{ lane: null, cards }];
  }
  const lanes = new Map<string, PlanCard[]>();
  for (const card of cards) {
    const lane = card.lane === '' ? 'unplanned' : card.lane;
    lanes.set(lane, [...(lanes.get(lane) ?? []), card]);
  }
  return [...lanes.entries()]
    .sort(([a], [b]) => laneRank(a) - laneRank(b))
    .map(([lane, inLane]) => ({ lane, cards: inLane }));
}

/** The label of the button that moves a story to `to`. */
export function moveLabel(to: string): string {
  switch (to) {
    case 'active':
      return 'Approve → Accepted';
    case 'rejected':
      return 'Reject';
    case 'proposed':
      return 'Propose';
    case 'draft':
      return 'Back to draft';
    case 'implemented':
      return 'Mark done';
    case 'archived':
      return 'Archive';
    default:
      return `Move to ${to}`;
  }
}

/** The write a card dropped on `column` asks for; none when no move of its lifecycle lands there. */
export function dropWrite(story: PlanCard, column: Column, moves: string[] | null, repository: string): PlanWrite | null {
  if (story.column === column) return null;
  const base = { repository, id: story.id };
  if (column === 'InProgress') {
    return story.column === 'Accepted' ? { ...base, action: 'Start' } : null;
  }
  const status: Record<Exclude<Column, 'InProgress'>, string> = {
    Draft: 'draft',
    Proposed: 'proposed',
    Accepted: 'active',
    Done: 'implemented',
  };
  const to = status[column];
  if (moves !== null && !moves.includes(to)) return null;
  return { ...base, action: 'Move', to };
}

async function readJson<T>(fetcher: Fetch, path: string): Promise<{ value: T | null; error: string | null }> {
  try {
    const answer = await fetcher(path, { cache: 'no-store' });
    if (!answer.ok) {
      return { value: null, error: `could not read ${path}: ${answer.status} ${(await answer.text()).trim()}` };
    }
    return { value: (await answer.json()) as T, error: null };
  } catch (error) {
    return { value: null, error: `could not read ${path}: ${String(error)}` };
  }
}

/** Reads the board of `repository`, else the first that holds a planning store. */
export async function loadBoard(
  repository: string | null,
  fetcher: Fetch = (input, init) => fetch(input, init),
): Promise<{ board: PlanBoard | null; error: string | null }> {
  const path = repository === null ? PLAN_PATH : `${PLAN_PATH}?repository=${encodeURIComponent(repository)}`;
  const { value, error } = await readJson<PlanBoard>(fetcher, path);
  return { board: value, error };
}

/** Reads one story of `repository`. */
export async function loadStory(
  repository: string,
  id: string,
  fetcher: Fetch = (input, init) => fetch(input, init),
): Promise<{ story: PlanStory | null; error: string | null }> {
  const path = `${STORY_PATH}?repository=${encodeURIComponent(repository)}&id=${encodeURIComponent(id)}`;
  const { value, error } = await readJson<PlanStory>(fetcher, path);
  return { story: value, error };
}

/** Sends one write with the page's token; what the store did, or why it refused. */
export async function sendPlan(
  write: PlanWrite,
  fetcher: Fetch = (input, init) => fetch(input, init),
  token: string | null = pageToken(),
): Promise<{ written: PlanWritten | null; refusal: string | null; error: string | null }> {
  if (token === null || token === '') {
    return { written: null, refusal: null, error: 'the page carries no token; reload it' };
  }
  try {
    const answer = await fetcher(PLAN_PATH, {
      method: 'POST',
      cache: 'no-store',
      credentials: 'same-origin',
      headers: { 'Content-Type': 'application/json', 'X-Conductor-Token': token },
      body: JSON.stringify(write),
    });
    const text = await answer.text();
    if (!answer.ok) {
      try {
        const refused = JSON.parse(text) as { refusal: string; detail: string };
        return { written: null, refusal: refused.refusal, error: refused.detail };
      } catch {
        return { written: null, refusal: null, error: `${answer.status} ${text.trim()}` };
      }
    }
    return { written: JSON.parse(text) as PlanWritten, refusal: null, error: null };
  } catch (error) {
    return { written: null, refusal: null, error: String(error) };
  }
}
