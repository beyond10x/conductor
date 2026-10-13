<script setup lang="ts">
// The health view, `#/health` (story:dashboard-health): doctor's rows, the newest snapshot with
// why it failed, its collector times and the members it could not read with their fixes, and the
// free disk against the floors with every disk-waste row. It reads `/api/health` every 20 s, as
// the overview reads `/data.json`.
import { onBeforeUnmount, onMounted, ref } from 'vue';
import { loadHealth, type HealthData } from '../api/health';
import DoctorPanel from './health/DoctorPanel.vue';
import SnapshotPanel from './health/SnapshotPanel.vue';
import CollectorRuns from './health/CollectorRuns.vue';
import MemberFailures from './health/MemberFailures.vue';
import DiskPanel from './health/DiskPanel.vue';
import PanelState from '../components/PanelState.vue';

/** Milliseconds between two reads, as the overview reads `/data.json`. */
const RELOAD_MS = 20_000;

const health = ref<HealthData | null>(null);
const failure = ref<string | null>(null);
let timer: ReturnType<typeof setInterval> | undefined;
/** The read in flight; an older one that ends later is not shown. */
let reading = 0;

async function load(): Promise<void> {
  const mine = ++reading;
  const read = await loadHealth();
  if (mine !== reading) {
    return;
  }
  failure.value = read.error;
  if (read.health !== null) {
    health.value = read.health;
  }
}

onMounted(() => {
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
  <div class="health">
    <h1 class="page">Health</h1>
    <PanelState v-if="failure !== null" state="error" card>{{ failure }}</PanelState>
    <PanelState v-if="health === null && failure === null" state="loading" card
      >reading /api/health…</PanelState
    >
    <template v-if="health !== null">
      <ul v-if="health.errors.length > 0" class="lines c-red errors">
        <li v-for="error in health.errors" :key="error">{{ error }}</li>
      </ul>
      <div class="split">
        <div class="main-col">
          <SnapshotPanel :snapshot="health.snapshot" :failures="health.member_failures.length" />
          <MemberFailures :failures="health.member_failures" />
          <CollectorRuns :runs="health.collector_runs" />
        </div>
        <div class="side-col">
          <DoctorPanel :doctor="health.doctor" />
          <DiskPanel :disk="health.disk" />
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.health {
  display: flex;
  flex-direction: column;
  gap: var(--space-5);
}
.split {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-5);
}
.main-col,
.side-col {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}
.main-col {
  flex: 999 1 560px;
}
.side-col {
  flex: 1 1 340px;
}
</style>
