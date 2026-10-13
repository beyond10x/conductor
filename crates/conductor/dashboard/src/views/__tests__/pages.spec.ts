// story:dashboard-polish: every page, read from the fixtures, checked by axe-core (no serious or
// critical finding), held to one landmark structure, and kept as a snapshot; the palette of both
// themes held to WCAG AA contrast (axe cannot measure colour in jsdom, which lays nothing out);
// and no rule that is wider than a 390 px screen.
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { flushPromises, mount } from '@vue/test-utils';
import axe from 'axe-core';
import App from '../../App.vue';
import { readFileSync, readdirSync } from 'node:fs';
import { join } from 'node:path';
import { NOW, answers } from './overview-fixtures';
import { seriesResponse } from './series-fixtures';
import waitingOn from './fixtures/waiting-on.json';
import sessions from './fixtures/sessions.json';
import blockers from './fixtures/blockers.json';
import specifications from './fixtures/specifications.json';
import health from './fixtures/health.json';

/** The app's sources, read from disk: Vite hands a test no CSS, even as `?raw`. */
const SRC = join(process.cwd(), 'src');
const css = readFileSync(join(SRC, 'style.css'), 'utf8');

/** Every endpoint the pages read, by path; `/api/series` and `/api/kpis` from their fixtures. */
function everything(): Record<string, unknown> {
  return {
    ...answers(),
    '/api/waiting-on': waitingOn,
    '/api/sessions': sessions,
    '/api/blockers': blockers,
    '/api/specifications': specifications,
    '/api/health': health,
  };
}

function fixtureFetch() {
  const all = everything();
  return vi.fn(async (input: RequestInfo | URL) => {
    const url = String(input);
    const series = seriesResponse(url);
    if (series !== null) {
      return series;
    }
    const path = url.split('?')[0]!;
    return path in all ? Response.json(all[path]) : new Response('nope', { status: 404 });
  });
}

const PAGES: [string, string][] = [
  ['overview', '#/'],
  ['repository', '#/repo/alpha'],
  ['metrics', '#/metrics'],
  ['kpis', '#/kpis'],
  ['sessions', '#/sessions'],
  ['health', '#/health'],
];

beforeEach(() => {
  vi.useFakeTimers({ toFake: ['Date', 'setInterval', 'clearInterval'] });
  vi.setSystemTime(NOW);
});

afterEach(() => {
  vi.useRealTimers();
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
  window.location.hash = '';
});

describe.each(PAGES)('the %s page', (_name, hash) => {
  it('has no axe finding, one main, one banner and the navigation', async () => {
    window.location.hash = hash;
    vi.stubGlobal('fetch', fixtureFetch());
    const host = document.createElement('div');
    document.body.appendChild(host);
    const wrapper = mount(App, { attachTo: host });
    await flushPromises();

    expect(document.querySelectorAll('main')).toHaveLength(1);
    expect(document.querySelectorAll('header.top')).toHaveLength(1);
    expect(document.querySelectorAll('nav[aria-label="Views"]')).toHaveLength(1);
    expect(document.querySelectorAll('main h1')).toHaveLength(1);

    const results = await axe.run(document.body, {
      // Colour needs a layout jsdom does not have; the palette test below holds it instead.
      rules: { 'color-contrast': { enabled: false } },
    });
    // axe ran: it passed rules on this page, so an empty list below is a finding and not a miss.
    expect(results.passes.length).toBeGreaterThan(10);
    // The story asks for no serious finding; every page has none of any impact, and keeps it so.
    const findings = results.violations.map((violation) => `${violation.id}: ${violation.nodes.map((node) => node.target.join(' ')).join(', ')}`);
    expect(findings).toEqual([]);

    expect(wrapper.html()).toMatchSnapshot();
    wrapper.unmount();
  });
});

