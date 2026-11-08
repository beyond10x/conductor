---
format: aep.planning-md/3
id: story:watch-relays-checks-after-done
kind: story
status: active
title: The watch relays concluded checks of a controller's open request whatever its dispatch's status
summary: A controller idled 26 min on a release PR whose dispatch was already Done
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-08T14:29:17Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-11-08T14:29:18Z", actor: "human:timo", revision: 5}
---
## Why

Host supervisor, 2026-11-08: a controller sat idle from 15:02 to 15:28 local, waiting to be told
about its release pull request on GitHub. The request's checks ended at 14:21Z, one of them
cancelled. Its dispatch was already Done, so neither `story:watch-unblocks-merged-requests` (Blocked
dispatches only) nor any other path relayed the result.

## Acceptance

1. At a watch tick, every open pull or merge request a controller opened, in a repository whose
   running controller is idle at its prompt, is read on the forge whatever its dispatch's status
   (Done included); GitHub and GitLab both. Test with a fixture forge answer for each.
2. A request whose checks have concluded since the last tick (success, failure or cancelled) is
   relayed to the controller once per head commit, naming each failed or cancelled job. Test.

## Ambiguities

None.

## Activation

Moved to active on the host supervisor's decision D26 (2026-11-08): it caused a controller's
26-min idle that day and serves K5. Planned into the wave after 48, beside
story:build-admission-rereads-and-explains; the two land on different files.
