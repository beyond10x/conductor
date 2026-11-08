// story:dashboard-plan-board: the Plan page shows one planning store as columns Draft,
// Proposed, Accepted (next), In progress and Done, grouped by wave; a card opens the story in a
// drawer with its Why, Acceptance (a tick per evidenced line), Scope, History and a comment box;
// its buttons send one write each with the page's token, and a refused move shows the store's
// reason and leaves the board as it was.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import App from '../src/App.vue';
import { fixture } from './fixture';
import Plan from '../src/views/Plan.vue';
import {
  dropWrite,
  lanesOf,
  moveLabel,
  sendPlan,
  type PlanBoard,
  type PlanCard,
  type PlanStory,
} from '../src/api/plan';

function card(id: string, column: PlanCard['column'], lane = '', important = false): PlanCard {
  const status = { Draft: 'draft', Proposed: 'proposed', Accepted: 'active', InProgress: 'active', Done: 'implemented' }[
    column
  ];
  return { id, title: `Title of ${id}`, status, column, lane, important, started: column === 'InProgress' };
}

const BOARD: PlanBoard = {
  repositories: ['alpha', 'beta'],
  repository: 'alpha',
  cards: [
    card('story:idea', 'Draft'),
    card('story:ask', 'Proposed', 'unplanned'),
    card('story:later', 'Accepted', 'wave 2'),
    card('story:soon', 'Accepted', 'next'),
    card('story:urgent', 'Accepted', 'next', true),
    card('story:doing', 'InProgress', 'next'),
    card('story:shipped', 'Done'),
  ],
  errors: [],
};

const STORY: PlanStory = {
  id: 'story:ask',
  title: 'Title of story:ask',
  status: 'proposed',
  column: 'Proposed',
  why: 'The operator asked for it.',
  acceptance: [
    { text: 'the `board_case` test reads the board', evidence: true },
    { text: 'a second line', evidence: false },
  ],
  scope: ['crates/conductor/src/dashboard/plan.rs'],
  history: [{ at: '2026-11-07T10:00:00Z', actor: 'human:operator', what: 'draft → proposed' }],
  moves: ['draft', 'active', 'rejected'],
  comments: '',
  important: false,
  started: false,
};

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('the board', () => {
  it('groups the laned columns by wave, next first and important cards first in it', () => {
    expect(
      lanesOf(BOARD, 'Accepted').map((lane) => [lane.lane, lane.cards.map((each) => each.id)]),
    ).toEqual([
      ['next', ['story:urgent', 'story:soon']],
      ['wave 2', ['story:later']],
    ]);
    expect(lanesOf(BOARD, 'Draft')).toEqual([{ lane: null, cards: [BOARD.cards[0]] }]);
    expect(lanesOf(BOARD, 'Accepted', '', false)[0]?.lane).toBeNull();
    expect(lanesOf(BOARD, 'Accepted', 'later')[0]?.cards.map((each) => each.id)).toEqual(['story:later']);
  });

  it('labels each move and maps a drop to the write it asks for', () => {
    expect(moveLabel('active')).toBe('Approve → Accepted');
    expect(moveLabel('rejected')).toBe('Reject');
    const ask = BOARD.cards[1]!;
    expect(dropWrite(ask, 'Accepted', ['draft', 'active', 'rejected'], 'alpha')).toEqual({
      repository: 'alpha',
      id: 'story:ask',
      action: 'Move',
      to: 'active',
    });
    expect(dropWrite(ask, 'Done', ['draft', 'active', 'rejected'], 'alpha')).toBeNull();
    expect(dropWrite(BOARD.cards[3]!, 'InProgress', null, 'alpha')).toEqual({
      repository: 'alpha',
      id: 'story:soon',
      action: 'Start',
    });
  });

  it('sends nothing without the page token, and the token in X-Conductor-Token', async () => {
    const fetcher = vi.fn(async () =>
      Response.json({ id: 'story:ask', status: 'active', decision_id: 'D-1', detail: '' }),
    );
    const write = { repository: 'alpha', id: 'story:ask', action: 'Move' as const, to: 'active' };
    expect((await sendPlan(write, fetcher, null)).error).toMatch(/token/);
    expect(fetcher).not.toHaveBeenCalled();
    const sent = await sendPlan(write, fetcher, 'abc');
    expect(sent.written?.decision_id).toBe('D-1');
    const init = (fetcher.mock.calls[0] as unknown as [string, RequestInit])[1];
    expect((init.headers as Record<string, string>)['X-Conductor-Token']).toBe('abc');
    expect(JSON.parse(init.body as string)).toEqual(write);
  });
});

/** A server for the page: the board, the story, and the answer `post` gives each write. */
function server(post: () => Response) {
  return vi.fn(async (input: string, init?: RequestInit) => {
    if (init?.method === 'POST') return post();
    if (input.startsWith('/api/plan/story')) return Response.json(STORY);
    return Response.json(BOARD);
  });
}

