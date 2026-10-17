---
format: aep.planning-md/3
id: story:session-commands-complete
kind: story
status: draft
title: conductor session restart and conductor maintainer cover every session task
relations:
- decomposes: epic:binary-only-operator
- depends_on: story:profiles-embedded
- depends_on: story:session-attach-command
- depends_on: story:session-list-attachable
revision: 2
---
# Story: session-commands-complete

## Outcome

Every session task in the Taskfile has a `conductor` command: `conductor session restart` and `conductor maintainer start|attach` are added, and `session attach` and `session list` (owned by `epic:session-attach`) are what the attach and list tasks call. The tasks become one-line wrappers, and an operator with only the binary starts, restarts, lists and opens the director and the maintainer.

## Context

- The tasks and what they do beyond the command: `sessions` lists the instance's role sessions and refuses names held twice (Taskfile.yml:14-30), `conductor:start` checks the model, harness, records directory, profile, checkout behind upstream, agent file and the bypass-permissions disclaimer before it starts (:31-80), `conductor:attach` opens the session in the records directory (:81-100), `conductor:restart` makes the same checks, stops, then starts (:101-153), `dev:start` and `dev:attach` do the same for the maintainer (:154-198).
- `conductor session start|stop` exist with `--role` (crates/conductor/cli.yaml:306-314, spec/domains/session.yaml:31-42); `session start` already checks provider, measurement, unknown role and a running session (cli.yaml:113-122). The checks of `conductor:start` that the command lacks are profile-is-a-file, checkout-behind-upstream, agent-file and the disclaimer.
- `story:session-attach-command` adds `[session, attach]` and `story:session-list-attachable` adds `[session, list]`; both share `spec/domains/session.yaml` and `crates/conductor/cli.yaml` with this story. This story does not model them; it depends on both and only repoints the tasks.
- The maintainer is one per host, outside every instance (`epic:role-names`, docs/design/roles-and-surfaces.md section 2), so its commands are a noun of their own, `maintainer`, not a flag of `session`; `--role maintainer` on `session start` stays an alias for one release.
- With no checkout there is no `.claude/conductor-settings.json` to default the settings file to (Taskfile.yml:50); the embedded profiles (`story:profiles-embedded`) replace the checkout as the source of a profile, and a role with no `settings` key starts with the project and local setting sources only.
- Shared surface: `spec/domains/session.yaml`, `crates/conductor/cli.yaml`, `crates/conductor/src/cli.rs` and the generated reference with `story:session-attach-command`, `story:session-list-attachable` and `story:demo-mode`; no ordering edge beyond the dependencies, the merge cost is accepted.

## Acceptance

1. Spec first: model `conductor.session.RestartSession` (field `role`) with the refusals of `StartSession` plus `profile-missing`, `profile-behind-upstream`, `agent-missing` and `disclaimer-not-accepted`, and `conductor.session.StartMaintainer` and `AttachMaintainer`, in `spec/domains/session.yaml` beside `StopSession` (:38); add `[session, restart]`, `[maintainer, start]` and `[maintainer, attach]` to `crates/conductor/cli.yaml`; validated with `ess specify validate --path spec`.
2. `conductor session start --role director` (and `conductor`, the old name) makes the checks `task conductor:start` makes, in the same order, and refuses with the same texts; a case for each refusal in `crates/conductor/tests`, run with a stand-in `claude` and no checkout.
3. `conductor session restart --role director` makes every check before it stops anything, then stops and starts; a refused check leaves the running session running (a case with a running stand-in session).
4. `conductor maintainer start` starts the maintainer on the host's maintainer role in the box its confinement asks for; `conductor maintainer attach` opens it as `conductor session attach` opens a session; both refuse when no maintainer role is configured, naming the config key.
5. `task sessions`, `task conductor:attach`, `task conductor:restart`, `task conductor:start`, `task dev:start` and `task dev:attach` each run one `conductor` command and hold no `claude agents`, `jq` or `git` logic; a test in `crates/conductor/tests/taskfile.rs` reads the Taskfile and fails on any of those words in them.
6. With the binary only (no checkout, a home holding a config and the installed profiles), the commands of items 2 to 4 give the same results as the task cases in `crates/conductor/tests/taskfile.rs`.
7. `website/docs/reference/cli.md` is regenerated and `task docs-check` passes.

## Out of Scope

Attach, list and the relay (`epic:session-attach`); starting a controller (`conductor spawn`); the confinement providers; Windows.

## Ambiguities

- Decided: the operator asked for `session attach|list`; `epic:session-attach` already plans them, so this story depends on `story:session-attach-command` and `story:session-list-attachable` rather than model them twice. The operator can overrule by moving them into this story.
- Decided: `maintainer` is a noun, not a role flag, because the maintainer is per host (`epic:role-names`). Until `story:role-names-in-files` lands the config says `conductor` and `conductor-dev`; the commands accept both.
