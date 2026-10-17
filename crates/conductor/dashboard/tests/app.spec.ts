import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import App from '../src/App.vue';
import { fixture } from './fixture';

// The panels of /data.json are the `#/sessions` view; `#/` is the overview (its own tests).
beforeEach(() => {
  window.location.hash = '#/sessions';
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('App', () => {
  it('reads /data.json, shows every panel, and reads it again every 20 s', async () => {
    vi.useFakeTimers();
    const fetch = vi.fn(async () => new Response(JSON.stringify(fixture())));
    vi.stubGlobal('fetch', fetch);
    const wrapper = mount(App);
    await flushPromises();
    expect(fetch).toHaveBeenCalledWith('/data.json', expect.anything());
    // Red CI on main is the Overview's and disk waste Health's (story:sessions-page-coherent).
    expect(wrapper.findAll('h2').map((heading) => heading.text())).toEqual([
      'Sessions of this instance (3 running, 0 not running)',
      'Awaiting the operator (2)',
      'Usage limit and context',
      'Recent decisions',
      'Token spend since 2026-10-13T00:00:00Z',
      'Could not read',
    ]);
    const summary = wrapper.find('.summary').text();
    expect(summary).toContain('read 2026-10-13T09:30:00Z');
    expect(summary).toContain('sessions: 3');
    expect(summary).toContain('controllers running: 2');
    expect(summary).toContain('free on /: 33.6 GiB of 847.0 GiB');
    expect(summary).toContain('build slot: not recorded');
    expect(wrapper.find('.snapshot-age').text()).toBe(
      'newest snapshot 7h 0m old, older than its 2h cycle: what it shows may be out of date',
    );
    expect(wrapper.find('.moved').findAll('a').map((link) => link.attributes('href'))).toEqual([
      '#/',
      '#/health',
    ]);
    const footer = wrapper.find('footer').text();
    expect(footer).toContain('claude agents --json');
    expect(footer).toContain('~/example-org/conductor');

    await vi.advanceTimersByTimeAsync(20_000);
    expect(fetch).toHaveBeenCalledTimes(2);
    wrapper.unmount();
  });

  it('shows a fresh snapshot without a warning, and none at all as a warning', async () => {
    const data = fixture();
    data.snapshot = { ...data.snapshot!, age: '30m', age_seconds: 1800, stale: false };
    vi.stubGlobal('fetch', vi.fn(async () => new Response(JSON.stringify(data))));
    const fresh = mount(App);
    await flushPromises();
    expect(fresh.find('.snapshot-age').text()).toBe('newest snapshot 30m old');
    expect(fresh.find('.snapshot-age').classes()).not.toContain('c-amber-strong');
    fresh.unmount();
    data.snapshot = null;
    const none = mount(App);
    await flushPromises();
    expect(none.find('.snapshot-age').text()).toBe('no complete snapshot yet');
    expect(none.find('.snapshot-age').classes()).toContain('c-amber-strong');
    none.unmount();
  });

  it('opens #/decisions/<id> on the Decisions page at that item', async () => {
    window.location.hash = '#/decisions/REQ-login';
    const fetch = vi.fn(async (url: string) =>
      url === '/data.json'
        ? new Response(JSON.stringify(fixture()))
        : new Response(JSON.stringify({ open: [], history: [], errors: [] })),
    );
    vi.stubGlobal('fetch', fetch);
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.find('nav a[aria-current="page"]').text()).toBe('Decisions');
    wrapper.unmount();
  });

  it('names a failed read', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => new Response('nope', { status: 500 })),
    );
    const wrapper = mount(App);
    await flushPromises();
    expect(wrapper.find('.live').text()).toContain('could not read /data.json: 500');
    wrapper.unmount();
  });
});
