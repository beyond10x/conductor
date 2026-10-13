<script setup lang="ts">
import type { BlockerRow, SpecificationRow } from '../../api/repo';

import PanelState from '../../components/PanelState.vue';
withDefaults(
  defineProps<{
    blockers: Map<string, BlockerRow[]>;
    specification: SpecificationRow | null;
    error?: string | null;
  }>(),
  { error: null },
);

/** A presence as the page writes it. */
function presence(specification: SpecificationRow): string {
  return specification.presence === 'OptedOut' ? 'opted out' : specification.presence.toLowerCase();
}

/** A validation as the page writes it. */
function validation(specification: SpecificationRow): string {
  switch (specification.validation) {
    case 'Valid':
      return 'valid';
    case 'Refused':
      return `refused (${specification.validation_refusals})`;
    default:
      return 'not validated';
  }
}
</script>

<template>
  <section class="col" aria-labelledby="plan">
    <h2 id="plan">Plan</h2>
    <PanelState v-if="error !== null" state="error" card>{{ error }}</PanelState>
    <div class="plan">
      <div class="card blockers" :class="{ danger: blockers.size > 0 }">
        <div class="lbl">open blockers</div>
        <p v-if="blockers.size === 0" class="muted">none</p>
        <div v-for="[kind, rows] in blockers" :key="kind" class="kind" :data-kind="kind">
          <div class="small c-red">{{ kind }} ({{ rows.length }})</div>
          <ul>
            <li v-for="row in rows" :key="row.reference">
              <span class="mono">{{ row.reference }}</span> · {{ row.title }}
            </li>
          </ul>
        </div>
      </div>
      <div class="card specification">
        <div class="lbl">specification</div>
        <p v-if="specification === null" class="muted">not observed</p>
        <template v-else>
          <div>
            {{ presence(specification) }}
            <template v-if="specification.format"> · <span class="mono">{{ specification.format }}</span></template>
            · {{ validation(specification) }}
          </div>
          <div v-if="specification.scenarios !== null" class="small muted">
            {{ specification.scenarios }} scenarios
          </div>
        </template>
      </div>
    </div>
  </section>
</template>

<style scoped>
.plan {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: var(--space-3);
}
.lbl {
  font-size: var(--text-sm);
  color: var(--muted);
  margin-bottom: var(--space-2);
}
ul {
  list-style: none;
  margin: var(--space-1) 0 var(--space-2);
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
</style>
