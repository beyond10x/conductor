---
title: The dashboard and the TUI
sidebar_position: 5
description: Open the dashboard conductord serves, pick an instance with ?instance=, read its views, read the same data from the /v1 routes, know which writes need which grant, and use conductor tui in a terminal.
lede: Conductord serves one dashboard for every instance of its config file, on 127.0.0.1 at daemon.port, and proxies each instance's API under /v1/instances/<name>/. The terminal UI reads the same API.
source: crates/conductor/src/daemon/mod.rs, daemon/api.rs, daemon/federate.rs; crates/conductor/src/dashboard.rs, dashboard/series.rs; crates/conductor/src/instance.rs; crates/conductor/src/tui/; crates/conductor/dashboard/src/App.vue, src/api/route.ts; Taskfile.yml, dashboard; conductor daemon serve --help, conductor tui --help
---

## 1. Open it

The dashboard is served by conductord, the daemon, from the app built into the binary. No
per-instance dashboard process runs. Start the daemon as a user unit, or in the foreground:

```console
$ conductor daemon serve --install
$ conductor daemon serve
conductord: config ~/.b10x/conductor/conductor.yaml, serving the dashboard and /v1/ on http://127.0.0.1:7300/
```

The daemon reads only a `conductor.config/2` file that writes `daemon.store`
([configure an instance](./configure-an-instance.md)). It listens on `127.0.0.1` at
`daemon.port`, 7300 unless the file writes another. `task dashboard` prints the active
instance's page:

```console
$ task dashboard
Open http://127.0.0.1:7300/?instance=personal
```

The page shows the instance `?instance=` names, else the first that `GET /v1/instances` lists.
The selector in its header switches instance and keeps the view.

For each instance the daemon starts a runtime, `conductor instance serve`, which serves the
instance API on `api.sock` in its state directory, and proxies `/v1/instances/<name>/<route>` to
it. The runtime runs the loop, so the data changes each `cadence.cycle`, or when a command such
as `conductor snapshot start-snapshot` writes to the store.

## 2. The views

| view | shows |
|---|---|
| Overview, `#/` | KPIs, what waits for review, trends, what needs the operator, the repositories with red CI on `main`, today's events, disk and spend |
| Decisions, `#/decisions` | every open class-O and class-H request with its options, and the decision log. The operator answers here |
| Plan, `#/plan` | one repository's planning store as a board, grouped by wave, with the moves each story's lifecycle allows |
| Metrics, `#/metrics` | a chart per series over 7, 30 or 90 days |
| KPIs, `#/kpis` | every KPI against its target |
| Sessions, `#/sessions` | the instance's sessions with their dispatch, what awaits the operator, usage limits and context above `thresholds.context_handover`, recent decisions, token spend since 00:00 UTC, and each source that could not be read |
| Health, `#/health` | doctor's rows, the newest snapshot and why it failed, collector times, members it could not read, free disk and disk waste |
| Configuration, `#/config` | every key of `conductor config show` with its YAML path. Edits are staged, checked, shown as a YAML diff and written with a reason |
| `#/repo/<name>` | one repository: what it waits on and works on, its `main` CI and release, its pull requests, plan, sessions and today's events |

Each view reads its routes when it opens and every 20 seconds.

## 3. Read the data

Every view reads an instance route through the daemon. The Sessions view reads `data.json`:

```console
$ curl -s http://127.0.0.1:7300/v1/instances/personal/data.json | jq -c '{controllers, snapshot, build_slot, awaiting_operator, errors}'
{"controllers":0,"snapshot":null,"build_slot":"not recorded","awaiting_operator":[],"errors":[]}
```

`snapshot` is `null` until the first complete snapshot. `data.json` writes every path under the
home directory as `~`. The charts and KPI panels read the daily
rollups of the store:

```console
$ curl -s http://127.0.0.1:7300/v1/instances/personal/series | jq -c '.[0]'
$ curl -s 'http://127.0.0.1:7300/v1/instances/personal/series?series=prs_merged&by=repository&days=30' | jq -c '.rows[0]'
$ curl -s http://127.0.0.1:7300/v1/instances/personal/kpis | jq -c '.[] | {id, value, measured}'
```

`series` without a query lists every series. With one, it answers rows of `day` (UTC), `key` and
`value` for the last `days`, 30 by default, 1 to 365. A series with no source yet answers 404,
an unknown series or `by` 400 with the allowed values. `kpis` answers each KPI over the last 7
days, or `measured: false`.

The daemon's own routes cover every instance:

| route | answers |
|---|---|
| `GET /v1/instances` | each instance of the config file: `name`, `state`, `placement`, `roles`, `binary_version`, `last_seen` |
| `GET /v1/sessions`, `/v1/decisions`, `/v1/unanswered`, `/v1/kpis` | the union of every instance's rows, each with `instance` |
| `GET /v1/events` | every instance's events as server-sent events |

An instance that does not answer within `daemon.timeout`, 5 s by default, is one row with
`state: unreachable`, and the view still answers 200.

## 4. What it writes

The loopback listener answers only requests that name its own host, else 403. Reads are open. A
write comes from the daemon's own page, which carries the daemon's token, and is recorded as the
operator's:

| write | grant |
|---|---|
| answers on the Decisions page, `POST .../answers` | `answer-decisions` |
| a Plan page move, `POST .../plan` | `answer-decisions` |
| the Configuration page, `PUT .../settings`, and `PUT /v1/config` | `change-settings` |
| an upgrade, `POST /v1/instances/<name>/upgrade` | `upgrade-instances` |
| `POST /v1/instances/<name>/join` and `leave` | `join` |

While `daemon.operators` is empty, the Unix user holds every grant on loopback. Otherwise an
operator without the grant is refused 403 with `conductor.operator.GrantMissing`. An answer is
recorded as the operator's decision, `recorded_by` `dashboard`, its request moves to Answered,
and the watch prints `answered <request-id> <choice>`. Who may write, and what that does not stop,
is in [operators and credentials](../security/operators-and-credentials.md). The TLS listener for a
browser on another machine is in [remote daemon](./remote-daemon.md).

## 5. The terminal UI

```console
$ conductor tui
```

`conductor tui`, or `conductor` with no arguments at a terminal, opens a read-only view of the
daemon's global API: the instances, the sessions of the selected one, and its settings.

| key | does |
|---|---|
| `Enter` | attach to the selected session |
| `w` | watch it, read-only |
| `l` | follow its terminal log |
| `ctrl-]`, then `d` | come back, the session untouched |

It reads the daemon `--daemon <url>` names, else `CONDUCTOR_DAEMON`, else
`127.0.0.1:<daemon.port>`. Against a daemon on another host it presents your
[`conductor login`](./remote-daemon.md). Attaching there is not served yet and answers `501`.

To change the app itself, see [regenerate and check](../develop/regenerate-and-check.md).
