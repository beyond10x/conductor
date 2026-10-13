<script setup lang="ts">
import { clock, type EventRow } from '../../api/repo';

import PanelState from '../../components/PanelState.vue';
withDefaults(defineProps<{ events: EventRow[]; error?: string | null }>(), {
  error: null,
});
</script>

<template>
  <section class="col" aria-labelledby="today-here">
    <h2 id="today-here">Today here</h2>
    <PanelState v-if="error !== null" state="error" card>{{ error }}</PanelState>
    <PanelState v-else-if="events.length === 0" state="empty">nothing yet today</PanelState>
    <ol v-else class="timeline">
      <li v-for="(event, at) in events" :key="at" :data-kind="event.kind">
        <div class="small muted mono">
          <span class="time">{{ clock(event.at) }}</span> · <span class="kind">{{ event.kind }}</span>
        </div>
        <a v-if="event.link" class="text" :href="event.link" rel="noreferrer">{{ event.text }}</a>
        <div v-else class="text">{{ event.text }}</div>
      </li>
    </ol>
  </section>
</template>

<style scoped>
ol.timeline {
  list-style: none;
  margin: 0;
  padding: 0;
  border-left: 2px solid var(--border);
}
ol.timeline li {
  padding: var(--space-2) 0 var(--space-3) var(--space-4);
}
</style>
