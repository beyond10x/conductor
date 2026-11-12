---
format: aep.planning-md/3
id: review-result:pod-checkouts-security-round-1
kind: review-result
status: active
title: Source include list, daemon version and pod checkouts, security review round 1
relations:
- reviews: story:pod-source-checkouts
- reviews: story:source-include-list
- reviews: story:daemon-version-json
revision: 1
---
needs-change

Recorder: the block at the end writes the reviewer's `major` as `warning` and `minor` as `note`, the severities the store reads; the text above it keeps the reviewer's words. The entry of finding F2 carries no `verdict`: the reviewer wrote `PLAUSIBLE` (not run), which the store has no word for.
unit: security review of `include`, `GET /v1/version` and pod checkouts, commits 55f31e50f, 61c0f311e and 1a7838b3a (`812cb885e..917848c1e`), plus my one untracked test file
verdict: needs-change
cases: executed 20→23, red 1 (20 is the same three test files without my three cases, counted from the run below)
origin: introduced 3 / pre-existing 1 / undecided 0
wrote-outside-worktree: 6 paths (listed below)
needs-coordinator: whether trusting a clone from a host the caller chose is acceptable (F2)

1. `git diff --stat` is empty. `git status --short` shows only `?? crates/conductor/tests/sec_pod_checkouts.rs`, a test file. No implementation file was touched.

2. **Cases added** in `crates/conductor/tests/sec_pod_checkouts.rs`, first run alone:
- `a_listed_path_step_named_dot_git_runs_no_code_from_the_repository`: **red**. `sec_pod_checkouts.rs:350`: "the repository's fsmonitor program ran in the pod; the clone made platform/.git/.git: true; clones: ["clone --quiet -- https://gitlab.example.com/example-org/platform/.git.git …/root/.conductor-clone-platform%2F.git/partial/.git"]". Both passes exited 0.
- `hostile_listing_paths_write_nothing_outside_the_checkouts_root`: green.
- `a_clone_stopped_at_the_collector_budget_leaves_no_temporary_directory`: green (4.7 s).

3. **Suite run.** Command: `zsh …/sec/gate.sh`, which runs clippy, then `cargo nextest run -p conductor-cli --test sec_pod_checkouts --test pod_checkouts --test isolated_home`. Clippy exit 0. Summary: `23 tests run: 22 passed, 1 failed, 0 skipped`; the one failure is the `.git` case with the same message; nextest exit 100.

**Invariants**
1. **Does not hold.** Each listed step goes through `checked` (`gitlab.rs:610`), which accepts `.git`. `bring` (`checkouts.rs:168-184`) refuses neither `.git` nor a name starting `.conductor-clone-`.
   - The listing `example-org/platform/.git` is covered by the entry `platform`. Its clone's work tree is renamed to `<root>/platform/.git`, so it becomes the git directory of `<root>/platform`, and the repository wrote that directory's `config`.
   - `..`, newline and absolute paths are refused or trimmed (`under` at `gitlab.rs:1103`, `checked` at `repositories.rs:1726`). A leading `-` is cloned after `--`. Both shown green.
2. **Holds**, by reading.
   - The URL is built from validated steps and a validated host. The token reaches git only through the helper, which is scoped to `credential.https://<host>` (`pods.rs:143-155`).
   - A redirect to another host or to `http` does not match that key; not run.
   - `redacted` misses values under 4 characters and encoded forms, such as base64 of a Basic header echoed in a `remote:` line. Only the host the token belongs to ever receives it, so this is not reachable.
   - **Coordinator's question on `carries_token`:** a pod carrying only, say, `GITLAB_ACCESS_TOKEN` gets no helper. git then fails at once: `GIT_TERMINAL_PROMPT=0` (`mod.rs:535`), standard input closed (`mod.rs:651`), and no askpass is set in the crates or the Dockerfile. Not executed, because it needs an HTTP server.
3. **Does not hold** (the red case). The pass after the clone finds `<root>/platform` (`gitlab.rs:1185`) and reads `origin` from the repository's own config. It fetches there and runs `git status --porcelain` (`repositories.rs:1021`), which starts the repository's `core.fsmonitor` program.
   - I checked this with git 2.56 directly as well, with a relative program path, so the attacker does not need to know the root.
   - The same issue at one step (entry `.git`, which the config accepts) makes the checkouts root itself a repository.
   - Hooks, submodules, filters and `ext::` cannot be triggered by the clone alone: no `--recurse-submodules`, and git's defaults apply.
4. **Partly holds.** Trust sets `hasTrustDialogAccepted` in `~/.claude.json` (`trust.rs:1-3`). `refresh` (`checkouts.rs:475-485`) trusts every git directory one level under the root, so the fake one from invariant 1 too. Separately, see F2.
5. **Holds.** `covers` matches whole steps and is case-sensitive, like `exclude` (`mod.rs:327`). `relative` refuses `..` and absolute entries; empty lists and `include` beside `exclude` are refused (`config.rs:4375,4454`). Clones happen only for owners whose every source names `include`, and for forges with `include`.
   - The rule "left out only where every source on the root names `include`" is safe. Example: a gitlab source with `include: [alpha]` plus a github source without `include` on the same root lets checkout `beta` through the guard. That is exactly what the base already did for every checkout under the root, so it narrows nothing but widens nothing either.
