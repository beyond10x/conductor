<script setup lang="ts">
import { cell } from '../../format';
import type { Session } from '../../types';

import PanelState from '../../components/PanelState.vue';
withDefaults(defineProps<{ session: Session | null; error?: string | null }>(), {
  error: null,
});
</script>

<template>
  <div class="card working">
    <div class="small muted">working on</div>
    <PanelState v-if="error !== null" state="error">{{ error }}</PanelState>
    <template v-else-if="session !== null && session.dispatch !== null">
      <div class="mid">{{ cell(session.dispatch.goal) }}</div>
      <div class="note brief">{{ session.dispatch.brief }}</div>
      <div class="small muted report">
        <span class="mono">{{ session.dispatch.id }}</span>
        <template v-if="session.dispatch.event !== null">
          · {{ session.dispatch.event }}, {{ cell(session.dispatch.event_ago) }} ago
        </template>
        <template v-else> · no report yet</template>
      </div>
    </template>
    <p v-else-if="session !== null" class="mid muted">no dispatch</p>
    <p v-else class="mid muted">no controller session</p>
  </div>
</template>

<style scoped>
.mid {
  font-size: var(--text-lg);
  font-weight: 600;
  margin: var(--space-1) 0 0;
}
.note {
  font-size: var(--text-md);
  color: var(--soft);
  margin-top: var(--space-1);
}
.report {
  margin-top: var(--space-2);
}
</style>
