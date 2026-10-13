<script setup lang="ts">
// The overview (`#/`, story:dashboard-overview): what waits for review (story:review-limit), the
// summary strip, what needs the operator, the repositories with their state chips, today's
// events, and disk and spend collapsed. It reads every endpoint at once on mount and every 20 s
// (src/api/overview.ts).
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  loadOverview,
  mineFrom,
  needsYou,
  repositoryRows,
  summary,
  waitingForReview,
  type OverviewData,
} from '../api/overview';
import WaitingForReview from './overview/WaitingForReview.vue';
import SummaryTiles from './overview/SummaryTiles.vue';
import NeedsYou from './overview/NeedsYou.vue';
import RepositoryTable from './overview/RepositoryTable.vue';
import TodayTimeline from './overview/TodayTimeline.vue';
import DiskSpend from './overview/DiskSpend.vue';

/** Milliseconds between two reads, as `/data.json` is read. */
const RELOAD_MS = 20_000;

const view = ref<OverviewData | null>(null);
const mine = ref<string[]>([]);
let timer: ReturnType<typeof setInterval> | undefined;

async function load(): Promise<void> {
  view.value = await loadOverview(new Date());
}

/** The browser's storage, or none where it cannot be read. */
function storage(): Storage | null {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
}

const reviews = computed(() => (view.value === null ? [] : waitingForReview(view.value)));
const tiles = computed(() => (view.value === null ? [] : summary(view.value)));
const cards = computed(() => (view.value === null ? [] : needsYou(view.value)));
const rows = computed(() => (view.value === null ? [] : repositoryRows(view.value)));

onMounted(() => {
  mine.value = mineFrom(window.location.search, storage());
  void load();
  timer = setInterval(() => void load(), RELOAD_MS);
});

onBeforeUnmount(() => {
  if (timer !== undefined) {
    clearInterval(timer);
  }
});
</script>

<template>
  <div class="overview">
    <h1 class="sr-only">Overview</h1>
    <p v-if="view === null" class="muted">reading the overview</p>
    <template v-else>
      <p class="small muted">
        <template v-if="view.board.snapshot !== null"
          >snapshot of {{ view.board.snapshot.started_at }}</template
        >
        <template v-else>no complete snapshot yet</template>
        · today since {{ view.since }}
      </p>
      <WaitingForReview :rows="reviews" :at="view.reviews.snapshot?.started_at ?? null" />
      <SummaryTiles :tiles="tiles" />
      <NeedsYou :cards="cards" />
      <RepositoryTable :rows="rows" :mine="mine" />
      <TodayTimeline :events="view.events" />
      <DiskSpend :data="view.data" />
      <ul v-if="view.errors.length > 0" class="read-errors small c-red">
        <li v-for="error in view.errors" :key="error">could not read {{ error }}</li>
      </ul>
    </template>
  </div>
</template>

<style scoped>
.overview {
  display: flex;
  flex-direction: column;
  gap: 24px;
}
.sr-only {
  position: absolute;
  width: 1px;
  height: 1px;
  overflow: hidden;
  clip: rect(0 0 0 0);
}
</style>
