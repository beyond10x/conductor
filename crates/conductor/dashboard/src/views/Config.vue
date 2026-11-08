<script setup lang="ts">
// The Configuration page, `#/config` (story:dashboard-config-page; the look of
// docs/design/dashboard-config-mockup.html): the effective config of the instance this
// dashboard runs, every key of `conductor config show`, with its YAML path, its default when it
// differs and when a change takes effect. Edits are staged, checked as `config validate` checks
// them (here, and by the server's check of the whole result), shown as a YAML diff, and written
// only through the review dialog, with the operator's reason, which the server records as one
// class-C decision.
import { computed, onMounted, reactive, ref, watch } from 'vue';
import PanelState from '../components/PanelState.vue';
import {
  CHOICES,
  count,
  diff,
  emptyStaging,
  loadConfig,
  needsRestart,
  problems,
  sendConfig,
  shown,
  stage,
  type ConfigField,
  type ConfigPage,
  type Effect,
  type Problem,
  type RepositoryRow,
} from '../api/config';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;
const props = defineProps<{ fetcher?: Fetch; token?: string | null }>();

const page = ref<ConfigPage | null>(null);
const failure = ref<string | null>(null);
const staging = reactive(emptyStaging());
const serverProblems = ref<Problem[]>([]);
const serverError = ref<string | null>(null);
const query = ref('');
const repoFilter = ref<'all' | 'Active' | 'Inactive' | 'changed'>('all');
const repoQuery = ref('');
const reviewing = ref(false);
const reason = ref('');
const outcome = ref<string | null>(null);
const writing = ref(false);
const added = reactive({ forge: 'github', name: '', reason: '' });

const EFFECT: Record<Effect, { word: string; tone: string; what: string }> = {
  WatchTick: { word: 'tick', tone: 'eff-t', what: 'next watch tick' },
  Cycle: { word: 'cycle', tone: 'eff-c', what: 'next cycle' },
  LoopRestart: { word: 'restart', tone: 'eff-x', what: 'loop restart' },
  Fixed: { word: 'fixed', tone: 'eff-f', what: 'config migrate moves it' },
};

/** The page's sections: an id, a title, what it holds, and its groups of keys. */
interface Section {
  id: string;
  title: string;
  sub: string;
  groups: { name: string | null; fields: ConfigField[] }[];
}

const DISK = ['disk_path', 'disk_low', 'disk_clear', 'disk_admit', 'gate_stop', 'gate_resume', 'watchdog_every'];
const SESSIONS = ['context_handover', 'silent_session', 'stalled_session', 'tree_stale'];

const fields = computed<ConfigField[]>(() => page.value?.fields ?? []);
const byPrefix = (prefix: string): ConfigField[] =>
  fields.value.filter((field) => field.path.startsWith(prefix));

const fixed = computed(() => fields.value.filter((field) => field.takes_effect === 'Fixed'));
const sourceFields = computed(() => byPrefix('sources'));

const sections = computed<Section[]>(() => {
  const thresholds = byPrefix('thresholds.');
  const key = (field: ConfigField): string => field.path.split('.')[1] ?? '';
  const claimed = new Set<string>();
  const take = (list: ConfigField[]): ConfigField[] => {
    list.forEach((field) => claimed.add(field.path));
    return list;
  };
  const list: Section[] = [
    {
      id: 'controllers',
      title: 'Controllers',
      sub: 'How much work runs at once.',
      groups: [{ name: null, fields: take(byPrefix('controllers.')) }],
    },
    {
      id: 'cadence',
      title: 'Cadence',
      sub: 'When the loop looks and acts. Times are UTC.',
      groups: [{ name: null, fields: take(byPrefix('cadence.')) }],
    },
    {
      id: 'thresholds',
      title: 'Thresholds',
      sub: 'Disk, builds and sessions.',
      groups: [
        { name: 'Disk', fields: take(thresholds.filter((field) => DISK.includes(key(field)))) },
        {
          name: 'Builds',
          fields: take(
            thresholds.filter((field) => !DISK.includes(key(field)) && !SESSIONS.includes(key(field))),
          ),
        },
        { name: 'Sessions', fields: take(thresholds.filter((field) => SESSIONS.includes(key(field)))) },
      ],
    },
    {
      id: 'roles',
      title: 'Roles & models',
      sub: 'Which harness and model each role starts on; a running session keeps its model until it restarts.',
      groups: [
        {
          name: null,
          fields: take(byPrefix('roles[').filter((field) => /\.(model|harness)$/.test(field.path))),
        },
      ],
    },
    {
      id: 'authority',
      title: 'Authority',
      sub: 'Who takes each decision class.',
      groups: [{ name: null, fields: take(byPrefix('authority.')) }],
    },
  ];
  const rest = fields.value.filter(
    (field) =>
      !claimed.has(field.path) &&
      field.takes_effect !== 'Fixed' &&
      !field.path.startsWith('sources') &&
      !field.path.startsWith('roles['),
  );
  list.push({
    id: 'other',
    title: 'Other keys',
    sub: 'Every other key config show writes for this instance.',
    groups: [{ name: null, fields: rest }],
  });
  const q = query.value.trim().toLowerCase();
  return list
    .map((section) => ({
      ...section,
      groups: section.groups
        .map((group) => ({
          ...group,
          fields: group.fields.filter((field) => q === '' || field.path.toLowerCase().includes(q)),
        }))
        .filter((group) => group.fields.length > 0),
    }))
    .filter((section) => section.groups.length > 0);
});

