---
format: aep.planning-md/3
id: review-result:onboarding-scope-r2
kind: review-result
status: active
title: Scope critic, round 2, onboarding
relations:
- reviews: epic:onboarding-from-url
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
- reviews: story:spec-first-wording
revision: 1
---
needs-revision

The round-1 finding no longer holds. A different gap in the same bullet does.

**Round 1.** The finding said no sentence in the epic asks for a release gate. The epic now does, at `.engineering/planning/epic/onboarding-from-url.md:22-23`: "it stays that way: two recorded runs prove it and are repeated before a release that changes it". The bullet at `.engineering/planning/story/onboarding-run.md:55` traces to that sentence and cites it. `AGENTS.md` in scope is now justified. I am not repeating the finding.

**New finding.** The epic promises the repeat before a release that changes onboarding. Onboarding ends with "`conductor doctor` ending `ready`" (epic line 21). The story narrows the trigger to a release that changes `SETUP.md`, `conductor setup` or the prerequisites. A release that changes only `conductor doctor` falls outside it. That includes the doctor rows `story:doctor-identity` adds, the verdict reason, and the doctor gap fixes the run story itself makes in `doctor.rs`. Nothing records the omission as a decision. The same goes for a change to the README pointer or the install path (the release archive). The fix is to add `conductor doctor` to the trigger list, or to word the trigger as the epic does, as any change to what onboarding runs.

**Promises.** I extracted 9 from the epic and traced all 9 to an item. The 8 from round 1 are unchanged:
1. Only the URL and the prompt are given, and it works in any repository.
2. Conductor is installed.
3. The forge CLI is installed and logged in.
4. The planning store is current.
5. An instance is configured.
6. Doctor ends `ready`.
7. The person is asked only for what needs their hands.
8. The v0.4.0 failures are addressed.

The new promise is that two recorded runs prove it and are repeated before a release that changes it. `story:onboarding-run` claims it through its first, second and third acceptance bullets. No other item claims it, and nothing is claimed twice.

**What I read.** Five artifacts: the epic (`aep plan artifact show` and `cat -n` of the file), the four stories (`aep plan artifact show`), `review-result:onboarding-scope-r1`, and `aep plan artifact graph`. The graph shows no other item decomposing the epic.

**What I could not establish.**
- `story:doctor-identity` adds machine rows and account names. No sentence in the epic names them. I did not call this reach, because the epic's Stories line names the story and the epic's Outcome asks for the forge CLI to be logged in. Round 1 judged `story:setup-command` the same way.
- Still unestablished from round 1: whether "planning store current" covers a repository with no store at all.
- Out of my lane, as in round 1: `SETUP.md` appears in the scope of three stories, and `SETUP.md` hard-codes a pin while `b10x.toml` holds the pins.

```findings
- file: .engineering/planning/story/onboarding-run.md
  line: 55
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the epic promises the runs are repeated before a release that changes onboarding, which ends in `conductor doctor` ready, but the acceptance narrows the trigger to a release that changes SETUP.md, conductor setup or the prerequisites, so a release that changes only conductor doctor (doctor-identity's rows, the doctor gap fixes) is not gated and nothing records the omission"
```
