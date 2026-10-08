---
format: aep.planning-md/3
id: review-result:conf-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, session-confinement, round 2
relations:
- reviews: epic:session-confinement
- reviews: story:instance-usage-metrics
- reviews: story:session-envelope-seam
- reviews: story:substrate-client
- reviews: story:substrate-gate-provider
- reviews: story:substrate-tool-server
- reviews: upstream-blocker:substrate-exec-in-checkout
revision: 1
---
needs-revision

- epic:session-confinement — [introduced] the closing paragraph ("close the 'later' line … this epic can complete its first part without them") names no outcome or check for the four substrate stories and never says when the epic is done. It also says "two of them wait on upstream blockers", but `aep plan artifact blocked` shows only `story:substrate-gate-provider` is blocked by a live upstream blocker. — .engineering/planning/epic/session-confinement.md:41-43
- epic:session-confinement — [introduced] the Outcome promises that conductor, conductor-dev and "the workers they spawn" run boxed, but every acceptance bullet observes "a controller session". Nothing checks a conductor session or a spawned worker. — .engineering/planning/epic/session-confinement.md:13-14 against :30-37
- story:instance-usage-metrics — [introduced] the title promises "The dashboard and the watch show CPU, memory and io", but the revision removed the watch outcome and line 46 says "Not here: a threshold line in the watch". No watch outcome remains for the title's claim to be checked against. — .engineering/planning/story/instance-usage-metrics.md:6 and :46
- story:instance-usage-metrics — [carried, round 1 finding 3 partly fixed] the render test with fixture readings (line 44) now checks the display. The epic bullet reads "With a test instance on this host … the dashboard shows that session's CPU time and memory", and no recorded run shows real values for a live session. Whether `claude agents --json` carries the pid line 31 relies on is unestablished. — .engineering/planning/story/instance-usage-metrics.md:44 (epic .engineering/planning/epic/session-confinement.md:35)
- story:session-envelope-seam — [introduced] the bullet joins two commands, `session start --role` and `controller start-controller`, as independent outcomes. The "tests with a fake provider, one per case" do not say which command they run through, so no check decides that both go through the provider. — .engineering/planning/story/session-envelope-seam.md:40-43
- story:session-envelope-seam — [introduced] "The `.agents/conductor.md` diff goes to the operator before the merge" is a process step with no recorded artifact. Nobody can observe afterwards whether it happened. — .engineering/planning/story/session-envelope-seam.md:52-53
- story:substrate-client — [introduced] "connect, run one exec, read its usage record" has no check. "Tests with a fake substrate" is attached only to the doctor-probe bullet, and the `substrate` provider value and the `Cargo.toml` dependency name no check either. — .engineering/planning/story/substrate-client.md:34-38
- story:substrate-gate-provider — [carried, round 1 finding 2 partly fixed] the refusal outcome is now its own test bullet, and the command is named. The recorded-run bullet still joins two independent outcomes: "exits as the plain run does, and its usage … is recorded in the observation store". — .engineering/planning/story/substrate-gate-provider.md:29-31
- story:substrate-tool-server — [introduced] "The guard sees the `mcp__conductor__*` calls" names no test or recorded observation. — .engineering/planning/story/substrate-tool-server.md:50
- story:substrate-tool-server — [introduced] "keeps SendMessage and Agent … and loses Bash, Write and Edit" has no check. The recorded-run list (lines 47-49) never shows the session's tool list or that Bash is absent. — .engineering/planning/story/substrate-tool-server.md:45-49
- upstream-blocker:substrate-exec-in-checkout — [carried, round 1 raised no finding] its clearing statement joins two independent conditions: an exec in an adopted directory, and more than one core per exec. The only dependent story's acceptance needs the first (a `cargo test` run in a managed tree), so nobody can tell when the story unblocks. — .engineering/planning/upstream-blocker/substrate-exec-in-checkout.md:13-15

What I read: 18 artifacts, all via `aep plan artifact show` (epic, 13 `decomposes` stories found with `aep plan artifact graph`, the ADR, the decision blocker and both upstream blockers). I also read round 1 and its `a03e0ee7d` diff, § 5.4 of the benefits page, `aep plan artifact blocked`, and the tree for `session start` and `start-controller` in `cli.yaml`.

What I could not establish:
- Whether `claude agents --json` entries carry a pid. The two analysis pages do not mention it, and I did not run the command.
- Round-1 findings I judged fixed: `story:session-envelope-model` (the doctor bullet moved to the seam story), `story:session-writes-bwrap` (network and harness state now in the recorded run), `story:quota-support-report` (it names `prjquota` and `pquota`), and `decision-blocker:project-quota-filesystem` ("What clears it" is present).
- `story:substrate-session-provider` is archived, so its round-1 blocker lapses with the retirement.
- Out of my lane, and they did not set the verdict:
  - Scope: `upstream-blocker:substrate-long-sessions` now blocks only an archived story.
  - Design: `story:substrate-client` is ordered after `story:instance-usage-metrics`.
- `origin`: the findings block accepts only `introduced`, `pre-existing` or `undecided`. I tagged round-1 carry-overs `carried` in the lines above and recorded them as `introduced` in the block, since the drafted set created them.

```findings
- file: .engineering/planning/epic/session-confinement.md
  line: 41
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the closing paragraph names no observable outcome or check for the four substrate stories and never says when the epic is done; it also says two of them wait on upstream blockers when only story:substrate-gate-provider is blocked"
- file: .engineering/planning/epic/session-confinement.md
  line: 13
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Outcome covers conductor, conductor-dev and the workers controllers spawn, but every acceptance bullet observes only a controller session, so nothing checks the others"
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 6
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the title promises that the dashboard and the watch show usage, but the acceptance excludes the watch ('Not here: a threshold line in the watch'), so no watch outcome exists to check"
- file: .engineering/planning/story/instance-usage-metrics.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "carried from round 1: the epic bullet is checked on a test instance on this host, but the story's only check is a fixture render test, so no run shows a real session's values and the pid from claude agents --json is unproven"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 40
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the bullet joins session start --role and controller start-controller as two outcomes, and the per-case fake-provider tests do not say which command they run through"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 52
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'The .agents/conductor.md diff goes to the operator before the merge' is a process step with no recorded artifact, so nobody can observe afterwards that it happened"
- file: .engineering/planning/story/substrate-client.md
  line: 36
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'connect, run one exec, read its usage record' names no check; 'Tests with a fake substrate' belongs to the doctor-probe bullet only, and the substrate provider value and Cargo.toml dependency bullets name no check either"
- file: .engineering/planning/story/substrate-gate-provider.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "carried from round 1: the recorded-run bullet still joins two independent outcomes, 'exits as the plain run does, and its usage is recorded in the observation store'"
- file: .engineering/planning/story/substrate-tool-server.md
  line: 50
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'The guard sees the mcp__conductor__* calls' names no test or recorded observation"
- file: .engineering/planning/story/substrate-tool-server.md
  line: 45
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'keeps SendMessage and Agent among its built-ins and loses Bash, Write and Edit' has no check; the recorded run never shows the session's tool list or that Bash is absent"
- file: .engineering/planning/upstream-blocker/substrate-exec-in-checkout.md
  line: 13
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "carried from round 1: the clearing statement joins two independent conditions (exec in an adopted directory, more than one core per exec) while the only dependent story needs the first, so nobody can tell when the story unblocks"
```
