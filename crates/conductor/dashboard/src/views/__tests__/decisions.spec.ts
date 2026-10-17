// story:decisions-page, the app half: the top-level Decisions page lists every open class-O and
// class-H request of `/api/decisions` (fixtures/decisions.json) oldest first, each with its own
// control for its kind (two buttons Yes and No; a radio list; a checkbox list), the
// recommendation and the default marked, its options' costs, its deadline and its age; every
// control is live since story:decisions-batch-answer (decisions-answer.spec.ts). Below, the
// decision history, newest first.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import App from '../../App.vue';
import Decisions from '../Decisions.vue';
import { loadDecisions, type DecisionsAnswer } from '../../api/decisions';
import { NOW } from './overview-fixtures';
import fixture from './fixtures/decisions.json';

function answer(): DecisionsAnswer {
  return structuredClone(fixture) as DecisionsAnswer;
}

function serving(body: DecisionsAnswer) {
  return vi.fn(async () => Response.json(body, { headers: { ETag: '"d-1"' } }));
}

async function page(body: DecisionsAnswer = answer()): Promise<VueWrapper> {
  vi.stubGlobal('fetch', serving(body));
  const wrapper = mount(Decisions);
  await flushPromises();
  return wrapper;
}

function item(wrapper: VueWrapper, id: string) {
  const found = wrapper.findAll('article.decision').find((one) => one.attributes('data-request') === id);
  if (found === undefined) {
    throw new Error(`no item ${id}`);
  }
  return found;
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
  window.location.hash = '';
});

// story:sessions-page-coherent: "Awaiting the operator" links each item to `#/decisions/<id>`,
// which opens this page at that item.
describe('the Decisions page at a linked item', () => {
  it('marks and focuses the item the link names, once it is listed', async () => {
    vi.stubGlobal('fetch', serving(answer()));
    const wrapper = mount(Decisions, { props: { item: 'REQ-store' }, attachTo: document.body });
    await flushPromises();
    const linked = item(wrapper, 'REQ-store');
    expect(linked.classes()).toContain('linked');
    expect(document.activeElement).toBe(linked.element);
    expect(item(wrapper, 'REQ-release').classes()).not.toContain('linked');
    wrapper.unmount();
  });

  it('marks nothing for an item that is not open', async () => {
    vi.stubGlobal('fetch', serving(answer()));
    const wrapper = mount(Decisions, { props: { item: 'REQ-gone' }, attachTo: document.body });
    await flushPromises();
    expect(wrapper.findAll('article.linked')).toHaveLength(0);
    wrapper.unmount();
  });
});

describe('reading /api/decisions', () => {
  it('answers the open items and the history', async () => {
    const read = await loadDecisions(serving(answer()));
    expect(read.error).toBeNull();
    expect(read.answer?.open.map((row) => row.request_id)).toEqual([
      'REQ-legacy',
      'REQ-release',
      'REQ-store',
      'REQ-checks',
    ]);
    expect(read.answer?.history).toHaveLength(3);
  });

  it('asks again with the ETag it was given and keeps the answer on a 304', async () => {
    const asked: (string | null)[] = [];
    const fetcher = vi.fn(async (_input: string, init?: RequestInit) => {
      const tag = new Headers(init?.headers).get('If-None-Match');
      asked.push(tag);
      return tag === '"d-1"'
        ? new Response(null, { status: 304, headers: { ETag: '"d-1"' } })
        : Response.json(answer(), { headers: { ETag: '"d-1"' } });
    });
    const first = await loadDecisions(fetcher);
    const second = await loadDecisions(fetcher, first);
    expect(asked).toEqual([null, '"d-1"']);
    expect(second.error).toBeNull();
    expect(second.answer).toBe(first.answer);
  });

  it('names an answer it could not read', async () => {
    const read = await loadDecisions(vi.fn(async () => new Response('down', { status: 503 })));
    expect(read.error).toBe('could not read /api/decisions: 503');
    expect(read.answer).toBeNull();
  });
});

