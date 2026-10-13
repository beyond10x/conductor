<script setup lang="ts">
import { computed } from 'vue';
import { clock, newestFirst, type EventRow } from '../../api/overview';

const props = defineProps<{ events: EventRow[] }>();

const lines = computed(() => newestFirst(props.events));
</script>

<template>
  <section aria-labelledby="today" class="col">
    <h2 id="today">Today</h2>
    <p v-if="lines.length === 0" class="card muted">nothing yet today</p>
    <ol v-else class="timeline">
      <li v-for="(event, at) in lines" :key="at" :class="`kind-${event.kind}`">
        <span class="tdot" aria-hidden="true"></span>
        <div class="small muted mono">
          <span class="when">{{ clock(event.at)
          }}<template v-if="event.repository !== null"> · {{ event.repository }}</template></span>
          <span class="kind">{{ event.kind }}</span>
        </div>
        <div class="what">
          <a v-if="event.link" :href="event.link" rel="noreferrer">{{ event.text }}</a>
          <template v-else>{{ event.text }}</template>
        </div>
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
  position: relative;
  padding: 6px 0 12px 18px;
}
.tdot {
  position: absolute;
  left: -6px;
  top: 10px;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  background: var(--dim);
}
.kind {
  margin-left: 8px;
}
.kind-release .tdot,
.kind-merge .tdot {
  background: var(--green);
}
.kind-decision .tdot {
  background: var(--amber);
}
.kind-report .tdot {
  background: var(--link);
}
</style>
