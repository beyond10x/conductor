---
title: Read the dashboard
sidebar_position: 4
description: Serve the dashboard on 127.0.0.1, read each of its sections, read the same data as JSON, and rebuild the app.
lede: The dashboard is a read-only app of what every session works on, embedded in the conductor binary; the data it shows is rebuilt from its sources on every request and written to nothing.
source: crates/conductor/src/dashboard.rs, crates/conductor/src/dashboard/; crates/conductor/dashboard/; crates/conductor/tests/dashboard.rs; Taskfile.yml, dashboard, ui, check; conductor dashboard serve --help
---

## 1. Serve it

As a user service, from a checkout of the conductor repository:

```console
task dashboard            # starts the instance's dashboard unless it is active, prints its URL
task dashboard:stop
```

Both tasks act on the active instance only. The dashboard serves on the instance's
`dashboard_port`: 7313 plus the instance's position in the config file (7313 for the first
instance, 7314 for the second), unless the file writes a port. `config validate` refuses two
instances that share a port. Where `systemctl` is on `PATH`, the user service is
`<session_prefix>-conductor-dashboard`, or `conductor-dashboard` for an instance without a prefix.
Elsewhere the dashboard runs in the background, with its pid in `dashboard.pid` and its output in
`dashboard.log` of the instance's `state` directory.

Or in the foreground, on any port (`0` picks a free one):

```console
$ conductor dashboard serve --port 7399
listening on http://127.0.0.1:7399
```

It listens on `127.0.0.1` only and answers `GET` alone. `GET /` is the dashboard app, a small Vue
app built into the binary: its `index.html` loads one script and one stylesheet from `/assets/`,
and the app reads `GET /data.json` when it opens and again every 20 seconds. `/data.json` is all
the data the app shows, as JSON. Any other path answers 404, any other method 405, and a request
naming another host answers 403, so a page served from elsewhere cannot read the data:

```console
$ curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:7399/other
404
$ curl -s -o /dev/null -w "%{http_code}\n" -H "Host: example.org" http://127.0.0.1:7399/
403
```

Every answer carries the policy
`Content-Security-Policy: default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'`:
the app runs only the script this server sends, and requests nothing from anywhere else. It needs
no network beyond `127.0.0.1`.

## 2. What it reads

With a config file, the dashboard reads the instance's `records` (unless `--root` names another
directory), the watch's files under its `state`, and the sessions under its checkouts root. It
names what it read at the foot of the app and in `reading`:

```console
$ curl -s http://127.0.0.1:7399/data.json | jq -c .reading
{"checkouts":"~/work","ci_state":"~/.b10x/conductor/demo/state/watch/ci.state","root":"~/.b10x/conductor/demo/records","sessions":"claude agents --json"}
```

The run on this page used the [getting-started](../getting-started.md) instance, with two lines in
the session's decision log `decisions/2026-10.jsonl` in its records:

```json
{"id":"DEC-20261008-01","class":"C","request":"REQ-alpha-1","options":["A: merge now","B: wait for review"],"choice":"A","reason":"gate green, change inside the approved design"}
{"id":"DEC-20261008-02","class":"O","request":"REQ-beta-1","question":"Split beta into two repositories?","choice":null}
```

## 3. The sections

The app shows these sections, each from its part of `/data.json`, in this order:

| section | shows | from |
|---|---|---|
| summary | when it was read, free disk on `/`, controllers running, build slot | `df`, the session list |
| Sessions under the checkouts root | each session's name, kind, state, age, working directory, and its current dispatch: goal, brief, last event; exited sessions below the live ones | `claude agents --json`, `dispatches/*.jsonl` |
| Usage limit and context | usage limits found now and earlier today, and the sessions the watch reported above `thresholds.context_handover` tokens of context (default 300k), which the page names in each line | the watch's files |
| Red CI on main | each workflow whose newest `main` run failed | the watch's CI state |
| Awaiting the operator | decisions whose newest line is class O or H with no choice yet | `decisions/*.jsonl` |
| Recent decisions | the newest decision lines: id, class, choice, reason | `decisions/*.jsonl` |
| Disk waste | per tracked repository: build cache of idle managed trees, trees eligible for removal, worktree archives and the checkout's `target/`; shared: build targets, toolchains and scratch. Largest first, with each row's owner | `worktree sweep --dry-run`, `worktree gc --dry-run`, a walk of each tree counting allocated bytes, measured in the background |
| Token spend | tokens per repository since 00:00 UTC | the session transcripts, measured in the background |
| Could not read | each source that failed; the other sections still show | |

