import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import TokenSpendPanel from '../src/panels/TokenSpendPanel.vue';
import { fixture, rows } from './fixture';

describe('TokenSpendPanel', () => {
  it('names the instant it counts from and shows one row per repository', () => {
    const wrapper = mount(TokenSpendPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Token spend since 2026-10-13T00:00:00Z');
    expect(rows(wrapper)).toEqual([
      ['alpha', '2', '14', '1200', '3400', '250000', '18000', '83.3'],
      ['conductor', '1', '5', '300', '900', '50000', '4000', '16.7'],
    ]);
    expect(wrapper.find('ul').text()).toBe('~/.claude/projects/broken.jsonl: line 3 is not JSON');
  });

  it('says so before the first measurement', () => {
    const data = fixture();
    data.usage = {
      since: null,
      measured_at: null,
      measured_ago: null,
      measuring: false,
      rows: [],
      errors: [],
    };
    const wrapper = mount(TokenSpendPanel, { props: { data } });
    expect(wrapper.find('h2').text()).toBe('Token spend');
    expect(wrapper.text()).toContain('no measurement');
  });
});
