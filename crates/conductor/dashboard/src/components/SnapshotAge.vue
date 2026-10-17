<script setup lang="ts">
// The age of the newest complete snapshot, which `conductor status` reads, with a warning when it
// is older than the instance's cycle, the time between two snapshots (story:sessions-page-coherent).
import { computed } from 'vue';
import type { SnapshotAge } from '../types';

const props = defineProps<{ snapshot: SnapshotAge | null }>();

/** A cadence in seconds as the config writes it: `2h`, `90m`, `45s`. */
function cadence(seconds: number): string {
  if (seconds % 3600 === 0) {
    return `${seconds / 3600}h`;
  }
  return seconds % 60 === 0 ? `${seconds / 60}m` : `${seconds}s`;
}

const text = computed(() => {
  const snapshot = props.snapshot;
  if (snapshot === null) {
    return 'no complete snapshot yet';
  }
  const age = snapshot.age === null ? 'of unknown age' : `${snapshot.age} old`;
  if (!snapshot.stale) {
    return `newest snapshot ${age}`;
  }
  const over =
    snapshot.cadence_seconds === null ? '' : `, older than its ${cadence(snapshot.cadence_seconds)} cycle`;
  return `newest snapshot ${age}${over}: what it shows may be out of date`;
});

const warn = computed(() => props.snapshot === null || props.snapshot.stale);
</script>

<template>
  <p class="snapshot-age small" :class="{ 'c-amber-strong': warn, muted: !warn }" :role="warn ? 'status' : undefined">
    {{ text }}
  </p>
</template>
