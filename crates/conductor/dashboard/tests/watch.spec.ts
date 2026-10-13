import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import WatchPanel from '../src/panels/WatchPanel.vue';
import { fixture } from './fixture';

describe('WatchPanel', () => {
  it("shows the watch's usage-limit and context lines", () => {
    const wrapper = mount(WatchPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Usage limit and context');
    expect(wrapper.findAll('li').map((line) => line.text())).toEqual([
      'usage limit: Usage limit reached · resets 15:00; 2 sessions: alpha conductor',
      'usage limit seen today: Weekly limit reached',
      'context: conductor over 300k tokens (session s2)',
    ]);
  });

  it('names the configured hand-over threshold', () => {
    const data = fixture();
    data.context_handover = 150000;
    const wrapper = mount(WatchPanel, { props: { data } });
    expect(wrapper.text()).toContain('context: conductor over 150k tokens (session s2)');
    expect(wrapper.text()).not.toContain('300k');
  });

  it('tells no watch file from no line', () => {
    const data = fixture();
    data.watch = { usage_limit: null, usage_limit_today: null, context: null };
    expect(mount(WatchPanel, { props: { data } }).text()).toContain('no watch state');
    data.watch = { usage_limit: [], usage_limit_today: [], context: [] };
    const none = mount(WatchPanel, { props: { data } });
    expect(none.text()).toContain('none');
    expect(none.findAll('li')).toHaveLength(0);
  });
});