A controller counts as running when a live session named after its repository runs in that
repository's checkout. The two background measurements show the last complete result with its
age; until the first one finishes the app says so. The header names when the data was read, or
that `/data.json` could not be read, and switches between a dark and a light theme.

The header and the operator's queue as JSON:

```console
$ curl -s http://127.0.0.1:7399/data.json | jq -c '{controllers, build_slot, awaiting_operator, red_ci, errors}'
{"controllers":0,"build_slot":"not recorded","awaiting_operator":[{"class":"O","id":"DEC-20261008-02","question":"Split beta into two repositories?"}],"red_ci":null,"errors":[]}
$ curl -s http://127.0.0.1:7399/data.json | jq -c '.decisions[]'
{"choice":null,"class":"O","id":"DEC-20261008-02","reason":null}
{"choice":"A","class":"C","id":"DEC-20261008-01","reason":"gate green, change inside the approved design"}
```

`red_ci` is `null` because no watch has run yet; the app says "no watch state". The build slot is
not recorded by any command yet.

### Series and KPIs

The charts read `/api/series`, and the KPI panels `/api/kpis`. Both read the daily rollups: each
snapshot that completes adds what it saw (pull requests, runs, releases) and what the dispatch,
decision and guard records and the host ledger hold to one rollup per series, day and group, kept
under the state directory's `series/`, whatever `retention.snapshots` drops.

```console
$ curl -s http://127.0.0.1:7399/api/series | jq -c '.[0]'
$ curl -s 'http://127.0.0.1:7399/api/series?series=prs_merged&by=repository&days=30' | jq -c '.rows[0]'
$ curl -s http://127.0.0.1:7399/api/kpis | jq -c '.[] | {id, value, measured}'
```

`/api/series` without a query lists every series with its unit, the groups it is split `by` and
its source. With `series`, it answers rows of `day` (UTC), `key` (the group, or `total` without
`by`) and `value` for the last `days` (30 by default, at most 365); a day without data is absent.
A series with no source yet answers 404 with the story that adds it, an unknown series or `by`
400 with the allowed values. `/api/kpis` answers each KPI's value over the last 7 days against
its target, or `measured: false` and the story that measures it.

## 4. What it does not do

The dashboard writes one thing: the operator's answers on its Decisions page. There he chooses on
each open escalated request (Yes or No, one option, several options, or Done for a class-H item),
may add his words, and sends them as one batch; each is recorded as his decision, `recorded_by`
`dashboard`, its request moves to Answered, and the watch prints `answered <request-id> <choice>`
so conductor relays it. The page shows each answer recorded or refused. Anyone who can read the
page can send answers as the operator: see
[the Decisions page writes](/docs/security/overview#the-decisions-page-writes) for the boundary
and what it does not stop. Nothing else changes: no button cleans up a tree, and cleanup goes
through each tree's owner. It writes every path under the home directory as `~`.

## 5. Change the app

The app's source is `crates/conductor/dashboard/`: Vue 3, Vite and TypeScript, on the Node
version its `.nvmrc` names, with the packages its `package-lock.json` locks. Its build,
`crates/conductor/dashboard/dist/`, is committed, and the binary embeds that directory when it is
compiled, so building or installing conductor needs no Node. After a change to the app, rebuild
it and commit `dist/` with the change:

```console
task ui        # npm ci, then the type check and the build into crates/conductor/dashboard/dist/
```

`npm test` in `crates/conductor/dashboard` runs its Vitest suite: each section rendered from a
fixture in the shape of `/data.json`, `src/fixtures/data.json`, whose keys a test of the binary
holds to the keys the server sends. `task check` runs `npm ci`, that suite and `task ui`, and fails
when the rebuilt `dist/` differs from the committed one.
