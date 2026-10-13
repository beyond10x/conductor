---
format: aep.planning-md/3
id: review-result:sandbox-acceptance-r1
kind: review-result
status: active
title: Acceptance critic, round 1, sandbox
relations:
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
needs-revision

story:session-envelope-seam — the `.agents/conductor.md` change has no check: "`.agents/conductor.md`'s fallback start, resume and stop forms name the boxed forms; a test in `crates/conductor/tests/taskfile.rs` reads the tasks" names no text that was absent before and is present after, and the only test named reads the tasks, not that file — .engineering/planning/story/session-envelope-seam.md:94
story:session-envelope-seam — the spec declares five `session start` refusals (unknown role, session already running and launch failed among them) but only three cases get a test, so those three can be declared and never produced while the story passes — .engineering/planning/story/session-envelope-seam.md:68
story:session-limits-systemd — the live run "shows its cgroup and limits" without saying against what, so it passes whatever the cgroup holds. It should say the limits read back equal the role's configured `limits` — .engineering/planning/story/session-limits-systemd.md:50
story:session-writes-bwrap — the home bind is "limited by the live run's finding", so the argument-list test cannot be written before the live run. Nothing says which write outside the places the run must try. Once `~` is a writable bind, a write under `$HOME` succeeds, which contradicts "can write only its own places" — .engineering/planning/story/session-writes-bwrap.md:57
story:session-writes-bwrap — the measurement bullet names a test only for a missing `bwrap`. Neither the recording of `envelope_writable` nor the refusal for "a requested place not bound" has a test — .engineering/planning/story/session-writes-bwrap.md:61
story:session-writes-bwrap — "say what the box enforces and what it does not" is a vote. The transition is nameable: the README.md:42 sentence "There is no operating-system sandbox yet" is gone, and the `bash-sandbox` pointer at docs/design/conductor.md:284 is gone — .engineering/planning/story/session-writes-bwrap.md:71
story:session-forge-credentials — the live run sets a read-only token, and an environment token takes precedence over the keyring, so `gh pr merge` fails with an authorization error whether or not the bus is unbound and the config directories are masked. A run without a token must show `gh api rate_limit` failing to authenticate — .engineering/planning/story/session-forge-credentials.md:52
story:session-forge-credentials — under `bwrap --ro-bind / /` the bus path is already visible, so an argument-list test cannot show the bus "not bound". The acceptance must name the mask over `/run/user/<uid>/bus` that the arguments contain — .engineering/planning/story/session-forge-credentials.md:44
story:session-forge-credentials — the doctor warns when "no read-only forge token [is] named", but conductor can only see whether `forge_token_env` lists names, not whether a token is read-only. The condition should be an empty `forge_token_env` — .engineering/planning/story/session-forge-credentials.md:50

What I read: the four ids, in full, with `aep plan artifact show <id>`, plus `aep plan artifact kinds` and `aep plan artifact lifecycle story`. In the tree I checked `Taskfile.yml` (`conductor:restart` delegates to `conductor:start`, so it needs no separate bullet), `README.md:42`, `docs/design/conductor.md:284`, `spec/domains/observation.yaml` (the `envelope_*` fields) and `spec/domains/config.yaml`. I also read the sibling stories `session-envelope-model` and `session-box-spike`.

What I could not establish:
- Each acceptance is a multi-bullet checklist rather than one sentence. The store's other stories and the epic use the same form, and each bullet is separately checkable, so I did not raise it.
- Out of my lane:
  - `--unit …` is left as an ellipsis, while the live-run cleanup check assumes a `conductor-*` unit name. That is a design question.
  - `docs/config.md` is in the writes-bwrap scope but not in its acceptance, which is a scope question.
- The forge live run waits on `decision-blocker:sandbox-forge-tokens`. I did not count that against the acceptance.
- The store's working tree is modified (`git status` shows the three session stories as `M`). I judged the working-tree bodies.

```findings
- file: .engineering/planning/story/session-envelope-seam.md
  line: 94
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the acceptance says `.agents/conductor.md`'s fallback start, resume and stop forms 'name the boxed forms' with no text that was absent before and is present after, and the only test named reads the tasks, not that file"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 68
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the spec declares five `session start` refusals (unknown role, session already running, launch failed among them) but only three cases get a test, so those three can be declared and never produced while the story passes"
- file: .engineering/planning/story/session-limits-systemd.md
  line: 50
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the live run 'shows its cgroup and limits' without saying against what, so it passes whatever the cgroup holds; it should say the limits read back equal the role's configured limits"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 57
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the home bind is 'limited by the live run's finding', so the argument-list test cannot be written before the run, and nothing names which write outside the places the run must try; with ~ a writable bind, a write under $HOME succeeds, contradicting 'can write only its own places'"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 61
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the measurement bullet names a test only for a missing bwrap; the recording of envelope_writable and the refusal for a requested place not bound have no test"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 71
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "'say what the box enforces and what it does not' is a vote; name the transition, that the README.md:42 sentence 'There is no operating-system sandbox yet' and the bash-sandbox pointer at docs/design/conductor.md:284 are gone"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 52
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the live run sets a read-only token, and an environment token takes precedence over the keyring, so `gh pr merge` fails with an authorization error whether or not the bus is unbound and the config directories masked; a run without a token must show `gh api rate_limit` failing to authenticate"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 44
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "under `bwrap --ro-bind / /` the bus path is already visible, so an argument-list test cannot show it 'not bound'; the acceptance must name the mask over /run/user/<uid>/bus that the arguments contain"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 50
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the doctor warns when 'no read-only forge token [is] named', but conductor can only see whether forge_token_env lists names, not whether a token is read-only; the condition should be an empty forge_token_env"
```
