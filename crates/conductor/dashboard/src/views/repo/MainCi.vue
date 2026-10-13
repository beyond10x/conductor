<script setup lang="ts">
import { computed } from 'vue';
import { clock, type WorkflowRunRow } from '../../api/repo';

import PanelState from '../../components/PanelState.vue';
const props = withDefaults(defineProps<{ run: WorkflowRunRow | null; error?: string | null }>(), {
  error: null,
});

/** The run's conclusion as the page names it, and its colour. */
const shown = computed(() => {
  const conclusion = props.run?.conclusion;
  switch (conclusion) {
    case 'Success':
      return { text: 'green', tone: 'c-green' };
    case 'Failure':
      return { text: 'red', tone: 'c-red' };
    case 'Pending':
      return { text: 'running', tone: 'c-blue' };
    case undefined:
      return { text: 'no run', tone: 'muted' };
    default:
      return { text: conclusion.toLowerCase(), tone: 'muted' };
  }
});
</script>

<template>
  <div class="card main-ci">
    <div class="small muted">main</div>
    <PanelState v-if="error !== null" state="error">{{ error }}</PanelState>
    <div v-else class="big" :class="shown.tone">{{ shown.text }}</div>
    <div v-if="error === null && run !== null" class="note">{{ run.workflow }} · {{ clock(run.run_at) }}</div>
  </div>
</template>

<style scoped>
.big {
  font-size: var(--text-xl);
  font-weight: 600;
  margin-top: var(--space-1);
}
.note {
  font-size: var(--text-md);
  color: var(--soft);
  margin-top: var(--space-1);
}
.c-green {
  color: var(--green);
}
.c-blue {
  color: var(--blue);
}
</style>
