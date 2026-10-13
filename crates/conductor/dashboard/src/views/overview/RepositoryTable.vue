<script setup lang="ts">
import { computed, ref } from 'vue';
import { filtered, type Filter, type RepositoryLine } from '../../api/overview';

const props = defineProps<{ rows: RepositoryLine[]; mine: string[] }>();

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
            <td colspan="7" class="empty">No repositories match this filter.</td>
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
  gap: 12px;
  flex-wrap: wrap;
}
.chips {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.chips button {
  padding: 4px 10px;
  border-radius: 99px;
  border: 1px solid var(--input-border);
  background: transparent;
  color: var(--text);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
.chips button[aria-pressed='true'] {
  background: var(--chip-active);
  border-color: var(--chip-active-border, var(--input-border));
}
.chip {
  display: inline-block;
  font-size: 11px;
  padding: 2px 8px;
  margin: 0 4px 4px 0;
  border-radius: 99px;
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
  padding: 16px 14px;
  color: var(--muted);
}
</style>
