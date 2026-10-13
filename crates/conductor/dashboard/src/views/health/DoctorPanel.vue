<script setup lang="ts">
// The doctor part of the health view: `conductor doctor`'s rows and verdict, from the server's
// cache, with when they were taken.
import { computed } from 'vue';
import { clock, type DoctorHealth } from '../../api/health';

const props = defineProps<{ doctor: DoctorHealth }>();

const tone = computed(() => (props.doctor.ready === true ? 'c-green' : 'c-red'));
</script>

<template>
  <section class="card part doctor" aria-labelledby="health-doctor">
    <div class="row">
      <h2 id="health-doctor">doctor</h2>
      <span v-if="doctor.taken_at === null" class="muted">taking doctor…</span>
      <span v-else-if="doctor.verdict !== null" class="verdict" :class="tone">
        {{ doctor.verdict }}
      </span>
    </div>
    <p v-if="doctor.taken_at !== null" class="taken small muted">
      taken {{ clock(doctor.taken_at) }}{{ doctor.taking ? ' · taking again' : '' }} ·
      <code>conductor doctor</code> prints the same rows
    </p>
    <p v-if="doctor.error !== null" class="error c-red">{{ doctor.error }}</p>
    <ul v-if="doctor.rows.length > 0" class="lines small mono">
      <li v-for="(row, at) in doctor.rows" :key="at">{{ row }}</li>
    </ul>
  </section>
</template>

<style scoped>
.row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: baseline;
}
.part {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.c-green {
  color: var(--green);
}
.mono {
  font-family: ui-monospace, monospace;
}
</style>