/** The custom properties of the first block whose selector is `selector`. */
function palette(selector: string): Record<string, string> {
  const at = css.indexOf(`${selector} {`);
  expect(at, selector).toBeGreaterThanOrEqual(0);
  const body = css.slice(at, css.indexOf('}', at));
  const found: Record<string, string> = {};
  for (const match of body.matchAll(/--([a-z0-9-]+):\s*(#[0-9a-f]{6})/gi)) {
    found[match[1]!] = match[2]!;
  }
  return found;
}

function luminance(hex: string): number {
  const [r, g, b] = [1, 3, 5].map((at) => {
    const channel = parseInt(hex.slice(at, at + 2), 16) / 255;
    return channel <= 0.03928 ? channel / 12.92 : ((channel + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * r! + 0.7152 * g! + 0.0722 * b!;
}

function contrast(one: string, two: string): number {
  const [light, dark] = [luminance(one), luminance(two)].sort((a, b) => b - a);
  return (light! + 0.05) / (dark! + 0.05);
}

const THEMES: [string, string][] = [
  ['light', ":root[data-theme='light']"],
  ['dark', ":root[data-theme='dark']"],
];

describe.each(THEMES)('the %s palette', (_name, selector) => {
  const colours = palette(selector);

  it('writes text at 4.5:1 or more on every surface it sits on', () => {
    const low: string[] = [];
    const text = ['text', 'soft', 'dim', 'muted', 'faint', 'link', 'blue', 'green', 'amber', 'amber-strong', 'red'];
    for (const fore of text) {
      for (const back of ['bg', 'surface', 'header', 'chip-active']) {
        const ratio = contrast(colours[fore]!, colours[back]!);
        if (ratio < 4.5) {
          low.push(`${fore} on ${back}: ${ratio.toFixed(2)}`);
        }
      }
    }
    for (const [fore, back] of [
      ['red', 'danger-surface'],
      ['red', 'red-bg'],
      ['amber', 'warn-surface'],
      ['green-fg', 'green-bg'],
      ['text', 'warn-surface'],
      ['text', 'danger-surface'],
    ] as const) {
      const ratio = contrast(colours[fore]!, colours[back]!);
      if (ratio < 4.5) {
        low.push(`${fore} on ${back}: ${ratio.toFixed(2)}`);
      }
    }
    expect(low).toEqual([]);
  });

  it('draws each chart colour at 3:1 or more on a panel', () => {
    const charts = Object.keys(colours).filter((name) => name.startsWith('chart-'));
    expect(charts.length).toBeGreaterThanOrEqual(6);
    for (const name of charts) {
      expect(contrast(colours[name]!, colours.surface!), name).toBeGreaterThanOrEqual(3);
    }
  });
});

describe('the themes', () => {
  it('follow the system setting, and the dark one is the same with or without it', () => {
    expect(css).toMatch(/@media \(prefers-color-scheme: dark\) \{\s*:root:not\(\[data-theme='light'\]\) \{/);
    const system = css.slice(css.indexOf('@media (prefers-color-scheme: dark)'));
    const block = system.slice(system.indexOf('{', system.indexOf(':root')) + 1, system.indexOf('}'));
    const dark = palette(":root[data-theme='dark']");
    for (const [name, value] of Object.entries(dark)) {
      expect(block, name).toContain(`--${name}: ${value}`);
    }
  });
});

describe('the spacing and type scale', () => {
  it('is one set of custom properties, and no stylesheet writes a font size of its own', () => {
    expect(css.length).toBeGreaterThan(1000);
    for (const step of ['--space-1', '--space-2', '--space-3', '--space-4', '--space-5', '--space-6']) {
      expect(css).toContain(`${step}:`);
    }
    for (const step of ['--text-xs', '--text-sm', '--text-md', '--text-base', '--text-lg', '--text-xl', '--text-2xl']) {
      expect(css).toContain(`${step}:`);
    }
    const sheets = { 'style.css': css, ...components() };
    const own: string[] = [];
    for (const [file, sheet] of Object.entries(sheets)) {
      for (const match of sheet.matchAll(/font-size:\s*([^;]+);/g)) {
        if (!match[1]!.trim().startsWith('var(--text-')) {
          own.push(`${file}: font-size: ${match[1]}`);
        }
      }
    }
    expect(own).toEqual([]);
  });
});

/** The `<style>` of every component, by file. */
function components(): Record<string, string> {
  const styles: Record<string, string> = {};
  const files = readdirSync(SRC, { recursive: true, encoding: 'utf8' }).filter((file) => file.endsWith('.vue'));
  expect(files.length).toBeGreaterThan(30);
  for (const file of files) {
    const style = /<style[^>]*>([\s\S]*?)<\/style>/.exec(readFileSync(join(SRC, file), 'utf8'));
    if (style !== null) {
      styles[file] = style[1]!;
    }
  }
  return styles;
}

describe('a 390 px screen', () => {
  it('fits every fixed width: none wider than the 358 px a page has inside its padding', () => {
    const wide: string[] = [];
    for (const [file, sheet] of Object.entries({ 'style.css': css, ...components() })) {
      for (const match of sheet.matchAll(/(?:^|[;{\s])(min-width|width|flex-basis):\s*(\d+)px/g)) {
        if (Number(match[2]) > 358) {
          wide.push(`${file}: ${match[1]}: ${match[2]}px`);
        }
      }
      for (const match of sheet.matchAll(/minmax\((\d+)px/g)) {
        if (Number(match[1]) > 358) {
          wide.push(`${file}: minmax(${match[1]}px`);
        }
      }
    }
    expect(wide).toEqual([]);
  });

  it('narrows the page padding and lets a table scroll inside its frame', () => {
    expect(css).toMatch(/@media \(max-width: 640px\)/);
    expect(css).toMatch(/\.table-wrap \{[^}]*overflow: auto/);
  });
});

describe('a table', () => {
  it('keeps its header in view and sets numbers to the right in figures of one width', () => {
    expect(css).toMatch(/thead th \{[^}]*position: sticky;[^}]*top: 0/);
    expect(css).toMatch(/\.num \{[^}]*text-align: right;[^}]*font-variant-numeric: tabular-nums/);
  });
});

describe('focus', () => {
  it('is drawn on everything that takes it, the charts included', () => {
    expect(css).toMatch(/:focus-visible \{[^}]*outline: 2px solid var\(--focus\)/);
    expect(css).toContain('--focus:');
  });
});

describe('the theme button', () => {
  it('follows the system until pinned, then pins light, dark, and back to the system', async () => {
    localStorage.removeItem('conductor-dashboard-theme');
    vi.stubGlobal('fetch', fixtureFetch());
    const wrapper = mount(App);
    await flushPromises();
    const root = document.documentElement;
    const button = wrapper.find('button.theme');
    expect(root.hasAttribute('data-theme')).toBe(false);
    expect(button.text()).toBe('Theme: system');
    await button.trigger('click');
    expect(root.getAttribute('data-theme')).toBe('light');
    expect(localStorage.getItem('conductor-dashboard-theme')).toBe('light');
    await button.trigger('click');
    expect(root.getAttribute('data-theme')).toBe('dark');
    await button.trigger('click');
    expect(root.hasAttribute('data-theme')).toBe(false);
    expect(localStorage.getItem('conductor-dashboard-theme')).toBeNull();
    wrapper.unmount();
  });
});
