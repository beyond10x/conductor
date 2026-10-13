<script setup lang="ts">
// Every KPI of `/api/kpis`, a panel each, and the trend of each measured one from its series over
// the last 30 days. The KPI page shows it whole; the overview's strip shows it compact.
import { onMounted, ref } from 'vue';
import { loadKpis, type Kpi } from '../../api/kpis';
import { chartData, loadSeries, panelView, type PanelView } from '../../api/series';
import PanelState from '../../components/PanelState.vue';
import KpiPanel from './KpiPanel.vue';

withDefaults(defineProps<{ compact?: boolean }>(), { compact: false });

/** The days a trend covers. */
const DAYS = 30;

const kpis = ref<Kpi[] | null>(null);
const error = ref<string | null>(null);
const trends = ref<Record<string, PanelView>>({});

async function load(): Promise<void> {
  const read = await loadKpis();
  kpis.value = read.kpis;
  error.value = read.error;
  const now = new Date();
  const measured = read.kpis.filter((kpi) => kpi.measured && kpi.series !== undefined);
  trends.value = Object.fromEntries(measured.map((kpi) => [kpi.id, { state: 'loading' } as PanelView]));
  await Promise.all(
    measured.map(async (kpi) => {
      const series = await loadSeries(kpi.series!, null, DAYS);
      trends.value = {
        ...trends.value,
        [kpi.id]: panelView(series, (answer) => chartData(answer, now, DAYS)),
      };
    }),
  );
}

onMounted(() => void load());
</script>

<template>
  <PanelState v-if="kpis === null" state="loading" card>reading /api/kpis…</PanelState>
  <PanelState v-else-if="error !== null" state="error" card>{{ error }}</PanelState>
  <PanelState v-else-if="kpis.length === 0" state="empty" card>no KPI answered</PanelState>
  <div v-else class="kpi-grid" :class="{ compact }">
    <KpiPanel
      v-for="kpi in kpis"
      :key="kpi.id"
      :kpi="kpi"
      :trend="kpi.measured ? (trends[kpi.id] ?? { state: 'loading' }) : null"
      :compact="compact"
      :level="compact ? 3 : 2"
    />
  </div>
</template>

<style scoped>
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(min(320px, 100%), 1fr));
  gap: var(--space-3);
}
.kpi-grid.compact {
  grid-template-columns: repeat(auto-fill, minmax(min(240px, 100%), 1fr));
}
</style>