6. **Partly bounded.**
   - Bounded: each clone (20 min), each fetch (`collect::BOUND`, two tries), parallel width, and the budget when one is given (shown green).
   - Not bounded: how many repositories are listed, repository size, and the start. `at_start` clones one after another with no overall limit (`checkouts.rs:424`), before seeding. Passes run without `--budget` (`snapshot.rs:154,170`).
   - The cost falls only on the pod of an instance whose host is hostile or slow: its own volume claim and its own loop.
   - Leftovers: a stopped or failed clone removes its temporary directory (`checkouts.rs:219,245`). Only a killed process leaves one, and it is removed at the next try of that same path.
7. **Holds.** The API thread starts before `seed_alone` (`instance.rs:480-504`), and both probes are `/healthz` (`pods.rs:415-435`).
8. **Holds.** `/v1/version` is answered after admission (`api.rs:1009-1053`) and carries the same two labels as `conductor_build_info`. GET `/v1/instances` checks no role either.
9. **Holds.** Only `pods.rs:332` sets `CONDUCTOR_API_LISTEN`. A workstation session that sets it would clone repositories its config already includes, with the user's own credentials, which it could do itself anyway.

**Findings**
- **F1** (blocker, CONFIRMED, introduced): `checkouts.rs:168`. Reached by a caller-chosen GitLab host plus a subgroup entry under `include`. Suggested fix: refuse a step equal to `.git` (ignoring ASCII case) or starting with `PARTIAL`, in `bring` and in `checked`. Arguably the config should also refuse such an `include` entry.
- **F2** (major, PLAUSIBLE, introduced): `checkouts.rs:475`. A checkout cloned from a host the caller chose is trusted, so its `.claude/settings.json` hooks run in any session started there. Not run: it depends on the pod spawning a session in that checkout.
- **F3** (minor, CONFIRMED, introduced): `checkouts.rs:424`. The start and the passes have no overall bound on clone count or time (invariant 6).
- **F4** (note, CONFIRMED, pre-existing, base `gitlab.rs:599`): `gitlab.rs:610`. One listed path with `..` or a newline fails the whole `repositories` collector for every source of the instance.
- **F5** (note, INFEASIBLE, introduced): `checkouts.rs:222`. A repository named `.conductor-clone-<x>` is deleted when `<x>` is next cloned. Only a caller-written `include` entry reaches this, and it deletes only the caller's own clone.

**Reviewed, no fault:** `userinfo_hidden` and `shapes_hidden` (no panics, ASCII slicing); `under`; deadline propagation in `each`; process-group kill; slot names cannot collide (`%` is not a name character); `refresh` errors are redacted; `Origins::of` union.

**Not checked:** redirect behaviour of git or curl and `remote:` control characters (needs a network); the pod's real git version; whether sessions spawn in a fresh checkout (F2); GitHub's own name rules (github.com is outside the threat model).

**Written outside the worktree** (the red case keeps its directory, with nested `.git`):
- `<the review scratch directory>/dotgit.sh`
- `<the review scratch directory>/probe/` (`dotgit/` fixture)
- `<the review scratch directory>/run-own.log`
- `<the review scratch directory>/gate.sh`
- `<the review scratch directory>/gate.log`
- `<the shared build target>/tmp/sec_pod_checkouts/dot-git-step/`

```findings
[
{"file":"crates/conductor/src/collect/checkouts.rs","line":168,"category":"integrity","severity":"blocker","verdict":"CONFIRMED","origin":"introduced","message":"a listed GitLab path step .git is cloned so its work tree becomes the git directory of the parent path, and the next pass runs git status there, which starts the repository's core.fsmonitor program in the pod"},
{"file":"crates/conductor/src/collect/checkouts.rs","line":475,"category":"judgement","severity":"warning","origin":"introduced","message":"refresh trusts every new checkout, including one cloned from a GitLab host the caller chose, which lets that repository's project settings hooks run in any session started there"},
{"file":"crates/conductor/src/collect/checkouts.rs","line":424,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the start clones one by one with no overall bound and passes run without a budget, so a hostile or slow listing holds the instance for the repository count times twenty minutes and fills its claim without a size limit"},
{"file":"crates/conductor/src/collect/gitlab.rs","line":610,"category":"integrity","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"one listed project path with a dot-dot step or a newline fails the whole repositories collector for every source of the instance"},
{"file":"crates/conductor/src/collect/checkouts.rs","line":222,"category":"boundary","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"a repository whose name starts with the temporary clone prefix is removed when the repository it shadows is next cloned, reachable only through an include entry the caller writes"}
]
```
