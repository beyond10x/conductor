import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import TodayTimeline from '../overview/TodayTimeline.vue';
import { overview } from './overview-fixtures';

describe('TodayTimeline', () => {
  it('shows one line per event of each kind, newest first, linked where it has a link', () => {
    const wrapper = mount(TodayTimeline, { props: { events: overview().events } });
    expect(wrapper.find('h2').text()).toBe('Today');
    const lines = wrapper.findAll('ol.timeline li');
    expect(
      lines.map((line) => [
        line.find('.when').text(),
        line.find('.kind').text(),
        line.find('.what').text(),
      ]),
    ).toEqual([
      ['09:10Z · delta', 'release', 'v2.0.0'],
      ['09:00Z · alpha', 'merge', '#5 Count the cards'],
      ['08:30Z', 'decision', 'O: Release now?'],
      ['08:20Z · alpha', 'report', 'started'],
      ['08:00Z · alpha', 'dispatch', 'G1'],
    ]);
    expect(lines[0]!.find('a').attributes('href')).toBe(
      'https://forge.example.org/example-org/delta/releases/tag/v2.0.0',
    );
    expect(lines[1]!.find('a').attributes('href')).toBe(
      'https://forge.example.org/example-org/alpha/pull/5',
    );
    expect(lines[2]!.find('a').exists()).toBe(false);
  });

  it('says so when nothing happened today', () => {
    const wrapper = mount(TodayTimeline, { props: { events: [] } });
    expect(wrapper.text()).toContain('nothing yet today');
  });
});
