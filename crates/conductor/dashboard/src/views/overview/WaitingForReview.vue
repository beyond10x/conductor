<script setup lang="ts">
// The overview's first panel (story:review-limit): the pull requests that wait for a person's
// review, oldest first, each linked where the forge gave a link, with its age at the snapshot.
import { age, type ReviewRow } from '../../api/overview';
import PanelState from '../../components/PanelState.vue';

withDefaults(defineProps<{ rows: ReviewRow[]; at: string | null; error?: string | null }>(), {
  error: null,
});
</script>

<template>
  <section aria-labelledby="reviews" class="col">
    <h2 id="reviews">Waiting for review · {{ rows.length }}</h2>
    <PanelState v-if="error !== null" state="error" card>{{ error }}</PanelState>
    <PanelState v-else-if="rows.length === 0" state="empty">nothing waits for review</PanelState>
    <ol v-else class="reviews">
      <li v-for="row in rows" :key="`${row.repository}#${row.number}`">
        <b>{{ row.repository }}</b>{{ ' ' }}<a v-if="row.url !== null" :href="row.url" rel="noreferrer">#{{ row.number }}</a>
        <template v-else>#{{ row.number }}</template>
        {{ row.title }} <span class="muted">— {{ age(row.opened_at, at) }}</span>
      </li>
    </ol>
  </section>
</template>

<style scoped>
.reviews {
  margin: 0;
  padding-left: var(--space-5);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
</style>
