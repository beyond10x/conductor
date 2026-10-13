import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import NeedsYou from '../overview/NeedsYou.vue';
import { needsYou } from '../../api/overview';
import { overview } from './overview-fixtures';

describe('NeedsYou', () => {
  it('has one card per decision awaiting the operator and one per red main, each with its command', () => {
    const wrapper = mount(NeedsYou, { props: { cards: needsYou(overview()) } });
    expect(wrapper.find('h2').text()).toBe('Needs you · 3');
    const cards = wrapper.findAll('.card');
    expect(cards).toHaveLength(3);

    expect(cards[0]!.find('.pill').text()).toBe('operator · class O');
    expect(cards[0]!.text()).toContain('Release now?');
    expect(cards[0]!.find('code.cmd').text()).toBe('[DECISION DEC-20261013-02] <option> <reason>');

    expect(cards[1]!.find('.pill').text()).toBe('red CI · main');
    expect(cards[1]!.text()).toContain('alpha');
    expect(cards[1]!.text()).toContain('CI');
    expect(cards[1]!.find('code.cmd').text()).toBe(
      "cd ~/example-org/alpha && gh run list --branch main --workflow 'CI' --limit 5",
    );

    expect(cards[2]!.text()).toContain('delta');
    expect(cards[2]!.find('code.cmd').text()).toBe(
      "cd ~/example-org/delta && gh run list --branch main --workflow 'Release' --limit 5",
    );
  });

  it('names a class H request as one for the operator’s hands', () => {
    const view = overview();
    view.data!.awaiting_operator[0]!.class = 'H';
    const [card] = needsYou(view);
    expect(card!.pill).toBe('hands · class H');
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
