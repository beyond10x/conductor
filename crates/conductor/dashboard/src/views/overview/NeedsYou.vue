<script setup lang="ts">
import type { NeedCard } from '../../api/overview';

import PanelState from '../../components/PanelState.vue';
withDefaults(defineProps<{ cards: NeedCard[]; error?: string | null }>(), { error: null });
</script>

<template>
  <section aria-labelledby="needs" class="col">
    <h2 id="needs">Needs you · {{ cards.length }}</h2>
    <PanelState v-if="error !== null" state="error" card>{{ error }}</PanelState>
    <PanelState v-else-if="cards.length === 0" state="empty">nothing needs you</PanelState>
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
        <a v-if="card.link !== null" class="small" :href="card.link">open it on the Decisions page</a>
      </div>
    </div>
  </section>
</template>

<style scoped>
.needs {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(min(360px, 100%), 1fr));
  gap: var(--space-3);
}
.card.stack {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.card.warn {
  background: var(--warn-surface);
  border-color: var(--warn-border);
}
.head {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.pill {
  font-size: var(--text-xs);
  padding: 2px var(--space-2);
  border-radius: var(--radius-pill);
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
  font-size: var(--text-sm);
  color: var(--dim);
  background: var(--header);
  padding: var(--space-2) var(--space-2);
  border-radius: var(--radius-sm);
  display: block;
  overflow-x: auto;
  user-select: all;
}
</style>
