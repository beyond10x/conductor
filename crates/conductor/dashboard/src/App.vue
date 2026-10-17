<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { cell, gib } from './format';
import type { DashboardData } from './types';
import SessionsPanel from './panels/SessionsPanel.vue';
import WatchPanel from './panels/WatchPanel.vue';
import AwaitingPanel from './panels/AwaitingPanel.vue';
import DecisionsPanel from './panels/DecisionsPanel.vue';
import TokenSpendPanel from './panels/TokenSpendPanel.vue';
import ErrorsPanel from './panels/ErrorsPanel.vue';
import PanelState from './components/PanelState.vue';
import SnapshotAge from './components/SnapshotAge.vue';
import Overview from './views/Overview.vue';
import Repo from './views/Repo.vue';
import Health from './views/Health.vue';
import Metrics from './views/Metrics.vue';
import Kpis from './views/Kpis.vue';
import Decisions from './views/Decisions.vue';

/**
 * The view a hash names: `#/` the overview, `#/decisions` (story:decisions-page) and
 * `#/decisions/<id>`, the page at that open item (story:sessions-page-coherent), `#/repo/<name>`,
 * `#/metrics`, `#/kpis`, `#/health`, `#/sessions`.
 */
type Route =
  | { view: 'overview' }
  | { view: 'decisions'; item: string | null }
  | { view: 'repo'; name: string }
  | { view: 'metrics' }
  | { view: 'kpis' }
  | { view: 'health' }
  | { view: 'sessions' };

function routeOf(hash: string): Route {
  const path = hash.replace(/^#/, '') || '/';
  const repo = /^\/repo\/(.+)$/.exec(path);
  if (repo !== null) {
    return { view: 'repo', name: decodeURIComponent(repo[1] ?? '') };
  }
  const item = /^\/decisions\/(.+)$/.exec(path);
  if (item !== null) {
    return { view: 'decisions', item: decodeURIComponent(item[1] ?? '') };
  }
  switch (path) {
    case '/decisions':
      return { view: 'decisions', item: null };
    case '/metrics':
      return { view: 'metrics' };
    case '/kpis':
      return { view: 'kpis' };
    case '/health':
      return { view: 'health' };
    case '/sessions':
      return { view: 'sessions' };
    default:
      return { view: 'overview' };
  }
}

const route = ref<Route>(routeOf(window.location.hash));
function followHash(): void {
  route.value = routeOf(window.location.hash);
}

/** Milliseconds between two reads of `/data.json`, as the page reloaded itself before. */
const RELOAD_MS = 20_000;

const THEME_KEY = 'conductor-dashboard-theme';

/** The theme: the system's setting, or one the button pinned. */
type Theme = 'system' | 'light' | 'dark';
const NEXT: Record<Theme, Theme> = { system: 'light', light: 'dark', dark: 'system' };

const data = ref<DashboardData | null>(null);
const failure = ref<string | null>(null);
const theme = ref<Theme>('system');
let timer: ReturnType<typeof setInterval> | undefined;

async function load(): Promise<void> {
  try {
    const answer = await fetch('/data.json', { cache: 'no-cache' });
    if (!answer.ok) {
      failure.value = `could not read /data.json: ${answer.status}`;
      return;
    }
    data.value = (await answer.json()) as DashboardData;
    failure.value = null;
  } catch (error) {
    failure.value = `could not read /data.json: ${String(error)}`;
  }
}

function applyTheme(next: Theme): void {
  theme.value = next;
  if (next === 'system') {
    document.documentElement.removeAttribute('data-theme');
  } else {
    document.documentElement.setAttribute('data-theme', next);
  }
  try {
    if (next === 'system') {
      localStorage.removeItem(THEME_KEY);
    } else {
      localStorage.setItem(THEME_KEY, next);
    }
  } catch {
    // A browser without storage keeps the theme for this page only.
  }
}

function toggleTheme(): void {
  applyTheme(NEXT[theme.value]);
}

onMounted(() => {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(THEME_KEY);
  } catch {
    stored = null;
  }
  applyTheme(stored === 'light' || stored === 'dark' ? stored : 'system');
  window.addEventListener('hashchange', followHash);
  void load();
  timer = setInterval(() => void load(), RELOAD_MS);
});

