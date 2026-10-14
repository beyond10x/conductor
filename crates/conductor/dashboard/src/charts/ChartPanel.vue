<script setup lang="ts">
// A chart under its title: the chart, or what the panel has instead: reading, no source yet with
// the story that adds one, could not read, or nothing in the range.
import { computed } from 'vue';
import { chartKind, hasValues, type PanelView } from '../api/series';
import PanelState from '../components/PanelState.vue';
import LineChart from './LineChart.vue';
import StackedBars from './StackedBars.vue';

const props = withDefaults(
  defineProps<{
    title: string;
    view: PanelView;
    compact?: boolean;
    /** The heading level: 2 on a page of charts, 3 inside a section. */
    level?: 2 | 3;
    /** What the chart covers, beside its title: a repository, all of them, a grouping. */
    scope?: string | null;
    source?: string | null;
  }>(),
  { compact: false, level: 3, scope: null, source: null },
);

const empty = computed(() => props.view.state === 'ok' && !hasValues(props.view.data));
</script>

<template>
  <section class="chart-panel card" :class="{ compact }">
    <div class="head">
      <component :is="`h${level}`">{{ title }}</component>
      <span v-if="scope !== null" class="scope small muted">{{ scope }}</span>
    </div>
    <PanelState v-if="view.state === 'loading'" state="loading">reading…</PanelState>
    <PanelState v-else-if="view.state === 'no-source'" state="no-source"
      >no source yet{{ view.story !== null ? ` · ${view.story}` : '' }}</PanelState
    >
    <PanelState v-else-if="view.state === 'error'" state="error">{{ view.message }}</PanelState>
    <PanelState v-else-if="empty" state="empty">nothing in this range</PanelState>
    <template v-else-if="view.state === 'ok'">
      <StackedBars
        v-if="chartKind(view.data.unit) === 'bars'"
        :data="view.data"
        :label="title"
        :compact="compact"
      />
      <LineChart v-else :data="view.data" :label="title" :compact="compact" />
    </template>
    <p v-if="source !== null" class="source small muted">source: {{ source }}</p>
  </section>
</template>
