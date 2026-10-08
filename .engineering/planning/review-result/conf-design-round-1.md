---
format: aep.planning-md/3
id: review-result:conf-design-round-1
kind: review-result
status: active
title: Design critic, session-confinement, round 1
relations:
- reviews: epic:session-confinement
- reviews: story:session-envelope-model
- reviews: story:substrate-session-provider
revision: 1
---
needs-revision

- story:session-envelope-model — its acceptance has it record and show a measurement that only story:session-limits-systemd's provider can produce ("The measurement is recorded"), and the edge between the two already points the other way, so the item cannot be demonstrated alone; the body should say the snapshot test uses a fixture measurement and that recording belongs to session-limits-systemd — .engineering/planning/story/session-envelope-model.md (Acceptance, "a measurement recorded and shown by `conductor snapshot sessions`"), .engineering/planning/story/session-limits-systemd.md (Acceptance, "The measurement is recorded"), `aep plan artifact graph`
- story:session-envelope-model — nothing in the set owns the provider seam and strict gate that the ADR and the metaharness pattern call for: a provider trait, a start step that compares request with measurement, strict refusal, and report-as-unboxed. The model story carries only the data and limits-systemd carries one implementation. Either this body takes the seam (a `confine` module trait plus start-in-envelope), or a seam item is added that both substrate stories depend on. The epic's "strict and no provider → `conductor:start` refuses" and "selectable per instance" then have an owner — docs/analysis (ADR `architecture-decision-record:external-confinement`, "Consequences": "a request …, a provider that starts the harness in it, and measurements at the boundary"); ~/beyond10x/metaharness/crates/metaharness/src/process.rs:99-193 (`ProcessEnvelope`, `start_in_envelope`)
- story:substrate-session-provider — its acceptance reads the `confinement` config and envelope request that story:session-envelope-model creates and the seam and `confine` module that story:session-limits-systemd creates, and "usage" that story:instance-usage-metrics shows; the only edge is the upstream blocker. Record `depends_on` story:session-envelope-model (and the seam item or story:session-limits-systemd) — `aep plan artifact graph` (no `depends_on` from story:substrate-session-provider), .engineering/planning/story/substrate-session-provider.md (Acceptance, "the provider is selectable per instance")
- story:substrate-session-provider — it and story:substrate-gate-provider both claim `crates/conductor/src/substrate.rs`, and only the gate story adds `Cargo.toml` / `b10x-substrate-sdk`. So the session provider builds on a client the gate story creates, behind a different blocker. Either record `depends_on` story:substrate-gate-provider (which ties sessions to the exec-in-checkout blocker), or split the shared substrate client into its own item both depend on (preferred, since the two blockers are independent) — .engineering/planning/story/substrate-gate-provider.md (Scope: `Cargo.toml`, `substrate.rs`), .engineering/planning/story/substrate-session-provider.md (Scope: `substrate.rs`)

What I read: 14 artifacts (the 11 stories, epic, ADR and 3 blockers, plus the `session-confinement` and `multi-instance-host` neighbours) with `aep plan artifact show`, `relations`, `graph` and `validate` (validate: valid). I also read critic-rubric.md and process.rs.

Edges walked: about 35, including outside the set (`instance-session-names`, `host-grants`, `install-prerequisites`, `instance-cgroup-limits`, `bash-sandbox`). No cycle found. The only chain is spike and model → limits-systemd → bwrap and usage-metrics; the bwrap edge records a shared module (`confine.rs`), which is a trade-off, not a defect.

Acceptance reads traced to a producer in the set: 9. Recorded by an edge: 5 (limits → envelope-model, limits → box-spike, bwrap → limits, usage-metrics → limits, storage-quota → host-grants). Not recorded: 4 (the two above, plus substrate-session-provider's reads of the envelope request and of usage).

What I could not establish:
- Out of lane (scope): no story's acceptance covers the epic line "still answers SendMessage and appears in `claude agents --json`" except the spike. The epic's outcome and acceptance do not mention storage, yet story:instance-storage-quota and story:quota-support-report decompose it. Three children are blocked by open external items, so the epic cannot close.
- Out of lane (parallel safety): `doctor.rs` and `config.yaml` are shared between envelope-model, quota-support-report, usage-metrics and storage-quota.
- Unease, not a finding: story:instance-usage-metrics' Why still cites the archived `instance-cgroup-limits`, which story:session-limits-systemd supersedes.

```findings
- file: .engineering/planning/story/session-envelope-model.md
  line: 24
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance records and shows a measurement that only story:session-limits-systemd's provider produces ('The measurement is recorded'), and the edge between them points the other way, so the item cannot be demonstrated alone; the body should say the snapshot test uses a fixture measurement and that recording belongs to session-limits-systemd"
- file: .engineering/planning/story/session-envelope-model.md
  line: 1
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "no item in the set owns the provider seam and strict gate that the ADR and metaharness process.rs:99-193 call for (provider trait, start-in-envelope comparing request with measurement, strict refusal, report-as-unboxed); the model story holds only data and limits-systemd one implementation, so this body should take the seam or a seam item should be added that both substrate stories depend on"
- file: .engineering/planning/story/substrate-session-provider.md
  line: 1
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its acceptance reads the confinement config and envelope request that story:session-envelope-model creates, the seam and confine module that story:session-limits-systemd creates, and usage from story:instance-usage-metrics, and no depends_on edge records any of them; add depends_on to story:session-envelope-model and the seam item or story:session-limits-systemd"
- file: .engineering/planning/story/substrate-session-provider.md
  line: 1
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it and story:substrate-gate-provider both claim crates/conductor/src/substrate.rs and only the gate story adds Cargo.toml and b10x-substrate-sdk, so the session provider builds on a client created behind a different blocker; split the shared substrate client into its own item both depend on, or record depends_on story:substrate-gate-provider"
```
