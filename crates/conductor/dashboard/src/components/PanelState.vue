<script setup lang="ts">
// What a panel says instead of its content (story:dashboard-polish): that it is reading, that it
// has nothing, that what it reads could not be read, or that its series has no source yet. Every
// panel says it this way, so a reader and a test find it by the same class.
withDefaults(
  defineProps<{
    state: 'loading' | 'empty' | 'error' | 'no-source';
    /** Drawn as a card of its own, where the panel has no frame around it. */
    card?: boolean;
  }>(),
  { card: false },
);
</script>

<template>
  <p
    class="state"
    :class="[`state-${state}`, { card }]"
    :role="state === 'error' ? 'alert' : state === 'loading' ? 'status' : undefined"
  >
    <slot />
  </p>
</template>
