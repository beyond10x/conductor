<script setup lang="ts">
// The usage limits in force, each with its reset, and the context lines of the instance's
// sessions (story:sessions-page-coherent): a limit whose reset passed is not shown.
import { computed } from 'vue';
import { tokens } from '../format';
import { clock } from '../api/overview';
import type { DashboardData } from '../types';

const props = defineProps<{ data: DashboardData }>();

/** No file of the watch's is there yet. */
const noState = computed(() => {
  const watch = props.data.watch;
  return watch.limits === null && watch.context === null;
});

/** The watch's lines; `red` marks one that needs a look. */
const lines = computed(() => {
  const watch = props.data.watch;
  const threshold = tokens(props.data.context_handover);
  const out: { text: string; red: boolean }[] = [];
  for (const limit of watch.limits ?? []) {
    const until = limit.resets_at === null ? 'reset time not read' : `until ${clock(limit.resets_at)}`;
    const sessions =
      limit.sessions.length === 0 ? '' : `; sessions: ${limit.sessions.join(' ')}`;
    out.push({ text: `usage limit ${until}: ${limit.text}${sessions}`, red: true });
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
      <p v-if="noState" class="state state-empty">no watch state</p>
      <p v-else-if="lines.length === 0" class="state state-empty">no limit in force, no context warning</p>
      <ul v-else class="lines">
        <li v-for="(line, at) in lines" :key="at" :class="{ 'c-red': line.red }">{{ line.text }}</li>
      </ul>
    </div>
  </section>
</template>