/** A role's label: `roles[0].model` as `conductor.model`. */
function label(field: ConfigField): string {
  const role = /^roles\[(\d+)\]\.(.+)$/.exec(field.path);
  if (role !== null) {
    const name = fields.value.find((known) => known.path === `roles[${role[1]}].role`)?.value;
    return `${name ?? `roles[${role[1]}]`}.${role[2]}`;
  }
  const at = field.path.indexOf('.');
  return at < 0 ? field.path : field.path.slice(at + 1);
}

const localProblems = computed(() => problems(staging, fields.value));
/** A server problem's path under the instance: `instances[0].cadence.watch` as `cadence.watch`. */
const relative = (path: string): string => path.replace(/^instances\[\d+\]\./, '');
const allProblems = computed<Problem[]>(() => {
  const seen = new Set(localProblems.value.map((problem) => problem.path));
  return [
    ...localProblems.value,
    ...serverProblems.value.filter((problem) => !seen.has(relative(problem.path))),
  ];
});
function problemFor(path: string): string | null {
  return allProblems.value.find((problem) => relative(problem.path) === path)?.message ?? null;
}

const staged = computed(() => count(staging));
const diffLines = computed(() => diff(staging, fields.value));
const restart = computed(() => needsRestart(staging, fields.value));
const writable = computed(
  () => page.value?.writable === true && staged.value > 0 && allProblems.value.length === 0,
);
const effectNow = computed<Effect>(() => {
  const effects = fields.value
    .filter((field) => staging.values.has(field.path))
    .map((field) => field.takes_effect);
  if (effects.includes('LoopRestart')) return 'LoopRestart';
  if (effects.includes('Cycle') || staging.activity.size > 0) return 'Cycle';
  return 'WatchTick';
});

function changed(field: ConfigField): boolean {
  return staging.values.has(field.path);
}
function onInput(field: ConfigField, event: Event): void {
  stage(staging, field, (event.target as HTMLInputElement | HTMLSelectElement).value);
}
function undo(field: ConfigField): void {
  staging.values.delete(field.path);
}
function reset(field: ConfigField): void {
  stage(staging, field, null);
}
function discard(): void {
  staging.values.clear();
  staging.activity.clear();
}
function choices(field: ConfigField): string[] | undefined {
  if (CHOICES[field.path] !== undefined) return CHOICES[field.path];
  if (/^roles\[\d+\]\.harness$/.test(field.path)) return ['claude', 'codex'];
  return undefined;
}

// Repositories ------------------------------------------------------------------------------

