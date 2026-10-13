<script setup lang="ts">
import type { NeedCard } from '../../api/overview';

defineProps<{ cards: NeedCard[] }>();
</script>

<template>
  <section aria-labelledby="needs" class="col">
    <h2 id="needs">Needs you · {{ cards.length }}</h2>
    <p v-if="cards.length === 0" class="muted">nothing needs you</p>
    <div v-else class="needs">
      <div
        v-for="(card, at) in cards"
        :key="at"
        class="card stack"
        :class="card.kind === 'red-ci' ? 'danger' : 'warn'"
      >
        <div class="head">
          <span class="pill" :class="card.kind === 'red-ci' ? 'pill-red' : 'pill-amber'">{{
            card.pill
          }}</span>
        </div>
        <div>
          <b v-if="card.repository !== null">{{ card.repository }}</b>
          <template v-if="card.repository !== null">: </template>{{ card.text }}
        </div>
        <code class="cmd">{{ card.command }}</code>
      </div>
    </div>
  </section>
</template>

<style scoped>
.needs {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(360px, 100%), 1fr));
  gap: 12px;
}
.card.stack {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.card.warn {
  background: var(--warn-surface);
  border-color: var(--warn-border);
}
.head {
  display: flex;
  gap: 8px;
  align-items: center;
}
.pill {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 99px;
}
.pill-amber {
  color: var(--amber);
  border: 1px solid var(--warn-border);
}
.pill-red {
  color: var(--red);
  border: 1px solid var(--danger-border);
}
.cmd {
  font-size: 12px;
  color: var(--dim);
  background: var(--header);
  padding: 6px 8px;
  border-radius: 6px;
  display: block;
  overflow-x: auto;
  user-select: all;
}
</style>
