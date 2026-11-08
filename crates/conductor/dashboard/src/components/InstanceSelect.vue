<script setup lang="ts">
// The header's instance selector (story:dashboard-instance-switch, story:instance-api): every
// instance the daemon lists, the one this page shows marked. One page serves every instance;
// choosing another opens it at the same view. With one instance it shows just its name; with none
// read, nothing.
import { computed, onMounted, ref } from 'vue';
import { loadInstances, namedInstance, pageOf, type InstanceRow } from '../api/instances';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const props = withDefaults(
  defineProps<{
    fetcher?: Fetch;
    /** Opens a page; the browser's own navigation unless a test gives another. */
    navigate?: (url: string) => void;
    /** The instance this page shows; the page's `?instance=` unless a test gives another. */
    named?: string | null;
  }>(),
  {
    fetcher: (input: string, init?: RequestInit) => fetch(input, init),
    navigate: (url: string) => window.location.assign(url),
  },
);

const instances = ref<InstanceRow[]>([]);
const current = computed(
  () =>
    instances.value.find((instance) => instance.name === (props.named ?? namedInstance())) ??
    instances.value[0] ??
    null,
);

function choose(event: Event): void {
  const name = (event.target as HTMLSelectElement).value;
  if (current.value === null || name === current.value.name) {
    return;
  }
  props.navigate(pageOf(name, window.location.hash));
}

onMounted(async () => {
  instances.value = (await loadInstances(props.fetcher)).instances;
});
</script>

<template>
  <template v-if="current !== null">
    <span v-if="instances.length === 1" class="instance-name">{{ current.name }}</span>
    <select
      v-else
      class="instance-select"
      aria-label="Instance"
      :value="current.name"
      @change="choose"
    >
      <option
        v-for="instance in instances"
        :key="instance.name"
        :value="instance.name"
        :aria-current="instance.name === current.name ? 'page' : undefined"
      >
        {{ instance.name === current.name ? `${instance.name} (this page)` : instance.name }}
      </option>
    </select>
  </template>
</template>
