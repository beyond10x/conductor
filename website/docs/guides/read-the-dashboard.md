---
title: Read the dashboard
sidebar_position: 4
description: Serve the read-only dashboard on 127.0.0.1, read each of its sections, and read the same data as JSON.
lede: The dashboard is one read-only page of what every session works on, rebuilt from its sources on every request and written to nothing.
source: crates/conductor/src/dashboard.rs, crates/conductor/src/dashboard/; crates/conductor/tests/dashboard.rs; Taskfile.yml, dashboard; conductor dashboard serve --help
---

## 1. Serve it

As a user service on port 7313, from a checkout of the conductor repository:

```console
task dashboard            # starts conductor-dashboard unless it is active, prints http://127.0.0.1:7313/
task dashboard:stop
```

Or in the foreground, on any port (`0` picks a free one):

```console
$ conductor dashboard serve --port 7399
listening on http://127.0.0.1:7399
```

It listens on `127.0.0.1` only and answers two requests: `GET /`, an HTML page that reloads itself
every 20 seconds, and `GET /data.json`, the same data as JSON. Any other path answers 404, and a
request naming another host answers 403, so a page served from elsewhere cannot read the data:

```console
$ curl -s -o /dev/null -w "%{http_code}\n" http://127.0.0.1:7399/other
404
$ curl -s -o /dev/null -w "%{http_code}\n" -H "Host: example.org" http://127.0.0.1:7399/
403
```

## 2. What it reads

With a config file, the dashboard reads the instance's `records` (unless `--root` names another
directory), the watch's files under its `state`, and the sessions under its checkouts root. It
names what it read at the foot of the page and in `reading`:

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

| section | shows | from |
|---|---|---|
| header | when it was read, free disk on `/`, controllers running, build slot | `df`, the session list |
| Sessions under the checkouts root | each session's name, kind, state, age, working directory, and its current dispatch: goal, brief, last event; exited sessions below the live ones | `claude agents --json`, `dispatches/*.jsonl` |
| Usage limit and context | usage limits found now and earlier today, and sessions over 300k tokens of context | the watch's files |
| Red CI on main | each workflow whose newest `main` run failed | the watch's CI state |
| Awaiting the operator | decisions whose newest line is class O or H with no choice yet | `decisions/*.jsonl` |
| Recent decisions | the newest decision lines: id, class, choice, reason | `decisions/*.jsonl` |
| Disk waste | per tracked repository: build cache of idle managed trees, trees eligible for removal, worktree archives and the checkout's `target/`; shared: build targets, toolchains and scratch. Largest first, with each row's owner | `worktree sweep --dry-run`, `worktree gc --dry-run`, `du`, measured in the background |
| Token spend | tokens per repository since 00:00 UTC | the session transcripts, measured in the background |
| Could not read | each source that failed; the other sections still show | |

A controller counts as running when a live session named after its repository runs in that
repository's checkout. The two background measurements show the last complete result with its
age; until the first one finishes the page says so.

The header and the operator's queue as JSON:

```console
$ curl -s http://127.0.0.1:7399/data.json | jq -c '{controllers, build_slot, awaiting_operator, red_ci, errors}'
{"controllers":0,"build_slot":"not recorded","awaiting_operator":[{"class":"O","id":"DEC-20261008-02","question":"Split beta into two repositories?"}],"red_ci":null,"errors":[]}
$ curl -s http://127.0.0.1:7399/data.json | jq -c '.decisions[]'
{"choice":null,"class":"O","id":"DEC-20261008-02","reason":null}
{"choice":"A","class":"C","id":"DEC-20261008-01","reason":"gate green, change inside the approved design"}
```

`red_ci` is `null` because no watch has run yet; the page says "no watch state". The build slot is
not recorded by any command yet.

## 4. What it does not do

The dashboard writes nothing and changes nothing: no button cleans up a tree or answers a
decision. Cleanup goes through each tree's owner; a decision through `conductor decision`. It
writes every path under the home directory as `~`.
