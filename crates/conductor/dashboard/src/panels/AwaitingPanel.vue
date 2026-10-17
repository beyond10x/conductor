<script setup lang="ts">
// What awaits the operator (story:sessions-page-coherent): the Decisions page's open items, each
// with the one thing it asks of him (answer, with its options; or do, with the command) and a
// link to it there. Nothing answered, closed or informational; an empty list says so in one line.
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section class="col" aria-labelledby="awaiting">
    <h2 id="awaiting">
      Awaiting the operator <span class="c-amber-strong">({{ data.awaiting_operator.length }})</span>
    </h2>
    <p v-if="data.awaiting_operator.length === 0" class="card state state-empty">
      Nothing awaits you.
    </p>
    <div v-else class="table-wrap warn">
      <table aria-labelledby="awaiting">
        <thead>
          <tr>
            <th scope="col">item</th>
            <th scope="col">class</th>
            <th scope="col">what to do</th>
            <th scope="col">question</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="item in data.awaiting_operator" :key="item.id">
            <td class="mono small"><a :href="item.link">{{ item.id }}</a></td>
            <td>{{ item.class }}</td>
            <td class="action">{{ item.action }}</td>
            <td class="work small">{{ item.question }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
