import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import AwaitingPanel from '../src/panels/AwaitingPanel.vue';
import { fixture, rows } from './fixture';

// story:sessions-page-coherent: each open item with the one thing it asks of the operator and a
// link to it on the Decisions page; an empty list says so in one line.
describe('AwaitingPanel', () => {
  it('lists each open item with its action and its link', () => {
    const wrapper = mount(AwaitingPanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Awaiting the operator (2)');
    expect(rows(wrapper)).toEqual([
      [
        'REQ-login',
        'H',
        'do: task registry-login',
        'Log in to the registry: run `task registry-login` in the alpha checkout',
      ],
      ['REQ-release', 'O', 'answer: Y release today · N wait', 'Release alpha today?'],
    ]);
    expect(wrapper.findAll('tbody a').map((link) => link.attributes('href'))).toEqual([
      '#/decisions/REQ-login',
      '#/decisions/REQ-release',
    ]);
  });

  it('says in one line that nothing awaits', () => {
    const data = fixture();
    data.awaiting_operator = [];
    const wrapper = mount(AwaitingPanel, { props: { data } });
    expect(wrapper.find('table').exists()).toBe(false);
    expect(wrapper.find('.state-empty').text()).toBe('Nothing awaits you.');
  });
});
