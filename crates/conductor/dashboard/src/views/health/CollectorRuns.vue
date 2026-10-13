<script setup lang="ts">
// The collector times of the health view: each collector's run in the newest snapshot, as
// `conductor snapshot collector-runs` lists them, in the order they started.
import { computed } from 'vue';
import { clock, duration, words, type CollectorRun } from '../../api/health';

const props = defineProps<{ runs: CollectorRun[] }>();

const ordered = computed(() =>
  [...props.runs].sort((a, b) => a.started_at.localeCompare(b.started_at)),
);

const summary = computed(() =>
  ordered.value.map((run) => `${run.collector} ${duration(run.duration_ms)}`).join(' · '),
);
</script>

<template>
  <section class="col part collector-runs" aria-labelledby="health-collectors">
    <div class="row">
      <h2 id="health-collectors">collector times</h2>
      <span v-if="runs.length > 0" class="summary small mono">{{ summary }}</span>
    </div>
    <p v-if="runs.length === 0" class="card muted">no collector runs in this snapshot</p>
    <div v-else class="table-wrap">
      <table aria-labelledby="health-collectors">
        <thead>
          <tr>
            <th scope="col">collector</th>
            <th scope="col">started</th>
            <th scope="col" class="num">took</th>
            <th scope="col">outcome</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="run in ordered"
            :key="run.collector + run.started_at"
            :class="{ 'c-red': run.outcome !== 'Completed' }"
          >
            <td>{{ run.collector }}</td>
            <td class="mono small">{{ clock(run.started_at) }}</td>
            <td class="num mono small">{{ duration(run.duration_ms) }}</td>
            <td>{{ words(run.outcome) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  align-items: baseline;
  flex-wrap: wrap;
}
.mono {
  font-family: ui-monospace, monospace;
}
table {
  min-width: 0;
}
</style>
