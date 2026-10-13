<script setup lang="ts">
import { cell } from '../../format';
import type { Session } from '../../types';

defineProps<{ session: Session | null }>();
</script>

<template>
  <div class="card working">
    <div class="small muted">working on</div>
    <template v-if="session !== null && session.dispatch !== null">
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
  font-size: 16px;
  font-weight: 600;
  margin: 4px 0 0;
}
.note {
  font-size: 13px;
  color: var(--soft);
  margin-top: 4px;
}
.report {
  margin-top: 6px;
}
</style>
