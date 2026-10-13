<script setup lang="ts">
import { computed } from 'vue';
import { tokens } from '../format';
import type { DashboardData } from '../types';

const props = defineProps<{ data: DashboardData }>();

/** No file of the watch's is there yet. */
const noState = computed(() => {
  const watch = props.data.watch;
  return watch.usage_limit === null && watch.usage_limit_today === null && watch.context === null;
});

/** The watch's lines as it prints them; `red` marks one that needs a look. */
const lines = computed(() => {
  const watch = props.data.watch;
  const threshold = tokens(props.data.context_handover);
  const out: { text: string; red: boolean }[] = [];
  for (const limit of watch.usage_limit ?? []) {
    out.push({
      text: `usage limit: ${limit.text}; ${limit.sessions.length} sessions: ${limit.sessions.join(' ')}`,
      red: true,
    });
  }
  for (const limit of watch.usage_limit_today ?? []) {
    out.push({ text: `usage limit seen today: ${limit}`, red: false });
  }
  for (const context of watch.context ?? []) {
    out.push({
      text: `context: ${context.name ?? 'a session'} over ${threshold} tokens (session ${context.session})`,
      red: true,
    });
  }
  return out;
});
</script>

<template>
  <section class="col" aria-labelledby="watch">
    <h2 id="watch">Usage limit and context</h2>
    <div class="card">
      <p v-if="noState" class="muted">no watch state</p>
      <p v-else-if="lines.length === 0" class="muted">none</p>
      <ul v-else class="lines">
        <li v-for="(line, at) in lines" :key="at" :class="{ 'c-red': line.red }">{{ line.text }}</li>
      </ul>
    </div>
  </section>
</template>
