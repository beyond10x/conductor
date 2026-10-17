import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import SessionsPanel from '../src/panels/SessionsPanel.vue';
import { fixture, rows } from './fixture';

// story:sessions-page-coherent: the rows are the instance's sessions from its records, on a
// prefixed instance with a conductor and two controllers.
describe('SessionsPanel', () => {
  it('counts the running and the not-running sessions of the instance', () => {
    const wrapper = mount(SessionsPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Sessions of this instance (3 running, 0 not running)');
  });

  it("shows each session's name, role with its repository, and state", () => {
    const wrapper = mount(SessionsPanel, { props: { data: fixture() } });
    expect(rows(wrapper).map((cells) => cells.slice(0, 3))).toEqual([
      ['ex-conductor', 'conductor', 'idle'],
      ['ex-alpha', 'controller · alpha', 'busy (working)'],
      ['ex-beta', 'controller · beta', 'idle'],
    ]);
  });

  it("shows a controller's open dispatch, its brief as text, and its last event", () => {
    const wrapper = mount(SessionsPanel, { props: { data: fixture() } });
    expect(rows(wrapper)[1]).toEqual([
      'ex-alpha',
      'controller · alpha',
      'busy (working)',
      '2h 0m ago',
      '~/example-org/alpha',
      'DSP-20261013-01',
      'G1',
      'Fix <b>bold</b> & "quoted" refund retries',
      'started, 5m ago',
    ]);
    expect(wrapper.find('tbody b').exists()).toBe(false);
    expect(rows(wrapper)[0]?.slice(5)).toEqual(['—', '—', '—', '—']);
  });

  it('marks a dispatch that is no longer open, and a controller with none', () => {
    const data = fixture();
    const wrapper = mount(SessionsPanel, { props: { data } });
    expect(rows(wrapper)[2]?.slice(5)).toEqual([
      'DSP-20261013-02 (closed)',
      'G2',
      'Pin the schema',
      'done, 30m ago',
    ]);
    data.sessions[2]!.dispatch = null;
    const none = mount(SessionsPanel, { props: { data } });
    expect(rows(none)[2]?.[5]).toBe('none');
  });

  it('lists a controller whose record no live session holds as not running', () => {
    const data = fixture();
    Object.assign(data.sessions[2]!, {
      exited: true,
      status: null,
      state: 'not running',
      cwd: null,
      kind: null,
      started_at: null,
      started_ago: null,
    });
    const wrapper = mount(SessionsPanel, { props: { data } });
    expect(wrapper.find('h2').text()).toBe('Sessions of this instance (2 running, 1 not running)');
    expect(rows(wrapper)[2]?.slice(0, 5)).toEqual([
      'ex-beta',
      'controller · beta',
      'not running',
      '—',
      '—',
    ]);
  });
});
