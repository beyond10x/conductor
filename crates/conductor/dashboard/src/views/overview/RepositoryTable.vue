<script setup lang="ts">
import { computed, ref } from 'vue';
import { filtered, type Filter, type RepositoryLine } from '../../api/overview';

const props = withDefaults(
  defineProps<{ rows: RepositoryLine[]; mine: string[]; error?: string | null }>(),
  { error: null },
);

const filter = ref<Filter>('all');

const filters = computed<[Filter, string][]>(() => [
  ['all', `All ${props.rows.length}`],
  ['mine', 'Mine'],
  ['red', 'Red CI'],
]);

const shown = computed(() => filtered(props.rows, filter.value, props.mine));

/** The page of a repository, as the repository view routes it. */
function page(name: string): string {
  return `#/repo/${encodeURIComponent(name)}`;
}
</script>

<template>
  <section aria-labelledby="repos" class="col">
    <div class="filter-head">
      <h2 id="repos">Repositories</h2>
      <div role="group" aria-label="Filter" class="chips">
        <button
          v-for="[name, label] in filters"
          :key="name"
          type="button"
          :aria-pressed="filter === name ? 'true' : 'false'"
          @click="filter = name"
        >
          {{ label }}
        </button>
      </div>
    </div>
    <div class="table-wrap">
      <table aria-labelledby="repos">
        <thead>
          <tr>
            <th scope="col">repository</th>
            <th scope="col">state</th>
            <th scope="col">controller</th>
            <th scope="col">PRs / MRs</th>
            <th scope="col">main</th>
            <th scope="col">release</th>
            <th scope="col">last report</th>
          </tr>
        </thead>
        <tbody aria-live="polite">
          <tr v-if="shown.length === 0">
            <td v-if="error !== null" colspan="7" class="empty state state-error" role="alert">{{ error }}</td>
            <td v-else colspan="7" class="empty state state-empty">No repositories match this filter.</td>
          </tr>
          <tr v-for="row in shown" :key="row.name">
            <td class="name">
              <a :href="page(row.name)">{{ row.name }}</a>
            </td>
            <td class="chips-cell">
              <span
                v-for="chip in row.chips"
                :key="chip"
                class="chip"
                :class="{ red: chip === 'red main', ok: chip === 'ok' }"
                >{{ chip }}</span
              >
            </td>
            <td class="controller small">{{ row.controller }}</td>
            <td class="prs mono small">{{ row.prs }}</td>
            <td class="main small" :class="{ 'c-red': row.main === 'red' }">{{ row.main }}</td>
            <td class="release mono small">{{ row.release }}</td>
            <td class="last small muted">{{ row.last }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>

<style scoped>
.filter-head {
  display: flex;
  align-items: center;
  gap: var(--space-3);
  flex-wrap: wrap;
}
.chips {
  display: flex;
  gap: var(--space-2);
  flex-wrap: wrap;
}
.chips button {
  padding: var(--space-1) var(--space-3);
  border-radius: var(--radius-pill);
  border: 1px solid var(--input-border);
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: var(--text-sm);
  cursor: pointer;
}
.chips button[aria-pressed='true'] {
  background: var(--chip-active);
  border-color: var(--chip-active-border, var(--input-border));
}
.chip {
  display: inline-block;
  font-size: var(--text-xs);
  padding: 2px var(--space-2);
  margin: 0 var(--space-1) var(--space-1) 0;
  border-radius: var(--radius-pill);
  border: 1px solid var(--warn-border);
  color: var(--amber);
}
.chip.red {
  border-color: var(--danger-border);
  color: var(--red);
}
.chip.ok {
  border-color: var(--border);
  color: var(--green);
}
.empty {
  padding: var(--space-4) var(--space-3);
  color: var(--muted);
}
</style>
