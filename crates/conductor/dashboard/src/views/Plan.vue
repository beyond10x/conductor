<script setup lang="ts">
// The Plan page, `#/plan` (story:dashboard-plan-board; the look of
// docs/design/dashboard-plan-board-mockup.html): one repository's planning store as a board,
// columns Draft, Proposed, Accepted (next), In progress and Done, the laned ones grouped by the
// wave `aep plan artifact waves` derives. A card opens the story in a drawer: its Why, its
// Acceptance with a tick on each line evidence names, its Scope, its History and a comment box.
// Its buttons are the moves the store's lifecycle allows from where it is, Start for an accepted
// one, and Make it important (P1); each is one write the server records as the operator's
// decision. A refused write shows the store's reason and reads nothing back: the board is as it
// was.
import { computed, onBeforeUnmount, onMounted, ref } from 'vue';
import PanelState from '../components/PanelState.vue';
import {
  COLUMNS,
  dropWrite,
  lanesOf,
  loadBoard,
  loadStory,
  moveLabel,
  sendPlan,
  type Column,
  type PlanBoard,
  type PlanCard,
  type PlanStory,
  type PlanWrite,
} from '../api/plan';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;
const props = defineProps<{ fetcher?: Fetch; token?: string | null }>();

const board = ref<PlanBoard | null>(null);
const failure = ref<string | null>(null);
const repository = ref<string | null>(null);
const query = ref('');
const grouped = ref(true);
/** The "Hide done" toggle (story:board-hide-done): pressed unless this browser released it. */
const HIDE_DONE_KEY = 'conductor.plan.hideDone';
function readHideDone(): boolean {
  try {
    return window.localStorage.getItem(HIDE_DONE_KEY) !== 'false';
  } catch {
    return true;
  }
}
const hideDone = ref(readHideDone());
function toggleHideDone(): void {
  hideDone.value = !hideDone.value;
  try {
    window.localStorage.setItem(HIDE_DONE_KEY, String(hideDone.value));
  } catch {
    // No storage: the choice lasts for this page only.
  }
}
const selected = ref<string | null>(null);
const story = ref<PlanStory | null>(null);
const storyError = ref<string | null>(null);
const refusal = ref<string | null>(null);
const outcome = ref<string | null>(null);
const comment = ref('');
const writing = ref(false);
const dropTarget = ref<Column | null>(null);

const columns = computed(() =>
  COLUMNS.filter((column) => !(hideDone.value && column.column === 'Done')).map((column) => ({
    ...column,
    lanes: board.value === null ? [] : lanesOf(board.value, column.column, query.value, grouped.value),
    count:
      board.value === null
        ? 0
        : lanesOf(board.value, column.column, query.value, false)[0]?.cards.length ?? 0,
  })),
);

async function read(): Promise<void> {
  const loaded = await loadBoard(repository.value, props.fetcher);
  failure.value = loaded.error;
  board.value = loaded.board;
  if (loaded.board !== null && loaded.board.repository !== '') {
    repository.value = loaded.board.repository;
  }
}

async function open(id: string): Promise<void> {
  selected.value = id;
  refusal.value = null;
  storyError.value = null;
  if (repository.value === null) return;
  const loaded = await loadStory(repository.value, id, props.fetcher);
  story.value = loaded.story;
  storyError.value = loaded.error;
}

function close(): void {
  selected.value = null;
  story.value = null;
  refusal.value = null;
}

async function send(write: PlanWrite, done: string): Promise<void> {
  writing.value = true;
  refusal.value = null;
  outcome.value = null;
  const sent = await sendPlan(write, props.fetcher, props.token);
  writing.value = false;
  if (sent.written === null) {
    refusal.value =
      sent.refusal === 'MoveRefused'
        ? `Refused by the store: ${sent.error ?? ''}`
        : `Not written: ${sent.error ?? ''}`;
    return;
  }
  outcome.value = `${write.id}: ${done} · recorded as your decision ${sent.written.decision_id}`;
  await read();
  if (selected.value === write.id) {
    const keep = refusal.value;
    await open(write.id);
    refusal.value = keep;
  }
}

function base(): { repository: string; id: string } | null {
  if (repository.value === null || story.value === null) return null;
  return { repository: repository.value, id: story.value.id };
}

async function move(to: string): Promise<void> {
  const target = base();
  if (target !== null) await send({ ...target, action: 'Move', to }, `moved to ${to}`);
}

async function start(): Promise<void> {
  const target = base();
  if (target !== null) await send({ ...target, action: 'Start' }, 'started');
}