interface RepoView {
  row: RepositoryRow;
  active: boolean;
  changed: boolean;
  added: boolean;
}
const repos = computed<RepoView[]>(() => {
  const rows = [...(page.value?.repositories ?? [])];
  for (const switched of staging.activity.values()) {
    if (switched.added && !rows.some((row) => row.repository === switched.repository)) {
      rows.push({
        repository: switched.repository,
        activity: 'Inactive',
        decided_by: 'Snapshot',
        marked_by: null,
        reason: null,
      });
    }
  }
  return rows.map((row) => {
    const switched = staging.activity.get(row.repository);
    return {
      row,
      active: (switched?.activity ?? row.activity) === 'Active',
      changed: switched !== undefined,
      added: switched?.added ?? false,
    };
  });
});
const shownRepos = computed(() => {
  const q = repoQuery.value.trim().toLowerCase();
  return repos.value.filter(
    (repo) =>
      (repoFilter.value === 'all' ||
        (repoFilter.value === 'changed' ? repo.changed : (repo.active ? 'Active' : 'Inactive') === repoFilter.value)) &&
      (q === '' || repo.row.repository.toLowerCase().includes(q)),
  );
});
const repoCount = computed(() => {
  const active = repos.value.filter((repo) => repo.active).length;
  return `${active} Active · ${repos.value.length - active} Inactive · ${repos.value.length} listed`;
});
function toggle(repo: RepoView): void {
  const next = repo.active ? 'Inactive' : 'Active';
  if (next === repo.row.activity && !repo.added) {
    staging.activity.delete(repo.row.repository);
    return;
  }
  const held = staging.activity.get(repo.row.repository);
  staging.activity.set(repo.row.repository, {
    repository: repo.row.repository,
    activity: next,
    reason: held?.reason ?? '',
    added: repo.added,
  });
}
function setReason(repo: RepoView, event: Event): void {
  const held = staging.activity.get(repo.row.repository);
  if (held !== undefined) {
    held.reason = (event.target as HTMLInputElement).value;
  }
}
function undoRepo(repo: RepoView): void {
  staging.activity.delete(repo.row.repository);
}
/** `owner/name`, `https://github.com/owner/name(.git)` or a GitLab URL as the repository name. */
function repositoryName(text: string): string {
  const trimmed = text.trim().replace(/\.git$/, '').replace(/\/$/, '');
  const url = /^[a-z]+:\/\/[^/]+\/(.+)$/.exec(trimmed);
  const path = url === null ? trimmed : (url[1] ?? '');
  const parts = path.split('/');
  return parts[parts.length - 1] ?? path;
}
const addProblem = computed(() => {
  if (added.name.trim() === '') return null;
  const name = repositoryName(added.name);
  if (!/^[A-Za-z0-9._-]+$/.test(name)) return 'a repository name of letters, digits, ., _ and -';
  if (repos.value.some((repo) => repo.row.repository === name)) return `${name} is listed already`;
  return null;
});
function add(): void {
  const name = repositoryName(added.name);
  if (name === '' || addProblem.value !== null) return;
  staging.activity.set(name, { repository: name, activity: 'Active', reason: added.reason, added: true });
  added.name = '';
  added.reason = '';
}

// Server check, write ----------------------------------------------------------------------

let checkTimer: ReturnType<typeof setTimeout> | undefined;
async function check(): Promise<void> {
  if (staged.value === 0 || localProblems.value.length > 0) {
    serverProblems.value = [];
    serverError.value = null;
    return;
  }
  const sent = await sendConfig(staging, '', true, props.fetcher, props.token);
  serverProblems.value = sent.problems;
  serverError.value = sent.error;
}
watch(
  () => [...staging.values.entries(), ...[...staging.activity.values()].map((s) => [s.repository, s.activity])],
  () => {
    if (checkTimer !== undefined) clearTimeout(checkTimer);
    checkTimer = setTimeout(() => void check(), 300);
  },
  { deep: true },
);

async function write(): Promise<void> {
  writing.value = true;
  const sent = await sendConfig(staging, reason.value, false, props.fetcher, props.token);
  writing.value = false;
  if (sent.written === null) {
    serverProblems.value = sent.problems;
    serverError.value = sent.error;
    return;
  }
  const written = sent.written;
  outcome.value =
    `Written: decision ${written.decision_id ?? ''}` +
    (written.backup === null ? '' : `; the old file is kept as ${written.backup}`) +
    (written.restart ? '. A changed key takes effect at the loop restart: conductor run --uninstall, then --install.' : '.');
  reviewing.value = false;
  reason.value = '';
  discard();
  await read();
}

