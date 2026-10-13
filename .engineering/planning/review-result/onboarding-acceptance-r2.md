---
format: aep.planning-md/3
id: review-result:onboarding-acceptance-r2
kind: review-result
status: active
title: Acceptance critic, round 2, onboarding
relations:
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
- reviews: story:spec-first-wording
revision: 1
---
needs-revision

All five round-1 findings are fixed. The re-read turned up three new findings, and the first of them is a blocker.

**Round 1, finding by finding**
- **Setup guide (r1#1), no longer holds.** Line 42 now has the test require `Check:`, `Fix:` and `Hands:` lines for every step. "Safe to repeat" is now only the definition of a `Check:` line, so the unchecked promise is gone.
- **Doctor identity (r1#2), no longer holds.** Line 44 now says the hosts come from the config file's sources across every instance. That is `github.com` for a `github` source and the source's `host` for a `gitlab` source. Line 51 adds the verdict reason. `doctor.rs` still has no host concept, but the acceptance now says where hosts come from, so the story is checkable.
- **Onboarding run (r1#3), no longer holds.** Line 45 now requires `conductor setup` output before and after. Its store row must read old before and current after, and the verdict must read `ready` after.
- **Snapshot duration, view not named (r1#4), no longer holds.** Line 40 names the view `snapshot collector-runs`.
- **Snapshot duration, borrowed term (r1#5), no longer holds.** Line 44 now declares its own record and states: a collector run with `completed`, `failed` or `over budget`. The "member failure" wording is gone.

**New findings**
- **story:setup-guide-for-agents (blocker).** Line 42 limits "every other tool named" to `b10x.toml` or `git`, `gh` and `glab`, but the first bullet requires `cargo install` and a `SHA256SUMS` check. `b10x.toml` pins only `aep`, `ess` and `worktree`, so the test cannot pass on the SETUP.md that bullet requires.
- **story:doctor-identity (warning).** Line 44 promises "never a token or an e-mail address". The test asserts only that no token string appears, so the e-mail half has no check.
- **story:spec-first-wording (warning).** Line 29 names two places in `repo-controller.md` (the spec-first paragraph and the brief text it copies) and a paragraph in `conductor.md`. The test only finds the sentence in each of the three files. In `.agents/repo-controller.md` the paragraph and the brief block are separate places (lines 15 to 26), so one occurrence passes while the other place is left without it.

story:setup-command, story:snapshot-duration and story:onboarding-run get no finding. Each bullet is observable and names a test, a view or a recorded run, and the transitions are stated (old to current store, missing to written config, `snapshot collector-runs` before and after the change).

**What I read:** 7 artifacts. I opened the six stories and `review-result:onboarding-acceptance-r1` in full with `aep plan artifact show`. I also read `b10x.toml`, `cli.yaml` (by grep), `doctor.rs` (by grep), `.agents/repo-controller.md` and `crates/conductor/tests/profiles.rs`, and checked that `SETUP.md` does not exist yet. I did not run `aep plan artifact kinds` or `lifecycle`.

**What I could not establish:**
- The harness refused a loop over ids, so I ran each `aep plan artifact show` as a separate command after `cd` into conductor-wave-17.
- I could not tell how `docs_setup.rs` will extract "tool named" from a step. The blocker assumes any program in a `Check:` or `Fix:` command counts.
- Out of my lane, not counted in the verdict: `story:snapshot-duration` still has `depends_on story:setup-command`, and its Notes tie that only to shared files (design or parallel-safety critic).

```findings
- file: .engineering/planning/story/setup-guide-for-agents.md
  line: 42
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the test requires every tool named other than conductor to be in b10x.toml or be git, gh or glab, but the first bullet's steps name cargo install and a SHA256SUMS check, and b10x.toml pins only aep, ess and worktree, so the check cannot pass on the SETUP.md the acceptance requires"
- file: .engineering/planning/story/doctor-identity.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says the identity row shows 'login name only, never a token or an e-mail address', but the test asserts only that no token string from the fake's output appears, so the e-mail half has no check"
- file: .engineering/planning/story/spec-first-wording.md
  line: 29
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance names two places in repo-controller.md (its spec-first paragraph and the brief text conductor copies) but the test only finds the sentence in each of the three files, so one occurrence passes while the other place lacks it"
```
