// The dashboard's adapter of the UI iteration loop (the ui-iteration-loop skill, § 1,
// story:dashboard-ui-loop). The generic core (contract.ts, measure.ts, ratchet.ts, gate.ts,
// round.ts) reads the dashboard only through this file and the scenarios under ux/scenarios/.
//
// - scenarios: one per page, ux/scenarios/<page>/scenario.json, driven state by state from the
//   store fixtures (fixtures.ts): loading, error, empty, full, and on the Decisions page the
//   answered state;
// - gating viewports 1440x900 and 1280x800; measured-only 1024x768, 768x1024 and 390x844;
// - themes: light and dark, the two the theme button pins (src/style.css);
// - always visible: the header, its instance selector and the page navigation;
// - completion state: an answered decision on the Decisions page;
// - no guidance cue (no L0, no gate 6) and no protected focus.
import { readFileSync, readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export const UX_DIR = dirname(fileURLToPath(import.meta.url));
export const APP_DIR = dirname(UX_DIR);
export const DIST_DIR = join(APP_DIR, 'dist');

export interface Viewport {
  name: string;
  width: number;
  height: number;
  gating: boolean;
}

export const VIEWPORTS: Viewport[] = [
  { name: '1440x900', width: 1440, height: 900, gating: true },
  { name: '1280x800', width: 1280, height: 800, gating: true },
  { name: '1024x768', width: 1024, height: 768, gating: false },
  { name: '768x1024', width: 768, height: 1024, gating: false },
  { name: '390x844', width: 390, height: 844, gating: false },
];
export const GATING = VIEWPORTS.filter((viewport) => viewport.gating);

export type Theme = 'light' | 'dark';
export const THEMES: Theme[] = ['light', 'dark'];
/** Where the app keeps a pinned theme (src/App.vue). */
export const THEME_KEY = 'conductor-dashboard-theme';

export const AXE_TAGS = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];

/** The elements that must be wholly inside the viewport and uncovered in every state. */
export const ALWAYS_VISIBLE: { kind: string; selector: string }[] = [
  { kind: 'header', selector: 'header.top' },
  { kind: 'instance-selector', selector: 'header.top select[aria-label="Instance"]' },
  { kind: 'navigation', selector: 'header.top nav[aria-label="Views"]' },
];

/** The instance the page shows (`?instance=`), and the instant every state is taken at. */
export const INSTANCE = 'alpha';
export const NOW = '2026-10-13T09:30:00Z';
/** The answer token the daemon writes into the page it serves (`<meta name="conductor-token">`). */
export const TOKEN = 'a'.repeat(64);

export type StoreState = 'loading' | 'error' | 'empty' | 'full';

/** One step of the screen-only user: a control found by its role and accessible name. */
export interface Step {
  role: 'button' | 'link' | 'radio' | 'checkbox';
  name: string;
  /** A CSS selector the control is looked for in, when the name alone is not unique. */
  within?: string;
}

export interface State {
  name: string;
  store: StoreState;
  /** The steps that take the page from the state before to this one. */
  act?: Step[];
}

export interface Scenario {
  name: string;
  page: string;
  anchor: boolean;
  states: State[];
  /** What the page shows when the task is done: a selector whose text matches `text`. */
  completion?: { selector: string; text: string };
  expectations: { maxActions: number };
}

/** The page's URL path for `scenario`, below the served origin. */
export function pagePath(scenario: Scenario): string {
  return `/?instance=${INSTANCE}${scenario.page}`;
}

export function scenarios(): Scenario[] {
  const dir = join(UX_DIR, 'scenarios');
  return readdirSync(dir)
    .sort()
    .map((name) => ({ name, ...(JSON.parse(readFileSync(join(dir, name, 'scenario.json'), 'utf8')) as Omit<Scenario, 'name'>) }));
}
