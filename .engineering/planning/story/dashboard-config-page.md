---
format: aep.planning-md/3
id: story:dashboard-config-page
kind: story
status: active
title: The dashboard shows each instance's configuration and edits it through staged, validated, recorded writes
summary: the operator asked to see and change instance configuration from the dashboard
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-07T13:19:31Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-07T13:19:32Z", actor: "human:timo", revision: 3}
---
## Why

The operator, 2026-11-07: "as one dashboard now can basically show multiple instances of
conductor, what would be also great is in the dashboard some way to see the configuration of the
instances and even to modify that, e.g. like adding a new repository or changing ... the
configuration". He approved the second mock-up the same day ("I give the okay for the config
stuff. And the edit mode and such"). The mock-up is committed as
`docs/design/dashboard-config-mockup.html`.

## Acceptance

Spec first: the view of an instance's effective config, the staged-change write command, its
errors and the decision it records are modelled in `spec/` before the handler.

- A Configuration page lists the host registry's instances (loop state, port, prefix, forge) and,
  for the chosen one, every section of `conductor config show`: instance paths read-only; sources
  and repositories; controllers; cadence; thresholds grouped Disk, Builds, Sessions; roles and
  models; authority; KPI targets. Each field shows its YAML key, its default when it differs, and
  when a change takes effect (next watch tick, next cycle, loop restart).
- Edits are staged, not written: a YAML diff of the staged changes, each field validated as
  `config validate` does and a bad value named by its YAML path; Write is disabled while any check
  fails. Undo per field, reset to default, discard all.
- Repositories: switch Active/Inactive with a required reason, set how a repository releases
  (Releases, semver tags, ships without releases: the K5 inputs of
  `story:k5-tags-and-no-release-repositories`), add a repository by forge and owner/name or URL
  (the project is checked on the forge and the checkout cloned if missing), exclude one from its
  source.
- Write: from 127.0.0.1 with the page's token only (as the Decisions page's answers); runs
  `config validate` on the result, keeps the old file as `conductor.yaml.<time>.bak`, writes it,
  and records one class-C decision decided by the operator with the diff as its evidence. When a
  staged key needs a loop restart, the dialog offers Write and restart.
- A history list shows the config decisions, each with a revert that stages its inverse.
- A read-only table compares the keys whose values differ between the host's instances.
- Tests: the write handler refuses a non-local caller, a missing token and an invalid result, and
  writes the backup and one decision for a valid one; Vitest covers staging, validation and the
  diff; the page joins the UI loop's layout and accessibility gates (`story:dashboard-ui-loop`).
