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
    expect(wrapper.findAll('h2').map((heading) => heading.text())).toEqual([
      'Sessions under ~/example-org (2 live, 1 exited)',
      'Usage limit and context',
      'Red CI on main',
      'Awaiting the operator (1)',
      'Recent decisions',
      'Disk waste',
      'Token spend since 2026-10-13T00:00:00Z',
      'Could not read',
    ]);
    const summary = wrapper.find('.summary').text();
    expect(summary).toContain('read 2026-10-13T09:30:00Z');
    expect(summary).toContain('free on /: 33.6 GiB of 847.0 GiB');
    expect(summary).toContain('controllers running: 1');
    expect(summary).toContain('build slot: not recorded');
    const footer = wrapper.find('footer').text();
    expect(footer).toContain('claude agents --json');
    expect(footer).toContain('~/example-org/conductor');
    expect(footer).toContain('~/.cache/conductor-watch/ci.state');

    await vi.advanceTimersByTimeAsync(20_000);
    expect(fetch).toHaveBeenCalledTimes(2);
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
