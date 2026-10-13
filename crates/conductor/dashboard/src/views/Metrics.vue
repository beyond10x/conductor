<script setup lang="ts">
// The Metrics page, `#/metrics` (story:dashboard-charts-ui): a chart per series `/api/series`
// lists, grouped by repository where the series is, filtered to one repository on request, over
// 7, 30 or 90 days. A series without a source says so with the story that adds it.
import { computed, onMounted, ref } from 'vue';
import {
  RANGES,
  chartData,
  loadSeries,
  loadSeriesList,
  panelView,
  type PanelView,
  type SeriesInfo,
  type SeriesRead,
} from '../api/series';
import ChartPanel from '../charts/ChartPanel.vue';
import PanelState from '../components/PanelState.vue';

const list = ref<SeriesInfo[] | null>(null);
const listError = ref<string | null>(null);
const range = ref<number>(30);
const repository = ref<string>('');
const reads = ref<Record<string, SeriesRead | null>>({});
const now = ref(new Date());
/** The read in flight; an older one that ends later is not shown. */
let reading = 0;

/** The grouping a series is read by: the repository where it has one, else its first, else none. */
function byOf(info: SeriesInfo): string | null {
  return info.by.includes('repository') ? 'repository' : (info.by[0] ?? null);
}

async function loadAll(): Promise<void> {
  const mine = ++reading;
  const series = list.value ?? [];
  now.value = new Date();
  reads.value = Object.fromEntries(series.map((info) => [info.series, null]));
  await Promise.all(
    series.map(async (info) => {
      const read = await loadSeries(info.series, byOf(info), range.value);
      if (mine === reading) {
        reads.value = { ...reads.value, [info.series]: read };
      }
    }),
  );
}

async function load(): Promise<void> {
  const read = await loadSeriesList();
  list.value = read.list;
  listError.value = read.error;
  await loadAll();
}

function choose(days: number): void {
  if (range.value !== days) {
    range.value = days;
    void loadAll();
  }
}

/** Every repository a series grouped by repository names. */
const repositories = computed(() => {
  const names = new Set<string>();
  for (const info of list.value ?? []) {
    const read = reads.value[info.series];
    if (byOf(info) === 'repository' && read?.state === 'ok') {
      for (const row of read.answer.rows) {
        names.add(row.key);
      }
    }
  }
  if (repository.value !== '') {
    names.add(repository.value);
  }
  return [...names].sort((one, two) => one.localeCompare(two));
});

function view(info: SeriesInfo): PanelView {
  const by = byOf(info);
  const only = by === 'repository' && repository.value !== '' ? repository.value : null;
  return panelView(reads.value[info.series] ?? null, (answer) =>
    chartData(answer, now.value, range.value, only === null ? undefined : (key) => key === only),
  );
}

function scope(info: SeriesInfo): string {
  const by = byOf(info);
  if (by === 'repository') {
    return repository.value === '' ? 'by repository' : repository.value;
  }
  return by === null ? 'all repositories' : `by ${by} · all repositories`;
}

function source(info: SeriesInfo): string | null {
  if (info.source === null) {
    return null;
  }
  return info.kpi !== null ? `${info.source} · ${info.kpi}` : info.source;
}

onMounted(() => void load());
</script>

<template>
  <div class="metrics">
    <h1 class="page">Metrics</h1>
    <div class="controls" role="group" aria-label="What the charts show">
      <label class="small muted" for="metrics-repository">Repository</label>
      <select id="metrics-repository" v-model="repository">
        <option value="">all repositories</option>
        <option v-for="name in repositories" :key="name" :value="name">{{ name }}</option>
      </select>
      <div class="segmented range" role="group" aria-label="Range">
        <button
          v-for="days in RANGES"
          :key="days"
          type="button"
          class="chip-button"
          :aria-pressed="range === days ? 'true' : 'false'"
          @click="choose(days)"
        >
          {{ days }} days
        </button>
      </div>
    </div>
    <PanelState v-if="list === null" state="loading" card>reading /api/series…</PanelState>
    <PanelState v-else-if="listError !== null" state="error" card>{{ listError }}</PanelState>
    <PanelState v-else-if="list.length === 0" state="empty" card>no series answered</PanelState>
    <div v-else class="chart-grid">
      <ChartPanel
        v-for="info in list"
        :key="info.series"
        :title="info.title"
        :view="view(info)"
        :level="2"
        :scope="scope(info)"
        :source="source(info)"
      />
    </div>
  </div>
</template>

<style scoped>
.metrics {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
</style>
