<script setup lang="ts">
import type { ReleaseRow } from '../../api/repo';

import PanelState from '../../components/PanelState.vue';
withDefaults(defineProps<{ release: { latest: ReleaseRow | null; since: number | null }; error?: string | null }>(), {
  error: null,
});
</script>

<template>
  <div class="card release">
    <div class="small muted">release</div>
    <PanelState v-if="error !== null" state="error">{{ error }}</PanelState>
    <template v-else-if="release.latest !== null">
      <a class="big mono" :href="release.latest.url" rel="noreferrer">{{ release.latest.tag }}</a>
      <div class="note">
        <template v-if="release.since !== null">
          {{ release.since }} commit{{ release.since === 1 ? '' : 's' }} on main since ·
        </template>
        {{ release.latest.published_at.slice(0, 10) }}
      </div>
    </template>
    <p v-else class="big muted">no release</p>
  </div>
</template>

<style scoped>
.big {
  display: block;
  font-size: var(--text-xl);
  font-weight: 600;
  margin: var(--space-1) 0 0;
}
.note {
  font-size: var(--text-md);
  color: var(--soft);
  margin-top: var(--space-1);
}
</style>
