<script setup lang="ts">
// The Decisions page, `#/decisions` (story:decisions-page): every open class-O and class-H request
// with what the operator chooses from, oldest first, read-only for now, and the decision history
// from the records' decision log below it, newest first. It reads `/api/decisions` every 20 s,
// sending the last `ETag`, so an unchanged answer costs the server nothing.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  loadDecisions,
  newestFirst,
  oldestFirst,
  stamp,
  type DecisionsRead,
} from '../api/decisions';
import PanelState from '../components/PanelState.vue';
import DecisionItem from './decisions/DecisionItem.vue';

/** Milliseconds between two reads, as the rest of the app reloads. */
const RELOAD_MS = 20_000;

const read = ref<DecisionsRead | null>(null);
const now = ref(new Date());
let timer: ReturnType<typeof setInterval> | undefined;

async function load(): Promise<void> {
  read.value = await loadDecisions(undefined, read.value);
  now.value = new Date();
}

const open = computed(() => oldestFirst(read.value?.answer?.open ?? []));
const history = computed(() => newestFirst(read.value?.answer?.history ?? []));
const errors = computed(() => read.value?.answer?.errors ?? []);

onMounted(() => {
  void load();
  timer = setInterval(() => void load(), RELOAD_MS);
});

onBeforeUnmount(() => {
  if (timer !== undefined) {
    clearInterval(timer);
  }
});
</script>

<template>
  <div class="decisions">
    <h1 class="page">Decisions</h1>
    <p class="read-only-note small muted">
      Each open class-O and class-H request, with its options, their cost, the recommendation and
      the default. The controls are read-only: Answering from this page comes next; until then
      answer in conductor's session.
    </p>
    <PanelState v-if="read === null" state="loading" card>reading /api/decisions…</PanelState>
    <PanelState v-else-if="read.error !== null" state="error" card>{{ read.error }}</PanelState>
    <template v-else>
      <PanelState v-if="errors.length > 0" state="error" card>{{ errors.join('; ') }}</PanelState>
      <section aria-labelledby="open-decisions" class="col">
        <h2 id="open-decisions">Open · {{ open.length }}</h2>
        <PanelState v-if="open.length === 0" state="empty">nothing waits for the operator</PanelState>
        <DecisionItem v-for="item in open" :key="item.request_id" :item="item" :now="now" />
      </section>
      <section aria-labelledby="decision-history" class="col">
        <h2 id="decision-history">History</h2>
        <PanelState v-if="history.length === 0" state="empty">no decision recorded yet</PanelState>
        <ol v-else class="history card">
          <li v-for="(row, at) in history" :key="at" :data-id="row.id ?? ''">
            <span class="when mono small muted">{{ row.at === null ? '—' : stamp(row.at) }}</span>
            <span class="event">{{ row.event ?? 'line' }}</span>
            <span v-if="row.class !== null" class="class">{{ row.class }}</span>
            <span class="id mono small">{{ row.id }}</span>
            <span v-if="row.repo !== null" class="repo small muted">{{ row.repo }}</span>
            <span v-if="row.question !== null" class="question">{{ row.question }}</span>
            <span v-if="row.choice !== null" class="choice-made">chose {{ row.choice }}</span>
            <span v-if="row.decided_by !== null" class="decided-by small muted">{{ row.decided_by }}</span>
            <span v-if="row.reason !== null" class="why small muted">{{ row.reason }}</span>
          </li>
        </ol>
      </section>
    </template>
  </div>
</template>

<style scoped>
.decisions,
.col {
  display: flex;
  flex-direction: column;
  gap: var(--space-3);
}
.read-only-note {
  margin: 0;
}
.history {
  list-style: none;
  margin: 0;
  padding: var(--space-2) var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.history li {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  align-items: baseline;
  border-bottom: 1px solid var(--border);
  padding-bottom: var(--space-2);
}
.history li:last-child {
  border-bottom: 0;
  padding-bottom: 0;
}
.event {
  font-weight: 600;
}
.question {
  overflow-wrap: anywhere;
}
</style>
