<script setup lang="ts">
// One open class-O or class-H request (story:decisions-page): its question and what the operator
// chooses from, with the control of its kind: two buttons Yes and No (`yes-no`), a radio list
// (`one-of`) or a checkbox list (`several-of`). The recommendation and the default are marked on
// their options, each option shows its cost. Every control is disabled: answering from the page
// is the next story. A request escalated before its options were recorded shows its own words.
import { computed } from 'vue';
import { kindNote, stamp, type DecisionOption, type OpenDecision } from '../../api/decisions';
import { age } from '../../api/overview';

const props = defineProps<{ item: OpenDecision; now: Date }>();

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

const recommended = (letter: string) => props.item.recommended.includes(letter);
const isDefault = (letter: string) => props.item.default.includes(letter);
const name = computed(() => `choice-${props.item.request_id}`);
const optionId = (option: DecisionOption) => `${name.value}-${option.letter}`;
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

    <fieldset v-if="item.kind === 'yes-no'" class="choices" disabled>
      <legend class="kind small muted">{{ kindNote(item.kind) }}</legend>
      <div class="buttons">
        <button
          v-for="one in answers"
          :key="one.letter"
          type="button"
          class="choice"
          :class="{ recommended: recommended(one.letter) }"
          disabled
        >
          <span class="answer">{{ one.text }}</span>
          <span v-if="one.option !== null" class="label small">{{ one.option.label }}</span>
          <span v-if="one.option !== null" class="cost small muted">{{ one.option.cost }}</span>
          <span v-if="recommended(one.letter)" class="mark mark-recommended">recommended</span>
          <span v-if="isDefault(one.letter)" class="mark mark-default">default</span>
        </button>
      </div>
    </fieldset>

    <fieldset v-else-if="listed" class="choices" disabled>
      <legend class="kind small muted">{{ kindNote(item.kind) }}</legend>
      <label
        v-for="option in item.options"
        :key="option.letter"
        class="option"
        :class="{ recommended: recommended(option.letter) }"
        :for="optionId(option)"
      >
        <input
          :id="optionId(option)"
          :type="item.kind === 'several-of' ? 'checkbox' : 'radio'"
          :name="name"
          :value="option.letter"
          :checked="false"
          disabled
        />
        <span class="letter mono">{{ option.letter }}</span>
        <span class="label">{{ option.label }}</span>
        <span class="cost small muted">{{ option.cost }}</span>
        <span v-if="recommended(option.letter)" class="mark mark-recommended">recommended</span>
        <span v-if="isDefault(option.letter)" class="mark mark-default">default</span>
      </label>
    </fieldset>

    <div v-else class="words">
      <p class="small muted no-options">Escalated before its options were recorded; its own words:</p>
      <p class="options-text">{{ item.options_text }}</p>
      <p class="recommendation-text small">recommended: {{ item.recommendation_text }}</p>
    </div>
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
  cursor: not-allowed;
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
  cursor: not-allowed;
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
</style>
