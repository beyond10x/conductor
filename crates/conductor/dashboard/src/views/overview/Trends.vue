<script setup lang="ts">
// The overview's trends (story:dashboard-charts-ui): PRs merged, the CI pass rate from the runs
// passed and failed, and the operator's touches, each per day over 30 days, for every repository.
import { computed, onMounted, ref } from 'vue';
import {
  chartData,
  loadSeries,
  panelView,
  passRate,
  type PanelView,
  type SeriesRead,
} from '../../api/series';
import ChartPanel from '../../charts/ChartPanel.vue';

/** The days the trends cover. */
const DAYS = 30;

const now = ref(new Date());
const merged = ref<SeriesRead | null>(null);
const passed = ref<SeriesRead | null>(null);
const failed = ref<SeriesRead | null>(null);
const touches = ref<SeriesRead | null>(null);

async function load(): Promise<void> {
  now.value = new Date();
  const reads: [typeof merged, string][] = [
    [merged, 'prs_merged'],
    [passed, 'ci_passed'],
    [failed, 'ci_failed'],
    [touches, 'operator_touches'],
  ];
  await Promise.all(
    reads.map(async ([into, series]) => {
      into.value = await loadSeries(series, null, DAYS);
    }),
  );
}

const draw = (read: SeriesRead | null): PanelView =>
  panelView(read, (answer) => chartData(answer, now.value, DAYS));

/** The pass rate: drawn once both halves are read, else what keeps either from being drawn. */
const rate = computed<PanelView>(() => {
  const both = [passed.value, failed.value];
  if (both.some((read) => read === null)) {
    return { state: 'loading' };
  }
  for (const read of both) {
    if (read !== null && read.state !== 'ok') {
      return read;
    }
  }
  if (passed.value?.state === 'ok' && failed.value?.state === 'ok') {
    return {
      state: 'ok',
      data: passRate(
        chartData(passed.value.answer, now.value, DAYS),
        chartData(failed.value.answer, now.value, DAYS),
      ),
    };
  }
  return { state: 'loading' };
});

onMounted(() => void load());
</script>

<template>
  <section class="col trends" aria-labelledby="trends">
    <div class="row">
      <h2 id="trends">Trends · {{ DAYS }} days</h2>
      <a href="#/metrics" class="small">every metric</a>
    </div>
    <div class="chart-grid">
      <ChartPanel title="PRs merged" :view="draw(merged)" compact />
      <ChartPanel title="CI pass rate" :view="rate" compact />
      <ChartPanel title="Operator touches" :view="draw(touches)" compact />
    </div>
  </section>
</template>

<style scoped>
.row {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-3);
}
</style>
