<script setup lang="ts">
// The decision log's newest decisions (story:sessions-page-coherent): decisions only, newest
// first, each once.
import { cell } from '../format';
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section class="col" aria-labelledby="decisions">
    <h2 id="decisions">Recent decisions</h2>
    <div class="table-wrap">
      <table aria-labelledby="decisions">
        <thead>
          <tr>
            <th scope="col">decision</th>
            <th scope="col">at</th>
            <th scope="col">class</th>
            <th scope="col">choice</th>
            <th scope="col">reason</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="data.decisions.length === 0">
            <td colspan="5" class="empty state state-empty">no decision yet</td>
          </tr>
          <tr v-for="decision in data.decisions" :key="decision.id">
            <td class="mono small">{{ decision.id }}</td>
            <td class="mono small">{{ cell(decision.at) }}</td>
            <td>{{ cell(decision.class) }}</td>
            <td>{{ decision.choice }}</td>
            <td class="work">{{ cell(decision.reason) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