async function important(on: boolean): Promise<void> {
  const target = base();
  if (target !== null) {
    await send(
      { ...target, action: on ? 'Important' : 'Unimportant' },
      on ? 'marked P1: it goes first in the next wave' : 'priority cleared',
    );
  }
}

async function sendComment(): Promise<void> {
  const target = base();
  if (target === null || comment.value.trim() === '') return;
  await send({ ...target, action: 'Comment', text: comment.value }, 'comment recorded');
  if (refusal.value === null) comment.value = '';
}

function cardOf(id: string): PlanCard | undefined {
  return board.value?.cards.find((card) => card.id === id);
}

function onDragStart(event: DragEvent, id: string): void {
  event.dataTransfer?.setData('text/plain', id);
}

async function onDrop(event: DragEvent, column: Column): Promise<void> {
  dropTarget.value = null;
  const id = event.dataTransfer?.getData('text/plain') ?? '';
  const card = cardOf(id);
  if (card === undefined || repository.value === null) return;
  const write = dropWrite(card, column, null, repository.value);
  if (write === null) {
    if (card.column !== column) {
      refusal.value = `Refused: ${id} cannot go from ${card.column} to ${column} by a drop`;
    }
    return;
  }
  await send(write, write.action === 'Start' ? 'started' : `moved to ${write.to ?? ''}`);
}

function onKey(event: KeyboardEvent): void {
  if (event.key === 'Escape' && selected.value !== null) close();
}

async function chooseRepository(event: Event): Promise<void> {
  repository.value = (event.target as HTMLSelectElement).value;
  close();
  await read();
}

onMounted(() => {
  void read();
  window.addEventListener('keydown', onKey);
});
onBeforeUnmount(() => window.removeEventListener('keydown', onKey));
</script>

