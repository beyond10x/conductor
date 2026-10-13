---
format: aep.planning-md/3
id: review-result:forge-parallel-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2, epic forge-neutral
relations:
- reviews: story:gitlab-source-host
- reviews: story:gitlab-repositories
- reviews: story:gitlab-merge-requests
- reviews: story:gitlab-workspace
- reviews: story:watch-gitlab-ci
- reviews: story:guard-glab-writes
- reviews: story:gh-optional
- reviews: story:profiles-forge-neutral
- reviews: story:gitlab-guide
- reviews: story:doctor-verdict
- reviews: story:doctor-minimum-versions
- reviews: story:version-flag
- reviews: story:gc-archive-profile
revision: 1
---
needs-revision

Seven of the ten round-1 findings are fixed and two are fixed in part. One part of finding 4 still holds. Three warnings remain, one of them new.

Round-1 findings, by their order in `review-result:forge-parallel-r1`:

1. `story:gitlab-guide`, missing edges: fixed. It now depends on all nine stories whose behaviour it documents (`.engineering/planning/story/gitlab-guide.md`, relations), and it sits alone in wave 7 behind them.
2. `story:doctor-minimum-versions`, shared files not named: fixed. Its Notes now name `doctor.rs` and `tests/doctor.rs` with the edge on `doctor-verdict`.
3. `story:doctor-minimum-versions`, scope: fixed. `build.rs` is marked new, `Cargo.toml` is in scope, and the pin wording in README, getting-started and troubleshoot moved to `gitlab-guide`.
4. `story:doctor-verdict`, partly fixed. The sample output and README text are now `gitlab-guide`'s, which comes after `doctor-verdict`. The `cli.yaml:196` `about` line and generated `cli.md:124` are still unlisted. The line that goes stale is "older than its b10x.toml pin", and `doctor-minimum-versions` makes it stale, so the first finding below sits on that story.
5. `story:gitlab-source-host`: fixed. The generated model, `config.md` and `grouped_instance.rs` are in scope, and its Notes name the sharing.
6. `story:gitlab-workspace`: fixed. The generated files are in scope, ordered after `gitlab-source-host` and `gitlab-repositories`, and `instances-and-config.md` is `gitlab-guide`'s.
7. `story:gitlab-repositories`, partly fixed. `grouped_instance.rs` is in scope and the docs moved to `gitlab-guide`. The `instance_sources.rs` gitlab case is still in nobody's scope as far as this story goes (second finding).
8. `story:gh-optional`, partly fixed. `instance_sources.rs` is now named and in scope, but the other broken test file is not (third finding).
9. `story:watch-gitlab-ci`: fixed. The gh-only prose moved to `gitlab-guide`, and edges now run to `gitlab-merge-requests` and `gitlab-source-host`.
10. `story:guard-glab-writes`: fixed. The README, design and decisions prose moved to `gitlab-guide`.

Findings, round 2:

story:gh-optional — its second acceptance makes `collect::programs` ask for `glab` for a gitlab source and refuses a snapshot start when it is missing (`crates/conductor/src/snapshot.rs:598`), so the existing gitlab case `a_gitlab_source_asks_nothing_and_config_show_prints_it` (`crates/conductor/tests/grouped_instance.rs:677-690`) fails. That case starts a snapshot expecting exit 0 on a `PATH` that holds only `gh`, `aep`, `claude` and `git` (the `programs` helper at `:186`). The file is in neither this story's scope nor its Notes, though `story:gitlab-source-host` (before it) and `story:gitlab-repositories` (after it) both claim it. This is a surface the body does not admit (cited). Name the file and the fake `glab` in this scope, or leave the case to `story:gitlab-source-host` and make the order explicit — `.engineering/planning/story/gh-optional.md:56`, `crates/conductor/tests/grouped_instance.rs:677`

story:gitlab-repositories — once it lands, the gitlab case at `crates/conductor/tests/instance_sources.rs:1468` runs real collectors against whatever fake `glab` `story:gh-optional` put on `PATH`. `instance_sources.rs` is in `gh-optional`'s scope only, and `gitlab-repositories` neither claims it nor says it is changed. That the stub breaks is inferred, not read. The file is ordered by an existing edge, so this is an unnamed surface, not a parallel collision — `.engineering/planning/story/gitlab-repositories.md:62`, `crates/conductor/tests/instance_sources.rs:1468`

