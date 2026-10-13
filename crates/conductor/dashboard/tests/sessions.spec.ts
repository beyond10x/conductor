import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import SessionsPanel from '../src/panels/SessionsPanel.vue';
import { fixture, rows } from './fixture';

describe('SessionsPanel', () => {
  it('names the checkouts root with the live and exited counts', () => {
    const wrapper = mount(SessionsPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Sessions under ~/example-org (2 live, 1 exited)');
  });

  it("shows each session's name, kind and state, exited ones below the live", () => {
    const wrapper = mount(SessionsPanel, { props: { data: fixture() } });
    expect(rows(wrapper).map((cells) => cells.slice(0, 3))).toEqual([
      ['alpha', 'background', 'busy (working)'],
      ['conductor', 'background', 'blocked'],
      ['aardvark', 'interactive', 'exited'],
    ]);
  });

  it("shows a session's dispatch, its brief as text, and its last event", () => {
    const wrapper = mount(SessionsPanel, { props: { data: fixture() } });
    expect(rows(wrapper)[0]).toEqual([
      'alpha',
      'background',
      'busy (working)',
      '2h 0m ago',
      '~/example-org/alpha',
      'DSP-20261013-01',
      'G1',
      'Fix <b>bold</b> & "quoted" refund retries',
      'started, 5m ago',
    ]);
    expect(wrapper.find('tbody b').exists()).toBe(false);
    expect(rows(wrapper)[1]?.slice(5)).toEqual(['—', '—', '—', '—']);
  });
});
