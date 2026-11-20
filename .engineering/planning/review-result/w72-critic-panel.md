---
format: aep.planning-md/3
id: review-result:w72-critic-panel
kind: review-result
status: active
title: Critic panel, wave 72 (person-wait-and-pause-hold-restarts)
summary: scope PASS-WITH-CHANGES; design PASS-WITH-CHANGES; acceptance PASS-WITH-CHANGES; parallel-safety PASS-WITH-CHANGES; all edits taken in, no second round
relations:
- reviews: story:person-wait-and-pause-hold-restarts
prose_only: a planning panel over one story; its findings are edits taken into the story body
revision: 1
---
PASS-WITH-CHANGES (every required edit taken into revision 3 of the story; no lens FAILed, so no second round)

Four lenses over `story:person-wait-and-pause-hold-restarts` revision 2, against the wave branch's base (release 1.21.0 plus wave 70).

## Scope: PASS-WITH-CHANGES

- Kept one unit; Boundaries now say why (same functions and test file).
- `run/dispatch.rs` only its module doc (`:10-12`); a new `NotStarted::Held` / `Again::Held` variant named instead of reusing `Refused`.
- Citations fixed: `run/dispatch.rs:268` -> `:271`, `run.rs:899-978` -> `:899-975`.
- `person.json` named in `run/handlers.rs`'s module doc and the design page; `website/docs/guides/start-conductor.md` (`:89-95`) added to scope with one sentence on pause.
- Overlapping drafts named in Boundaries: `handover-restart-by-envelope-id`, `controllers-reconciled-with-live-sessions`, `merge-wakes-idle-controller`.

## Design: PASS-WITH-CHANGES

- The person pass runs again after the mailbox and `disk_recovered`, each pass followed by the kept deliveries, so an input kept in a tick starts its controller in that tick.
- A kept item with `refused_by` set does not end a person wait.
- Profile: a dispatch blocked for another reason is reported `unblocked` first, then `blocked person: …`; after an input a controller still waiting reports it again and ends its turn, its idle session stopped into the hold by the silent restart.
- `Held` variant; `restart_with` writes `<repo> waits: …`, records `person.json`, forgets the session; reconcile and spawn take it as a wait; any refusal of the lift keeps the entry.
- The dispatches log read only when every open dispatch is `Blocked`; one parse shared with `blocked_on_disk`.
- Pause race (`pause_here` ends the session before it records `Paused`, in another process) named in Boundaries as a follow-up.
- `resume_stopped` on a `Paused` repository takes the session out of `limited.json`.

## Acceptance: PASS-WITH-CHANGES

- Same-tick starts made possible by the second person pass.
- Refusal text strips the leading `person` and `:` so it never reads `person: person:`.
- Socket note corrected: nothing binds a socket; the cases need a host where `socket(AF_UNIX)` may be created (else the `none` provider becomes `child`, `confine/mod.rs:250`), so they run on CI.
- Tests added: one journal line for a repeated refusal, red `main` kept not spawned, cancelled dispatch drops the entry, lift refused at `max_working` keeps the entry, a paused repository's entry dropped, a paused repository's stopped limit mark resumes nothing, a refused-for-good delivery ends no wait, a person wait after another block. `withdrawn` named in the pause bullet; the cycle case uses `stop_alpha`, the hand-over case `handed_over` and asserts the journal line and the `person.json` entry.

## Parallel safety (vs story:supervisor-watch-forge-triggers): PASS-WITH-CHANGES

- Scopes disjoint by path component; `crates/conductor/src/watch.rs` added to that story's list in Boundaries.
- Report form fixed as `blocked person: waits on <who> for <what>`, so the supervisor watch's idle check (it skips reports containing `waits on`) holds the controller back without a change there; reading `blocked person` there, and the supervisor's by-hand restart rule, are follow-ups.
- No other wave-72 unit scopes `run.rs`, `run/*.rs`, `tests/run.rs` or `roles/controller.md`.
