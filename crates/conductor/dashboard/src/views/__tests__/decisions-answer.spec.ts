// story:decisions-batch-answer, the app half: on the Decisions page the operator answers each open
// request by its kind (Yes or No; one option; several options; Done for a class-H item), may add
// his words to each, and sends the batch: one POST to /api/answers carrying the token the page
// was served with (its `<meta name="conductor-token">`), answered row by row, each row's result
// shown on its item and in the list of what was sent.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount, type VueWrapper } from '@vue/test-utils';
import Decisions from '../Decisions.vue';
import {
  pageToken,
  sendAnswers,
  type AnswerResult,
  type DecisionsAnswer,
  type OpenDecision,
} from '../../api/decisions';
import { NOW } from './overview-fixtures';
import fixture from './fixtures/decisions.json';

const TOKEN = 'a'.repeat(64);

function answer(): DecisionsAnswer {
  return structuredClone(fixture) as DecisionsAnswer;
}

/** One POST the page sent: its URL, its headers and its body. */
interface Sent {
  url: string;
  headers: Headers;
  body: unknown;
}

/**
 * A server that answers /api/decisions with `body` and a POST to /api/answers with `results`
 * (each row recorded unless `refuse` names it), keeping each POST in `sent`.
 */
function server(body: DecisionsAnswer, refuse: Record<string, AnswerResult> = {}) {
  const sent: Sent[] = [];
  const fetcher = vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
    const url = String(input);
    if (url === '/api/answers' && init?.method === 'POST') {
      const parsed = JSON.parse(String(init.body)) as { answers: { request_id: string }[] };
      sent.push({ url, headers: new Headers(init.headers), body: parsed });
      const results = parsed.answers.map(
        (row, at) =>
          refuse[row.request_id] ?? {
            request_id: row.request_id,
            outcome: 'recorded',
            decision_id: `DEC-20261013-${String(at + 10)}`,
          },
      );
      return Response.json({ results });
    }
    return Response.json(body, { headers: { ETag: '"d-1"' } });
  });
  return { fetcher, sent };
}

async function page(fetcher: ReturnType<typeof server>['fetcher']): Promise<VueWrapper> {
  vi.stubGlobal('fetch', fetcher);
  const wrapper = mount(Decisions, { attachTo: document.body });
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

function sendButton(wrapper: VueWrapper) {
  return wrapper.get('button.send');
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
  vi.setSystemTime(NOW);
  const meta = document.createElement('meta');
  meta.setAttribute('name', 'conductor-token');
  meta.setAttribute('content', TOKEN);
  document.head.appendChild(meta);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.head.querySelectorAll('meta[name="conductor-token"]').forEach((meta) => meta.remove());
  document.body.innerHTML = '';
});

describe('sending answers', () => {
  it('reads the token from the meta tag the page was served with', () => {
    expect(pageToken()).toBe(TOKEN);
    document.head.querySelector('meta[name="conductor-token"]')?.remove();
    expect(pageToken()).toBeNull();
  });

  it('POSTs the batch as JSON with the token and answers its rows', async () => {
    const { fetcher, sent } = server(answer());
    const sentBack = await sendAnswers([{ request_id: 'REQ-store', choice: ['B'] }], fetcher, TOKEN);
    expect(sentBack.error).toBeNull();
    expect(sentBack.results).toEqual([
      { request_id: 'REQ-store', outcome: 'recorded', decision_id: 'DEC-20261013-10' },
    ]);
    expect(sent).toHaveLength(1);
    expect(sent[0]!.headers.get('Content-Type')).toBe('application/json');
    expect(sent[0]!.headers.get('X-Conductor-Token')).toBe(TOKEN);
    expect(sent[0]!.body).toEqual({ answers: [{ request_id: 'REQ-store', choice: ['B'] }] });
  });

  it('names a batch the server refused whole, and sends nothing without a token', async () => {
    const refused = vi.fn(async () => new Response('the token does not match\n', { status: 403 }));
    const read = await sendAnswers([{ request_id: 'REQ-store', choice: ['B'] }], refused, TOKEN);
    expect(read.results).toBeNull();
    expect(read.error).toBe('could not send the answers: 403 the token does not match');

    const never = vi.fn();
    const untokened = await sendAnswers([{ request_id: 'REQ-store', choice: ['B'] }], never, null);
    expect(never).not.toHaveBeenCalled();
    expect(untokened.error).toContain('no token');
  });
});

