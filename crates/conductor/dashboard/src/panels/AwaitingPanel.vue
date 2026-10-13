<script setup lang="ts">
import { cell } from '../format';
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section class="col" aria-labelledby="awaiting">
    <h2 id="awaiting">
      Awaiting the operator <span class="c-amber-strong">({{ data.awaiting_operator.length }})</span>
    </h2>
    <div class="table-wrap" :class="{ warn: data.awaiting_operator.length > 0 }">
      <table aria-labelledby="awaiting">
        <thead>
          <tr>
            <th scope="col">decision</th>
            <th scope="col">class</th>
            <th scope="col">question</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(decision, at) in data.awaiting_operator" :key="at">
            <td class="mono small">{{ cell(decision.id) }}</td>
            <td>{{ cell(decision.class) }}</td>
            <td>{{ cell(decision.question) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
