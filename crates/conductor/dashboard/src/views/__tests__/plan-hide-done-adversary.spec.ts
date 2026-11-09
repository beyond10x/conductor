// story:board-hide-done, adversary pass: probes of the "Hide done" toggle the unit's own spec does
// not make. Each case states the acceptance line or the guard it holds.
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
  vi.restoreAllMocks();
  window.localStorage.clear();
});

describe('the Hide done toggle, adversary probes', () => {
  it('the unit spec\'s "page without storage" spy really takes storage away', () => {
    vi.spyOn(window, 'localStorage', 'get').mockImplementation(() => {
      throw new DOMException('storage is disabled', 'SecurityError');
    });
    expect(() => window.localStorage).toThrow('storage is disabled');
  });

  it('keeps working when storage is readable but refuses writes (quota or private mode)', async () => {
    vi.spyOn(Storage.prototype, 'setItem').mockImplementation(() => {
      throw new DOMException('quota', 'QuotaExceededError');
    });
    const errors: unknown[] = [];
    const wrapper = mount(Plan, {
      props: { fetcher: fetcher() },
      global: { config: { errorHandler: (error) => void errors.push(error) } },
    });
    await flushPromises();
    expect(hideDone(wrapper).attributes('aria-pressed')).toBe('true');

    await hideDone(wrapper).trigger('click');

    expect(errors).toEqual([]);
    expect(hideDone(wrapper).attributes('aria-pressed')).toBe('false');
    expect(wrapper.find('[data-col="Done"]').text()).toContain('Shipped');
    wrapper.unmount();
  });

  it('pressing the toggle again hides Done and the next visit reads it back pressed', async () => {
    window.localStorage.setItem('conductor.plan.hideDone', 'false');
    const first = mount(Plan, { props: { fetcher: fetcher() } });
    await flushPromises();
    expect(first.find('[data-col="Done"]').exists()).toBe(true);

    await hideDone(first).trigger('click');

    expect(first.find('[data-col="Done"]').exists()).toBe(false);
    expect(window.localStorage.getItem('conductor.plan.hideDone')).toBe('true');
    first.unmount();

    const second = mount(Plan, { props: { fetcher: fetcher() } });
    await flushPromises();
    expect(hideDone(second).attributes('aria-pressed')).toBe('true');
    expect(second.text()).not.toContain('Shipped');
    second.unmount();
  });

  it('an unreadable stored value is a first visit: pressed', async () => {
    window.localStorage.setItem('conductor.plan.hideDone', '');
    const wrapper = mount(Plan, { props: { fetcher: fetcher() } });
    await flushPromises();
    expect(hideDone(wrapper).attributes('aria-pressed')).toBe('true');
    expect(wrapper.find('[data-col="Done"]').exists()).toBe(false);
    wrapper.unmount();
  });
});