describe('answering on the Decisions page', () => {
  it('answers a yes-no request with one of its two buttons', async () => {
    const { fetcher, sent } = server(answer());
    const wrapper = await page(fetcher);
    const release = item(wrapper, 'REQ-release');
    const [yes, no] = release.findAll('button.choice');
    expect(yes!.attributes('disabled')).toBeUndefined();
    await yes!.trigger('click');
    expect(yes!.attributes('aria-pressed')).toBe('true');
    await no!.trigger('click');
    expect(yes!.attributes('aria-pressed')).toBe('false');
    expect(no!.attributes('aria-pressed')).toBe('true');
    await release.get('textarea.words-input').setValue('not today');
    await sendButton(wrapper).trigger('click');
    await flushPromises();
    expect(sent[0]!.body).toEqual({
      answers: [{ request_id: 'REQ-release', choice: ['N'], words: 'not today' }],
    });
    wrapper.unmount();
  });

  it('answers a one-of request with one radio', async () => {
    const { fetcher, sent } = server(answer());
    const wrapper = await page(fetcher);
    const store = item(wrapper, 'REQ-store');
    const radios = store.findAll('input[type="radio"]');
    for (const radio of radios) {
      expect(radio.attributes('disabled')).toBeUndefined();
    }
    await radios[0]!.setValue(true);
    await radios[1]!.setValue(true);
    await sendButton(wrapper).trigger('click');
    await flushPromises();
    expect(sent[0]!.body).toEqual({ answers: [{ request_id: 'REQ-store', choice: ['B'] }] });
    wrapper.unmount();
  });

  it('answers a several-of request with every box ticked, in the options order', async () => {
    const { fetcher, sent } = server(answer());
    const wrapper = await page(fetcher);
    const checks = item(wrapper, 'REQ-checks');
    const boxes = checks.findAll('input[type="checkbox"]');
    await boxes[2]!.setValue(true);
    await boxes[0]!.setValue(true);
    await boxes[1]!.setValue(true);
    await boxes[1]!.setValue(false);
    await sendButton(wrapper).trigger('click');
    await flushPromises();
    expect(sent[0]!.body).toEqual({ answers: [{ request_id: 'REQ-checks', choice: ['A', 'C'] }] });
    wrapper.unmount();
  });

  it('answers a class-H item with its one Done control', async () => {
    const body = answer();
    const legacy = body.open[0] as OpenDecision;
    expect(legacy.class).toBe('H');
    const { fetcher, sent } = server(body);
    const wrapper = await page(fetcher);
    const hands = item(wrapper, 'REQ-legacy');
    const done = hands.findAll('button.choice');
    expect(done.map((button) => button.text())).toEqual(['Done']);
    expect(hands.find('input').exists()).toBe(false);
    await done[0]!.trigger('click');
    await sendButton(wrapper).trigger('click');
    await flushPromises();
    expect(sent[0]!.body).toEqual({ answers: [{ request_id: 'REQ-legacy', choice: ['done'] }] });
    wrapper.unmount();
  });

  it('shows a class-O request escalated without options with its words and no control', async () => {
    const body = answer();
    (body.open[0] as OpenDecision).class = 'O';
    const { fetcher } = server(body);
    const wrapper = await page(fetcher);
    const legacy = item(wrapper, 'REQ-legacy');
    expect(legacy.find('.options-text').text()).toBe('A: renew it; B: drop the job');
    expect(legacy.find('button').exists()).toBe(false);
    expect(legacy.find('input').exists()).toBe(false);
    expect(legacy.find('.no-options').text()).toContain("answer it in conductor's session");
    wrapper.unmount();
  });

  it('sends every chosen answer in one POST and nothing for an item left alone', async () => {
    const { fetcher, sent } = server(answer());
    const wrapper = await page(fetcher);
    expect(sendButton(wrapper).attributes('disabled')).toBeDefined();
    await item(wrapper, 'REQ-release').findAll('button.choice')[0]!.trigger('click');
    await item(wrapper, 'REQ-store').findAll('input[type="radio"]')[2]!.setValue(true);
    expect(sendButton(wrapper).text()).toBe('Send 2 answers');
    expect(sendButton(wrapper).attributes('disabled')).toBeUndefined();
    await sendButton(wrapper).trigger('click');
    await flushPromises();
    expect(sent).toHaveLength(1);
    expect(sent[0]!.headers.get('X-Conductor-Token')).toBe(TOKEN);
    expect(sent[0]!.body).toEqual({
      answers: [
        { request_id: 'REQ-release', choice: ['Y'] },
        { request_id: 'REQ-store', choice: ['C'] },
      ],
    });
    wrapper.unmount();
  });

  it('shows each row of the batch recorded or refused, and keeps a refused choice', async () => {
    const { fetcher } = server(answer(), {
      'REQ-store': {
        request_id: 'REQ-store',
        outcome: 'refused',
        error: 'RequestStateConflict',
        reason: 'request "REQ-store" is not escalated; nothing was recorded',
      },
    });
    const wrapper = await page(fetcher);
    await item(wrapper, 'REQ-release').findAll('button.choice')[0]!.trigger('click');
    await item(wrapper, 'REQ-store').findAll('input[type="radio"]')[0]!.setValue(true);
    await sendButton(wrapper).trigger('click');
    await flushPromises();
    const rows = wrapper.findAll('ul.results li');
    expect(rows.map((row) => row.attributes('data-request'))).toEqual(['REQ-release', 'REQ-store']);
    expect(rows[0]!.text()).toContain('recorded DEC-20261013-10');
    expect(rows[1]!.text()).toContain('refused: RequestStateConflict');
    expect(rows[1]!.classes()).toContain('refused');
    const store = item(wrapper, 'REQ-store');
    expect(store.find('.result').text()).toContain('RequestStateConflict');
    expect((store.findAll('input[type="radio"]')[0]!.element as HTMLInputElement).checked).toBe(true);
    wrapper.unmount();
  });

  it('names a batch refused whole', async () => {
    const fetcher = vi.fn(async (_input: RequestInfo | URL, init?: RequestInit) =>
      init?.method === 'POST'
        ? new Response('the token does not match\n', { status: 403 })
        : Response.json(answer()),
    );
    vi.stubGlobal('fetch', fetcher);
    const wrapper = mount(Decisions, { attachTo: document.body });
    await flushPromises();
    await item(wrapper, 'REQ-release').findAll('button.choice')[0]!.trigger('click');
    await sendButton(wrapper).trigger('click');
    await flushPromises();
    expect(wrapper.find('.send-error').text()).toBe(
      'could not send the answers: 403 the token does not match',
    );
    wrapper.unmount();
  });

  it('says answers are recorded as the operator', async () => {
    const { fetcher } = server(answer());
    const wrapper = await page(fetcher);
    expect(wrapper.find('.answer-note').text()).toContain('recorded as your decision');
    wrapper.unmount();
  });
});
