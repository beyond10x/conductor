import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import NeedsYou from '../overview/NeedsYou.vue';
import { needsYou } from '../../api/overview';
import { overview } from './overview-fixtures';

describe('NeedsYou', () => {
  it('has one card per open item, with its action and link, and one per red main, with its command', () => {
    const wrapper = mount(NeedsYou, { props: { cards: needsYou(overview()) } });
    expect(wrapper.find('h2').text()).toBe('Needs you · 4');
    const cards = wrapper.findAll('.card');
    expect(cards).toHaveLength(4);

    // story:sessions-page-coherent: the one thing the item asks, and where to answer it.
    expect(cards[0]!.find('.pill').text()).toBe('hands · class H · REQ-login');
    expect(cards[0]!.find('code.cmd').text()).toBe('do: task registry-login');
    expect(cards[0]!.find('a').attributes('href')).toBe('#/decisions/REQ-login');
    expect(cards[1]!.find('.pill').text()).toBe('operator · class O · REQ-release');
    expect(cards[1]!.text()).toContain('Release alpha today?');
    expect(cards[1]!.find('code.cmd').text()).toBe('answer: Y release today · N wait');

    expect(cards[2]!.find('.pill').text()).toBe('red CI · main');
    expect(cards[2]!.text()).toContain('alpha');
    expect(cards[2]!.find('code.cmd').text()).toBe(
      "cd ~/example-org/alpha && gh run list --branch main --workflow 'CI' --limit 5",
    );
    expect(cards[2]!.find('a').exists()).toBe(false);

    expect(cards[3]!.text()).toContain('delta');
    expect(cards[3]!.find('code.cmd').text()).toBe(
      "cd ~/example-org/delta && gh run list --branch main --workflow 'Release' --limit 5",
    );
  });

  it('names a class H request as one for the operator’s hands', () => {
    const [card] = needsYou(overview());
    expect(card!.kind).toBe('hands');
  });

  it('says so when nothing needs the operator', () => {
    const view = overview();
    view.data!.awaiting_operator = [];
    view.board = { snapshot: null, rows: [] };
    const wrapper = mount(NeedsYou, { props: { cards: needsYou(view) } });
    expect(wrapper.find('h2').text()).toBe('Needs you · 0');
    expect(wrapper.findAll('.card')).toHaveLength(0);
    expect(wrapper.text()).toContain('nothing needs you');
  });
});
