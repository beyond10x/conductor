<script setup lang="ts">
import { computed } from 'vue';
import { compactTokens, spentToday } from '../../api/overview';
import { gib, size } from '../../format';
import type { DashboardData } from '../../types';

import PanelState from '../../components/PanelState.vue';
const props = withDefaults(
  defineProps<{ data: DashboardData | null; error?: string | null }>(),
  { error: null },
);

const spent = computed(() => spentToday(props.data));

const summary = computed(() => {
  const free = props.data === null ? '—' : gib(props.data.disk.free_bytes);
  const tokens = spent.value === null ? '—' : compactTokens(spent.value.tokens);
  return `Disk & spend · ${free} free · ${tokens} tokens today`;
});
</script>

<template>
  <details class="disk-spend card">
    <summary>{{ summary }}</summary>
    <PanelState v-if="error !== null" state="error">{{ error }}</PanelState>
    <template v-if="data !== null">
      <p class="small muted">
        free on {{ data.disk.path }}: {{ gib(data.disk.free_bytes) }} of
        {{ gib(data.disk.size_bytes) }}<template v-if="data.disk.measured_at !== null">
          · waste {{ size(data.disk.total_bytes) }} over {{ data.disk.rows.length }} rows,
          {{ size(data.disk.reclaimable_bytes) }} reclaimable</template
        >
      </p>
      <ul class="spend small">
        <li v-for="row in data.usage.rows" :key="row.repository">
          {{ row.repository }}: {{ compactTokens(row.input + row.output) }} tokens,
          {{ row.calls }} calls
        </li>
      </ul>
    </template>
  </details>
</template>

<style scoped>
summary {
  cursor: pointer;
  color: var(--dim);
}
.spend {
  margin: var(--space-2) 0 0;
  padding-left: var(--space-4);
}
</style>
