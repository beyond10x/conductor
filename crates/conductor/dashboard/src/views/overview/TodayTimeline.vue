<script setup lang="ts">
import { computed } from 'vue';
import { clock, newestFirst, type EventRow } from '../../api/overview';

import PanelState from '../../components/PanelState.vue';
const props = withDefaults(defineProps<{ events: EventRow[]; error?: string | null }>(), {
  error: null,
});

const lines = computed(() => newestFirst(props.events));
</script>

<template>
  <section aria-labelledby="today" class="col">
    <h2 id="today">Today</h2>
    <PanelState v-if="error !== null" state="error" card>{{ error }}</PanelState>
    <PanelState v-else-if="lines.length === 0" state="empty" card>nothing yet today</PanelState>
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
  padding: var(--space-2) 0 var(--space-3) var(--space-4);
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
  margin-left: var(--space-2);
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
