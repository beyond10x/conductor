<script setup lang="ts">
// The overview's KPI strip (story:dashboard-kpi-panels): every KPI, compact, at the top.
import KpiBoard from '../kpi/KpiBoard.vue';
</script>

<template>
  <!-- A strip above the panels, not a panel: story:review-limit keeps "Waiting for review" the
       overview's first section. -->
  <div class="col kpi-strip" role="region" aria-labelledby="kpi-strip">
    <div class="row">
      <h2 id="kpi-strip">KPIs</h2>
      <a href="#/kpis" class="small">every KPI in full</a>
    </div>
    <KpiBoard compact />
  </div>
</template>

<style scoped>
.kpi-strip {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.row {
  display: flex;
  flex-wrap: wrap;
  align-items: baseline;
  gap: var(--space-3);
}
</style>
