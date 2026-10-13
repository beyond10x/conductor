<script setup lang="ts">
import { cell, percent } from '../format';
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section class="col" aria-labelledby="token-spend">
    <template v-if="data.usage.measured_at === null">
      <h2 id="token-spend">Token spend</h2>
      <p class="card muted">
        {{ data.usage.measuring ? 'measuring, no measurement yet' : 'no measurement' }}
      </p>
    </template>
    <template v-else>
      <h2 id="token-spend">Token spend since {{ cell(data.usage.since) }}</h2>
      <p class="meta small muted">
        measured {{ data.usage.measured_at }} ({{ cell(data.usage.measured_ago) }} ago){{
          data.usage.measuring ? ' · measuring again' : ''
        }}
        · from the session transcripts; per session: <code>conductor resource usage</code>
      </p>
      <div class="table-wrap">
        <table aria-labelledby="token-spend">
          <thead>
            <tr>
              <th scope="col">repository</th>
              <th scope="col">sessions</th>
              <th scope="col">calls</th>
              <th scope="col">input</th>
              <th scope="col">output</th>
              <th scope="col">cache read</th>
              <th scope="col">cache write</th>
              <th scope="col">cache read %</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, at) in data.usage.rows" :key="at">
              <td>{{ row.repository }}</td>
              <td>{{ row.sessions }}</td>
              <td>{{ row.calls }}</td>
              <td>{{ row.input }}</td>
              <td>{{ row.output }}</td>
              <td>{{ row.cache_read }}</td>
              <td>{{ row.cache_write }}</td>
              <td>{{ percent(row.cache_read_percent) }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <ul v-if="data.usage.errors.length > 0" class="lines">
        <li v-for="(error, at) in data.usage.errors" :key="at" class="c-red">{{ error }}</li>
      </ul>
    </template>
  </section>
</template>
