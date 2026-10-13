import type { VueWrapper } from '@vue/test-utils';
import raw from '../src/fixtures/data.json';
import type { DashboardData } from '../src/types';

/** A fresh copy of the fixture, today's `/data.json` shape, for one test to change. */
export function fixture(): DashboardData {
  return structuredClone(raw) as DashboardData;
}

/** The cells of each body row of the tables in `wrapper`, as their text. */
export function rows(wrapper: VueWrapper): string[][] {
  return wrapper.findAll('tbody tr').map((row) => row.findAll('td').map((cell) => cell.text()));
}
