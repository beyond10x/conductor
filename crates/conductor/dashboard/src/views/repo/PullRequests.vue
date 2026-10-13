<script setup lang="ts">
import { pipeline, type PullRequestRow } from '../../api/repo';

defineProps<{ rows: PullRequestRow[] }>();

const TONE = { failing: 'pill-red', passing: 'pill-green' } as const;
</script>

<template>
  <section class="col" aria-labelledby="mrs">
    <h2 id="mrs">Merge and pull requests</h2>
    <p v-if="rows.length === 0" class="muted">none open</p>
    <div v-else class="mrs">
      <div v-for="row in rows" :key="row.number" class="mr">
        <span class="id mono">#{{ row.number }}</span>
        <a v-if="row.url" :href="row.url" rel="noreferrer">{{ row.title }}</a>
        <span v-else class="title">{{ row.title }}</span>
        <span v-if="row.draft" class="pill12 pill-idle">draft</span>
        <span class="pill12 pipeline" :class="TONE[pipeline(row)]">
          {{ pipeline(row) }}<template v-if="row.failing_checks > 0"> ({{ row.failing_checks }})</template>
        </span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.mrs {
  border: 1px solid var(--border);
  border-radius: 10px;
  overflow: hidden;
}
.mr {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  border-top: 1px solid var(--row-border);
}
.mr:first-child {
  border-top: 0;
}
.mr .id {
  font-size: 12px;
  color: var(--muted);
  width: 48px;
}
.mr a,
.mr .title {
  flex: 1 1 260px;
  min-width: 0;
}
.pill12 {
  font-size: 12px;
  padding: 2px 8px;
  border-radius: 99px;
}
.pill-red {
  background: var(--red-bg);
  color: var(--red);
}
.pill-green {
  background: var(--green-bg);
  color: var(--green-fg);
}
.pill-idle {
  background: var(--border);
  color: var(--dim);
}
</style>
