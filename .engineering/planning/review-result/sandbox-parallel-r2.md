---
format: aep.planning-md/3
id: review-result:sandbox-parallel-r2
kind: review-result
status: active
title: Parallel-safety critic, round 2, sandbox
relations:
- reviews: story:sandbox-spec
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
approve

Round 1 had nine findings. None of them still holds, and the revision introduced no new collision inside the set. Each pair is ordered by a direct `depends_on` edge, and the bodies name the shared files.

**Round 1 findings, one by one**

1. `story:session-writes-bwrap` against `story:gitlab-guide` on README, design and `the-guard.md`: **does not hold**.
   - The seam now has `depends_on story:gitlab-guide`, so writes is ordered after it through `writes → limits → seam → gitlab-guide`.
   - Writes' Notes name the pair and the three shared pages (`session-writes-bwrap.md`, Notes).
2. The seam against `gitlab-repositories` and `gitlab-workspace` on the generated model: **does not hold**.
   - The seam no longer lists `crates/conductor-model` or `spec/`. `story:sandbox-spec` owns them.
   - `sandbox-spec` has `depends_on story:gitlab-guide`, which depends on `gh-optional`, `gitlab-source-host`, `gitlab-repositories`, `gitlab-merge-requests` and `gitlab-workspace`.
3. The seam against `story:watch-gitlab-ci` on `tests/watch.rs`: **does not hold**.
   - The seam's Notes name `tests/watch.rs` as shared with wave 17.
   - The seam is ordered after `watch-gitlab-ci` through `gitlab-guide`.
4. The seam's shared-file list naming `Taskfile.yml` and `tests/taskfile.rs`: **does not hold**.
   - The Notes now list `doctor.rs`, `tests/doctor.rs`, `tests/watch.rs`, `.agents/conductor.md`, `tests/profiles.rs` and `tests/docs_forge.rs`.
   - I checked these against the stories' scopes and against `aep plan artifact waves --kind story --status draft`, and they match.
   - `Taskfile.yml` and `tests/taskfile.rs` appear in no wave-17 scope and collide with nothing in the waves output.
5. The seam omitting `website/docs/reference/cli.md`: **does not hold**.
   - The `[session, start]` binding and its `cli.md` regeneration moved to `story:sandbox-spec`, which lists `cli.md` and `cli.yaml`.
   - The seam only fills the handler in `cli.rs`. `cli_tree.rs` is spec-derived, so the seam needs no edit there.
6. The seam omitting three hand-written pages: **does not hold**.
   - `website/docs/security/confinement.md`, `website/docs/guides/start-conductor.md` and `docs/config.md` are now in its scope and acceptance.
7. `story:session-limits-systemd` omitting `docs/config.md`: **does not hold**.
   - It is in the scope, and the Notes say the seam edits it first.
8. `story:session-forge-credentials` omitting `docs/config.md`: **does not hold**.
   - It is in the scope, and the Notes name it as shared with writes.
9. The forge story's `gitlab-workspace` edge with no stated reason: **does not hold**.
   - The edge is removed, and the Notes say the seam's edge to `gitlab-guide` replaces it.

**Ordering in the set**
- `aep plan artifact waves` puts `sandbox-spec`, the seam, limits, writes and forge in waves 8, 9, 10, 11 and 12. These are strictly serial, directly after `gitlab-guide` in wave 7.
- No two set items share a wave.
- Every in-set collision the tool lists is ordered by the direct chain: `sandbox-spec`, then seam, then limits, then writes, then forge.

**What I read**
- Five set stories in full.
- `review-result:sandbox-parallel-r1`.
- `docs/waves/wave-17.md`.
- `story:host-grants` and `story:watch-unsent-report`, to place the outside collisions.
- Commands:
  - `aep plan artifact waves --kind story --status draft`
  - `aep plan artifact graph`
  - `aep plan artifact show` on each story
  - `git grep` in the tree for `claude --bg`, `docs/config.md`, `components.yaml` and the `conductor-docs` generators
- Surfaces: 5 items cited, 0 inferred, 0 unplaceable.
- Acceptance reads traced to a producer in the set: 7.
  - 4 are recorded by a direct `depends_on` edge:
    - seam from `sandbox-spec`
    - limits from seam
    - writes from limits
    - forge from writes
  - 3 are recorded only through the chain: limits, writes and forge each read `sandbox-spec` types, and each is ordered by the chain through the seam.
- Reads of wave-17 producers: seam from `doctor-minimum-versions` (direct), `doctor-verdict` (through `doctor-minimum-versions`), `profiles-forge-neutral` (direct), `version-flag` (direct), and `gitlab-guide`'s `docs_forge.rs` (direct).

**What I could not establish**
- Three collisions are between a set item and a draft story outside the set. No `depends_on` edge and no body in the set names them. They do not set my verdict, because outside items are not in my lane. The waves tool already puts the outside story in an earlier wave than the set item, apart from the last pair below, but no edge records it. Someone should decide before the set is dispatched:
  - `story:sandbox-spec` against `story:host-grants` on `crates/conductor-model`, `crates/conductor/cli.yaml`, `crates/conductor/src/cli.rs`, `spec/ess-inputs.yaml` and `spec/system.yaml`. The seam also meets it on `cli.rs` and `.agents/conductor.md`.
  - `story:session-envelope-seam` against `story:watch-unsent-report` on `crates/conductor/tests/watch.rs`.
  - `story:session-forge-credentials` against `story:quota-support-report` on `doctor.rs` and `tests/doctor.rs`. That story depends on the seam, which puts both after it, but no edge orders them against each other.
- Wave 17's other units (`gc-archive-profile`, `dashboard-worktree-repo`, the `sec_glab_*` parser fixes) are not in `gitlab-guide`'s edge closure. The waves output shows no shared surface between them and the set, so "after all of wave 17" is a statement in the plan, not a recorded edge. The parser-fix stories are not in the store, so I could not check them.
- Out of my lane (acceptance critic): the seam's provider rows land "after the program rows and before the verdict line, which stays last", while forge adds per-role doctor lines. Its acceptance does not say where those sit relative to the verdict line.
- Out of my lane (scope critic): `docs/design/conductor.md:481` says "Both sessions run in the background (`claude --bg`)". That goes stale when the seam lands, and the seam does not list the page.
- Out of my lane (design critic): `decision-blocker:sandbox-forge-tokens` blocks forge, which sits at the end of the one-worktree chain.

```findings
[]
```
