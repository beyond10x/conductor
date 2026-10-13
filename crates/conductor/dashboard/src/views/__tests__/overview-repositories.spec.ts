import { describe, expect, it } from 'vitest';
import { mount } from '@vue/test-utils';
import RepositoryTable from '../overview/RepositoryTable.vue';
import { MINE_KEY, mineFrom, repositoryRows } from '../../api/overview';
import { memoryStorage, overview } from './overview-fixtures';

/** The repository names the table shows, in order. */
function names(wrapper: ReturnType<typeof mount>): string[] {
  return wrapper.findAll('tbody tr td.name a').map((cell) => cell.text());
}

function table(mine: string[]) {
  return mount(RepositoryTable, { props: { rows: repositoryRows(overview()), mine } });
}

describe('RepositoryTable', () => {
  it('has one row per repository of the board with its state chips', () => {
    const wrapper = table([]);
    expect(names(wrapper)).toEqual(['alpha', 'beta', 'delta', 'gamma']);
    const chips = wrapper
      .findAll('tbody tr')
      .map((row) => row.findAll('.chip').map((chip) => chip.text()));
    expect(chips).toEqual([['red main', 'failing PR'], ['no checkout'], ['red main'], ['ok']]);
    const alpha = wrapper.find('tbody tr');
    expect(alpha.find('td.prs').text()).toBe('2 open · 1 failing');
    expect(alpha.find('td.release').text()).toBe('v1.1.0 · +0');
    expect(alpha.find('td.controller').text()).toBe('busy');
    expect(alpha.find('td.last').text()).toBe('5m ago');
  });

  it('filters all, mine and red CI', async () => {
    const wrapper = table(['beta', 'delta']);
    const chip = (label: string) =>
      wrapper.findAll('[role="group"] button').find((button) => button.text().startsWith(label))!;
    expect(chip('All').text()).toBe('All 4');
    expect(chip('All').attributes('aria-pressed')).toBe('true');

    await chip('Mine').trigger('click');
    expect(chip('Mine').attributes('aria-pressed')).toBe('true');
    expect(names(wrapper)).toEqual(['beta', 'delta']);

    await chip('Red CI').trigger('click');
    expect(names(wrapper)).toEqual(['alpha', 'delta']);

    await chip('All').trigger('click');
    expect(names(wrapper)).toEqual(['alpha', 'beta', 'delta', 'gamma']);
  });

  it('says so when no repository matches the filter', async () => {
    const wrapper = table([]);
    await wrapper.findAll('[role="group"] button')[1]!.trigger('click');
    expect(wrapper.text()).toContain('No repositories match this filter.');
  });
});

describe('mineFrom', () => {
  it('reads ?mine= and keeps it in localStorage', () => {
    const storage = memoryStorage();
    expect(mineFrom('?mine=alpha,%20delta,', storage)).toEqual(['alpha', 'delta']);
    expect(storage.getItem(MINE_KEY)).toBe('alpha,delta');
  });

  it('reads localStorage when the address has no ?mine=', () => {
    const storage = memoryStorage({ [MINE_KEY]: 'beta,gamma' });
    expect(mineFrom('', storage)).toEqual(['beta', 'gamma']);
    expect(mineFrom('?other=1', storage)).toEqual(['beta', 'gamma']);
  });

  it('is empty with neither, and without a storage', () => {
    expect(mineFrom('', memoryStorage())).toEqual([]);
    expect(mineFrom('?mine=alpha', null)).toEqual(['alpha']);
  });
});
