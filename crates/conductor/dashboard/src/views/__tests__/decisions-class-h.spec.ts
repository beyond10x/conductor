// A live /api/decisions answer holding only class-H open items (a yes-no kind with options Y/N,
// no `state` field): the Decisions page must show each of them in its default view.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Decisions from '../Decisions.vue';
import fixture from './fixtures/decisions-class-h.json';

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('class-H open items on the Decisions page', () => {
  it('shows every open class-H item of the live answer', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => Response.json(structuredClone(fixture))));
    const wrapper = mount(Decisions);
    await flushPromises();
    const ids = wrapper.findAll('article.decision').map((one) => one.attributes('data-request'));
    expect(ids.sort()).toEqual(fixture.open.map((row) => row.request_id).sort());
    expect(wrapper.text()).toContain('Open · 3');
  });

  it('shows a class-H yes-no item its options, costs, recommendation and default', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => Response.json(structuredClone(fixture))));
    const wrapper = mount(Decisions);
    await flushPromises();
    const card = wrapper
      .findAll('article.decision')
      .find((one) => one.attributes('data-request') === 'H-20261007-key');
    expect(card).toBeDefined();
    const text = card!.text();
    expect(text).toContain('move onto key.create now');
    expect(text).toContain('keep the current key');
    expect(text).toContain('a new $5/month-capped inference key');
    expect(text).toContain('recommended');
    expect(text).toContain('default');
    expect(card!.find('button.choice').text()).toBe('Done');
  });
});
