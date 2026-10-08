---
format: aep.planning-md/3
id: review-result:conf-parallel-round-2
kind: review-result
status: active
title: Parallel-safety critic, session-confinement, round 2
relations:
- reviews: story:instance-storage-quota
- reviews: story:substrate-tool-server
- reviews: story:session-limits-systemd
- reviews: story:session-box-spike
- reviews: story:spawn
- reviews: story:guard-all-tools
revision: 1
---
needs-revision

Findings (`artifact — reason — citation`). Origin is marked `introduced` or `carried` against round 1. Paths are under `.engineering/planning/story/`.

1. story:instance-storage-quota — it shares `crates/conductor-model` with `story:instance-usage-metrics`. Neither body names the other, and no edge or chain runs between them, so `waves` separates them only by tie-break (storage-quota in wave 7, usage-metrics in wave 8). Remedies: an ordering edge that records the generated crate as its reason, or splitting the surface. (cited; carried from round 1 finding 4) — instance-storage-quota.md:44-47; instance-usage-metrics.md:51-54
2. story:instance-storage-quota — it shares `spec/domains/config.yaml` and `crates/conductor-model` with `story:substrate-client` and with `story:substrate-tool-server`. Its "after" sentence names only `host-grants` and `quota-support-report`, which lead to `session-envelope-model` and `session-envelope-seam`, and no path leads to either substrate story. Remedies: ordering edges, or splitting the surface. (cited; introduced, both substrate stories are new in this revision) — instance-storage-quota.md:44-47; substrate-client.md:44-47; substrate-tool-server.md:54-57
3. story:substrate-tool-server — it shares `crates/conductor/src/confine/substrate.rs` and `crates/conductor/cli.yaml` with `story:substrate-gate-provider`. Both depend on `substrate-client`, there is no edge between them, and neither body names the other. Remedies: an ordering edge recording both files, or splitting the surface. (cited; carried from round 1 finding 9, the substrate.rs collision, which this story inherits from `substrate-session-provider`) — substrate-tool-server.md:54-57; substrate-gate-provider.md:37-39
4. story:session-limits-systemd — its typed scope is `confine/systemd.rs` alone, but its body Scope also lists `crates/conductor/tests/` and `docs/analysis/`. `waves` therefore cannot see that directory-level `tests/` includes `tests/doctor.rs`, which `quota-support-report` owns in the same wave 6 with no edge between the two. Its acceptance puts the probe test in `tests/`. Remedies: list the exact test files in both, or add an ordering edge. (the `tests/doctor.rs` overlap is inferred; the typed/body mismatch is cited; introduced) — session-limits-systemd.md:13-15,38-39; quota-support-report.md:32-35
5. story:session-box-spike — its scope `docs/analysis` is a directory shared with `story:substrate-tool-spike`, and `waves` reports a collision and separates them only by tie-break. Each spike writes its own new page and neither names it, so the collision may be only apparent. Naming each page file in the typed scope and the body would clear it. (cited; introduced) — session-box-spike.md:32-34; substrate-tool-spike.md:33-35
6. story:spawn — it is unassessed (no typed scope and no Scope section) and its step 3 starts `claude --bg -n <repo>`. Its acceptance reads the session name and the `conductor` recipient that `instance-session-names` changes to `<prefix>-<repo>`, so it reads state that story produces. No `depends_on` runs from `spawn` to `instance-session-names`, and `spawn.rs` is not in that story's scope. The missing edge, or `spawn.rs` added to that scope, would fix it. (reader beside producer, cited; carried, round 1 read `spawn` and did not flag it) — spawn.md:19,53-58; instance-session-names.md:47-49,64-68
7. story:guard-all-tools — it has no typed scope (`waves` lists it as unassessed), and its body Scope names `crates/conductor/src/guard/rules.rs` and `spec/domains/config.yaml`. `instance-session-names` (wave 1) changes both, and no edge orders them. Remedies: a typed scope plus an ordering edge, or splitting the surface. (cited; carried, round 1 did not list it) — guard-all-tools.md:26-29; instance-session-names.md:64-68

Round 1 findings that no longer hold:
- Findings 1, 3, 4 (except the pair in finding 1 above), 5, 6, 7, 8, 10 and 11 are fixed. I computed `depends_on` reachability over every collision line `waves` printed, and all but the five pairs above are ordered by a direct edge or a chain.
- Finding 9 (the `substrate.rs` collision between the two providers) is fixed for `substrate-client`, but its successor `substrate-tool-server` still collides with `substrate-gate-provider` (finding 3).

