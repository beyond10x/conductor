<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { cell, gib } from './format';
import type { DashboardData } from './types';
import SessionsPanel from './panels/SessionsPanel.vue';
import WatchPanel from './panels/WatchPanel.vue';
import RedCiPanel from './panels/RedCiPanel.vue';
import AwaitingPanel from './panels/AwaitingPanel.vue';
import DecisionsPanel from './panels/DecisionsPanel.vue';
import DiskWastePanel from './panels/DiskWastePanel.vue';
import TokenSpendPanel from './panels/TokenSpendPanel.vue';
import ErrorsPanel from './panels/ErrorsPanel.vue';
import Overview from './views/Overview.vue';
import Repo from './views/Repo.vue';
import Health from './views/Health.vue';

/** The view a hash names: `#/` the overview, `#/repo/<name>`, `#/health`, `#/sessions`. */
type Route = { view: 'overview' } | { view: 'repo'; name: string } | { view: 'health' } | { view: 'sessions' };

function routeOf(hash: string): Route {
  const path = hash.replace(/^#/, '') || '/';
  const repo = /^\/repo\/(.+)$/.exec(path);
  if (repo !== null) {
    return { view: 'repo', name: decodeURIComponent(repo[1] ?? '') };
  }
  if (path === '/health') {
    return { view: 'health' };
  }
  if (path === '/sessions') {
    return { view: 'sessions' };
  }
  return { view: 'overview' };
}

const route = ref<Route>(routeOf(window.location.hash));
function followHash(): void {
  route.value = routeOf(window.location.hash);
}

/** Milliseconds between two reads of `/data.json`, as the page reloaded itself before. */
const RELOAD_MS = 20_000;

const THEME_KEY = 'conductor-dashboard-theme';

const data = ref<DashboardData | null>(null);
const failure = ref<string | null>(null);
const theme = ref<'dark' | 'light'>('dark');
let timer: ReturnType<typeof setInterval> | undefined;

async function load(): Promise<void> {
  try {
    const answer = await fetch('/data.json', { cache: 'no-store' });
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

function applyTheme(next: 'dark' | 'light'): void {
  theme.value = next;
  document.documentElement.setAttribute('data-theme', next);
  try {
    localStorage.setItem(THEME_KEY, next);
  } catch {
    // A browser without storage keeps the theme for this page only.
  }
}

function toggleTheme(): void {
  applyTheme(theme.value === 'dark' ? 'light' : 'dark');
}

onMounted(() => {
  let stored: string | null = null;
  try {
    stored = localStorage.getItem(THEME_KEY);
  } catch {
    stored = null;
  }
  applyTheme(stored === 'light' ? 'light' : 'dark');
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
        :aria-label="`Switch to ${theme === 'dark' ? 'light' : 'dark'} theme`"
        @click="toggleTheme"
      >
        {{ theme === 'dark' ? 'Light' : 'Dark' }}
      </button>
    </div>
  </header>
  <main id="main">
    <Overview v-if="route.view === 'overview'" />
    <Repo v-else-if="route.view === 'repo'" :name="route.name" />
    <Health v-else-if="route.view === 'health'" />
    <h1 v-if="route.view === 'sessions'" class="page">What every session works on</h1>
    <template v-if="route.view === 'sessions' && data !== null">
      <p class="summary small muted">
        read {{ data.generated_at }} · free on {{ data.disk.path }}: {{ gib(data.disk.free_bytes) }}
        of {{ gib(data.disk.size_bytes) }} · controllers running: {{ data.controllers }} · build
        slot: {{ cell(data.build_slot) }}
      </p>
      <SessionsPanel :data="data" />
      <WatchPanel :data="data" />
      <RedCiPanel :data="data" />
      <AwaitingPanel :data="data" />
      <DecisionsPanel :data="data" />
      <DiskWastePanel :data="data" />
      <TokenSpendPanel :data="data" />
      <ErrorsPanel :data="data" />
    </template>
  </main>
  <footer v-if="data !== null" class="small muted">
    sessions from <code>{{ data.reading.sessions }}</code> · records under {{ data.reading.root }} ·
    CI from {{ data.reading.ci_state }} · reads again every 20 s
  </footer>
</template>
