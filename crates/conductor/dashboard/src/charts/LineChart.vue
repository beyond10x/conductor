<script setup lang="ts">
// One line per key, for rates and durations: a day without a value breaks the line.
import ChartFrame from './ChartFrame.vue';
import type { ChartData } from '../api/series';

withDefaults(
  defineProps<{ data: ChartData; label: string; compact?: boolean; target?: number | null }>(),
  { compact: false, target: null },
);
</script>

<template>
  <ChartFrame :data="data" :label="label" kind="line" :compact="compact" :target="target" />
</template>
