// story:board-hide-done: the Plan board hides the Done column until its "Hide done" toggle is
// released, and remembers the choice in this browser.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Plan from '../Plan.vue';

const BOARD = {
  repositories: ['alpha'],
  repository: 'alpha',
  cards: [
    { id: 'story:doing', title: 'Being built', status: 'active', column: 'InProgress', lane: 'next', important: false, started: true },
    { id: 'story:done', title: 'Shipped', status: 'implemented', column: 'Done', lane: '', important: false, started: false },
  ],
  errors: [],
};

function fetcher() {
  return vi.fn(async (input: string) =>
    input.split('?')[0] === '/api/plan' ? Response.json(BOARD) : new Response('nope', { status: 404 }),
  );
}

function hideDone(wrapper: ReturnType<typeof mount>) {
  return wrapper.findAll('button').find((button) => button.text() === 'Hide done')!;
}

afterEach(() => {
  window.localStorage.clear();
});

describe('the Plan board', () => {
  it('hides done stories by default and shows them once the toggle is released', async () => {
    const wrapper = mount(Plan, { props: { fetcher: fetcher() } });
    await flushPromises();

    expect(hideDone(wrapper).attributes('aria-pressed')).toBe('true');
    expect(wrapper.text()).toContain('Being built');
    expect(wrapper.text()).not.toContain('Shipped');
    expect(wrapper.find('[data-col="Done"]').exists()).toBe(false);

    await hideDone(wrapper).trigger('click');

    expect(hideDone(wrapper).attributes('aria-pressed')).toBe('false');
    expect(wrapper.text()).toContain('Shipped');
    expect(window.localStorage.getItem('conductor.plan.hideDone')).toBe('false');
    wrapper.unmount();
  });

  it('reads a released toggle back on the next visit', async () => {
    window.localStorage.setItem('conductor.plan.hideDone', 'false');
    const wrapper = mount(Plan, { props: { fetcher: fetcher() } });
    await flushPromises();

    expect(hideDone(wrapper).attributes('aria-pressed')).toBe('false');
    expect(wrapper.text()).toContain('Shipped');
    wrapper.unmount();
  });
});
