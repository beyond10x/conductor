// story:dashboard-repo-view: the repository page, `#/repo/<name>`, each part over fixtures whose
// keys crates/conductor/tests/dashboard_repo.rs holds to the endpoints the server answers.
import { afterEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import Repo from '../Repo.vue';
import WaitingOn from '../repo/WaitingOn.vue';
import WorkingOn from '../repo/WorkingOn.vue';
import MainCi from '../repo/MainCi.vue';
import ReleaseCard from '../repo/ReleaseCard.vue';
import PullRequests from '../repo/PullRequests.vue';
import PlanPanel from '../repo/PlanPanel.vue';
import RepoSessions from '../repo/RepoSessions.vue';
import TodayEvents from '../repo/TodayEvents.vue';
import {
  blockersByKind,
  filterWaiting,
  mainCi,
  newestFirst,
  pullRequestsOf,
  releaseOf,
  specificationOf,
  todaySince,
  waitingFor,
  workingOn,
  type EventRow,
  type RepoSession,
  type WaitingRow,
} from '../../api/repo';
import type { DashboardData } from '../../types';
import data from '../../fixtures/data.json';
import waitingOn from './fixtures/waiting-on.json';
import sessions from './fixtures/sessions.json';
import blockers from './fixtures/blockers.json';
import specifications from './fixtures/specifications.json';
import pullRequests from './fixtures/pull-requests.json';
import workflowRuns from './fixtures/workflow-runs.json';
import releases from './fixtures/releases.json';
import repositories from './fixtures/repositories.json';
import events from './fixtures/events.json';

const waiting = waitingOn.rows as WaitingRow[];

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
});

describe('the overview filter "waiting"', () => {
  it('shows the one repository of three that /api/waiting-on answers a cause for', () => {
    expect(filterWaiting(['alpha', 'beta', 'gamma'], waiting)).toEqual(['alpha']);
  });
});

describe('today', () => {
  it('is 00:00 UTC of the UTC day, whatever the zone of the instant', () => {
    expect(todaySince(new Date('2026-10-13T23:30:00-05:00'))).toBe('2026-10-14T00:00:00Z');
    expect(todaySince(new Date('2026-10-13T00:00:00Z'))).toBe('2026-10-13T00:00:00Z');
  });
});

describe('waiting on', () => {
  it("lists each of the repository's causes, naming the repository it waits for", () => {
    const wrapper = mount(WaitingOn, { props: { causes: waitingFor(waiting, 'alpha') } });
    const causes = wrapper.findAll('li');
    expect(causes.map((cause) => cause.attributes('data-cause'))).toEqual([
      'operator',
      'another-repository',
    ]);
    expect(causes[0]?.text()).toContain('the operator');
    expect(causes[0]?.text()).toContain('licence-less OpenAPI document');
    expect(causes[1]?.find('.big').text()).toBe('beta');
  });

  it('says nothing for a repository without a cause', () => {
    const wrapper = mount(WaitingOn, { props: { causes: waitingFor(waiting, 'beta') } });
    expect(wrapper.findAll('li')).toHaveLength(0);
    expect(wrapper.text()).toContain('nothing');
  });
});

describe('working on', () => {
  it("shows the controller's newest dispatch, its goal and brief, and its newest report", () => {
    const session = workingOn(data as DashboardData, 'alpha');
    const wrapper = mount(WorkingOn, { props: { session } });
    expect(wrapper.find('.mid').text()).toBe('G1');
    expect(wrapper.find('.brief').text()).toBe('Fix <b>bold</b> & "quoted" refund retries');
    expect(wrapper.find('.brief b').exists()).toBe(false);
    expect(wrapper.find('.report').text()).toContain('DSP-20261013-01');
    expect(wrapper.find('.report').text()).toContain('started, 5m ago');
  });

  it('names a repository without a controller session', () => {
    const wrapper = mount(WorkingOn, {
      props: { session: workingOn(data as DashboardData, 'gamma') },
    });
    expect(wrapper.text()).toContain('no controller session');
  });
});

describe('main CI and release', () => {
  it('shows the newest main run', () => {
    const wrapper = mount(MainCi, { props: { run: mainCi(workflowRuns.rows, 'alpha') } });
    expect(wrapper.find('.big').text()).toBe('green');
    expect(wrapper.find('.note').text()).toBe('CI · 08:38Z');
  });

  it('shows the newest release and the commits since it', () => {
    const release = releaseOf(releases.rows, repositories.rows, 'alpha');
    const wrapper = mount(ReleaseCard, { props: { release } });
    const tag = wrapper.find('a');
    expect(tag.text()).toBe('v2.14.0');
    expect(tag.attributes('href')).toBe(
      'https://forge.example.org/example-org/alpha/releases/v2.14.0',
    );
    expect(wrapper.find('.note').text()).toContain('6 commits on main since');
  });
});

describe('merge and pull requests', () => {
  it('shows each with its pipeline status and a link to its url, newest first', () => {
    const wrapper = mount(PullRequests, {
      props: { rows: pullRequestsOf(pullRequests.rows, 'alpha') },
    });
    const rows = wrapper.findAll('.mr');
    expect(rows.map((row) => row.find('.id').text())).toEqual(['#221', '#219']);
    expect(rows[0]?.find('.pipeline').text()).toBe('failing (2)');
    expect(rows[1]?.find('.pipeline').text()).toBe('passing');
    expect(rows[0]?.find('a').attributes('href')).toBe(
      'https://forge.example.org/example-org/alpha/pull/221',
    );
    expect(wrapper.text()).not.toContain("not alpha's");
  });
});

