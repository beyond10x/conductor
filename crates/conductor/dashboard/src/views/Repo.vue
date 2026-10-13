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
  <header class="top repo">
    <a href="#/" class="dim">← Overview</a>
    <h1 class="title">{{ name }}</h1>
  </header>
  <main id="main" tabindex="-1">
    <p v-if="view === null" class="muted loading">reading…</p>
    <template v-else>
      <ul v-if="view.errors.length > 0" class="lines c-red errors">
        <li v-for="error in view.errors" :key="error">{{ error }}</li>
      </ul>
      <section aria-label="Status" class="status">
        <WaitingOn :causes="view.waiting" />
        <WorkingOn :session="view.working" />
        <MainCi :run="view.main" />
        <ReleaseCard :release="view.release" />
      </section>
      <div class="split">
        <div class="main-col">
          <PullRequests :rows="view.pullRequests" />
          <PlanPanel :blockers="view.blockers" :specification="view.specification" />
        </div>
        <div class="side-col">
          <RepoSessions :rows="view.sessions" />
          <TodayEvents :events="view.events" />
        </div>
      </div>
    </template>
  </main>
</template>

<style scoped>
header.repo .title {
  font-weight: 600;
  font-size: 18px;
  margin: 0;
}
.status {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(240px, 100%), 1fr));
  gap: 12px;
}
.split {
  display: flex;
  flex-wrap: wrap;
  gap: 24px;
}
.main-col {
  flex: 999 1 560px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 24px;
}
.side-col {
  flex: 1 1 340px;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 24px;
}
</style>
