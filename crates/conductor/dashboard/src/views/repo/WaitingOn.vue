<script setup lang="ts">
import type { Cause } from '../../api/repo';

import PanelState from '../../components/PanelState.vue';
withDefaults(defineProps<{ causes: Cause[]; error?: string | null }>(), {
  error: null,
});

/** How the page names a cause. */
const LABEL: Record<Cause['cause'], string> = {
  operator: 'the operator',
  ci: 'CI',
  'build-slot': 'a build slot',
  'another-repository': 'another repository',
  'usage-limit': 'the usage limit',
  silent: 'silent',
  stalled: 'stalled',
  'build-queued': 'a queued build',
};

function label(cause: Cause): string {
  return cause.cause === 'another-repository' && cause.on ? cause.on : LABEL[cause.cause];
}
</script>

<template>
  <div class="card waiting" :class="{ warn: causes.length > 0 }">
    <div class="small muted">waiting on</div>
    <PanelState v-if="error !== null" state="error">{{ error }}</PanelState>
    <p v-else-if="causes.length === 0" class="big muted state state-empty">nothing</p>
    <ul v-else class="causes">
      <li v-for="(cause, at) in causes" :key="at" :data-cause="cause.cause">
        <span class="big c-amber">{{ label(cause) }}</span>
        <span class="note">{{ cause.detail }}</span>
      </li>
    </ul>
  </div>
</template>

<style scoped>
.causes {
  list-style: none;
  margin: var(--space-1) 0 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.big {
  font-size: var(--text-xl);
  font-weight: 600;
  margin: var(--space-1) 0 0;
}
.note {
  display: block;
  font-size: var(--text-md);
  color: var(--soft);
}
</style>
