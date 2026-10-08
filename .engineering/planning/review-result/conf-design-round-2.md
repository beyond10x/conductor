---
format: aep.planning-md/3
id: review-result:conf-design-round-2
kind: review-result
status: active
title: Design critic, session-confinement, round 2
relations:
- reviews: story:session-envelope-seam
- reviews: story:session-writes-bwrap
- reviews: story:substrate-tool-server
revision: 1
---
needs-revision

- story:session-envelope-seam — nobody owns writing a started session's measurement into the observation `story:session-envelope-model` defines (round-1 design finding 1 moved, not closed). The model hands recording to "the providers' work" (`session-envelope-model.md:49-50`). The seam records only `unboxed` (`session-envelope-seam.md:47`). `session-limits-systemd` stops at "Its measurement is the cgroup path and the limits read back" (`session-limits-systemd.md:29`). `instance-usage-metrics` reads a cgroup path from "the envelope measurement" (`instance-usage-metrics.md:42-43`), so the real reader has no real writer. Add to the seam's acceptance that the start writes the provider's measurement where `snapshot sessions` reads it, with a fake-provider test — `.engineering/planning/story/session-envelope-seam.md:43-48`
- story:session-writes-bwrap — the observation field "the writable paths applied" (`session-envelope-model.md:46-47`) has no producer. No bwrap acceptance line records it, and no line says what `strict` does when `bwrap` is missing. bwrap is not a value of `confinement.provider: systemd | none`, yet "each provider supplies its own probe" (`session-envelope-seam.md:49`). The body should say bwrap is a layer of the `systemd` provider that adds `writable` to the measurement and counts as a shortfall under `strict` — `.engineering/planning/story/session-writes-bwrap.md:29-31`
- story:substrate-tool-server — `story:substrate-client` adds `substrate` to `confinement.provider` (`substrate-client.md:34`) but implements only connect, run and read usage, not the seam's `Provider` trait. This story starts the controller with `--tools` and `--mcp-config` but never says what `provider: substrate` starts, what measurement it returns, or where the harness's own cgroup and limits come from. The `systemd` provider owns those today, and one enum value cannot express "harness in a systemd unit, commands in substrate". The body should name the provider (or a composition of two) and its measurement — `.engineering/planning/story/substrate-tool-server.md:45-49`, `.engineering/planning/story/substrate-client.md:34-37`
- story:substrate-tool-server — it defines a second per-role set of "write subtrees" (`substrate-tool-server.md:39-40`). `session-envelope-model` already owns per-role `writable` paths (`session-envelope-model.md:39-41`), and says only that programs belong to the tool server (`session-envelope-model.md:42-45`). Two lists would decide where one session may write. The body should derive the subtrees from `writable` or say why they differ — `.engineering/planning/story/substrate-tool-server.md:39-40`

What I read: 21 artifacts. Set: the epic, all 14 stories that `decomposes` it, the ADR and the 3 blockers. Neighbours: `story:instance-session-names`, `story:instance-scoped-tasks`, `story:host-grants`, `story:spawn` and `story:guard-all-tools`. Also the round-1 results (design, parallel), the new § 5 of the benefits page, and `git show a03e0ee7d --stat`. Commands: `aep plan artifact show`, `relations`, `graph`, `validate` (valid) and `git show`. `aep plan artifact findings` needs an `<ID>` argument, so I read the review-result file directly.

Edges and reads: I walked about 50 edges, including outside the set. They reach `spawn`, `guard-all-tools`, `host-grants`, `instance-session-names`, `instance-scoped-tasks`, `install-prerequisites`, `instance-cgroup-limits` and `guard-bash-write-targets`. No cycle. The chain `names → tasks → grants → model → seam → systemd → bwrap, usage → client → gate, tool-server` is serial, but every edge carries a stated reason (shared `config.yaml`, `config.rs`, generated crate, `doctor.rs`, `usage_metrics.rs`, `Taskfile.yml`, or a read of produced state). I did not ask for any to be removed. About 23 acceptance reads traced to a producer in the set; 20 have a recorded edge and the 3 above do not.

Round-1 status:
- Round-1 design findings 2, 3 and 4 are closed. The seam story exists, the session provider is archived, and `substrate-client` holds the shared client.
- Round-1 design finding 1 is half closed. The acceptance now uses a fixture measurement, but recording the live one is still unowned (first finding).
- The round-1 unease about the stale `instance-cgroup-limits` reference in `instance-usage-metrics` is fixed.

What I could not establish:
- Out of lane (parallel safety): `substrate-gate-provider` and `substrate-tool-server` both depend on `substrate-client` and both edit `crates/conductor/src/confine/substrate.rs` and `crates/conductor/cli.yaml`, with no edge between them. `substrate-client` is also edge-ordered after `instance-usage-metrics` only for `usage_metrics.rs`. That is a shared-file trade-off, not a defect.
- Out of lane (scope): the epic's "still answers SendMessage and appears in `claude agents --json`" is checked in `session-limits-systemd` and `session-writes-bwrap`, but not in the epic's tool-route line.
- Whether the harness itself must also sit in a systemd unit under the tool route is a design decision I cannot read from the benefits page (§ 5 covers only the tools).

```findings
- file: .engineering/planning/story/session-envelope-seam.md
  line: 43
  category: design
  severity: blocker
  verdict: needs-revision
  origin: pre-existing
  message: "nobody owns writing a started session's measurement into the observation story:session-envelope-model defines: the model hands recording to the providers, the seam records only unboxed, session-limits-systemd stops at 'its measurement is the cgroup path and the limits read back', and instance-usage-metrics reads a cgroup path from that measurement; the seam's acceptance should have the start write the provider's measurement where snapshot sessions reads it, with a fake-provider test"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 29
  category: design
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "the observation field 'the writable paths applied' has no producer: no acceptance line records it and none says what strict does when bwrap is missing, though bwrap is not a confinement.provider value and each provider supplies its own probe; the body should say bwrap is a layer of the systemd provider that adds writable to the measurement and counts as a shortfall under strict"
- file: .engineering/planning/story/substrate-tool-server.md
  line: 45
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "story:substrate-client adds substrate to confinement.provider without implementing the seam's Provider trait, and this story starts the controller with --tools and --mcp-config without saying what provider substrate starts, what measurement it returns, or where the harness's own cgroup and limits come from; the body should name the provider or a composition of two and its measurement"
- file: .engineering/planning/story/substrate-tool-server.md
  line: 39
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it defines a second per-role set of write subtrees while story:session-envelope-model already owns per-role writable paths and defers only programs to this story, so two lists decide where one session may write; derive the subtrees from writable or say why they differ"
```
