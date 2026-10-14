<script setup lang="ts">
// The KPI page, `#/kpis` (story:dashboard-kpi-panels): every conductor KPI, K1–K12 of
// docs/design/self-improvement.md § 2 and the per-task ones, a panel each.
import KpiBoard from './kpi/KpiBoard.vue';
</script>

<template>
  <div class="kpis">
    <h1 class="page">KPIs</h1>
    <p class="small muted">
      Each value is over its last days, against its target; the line is its daily value over 30 days,
      the dashed line its target.
    </p>
    <KpiBoard />
  </div>
</template>

<style scoped>
.kpis {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
}
</style>
