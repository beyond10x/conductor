import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import WatchPanel from '../src/panels/WatchPanel.vue';
import { fixture } from './fixture';

describe('WatchPanel', () => {
  it('shows each usage limit in force with its reset, and the context lines', () => {
    const wrapper = mount(WatchPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Usage limit and context');
    expect(wrapper.findAll('li').map((line) => line.text())).toEqual([
      "usage limit until 15:00Z: You've hit your session limit · resets 3:00pm (UTC); sessions: ex-alpha ex-beta",
      'context: ex-conductor over 300k tokens (session s2)',
    ]);
  });

  it('says so when a limit names no reset it could read', () => {
    const data = fixture();
    data.watch.limits = [{ text: 'Usage limit reached', resets_at: null, sessions: [] }];
    expect(mount(WatchPanel, { props: { data } }).text()).toContain(
      'usage limit reset time not read: Usage limit reached',
    );
  });

  it('names the configured hand-over threshold', () => {
    const data = fixture();
    data.context_handover = 150000;
    const wrapper = mount(WatchPanel, { props: { data } });
    expect(wrapper.text()).toContain('context: ex-conductor over 150k tokens (session s2)');
    expect(wrapper.text()).not.toContain('300k');
  });

  it('tells no watch file from no line', () => {
    const data = fixture();
    data.watch = { limits: null, context: null };
    expect(mount(WatchPanel, { props: { data } }).text()).toContain('no watch state');
    data.watch = { limits: [], context: [] };
    const none = mount(WatchPanel, { props: { data } });
    expect(none.text()).toContain('no limit in force, no context warning');
    expect(none.findAll('li')).toHaveLength(0);
  });
});
