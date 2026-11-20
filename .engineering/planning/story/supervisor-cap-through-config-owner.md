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
  path: crates/conductor/tests/host_roles.rs
revision: 2
---
## Why

Issue #264. On 2026-11-20 the supervisor took a cap decision (an instance's
`controllers.max_working` from 4 to 5) and carried it out with `conductor config put` on the host.
The host's own rules route box-level changes through the repository that owns the config file;
the edit bypassed it (and, through #263, rewrote the whole file). The decision stays the
supervisor's; only its execution goes through the owner of the file.

## What changes

`crates/conductor/roles/supervisor.md`, section "Which decisions are yours", the table row
"yours, with the operator's authority": in the examples cell, after "a cap such as
`controllers.max_working`", append "(carried out through the config file's owner when the host's
file names one)".

## Acceptance

- A test in `crates/conductor/tests/` (beside the existing profile tests) that fails first on main:
  `conductor role show --role supervisor` prints the row with the appended words.
- `task check` green; the diff of the profile is reported (AGENTS.md rule 9).
- The profile names no organization, repository or person (rule 9; `no_organization.rs`).
