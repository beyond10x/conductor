<script setup lang="ts">
// One open class-O or class-H request (story:decisions-page): its question and what the operator
// chooses from, with the control of its kind: two buttons Yes and No (`yes-no`), a radio list
// (`one-of`) or a checkbox list (`several-of`); a class-H item has one Done control. The
// recommendation and the default are marked on their options, each option shows its cost. The
// operator answers here (story:decisions-batch-answer): the control sets `choice`, the text box
// his words, and the page sends every answer chosen as one batch; `result` is what the server
// did with this one. A class-O request escalated before its options were recorded shows its own
// words and no control: it is answered in conductor's session.
import { computed } from 'vue';
import {
  DONE,
  answerable,
  kindNote,
  stamp,
  type AnswerResult,
  type DecisionOption,
  type OpenDecision,
} from '../../api/decisions';
import { age } from '../../api/overview';

const props = defineProps<{
  item: OpenDecision;
  now: Date;
  choice?: string[];
  words?: string;
  result?: AnswerResult | null;
}>();

const emit = defineEmits<{
  'update:choice': [choice: string[]];
  'update:words': [words: string];
}>();

/** A yes-no request's two answers, with their option where the escalation named one. */
const answers = computed(() =>
  (['Y', 'N'] as const).map((letter) => ({
    letter,
    text: letter === 'Y' ? 'Yes' : 'No',
    option: props.item.options.find((option) => option.letter === letter) ?? null,
  })),
);

/** Whether the escalation named lettered options (a yes-no request shows its buttons anyway). */
const listed = computed(() => props.item.options.length > 0);

const chosen = computed(() => props.choice ?? []);
const isChosen = (letter: string) => chosen.value.includes(letter);
const recommended = (letter: string) => props.item.recommended.includes(letter);
const isDefault = (letter: string) => props.item.default.includes(letter);
const name = computed(() => `choice-${props.item.request_id}`);
const optionId = (option: DecisionOption) => `${name.value}-${option.letter}`;
const wordsId = computed(() => `words-${props.item.request_id}`);

/** What a request escalated before its options were recorded says above its own words. */
const noOptionsNote = computed(() =>
  props.item.class === 'O'
    ? "Escalated before its options were recorded, so there is nothing to click: answer it in conductor's session. Its own words:"
    : 'Escalated before its options were recorded; its own words:',
);

/** Chooses `letter` alone, or chooses nothing when it was the one chosen (a button toggles). */
function toggleOne(letter: string): void {
  emit('update:choice', isChosen(letter) ? [] : [letter]);
}

/** A radio or checkbox changed: one letter for `one-of`, the ticked ones for `several-of`. */
function picked(letter: string, event: Event): void {
  const checked = (event.target as HTMLInputElement).checked;
  if (props.item.kind !== 'several-of') {
    emit('update:choice', checked ? [letter] : []);
    return;
  }
  const others = chosen.value.filter((one) => one !== letter);
  emit('update:choice', checked ? [...others, letter] : others);
}

function typed(event: Event): void {
  emit('update:words', (event.target as HTMLTextAreaElement).value);
}

const resultText = computed(() => {
  const result = props.result;
  if (!result) {
    return '';
  }
  switch (result.outcome) {
    case 'recorded':
      return `recorded ${result.decision_id ?? ''}`.trim();
    case 'refused':
      return `refused: ${result.error ?? ''} ${result.reason ?? ''}`.trim();
    case 'failed':
      return `not recorded: ${result.reason ?? ''}`.trim();
  }
  return '';
});
</script>