describe('Plan', () => {
  it('shows five columns, with the count of each', async () => {
    // Done is shown once "Hide done" is released (story:board-hide-done).
    window.localStorage.setItem('conductor.plan.hideDone', 'false');
    const wrapper = mount(Plan, { props: { fetcher: server(() => Response.json({})), token: 'abc' } });
    await flushPromises();
    expect(wrapper.findAll('section.col h2').map((title) => title.text())).toEqual([
      'Draft',
      'Proposed',
      'Accepted · next',
      'In progress',
      'Done',
    ]);
    const accepted = wrapper.find('section[data-col="Accepted"]');
    expect(accepted.findAll('h3.lane').map((lane) => lane.text())).toEqual(['next 2', 'wave 2 1']);
    expect(accepted.find('.story-card.p1').attributes('data-id')).toBe('story:urgent');
    expect(accepted.find('.ch .n').text()).toBe('3');
  });

  it('opens a story with its Why, ticked Acceptance, Scope, History and a comment box', async () => {
    const wrapper = mount(Plan, { props: { fetcher: server(() => Response.json({})), token: 'abc' } });
    await flushPromises();
    await wrapper.find('.story-card[data-id="story:ask"]').trigger('click');
    await flushPromises();
    const drawer = wrapper.find('aside.drawer');
    expect(drawer.findAll('h3').map((title) => title.text())).toEqual([
      'Why',
      'Acceptance',
      'Scope',
      'History',
      'Comment to conductor-dev',
    ]);
    expect(drawer.text()).toContain('The operator asked for it.');
    const lines = drawer.findAll('ul.acceptance li');
    expect(lines.map((line) => line.classes().includes('done'))).toEqual([true, false]);
    expect(drawer.find('ul.scope').text()).toContain('crates/conductor/src/dashboard/plan.rs');
    expect(drawer.find('.tl').text()).toContain('draft → proposed');
    expect(drawer.find('textarea').exists()).toBe(true);
    expect(drawer.findAll('.df button').map((button) => button.text())).toEqual([
      'Back to draft',
      'Approve → Accepted',
      'Reject',
      'Make it important (P1)',
    ]);
  });

  it('shows a refused move with the store reason and reads nothing back', async () => {
    const fetcher = server(() =>
      Response.json(
        { refusal: 'MoveRefused', detail: 'story:ask is proposed; a story may move to: draft, rejected, active' },
        { status: 409 },
      ),
    );
    const wrapper = mount(Plan, { props: { fetcher, token: 'abc' } });
    await flushPromises();
    await wrapper.find('.story-card[data-id="story:ask"]').trigger('click');
    await flushPromises();
    const reads = fetcher.mock.calls.length;
    await wrapper.findAll('.df button').find((button) => button.text() === 'Reject')!.trigger('click');
    await flushPromises();
    expect(wrapper.find('[role="alert"]').text()).toBe(
      'Refused by the store: story:ask is proposed; a story may move to: draft, rejected, active',
    );
    expect(fetcher.mock.calls.length).toBe(reads + 1);
    expect(wrapper.find('section[data-col="Proposed"]').find('[data-id="story:ask"]').exists()).toBe(true);
  });

  it('approves through one write with the token, then reads the board again', async () => {
    const fetcher = server(() =>
      Response.json({ id: 'story:ask', status: 'active', decision_id: 'D-20261107-01', detail: '' }),
    );
    const wrapper = mount(Plan, { props: { fetcher, token: 'abc' } });
    await flushPromises();
    await wrapper.find('.story-card[data-id="story:ask"]').trigger('click');
    await flushPromises();
    await wrapper.find('.df button.primary').trigger('click');
    await flushPromises();
    const posts = fetcher.mock.calls.filter(([, init]) => init?.method === 'POST');
    expect(posts).toHaveLength(1);
    expect(JSON.parse(posts[0]![1]!.body as string)).toEqual({
      repository: 'alpha',
      id: 'story:ask',
      action: 'Move',
      to: 'active',
    });
    expect(wrapper.find('[role="status"]').text()).toContain('recorded as your decision D-20261107-01');
  });
});

describe('App', () => {
  it('links the Plan page in its navigation and opens it at #/plan', async () => {
    window.location.hash = '#/plan';
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: string) => {
        if (input === '/data.json') return Response.json(fixture());
        if (input.startsWith('/api/plan')) return Response.json(BOARD);
        return new Response('nope', { status: 404 });
      }),
    );
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.find('nav a[aria-current="page"]').text()).toBe('Plan');
    expect(wrapper.find('h1.page').text()).toBe('Plan');
    wrapper.unmount();
    window.location.hash = '';
  });
});
