---
format: aep.planning-md/3
id: story:supervisor-cap-through-config-owner
kind: story
status: draft
title: The supervisor carries out a cap decision through the config file's owner
summary: The supervisor profile's cap example says the change goes through the config file's owner when the host names one
tags:
- from-issue
refs:
- provider: github
  reference: beyond10x/conductor#264
relations:
- decomposes: epic:loop-without-handwork
scope:
- confidence: cited
  path: crates/conductor/roles/supervisor.md
- confidence: cited
  path: crates/conductor/tests/profiles.rs
revision: 4
---
## Why

Issue #264. On 2026-11-20 the supervisor took a cap decision, raising an instance's
`controllers.max_working` from 4 to 5, and carried it out with `conductor config put` on the host.
The host's own rules route box-level changes through the repository that owns the config file, and
the edit bypassed it (through #263 it also rewrote the whole file). The decision stays the
supervisor's; only how it is carried out goes through the owner of the file.

## What changes

In `crates/conductor/roles/supervisor.md`, section "Which decisions are yours" (`:231`), the table
row "yours, with the operator's authority" (`:238`): in the examples cell, after "a cap such as
`controllers.max_working`", append "(carried out through the config file's owner when the host's
own file names one)".

"The host's own file" is the profile's existing name for `host/supervisor.md` beside the config
file (`:29`, `:82`). Who owns a host's config file is true for one host only, so it is not named
in the profile (AGENTS.md rule 9). The row stays one line: a Markdown table row cannot wrap. The
profile is 462 lines against its cap of 463 (`tests/profiles.rs:152-174`), and this edit adds no
line.

## Acceptance

- A test in `crates/conductor/tests/profiles.rs`, beside
  `the_supervisor_watches_with_conductor_supervisor_watch` (`:154`), that fails first on main. It
  reads the profile with that file's `profile("supervisor.md")`, which joins whitespace. It
  asserts that the text holds "a cap such as `controllers.max_working` (carried out through the
  config file's owner when the host's own file names one)". The binary prints the same file
  (`include_str!`; `tests/roles.rs` holds `conductor role show` to it), so no second test runs the
  binary. `tests/host_roles.rs` tests the daemon starting the host roles, not profile text.
- The line cap, asserted in that same case (`profiles.rs:170-174`), stays green.
- `task check` green; the diff of the profile is reported (AGENTS.md rule 9).
- The profile names no organization, repository or person (rule 9; `no_organization.rs`).

## Scope

- `crates/conductor/roles/supervisor.md`: the one row.
- `crates/conductor/tests/profiles.rs`: the new case.

## Boundaries

- No other row or section changes. No mirror of the profile exists under `website/` or in a
  generated file.
- Shares no file with `story:config-put-touches-only-its-path` (wave 73).