<template>
  <div class="plan-page">
    <h1 class="page">Plan</h1>
    <PanelState v-if="failure !== null" state="error" card>{{ failure }}</PanelState>
    <PanelState v-else-if="board === null" state="loading" card>reading /api/plan…</PanelState>
    <template v-else>
      <div class="bar">
        <label class="small">
          Repository
          <select :value="board.repository" aria-label="Repository" @change="chooseRepository">
            <option v-for="name in board.repositories" :key="name" :value="name">{{ name }}</option>
          </select>
        </label>
        <input v-model="query" type="search" aria-label="Search title or id" placeholder="Search title or id" />
        <div class="segmented" role="group" aria-label="Lanes">
          <button type="button" class="chip-button" :aria-pressed="grouped" @click="grouped = true">
            Group by wave
          </button>
          <button type="button" class="chip-button" :aria-pressed="!grouped" @click="grouped = false">
            No groups
          </button>
        </div>
        <button type="button" class="chip-button" :aria-pressed="hideDone" @click="toggleHideDone">
          Hide done
        </button>
        <span class="small muted hint"
          >Drag a card to move it; the store checks every move (<code>aep plan artifact move</code>).</span
        >
      </div>
      <PanelState v-for="error in board.errors" :key="error" state="error" card>{{ error }}</PanelState>
      <p v-if="outcome !== null" class="banner ok small" role="status">{{ outcome }}</p>
      <p v-if="refusal !== null && selected === null" class="banner bad small" role="alert">{{ refusal }}</p>

      <div class="board" :style="{ '--board-columns': columns.length }">
        <section
          v-for="column in columns"
          :key="column.column"
          class="col"
          :class="{ drop: dropTarget === column.column }"
          :data-col="column.column"
          :aria-label="column.name"
          @dragover.prevent="dropTarget = column.column"
          @dragleave="dropTarget = null"
          @drop.prevent="onDrop($event, column.column)"
        >
          <div class="ch">
            <h2>{{ column.name }}</h2>
            <span class="n">{{ column.count }}</span>
            <span class="hint small">{{ column.hint }}</span>
          </div>
          <template v-for="lane in column.lanes" :key="lane.lane ?? 'all'">
            <h3 v-if="lane.lane !== null" class="lane">
              {{ lane.lane }} <span>{{ lane.cards.length }}</span>
            </h3>
            <button
              v-for="card in lane.cards"
              :key="card.id"
              type="button"
              class="story-card"
              :class="{ p1: card.important, sel: selected === card.id }"
              draggable="true"
              :data-id="card.id"
              @click="open(card.id)"
              @dragstart="onDragStart($event, card.id)"
            >
              <span class="t">{{ card.title }}</span>
              <span class="id">{{ card.id }}</span>
              <span class="meta">
                <span v-if="card.important" class="pill r">P1</span>
                <span v-if="!grouped && card.lane !== ''" class="pill b">{{ card.lane }}</span>
                <span v-if="card.started" class="pill v">started</span>
              </span>
            </button>
          </template>
          <p v-if="column.count === 0" class="small muted empty">empty</p>
        </section>
      </div>
    </template>

    <aside v-if="selected !== null" class="drawer" role="dialog" aria-modal="false" aria-label="Story">
      <PanelState v-if="storyError !== null" state="error" card>{{ storyError }}</PanelState>
      <PanelState v-else-if="story === null" state="loading" card>reading the story…</PanelState>
      <template v-else>
        <div class="dh">
          <div class="dh-row">
            <span class="pill m">story</span>
            <span class="pill b">{{ story.status }}</span>
            <span v-if="story.important" class="pill r">P1</span>
            <span v-if="story.started" class="pill v">started</span>
            <button type="button" class="ghost close" aria-label="Close" @click="close">✕</button>
          </div>
          <h2>{{ story.title }}</h2>
          <div class="id">{{ story.id }}</div>
        </div>
        <div class="db">
          <p v-if="refusal !== null" class="banner bad small" role="alert">{{ refusal }}</p>
          <h3>Why</h3>
          <p class="prose">{{ story.why || '—' }}</p>
          <h3>Acceptance</h3>
          <ul class="acceptance">
            <li v-for="(line, index) in story.acceptance" :key="index" :class="{ done: line.evidence }">
              <span class="tick" :aria-label="line.evidence ? 'evidence recorded' : 'no evidence yet'">{{
                line.evidence ? '✓' : '○'
              }}</span>
              {{ line.text }}
            </li>
            <li v-if="story.acceptance.length === 0" class="muted">The story writes no Acceptance section.</li>
          </ul>
          <h3>Scope</h3>
          <ul class="scope">
            <li v-for="entry in story.scope" :key="entry"><code>{{ entry }}</code></li>
            <li v-if="story.scope.length === 0" class="muted">No scope recorded.</li>
          </ul>
          <h3>History</h3>
          <div class="tl">
            <div v-for="(entry, index) in story.history" :key="index">
              {{ entry.at.replace('T', ' ').replace('Z', '') }} {{ entry.what }} · {{ entry.actor }}
            </div>
            <div v-if="story.history.length === 0" class="muted">No history.</div>
          </div>
          <template v-if="story.comments !== ''">
            <h3>Comments</h3>
            <p class="prose">{{ story.comments }}</p>
          </template>
          <h3><label for="plan-comment">Comment to conductor-dev</label></h3>
          <textarea
            id="plan-comment"
            v-model="comment"
            placeholder="Change the acceptance, narrow the scope, or say why it matters (recorded on the story as your words)"
          ></textarea>
          <button type="button" :disabled="writing || comment.trim() === ''" @click="sendComment">
            Add the comment
          </button>
        </div>
        <div class="df">
          <button
            v-for="to in story.moves"
            :key="to"
            type="button"
            :class="{ primary: to === 'active' }"
            :disabled="writing"
            @click="move(to)"
          >
            {{ moveLabel(to) }}
          </button>
          <button v-if="story.column === 'Accepted'" type="button" :disabled="writing" @click="start">
            Start
          </button>
          <span class="right">
            <button v-if="!story.important" type="button" :disabled="writing" @click="important(true)">
              Make it important (P1)
            </button>
            <button v-else type="button" class="ghost" :disabled="writing" @click="important(false)">
              Clear P1
            </button>
          </span>
        </div>
      </template>
    </aside>
  </div>
</template>

