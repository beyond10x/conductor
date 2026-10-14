<script setup lang="ts">
import { computed } from 'vue';
import { cell } from '../format';
import type { DashboardData, Session } from '../types';

const props = defineProps<{ data: DashboardData }>();

const exited = computed(() => props.data.sessions.filter((session) => session.exited).length);
const live = computed(() => props.data.sessions.length - exited.value);

/** The status and the state when they differ, `status (state)`, else the one there is. */
function state(session: Session): string {
  const { status, state } = session;
  if (status !== null && state !== null && status !== state) {
    return `${status} (${state})`;
  }
  return cell(status ?? state);
}

function lastEvent(session: Session): string {
  const dispatch = session.dispatch;
  if (dispatch === null || dispatch.event === null) {
    return '—';
  }
  return `${dispatch.event}, ${cell(dispatch.event_ago)} ago`;
}
</script>

<template>
  <section class="col" aria-labelledby="sessions">
    <h2 id="sessions">
      Sessions under {{ data.reading.checkouts }} ({{ live }} live, {{ exited }} exited)
    </h2>
    <div class="table-wrap">
      <table aria-labelledby="sessions">
        <thead>
          <tr>
            <th scope="col">name</th>
            <th scope="col">kind</th>
            <th scope="col">state</th>
            <th scope="col">started</th>
            <th scope="col">cwd</th>
            <th scope="col">dispatch</th>
            <th scope="col">goal</th>
            <th scope="col">brief</th>
            <th scope="col">last event</th>
          </tr>
        </thead>
        <tbody>
          <tr v-if="data.sessions.length === 0">
            <td colspan="9" class="empty state state-empty">no session</td>
          </tr>
          <tr v-for="(session, at) in data.sessions" :key="at" :class="{ faint: session.exited }">
            <td>{{ cell(session.name) }}</td>
            <td>{{ cell(session.kind) }}</td>
            <td>{{ state(session) }}</td>
            <td>{{ cell(session.started_ago) }} ago</td>
            <td class="mono small">{{ cell(session.cwd) }}</td>
            <td class="mono small">{{ cell(session.dispatch?.id ?? null) }}</td>
            <td>{{ cell(session.dispatch?.goal ?? null) }}</td>
            <td class="work">{{ cell(session.dispatch?.brief ?? null) }}</td>
            <td>{{ lastEvent(session) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
