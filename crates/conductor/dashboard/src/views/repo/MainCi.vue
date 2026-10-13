<script setup lang="ts">
import { computed } from 'vue';
import { clock, type WorkflowRunRow } from '../../api/repo';

const props = defineProps<{ run: WorkflowRunRow | null }>();

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
    <div class="big" :class="shown.tone">{{ shown.text }}</div>
    <div v-if="run !== null" class="note">{{ run.workflow }} · {{ clock(run.run_at) }}</div>
  </div>
</template>

<style scoped>
.big {
  font-size: 18px;
  font-weight: 600;
  margin-top: 4px;
}
.note {
  font-size: 13px;
  color: var(--soft);
  margin-top: 4px;
}
.c-green {
  color: var(--green);
}
.c-blue {
  color: var(--blue);
}
</style>
