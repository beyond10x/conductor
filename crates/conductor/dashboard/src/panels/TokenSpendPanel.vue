<script setup lang="ts">
import { cell, percent } from '../format';
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section class="col" aria-labelledby="token-spend">
    <template v-if="data.usage.measured_at === null">
      <h2 id="token-spend">Token spend</h2>
      <p class="card state" :class="data.usage.measuring ? 'state-loading' : 'state-empty'">
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
              <th scope="col" class="num">sessions</th>
              <th scope="col" class="num">calls</th>
              <th scope="col" class="num">input</th>
              <th scope="col" class="num">output</th>
              <th scope="col" class="num">cache read</th>
              <th scope="col" class="num">cache write</th>
              <th scope="col" class="num">cache read %</th>
            </tr>
          </thead>
          <tbody>
          <tr v-if="data.usage.rows.length === 0">
            <td colspan="8" class="empty state state-empty">no spend measured</td>
          </tr>
            <tr v-for="(row, at) in data.usage.rows" :key="at">
              <td>{{ row.repository }}</td>
              <td class="num">{{ row.sessions }}</td>
              <td class="num">{{ row.calls }}</td>
              <td class="num">{{ row.input }}</td>
              <td class="num">{{ row.output }}</td>
              <td class="num">{{ row.cache_read }}</td>
              <td class="num">{{ row.cache_write }}</td>
              <td class="num">{{ percent(row.cache_read_percent) }}</td>
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
