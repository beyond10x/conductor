<script setup lang="ts">
// A bar per day, the keys stacked in it, for counts.
import ChartFrame from './ChartFrame.vue';
import type { ChartData } from '../api/series';

withDefaults(defineProps<{ data: ChartData; label: string; compact?: boolean }>(), {
  compact: false,
});
</script>

<template>
  <ChartFrame :data="data" :label="label" kind="bars" :compact="compact" />
</template>
