// The UI iteration loop's adapter for the dashboard (story:dashboard-ui-loop, skill ui-iteration-loop
// § 1). Everything the generic gates need to know about this app is here: the viewports, the
// themes, the always-visible elements, and the hermetic scenarios, each a page driven state by
// state from the fixtures under src/ (no server, no network, no keys).

export interface Viewport {
  name: string;
  width: number;
  height: number;
}

/** Every merge checks these (hard gates 1, 2 and 5). */
export const GATING: Viewport[] = [
  { name: '1440x900', width: 1440, height: 900 },
  { name: '1280x800', width: 1280, height: 800 },
];

/** Reported by the round command, not gated. */
export const MEASURED: Viewport[] = [
  { name: '1024x768', width: 1024, height: 768 },
  { name: '768x1024', width: 768, height: 1024 },
  { name: '390x844', width: 390, height: 844 },
];

/** The themes the app ships: the theme button pins one with data-theme on <html>. */
export const THEMES = ['light', 'dark'] as const;
export type Theme = (typeof THEMES)[number];

/** The key the app keeps a pinned theme under (src/App.vue). */
export const THEME_KEY = 'conductor-dashboard-theme';

/**
 * The always-visible elements: the header with the instance selector and the page navigation.
 * Each must be 100 % inside the viewport, not clipped and not covered, at every state.
 */
export const ALWAYS_VISIBLE: { name: string; selector: string }[] = [
  { name: 'header', selector: 'header.top' },
  { name: 'instance selector', selector: 'header.top .instance-select' },
  { name: 'page navigation', selector: 'header.top nav.views' },
  { name: 'navigation link', selector: 'header.top nav.views a' },
];

/** The axe-core tags hard gate 5 runs. */
export const AXE_TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

/**
 * How the fixtures answer in a state: `loaded` from the fixtures, `empty` with no rows, `error`
 * 500 on every endpoint of the page, `loading` never answering. The header's `/api/instances` is
 * answered in every state, so the instance selector is checked in all of them.
 */
export type Mode = 'loaded' | 'empty' | 'error' | 'loading';

export interface State {
  name: string;
  mode: Mode;
  /** An interaction after the page settles: `answer` answers REQ-release Yes and sends it. */
  then?: 'answer';
}

export interface Scenario {
  name: string;
  hash: string;
  states: State[];
}

const ALL: State[] = [
  { name: 'loaded', mode: 'loaded' },
  { name: 'empty', mode: 'empty' },
  { name: 'error', mode: 'error' },
  { name: 'loading', mode: 'loading' },
];

export const SCENARIOS: Scenario[] = [
  { name: 'overview', hash: '#/', states: ALL },
  { name: 'repository', hash: '#/repo/alpha', states: ALL },
  {
    name: 'decisions',
    hash: '#/decisions',
    // The completion state of the Decisions page: an answered decision.
    states: [...ALL, { name: 'answered', mode: 'loaded', then: 'answer' }],
  },
  { name: 'kpis', hash: '#/kpis', states: ALL },
  {
    name: 'health',
    hash: '#/health',
    states: [
      { name: 'loaded', mode: 'loaded' },
      { name: 'error', mode: 'error' },
      { name: 'loading', mode: 'loading' },
    ],
  },
  { name: 'metrics', hash: '#/metrics', states: ALL },
  { name: 'sessions', hash: '#/sessions', states: ALL },
];

/** The instant every scenario runs at: 09:30 UTC of the fixtures' day. */
export const NOW = new Date('2026-10-13T09:30:00Z');

/**
 * The screen-only user's tasks (skill § 7), each from the landing page with visible controls only.
 * Their truth is in ux/screen-only.spec.ts beside the steps.
 */
export const TASKS = [
  'task-answer-decision',
  'task-failing-check',
  'task-switch-instance',
  'task-kpi-target',
] as const;
