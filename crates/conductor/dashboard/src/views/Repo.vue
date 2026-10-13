<script setup lang="ts">
// The repository page, `#/repo/<name>` (story:dashboard-repo-view): what one repository waits on
// and works on, its `main` CI and release, its merge and pull requests, its plan, its sessions
// and what happened to it today. It reads its endpoints every 20 s, as the overview does.
import { onBeforeUnmount, onMounted, ref, watch } from 'vue';
import { loadRepo, type RepoView } from '../api/repo';
import WaitingOn from './repo/WaitingOn.vue';
import WorkingOn from './repo/WorkingOn.vue';
import MainCi from './repo/MainCi.vue';
import ReleaseCard from './repo/ReleaseCard.vue';
import PullRequests from './repo/PullRequests.vue';
import PlanPanel from './repo/PlanPanel.vue';
import RepoSessions from './repo/RepoSessions.vue';
import TodayEvents from './repo/TodayEvents.vue';
import RepoTrends from './repo/RepoTrends.vue';
import PanelState from '../components/PanelState.vue';
import { errorFor } from '../api/errors';

/** Milliseconds between two reads, as the overview reads `/data.json`. */
const RELOAD_MS = 20_000;

const props = defineProps<{ name: string }>();

const view = ref<RepoView | null>(null);
let timer: ReturnType<typeof setInterval> | undefined;
/** The read in flight; an older one that ends later is not shown. */
let reading = 0;

async function load(): Promise<void> {
  const mine = ++reading;
  const read = await loadRepo(props.name);
  if (mine === reading) {
    view.value = read;
  }
}

/** The error of the endpoints `paths`, for the panel that reads them. */
function error(paths: string[]): string | null {
  return view.value === null ? null : errorFor(view.value.errors, paths);
}

onMounted(() => {
  void load();
  timer = setInterval(() => void load(), RELOAD_MS);
});

watch(
  () => props.name,
  () => {
    view.value = null;
    void load();
  },
);

onBeforeUnmount(() => {
  if (timer !== undefined) {
    clearInterval(timer);
  }
});
</script>

<template>
  <div class="repo-page">
    <div class="repo-head">
      <a href="#/" class="dim">← Overview</a>
      <h1 class="title">{{ name }}</h1>
    </div>
    <PanelState v-if="view === null" state="loading" card>reading {{ name }}…</PanelState>
    <template v-else>
      <ul v-if="view.errors.length > 0" class="lines c-red errors">
        <li v-for="error in view.errors" :key="error">{{ error }}</li>
      </ul>
      <section aria-label="Status" class="status">
        <WaitingOn :causes="view.waiting" :error="error(['/api/waiting-on'])" />
        <WorkingOn :session="view.working" :error="error(['/data.json'])" />
        <MainCi :run="view.main" :error="error(['/api/workflow-runs'])" />
        <ReleaseCard :release="view.release" :error="error(['/api/releases', '/api/repositories'])" />
      </section>
      <RepoTrends :name="name" />
      <div class="split">
        <div class="main-col">
          <PullRequests :rows="view.pullRequests" :error="error(['/api/pull-requests'])" />
          <PlanPanel
            :blockers="view.blockers"
            :specification="view.specification"
            :error="error(['/api/blockers', '/api/specifications'])"
          />
        </div>
        <div class="side-col">
          <RepoSessions :rows="view.sessions" :error="error(['/api/sessions'])" />
          <TodayEvents :events="view.events" :error="error(['/api/events'])" />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.repo-page {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
.repo-head {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-4);
}
.repo-head .title {
  font-weight: 600;
  font-size: var(--text-xl);
  margin: 0;
}
.status {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(240px, 100%), 1fr));
  gap: var(--space-3);
}
.split {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-5);
}
.main-col {
  flex: 999 1 560px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
.side-col {
  flex: 1 1 340px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
</style>
