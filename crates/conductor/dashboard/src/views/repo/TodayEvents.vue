<script setup lang="ts">
import { clock, type EventRow } from '../../api/repo';

defineProps<{ events: EventRow[] }>();
</script>

<template>
  <section class="col" aria-labelledby="today-here">
    <h2 id="today-here">Today here</h2>
    <p v-if="events.length === 0" class="muted">nothing yet today</p>
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
  padding: 6px 0 12px 18px;
}
</style>
