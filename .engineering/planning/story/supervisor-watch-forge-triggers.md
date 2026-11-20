---
format: aep.planning-md/3
id: story:supervisor-watch-forge-triggers
kind: story
status: active
title: conductor supervisor watch fires GREEN and CIEND from the forge, rate-bounded
summary: 'The two forge-reading supervisor triggers: a green unmerged request once per head sha, and the end of a blocking pipeline'
refs:
- provider: github
  reference: beyond10x/conductor#247
- provider: github
  reference: beyond10x/conductor#254
relations:
- decomposes: epic:loop-without-handwork
- informed_by: story:supervisor-watch-command
- depends_on: story:supervisor-watch-command
scope:
- confidence: inferred
  path: crates/conductor/src/supervisor_watch/forge.rs
- confidence: cited
  path: crates/conductor/src/supervisor_watch/mod.rs
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: inferred
  path: crates/conductor/tests/supervisor_watch_forge.rs
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T13:56:16Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-20T13:56:16Z", actor: "human:unknown", revision: 5}
---
## Why

Issues #254 (triggers 7 and 9) and #247 (its trigger only). Split from
`story:supervisor-watch-command` by the wave 69 critic panel: these two triggers read the forge,
need readers `conductor watch` does not have yet, and carry the rate-limit and credential risks;
the command, its specification (the `Green` and `CiEnd` variants and `--hold` included) and the
other eight triggers land first.

On main (release 1.20.1):

- `Sources::pulls` (`crates/conductor/src/watch.rs:403-406`) prints only `number`, `headRefOid`
  and `statusCheckRollup`; `OpenPull` (`watch.rs:2878`) holds `number`, `head` and `green`: no
  draft flag, no auto-merge, no time the checks ended. `open_merge_requests` (`watch.rs:3023`) reads
  each request's `head_pipeline` status but not `finished_at`, `draft` or
  `merge_when_pipeline_succeeds`.
- The snapshot cannot stand in: `conductor.observation.RecordPullRequest`
  (`spec/domains/observation.yaml:1208`) has `draft` and `failing_checks` but no check end time
  and no auto-merge, and a snapshot is taken once per cycle.
- The forge seams exist: fake `gh` and `glab` programs under a case's directory
  (`crates/conductor/tests/watch.rs:1350`), and `Watch::with_gitlab` (`watch.rs:1151`).

## Outcome

`conductor supervisor watch` (from `story:supervisor-watch-command`) gains:

7. `GREEN` (folds in #247's trigger): an open, non-draft pull or merge request of a repository in
   the instance's sources whose head pipeline or check suite succeeded over 15 min ago (the newest
   check end time for its head sha), with no auto-merge or merge-when-pipeline-succeeds set;
   reported at most once per head sha (the shas kept in `supervisor-watch/<instance>/green`); a ref
   given with `--hold` is left out. Subject: `<repo>!<iid>` (GitHub `<repo>#<n>`); detail: minutes
   green.
9. `CIEND`: a controller's newest report, in the records' `dispatches/` log, is `blocked ci <ref>`,
   and that ref's head pipeline ended after the report's time. Once per ref and report. Detail:
   ref, conclusion, end time.

Forge reads:

- Through the instance's own sources and credentials, as `conductor watch` builds them
  (`Sources::new`, `Watch::with_gitlab`); the command adds no credential and no new program.
- At most every 2 min per instance, and only for repositories with an open request (GREEN) or a
  `blocked ci` report (CIEND). A GitLab request's detail is read again only when its head sha
  changed or its pipeline was still running at the last read.
- A refused or rate-limited answer (HTTP 403/429, `gh` exit non-zero) is one note on standard
  error naming the instance and repository, and doubles that instance's interval up to 16 min until
  a read succeeds. An instance whose credentials the host cannot read (a boxed instance) is one note
  per run, not an error; the other triggers still fire.
- The readers are extended in `watch.rs` (`isDraft`, `autoMergeRequest`, the rollup's
  `completedAt`; GitLab `draft`, `merge_when_pipeline_succeeds`, `head_pipeline.finished_at`);
  the per-instance watch's lines stay byte for byte the same (its tests unchanged).

## Acceptance (tests that fail on main)

In `crates/conductor/tests/supervisor_watch_forge.rs`, over the library seam of
`story:supervisor-watch-command` with a fixed clock, fake `gh` and `glab` programs that print
fixtures (as `tests/watch.rs:1350`), a temporary `HOME` and the instance fixed with
`active::isolate` (rule 5); no network, no real forge:

- `green_names_an_unmerged_green_request_once_per_head_sha_and_skips_a_hold`
- `green_skips_a_draft_and_an_auto_merge_request_and_one_green_under_fifteen_minutes`
- `green_reads_a_gitlab_merge_request_and_a_github_pull_request`
- `ciend_fires_when_the_blocking_pipeline_ends_after_the_report`
- `a_rate_limited_forge_is_a_note_and_backs_off_while_the_other_triggers_fire`
- `the_forge_is_read_at_most_every_two_minutes_per_instance` (the fake program counts its calls)

And `cargo test -p conductor-cli --test watch` passes unchanged.

## Boundaries

- Depends on `story:supervisor-watch-command`; adds no spec, no config key and no profile text
  (the profile, the `--hold` flag and the variants land there).
- #247's cycle line, dashboard and `conductor status` listing and the delivery to a controller stay
  with #247.
- No organization in code or fixtures (rule 4): `example-org`.

## Scope

`crates/conductor/src/supervisor_watch/forge.rs` (new), `crates/conductor/src/supervisor_watch/mod.rs`
(one `mod forge;` and its call), `crates/conductor/src/watch.rs` (readers extended),
`crates/conductor/tests/supervisor_watch_forge.rs` (new).
