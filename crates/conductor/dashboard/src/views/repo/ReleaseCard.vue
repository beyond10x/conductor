<script setup lang="ts">
import type { ReleaseRow } from '../../api/repo';

defineProps<{ release: { latest: ReleaseRow | null; since: number | null } }>();
</script>

<template>
  <div class="card release">
    <div class="small muted">release</div>
    <template v-if="release.latest !== null">
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
  font-size: 18px;
  font-weight: 600;
  margin: 4px 0 0;
}
.note {
  font-size: 13px;
  color: var(--soft);
  margin-top: 4px;
}
</style>
