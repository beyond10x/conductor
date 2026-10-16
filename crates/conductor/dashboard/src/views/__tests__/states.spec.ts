// story:dashboard-polish: every panel of every page says when it is reading, when it has nothing,
// and when what it reads could not be read, each the same way (`.state-loading`, `.state-empty`,
// `.state-error`).
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import App from '../../App.vue';
import Overview from '../Overview.vue';
import Repo from '../Repo.vue';
import Health from '../Health.vue';
import { NOW, answers } from './overview-fixtures';
import { seriesResponse } from './series-fixtures';
import data from '../../fixtures/data.json';
import type { DashboardData } from '../../types';

/** A `fetch` answering `paths` by path (the query left out), `/api/series` from its fixtures. */
function answering(paths: Record<string, unknown>, failing: string[] = []) {
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    const path = url.split('?')[0]!;
    if (failing.includes(path)) {
      return new Response('nope', { status: 500 });
    }
    const series = seriesResponse(url);
    if (series !== null) {
      return series;
    }
    if (path in paths) {
      return Response.json(paths[path]);
    }
    return new Response('nope', { status: 500 });
  });
}

/** The endpoints of the overview and the repository page, each answering nothing. */
function nothing(): Record<string, unknown> {
  const none = { snapshot: null, rows: [] };
  const empty = structuredClone(data) as DashboardData;
  empty.sessions = [];
  empty.decisions = [];
  empty.awaiting_operator = [];
  empty.red_ci = [];
  empty.usage.rows = [];
  empty.disk.rows = [];
  empty.errors = [];
  empty.watch = { usage_limit: [], usage_limit_today: [], context: [] };
  return {
    '/data.json': empty,
    '/api/board': none,
    '/api/repositories': none,
    '/api/pull-requests': none,
    '/api/workflow-runs': none,
    '/api/releases': none,
    '/api/reviews': none,
    '/api/unanswered': { count: 0, oldest: null },
    '/api/events': { rows: [] },
    '/api/waiting-on': { rows: [] },
    '/api/blockers': none,
    '/api/specifications': none,
    '/api/sessions': { rows: [] },
  };
}

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  window.location.hash = '';
});

describe('the overview', () => {
  it('says it is reading', () => {
    vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
    const wrapper = mount(Overview);
    const reading = wrapper.find('.overview > .state-loading');
    expect(reading.text()).toBe('reading the overview…');
    expect(reading.attributes('role')).toBe('status');
    wrapper.unmount();
  });

  it('says what each panel has nothing of', async () => {
    vi.stubGlobal('fetch', answering(nothing()));
    const wrapper = mount(Overview);
    await flushPromises();
    const empty = wrapper.findAll('.state-empty').map((state) => state.text());
    expect(empty).toContain('nothing waits for review');
    expect(empty).toContain('nothing needs you');
    expect(empty).toContain('No repositories match this filter.');
    expect(empty).toContain('nothing yet today');
    wrapper.unmount();
  });

  it('names on its panel the endpoint it could not read', async () => {
    vi.stubGlobal('fetch', answering(answers(), ['/api/events', '/api/reviews']));
    const wrapper = mount(Overview);
    await flushPromises();
    const errors = wrapper.findAll('.state-error').map((state) => state.text());
    expect(errors).toContain('could not read /api/events: 500');
    expect(errors).toContain('could not read /api/reviews: 500');
    expect(wrapper.find('section[aria-labelledby="today"] .state-error').exists()).toBe(true);
    expect(wrapper.find('section[aria-labelledby="reviews"] .state-error').exists()).toBe(true);
    wrapper.unmount();
  });
});

describe('the repository page', () => {
  it('says it is reading', () => {
    vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
    const wrapper = mount(Repo, { props: { name: 'alpha' } });
    expect(wrapper.find('.state-loading').text()).toBe('reading alpha…');
    wrapper.unmount();
  });

  it('says what each panel has nothing of', async () => {
    vi.stubGlobal('fetch', answering(nothing()));
    const wrapper = mount(Repo, { props: { name: 'alpha' } });
    await flushPromises();
    const empty = wrapper.findAll('.state-empty').map((state) => state.text());
    for (const text of ['none open', 'no session works here', 'nothing yet today']) {
      expect(empty).toContain(text);
    }
    wrapper.unmount();
  });

  it('names on its panel the endpoint it could not read', async () => {
    vi.stubGlobal('fetch', answering(nothing(), ['/api/pull-requests', '/api/sessions']));
    const wrapper = mount(Repo, { props: { name: 'alpha' } });
    await flushPromises();
    expect(wrapper.find('section[aria-labelledby="mrs"] .state-error').text()).toBe(
      'could not read /api/pull-requests: 500',
    );
    expect(wrapper.find('section[aria-labelledby="sess"] .state-error').text()).toBe(
      'could not read /api/sessions?repository=alpha: 500',
    );
    wrapper.unmount();
  });
});

describe('the health page', () => {
  it('says it is reading, and names an answer it could not read', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('down', { status: 503 })));
    const wrapper = mount(Health);
    expect(wrapper.find('.state-loading').text()).toBe('reading /api/health…');
    await flushPromises();
    expect(wrapper.find('.state-error').text()).toBe('could not read /api/health: 503');
    expect(wrapper.find('.state-error').attributes('role')).toBe('alert');
    expect(wrapper.find('.state-loading').exists()).toBe(false);
    wrapper.unmount();
  });
});

describe('the sessions page', () => {
  beforeEach(() => {
    window.location.hash = '#/sessions';
  });

  it('says it is reading', () => {
    vi.stubGlobal('fetch', vi.fn(() => new Promise<Response>(() => {})));
    const wrapper = mount(App);
    expect(wrapper.find('main .state-loading').text()).toBe('reading /data.json…');
    wrapper.unmount();
  });

  it('names /data.json when it could not be read', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('down', { status: 500 })));
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.find('main .state-error').text()).toBe('could not read /data.json: 500');
    wrapper.unmount();
  });

  it('says what each table has nothing of', async () => {
    vi.stubGlobal('fetch', answering(nothing()));
    const wrapper = mount(App);
    await flushPromises();
    const empty = wrapper.findAll('main .state-empty').map((state) => state.text());
    for (const text of ['no session', 'nothing awaits the operator', 'no decision yet', 'no spend measured']) {
      expect(empty).toContain(text);
    }
    wrapper.unmount();
  });
});
