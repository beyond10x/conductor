---
format: aep.planning-md/3
id: review-result:onboarding-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, onboarding
relations:
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
revision: 1
---
needs-revision

story:setup-guide-for-agents — the acceptance promises steps "safe to repeat" and that "Each step names what needs the person's hands", but the named test only reads commands, the README pointer and command existence, so neither claim has a check — .engineering/planning/story/setup-guide-for-agents.md:35

story:doctor-identity — "on each host it knows" and "the host an instance's source names" do not say where doctor gets its hosts, and `crates/conductor/src/doctor.rs` has no host concept today (no `host` match), so the acceptance can't tell which hosts must show a row — .engineering/planning/story/doctor-identity.md:39

story:onboarding-run — the recorded end state may be the `conductor doctor` output alone ("or `conductor setup`"), and doctor does not read the planning store's version, so `ready` is the same for a store still one format old as for a migrated one; the record must show the store version before and after — .engineering/planning/story/onboarding-run.md:31

story:snapshot-duration — the view is left open ("`conductor snapshot snapshots` (or a `snapshot collectors` view)"), so the surface the test reads is not named — .engineering/planning/story/snapshot-duration.md:38

story:snapshot-duration — "a recorded member failure" borrows the term from draft `story:snapshot-member-failures`, whose observation records a member read by the blockers or specifications collector, not a whole collector that passes a budget, so what `--budget` records is not named — .engineering/planning/story/snapshot-duration.md:43

**What I read:** 5 of 5 ids (story:setup-guide-for-agents, story:doctor-identity, story:setup-command, story:onboarding-run, story:snapshot-duration), with `aep plan artifact show` on each. I also read epic:onboarding-from-url, `b10x.toml`, `doctor.rs`, the draft `story:snapshot-member-failures`, and `observation.yaml` and `cli.yaml` by grep. I did not run `aep plan artifact kinds` or `lifecycle`.

**What I could not establish:**
- story:setup-command raised no finding; its rows, states and check names are left to the spec-first bullet.
- Whether `crates/conductor/cli.yaml` already has a `[setup]` entry (it does not match `snapshots` or `start-snapshot` either); I took both as new.
- Out of my lane, not counted in the verdict: `SETUP.md` is in story:setup-command's scope but appears in none of its acceptance lines (scope critic), and snapshot-duration's `depends_on story:setup-command` has no apparent coupling (design or parallel-safety critic).

```findings
- file: .engineering/planning/story/setup-guide-for-agents.md
  line: 35
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance promises steps 'safe to repeat' and that 'Each step names what needs the person's hands', but the named test reads only commands, the README pointer and command existence, so neither claim has a check"
- file: .engineering/planning/story/doctor-identity.md
  line: 39
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'on each host it knows' and 'the host an instance's source names' do not say where doctor gets its hosts, and doctor.rs has no host concept today, so the acceptance cannot tell which hosts must show a row"
- file: .engineering/planning/story/onboarding-run.md
  line: 31
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the recorded end state may be the conductor doctor output alone ('or conductor setup'), and doctor does not read the planning store's version, so ready is the same for an old store as for a migrated one; the record must show the store version before and after"
- file: .engineering/planning/story/snapshot-duration.md
  line: 38
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the view is left open ('conductor snapshot snapshots (or a snapshot collectors view)'), so the surface the test reads is not named"
- file: .engineering/planning/story/snapshot-duration.md
  line: 43
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "'a recorded member failure' borrows the term from draft story:snapshot-member-failures, whose observation records a member read by the blockers or specifications collector, not a whole collector that passes a budget, so what --budget records is not named"
```
