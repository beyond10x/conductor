<script setup lang="ts">
// One KPI (story:dashboard-kpi-panels). Measured: its value over its window against its target,
// marked met or missed the way its direction reads, its daily trend from `/api/series` with the
// target drawn across it, and its source. Not measured yet: that, and the story that adds its
// source; never a number and never an empty chart.
import { computed } from 'vue';
import { formatValue, type PanelView } from '../../api/series';
import { standing, trendKey, type Kpi } from '../../api/kpis';
import LineChart from '../../charts/LineChart.vue';
import PanelState from '../../components/PanelState.vue';

const props = withDefaults(
  defineProps<{
    kpi: Kpi;
    trend: PanelView | null;
    compact?: boolean;
    /** The heading level: 2 on the KPI page, 3 in the overview's strip. */
    level?: 2 | 3;
  }>(),
  { compact: false, level: 3 },
);

/** A value as the KPI's unit writes it; a ratio rounded half up to two places. */
function shown(value: number): string {
  return props.kpi.unit === 'ratio' || props.kpi.unit === 'USD'
    ? formatValue(Math.round(value * 100) / 100, props.kpi.unit)
    : formatValue(value, props.kpi.unit);
}

const where = computed(() => standing(props.kpi));

const target = computed(() =>
  props.kpi.target === null
    ? 'target set after first measurement'
    : `target ${props.kpi.direction === 'higher' ? '≥' : '≤'} ${shown(props.kpi.target)}`,
);

/** The trend: the KPI's own key of its series, one line. */
const trendView = computed<PanelView | null>(() => {
  const trend = props.trend;
  if (trend === null || trend.state !== 'ok') {
    return trend;
  }
  const key = trendKey(trend.data.lines.map((line) => line.key));
  const line = trend.data.lines.find((candidate) => candidate.key === key);
  const lines = line === undefined ? [] : [{ ...line, label: `${props.kpi.id} per day` }];
  return { state: 'ok', data: { ...trend.data, lines } };
});

const drawable = computed(
  () =>
    trendView.value?.state === 'ok' &&
    trendView.value.data.lines.some((line) => line.values.some((value) => value !== null)),
);
</script>

<template>
  <article
    class="kpi card"
    :class="[where === 'not-measured' ? 'not-measured-kpi' : where, { compact }]"
    :aria-labelledby="`kpi-${kpi.id}`"
  >
    <component :is="`h${level}`" :id="`kpi-${kpi.id}`" class="kpi-title"
      ><span class="id mono">{{ kpi.id }}</span> · {{ kpi.title }}</component
    >
    <p v-if="!compact" class="definition small muted">{{ kpi.definition }}</p>
    <template v-if="kpi.measured && kpi.value !== null && kpi.value !== undefined">
      <div class="figures">
        <span class="value">{{ shown(kpi.value) }}</span>
        <span class="target small muted">{{ target }}</span>
        <span v-if="where === 'met' || where === 'missed'" class="standing pill small">{{
          where === 'met' ? 'meets target' : 'misses target'
        }}</span>
      </div>
      <PanelState v-if="trendView === null || trendView.state === 'loading'" state="loading"
        >reading the trend…</PanelState
      >
      <PanelState v-else-if="trendView.state === 'error'" state="error">{{
        trendView.message
      }}</PanelState>
      <PanelState v-else-if="trendView.state === 'no-source'" state="no-source"
        >no source yet{{ trendView.story !== null ? ` · ${trendView.story}` : '' }}</PanelState
      >
      <PanelState v-else-if="!drawable" state="empty">no daily value in this range</PanelState>
      <LineChart
        v-else-if="trendView.state === 'ok'"
        :data="trendView.data"
        :label="`${kpi.id} ${kpi.title}, per day`"
        :target="kpi.target"
        compact
      />
      <p class="source small muted">
        source: {{ kpi.source ?? '—' }}<template v-if="kpi.window_days !== undefined">
          · last {{ kpi.window_days }} days</template
        >
      </p>
    </template>
    <p v-else class="not-measured state state-no-source">
      not measured yet{{ kpi.story !== undefined ? ` · ${kpi.story}` : '' }}
    </p>
  </article>
</template>

<style scoped>
.kpi {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.id {
  color: var(--muted);
}
.figures {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-2);
}
.value {
  font-size: var(--text-2xl);
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}
.compact .value {
  font-size: var(--text-xl);
}
.pill {
  padding: 0 var(--space-2);
  border-radius: var(--radius-pill);
}
.met .standing {
  color: var(--green-fg);
  background: var(--green-bg);
}
.missed {
  border-color: var(--danger-border);
}
.missed .standing {
  color: var(--red);
  background: var(--red-bg);
}
.missed .value {
  color: var(--red);
}
.not-measured {
  color: var(--muted);
}
</style>
