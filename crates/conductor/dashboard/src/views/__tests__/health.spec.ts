// story:dashboard-health: the health view, `#/health`, each part over the fixture
// fixtures/health.json, whose keys crates/conductor/tests/dashboard.rs holds to `/api/health`.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Health from '../Health.vue';
import DoctorPanel from '../health/DoctorPanel.vue';
import SnapshotPanel from '../health/SnapshotPanel.vue';
import CollectorRuns from '../health/CollectorRuns.vue';
import MemberFailures from '../health/MemberFailures.vue';
import DiskPanel from '../health/DiskPanel.vue';
import { duration, loadHealth, type HealthData } from '../../api/health';
import raw from './fixtures/health.json';

/** A fresh copy of the fixture, for one test to change. */
function fixture(): HealthData {
  return structuredClone(raw) as HealthData;
}

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('a duration', () => {
  it('is whole seconds under a minute, then minutes and two-digit seconds', () => {
    expect(duration(41_000)).toBe('41s');
    expect(duration(18_250)).toBe('18s');
    expect(duration(123_000)).toBe('2m03s');
    expect(duration(3_725_000)).toBe('1h02m');
    expect(duration(null)).toBe('—');
  });
});

describe('the doctor part', () => {
  it('shows the verdict, when it was taken and every row doctor printed', () => {
    const wrapper = mount(DoctorPanel, { props: { doctor: fixture().doctor } });
    expect(wrapper.find('.verdict').text()).toBe('not ready: gh (not logged in to github.com)');
    expect(wrapper.find('.verdict').classes()).toContain('c-red');
    expect(wrapper.find('.taken').text()).toContain('taken 09:25Z');
    expect(wrapper.findAll('li').map((row) => row.text())).toEqual(fixture().doctor.rows);
  });

  it('shows a ready verdict in green', () => {
    const doctor = fixture().doctor;
    doctor.verdict = 'ready';
    doctor.ready = true;
    const verdict = mount(DoctorPanel, { props: { doctor } }).find('.verdict');
    expect(verdict.text()).toBe('ready');
    expect(verdict.classes()).toContain('c-green');
  });

  it('says so while the first run is taken, and names an error', () => {
    const doctor = fixture().doctor;
    doctor.taken_at = null;
    doctor.taking = true;
    doctor.rows = [];
    doctor.verdict = null;
    doctor.ready = null;
    expect(mount(DoctorPanel, { props: { doctor } }).text()).toContain('taking doctor…');
    doctor.taken_at = '2026-10-13T09:25:00Z';
    doctor.taking = false;
    doctor.ready = false;
    doctor.error = 'doctor: read b10x.toml: permission denied';
    expect(mount(DoctorPanel, { props: { doctor } }).find('.error').text()).toBe(
      'doctor: read b10x.toml: permission denied',
    );
  });
});

describe('the snapshot part', () => {
  it('shows the newest snapshot complete with the count of members it could not read', () => {
    const data = fixture();
    const wrapper = mount(SnapshotPanel, {
      props: { snapshot: data.snapshot, failures: data.member_failures.length },
    });
    expect(wrapper.find('.state').text()).toBe('complete · 2 members failed');
    expect(wrapper.find('.state').classes()).toContain('c-amber');
    expect(wrapper.text()).toContain('started 09:20Z');
  });

  it('shows a failed snapshot with its reason', () => {
    const data = fixture();
    if (data.snapshot === null) throw new Error('the fixture has a snapshot');
    data.snapshot.state = 'Failed';
    data.snapshot.failure_reason = 'blockers: `git` gave no answer within 120 s';
    const wrapper = mount(SnapshotPanel, { props: { snapshot: data.snapshot, failures: 0 } });
    expect(wrapper.find('.state').text()).toBe('failed');
    expect(wrapper.find('.state').classes()).toContain('c-red');
    expect(wrapper.find('.reason').text()).toBe('blockers: `git` gave no answer within 120 s');
  });

  it('shows a snapshot still collecting, and none when there is none', () => {
    const data = fixture();
    if (data.snapshot === null) throw new Error('the fixture has a snapshot');
    data.snapshot.state = 'Collecting';
    const collecting = mount(SnapshotPanel, { props: { snapshot: data.snapshot, failures: 0 } });
    expect(collecting.find('.state').text()).toBe('collecting');
    expect(mount(SnapshotPanel, { props: { snapshot: null, failures: 0 } }).text()).toContain(
      'no snapshot',
    );
  });
});

