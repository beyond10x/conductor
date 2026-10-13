import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import DecisionsPanel from '../src/panels/DecisionsPanel.vue';
import { fixture, rows } from './fixture';

describe('DecisionsPanel', () => {
  it('lists the recent decisions newest first, an open choice as —', () => {
    const wrapper = mount(DecisionsPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Recent decisions');
    expect(rows(wrapper)).toEqual([
      ['DEC-20261013-02', 'O', '—', 'needs the operator'],
      ['DEC-20261013-01', 'C', 'A', 'gate green, change inside the approved design'],
    ]);
  });
});
