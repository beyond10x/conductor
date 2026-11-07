<script setup lang="ts">
// The header's instance selector (story:dashboard-instance-switch): every instance of the host
// registry, the one this page shows marked. Each instance keeps its own dashboard; choosing
// another opens that dashboard's page at the same view, so its Decisions answers go to it. With
// one instance it shows just its name; with none read, nothing.
import { computed, onMounted, ref } from 'vue';
import { loadInstances, pageOf, type DashboardInstance } from '../api/instances';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

const props = withDefaults(
  defineProps<{
    fetcher?: Fetch;
    /** Opens a page; the browser's own navigation unless a test gives another. */
    navigate?: (url: string) => void;
  }>(),
  {
    fetcher: (input: string, init?: RequestInit) => fetch(input, init),
    navigate: (url: string) => window.location.assign(url),
  },
);

const instances = ref<DashboardInstance[]>([]);
const current = computed(() => instances.value.find((instance) => instance.current) ?? null);

function choose(event: Event): void {
  const url = (event.target as HTMLSelectElement).value;
  const chosen = instances.value.find((instance) => instance.url === url);
  if (chosen === undefined || chosen.current) {
    return;
  }
  props.navigate(pageOf(chosen, window.location.hash));
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
      :value="current.url"
      @change="choose"
    >
      <option
        v-for="instance in instances"
        :key="instance.url"
        :value="instance.url"
        :aria-current="instance.current ? 'page' : undefined"
      >
        {{ instance.current ? `${instance.name} (this page)` : instance.name }}
      </option>
    </select>
  </template>
</template>
