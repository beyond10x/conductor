<script setup lang="ts">
// The member failures of the health view: each member a collector could not read in the newest
// snapshot, the first line of its error, its class and what to do about it.
import type { MemberFailure } from '../../api/health';

defineProps<{ failures: MemberFailure[] }>();
</script>

<template>
  <section class="col part member-failures" aria-labelledby="health-failures">
    <h2 id="health-failures">members a collector could not read</h2>
    <p v-if="failures.length === 0" class="card muted">no member failed</p>
    <div
      v-for="(failure, at) in failures"
      v-else
      :key="at"
      class="card warn failure"
    >
      <div class="row">
        <span class="who">{{ failure.collector }} · <b>{{ failure.member }}</b></span>
        <span class="class pill">{{ failure.class }}</span>
      </div>
      <code class="line">{{ failure.error_line }}</code>
      <p class="fix">{{ failure.fix }}</p>
    </div>
  </section>
</template>

<style scoped>
.failure {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.warn {
  background: var(--warn-surface);
  border-color: var(--warn-border);
}
.row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  align-items: baseline;
}
.pill {
  font-size: var(--text-xs);
  padding: 2px var(--space-2);
  border-radius: var(--radius-pill);
  border: 1px solid currentColor;
  color: var(--amber);
}
.line {
  font-size: var(--text-sm);
  color: var(--dim);
  background: var(--header);
  padding: var(--space-2) var(--space-2);
  border-radius: var(--radius-sm);
  display: block;
  overflow-x: auto;
  white-space: pre-wrap;
  word-break: break-word;
}
.fix {
  margin: 0;
  color: var(--soft);
}
</style>
