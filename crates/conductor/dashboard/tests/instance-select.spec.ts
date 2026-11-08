// story:dashboard-instance-switch, story:instance-api: the header's selector lists the daemon's
// instances from /v1/instances, marks the one this page shows (its ?instance=, else the first),
// and opens the same page for another instance at the same view; with one instance it shows just
// its name.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import InstanceSelect from '../src/components/InstanceSelect.vue';
import type { InstanceRow } from '../src/api/instances';

function row(name: string): InstanceRow {
  return { name, state: 'Running', placement: 'local' };
}

function answering(instances: InstanceRow[]) {
  return vi.fn(async () => Response.json(instances));
}

afterEach(() => {
  window.location.hash = '';
});

describe('InstanceSelect', () => {
  it('lists three instances, marks this one, and opens another at the same view', async () => {
    window.location.hash = '#/decisions';
    const fetcher = answering([row('alpha'), row('beta'), row('gamma')]);
    const navigate = vi.fn();
    const wrapper = mount(InstanceSelect, { props: { fetcher, navigate, named: 'beta' } });
    await flushPromises();

    expect(fetcher).toHaveBeenCalledWith('/v1/instances', expect.anything());
    const select = wrapper.find('select');
    expect(select.attributes('aria-label')).toBe('Instance');
    expect(wrapper.findAll('option').map((option) => option.text())).toEqual([
      'alpha',
      'beta (this page)',
      'gamma',
    ]);
    expect((select.element as HTMLSelectElement).value).toBe('beta');
    expect(wrapper.find('option[aria-current="page"]').text()).toBe('beta (this page)');

    await select.setValue('gamma');
    expect(navigate).toHaveBeenCalledWith('?instance=gamma#/decisions');

    navigate.mockClear();
    await select.setValue('beta');
    expect(navigate).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('shows the first instance when the page names none', async () => {
    const wrapper = mount(InstanceSelect, {
      props: { fetcher: answering([row('alpha'), row('beta')]), navigate: vi.fn(), named: null },
    });
    await flushPromises();
    expect(wrapper.find('option[aria-current="page"]').text()).toBe('alpha (this page)');
    wrapper.unmount();
  });

  it('shows just the name when the daemon has one instance', async () => {
    const wrapper = mount(InstanceSelect, {
      props: { fetcher: answering([row('alpha')]), navigate: vi.fn(), named: null },
    });
    await flushPromises();
    expect(wrapper.find('select').exists()).toBe(false);
    expect(wrapper.find('.instance-name').text()).toBe('alpha');
    wrapper.unmount();
  });

  it('shows nothing when /v1/instances cannot be read', async () => {
    const wrapper = mount(InstanceSelect, {
      props: { fetcher: vi.fn(async () => new Response('nope', { status: 404 })), navigate: vi.fn() },
    });
    await flushPromises();
    expect(wrapper.find('select').exists()).toBe(false);
    expect(wrapper.find('.instance-name').exists()).toBe(false);
    wrapper.unmount();
  });
});
