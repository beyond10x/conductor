<script setup lang="ts">
// The instance's sessions (story:sessions-page-coherent): its conductor, its maintainer and each
// controller its records hold, never a session that only works in the same directory. A
// controller shows its dispatch, matched through its repository: the open one, else its last one,
// marked closed.
import { computed } from 'vue';
import { cell } from '../format';
import type { DashboardData, Session } from '../types';

const props = defineProps<{ data: DashboardData }>();

const running = computed(() => props.data.sessions.filter((session) => !session.exited).length);
const stopped = computed(() => props.data.sessions.length - running.value);

/** The status and the state when they differ, `status (state)`, else the one there is. */
function state(session: Session): string {
  const { status, state } = session;
  if (status !== null && state !== null && status !== state) {
    return `${status} (${state})`;
  }
  return cell(status ?? state);
}

/** The role, with the controller's repository: `controller · alpha`. */
function role(session: Session): string {
  return session.repository === null ? session.role : `${session.role} · ${session.repository}`;
}

/** The dispatch id, `(closed)` after one that is no longer open; `none` for a controller without. */
function dispatchId(session: Session): string {
  const dispatch = session.dispatch;
  if (dispatch === null) {
    return session.role === 'controller' ? 'none' : '—';
  }
  return dispatch.open ? dispatch.id : `${dispatch.id} (closed)`;
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
      Sessions of this instance ({{ running }} running, {{ stopped }} not running)
    </h2>
    <div class="table-wrap">
      <table aria-labelledby="sessions">
        <thead>
          <tr>
            <th scope="col">name</th>
            <th scope="col">role</th>
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
            <td colspan="9" class="empty state state-empty">no session of this instance</td>
          </tr>
          <tr v-for="(session, at) in data.sessions" :key="at" :class="{ faint: session.exited }">
            <td>{{ cell(session.name) }}</td>
            <td>{{ role(session) }}</td>
            <td>{{ state(session) }}</td>
            <td>{{ session.started_ago === null ? '—' : `${session.started_ago} ago` }}</td>
            <td class="mono small">{{ cell(session.cwd) }}</td>
            <td class="mono small">{{ dispatchId(session) }}</td>
            <td>{{ cell(session.dispatch?.goal ?? null) }}</td>
            <td class="work">{{ cell(session.dispatch?.brief ?? null) }}</td>
            <td>{{ lastEvent(session) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </section>
</template>
