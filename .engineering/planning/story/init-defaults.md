---
format: aep.planning-md/3
id: story:init-defaults
kind: story
status: draft
title: conductor init with no config writes a working one-instance setup
relations:
- decomposes: epic:quick-start
revision: 1
---
# Story: init-defaults

## Outcome

`conductor init` with no config file and no flags makes a working one-instance setup: it writes the config file at its default path, names one instance with every default, and seeds that instance's records, so no hand-written config is needed to reach a dashboard.

## Context

- Today a person writes the config by hand before most commands (website/docs/getting-started.md, "A config" section, from :110), and `conductor init` seeds the records directory only: git repository, `NORTHSTAR.md`, `docs/handoff/<date>.md`, `rules.md`, `README.md`, `.engineering/workspace.yaml`, `decisions/`, `dispatches/`, `charters/`, committing what it wrote and overwriting nothing (crates/conductor/src/init.rs:1-12).
- Absent keys have defaults: records `~/.b10x/conductor/<name>/records`, state `~/.b10x/conductor/<name>/state`, cache `~/.cache/b10x/conductor/<name>`, `dashboard_port` 7313 plus the index (website/docs/reference/config.md, instance section).
- `init`'s input is `conductor.config.InitRecords` with one field, `instance` (spec/domains/config.yaml:538-541); the CLI binding is at crates/conductor/cli.yaml:74-78.
- The first-run seed's texts "name no organization" (init.rs:11-12); a default instance must not either.

## Acceptance

1. Spec first: extend `conductor.config.InitRecords` in `spec/domains/config.yaml` (:538) with the config-writing outcome and its refusals (`config-exists` when a file is there and the flag to replace is not given), validated with `ess specify validate --path spec`; `crates/conductor-model` is regenerated.
2. In a home directory with no conductor config, `conductor init` writes the config at the documented default path with one instance named `default`, no sources but none needed, then seeds its records, and prints each path it wrote and the next command; a second run writes nothing and exits 0 saying the setup is there.
3. A config file that exists is never overwritten or edited: `conductor init` seeds the records of the instance it names (as today) and says the config was left alone.
4. The written config passes `conductor config validate` and `conductor doctor` does not fail on it (a missing program that the default instance does not need is `not needed`).
5. `conductor dashboard serve` with no flags on that setup serves on the default port and answers `/api/board` with an empty, well-formed answer (no snapshot yet is not an error) (crates/conductor/src/dashboard.rs:25-35).
6. Nothing in the written files names a person, an organization or a host path of the machine it ran on.
7. `website/docs/guides/initialise-the-records.md` and the generated CLI reference describe the new behaviour; `task docs-check` passes.

## Out of Scope

Interactive prompts; choosing an organization's name; a repository list (`conductor setup` adds a repository).

## Ambiguities

- `conductor init` writing the config is a change of a command's meaning for people who ran it only for records; the guard (docs/design/conductor.md § 7) denies writes of conductor's config from sessions, so the command must stay out of their reach. The alternative is a new `conductor init --defaults`; the operator decides.
