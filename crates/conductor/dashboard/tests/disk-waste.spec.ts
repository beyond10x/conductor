import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import DiskWastePanel from '../src/panels/DiskWastePanel.vue';
import { fixture, rows } from './fixture';

describe('DiskWastePanel', () => {
  it('shows the total, the reclaimable share and the age of the measurement', () => {
    const wrapper = mount(DiskWastePanel, { props: { data: fixture() } });
    expect(wrapper.find('h2').text()).toBe('Disk waste');
    const meta = wrapper.find('.meta').text();
    expect(meta).toContain('total 6.0 GiB over 3 rows, 5.0 GiB reclaimable');
    expect(meta).toContain('measured 2026-10-13T09:25:00Z (5m ago)');
  });

  it('shows each row largest first with its size, repository and what it measures', () => {
    const wrapper = mount(DiskWastePanel, { props: { data: fixture() } });
    expect(rows(wrapper).map((cells) => cells.slice(0, 3))).toEqual([
      ['5.0 GiB', 'alpha', 'build cache in idle managed trees'],
      ['1.0 GiB', 'alpha', 'target/ of the checkout'],
      ['6.0 MiB', '—', 'smaller /var/tmp entries'],
    ]);
    expect(rows(wrapper)[2]?.slice(4)).toEqual([
      '2',
      'agent',
      '2d 4h',
      'measured',
      "kept: not conductor's",
    ]);
  });

  it('says so while the first measurement runs', () => {
    const data = fixture();
    data.disk.measured_at = null;
    data.disk.measuring = true;
    data.disk.rows = [];
    expect(mount(DiskWastePanel, { props: { data } }).text()).toContain(
      'measuring, no measurement yet',
    );
  });
});
