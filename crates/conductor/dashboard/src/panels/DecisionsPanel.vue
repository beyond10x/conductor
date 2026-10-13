<script setup lang="ts">
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
            <th scope="col">class</th>
            <th scope="col">choice</th>
            <th scope="col">reason</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(decision, at) in data.decisions" :key="at">
            <td class="mono small">{{ cell(decision.id) }}</td>
            <td>{{ cell(decision.class) }}</td>
            <td>{{ cell(decision.choice) }}</td>
            <td class="work">{{ cell(decision.reason) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