<template>
  <article
    class="decision card"
    :class="[`kind-${item.kind}`, item.class === 'O' ? 'class-o' : 'class-h']"
    :data-request="item.request_id"
  >
    <div class="meta small">
      <span class="class-pill" :title="item.class === 'O' ? 'the operator decides' : 'the operator acts'">{{
        item.class
      }}</span>
      <span class="mono request-id">{{ item.request_id }}</span>
      <span class="repository">{{ item.repository }}</span>
      <span class="age muted">raised {{ age(item.raised_at, now.toISOString()) }} ago</span>
      <span class="deadline" :class="{ muted: item.deadline === null }">{{
        item.deadline === null ? 'no deadline' : `deadline ${stamp(item.deadline)}`
      }}</span>
    </div>
    <h3>{{ item.question }}</h3>
    <p v-if="item.reason" class="reason small muted">{{ item.reason }}</p>

    <div v-if="item.class === 'H' ? !listed : !answerable(item)" class="words">
      <p class="small muted no-options">{{ noOptionsNote }}</p>
      <p class="options-text">{{ item.options_text }}</p>
      <p class="recommendation-text small">recommended: {{ item.recommendation_text }}</p>
    </div>

    <fieldset v-if="item.class === 'H'" class="choices">
      <legend class="kind small muted">the operator acts; say when it is done</legend>
      <div class="buttons">
        <button
          type="button"
          class="choice"
          :aria-pressed="isChosen(DONE) ? 'true' : 'false'"
          :class="{ chosen: isChosen(DONE) }"
          @click="toggleOne(DONE)"
        >Done</button>
      </div>
    </fieldset>

    <fieldset v-else-if="item.kind === 'yes-no'" class="choices">
      <legend class="kind small muted">{{ kindNote(item.kind) }}</legend>
      <div class="buttons">
        <button
          v-for="one in answers"
          :key="one.letter"
          type="button"
          class="choice"
          :class="{ recommended: recommended(one.letter), chosen: isChosen(one.letter) }"
          :aria-pressed="isChosen(one.letter) ? 'true' : 'false'"
          @click="toggleOne(one.letter)"
        >
          <span class="answer">{{ one.text }}</span>
          <span v-if="one.option !== null" class="label small">{{ one.option.label }}</span>
          <span v-if="one.option !== null" class="cost small muted">{{ one.option.cost }}</span>
          <span v-if="recommended(one.letter)" class="mark mark-recommended">recommended</span>
          <span v-if="isDefault(one.letter)" class="mark mark-default">default</span>
        </button>
      </div>
    </fieldset>

    <fieldset v-else-if="listed" class="choices">
      <legend class="kind small muted">{{ kindNote(item.kind) }}</legend>
      <label
        v-for="option in item.options"
        :key="option.letter"
        class="option"
        :class="{ recommended: recommended(option.letter), chosen: isChosen(option.letter) }"
        :for="optionId(option)"
      >
        <input
          :id="optionId(option)"
          :type="item.kind === 'several-of' ? 'checkbox' : 'radio'"
          :name="name"
          :value="option.letter"
          :checked="isChosen(option.letter)"
          @change="picked(option.letter, $event)"
        />
        <span class="letter mono">{{ option.letter }}</span>
        <span class="label">{{ option.label }}</span>
        <span class="cost small muted">{{ option.cost }}</span>
        <span v-if="recommended(option.letter)" class="mark mark-recommended">recommended</span>
        <span v-if="isDefault(option.letter)" class="mark mark-default">default</span>
      </label>
    </fieldset>

    <div v-if="answerable(item)" class="your-words">
      <label class="small muted" :for="wordsId">Your words (optional)</label>
      <textarea
        :id="wordsId"
        class="words-input"
        rows="2"
        :value="words ?? ''"
        @input="typed"
      ></textarea>
    </div>

    <p v-if="result" class="result small" :class="result.outcome" role="status">{{ resultText }}</p>
  </article>
</template>

<style scoped>
.decision {
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.decision.class-o {
  border-color: var(--warn-border);
}
.meta {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  align-items: center;
}
.class-pill {
  font-size: var(--text-xs);
  padding: 2px var(--space-2);
  border-radius: var(--radius-pill);
  border: 1px solid var(--border);
}
.class-o .class-pill {
  color: var(--amber);
  border-color: var(--warn-border);
}
h3 {
  margin: 0;
  font-size: var(--text-md);
}
.reason,
.words p {
  margin: 0;
}
.choices {
  border: 0;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.buttons {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
}
.choice {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 2px;
  min-width: 10rem;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface);
  color: var(--text);
  cursor: pointer;
}
.answer {
  font-weight: 600;
}
.option {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-2);
  align-items: baseline;
  padding: var(--space-1) var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  cursor: pointer;
}
.recommended {
  border-color: var(--blue);
}
.letter {
  font-weight: 600;
}
.mark {
  font-size: var(--text-xs);
  padding: 0 var(--space-2);
  border-radius: var(--radius-pill);
  border: 1px solid var(--border);
}
.mark-recommended {
  color: var(--blue);
  border-color: var(--blue);
}
.mark-default {
  color: var(--dim);
}
.chosen {
  border-color: var(--blue);
  box-shadow: inset 0 0 0 1px var(--blue);
}
.your-words {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.words-input {
  font: inherit;
  padding: var(--space-1) var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--surface);
  color: var(--text);
  resize: vertical;
}
.result {
  margin: 0;
}
.result.refused,
.result.failed {
  color: var(--amber);
}
</style>
