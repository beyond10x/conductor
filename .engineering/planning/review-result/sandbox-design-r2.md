---
format: aep.planning-md/3
id: review-result:sandbox-design-r2
kind: review-result
status: active
title: Design critic, round 2, sandbox
relations:
- reviews: story:sandbox-spec
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
needs-revision

Findings, one line each:

- story:sandbox-spec — it declares only the `session start` leaf, but the seam's acceptance has `conductor:restart` stop "through the provider". The trait's `stop` and `resume` have no command, and `session start` refuses when a session is already running, so it cannot be the restart. The spec-first story has to declare the stop verb, or the seam has to say what restart calls — .engineering/planning/story/sandbox-spec.md:52-53, .engineering/planning/story/session-envelope-seam.md:99
- story:session-writes-bwrap — it makes `available` run `bwrap --ro-bind / / true`, so a missing `bwrap` makes the provider unavailable. The seam says an unavailable provider under `report` starts the session unboxed, and under `strict` it is a "provider unavailable" refusal. The same body also says a missing `bwrap` under `report` starts the unit with limits and no fence, and under `strict` is a "shortfall" in `start`. The partial box that `story:sandbox-spec` declares is therefore unreachable, and the body does not say whether `available` covers bwrap or only systemd — .engineering/planning/story/session-writes-bwrap.md:60-63, .engineering/planning/story/session-envelope-seam.md:84-87
- story:session-envelope-seam — the request is described as "absolute paths and limits", but the forge story's `systemd.rs` passes each `roles[].forge_token_env` name as `--setenv`. The request type in `confine/mod.rs` is the seam's. The forge scope does not list `mod.rs`, so the seam must say the request carries the token names or the forge story must add `mod.rs` — .engineering/planning/story/session-envelope-seam.md:73-74, .engineering/planning/story/session-forge-credentials.md:48-50
- story:session-envelope-seam — the resolver returns "existing absolute paths", but the bwrap story tests a place path removed after resolution as a provider-side shortfall. The seam does not say whether a word whose directory is missing is dropped, passed through or refused by the resolver, and `story:sandbox-spec` declares no refusal for it. One of the two bodies has to own that case — .engineering/planning/story/session-envelope-seam.md:77-81, .engineering/planning/story/session-writes-bwrap.md:59-61

Round-1 findings, one by one (`#` is the order in `review-result:sandbox-design-r1`):

1. **Provider surface: no longer holds.**
   - The seam lists `available`, `start`, `prompt`, `stop`, `resume` and `measure` as the whole surface.
   - `start` returns the identity, and the systemd story implements each method (FIFO prompt, `systemctl --user stop`, a new unit for resume).
   - One new gap remains: the stop verb in finding 1 above.
2. **Resolver: no longer holds.**
   - `confine/places.rs` is now the seam's, and the request carries absolute paths.
   - The new edge in finding 4 above is about a missing directory, not about who owns the resolver.
3. **bwrap layer and report-mode partial box: holds in part.**
   - Fixed: bwrap is the systemd provider's write fence, with `bwrap.rs` named. The seam is no longer needed to define a layer. The partial box is declared in `story:sandbox-spec` with a `fence_missing` reason.
   - Still open: the revision made `available` include bwrap, so that partial box cannot be reached (finding 2 above).
4. **Bus and argument-list split: no longer holds.** The bwrap story owns the single ordered list, with `--tmpfs /run/user/<uid>` and `cc-socks` re-bound. The forge story only adds masks after the binds and `--setenv`. No `~` bind remains for the masks to fight.
5. **Forge scope on the systemd layer: holds in part.**
   - Fixed: `systemd.rs`, `doctor.rs` and `confine_systemd.rs` are now in the forge scope, and `--setenv` is named as the mechanism.
   - Still open: the request type that carries `forge_token_env` (finding 3 above).
6. **Edge to `story:gitlab-workspace`: no longer holds.**
   - The edge is gone, and the seam and `story:sandbox-spec` depend on `story:gitlab-guide`. Their Notes give the shared files as the reason.
   - `story:gitlab-guide` sits at wave 13 and `story:sandbox-spec` at wave 16 in `aep plan artifact waves`, so the sandbox epic is not held behind unscheduled drafts.
7. **Docs claim made true later: no longer holds.** The bwrap story now says the box bounds file writes and not forge or network side effects. The forge story owns `the-guard.md`, `limits.md` and `overview.md` and changes the claim there.

Splitting `story:sandbox-spec` from the seam is not a horizontal-slice finding. AGENTS.md rule 1 requires spec first, `story:session-envelope-model` already follows the same pattern, and each story leaves the tree green. No cycle exists, and I found no hidden dependency beyond the findings above.

**What I read:** 6 artifacts and `review-result:sandbox-design-r1` in full, with `story:gitlab-guide`, `story:version-flag`, `story:doctor-minimum-versions` and `story:session-envelope-model` read to check edge reasons. Commands: `aep plan artifact show`, `relations`, `graph`, `waves`, `validate` and grep on the story files. I walked all 180 non-review edges, including those outside the set, and found no cycle. I traced 12 acceptance reads to a producer in the set: 5 are recorded by a direct `depends_on` edge and 7 by the chain forge -> bwrap -> systemd -> seam -> `story:sandbox-spec`. `validate` reported only the 5 old stories closed on an assertion, which is not mine.

**What I could not establish:**
- Whether `confine/mod.rs` should hold the stop verb's handler or `session.rs` should. I judged only that no story declares the verb.
- Whether `spawn` and `watch` already have a `resume` caller; the bodies do not name one.

**Out of my lane:**
- **Scope:** `epic:session-confinement` says "the first part's six checks pass" but lists seven bullets in that part. Its acceptance names no check for `story:sandbox-spec`.
- **Acceptance:** `story:sandbox-spec` has no test that the stub handler's "not implemented" output matches the error it declares.
- **Acceptance (from round 1, not yet fixed):** the forge story's `--ro-bind /` and `~/.ssh` masks bound a `git push` over SSH, but the title says only read-only credentials.

```findings
- file: .engineering/planning/story/sandbox-spec.md
  line: 52
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the spec declares only the session start leaf, but the seam's Taskfile acceptance has conductor:restart stop through the provider, and the trait's stop and resume have no command; session start refuses when a session already runs, so it cannot be the restart; declare the stop verb here, or have the seam say what restart calls"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 62
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "available runs bwrap --ro-bind / / true, so a missing bwrap makes the provider unavailable (strict: provider-unavailable refusal; report: unboxed start per the seam), yet the same body has a missing bwrap start the unit without the fence under report and refuse as a shortfall under strict, so the partial box sandbox-spec declares is unreachable; say whether available covers bwrap or only systemd"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 73
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the request is described as absolute paths and limits, but the forge story's systemd.rs passes roles[].forge_token_env names as --setenv and its scope has no confine/mod.rs, so the request type that carries them belongs to neither body; state that the request carries the token names, or add mod.rs to the forge scope"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 77
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the resolver returns existing absolute paths while the bwrap story tests a place path removed as a provider-side shortfall, and sandbox-spec declares no refusal for a missing place; the seam does not say whether the resolver drops, passes through or refuses a word whose directory is missing"
```
