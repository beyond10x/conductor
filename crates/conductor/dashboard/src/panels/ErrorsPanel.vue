<script setup lang="ts">
import type { DashboardData } from '../types';

defineProps<{ data: DashboardData }>();
</script>

<template>
  <section v-if="data.errors.length > 0" class="col" aria-labelledby="errors">
    <h2 id="errors" class="c-red">Could not read</h2>
    <div class="card danger">
      <ul class="lines">
        <li v-for="(error, at) in data.errors" :key="at" class="mono small">{{ error }}</li>
      </ul>
    </div>
  </section>
</template>
