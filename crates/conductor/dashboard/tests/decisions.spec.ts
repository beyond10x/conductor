import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import DecisionsPanel from '../src/panels/DecisionsPanel.vue';
import { fixture, rows } from './fixture';

describe('DecisionsPanel', () => {
  it('lists the recent decisions newest first, each with its choice', () => {
    const wrapper = mount(DecisionsPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Recent decisions');
    expect(rows(wrapper)).toEqual([
      ['DEC-20261013-02', '2026-10-13T09:10:00Z', 'O', 'Y', 'the operator: release today'],
      [
        'DEC-20261013-01',
        '2026-10-13T09:00:00Z',
        'C',
        'A',
        'gate green, change inside the approved design',
      ],
    ]);
  });
});
