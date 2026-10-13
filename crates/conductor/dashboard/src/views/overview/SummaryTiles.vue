<script setup lang="ts">
import type { Tile } from '../../api/overview';

defineProps<{ tiles: Tile[] }>();
</script>

<template>
  <section aria-label="Summary" class="tiles">
    <div v-for="tile in tiles" :key="tile.label" class="card tile" :class="`tone-${tile.tone}`">
      <div class="label small muted">{{ tile.label }}</div>
      <div class="val">{{ tile.value }}</div>
      <div class="sub">{{ tile.sub }}</div>
    </div>
  </section>
</template>

<style scoped>
.tiles {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
  gap: 12px;
}
.tile .val {
  font-size: 26px;
  font-weight: 600;
  margin-top: 4px;
}
.tile .sub {
  font-size: 12px;
  color: var(--muted);
  margin-top: 2px;
}
.tone-amber .val {
  color: var(--amber);
}
.tone-red .val {
  color: var(--red);
}
.tone-green .val {
  color: var(--green);
}
.tone-amber {
  border-color: var(--warn-border);
}
.tone-red {
  border-color: var(--danger-border);
}
</style>
