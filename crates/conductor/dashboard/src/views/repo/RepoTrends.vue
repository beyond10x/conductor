<script setup lang="ts">
// The repository page's trends (story:dashboard-charts-ui): that repository's PRs merged, its CI
// runs passed and failed stacked, and its releases, each per day over 30 days.
import { computed, onMounted, ref, watch } from 'vue';
import {
  chartData,
  loadSeries,
  panelView,
  type ChartData,
  type PanelView,
  type SeriesRead,
} from '../../api/series';
import ChartPanel from '../../charts/ChartPanel.vue';

const props = defineProps<{ name: string }>();

/** The days the trends cover. */
const DAYS = 30;

const now = ref(new Date());
const merged = ref<SeriesRead | null>(null);
const passed = ref<SeriesRead | null>(null);
const failed = ref<SeriesRead | null>(null);
const releases = ref<SeriesRead | null>(null);
/** The read in flight; an older one that ends later is not shown. */
let reading = 0;

async function load(): Promise<void> {
  const mine = ++reading;
  now.value = new Date();
  const reads: [typeof merged, string][] = [
    [merged, 'prs_merged'],
    [passed, 'ci_passed'],
    [failed, 'ci_failed'],
    [releases, 'releases'],
  ];
  for (const [into] of reads) {
    into.value = null;
  }
  await Promise.all(
    reads.map(async ([into, series]) => {
      const read = await loadSeries(series, 'repository', DAYS);
      if (mine === reading) {
        into.value = read;
      }
    }),
  );
}

/** One series of this repository only. */
const mine = (read: SeriesRead | null): PanelView =>
  panelView(read, (answer) => chartData(answer, now.value, DAYS, (key) => key === props.name));

/** This repository's line of a series, named `label`, or none when it has no row. */
function lineOf(data: ChartData, label: string, tone: 'good' | 'bad') {
  const line = data.lines.find((candidate) => candidate.key === props.name);
  return line === undefined ? [] : [{ key: label, label, values: line.values, tone }];
}

const runs = computed<PanelView>(() => {
  const both = [passed.value, failed.value];
  if (both.some((read) => read === null)) {
    return { state: 'loading' };
  }
  for (const read of both) {
    if (read !== null && read.state !== 'ok') {
      return read;
    }
  }
  if (passed.value?.state !== 'ok' || failed.value?.state !== 'ok') {
    return { state: 'loading' };
  }
  const good = chartData(passed.value.answer, now.value, DAYS);
  const bad = chartData(failed.value.answer, now.value, DAYS);
  return {
    state: 'ok',
    data: {
      days: good.days,
      unit: 'count',
      lines: [...lineOf(good, 'passed', 'good'), ...lineOf(bad, 'failed', 'bad')],
    },
  };
});

onMounted(() => void load());
watch(
  () => props.name,
  () => void load(),
);
</script>

<template>
  <section class="col trends" aria-labelledby="repo-trends">
    <h2 id="repo-trends">Trends · {{ DAYS }} days</h2>
    <div class="chart-grid">
      <ChartPanel title="PRs merged" :view="mine(merged)" compact />
      <ChartPanel title="CI runs" :view="runs" compact />
      <ChartPanel title="Releases" :view="mine(releases)" compact />
    </div>
  </section>
</template>