describe('the Decisions page', () => {
  it('lists every open item oldest first, with its question, class, deadline and age', async () => {
    const shuffled = answer();
    shuffled.open.reverse();
    const wrapper = await page(shuffled);
    expect(wrapper.find('h1').text()).toBe('Decisions');
    expect(wrapper.findAll('article.decision').map((one) => one.attributes('data-request'))).toEqual([
      'REQ-legacy',
      'REQ-release',
      'REQ-store',
      'REQ-checks',
    ]);
    const release = item(wrapper, 'REQ-release');
    expect(release.find('h3').text()).toBe('Release v2.0 today?');
    expect(release.find('.class-pill').text()).toBe('O');
    expect(release.find('.repository').text()).toBe('alpha');
    expect(release.find('.age').text()).toBe('raised 2h ago');
    expect(release.find('.deadline').text()).toBe('deadline 2026-10-13 18:00Z');
    expect(release.find('.reason').text()).toBe('a release cannot be undone');
    expect(item(wrapper, 'REQ-legacy').find('.deadline').text()).toBe('no deadline');
    wrapper.unmount();
  });

  it('renders a yes-no request as two buttons, Yes and No, with the marks and costs', async () => {
    const wrapper = await page();
    const release = item(wrapper, 'REQ-release');
    expect(release.classes()).toContain('kind-yes-no');
    const buttons = release.findAll('button.choice');
    expect(buttons.map((button) => button.find('.answer').text())).toEqual(['Yes', 'No']);
    for (const button of buttons) {
      expect(button.attributes('disabled')).toBeUndefined();
    }
    expect(buttons[0]!.find('.cost').text()).toBe('an hour of one controller');
    expect(buttons[0]!.find('.mark-recommended').text()).toBe('recommended');
    expect(buttons[0]!.find('.mark-default').exists()).toBe(false);
    expect(buttons[1]!.find('.mark-default').text()).toBe('default');
    expect(buttons[1]!.find('.mark-recommended').exists()).toBe(false);
    expect(release.find('input').exists()).toBe(false);
    wrapper.unmount();
  });

  it('renders a one-of request as a radio list, with the marks and costs', async () => {
    const wrapper = await page();
    const store = item(wrapper, 'REQ-store');
    expect(store.classes()).toContain('kind-one-of');
    const radios = store.findAll('input[type="radio"]');
    expect(radios).toHaveLength(3);
    for (const radio of radios) {
      expect(radio.attributes('disabled')).toBeUndefined();
      expect(radio.attributes('name')).toBe('choice-REQ-store');
    }
    const options = store.findAll('.option');
    expect(options.map((option) => option.find('.letter').text())).toEqual(['A', 'B', 'C']);
    expect(options[1]!.find('.label').text()).toBe('move to the tree store');
    expect(options[1]!.find('.cost').text()).toBe('half a day of one controller');
    expect(options[1]!.find('.mark-recommended').exists()).toBe(true);
    expect(options[0]!.find('.mark-default').exists()).toBe(true);
    expect(options[2]!.find('.mark-recommended').exists()).toBe(false);
    expect(options[2]!.find('.mark-default').exists()).toBe(false);
    expect(store.find('input[type="checkbox"]').exists()).toBe(false);
    wrapper.unmount();
  });

  it('renders a several-of request as a checkbox list, with every recommended letter marked', async () => {
    const wrapper = await page();
    const checks = item(wrapper, 'REQ-checks');
    expect(checks.classes()).toContain('kind-several-of');
    const boxes = checks.findAll('input[type="checkbox"]');
    expect(boxes).toHaveLength(3);
    for (const box of boxes) {
      expect(box.attributes('disabled')).toBeUndefined();
    }
    const options = checks.findAll('.option');
    expect(options.map((option) => option.find('.mark-recommended').exists())).toEqual([true, true, false]);
    expect(options.map((option) => option.find('.mark-default').exists())).toEqual([true, false, false]);
    expect(checks.find('.kind').text()).toBe('choose any, at least one');
    expect(checks.find('input[type="radio"]').exists()).toBe(false);
    wrapper.unmount();
  });

  it('shows a class-H item escalated before options were recorded with its text and its Done control', async () => {
    const wrapper = await page();
    const legacy = item(wrapper, 'REQ-legacy');
    expect(legacy.find('.options-text').text()).toBe('A: renew it; B: drop the job');
    expect(legacy.find('.recommendation-text').text()).toBe('recommended: A');
    expect(legacy.find('input').exists()).toBe(false);
    expect(legacy.findAll('button.choice').map((button) => button.text())).toEqual(['Done']);
    wrapper.unmount();
  });

  it('lists the history below, newest first', async () => {
    const shuffled = answer();
    shuffled.history.reverse();
    const wrapper = await page(shuffled);
    const rows = wrapper.findAll('ol.history li');
    expect(rows.map((row) => row.attributes('data-id'))).toEqual([
      'REQ-checks',
      'DEC-20261013-02',
      'DEC-20261013-01',
    ]);
    expect(rows[0]!.find('.event').text()).toBe('escalated');
    expect(rows[1]!.text()).toContain('Rename the alpha service?');
    expect(rows[1]!.find('.choice-made').text()).toBe('chose B');
    expect(rows[1]!.find('.decided-by').text()).toBe('Operator');
    const headings = wrapper.findAll('h2').map((heading) => heading.text());
    expect(headings).toEqual(['Open · 4', 'History']);
    wrapper.unmount();
  });

  it('says so when nothing is open and nothing is recorded', async () => {
    const wrapper = await page({ open: [], history: [], errors: [] });
    expect(wrapper.find('.state-empty').text()).toBe('nothing waits for the operator');
    expect(wrapper.findAll('.state-empty')[1]!.text()).toBe('no decision recorded yet');
    wrapper.unmount();
  });

  it('names /api/decisions when it could not be read', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('down', { status: 503 })));
    const wrapper = mount(Decisions);
    expect(wrapper.find('.state-loading').exists()).toBe(true);
    await flushPromises();
    expect(wrapper.find('.state-error').text()).toBe('could not read /api/decisions: 503');
    wrapper.unmount();
  });

  it('names a decision log it could not read and still shows the items', async () => {
    const wrapper = await page({ ...answer(), errors: ['decisions/2026-10.jsonl: 1 line(s) are not JSON objects'] });
    expect(wrapper.find('.state-error').text()).toContain('decisions/2026-10.jsonl');
    expect(wrapper.findAll('article.decision')).toHaveLength(4);
    wrapper.unmount();
  });
});

describe('the navigation', () => {
  it('has a top-level Decisions entry, second after the overview, that opens the page', async () => {
    window.location.hash = '#/decisions';
    vi.stubGlobal(
      'fetch',
      vi.fn(async (input: RequestInfo | URL) =>
        String(input).startsWith('/api/decisions')
          ? Response.json(answer())
          : new Response('nope', { status: 404 }),
      ),
    );
    const wrapper = mount(App);
    await flushPromises();
    const links = wrapper.findAll('nav[aria-label="Views"] a');
    expect(links.map((link) => link.text())[1]).toBe('Decisions');
    expect(links[1]!.attributes('href')).toBe('#/decisions');
    expect(links[1]!.attributes('aria-current')).toBe('page');
    expect(wrapper.find('main h1').text()).toBe('Decisions');
    expect(wrapper.findAll('article.decision')).toHaveLength(4);
    wrapper.unmount();
  });
});