async function read(): Promise<void> {
  const loaded = await loadConfig(props.fetcher);
  page.value = loaded.page;
  failure.value = loaded.error;
}
onMounted(() => void read());

/** Scrolls to the section `id`; the hash names the view, so no anchor is followed. */
function jump(id: string): void {
  document.getElementById(`s-${id}`)?.scrollIntoView({ block: 'start' });
}

const sourceLine = computed(() =>
  sourceFields.value.map((field) => `${field.path.replace(/^sources/, '')}: ${field.value}`).join(' · '),
);
</script>

<template>
  <div class="config-page">
    <h1 class="page">Configuration</h1>
    <PanelState v-if="failure !== null" state="error" card>{{ failure }}</PanelState>
    <PanelState v-else-if="page === null" state="loading" card>reading /api/config…</PanelState>
    <div v-else class="layout">
      <aside class="side" aria-label="Instances and sections">
        <h2 class="side-h">Instances of the daemon</h2>
        <ul class="insts">
          <li v-for="instance in page.instances" :key="instance.name">
            <a
              :href="`?instance=${encodeURIComponent(instance.name)}#/config`"
              :aria-current="instance.current ? 'page' : undefined"
              class="inst"
            >
              {{ instance.name }}
              <small class="muted"
                >{{ instance.session_prefix ?? 'no prefix' }} ·
                {{ instance.forge || 'forge unknown' }}</small
              >
            </a>
          </li>
          <li v-if="page.instances.length === 0" class="small muted">The config file lists none.</li>
        </ul>
        <h2 class="side-h">Sections</h2>
        <nav class="secnav" aria-label="Sections">
          <button type="button" class="link" @click="jump('instance')">Instance</button>
          <button type="button" class="link" @click="jump('repos')">Repositories</button>
          <button v-for="section in sections" :key="section.id" type="button" class="link" @click="jump(section.id)">
            {{ section.title }}
          </button>
        </nav>
        <h2 class="side-h">Takes effect</h2>
        <ul class="legend-list small">
          <li v-for="(effect, name) in EFFECT" :key="name">
            <span class="eff" :class="effect.tone">{{ effect.word }}</span> {{ effect.what }}
          </li>
        </ul>
      </aside>

      <div class="main-col">
        <p class="banner small" role="status">
          <template v-if="staged > 0"
            ><b>{{ staged }}</b> staged change{{ staged > 1 ? 's' : '' }} on <b>{{ page.instance }}</b>, not
            written yet.</template
          >
          <template v-else
            >Editing <b>{{ page.instance }}</b>. Changes are staged on the right and written only after
            <code>conductor config validate</code> passes.</template
          >
        </p>
        <p v-if="outcome !== null" class="banner ok small" role="status">{{ outcome }}</p>
        <PanelState v-for="error in page.errors" :key="error" state="error" card>{{ error }}</PanelState>
        <label class="find small">
          Find a setting
          <input v-model="query" type="search" placeholder="cadence, disk_low…" />
        </label>

        <section id="s-instance" class="card">
          <div class="card-h">
            <h2>Instance</h2>
            <code class="right small">instances[{{ page.index }}]</code>
          </div>
          <p class="small muted">
            Name, prefix and paths are fixed here; <code>conductor config migrate</code> moves them. File:
            <code>{{ page.file }}</code>
          </p>
          <dl class="kv">
            <template v-for="field in fixed" :key="field.path">
              <dt><code>{{ field.path }}</code></dt>
              <dd>{{ field.value }}</dd>
            </template>
          </dl>
        </section>

        <section id="s-repos" class="card">
          <div class="card-h">
            <h2>Sources &amp; repositories</h2>
            <code class="right small">sources[*], repository marks</code>
          </div>
          <p class="small muted">
            The sources find repositories; each is Active (a controller works it) or Inactive. A switch is
            written as the repository's mark by the operator, with its reason. Sources:
            <code>{{ sourceLine || 'none' }}</code>.
          </p>
          <div class="toolbar">
            <div class="segmented" role="group" aria-label="Show repositories">
              <button
                v-for="filter in ['all', 'Active', 'Inactive', 'changed'] as const"
                :key="filter"
                type="button"
                class="chip-button"
                :aria-pressed="repoFilter === filter"
                @click="repoFilter = filter"
              >
                {{ filter === 'all' ? 'All' : filter === 'changed' ? 'Changed' : filter }}
              </button>
            </div>
            <input v-model="repoQuery" type="search" aria-label="Filter repositories" placeholder="Filter repositories" />
            <span class="right small muted">{{ repoCount }}</span>
          </div>
          <div class="table-wrap">
            <table>
              <thead>
                <tr>
                  <th>repository</th>
                  <th>active</th>
                  <th>releases</th>
                  <th>decided by</th>
                  <th>reason</th>
                  <th><span class="sr-only">undo</span></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="repo in shownRepos" :key="repo.row.repository" :class="{ changed: repo.changed }">
                  <td>
                    {{ repo.row.repository }} <span v-if="repo.added" class="pill">new</span>
                  </td>
                  <td>
                    <button
                      type="button"
                      role="switch"
                      class="switch"
                      :aria-checked="repo.active"
                      :aria-label="`${repo.row.repository} active`"
                      @click="toggle(repo)"
                    ></button>
                  </td>
                  <td class="muted" title="read-only until the release-mode config key exists">—</td>
                  <td>{{ repo.row.decided_by === 'Mark' ? (repo.row.marked_by ?? 'Mark') : 'Snapshot' }}</td>
                  <td>
                    <input
                      v-if="repo.changed"
                      type="text"
                      :aria-label="`${repo.row.repository} reason`"
                      :value="staging.activity.get(repo.row.repository)?.reason ?? ''"
                      :class="{ bad: (staging.activity.get(repo.row.repository)?.reason ?? '').trim() === '' }"
                      placeholder="reason required"
                      @input="setReason(repo, $event)"
                    />
                    <span v-else class="small">{{ repo.row.reason ?? '' }}</span>
                  </td>
                  <td>
                    <button v-if="repo.changed" type="button" class="link" @click="undoRepo(repo)">undo</button>
                  </td>
                </tr>
                <tr v-if="shownRepos.length === 0">
                  <td colspan="6" class="empty">No repository matches.</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p class="small muted">
            Releases: how a repository releases is shown read-only; its config key is not there yet.
          </p>
          <form class="add" @submit.prevent="add">
            <label
              >forge
              <select v-model="added.forge">
                <option value="github">GitHub</option>
                <option value="gitlab">GitLab</option>
              </select>
            </label>
            <label
              >owner/name or URL
              <input v-model="added.name" type="text" placeholder="example-org/new-service" />
            </label>
            <label
              >reason
              <input v-model="added.reason" type="text" placeholder="why it is Active" />
            </label>
            <button type="submit" :disabled="added.name.trim() === '' || addProblem !== null">Add</button>
            <span v-if="addProblem !== null" class="err small">{{ addProblem }}</span>
          </form>
        </section>

        <section v-for="section in sections" :id="`s-${section.id}`" :key="section.id" class="card">
          <div class="card-h">
            <h2>{{ section.title }}</h2>
            <code class="right small">{{ section.id === 'other' ? '…' : section.id === 'roles' ? 'roles[*]' : `${section.id}.*` }}</code>
          </div>
          <p class="small muted">{{ section.sub }}</p>
          <template v-for="group in section.groups" :key="group.name ?? ''">
            <h3 v-if="group.name !== null" class="group">{{ group.name }}</h3>
            <div class="fields">
              <div
                v-for="field in group.fields"
                :key="field.path"
                class="field"
                :class="{ bad: problemFor(field.path) !== null, changed: changed(field) }"
              >
                <label :for="`f-${field.path}`">
                  {{ label(field) }} <span class="eff" :class="EFFECT[field.takes_effect].tone">{{ EFFECT[field.takes_effect].word }}</span>
                </label>
                <div class="row">
                  <select
                    v-if="choices(field) !== undefined"
                    :id="`f-${field.path}`"
                    :value="shown(staging, field)"
                    @change="onInput(field, $event)"
                  >
                    <option v-for="choice in choices(field)" :key="choice">{{ choice }}</option>
                  </select>
                  <input
                    v-else
                    :id="`f-${field.path}`"
                    type="text"
                    :value="shown(staging, field)"
                    @input="onInput(field, $event)"
                  />
                  <template v-if="field.default !== null && shown(staging, field) !== field.default">
                    <span class="small muted">default {{ field.default }}</span>
                    <button type="button" class="link" @click="reset(field)">reset</button>
                  </template>
                  <button v-if="changed(field)" type="button" class="link" @click="undo(field)">undo</button>
                </div>
                <div v-if="problemFor(field.path) !== null" class="err small">
                  {{ field.path }}: {{ problemFor(field.path) }}
                </div>
                <div v-else class="small faint"><code>{{ field.path }}</code></div>
              </div>
            </div>
          </template>
        </section>
        <section class="card">
          <h2>KPI targets</h2>
          <p class="small muted">
            KPI targets are records of the store, not keys of the config file: they are set with
            <code>conductor metric set-metric-target</code> and shown on the <a href="#/kpis">KPIs</a> page.
          </p>
        </section>
      </div>

      <aside class="rail" aria-label="Staged changes">
        <section class="card">
          <div class="card-h">
            <h2>Staged changes</h2>
            <span class="pill">{{ staged }}</span>
            <button type="button" class="link right" :disabled="staged === 0" @click="discard">Discard all</button>
          </div>
          <p class="small muted">The YAML that will be written to <code>{{ page.file }}</code>.</p>
          <pre class="diff" aria-label="Diff"><template v-if="diffLines.length === 0"><span class="c">No changes.</span></template><span
            v-for="(line, at) in diffLines"
            :key="at"
            :class="line.kind"
          >{{ line.text }}
