import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import ErrorsPanel from '../src/panels/ErrorsPanel.vue';
import { fixture } from './fixture';

describe('ErrorsPanel', () => {
  it('names each source it could not read', () => {
    const wrapper = mount(ErrorsPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Could not read');
    expect(wrapper.findAll('li').map((line) => line.text())).toEqual([
      '~/example-org/conductor/dispatches/2026-10.jsonl: 1 line(s) are not JSON objects',
    ]);
  });

  it('shows nothing when every source was read', () => {
    const data = fixture();
    data.errors = [];
    expect(mount(ErrorsPanel, { props: { data } }).find('h2').exists()).toBe(false);
  });
});
