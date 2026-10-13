---
format: aep.planning-md/3
id: review-result:onboarding-scope-r1
kind: review-result
status: active
title: Scope critic, round 1, onboarding
relations:
- reviews: epic:onboarding-from-url
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
revision: 1
---
needs-revision

- story:onboarding-run — its third acceptance bullet adds a standing release rule to AGENTS.md § Releases ("repeat both runs before a release that changes `SETUP.md`, `conductor setup` or the prerequisites"). No sentence in the epic's Why or Outcome asks for a release gate, so the item reaches beyond the parent, and it puts `AGENTS.md` in scope. The fix is to drop the bullet and `AGENTS.md` from scope, or get the epic to state the promise first. — `.engineering/planning/story/onboarding-run.md:34`

**Promises extracted from the epic: 8. Traced to an item: 8.**
1. Only the URL and "use conductor in this repo" are given, and the session works in any repository. `story:setup-guide-for-agents` claims this through the README pointer and `SETUP.md`. `story:onboarding-run` proves it on a GitHub and a GitLab scratch repository.
2. Conductor ends up installed. `SETUP.md` has an install step using the release archive with `SHA256SUMS`, or `cargo install`. `conductor setup` reports it against the pins.
3. The forge CLI is installed and logged in. `SETUP.md` covers `gh` and `glab` with the host. The `conductor setup` forge row checks it.
4. The planning store is current. `SETUP.md` and `conductor setup` check the version and show the migrate dry-run. The person approves the `--verify` migration.
5. An instance is configured for the repository. `SETUP.md` writes the instance config. `conductor setup --apply` writes it, then runs `conductor init` and `conductor trust`.
6. `conductor doctor` ends `ready`. `SETUP.md` runs doctor until `ready`. `story:onboarding-run` records the final output ending `ready`.
7. The person is asked only for what needs their hands (a login, a migration). `SETUP.md` names the hands step for each step. `conductor setup --apply` never logs in, installs or migrates. `story:onboarding-run` requires questions only for logins and the store migration.
8. The v0.4.0 failures are addressed: missing `gh`, old `aep`, an unmigrated store, a source build, and a `doctor` that exited 1. They map to the `gh`/`glab` step, the `aep`/`ess`/`worktree` pins, the migration step, the release-archive install, and doctor ending `ready`. I judged the last one a promise for `story:onboarding-run` and the stories it depends on.

I found no gap, no narrowed promise and no duplicated outcome. `story:setup-command` is listed in the epic's own Stories line. Its CLI surface is the means for promises 2 to 7, so it is not extra.

**What I read:** the epic and the three stories in full, and `aep plan artifact graph`. No other item claims part of the epic. Commands: `aep plan artifact show` on each of the four, `aep plan artifact graph`, a `grep` of the epic and story files for line numbers, and `ls` of `crates/conductor/src`.

**What I could not establish:**
- Whether "its planning store current" covers a repository with no planning store at all. Both stories handle only an old store, and `conductor setup` has a "records exist" row. I did not confirm that `conductor init` creates a store, so I did not call this a gap.
- Out of my lane: `SETUP.md` is in the scope of all three stories, and `story:setup-command` lists it in scope with no acceptance line about it. That belongs to parallel-safety and design. The hard-coded `aep.project/5` in `SETUP.md` against "pins of `b10x.toml`" also belongs to design.

```findings
- file: .engineering/planning/story/onboarding-run.md
  line: 34
  category: scope
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance adds a standing AGENTS.md § Releases rule (repeat both runs before a release that changes SETUP.md, conductor setup or the prerequisites) that no sentence of epic:onboarding-from-url asks for, and puts AGENTS.md in scope"
```