<style scoped>
.bar {
  display: flex;
  gap: var(--space-3);
  align-items: center;
  flex-wrap: wrap;
  margin-bottom: var(--space-3);
}
.bar .hint {
  margin-left: auto;
}
.banner {
  padding: var(--space-2) var(--space-3);
  border-radius: var(--radius-sm);
  border: 1px solid var(--border);
}
.banner.ok {
  color: var(--green-fg);
  background: var(--green-bg);
}
.banner.bad {
  color: var(--red);
  background: var(--danger-surface);
  border-color: var(--danger-border);
}
.board {
  display: grid;
  grid-template-columns: repeat(var(--board-columns, 5), minmax(220px, 1fr));
  gap: var(--space-3);
  align-items: start;
  overflow-x: auto;
}
.col {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius);
  padding: var(--space-2);
  min-height: 300px;
}
.col.drop {
  outline: 2px dashed var(--blue);
}
.ch {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
  margin: 2px 4px var(--space-2);
}
.ch h2 {
  font-size: var(--text-sm);
  margin: 0;
  text-transform: uppercase;
  letter-spacing: 0.06em;
}
.ch .n {
  color: var(--muted);
  font-size: var(--text-xs);
}
.ch .hint {
  margin-left: auto;
  color: var(--muted);
}
.lane {
  font-size: var(--text-xs);
  font-weight: 500;
  color: var(--muted);
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: var(--space-3) 4px var(--space-1);
}
.story-card {
  display: block;
  width: 100%;
  text-align: left;
  background: var(--bg);
  color: inherit;
  font: inherit;
  border: 1px solid var(--border);
  border-left: 3px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  margin-bottom: var(--space-2);
  cursor: grab;
}
.story-card:hover {
  border-color: var(--chip-active-border);
}
.story-card.sel {
  outline: 2px solid var(--focus);
}
.story-card.p1 {
  border-left-color: var(--red);
}
.story-card .t {
  display: block;
  font-size: var(--text-sm);
  font-weight: 550;
}
.story-card .id,
.dh .id {
  display: block;
  font-family: var(--mono);
  font-size: var(--text-xs);
  color: var(--muted);
  margin-top: 3px;
}
.story-card .meta {
  display: flex;
  gap: 5px;
  flex-wrap: wrap;
  margin-top: 4px;
}
.pill {
  display: inline-block;
  padding: 0 7px;
  border-radius: 99px;
  font-size: var(--text-xs);
  font-weight: 600;
  white-space: nowrap;
}
.pill.r {
  background: var(--danger-surface);
  color: var(--red);
}
.pill.b {
  background: var(--blue-bg);
  color: var(--blue);
}
.pill.v,
.pill.m {
  background: var(--chip-active);
  color: var(--muted);
}
.empty {
  padding: var(--space-2);
}
.drawer {
  position: fixed;
  top: 0;
  right: 0;
  bottom: 0;
  width: min(560px, 96vw);
  background: var(--surface);
  border-left: 1px solid var(--border);
  box-shadow: -10px 0 30px rgba(0, 0, 0, 0.35);
  z-index: 10;
  display: flex;
  flex-direction: column;
}
.dh {
  padding: var(--space-3) var(--space-4);
  border-bottom: 1px solid var(--border);
}
.dh-row {
  display: flex;
  gap: 6px;
  align-items: center;
}
.dh h2 {
  margin: 4px 0 6px;
  font-size: var(--text-lg);
}
.close {
  margin-left: auto;
}
.db {
  padding: var(--space-3) var(--space-4);
  overflow: auto;
  flex: 1;
}
.db h3 {
  font-size: var(--text-xs);
  text-transform: uppercase;
  letter-spacing: 0.06em;
  color: var(--muted);
  margin: var(--space-4) 0 var(--space-1);
}
.prose {
  white-space: pre-wrap;
}
.acceptance,
.scope {
  list-style: none;
  padding: 0;
  margin: 0;
}
.acceptance li {
  margin: 4px 0;
}
.acceptance .tick {
  display: inline-block;
  width: 1.2em;
  color: var(--muted);
}
.acceptance li.done .tick {
  color: var(--green);
}
.tl {
  font-size: var(--text-xs);
  color: var(--muted);
  border-left: 2px solid var(--border);
  padding-left: var(--space-2);
}
textarea {
  width: 100%;
  min-height: 70px;
  margin-bottom: var(--space-2);
  font: inherit;
}
.df {
  padding: var(--space-3) var(--space-4);
  border-top: 1px solid var(--border);
  display: flex;
  gap: var(--space-2);
  align-items: center;
  flex-wrap: wrap;
}
.db button,
.df button {
  padding: var(--space-1) var(--space-3);
  border-radius: var(--radius-sm);
  border: 1px solid var(--input-border);
  background: transparent;
  color: var(--text);
  cursor: pointer;
}
.db button:disabled,
.df button:disabled {
  opacity: 0.5;
  cursor: default;
}
.df button.primary {
  background: var(--blue);
  border-color: var(--blue);
  color: var(--bg);
  font-weight: 600;
}
.dh button.ghost,
.df button.ghost {
  background: transparent;
  border-color: transparent;
  color: var(--dim);
  cursor: pointer;
}
.df .right {
  margin-left: auto;
  display: flex;
  gap: 6px;
}
</style>
