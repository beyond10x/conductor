---
format: aep.planning-md/3
id: review-result:acceptance-attach-tui-quickstart-r1
kind: review-result
status: active
title: Acceptance critic, round 1 (session-attach, tui-frontend, quick-start)
relations:
- reviews: story:quick-start-docs-test
- reviews: story:session-attach-watch
- reviews: story:install-one-liner
- reviews: story:tui-host-disk-builds
revision: 1
---
needs-revision
story:quick-start-docs-test — acceptance 2 fails the check "if the home directory is not as it started" after `conductor demo --clean`, but the page's install command has put `~/.local/bin/conductor` into that home by then, so no baseline makes it checkable — .engineering/planning/story/quick-start-docs-test.md:28
story:quick-start-docs-test — acceptance 7 says "a release job that finds the check red on its tag opens no announcement", but no workflow step announces (`grep -n -i announce .github/workflows/*.yml` is empty) and AGENTS.md § Releases step 5 has conductor-dev create the Release by hand, so the sentence reads the same before and after the work and names no observable gate — .engineering/planning/story/quick-start-docs-test.md:33
story:session-attach-watch — acceptance 1 asks for the relay's `read-write` and `watch` modes and refusal frame to be modelled, which `story:session-pty-relay` acceptance 1 already delivers, so it is true or false regardless of this story's work and names no transition of its own — .engineering/planning/story/session-attach-watch.md:26
story:install-one-liner — acceptance 6 passes on either branch ("if the release must also publish a fixed-name file ... If none is needed, no spec change, and the story says so"), so nothing observable distinguishes done from not done; it should state the chosen branch, and acceptance 1 already describes the no-spec-change one — .engineering/planning/story/install-one-liner.md:31
story:install-one-liner — acceptance 2 tests the command "against a local file server", but acceptance 1 fixes the source as the `releases/latest` redirect of github.com and acceptance 7 requires the same text on three pages, so no acceptance names the override that lets a test redirect the command — .engineering/planning/story/install-one-liner.md:27
story:tui-host-disk-builds — acceptance 4 puts the longest wait in the attention lines "when over the instance's threshold", but the config reference has no wait threshold (`thresholds.*` in website/docs/reference/config.md:79-87 are all byte sizes), so "over" cannot be evaluated — .engineering/planning/story/tui-host-disk-builds.md:30

What I read: all 16 ids, in full, with `aep plan artifact show story:<id>` run once per id. I also read `aep plan artifact kinds` and `aep plan artifact lifecycle story`. I checked the tree with `git grep`, `grep` and `ls` for `AGENTS.md`, `.github/workflows`, `website/docs/reference/config.md`, `crates/conductor/tests/docs_*.rs` and the three referenced stories (`sessions-systemd-kind`, `dashboard-build-queue`, `build-run-cpu-slots`).

What I could not establish:
- I did not read `story:sessions-systemd-kind`, `story:dashboard-build-queue` or `story:build-run-cpu-slots`. I only confirmed they exist.
- Several stories lead with "Spec first: no new noun; the model is unchanged" (`tui-decisions-read`, `tui-dispatches-activity`, `tui-host-disk-builds`, acceptance 1). That item reads the same before and after the work. Each story's other acceptances carry the transition, so I did not count it as a finding.
- Out of my lane, for the design critic. `story:tui-open-session` acceptance 6 ("exits 1 naming `inside-session`") relies on `story:session-attach-refuses-sessions` but has no `depends_on` to it. `story:tui-decisions-answer` acceptance 6 also denies `conductor tui` to sessions, by a guard rule rather than a client exit.
- Out of my lane, for the scope critic. `story:init-defaults` leaves open whether `conductor init` or a new `init --defaults` writes the config. Its acceptance 2 and 3 assume the first.

```findings
- file: .engineering/planning/story/quick-start-docs-test.md
  line: 28
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2 fails the check 'if the home directory is not as it started' after `conductor demo --clean`, but the page's install command has put ~/.local/bin/conductor into that home by then, so no baseline makes it checkable"
- file: .engineering/planning/story/quick-start-docs-test.md
  line: 33
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 7 says 'a release job that finds the check red on its tag opens no announcement', but no workflow step announces (grep -n -i announce .github/workflows/*.yml is empty) and AGENTS.md Releases step 5 has conductor-dev create the Release by hand, so the sentence reads the same before and after the work and names no observable gate"
- file: .engineering/planning/story/session-attach-watch.md
  line: 26
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 1 asks for the relay's read-write and watch modes and refusal frame to be modelled, which story:session-pty-relay acceptance 1 already delivers, so it is true or false regardless of this story's work and names no transition of its own"
- file: .engineering/planning/story/install-one-liner.md
  line: 31
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 6 passes on either branch ('if the release must also publish a fixed-name file ... If none is needed, no spec change, and the story says so'), so nothing observable distinguishes done from not done; it should state the chosen branch, and acceptance 1 already describes the no-spec-change one"
- file: .engineering/planning/story/install-one-liner.md
  line: 27
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 2 tests the command against a local file server, but acceptance 1 fixes the source as the releases/latest redirect of github.com and acceptance 7 requires the same text on three pages, so no acceptance names the override that lets a test redirect the command"
- file: .engineering/planning/story/tui-host-disk-builds.md
  line: 30
  category: acceptance
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "acceptance 4 puts the longest wait in the attention lines 'when over the instance's threshold', but the config reference has no wait threshold (thresholds.* in website/docs/reference/config.md:79-87 are all byte sizes), so 'over' cannot be evaluated"
```