---
format: aep.planning-md/3
id: story:supervisor-watch-forge-triggers
kind: story
status: implemented
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
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/src/supervisor_watch
- confidence: cited
  path: crates/conductor/src/watch.rs
- confidence: cited
  path: crates/conductor/tests/supervisor_watch.rs
- confidence: cited
  path: crates/conductor/tests/supervisor_watch_forge.rs
- confidence: cited
  path: spec/domains/host.yaml
revision: 14
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T13:56:16Z", actor: "human:unknown", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-20T13:56:16Z", actor: "human:unknown", revision: 5}
- {from: "active", to: "implemented", at: "2026-11-20T17:26:43Z", actor: "human:unknown", revision: 14, decided_on: {"recorded":{"test_result":2}}}
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
   the instance's sources whose head pipeline or checks succeeded over 15 min ago (the newest
   check end time on its head sha), with no auto-merge or merge-when-pipeline-succeeds set;
   reported at most once per head sha (`supervisor-watch/<instance>/green`, each line kept 7 days
   after a read last saw its request, so a request that drops out of one read is not named
   again); a ref given with `--hold` (`<repo>#<n>`, `<repo>!<n>`, `#<n>`, `!<n>`, `<n>` or the
   request's URL) is left out. Subject: `<repo>!<iid>` (GitHub `<repo>#<n>`); detail:
   `green <m>m, head <sha>`.
9. `CIEND`: the newest report of a repository a `Running` controller works on, in the instance's
   mailbox (as `IDLE` reads it, `supervisor_watch/local.rs` `newest_reports`), is
   `blocked ci <ref>`, and the checks or head pipeline of the open request the ref names (in the
   repository the ref names, else the controller's) ended after the report's ULID time. Once per
   ref and report (`supervisor-watch/<instance>/ciend`, kept as `green` is). Detail:
   `<ref> <green|red>, ended <time>`.

Forge reads (`crates/conductor/src/supervisor_watch/forge.rs`):

- With the host's `gh` and `glab` over the instance's own `github` and `gitlab` sources
  (`Origins::of`, `Forge::all`); the programs are `Sources::gh` and `Sources::glab`, the watch's
  one seam; the command adds no credential.
- At most every 2 min per instance (`supervisor-watch/<instance>/forge`, across runs), and only
  for repositories with an open request: GitHub `gh search prs --owner <owner> --state open`
  names them (an `include` list filters), then `gh pr list -R <owner>/<repo>` reads `isDraft`,
  `autoMergeRequest` and the rollup with each check's `completedAt`; GitLab
  `groups/<group>/merge_requests?state=opened` (`draft`, `merge_when_pipeline_succeeds`), the
  group's projects for the names, and `projects/<id>/merge_requests/<iid>` for
  `head_pipeline.status` and `finished_at` (`supervisor-watch/<instance>/gitlab`). The detail is
  read again on a new head, while a `CIEND` waits on the request, while its pipeline runs or
  failed or was cancelled (a job or pipeline run again on the same head), and with no pipeline 30
  min after the last read; a pipeline that ended green on the head is not read again.
- The reads of one pass stop at `Sources::forge_budget` (60 s), or at the run's `--timeout` when
  nearer, with one note; the rest is read at the next due pass. The last reading of a run, once
  lines are pending, reads no forge.
- A refused or rate-limited answer (`gh`/`glab` exit non-zero: HTTP 403/429, no login on the host
  for a boxed instance's forge) is one note per run on standard error naming the instance and the
  repository or group, and doubles that instance's interval up to 16 min until a pass reads every
  source; what the other reads of that pass found is kept, and the other triggers still fire. A
  forge cursor that cannot be read or written is one note too. An instance with
  `forge_access: anonymous` is not read, one note per run.
- `watch.rs`'s `Check` gains `completedAt`/`startedAt` (serde defaults) and `ended_at`, shared
  with `forge.rs`; the per-instance watch's lines stay byte for byte the same (`tests/watch.rs`
  unchanged and green).

## Acceptance (tests that fail on main)

In `crates/conductor/tests/supervisor_watch_forge.rs`, over the library seam of
`story:supervisor-watch-command` (`Watch::pass`, `Watch::commit`, `Watch::run`, `Sources`) with a
clock the case sets, fake `gh` and `glab` programs (`sh` scripts the case writes, printing JSON
fixtures and logging their words), a scratch directory and the instance fixed with
`active::isolate` (rule 5); no network, no real forge:

- `green_names_an_unmerged_green_request_once_per_head_sha_and_skips_a_hold`
- `green_skips_a_draft_and_an_auto_merge_request_and_one_green_under_fifteen_minutes`
- `green_reads_a_gitlab_merge_request_and_a_github_pull_request` (and a request's detail is read
  again only while its pipeline runs)
- `ciend_fires_when_the_blocking_pipeline_ends_after_the_report`
- `a_rate_limited_forge_is_a_note_and_backs_off_while_the_other_triggers_fire`
- `the_forge_is_read_at_most_every_two_minutes_per_instance` (the fake program logs its calls)

From the adversary review of the first cut, each failing on it first:

- `a_gitlab_pipeline_run_again_on_the_same_head_fires_ciend_and_green`
- `ciend_matches_the_repository_its_ref_names`
- `a_pass_reads_the_forge_within_its_budget_and_notes_the_rest`
- `the_final_reading_of_a_run_reads_no_forge_once_lines_are_pending`
- `a_gitlab_request_with_no_pipeline_is_read_again_after_half_an_hour_or_a_new_head`
- `a_refused_pass_keeps_what_the_gitlab_read_beside_it_found`
- `an_unreadable_forge_cursor_is_a_note_and_the_other_triggers_fire`
- `green_is_not_named_again_when_a_request_drops_out_of_a_search`

And `cargo test -p conductor-cli --test watch` passes unchanged; `tests/supervisor_watch.rs`
points its sources, and the binary's `PATH`, at fakes that answer `[]`.

## Boundaries

- Depends on `story:supervisor-watch-command`; adds no noun, field, error or config key. The
  specification's comments on `conductor.host.WatchSupervisor` (`hold`) and
  `SupervisorTriggerKind` (`Green`, `CiEnd`) say what landed; the generated crate is unchanged.
  The supervisor profile's trigger list names `GREEN` and `CIEND` (no line added).
- #247's cycle line, dashboard and `conductor status` listing and the delivery to a controller stay
  with #247.
- No organization in code or fixtures (rule 4): `example-org`.

## Scope

`crates/conductor/src/supervisor_watch/forge.rs` (new), `crates/conductor/src/supervisor_watch/mod.rs`
(`Sources::gh`, `Sources::glab`, `mod forge;` and its call), `crates/conductor/src/supervisor_watch/local.rs`
(helpers shared with `forge.rs`), `crates/conductor/src/watch.rs` (`Check` end times),
`spec/domains/host.yaml` (comments), `crates/conductor/roles/supervisor.md` (trigger list),
`crates/conductor/tests/supervisor_watch_forge.rs` (new), `crates/conductor/tests/supervisor_watch.rs`
(fake forges).
