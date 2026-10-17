---
format: aep.planning-md/3
id: review-result:acceptance-attach-tui-quickstart-r2
kind: review-result
status: active
title: Acceptance critic, round 2 (session-attach, tui-frontend, quick-start)
relations:
- reviews: epic:session-attach
- reviews: epic:tui-frontend
- reviews: epic:quick-start
- reviews: story:session-attach-command
- reviews: story:session-attach-watch
- reviews: story:session-attach-refuses-sessions
- reviews: story:session-list-attachable
- reviews: story:tui-overview
- reviews: story:tui-decisions-read
- reviews: story:tui-decisions-answer
- reviews: story:tui-dispatches-activity
- reviews: story:tui-host-disk-builds
- reviews: story:tui-open-session
- reviews: story:install-one-liner
- reviews: story:demo-mode
- reviews: story:init-defaults
- reviews: story:quick-start-page
- reviews: story:quick-start-docs-test
revision: 1
---
needs-revision
story:quick-start-docs-test — acceptance 3 puts the tag-push run in "a separate workflow file" while acceptance 7 puts it in `.github/workflows/release.yml`, so no single place can be checked for the trigger and the two items cannot both be met as written; the body should say which file holds the tag-push trigger — .engineering/planning/story/quick-start-docs-test.md:33

What I read: all 16 stories and the 3 epics (19 artifacts), with `aep plan artifact show <id>` once per id, plus the round-1 acceptance record (`aep plan artifact show review-result:acceptance-attach-tui-quickstart-r1`), `aep plan artifact kinds` and `aep plan artifact lifecycle story`. In the tree I checked `story:watch-stalled-sessions` for `thresholds.build_queued` and ran `grep -n -i announce .github/workflows/*.yml`.

All six round-1 findings are fixed.
- `quick-start-docs-test` acceptance 2: the baseline is now the home as it stands right after install (line 28).
- `quick-start-docs-test` acceptance 7: it now names a gate, a check in `release.yml` before the publishing job (line 33). Acceptance 7 is the sentence the new finding is about.
- `session-attach-watch`: the modelling item is gone, and the context says this story models nothing.
- `install-one-liner` acceptance 6: it now states the chosen branch.
- `install-one-liner` acceptance 2: it names `CONDUCTOR_INSTALL_BASE`.
- `tui-host-disk-builds` acceptance 4: `thresholds.build_queued` (default 30m) exists in `story:watch-stalled-sessions` acceptance 2, and the story depends on it.

What I could not establish:
- Out of my lane, for design or scope: `quick-start-docs-test` acceptances 1 and 4 install "the newest release". On a tag push before publication that is the previous release, so a green check on a tag says nothing about the tag's own binary. No acceptance says the tag-push run uses a build of the tag.
- I did not read the stories outside the set, except for the one grep above.

```findings
- file: .engineering/planning/story/quick-start-docs-test.md
  line: 33
  category: acceptance
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "acceptance 3 puts the tag-push run in 'a separate workflow file' while acceptance 7 puts it in .github/workflows/release.yml, so no single place can be checked for the trigger and the two items cannot both be met as written; the body should say which file holds the tag-push trigger"
```