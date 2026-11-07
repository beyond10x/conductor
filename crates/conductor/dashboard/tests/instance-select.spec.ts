// story:dashboard-instance-switch: the header's selector lists the host's instances from
// /api/instances, marks the one this page shows, and opens another instance's own dashboard at
// the same view; with one instance it shows just its name.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import InstanceSelect from '../src/components/InstanceSelect.vue';
import type { DashboardInstance } from '../src/api/instances';

function row(name: string, port: number, current: boolean): DashboardInstance {
  return { name, dashboard_port: port, url: `http://127.0.0.1:${port}/`, current };
}

function answering(instances: DashboardInstance[]) {
  return vi.fn(async () => Response.json({ instances, errors: [] }));
}

afterEach(() => {
  window.location.hash = '';
});

describe('InstanceSelect', () => {
  it('lists three instances, marks this one, and opens another at the same view', async () => {
    window.location.hash = '#/decisions';
    const fetcher = answering([row('alpha', 7313, false), row('beta', 7314, true), row('gamma', 7315, false)]);
    const navigate = vi.fn();
    const wrapper = mount(InstanceSelect, { props: { fetcher, navigate } });
    await flushPromises();

    expect(fetcher).toHaveBeenCalledWith('/api/instances', expect.anything());
    const select = wrapper.find('select');
    expect(select.attributes('aria-label')).toBe('Instance');
    expect(wrapper.findAll('option').map((option) => option.text())).toEqual([
      'alpha',
      'beta (this page)',
      'gamma',
    ]);
    expect((select.element as HTMLSelectElement).value).toBe('http://127.0.0.1:7314/');
    expect(wrapper.find('option[aria-current="page"]').text()).toBe('beta (this page)');

    await select.setValue('http://127.0.0.1:7315/');
    expect(navigate).toHaveBeenCalledWith('http://127.0.0.1:7315/#/decisions');

    navigate.mockClear();
    await select.setValue('http://127.0.0.1:7314/');
    expect(navigate).not.toHaveBeenCalled();
    wrapper.unmount();
  });

  it('shows just the name when the host has one instance', async () => {
    const wrapper = mount(InstanceSelect, {
      props: { fetcher: answering([row('alpha', 7313, true)]), navigate: vi.fn() },
    });
    await flushPromises();
    expect(wrapper.find('select').exists()).toBe(false);
    expect(wrapper.find('.instance-name').text()).toBe('alpha');
    wrapper.unmount();
  });

  it('shows nothing when /api/instances cannot be read', async () => {
    const wrapper = mount(InstanceSelect, {
      props: { fetcher: vi.fn(async () => new Response('nope', { status: 404 })), navigate: vi.fn() },
    });
    await flushPromises();
    expect(wrapper.find('select').exists()).toBe(false);
    expect(wrapper.find('.instance-name').exists()).toBe(false);
    wrapper.unmount();
  });
});
