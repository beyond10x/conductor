---
format: aep.planning-md/3
id: story:watch-unblocks-merged-requests
kind: story
status: active
title: A Blocked dispatch whose request merged or closed is unblocked and its controller told
summary: A controller blocked on CI sat idle 16 h after its auto-merge request merged
relations:
- decomposes: epic:loop-without-handwork
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T10:59:41Z", actor: "human:timo", revision: 3, decided_on: {"recorded":{"approval":1}}}
- {from: "proposed", to: "active", at: "2026-11-08T10:59:41Z", actor: "human:timo", revision: 4, decided_on: {"recorded":{"approval":1}}}
---
# Story: watch-unblocks-merged-requests

## Why

Host supervisor, 2026-11-08: a controller reported `blocked ci` for its merge request with
auto-merge set, ended its turn and sat at its prompt. The merge request merged (one on
2026-11-07 17:05Z, another on 2026-11-08 03:27Z); nothing told the controller. The dispatch
stayed Blocked, the session was live so no `session gone` path fired, and the run journal held
no line for it. One instance's main product repository was idle about 16 h, another repository
342 min. K5 missed on that instance for 2026-11-07 (K5.p90 40.7 h, K5.unreleased 111).
`story:watch-pr-checks` reports concluded checks of open pull requests; it does not act on a
Blocked dispatch whose request merged or closed.

## Acceptance

1. At a watch tick, for every Blocked dispatch whose reason is ci, review or mr-route and that
   names a pull or merge request, the watch reads the request's state on the forge.
2. Merged or closed: the controller is sent `PR <url> merged|closed at <t>, continue`, and the
   dispatch's unblock is recorded in the dispatch log. A test with a fixture forge answer.
3. Open with a red head pipeline: the controller is sent the failing job's name and URL, once per
   head commit. Test.
4. The watch's `session silent:` line (`crates/conductor/src/watch.rs`) carries the minutes since
   the session's last turn ended and whether its pane is at the prompt, both read from the
   session's typescript, so a controller idle at its prompt with an open dispatch is a journal
   line without a pane scan (host teacher, 2026-11-08: two controllers sat at their prompts for
   about 16 h and 342 min after their requests merged, with no journal line). Test.
5. The host's interim idle check is deleted once this ships (the supervisor owns it).

## Out of Scope

Retrying or merging the request on the controller's behalf.

## Ambiguities

None.
