<script setup lang="ts">
// The disk part of the health view: the free space against the instance's floors
// (`thresholds.disk_low`, `thresholds.disk_admit`) and every disk-waste row.
import { computed } from 'vue';
import { cell, gib, size } from '../../format';
import type { DiskHealth } from '../../api/health';

const props = defineProps<{ disk: DiskHealth }>();

const pill = computed(() => {
  switch (props.disk.state) {
    case 'ok':
      return 'pill-green';
    case 'low':
      return 'pill-amber';
    case 'admit-only':
      return 'pill-red';
    default:
      return 'pill-idle';
  }
});
</script>

<template>
  <section class="card part disk" aria-labelledby="health-disk">
    <div class="row">
      <h2 id="health-disk">disk</h2>
      <span class="state pill" :class="pill">{{ disk.state ?? 'not read' }}</span>
    </div>
    <div class="free">{{ gib(disk.free_bytes) }} free on {{ disk.path }}</div>
    <div class="floors small muted">
      low at {{ gib(disk.disk_low_bytes) }} · admit-only at {{ gib(disk.disk_admit_bytes) }}
    </div>
    <p v-if="disk.measured_at === null" class="small muted">
      {{ disk.measuring ? 'disk waste: measuring, no measurement yet' : 'disk waste: no measurement' }}
    </p>
    <details v-else>
      <summary>Disk waste · {{ size(disk.total_bytes) }} in {{ disk.rows.length }} rows</summary>
      <p class="small muted">
        {{ size(disk.reclaimable_bytes) }} reclaimable · measured {{ disk.measured_at }} ({{
          cell(disk.measured_ago)
        }}
        ago){{ disk.measuring ? ' · measuring again' : '' }} · read-only:
        <code>conductor gc</code> removes the reclaimable rows
      </p>
      <div class="table-wrap">
        <table aria-label="Disk waste">
          <thead>
            <tr>
              <th scope="col" class="num">size</th>
              <th scope="col">repository</th>
              <th scope="col">what</th>
              <th scope="col">path</th>
              <th scope="col" class="num">trees or entries</th>
              <th scope="col">state</th>
              <th scope="col">conductor gc</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, at) in disk.rows" :key="at">
              <td class="num mono small">{{ size(row.bytes) }}</td>
              <td>{{ cell(row.repository) }}</td>
              <td>{{ row.what }}</td>
              <td class="mono small">{{ cell(row.path) }}</td>
              <td class="num">{{ cell(row.count) }}</td>
              <td>{{ row.state }}</td>
              <td class="mono small">{{ row.gc }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </details>
  </section>
</template>

<style scoped>
.part {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.row {
  display: flex;
  justify-content: space-between;
  gap: var(--space-3);
  align-items: baseline;
}
.free {
  font-size: var(--text-xl);
  font-weight: 600;
}
.pill {
  font-size: var(--text-sm);
  padding: 2px var(--space-2);
  border-radius: var(--radius-pill);
  border: 1px solid currentColor;
}
.pill-green {
  color: var(--green);
}
.pill-amber {
  color: var(--amber-strong);
}
.pill-red {
  color: var(--red);
}
.pill-idle {
  color: var(--dim);
}
.mono {
  font-family: ui-monospace, monospace;
}
summary {
  cursor: pointer;
  color: var(--dim);
}
</style>
