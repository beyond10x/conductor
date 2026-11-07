// story:dashboard-config-page: the Configuration page stages edits, checks each as
// `conductor config validate` checks its kind of key, shows the YAML diff of what is staged, and
// enables Write only while no check fails; the write carries the page's token.
import { describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Config from '../src/views/Config.vue';
import {
  diff,
  emptyStaging,
  needsRestart,
  problemOf,
  problems,
  sendConfig,
  shown,
  stage,
  writeBody,
  type ConfigField,
  type ConfigPage,
} from '../src/api/config';

function field(path: string, value: string, fallback: string | null, effect: ConfigField['takes_effect']): ConfigField {
  return { path, value, default: fallback, takes_effect: effect };
}

const FIELDS: ConfigField[] = [
  field('name', 'example-org', null, 'Fixed'),
  field('records', '~/records', null, 'Fixed'),
  field('controllers.max_working', '5', null, 'Cycle'),
  field('cadence.watch', '3m', '180s', 'LoopRestart'),
  field('cadence.daily', '08:07', null, 'LoopRestart'),
  field('thresholds.disk_low', '100G', null, 'WatchTick'),
  field('thresholds.disk_clear', '110G', null, 'WatchTick'),
  field('authority.class_c', 'conductor', null, 'Cycle'),
];
const at = (path: string): ConfigField => FIELDS.find((known) => known.path === path)!;

describe('staging', () => {
  it('stages a value, unstages the file value, and resets to the default', () => {
    const staging = emptyStaging();
    stage(staging, at('cadence.watch'), '1m');
    expect(shown(staging, at('cadence.watch'))).toBe('1m');
    stage(staging, at('cadence.watch'), '3m');
    expect(staging.values.size).toBe(0);
    stage(staging, at('cadence.watch'), null);
    expect(staging.values.get('cadence.watch')).toBeNull();
    expect(shown(staging, at('cadence.watch'))).toBe('180s');
    stage(staging, at('thresholds.disk_low'), null);
    expect(staging.values.has('thresholds.disk_low')).toBe(false);
  });

  it('sends the changes, a reset as a path without a value, and each activity switch', () => {
    const staging = emptyStaging();
    stage(staging, at('controllers.max_working'), '8');
    stage(staging, at('cadence.watch'), null);
    staging.activity.set('alpha', { repository: 'alpha', activity: 'Inactive', reason: ' paused ', added: false });
    expect(writeBody(staging, 'more work at once', false)).toEqual({
      changes: [{ path: 'controllers.max_working', value: '8' }, { path: 'cadence.watch' }],
      activity: [{ repository: 'alpha', activity: 'Inactive', reason: 'paused' }],
      reason: 'more work at once',
      check: false,
    });
  });
});

describe('validation', () => {
  it('names a bad value by its YAML path, as config validate does', () => {
    expect(problemOf('cadence.watch', 'soon')).toMatch(/duration/);
    expect(problemOf('cadence.watch', '90s')).toBeNull();
    expect(problemOf('thresholds.disk_low', '200')).toMatch(/GiB/);
    expect(problemOf('cadence.daily', '25:00')).toMatch(/HH:MM/);
    expect(problemOf('controllers.max_working', '0')).toBe('must be at least 1');
    expect(problemOf('thresholds.context_handover', '300k')).toBeNull();
    expect(problemOf('authority.class_c', 'nobody')).toMatch(/one of/);
    expect(problemOf('cadence.watch', null)).toBeNull();
  });

  it('checks disk_clear against disk_low as staged', () => {
    const staging = emptyStaging();
    stage(staging, at('thresholds.disk_low'), '200G');
    expect(problems(staging, FIELDS)).toEqual([
      { path: 'thresholds.disk_low', message: 'must be below disk_clear (110G)' },
    ]);
    stage(staging, at('thresholds.disk_clear'), '220G');
    expect(problems(staging, FIELDS)).toEqual([]);
  });

  it('refuses a fixed key and an activity switch without a reason', () => {
    const staging = emptyStaging();
    stage(staging, at('records'), '/elsewhere');
    staging.activity.set('alpha', { repository: 'alpha', activity: 'Inactive', reason: '', added: false });
    expect(problems(staging, FIELDS).map((problem) => problem.path)).toEqual([
      'records',
      'repositories.alpha.reason',
    ]);
  });
});

describe('the diff', () => {
  it('writes each staged key under its section, and the repositories last', () => {
    const staging = emptyStaging();
    stage(staging, at('thresholds.disk_low'), '90G');
    stage(staging, at('cadence.watch'), null);
    staging.activity.set('alpha', { repository: 'alpha', activity: 'Inactive', reason: 'paused', added: false });
    expect(diff(staging, FIELDS).map((line) => line.text)).toEqual([
      '  cadence:',
      '-   watch: 3m',
      '+   watch: 180s  # the default',
      '  thresholds:',
      '-   disk_low: 100G',
      '+   disk_low: 90G',
      '  repositories:',
      '+   alpha: inactive  # paused',
    ]);
    expect(needsRestart(staging, FIELDS)).toBe(true);
  });
});

describe('the write', () => {
  it('refuses to send without the page token, and sends it in X-Conductor-Token', async () => {
    const staging = emptyStaging();
    stage(staging, at('controllers.max_working'), '8');
    const fetcher = vi.fn(async () =>
      Response.json({ diff: '', restart: false, backup: 'b', decision_id: 'D-1', problems: [] }),
    );
    expect((await sendConfig(staging, 'why', false, fetcher, null)).error).toMatch(/token/);
    expect(fetcher).not.toHaveBeenCalled();
    const sent = await sendConfig(staging, 'why', false, fetcher, 'abc');
    expect(sent.written?.decision_id).toBe('D-1');
    const init = (fetcher.mock.calls[0] as unknown as [string, RequestInit])[1];
    expect((init.headers as Record<string, string>)['X-Conductor-Token']).toBe('abc');
  });

  it('reads the problems of a refusal', async () => {
    const fetcher = vi.fn(async () =>
      Response.json(
        { refusal: 'Invalid', detail: 'not valid', problems: [{ path: 'instances[0].cadence.watch', message: 'bad' }] },
        { status: 422 },
      ),
    );
    const sent = await sendConfig(emptyStaging(), 'why', true, fetcher, 'abc');
    expect(sent.error).toBe('not valid');
    expect(sent.problems).toEqual([{ path: 'instances[0].cadence.watch', message: 'bad' }]);
  });
});

const PAGE: ConfigPage = {
  file: '~/.b10x/conductor/conductor.yaml',
  instance: 'example-org',
  index: 0,
  fields: FIELDS,
  instances: [{ name: 'example-org', dashboard_port: 7313, session_prefix: 'eo', forge: 'github', current: true }],
  repositories: [{ repository: 'alpha', activity: 'Active', decided_by: 'Snapshot', marked_by: null, reason: null }],
  errors: [],
  writable: true,
};

describe('Config', () => {
  it('shows every section, disables Write on a bad value and enables it on a good one', async () => {
    const fetcher = vi.fn(async (_input: string, init?: RequestInit) =>
      init?.method === 'POST'
        ? Response.json({ diff: '', restart: false, backup: null, decision_id: null, problems: [] })
        : Response.json(PAGE),
    );
    const wrapper = mount(Config, { props: { fetcher, token: 'abc' } });
    await flushPromises();
    const titles = wrapper.findAll('h2').map((title) => title.text());
    for (const title of ['Instance', 'Sources & repositories', 'Controllers', 'Cadence', 'Authority', 'Staged changes']) {
      expect(titles).toContain(title);
    }
    const write = () => wrapper.find('button.primary');
    expect(write().attributes('disabled')).toBeDefined();
    const watchInput = wrapper.find('input[id="f-cadence.watch"]');
    await watchInput.setValue('soon');
    expect(wrapper.text()).toContain('cadence.watch: a duration');
    expect(write().attributes('disabled')).toBeDefined();
    await watchInput.setValue('1m');
    await flushPromises();
    expect(wrapper.find('pre.diff').text()).toContain('+   watch: 1m');
    expect(write().attributes('disabled')).toBeUndefined();
    await wrapper.find('button[role="switch"]').trigger('click');
    expect(write().attributes('disabled')).toBeDefined();
    await wrapper.find('input[aria-label="alpha reason"]').setValue('paused');
    expect(write().attributes('disabled')).toBeUndefined();
  });
});
