---
format: aep.planning-md/3
id: review-result:onboarding-design-r1
kind: review-result
status: active
title: Design critic, round 1, onboarding
relations:
- reviews: story:sandbox-spec
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
revision: 1
---
needs-revision

Findings, one line each:

- story:setup-command — its rows re-state what `doctor` already checks (programs against pins, forge CLI present, forge login) and its `ready` is a second verdict that nothing ties to doctor's, while the epic and SETUP.md end on doctor's `ready`; every doctor row the sandbox wave adds (provider availability, quota, fence, forge-token lines) would then sit outside setup's `ready`. The body should say setup's program and forge rows are doctor's own rows (`examine`, `merge_pins`, `verdict` in `doctor.rs`) and that it ends on doctor's verdict, adding only the repository rows. Those are store version, config names the repository, records, and trusted. — .engineering/planning/story/setup-command.md:43-46, crates/conductor/src/doctor.rs:312,457,511
- story:setup-command — its Notes put it "After … the sandbox wave (doctor.rs, cli.yaml, the model)", but sandbox-spec now depends_on onboarding-run, which depends_on setup-command, so the recorded edges put setup first. Following the note would close the walk sandbox-spec → onboarding-run → setup-command → sandbox wave → sandbox-spec. The Notes must drop the sandbox clause, or the edges must change. — .engineering/planning/story/setup-command.md:56, `aep plan artifact graph`
- story:sandbox-spec — its two new depends_on edges (onboarding-run, snapshot-duration) have no reason beside them, and the Notes still say "After wave 17" while both land in wave 18. The only shared surface I can find is the generated `crates/conductor-model`, `cli.yaml`, `cli.rs` and `reference/cli.md`, which setup-command also scopes. The choice is to keep the edges and name those files, or point the edge at setup-command and drop the live-session onboarding-run gate, which needs a person for logins and the migration, from the sandbox's critical path. — .engineering/planning/story/sandbox-spec.md:12-13,77
- story:snapshot-duration — it depends_on setup-command with no reason in its body. Nothing it reads or writes comes from setup-command: its acceptance reads snapshot records and `docs/analysis/snapshot-duration.md`. The edge can only record the shared generated model and `reference/cli.md`. The body should say so; the alternative is to drop the edge and let sandbox-spec wait on setup-command alone. — .engineering/planning/story/snapshot-duration.md:9

Verdicts on the questions asked:
- **Does setup duplicate doctor:** it does, as drafted. The repository rows (store version, instance config naming the repository, records, trust) are new and belong in setup. The program, pin and forge-login rows and the verdict are doctor's.
- **Cycles:** none in the declared edges. `depends_on` means needs first. The only cycle risk is the Notes contradiction in the second finding.
- **Set shape:** setup-guide → setup-command → onboarding-run is a chain, but each body gives a reason (shared SETUP.md, the output read). I do not flag it as serialising.
- **Hidden dependencies:** none unrecorded among acceptance reads. setup-command reads SETUP.md and the doctor forge identity, and both have edges. onboarding-run reads setup's output and SETUP.md, with the second only transitively. onboarding-run's edge to setup-command is recorded.

What I read: 6 artifacts in full (epic and 5 stories) plus sandbox-spec, session-envelope-seam and the grepped session stories. Commands: `aep plan artifact relations`, `graph`, `validate` (valid), `waves`, and `show` on each id. I also read `doctor.rs`, the heads of `init.rs` and `trust.rs`, the doctor, init and trust entries in `cli.yaml` and `spec/domains/config.yaml`, and `b10x.toml`.

Edges and acceptance reads:
- I walked every graph edge touching the set (about 40, inside and outside it), including sandbox-spec, session-envelope-seam and the downstream session stories.
- I traced 4 acceptance reads to a producer in the set; all 4 have an edge, one only transitively.

What I could not establish, all outside my lane and not counted in the verdict:
- Which pins `conductor setup` compares against in a repository with no `b10x.toml`. Doctor falls back to the build's pins; SETUP.md says "conductor's `b10x.toml`". (acceptance)
- setup-command lists SETUP.md in its scope but has no acceptance line saying what changes in it. (acceptance)
- onboarding-run's fixes to `conductor setup` edit `setup.rs`, `cli.yaml` and the model, none of which are in its scope. (parallel-safety/scope)
- doctor-identity is not under the epic, though the epic's "logged in" outcome and setup-command depend on it. (scope)
- onboarding-run installs conductor from a release archive, so it needs a release that contains `setup`; no item cuts that release. (outside the set)
- `aep plan artifact waves` puts doctor-identity in wave 15 although its only edge is to doctor-minimum-versions (wave 5). I did not trace why.

```findings
- file: .engineering/planning/story/setup-command.md
  line: 43
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its rows re-state doctor's program, pin and forge-login checks and its `ready` is a second verdict not tied to doctor's, so doctor rows the sandbox wave adds fall outside it; the body should say setup's program and forge rows are doctor's own rows (examine, merge_pins, verdict in doctor.rs), end on doctor's verdict, and add only the repository rows"
- file: .engineering/planning/story/setup-command.md
  line: 56
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "its Notes put it after the sandbox wave (doctor.rs, cli.yaml, the model) while sandbox-spec depends_on onboarding-run which depends_on setup-command, so the note and the recorded edges give opposite orders and following the note closes a cycle; drop the sandbox clause or change the edges"
- file: .engineering/planning/story/sandbox-spec.md
  line: 12
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "its new depends_on edges to onboarding-run and snapshot-duration have no reason written beside them and the Notes still say After wave 17; either name the shared files (crates/conductor-model, cli.yaml, cli.rs, reference/cli.md) beside the edges or point the edge at setup-command and drop the live-session onboarding-run gate from the sandbox's path"
- file: .engineering/planning/story/snapshot-duration.md
  line: 9
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "it depends_on setup-command though nothing its acceptance reads or writes comes from setup-command, and no sentence gives the reason; the body should say the edge records the shared generated model and reference/cli.md, or the edge should go"
```
