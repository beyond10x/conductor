<script setup lang="ts">
import { cell, size } from '../format';
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section class="col" aria-labelledby="disk-waste">
    <h2 id="disk-waste">Disk waste</h2>
    <p
      v-if="data.disk.measured_at === null"
      class="card state"
      :class="data.disk.measuring ? 'state-loading' : 'state-empty'"
    >
      {{ data.disk.measuring ? 'measuring, no measurement yet' : 'no measurement' }}
    </p>
    <template v-else>
      <p class="meta small muted">
        total {{ size(data.disk.total_bytes) }} over {{ data.disk.rows.length }} rows,
        {{ size(data.disk.reclaimable_bytes) }} reclaimable · measured
        {{ data.disk.measured_at }} ({{ cell(data.disk.measured_ago) }} ago){{
          data.disk.measuring ? ' · measuring again' : ''
        }}
        · read-only: <code>conductor gc</code> removes the reclaimable rows
      </p>
      <div class="table-wrap">
        <table aria-labelledby="disk-waste">
          <thead>
            <tr>
              <th scope="col" class="num">size</th>
              <th scope="col">repository</th>
              <th scope="col">what</th>
              <th scope="col">path</th>
              <th scope="col" class="num">trees or entries</th>
              <th scope="col">owner</th>
              <th scope="col">age</th>
              <th scope="col">state</th>
              <th scope="col">conductor gc</th>
            </tr>
          </thead>
          <tbody>
          <tr v-if="data.disk.rows.length === 0">
            <td colspan="9" class="empty state state-empty">no waste found</td>
          </tr>
            <tr v-for="(row, at) in data.disk.rows" :key="at">
              <td class="num mono small">{{ size(row.bytes) }}</td>
              <td>{{ cell(row.repository) }}</td>
              <td>{{ row.what }}</td>
              <td class="mono small">{{ cell(row.path) }}</td>
              <td class="num">{{ cell(row.count) }}</td>
              <td>{{ cell(row.owner) }}</td>
              <td>{{ cell(row.age) }}</td>
              <td>{{ row.state }}</td>
              <td class="mono small">{{ row.gc }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </template>
  </section>
</template>