story:doctor-minimum-versions — the `about` line of `conductor doctor` (`crates/conductor/cli.yaml:196`, "older than its b10x.toml pin") stops being true once a built-in minimum applies without a `b10x.toml`. Changing it regenerates `website/docs/reference/cli.md:124`. `story:version-flag` also lands on `cli.md` and no edge orders it against any doctor story. The two edit different lines, but both regenerate the whole file, so `docs-check` fails for whichever merges second until it regenerates. Surface inferred. Either state in the body that the `about` line stays, or add `cli.yaml` and `cli.md` to the scope and order the two stories — `.engineering/planning/story/doctor-minimum-versions.md:42`, `crates/conductor/cli.yaml:196`

What I read: 13 stories, the round-1 record, the epic only through the graph, and the tree. I ran `aep plan artifact waves --kind story --status draft`, `aep plan artifact graph` and `aep plan artifact show` on each story and on `review-result:forge-parallel-r1`, and used `rg` over `crates/conductor/{src,tests}`, `cli.yaml`, `Taskfile.yml`, the website docs and the profiles.

Surfaces established: 13 cited, 0 inferred, 0 unplaced. Acceptance reads traced to a producer in the set: 18, all recorded by a `depends_on` edge (17 direct, 1 through a path: `watch-gitlab-ci` to `gitlab-repositories` via `gitlab-merge-requests`). Every collision `waves` prints between two items of this set is either ordered by an edge or one of the three findings above. The new files (`collect/gitlab.rs`, `collect_gitlab.rs`, `fixtures/gitlab`, `docs_forge.rs`, `guides/gitlab.md`, `build.rs`) are marked new and do not exist yet. `gitlab-repositories` and `gitlab-merge-requests` both create `collect/gitlab.rs`, and the edge orders them.

What I could not establish:
- Collisions with items outside the set that `waves` lists, which I did not judge: `gitlab-guide` with `session-writes-bwrap` on `README.md` and `docs/design/conductor.md`; `watch-gitlab-ci` with `watch-unsent-report` on `watch.rs`; `version-flag` with `host-grants` on `cli.rs`; `profiles-forge-neutral` with `host-grants` and `session-envelope-seam` on `.agents/conductor.md`; the doctor stories with `quota-support-report` and `session-envelope-seam` on `doctor.rs`; and the set's spec and model stories with the substrate stories on `spec/domains/config.yaml` and `crates/conductor-model`.
- Whether `docs-check` also regenerates `website/docs/status.md` from story status. If it does, every story touches it, which is a store-wide matter and not one for this set.
- Out of my lane, for the acceptance critic: `gitlab-guide` lists `README.md`, `getting-started.md` and `troubleshoot.md` in its scope for the pin wording, and the gh-only CI and guard prose pages, but its acceptance tests neither. `website/docs/security/the-guard.md` carries gh-only guard prose and is in nobody's scope (scope critic).

```findings
- file: .engineering/planning/story/gh-optional.md
  line: 56
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its second acceptance makes collect::programs ask for glab for a gitlab source and refuse a snapshot start without it (snapshot.rs:598), which breaks the existing gitlab case at crates/conductor/tests/grouped_instance.rs:677-690 (PATH holds only gh, aep, claude, git, expects exit 0); that file is in neither this scope nor its Notes, though story:gitlab-source-host (before) and story:gitlab-repositories (after) both claim it (cited)"
- file: .engineering/planning/story/gitlab-repositories.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "once it lands, the gitlab case at crates/conductor/tests/instance_sources.rs:1468 runs real collectors against the fake glab story:gh-optional left on PATH; instance_sources.rs is in gh-optional's scope only and this body neither claims nor mentions it (inferred: that the stub breaks is not read, only that the case is exposed)"
- file: .engineering/planning/story/doctor-minimum-versions.md
  line: 42
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the doctor about line in crates/conductor/cli.yaml:196 ('older than its b10x.toml pin') stops being true with a built-in minimum, and changing it regenerates website/docs/reference/cli.md:124, which story:version-flag also regenerates with no edge between them; the body neither keeps the line nor lists cli.yaml and cli.md (inferred surface)"
```