**What I read:** 18 artifacts with `aep plan artifact show`: the 13 stories that decompose the epic (`substrate-session-provider` excluded as archived, so 12 read), plus `instance-session-names`, `instance-scoped-tasks`, `host-grants`, `story:spawn` and `story:guard-all-tools`. I also read `epic:session-confinement`, `review-result:conf-parallel-round-1`, and `waves --kind story --status draft`. `graph` supplied the edges, and I ran the reachability check over the `waves` collision lines and the `depends_on` edges. Surfaces: 17 cited (16 live set stories plus `guard-all-tools`), 0 inferred, 0 unplaceable. `spawn` is placed only on `spawn.rs` and its start line, with no Scope section. Acceptance reads traced to a producer in the set: 26. Twenty-one are recorded by a direct edge, 4 by a chain (bwrap's `writable` read, client's provider read, gate's usage record, storage-quota's config), and 1 is unrecorded (finding 6).

**What I could not establish:**
- **`confine/mod.rs` registration:** whether `session-limits-systemd`, `session-writes-bwrap` and `substrate-client` each add a provider arm and `pub mod` to `crates/conductor/src/confine/mod.rs` (the seam creates it, and none of them lists it). `bwrap` and `client` are unordered against each other, so this is unplaced.
- **`lib.rs` module lines:** `host.rs`, `quota.rs`, `gate.rs` and `tools_serve.rs` each need a `pub mod` line in `crates/conductor/src/lib.rs`. Only the seam and `usage-metrics` list it. I treated this as a trivial merge, as round 1 did.
- **Other typed-scope gaps:** `crates/conductor/tests/` and `docs/analysis/` are in the body Scope but not the typed scope of `session-writes-bwrap`, `session-envelope-seam`, `session-envelope-model`, `substrate-client`, `substrate-gate-provider` and `substrate-tool-server`. I found no further unordered pair from them. `host-grants` is the reverse: its typed scope has `config.rs`, which its body Scope omits (it is ordered, so harmless).
- **Out of my lane:** whether `session-writes-bwrap` has a config switch for "the provider on" (design or acceptance). `docs/config.md` is hand-written and listed by only two stories (round 1 already noted it). `substrate-gate-provider` records usage in the observation store without naming a spec surface for it (scope).

```findings
- file: .engineering/planning/story/instance-storage-quota.md
  line: 44
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "instance-storage-quota and instance-usage-metrics both change crates/conductor-model (the generated crate), neither body names the other, and no edge or chain orders them, so waves separates them only by tie-break. Remedies: an ordering edge recording the generated crate, or split the surface."
- file: .engineering/planning/story/instance-storage-quota.md
  line: 44
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "instance-storage-quota shares spec/domains/config.yaml and crates/conductor-model with substrate-client and substrate-tool-server; its after-sentence reaches only host-grants and quota-support-report, and no edge or chain connects it to either substrate story. Remedies: ordering edges recording the shared files, or split the surface."
- file: .engineering/planning/story/substrate-tool-server.md
  line: 54
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "substrate-tool-server and substrate-gate-provider both change crates/conductor/src/confine/substrate.rs and crates/conductor/cli.yaml, both depend on substrate-client, and no edge or sentence orders them. Remedies: an ordering edge recording both files, or split the surface."
- file: .engineering/planning/story/session-limits-systemd.md
  line: 38
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "typed scope is confine/systemd.rs only while the body Scope also lists crates/conductor/tests/ and docs/analysis/, so waves cannot see that directory-level tests/ includes tests/doctor.rs, which quota-support-report owns in the same wave with no edge; the doctor.rs test overlap is inferred. Remedies: list the exact test files in both scopes, or add an ordering edge."
- file: .engineering/planning/story/session-box-spike.md
  line: 32
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the typed scope docs/analysis is a directory shared with substrate-tool-spike, so waves reports a collision and separates the two by tie-break; each spike writes its own new page and neither body names the file. Name each page path in the typed scope and the body, or add an ordering edge."
- file: .engineering/planning/story/spawn.md
  line: 19
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "spawn has no typed scope or Scope section, starts the session as claude --bg -n <repo>, and its acceptance reads the session name that instance-session-names renames to <prefix>-<repo>; no depends_on runs from spawn to instance-session-names and spawn.rs is not in that story's scope. Add the edge, or name spawn.rs in the names story's scope."
- file: .engineering/planning/story/guard-all-tools.md
  line: 28
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "guard-all-tools has no typed scope (waves lists it unassessed) and its body Scope names guard/rules.rs and spec/domains/config.yaml, both also changed by instance-session-names, with no edge between them; round 1 did not list it. Add a typed scope and an ordering edge, or split the surface."
```