onBeforeUnmount(() => {
  window.removeEventListener('hashchange', followHash);
  if (timer !== undefined) {
    clearInterval(timer);
  }
});
</script>

<template>
  <header class="top">
    <div class="brand">
      <svg
        width="22"
        height="22"
        viewBox="0 0 24 24"
        fill="none"
        stroke="currentColor"
        class="brand-mark"
        stroke-width="2"
        stroke-linecap="round"
        aria-hidden="true"
      >
        <path d="M4 18c3-8 13-8 16 0" />
        <path d="M8 18c2-4 6-4 8 0" />
        <circle cx="12" cy="18" r="1.5" />
      </svg>
      <span class="brand-name">conductor</span>
    </div>
    <nav class="views" aria-label="Views">
      <a href="#/" :aria-current="route.view === 'overview' ? 'page' : undefined">Overview</a>
      <a href="#/decisions" :aria-current="route.view === 'decisions' ? 'page' : undefined">Decisions</a>
      <a href="#/metrics" :aria-current="route.view === 'metrics' ? 'page' : undefined">Metrics</a>
      <a href="#/kpis" :aria-current="route.view === 'kpis' ? 'page' : undefined">KPIs</a>
      <a href="#/sessions" :aria-current="route.view === 'sessions' ? 'page' : undefined">Sessions</a>
      <a href="#/health" :aria-current="route.view === 'health' ? 'page' : undefined">Health</a>
    </nav>
    <div class="head-right">
      <span class="live">
        <span class="dot8" :class="{ down: failure !== null }" aria-hidden="true"></span>
        <template v-if="failure !== null">{{ failure }}</template>
        <template v-else-if="data !== null">live · read {{ data.generated_at }}</template>
        <template v-else>reading /data.json</template>
      </span>
      <button
        type="button"
        class="theme"
        :aria-label="`Theme: ${theme}. Switch to ${NEXT[theme]}`"
        @click="toggleTheme"
      >
        Theme: {{ theme }}
      </button>
    </div>
  </header>
  <main id="main" tabindex="-1">
    <Overview v-if="route.view === 'overview'" />
    <Decisions
      v-else-if="route.view === 'decisions'"
      :key="route.item ?? ''"
      :item="route.item"
    />
    <Repo v-else-if="route.view === 'repo'" :name="route.name" />
    <Metrics v-else-if="route.view === 'metrics'" />
    <Kpis v-else-if="route.view === 'kpis'" />
    <Health v-else-if="route.view === 'health'" />
    <template v-else>
      <h1 class="page">What every session works on</h1>
      <PanelState v-if="failure !== null" state="error" card>{{ failure }}</PanelState>
      <PanelState v-if="data === null && failure === null" state="loading" card
        >reading /data.json…</PanelState
      >
      <template v-if="data !== null">
        <p class="summary small muted">
          read {{ data.generated_at }} · sessions: {{ data.sessions.length }} · controllers
          running: {{ data.controllers }} · free on {{ data.disk.path }}:
          {{ gib(data.disk.free_bytes) }} of {{ gib(data.disk.size_bytes) }} · build slot:
          {{ cell(data.build_slot) }}
        </p>
        <SnapshotAge :snapshot="data.snapshot" />
        <p class="moved small muted">
          Red CI on main is on the <a href="#/">Overview</a>; disk waste is on
          <a href="#/health">Health</a>.
        </p>
        <SessionsPanel :data="data" />
        <AwaitingPanel :data="data" />
        <WatchPanel :data="data" />
        <DecisionsPanel :data="data" />
        <TokenSpendPanel :data="data" />
        <ErrorsPanel :data="data" />
      </template>
    </template>
  </main>
  <footer v-if="data !== null" class="small muted">
    sessions from the instance's records and <code>{{ data.reading.sessions }}</code> · records
    under {{ data.reading.root }} · reads again every 20 s
  </footer>
</template>
