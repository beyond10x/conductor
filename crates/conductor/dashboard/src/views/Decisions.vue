<script setup lang="ts">
// The Decisions page, `#/decisions` (story:decisions-page): every open class-O and class-H request
// with what the operator chooses from, oldest first, and the decision history from the records'
// decision log below it, newest first. It reads `/api/decisions` every 20 s, sending the last
// `ETag`, so an unchanged answer costs the server nothing.
//
// The operator answers here (story:decisions-batch-answer): he chooses on each item he answers,
// may add his words, and sends every chosen answer as one batch, `POST /api/answers` with the
// page's token. The server answers row by row; each row's result shows on its item and in the
// list of what was sent, a recorded row's choice is cleared, a refused one's kept, and the page
// reads `/api/decisions` again at once.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import {
  answerable,
  inOptionOrder,
  loadDecisions,
  newestFirst,
  oldestFirst,
  sendAnswers,
  stamp,
  type AnswerResult,
  type AnswerRow,
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

/** What the operator chose on each item, and his words, by request id. */
const choices = ref<Record<string, string[]>>({});
const words = ref<Record<string, string>>({});
/** What the server did with each row of the last batch sent, in the batch's order. */
const results = ref<AnswerResult[]>([]);
const sendError = ref<string | null>(null);
const sending = ref(false);

/** The answers chosen on the items still open, in the page's order. */
const batch = computed<AnswerRow[]>(() =>
  open.value
    .filter(answerable)
    .map((item) => {
      const row: AnswerRow = {
        request_id: item.request_id,
        choice: inOptionOrder(item, choices.value[item.request_id] ?? []),
      };
      const said = (words.value[item.request_id] ?? '').trim();
      if (said !== '') {
        row.words = said;
      }
      return row;
    })
    .filter((row) => row.choice.length > 0),
);

const sendLabel = computed(() => {
  const count = batch.value.length;
  return count === 0 ? 'Send answers' : `Send ${count} answer${count === 1 ? '' : 's'}`;
});

function resultOf(id: string): AnswerResult | null {
  return results.value.find((result) => result.request_id === id) ?? null;
}

function resultText(result: AnswerResult): string {
  switch (result.outcome) {
    case 'recorded':
      return `${result.request_id}: recorded ${result.decision_id ?? ''}`.trim();
    case 'refused':
      return `${result.request_id}: refused: ${result.error ?? ''} ${result.reason ?? ''}`.trim();
    case 'failed':
      return `${result.request_id}: not recorded: ${result.reason ?? ''}`.trim();
  }
  return result.request_id;
}

async function send(): Promise<void> {
  const rows = batch.value;
  if (rows.length === 0 || sending.value) {
    return;
  }
  sending.value = true;
  sendError.value = null;
  const sent = await sendAnswers(rows);
  sending.value = false;
  if (sent.results === null) {
    sendError.value = sent.error;
    return;
  }
  results.value = sent.results;
  for (const result of sent.results) {
    if (result.outcome === 'recorded') {
      delete choices.value[result.request_id];
      delete words.value[result.request_id];
    }
  }
  await load();
}

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
    <p class="answer-note small muted">
      Each open class-O and class-H request, with its options, their cost, the recommendation and
      the default. Choose on each one you answer, add your words if you like, and send them
      together: each answer is recorded as your decision, and conductor relays it.
    </p>
    <PanelState v-if="read === null" state="loading" card>reading /api/decisions…</PanelState>
    <PanelState v-else-if="read.error !== null" state="error" card>{{ read.error }}</PanelState>
    <template v-else>
      <PanelState v-if="errors.length > 0" state="error" card>{{ errors.join('; ') }}</PanelState>
      <section aria-labelledby="open-decisions" class="col">
        <h2 id="open-decisions">Open · {{ open.length }}</h2>
        <PanelState v-if="open.length === 0" state="empty">nothing waits for the operator</PanelState>
        <DecisionItem
          v-for="item in open"
          :key="item.request_id"
          v-model:choice="choices[item.request_id]"
          v-model:words="words[item.request_id]"
          :item="item"
          :now="now"
          :result="resultOf(item.request_id)"
        />
        <div v-if="open.length > 0" class="send-row">
          <button
            type="button"
            class="send"
            :disabled="batch.length === 0 || sending"
            @click="send"
          >{{ sendLabel }}</button>
          <span v-if="sending" class="small muted">sending…</span>
        </div>
        <PanelState v-if="sendError !== null" state="error" class="send-error">{{ sendError }}</PanelState>
        <ul v-if="results.length > 0" class="results card small" aria-label="What was sent">
          <li
            v-for="result in results"
            :key="result.request_id"
            :data-request="result.request_id"
            :class="result.outcome"
          >{{ resultText(result) }}</li>
        </ul>
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
.answer-note {
  margin: 0;
}
.send-row {
  display: flex;
  gap: var(--space-2);
  align-items: center;
}
.send {
  padding: var(--space-2) var(--space-4);
  border: 1px solid var(--blue);
  border-radius: var(--radius-sm);
  background: var(--blue);
  color: #ffffff;
  font-weight: 600;
  cursor: pointer;
}
.send:disabled {
  background: var(--surface);
  color: var(--muted);
  border-color: var(--border);
  cursor: not-allowed;
}
.results {
  list-style: none;
  margin: 0;
  padding: var(--space-2) var(--space-3);
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.results .refused,
.results .failed {
  color: var(--amber);
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
