<script setup lang="ts">
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section class="col" aria-labelledby="red-ci">
    <h2 id="red-ci">Red CI on main</h2>
    <p v-if="data.red_ci === null" class="card muted">no watch state</p>
    <p v-else-if="data.red_ci.length === 0" class="card muted">none</p>
    <div v-else class="table-wrap danger">
      <table aria-labelledby="red-ci">
        <thead>
          <tr>
            <th scope="col">repository</th>
            <th scope="col">workflow</th>
            <th scope="col">run</th>
            <th scope="col">conclusion</th>
            <th scope="col">time</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(run, at) in data.red_ci" :key="at">
            <td>{{ run.repo }}</td>
            <td>{{ run.workflow }}</td>
            <td class="mono small">{{ run.run_id }}</td>
            <td class="c-red">{{ run.conclusion }}</td>
            <td class="mono small">{{ run.time }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