describe('the collector times part', () => {
  it('shows each collector of the snapshot with its time, in the order they ran', () => {
    const wrapper = mount(CollectorRuns, { props: { runs: fixture().collector_runs } });
    expect(wrapper.find('.summary').text()).toBe('repositories 41s · ci 18s · blockers 2m03s');
    const rows = wrapper
      .findAll('tbody tr')
      .map((row) => row.findAll('td').map((cell) => cell.text()));
    expect(rows).toEqual([
      ['repositories', '09:20Z', '41s', 'completed'],
      ['ci', '09:20Z', '18s', 'completed'],
      ['blockers', '09:20Z', '2m03s', 'over budget'],
    ]);
    expect(wrapper.findAll('tbody tr')[2]?.classes()).toContain('c-red');
  });

  it('says so when the snapshot has no collector runs', () => {
    expect(mount(CollectorRuns, { props: { runs: [] } }).text()).toContain('no collector runs');
  });
});

describe('the member failures part', () => {
  it('shows each member a collector could not read with its class and fix', () => {
    const wrapper = mount(MemberFailures, { props: { failures: fixture().member_failures } });
    const cards = wrapper.findAll('.failure');
    expect(cards).toHaveLength(2);
    expect(cards[0]?.find('.who').text()).toBe('blockers · billing-worker');
    expect(cards[0]?.find('.class').text()).toBe('needs login');
    expect(cards[0]?.find('code.line').text()).toContain('could not read Username');
    expect(cards[0]?.find('.fix').text()).toContain('load the SSH key');
    expect(cards[1]?.find('.class').text()).toBe('old store');
    expect(cards[1]?.find('.fix').text()).toContain('aep plan store migrate git --dry-run');
  });

  it('says so when no member failed', () => {
    expect(mount(MemberFailures, { props: { failures: [] } }).text()).toContain(
      'no member failed',
    );
  });
});

describe('the disk part', () => {
  it('shows the free space against the floors, and its state', () => {
    const wrapper = mount(DiskPanel, { props: { disk: fixture().disk } });
    expect(wrapper.find('.free').text()).toBe('36.0 GiB free on /');
    expect(wrapper.find('.state').text()).toBe('low');
    expect(wrapper.find('.state').classes()).toContain('pill-amber');
    expect(wrapper.find('.floors').text()).toBe('low at 40.0 GiB · admit-only at 20.0 GiB');
  });

  it('marks admit-only in red and ok in green', () => {
    const disk = fixture().disk;
    disk.state = 'admit-only';
    expect(mount(DiskPanel, { props: { disk } }).find('.state').classes()).toContain('pill-red');
    disk.state = 'ok';
    expect(mount(DiskPanel, { props: { disk } }).find('.state').classes()).toContain(
      'pill-green',
    );
  });

  it('shows every disk-waste row, with the total', () => {
    const wrapper = mount(DiskPanel, { props: { disk: fixture().disk } });
    expect(wrapper.find('summary').text()).toBe('Disk waste · 6.0 GiB in 3 rows');
    const rows = wrapper
      .findAll('tbody tr')
      .map((row) => row.findAll('td').map((cell) => cell.text()));
    expect(rows.map((cells) => cells.slice(0, 3))).toEqual([
      ['5.0 GiB', 'alpha', 'build cache in idle managed trees'],
      ['1.0 GiB', 'alpha', 'target/ of the checkout'],
      ['6.0 MiB', '—', 'smaller /var/tmp entries'],
    ]);
  });
});

describe('the health view', () => {
  it('reads /api/health and shows each part', async () => {
    const fetcher = vi.fn(async () => new Response(JSON.stringify(fixture())));
    vi.stubGlobal('fetch', fetcher);
    const wrapper = mount(Health);
    await flushPromises();
    expect(fetcher).toHaveBeenCalledWith('/api/health', { cache: 'no-store' });
    expect(wrapper.find('h1').text()).toBe('Health');
    for (const part of [DoctorPanel, SnapshotPanel, CollectorRuns, MemberFailures, DiskPanel]) {
      expect(wrapper.findComponent(part).exists()).toBe(true);
    }
    wrapper.unmount();
  });

  it('names an answer it could not read', async () => {
    const fetcher = vi.fn(async () => new Response('down', { status: 503 }));
    const read = await loadHealth(fetcher);
    expect(read).toEqual({ health: null, error: 'could not read /api/health: 503' });
  });
});
