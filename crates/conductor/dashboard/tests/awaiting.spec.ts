import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import AwaitingPanel from '../src/panels/AwaitingPanel.vue';
import { fixture, rows } from './fixture';

describe('AwaitingPanel', () => {
  it('counts and lists the decisions awaiting the operator', () => {
    const wrapper = mount(AwaitingPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Awaiting the operator (1)');
    expect(rows(wrapper)).toEqual([['DEC-20261013-02', 'O', 'Release now?']]);
  });
});
