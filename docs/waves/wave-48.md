# Wave 48 (the loop acts on what it already sees)

Four defects a host supervisor reported on 2026-11-07 and 2026-11-08: a KPI counted repositories
nobody works on, a controller sat at its prompt for hours after its blocking request merged, the
loop stopped a session restarted by hand under the same name, and controllers stood at a harness
prompt for an `rm` on a bare variable. One tree, one release.

## Units

| unit | story | note |
|---|---|---|
| U1 | kpi-scope-active-repositories | repository-scoped KPIs (K5, and K4 and K6 where they are repository-scoped) count only repositories Active on the reading's day; the instance row sums those |
| U2 | watch-unblocks-merged-requests | a Blocked dispatch (ci, review, mr-route) naming a request that merged or closed is unblocked and its controller told; a red head pipeline is sent once per head commit; `session silent:` carries the minutes since the last turn and whether the pane is at the prompt |
| U3 | handover-restart-by-controller-id | a pending hand-over is keyed by controller id and never stops a session started after its delivery; each `type_text` pastes a buffer of its own name |
| U4 | controller-rm-guarded-variables | the controller profile writes a variable in an `rm` path as `"${VAR:?}"` |
| U5 | delivery-submitted-or-reported | after its carriage return a delivery reads the pane and returns again while the input line holds its text, at most 3 times; each delivery logs its id and whether it was submitted. Lands on `type_text` after U3 |
| U6 | build-shared-target-opt-out | `repositories[].shared_target: false` sends that repository's cargo runs to `<tree>/target`; the shared target stays the default |

The supervisor profile's "Releases and announcements" section also gains how to tell whether a
merged pull request is in a release (`git merge-base --is-ancestor`).

## Metric

| | before | after |
|---|---|---|
| U1: K5.unreleased, instance row, 2026-11-07 (`conductor metric kpi-readings`) | 60, over 22 repositories with a non-zero row; 5 of 77 repositories Active (`conductor repository activity`, 2026-11-08) | |
| U2: hours a controller sat idle after its blocking request merged | about 16 h and 342 min, two controllers on two instances (2026-11-07/08) | |
| U3: sessions the loop stopped within 10 min of their start by hand | 1, plus 1 `LaunchFailed … paste-buffer` (2026-11-08) | |
| U4: controllers stopped at a "possibly-empty variable path" prompt | 2 in one day (2026-11-08) | |
| U5: deliveries left unsubmitted in a pane, seen | 1, 33 min idle (2026-11-08) | |
| U6: repositories needing `CARGO_TARGET_DIR` set by hand under `build run` | 1, with 10 failing gate steps (2026-11-08) | |
