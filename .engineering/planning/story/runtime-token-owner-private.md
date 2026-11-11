---
format: aep.planning-md/3
id: story:runtime-token-owner-private
kind: story
status: draft
title: The runtimes file is owner-private and no process a runtime starts inherits its token
tags:
- from-finding
revision: 1
---
## Why

Found by the first adversary pass over unit U4 of wave 51
(`review-result:w51-u4-adversary-1`), which sealed the daemon's token for a boxed runtime in an
owner-private slot file. Both points were there before the wave.

- `keep()` writes `runtimes.json`, which holds every instance's runtime token, at the umask mode
  (`crates/conductor/src/daemon/reconcile.rs`), and the daemon's directory is created at the
  umask mode too. The owner-private slot file guards a token that is readable beside it. The
  token opens only the daemon's socket, which is owner-private, so another user holding it
  cannot connect: the impact is low.
- A runtime started as a unit gets `CONDUCTOR_API_TOKEN` in its environment and passes it to
  every process it starts. Unit U4 removed the slot variable from those processes; the token
  variable of the unit path is still inherited.

## Acceptance

- `runtimes.json` and the daemon's directory are owner-private, on a new install and after an
  upgrade of an existing one; a test reads the modes.
- No process a runtime starts inherits the runtime's token, by the variable or by a descriptor,
  on the unit path and the box path; a static case or a spawn case holds it.