</span></pre>
          <ul v-if="staged > 0" class="checks small">
            <li v-for="problem in allProblems" :key="problem.path" class="bad-t">✗ {{ problem.path }}: {{ problem.message }}</li>
            <li v-if="allProblems.length === 0 && serverError === null" class="ok">✓ conductor config validate: no problem</li>
            <li v-if="serverError !== null" class="bad-t">✗ {{ serverError }}</li>
            <li>
              Takes effect: <span class="eff" :class="EFFECT[effectNow].tone">{{ EFFECT[effectNow].word }}</span>
              {{ EFFECT[effectNow].what }}
            </li>
          </ul>
          <button type="button" class="primary" :disabled="!writable" @click="reviewing = true">Review &amp; write</button>
          <p class="small muted">
            Writes only from 127.0.0.1 with the page's token. Keeps the old file as
            <code>conductor.yaml.&lt;time&gt;.bak</code> and records one decision with the diff.
          </p>
        </section>
      </aside>
    </div>

    <div v-if="reviewing" class="modal-bg">
      <div class="modal card" role="dialog" aria-modal="true" aria-labelledby="config-review">
        <h2 id="config-review">Write the configuration?</h2>
        <pre class="diff"><span v-for="(line, at) in diffLines" :key="at" :class="line.kind">{{ line.text }}
