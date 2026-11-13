---
format: aep.planning-md/3
id: review-result:forge-access-anonymous-security-round-1
kind: review-result
status: active
title: 'forge_access: anonymous, security review round 1'
relations:
- reviews: story:forge-access-anonymous
revision: 1
---
needs-change

Recorder's note: the reviewer's report on commit `253af3b835` "forge_access: anonymous", recorded
after release 1.8.0. The report came back after the commit was merged and the release tagged; a
second, source-only read had found no blocker before the merge. The sections up to "Not run" are
the reviewer's; "What followed" is the recorder's.

## What was reviewed

The commit that adds the instance member `forge_access: anonymous`: the pod of such an instance
carries no credential of any forge, clones public GitHub repositories over HTTPS with none, and
every forge write of a session is denied. Read by an independent security reviewer in a worktree
of its own, with six new cases in a test file of its own.

## Suites, as printed

- The reviewer's cases alone: `6 tests run: 3 passed, 3 failed, 0 skipped`.
- With `forge_access_anonymous` and `kubernetes_guide`: `32 tests run: 29 passed, 3 failed, 0
  skipped`.

## Findings

1. Blocker, introduced. `crates/conductor/src/daemon/pods.rs`, `manifests`: the pod leaves out
   `daemon.kubernetes.env` entries by variable name only. A second entry under another name that
   names the same secret as `GH_TOKEN` is kept: the daemon's forge token is in the anonymous pod's
   Secret, its environment and its secret volume. Shown by a failing case. The guide documents
   that double entry for other pods.
2. Should-fix, introduced. `is_gits_config` in the same file: `GIT_CONFIG_GLOBAL` and
   `GIT_CONFIG_SYSTEM` entries are kept and can point git at a file with a credential helper.
   Shown by a failing case.
3. Note, introduced. `crates/conductor/src/guard/rules.rs`, `anonymous_write`: a push behind
   `env`, `timeout`, `nohup`, `xargs`, `find -exec`, a variable, an alias, `git-push` by path,
   `git subtree push`, `git lfs push` and a write through the API route pass the rule. Shown by a
   failing case of 22 decisions. With no credential in the pod none of them can write.
4. Note, inferred and not run. `crates/conductor/src/collect/repositories.rs`: an `include` entry
   that validation accepts and the listing refuses fails the whole listing.

## Held

- An anonymous instance beside one with a `forge_token` of its own: nothing of the second is in
  the first's pod.
- A mapping that names the daemon's forge secret at another key is refused at that key's path.
- The member changes the rendered file and the pod's digest, so setting or removing it replaces
  the pod.
- Thirteen push spellings are denied and name the member, among them `sh -c`, a subshell,
  `command`, `exec`, `git -C`, `send-pack` and the bot route.
- Without the member every change is behind the member's check.

## Not run

- A pod on a cluster and a clone from the real forge.
- The watch's forge probes and `secret_syncs` in such a pod.

## What followed

- Release 1.8.0 was published with the commit. Its notes carry a "Known issue" paragraph that
  names findings 1 and 2 and says not to use the member with such a configuration before 1.8.1.
- The four findings and the second read's points are `story:anonymous-pod-no-forge-secret`.

```findings
[
{"file":"crates/conductor/src/daemon/pods.rs","line":381,"category":"integrity","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the anonymous filter drops env entries by name only, so a second entry naming the daemon's forge token secret puts the token into the anonymous pod's Secret, env and secret volume"},
{"file":"crates/conductor/src/daemon/pods.rs","line":169,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"GIT_CONFIG_GLOBAL and GIT_CONFIG_SYSTEM entries are kept in an anonymous pod and can point git at a configuration with a credential helper"},
{"file":"crates/conductor/src/guard/rules.rs","line":2375,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the anonymous rule misses git pushes behind launchers, expansions, aliases, git-push, subtree and lfs push, and the API route, contrary to the guide's every-write claim"},
{"file":"crates/conductor/src/collect/repositories.rs","line":677,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"an include entry validation accepts but checked() refuses fails the whole anonymous listing instead of one member failure, inferred and not run"}
]
```
