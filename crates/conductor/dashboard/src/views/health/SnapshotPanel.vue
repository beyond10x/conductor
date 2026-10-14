<script setup lang="ts">
// The snapshot part of the health view: the newest snapshot in any state, why it failed, and how
// many members its collectors could not read.
import { computed } from 'vue';
import { clock, type SnapshotHealth } from '../../api/health';

const props = defineProps<{ snapshot: SnapshotHealth | null; failures: number }>();

/** The snapshot's state in words, with the failed members of a complete one, and its colour. */
const shown = computed(() => {
  const snapshot = props.snapshot;
  if (snapshot === null) {
    return { text: 'no snapshot', tone: 'muted' };
  }
  switch (snapshot.state) {
    case 'Complete':
      return props.failures > 0
        ? {
            text: `complete · ${props.failures} member${props.failures === 1 ? '' : 's'} failed`,
            tone: 'c-amber',
          }
        : { text: 'complete', tone: 'c-green' };
    case 'Failed':
      return { text: 'failed', tone: 'c-red' };
    case 'Collecting':
      return { text: 'collecting', tone: 'c-blue' };
    default:
      return { text: snapshot.state.toLowerCase(), tone: 'muted' };
  }
});
</script>

<template>
  <section class="card part snapshot" aria-labelledby="health-snapshot">
    <div class="row">
      <h2 id="health-snapshot">last snapshot</h2>
      <span class="state" :class="shown.tone">{{ shown.text }}</span>
    </div>
    <p v-if="snapshot !== null" class="small muted">
      started {{ clock(snapshot.started_at) }} · <code>{{ snapshot.snapshot_id }}</code>
    </p>
    <p v-if="snapshot !== null && snapshot.failure_reason !== null" class="reason c-red">
      {{ snapshot.failure_reason }}
    </p>
  </section>
</template>

<style scoped>
.row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  align-items: baseline;
}
.part {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.c-green {
  color: var(--green);
}
.c-amber {
  color: var(--amber);
}
.c-blue {
  color: var(--link);
}
</style>
