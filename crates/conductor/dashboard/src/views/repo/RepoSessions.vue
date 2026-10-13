<script setup lang="ts">
import { ref } from 'vue';
import { cell } from '../../format';
import { clock, type RepoSession, type SpendToday } from '../../api/repo';

defineProps<{ rows: RepoSession[] }>();

/** The transcript last copied, to say so beside its button. */
const copied = ref<string | null>(null);

async function copy(path: string): Promise<void> {
  try {
    await navigator.clipboard.writeText(path);
    copied.value = path;
  } catch {
    copied.value = null;
  }
}

/** Today's spend as the page writes it: calls, and tokens in and out. */
function spend(today: SpendToday | null): string {
  if (today === null) {
    return 'none';
  }
  const tokens = today.input + today.output + today.cache_read + today.cache_write;
  return `${today.calls} calls · ${tokens.toLocaleString('en')} tokens`;
}
</script>

<template>
  <section class="col" aria-labelledby="sess">
    <h2 id="sess">Sessions</h2>
    <p v-if="rows.length === 0" class="muted">no session works here</p>
    <div v-for="row in rows" :key="row.name ?? row.transcript ?? ''" class="card stack session">
      <div class="row">
        <span class="name">{{ cell(row.name) }}</span>
        <span class="state">{{ cell(row.state) }}</span>
      </div>
      <div class="row s13 dim">
        <span>started</span><span class="started">{{ row.started_at ? clock(row.started_at) : '—' }}</span>
      </div>
      <div class="row s13 dim">
        <span>dispatch</span>
        <span class="dispatch">{{ row.dispatch ? `${row.dispatch.id} · ${row.dispatch.brief}` : '—' }}</span>
      </div>
      <div class="row s13 dim">
        <span>spend today (repository)</span><span class="spend">{{ spend(row.spend_today) }}</span>
      </div>
      <div v-if="row.transcript" class="row s13">
        <code class="transcript">{{ row.transcript }}</code>
        <button type="button" class="copy" @click="copy(row.transcript)">
          {{ copied === row.transcript ? 'copied' : 'copy' }}
        </button>
      </div>
    </div>
  </section>
</template>

<style scoped>
.stack {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.row {
  display: flex;
  justify-content: space-between;
  gap: 12px;
}
.s13 {
  font-size: 13px;
}
.dim {
  color: var(--dim);
}
.transcript {
  overflow-x: auto;
  white-space: nowrap;
}
button.copy {
  padding: 2px 8px;
  border-radius: 6px;
  border: 1px solid var(--input-border);
  background: transparent;
  color: var(--dim);
  font: inherit;
  font-size: 12px;
  cursor: pointer;
}
</style>