describe('the plan', () => {
  it('shows the open blockers by kind and the specification', () => {
    const wrapper = mount(PlanPanel, {
      props: {
        blockers: blockersByKind(blockers.rows, 'alpha'),
        specification: specificationOf(specifications.rows, 'alpha'),
      },
    });
    const kinds = wrapper.findAll('.kind');
    expect(kinds.map((kind) => kind.attributes('data-kind'))).toEqual([
      'decision-blocker',
      'upstream-blocker',
    ]);
    expect(kinds[1]?.find('.small').text()).toBe('upstream-blocker (2)');
    expect(kinds[1]?.text()).toContain('story:partner-sandbox');
    expect(wrapper.text()).not.toContain("not alpha's");
    const specification = wrapper.find('.specification').text();
    expect(specification).toContain('present');
    expect(specification).toContain('ess/22');
    expect(specification).toContain('valid');
    expect(specification).toContain('214 scenarios');
  });
});

describe('sessions', () => {
  it("shows each session's name, state, start, dispatch, spend today and transcript to copy", async () => {
    const writeText = vi.fn(async () => undefined);
    vi.stubGlobal('navigator', { clipboard: { writeText } });
    const rows = sessions.rows as RepoSession[];
    const wrapper = mount(RepoSessions, { props: { rows } });
    const cards = wrapper.findAll('.session');
    expect(cards.map((card) => card.find('.name').text())).toEqual(['alpha', 'alpha-w01']);
    expect(cards[0]?.find('.state').text()).toBe('working');
    expect(cards[1]?.find('.state').text()).toBe('exited');
    expect(cards[0]?.find('.started').text()).toBe('07:30Z');
    expect(cards[0]?.find('.dispatch').text()).toBe('DSP-20261013-01 · Fix the refund retries');
    expect(cards[1]?.find('.dispatch').text()).toBe('—');
    expect(cards[0]?.find('.spend').text()).toBe('42 calls · 37,000 tokens');
    const path = rows[0]?.transcript ?? '';
    expect(cards[0]?.find('.transcript').text()).toBe(path);
    await cards[0]?.find('button.copy').trigger('click');
    await flushPromises();
    expect(writeText).toHaveBeenCalledWith(path);
    expect(cards[0]?.find('button.copy').text()).toBe('copied');
  });
});

describe('today here', () => {
  it('shows each event of the day, newest first, with its time and kind, linked where it has one', () => {
    const wrapper = mount(TodayEvents, {
      props: { events: newestFirst(events.rows as EventRow[]) },
    });
    const items = wrapper.findAll('li');
    expect(items.map((item) => [item.find('.time').text(), item.find('.kind').text()])).toEqual([
      ['11:02Z', 'decision'],
      ['10:44Z', 'release'],
      ['10:40Z', 'report'],
      ['09:57Z', 'merge'],
      ['08:12Z', 'dispatch'],
    ]);
    expect(items[1]?.find('a').attributes('href')).toBe(
      'https://forge.example.org/example-org/alpha/releases/v2.14.1',
    );
    expect(items[0]?.find('a').exists()).toBe(false);
  });
});

describe('the repository page', () => {
  it("reads every endpoint for its repository and today's events since 00:00 UTC", async () => {
    vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
    vi.setSystemTime(new Date('2026-10-13T11:30:00Z'));
    const answers: Record<string, unknown> = {
      '/data.json': data,
      '/api/waiting-on': waitingOn,
      '/api/pull-requests': pullRequests,
      '/api/workflow-runs': workflowRuns,
      '/api/releases': releases,
      '/api/repositories': repositories,
      '/api/blockers': blockers,
      '/api/specifications': specifications,
      '/api/sessions?repository=alpha': sessions,
      '/api/events?since=2026-10-13T00%3A00%3A00Z&repository=alpha': events,
    };
    const fetch = vi.fn(async (path: string) =>
      path in answers
        ? new Response(JSON.stringify(answers[path]))
        : new Response('not found', { status: 404 }),
    );
    vi.stubGlobal('fetch', fetch);
    const wrapper = mount(Repo, { props: { name: 'alpha' } });
    await flushPromises();
    expect(fetch.mock.calls.map(([path]) => path).sort()).toEqual(Object.keys(answers).sort());
    expect(wrapper.find('h1').text()).toBe('alpha');
    expect(wrapper.find('.errors').exists()).toBe(false);
    expect(wrapper.findAll('h2').map((heading) => heading.text())).toEqual([
      'Merge and pull requests',
      'Plan',
      'Sessions',
      'Today here',
    ]);
    expect(wrapper.find('.waiting').text()).toContain('the operator');
    expect(wrapper.findAll('.mr')).toHaveLength(2);
    expect(wrapper.findAll('.session')).toHaveLength(2);
    expect(wrapper.findAll('ol.timeline li')).toHaveLength(5);
    wrapper.unmount();
  });

  it('names an endpoint it could not read and shows the others', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (path: string) =>
        path === '/api/waiting-on'
          ? new Response('nope', { status: 500 })
          : new Response(JSON.stringify(path === '/data.json' ? data : { rows: [] })),
      ),
    );
    const wrapper = mount(Repo, { props: { name: 'alpha' } });
    await flushPromises();
    expect(wrapper.find('.errors').text()).toBe('could not read /api/waiting-on: 500');
    expect(wrapper.find('.working .mid').text()).toBe('G1');
    wrapper.unmount();
  });
});
