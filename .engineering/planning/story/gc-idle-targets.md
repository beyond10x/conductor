---
format: aep.planning-md/3
id: story:gc-idle-targets
kind: story
status: active
title: conductor gc cleans idle cargo targets in trees and caches
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T16:52:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-07T16:52:26Z", actor: "human:timo", revision: 4}
---
## Why

The operator, 2026-11-07: "all modifications need to be persistent when generic, anything else should not even exist". A host disk keeper outside the product (a 111-line shell script) cleans idle cargo targets that `conductor gc` does not: on 2026-11-07 conductor-dev freed 23G of `target/` in six idle agent trees by hand while `/` was 27G under the 120G floor.

## Acceptance

- `conductor gc` finds cargo targets (by `CACHEDIR.TAG`) under the instance's trees and cache roots, never inside a primary checkout, and cleans one when it is idle: no process has its cwd, exe, an open file, `CARGO_TARGET_DIR` or a command-line path in it, and nothing in its top three levels was written for 180 min (90 min while `/` is under 50G). A target holding a `.keeper-hold` file is never cleaned. Oldest first, until free disk reaches the floor.
- `--dry-run` lists each candidate with size, last write and the reason it is or is not idle; a test covers a held target, a fresh one and an idle one.
- The host reclaim runs it; the host's disk keeper is retired once it does.
