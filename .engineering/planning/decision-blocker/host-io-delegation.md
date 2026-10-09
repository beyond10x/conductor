---
format: aep.planning-md/3
id: decision-blocker:host-io-delegation
kind: decision-blocker
status: open
title: Nobody has decided whether this host delegates the io controller to the user manager
relations:
- blocks: story:substrate-client
- blocks: story:substrate-tool-server
- blocks: story:substrate-gate-provider
revision: 1
---
## Question

Should this host delegate the cgroup `io` controller to the user manager? Today the user scope has
`cpu`, `memory` and `pids` only. Without `io`, substrate withholds `exec.resource-usage`, so harness
withholds its confined `run` tool, and a direct exec through `b10x-harness-substrate` is refused
with `exec.metrics-unserved` (`docs/analysis/substrate-tool-spike.md`, step 1 and step 2).

Options:
- (A) A root drop-in for `user@.service` adding `io` to `Delegate=`, then a restart of the user
  manager. The restart ends the Claude Code daemon and every background session on the host
  (conductor, conductor-dev, the controllers); they are resumed afterwards.
- (B) Leave the host as it is. The substrate stories stay parked; confinement goes ahead with the
  `systemd` provider and bwrap.

Operator decision: root access, and an interruption of every session.

## What clears it

The operator's choice, recorded as an `approval-record` that `decides` this blocker. A: after the
change, `cat /sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service/cgroup.controllers`
lists `io`; this blocker moves to `cleared`, and `story:substrate-tool-spike`'s step 3 runs before
`story:substrate-client`. B: this blocker moves to `cleared`, and the substrate stories are archived
with the record cited.