</span></pre>
        <p class="small">
          Recorded as one class-C decision by the operator, recorded by <code>dashboard</code>, with this diff as its
          evidence.
        </p>
        <p v-if="restart" class="small c-amber">
          A staged key takes effect only at the loop restart: after the write, run
          <code>conductor run --uninstall</code>, then <code>conductor run --install</code>.
        </p>
        <label class="small"
          >Why (your words, recorded)
          <input v-model="reason" type="text" placeholder="the reason for this change" />
        </label>
        <p v-if="serverError !== null" class="err small" role="alert">{{ serverError }}</p>
        <div class="actions">
          <button type="button" @click="reviewing = false">Cancel</button>
          <button type="button" class="primary" :disabled="reason.trim() === '' || writing" @click="write">
            Write
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.layout {
  display: grid;
  grid-template-columns: 200px minmax(0, 1fr) 320px;
  gap: var(--space-4);
  align-items: start;
}
.side,
.rail {
  position: sticky;
  top: var(--space-4);
}
.side-h {
  font-size: var(--text-xs);
  text-transform: uppercase;
  color: var(--muted);
  margin: var(--space-3) 0 var(--space-2);
}
.insts,
.legend-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.inst {
  display: block;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-sm);
}
.inst[aria-current='page'] {
  background: var(--chip-active);
}
.inst small {
  display: block;
}
.secnav {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.main-col {
  display: flex;
  flex-direction: column;
  gap: var(--space-4);
  min-width: 0;
}
.card-h {
  display: flex;
  align-items: baseline;
  gap: var(--space-2);
}
.card-h h2 {
  margin: 0;
}
.right {
  margin-left: auto;
}
.banner {
  margin: 0;
  padding: var(--space-2) var(--space-3);
  border: 1px solid var(--warn-border);
  background: var(--warn-surface);
  border-radius: var(--radius-sm);
}
.banner.ok {
  border-color: var(--border);
  background: var(--green-bg);
}
.find input {
  margin-left: var(--space-2);
}
.kv {
  display: grid;
  grid-template-columns: max-content 1fr;
  gap: var(--space-1) var(--space-4);
  margin: 0;
}
.kv dd {
  margin: 0;
  overflow-wrap: anywhere;
}
.toolbar {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  flex-wrap: wrap;
  margin-bottom: var(--space-2);
}
tr.changed td {
  background: var(--amber-bg);
}
.switch {
  width: 34px;
  height: 18px;
  border-radius: var(--radius-pill);
  border: 1px solid var(--input-border);
  background: var(--surface);
  position: relative;
  cursor: pointer;
}
.switch::after {
  content: '';
  position: absolute;
  top: 2px;
  left: 2px;
  width: 12px;
  height: 12px;
  border-radius: 50%;
  background: var(--muted);
}
.switch[aria-checked='true'] {
  background: var(--green-bg);
  border-color: var(--green);
}
.switch[aria-checked='true']::after {
  left: 18px;
  background: var(--green);
}
.add {
  display: flex;
  gap: var(--space-2);
  align-items: end;
  flex-wrap: wrap;
  margin-top: var(--space-2);
}
.add label,
.modal label {
  display: flex;
  flex-direction: column;
  gap: var(--space-1);
}
.group {
  font-size: var(--text-sm);
  color: var(--muted);
  margin: var(--space-3) 0 var(--space-1);
}
.fields {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--space-3);
}
.field {
  padding: var(--space-2);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
}
.field.changed {
  border-color: var(--amber);
}
.field.bad,
input.bad {
  border-color: var(--red);
}
.field label {
  display: block;
  font-weight: 600;
  margin-bottom: var(--space-1);
}
.row {
  display: flex;
  gap: var(--space-2);
  align-items: center;
  flex-wrap: wrap;
}
.row input {
  width: 9em;
}
.err,
.bad-t {
  color: var(--red);
}
.ok {
  color: var(--green);
}
.eff {
  font-size: var(--text-xs);
  padding: 0 var(--space-1);
  border-radius: var(--radius-pill);
  border: 1px solid var(--border);
}
.eff-t {
  color: var(--green);
}
.eff-c {
  color: var(--blue);
}
.eff-x {
  color: var(--amber);
}
.eff-f {
  color: var(--muted);
}
.pill {
  font-size: var(--text-xs);
  padding: 0 var(--space-2);
  border-radius: var(--radius-pill);
  background: var(--amber-bg);
  color: var(--amber);
}
.diff {
  font-family: var(--mono);
  font-size: var(--text-xs);
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-2);
  white-space: pre-wrap;
  max-height: 360px;
  overflow: auto;
}
.diff .removed {
  color: var(--red);
}
.diff .added {
  color: var(--green);
}
.diff .section,
.diff .c {
  color: var(--muted);
}
.checks {
  list-style: none;
  padding: 0;
}
button.link {
  background: none;
  border: none;
  color: var(--link);
  cursor: pointer;
  padding: 0;
}
button.primary {
  background: var(--blue-bg);
  border: 1px solid var(--blue);
  color: var(--text);
  border-radius: var(--radius-sm);
  padding: var(--space-1) var(--space-3);
}
button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.modal-bg {
  position: fixed;
  inset: 0;
  background: rgb(0 0 0 / 50%);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 20;
}
.modal {
  width: min(560px, 92vw);
  display: flex;
  flex-direction: column;
  gap: var(--space-2);
}
.actions {
  display: flex;
  gap: var(--space-2);
  justify-content: flex-end;
}
@media (max-width: 1100px) {
  .layout {
    grid-template-columns: minmax(0, 1fr);
  }
  .side,
  .rail {
    position: static;
  }
}
</style>
