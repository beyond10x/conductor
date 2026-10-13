import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import RedCiPanel from '../src/panels/RedCiPanel.vue';
import { fixture, rows } from './fixture';

describe('RedCiPanel', () => {
  it('shows each red workflow on main', () => {
    const wrapper = mount(RedCiPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Red CI on main');
    expect(rows(wrapper)).toEqual([['beta', 'Release', '202', 'failure', '2026-10-13T08:00:00Z']]);
  });

  it('tells no watch state from no red run', () => {
    const data = fixture();
    data.red_ci = null;
    expect(mount(RedCiPanel, { props: { data } }).text()).toContain('no watch state');
    data.red_ci = [];
    const none = mount(RedCiPanel, { props: { data } });
    expect(none.text()).toContain('none');
    expect(none.find('table').exists()).toBe(false);
  });
});
