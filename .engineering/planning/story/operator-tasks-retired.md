---
format: aep.planning-md/3
id: story:operator-tasks-retired
kind: story
status: draft
title: Operator verbs are commands, the Taskfile keeps developer tasks, and the docs never send an operator to task
relations:
- decomposes: epic:binary-only-operator
- depends_on: story:profiles-embedded
- depends_on: story:session-commands-complete
- depends_on: story:dashboard-broker-units
revision: 2
---
# Story: operator-tasks-retired

## Outcome

No operator verb is Taskfile logic: `trust`, `status`, `measure` and `agents:link` have `conductor` commands with the same results, the Taskfile keeps developer tasks plus thin wrappers, and a test fails when an operator page of the docs tells the reader to run `task ...` or to clone the repository.

## Context

- `task trust` already runs `conductor trust` (Taskfile.yml:199-203); `task status` is a snapshot then a board publish (:318-322) while `conductor status` prints the newest snapshot from the store and starts none (crates/conductor/cli.yaml:354-358), so the two are not the same verb today.
- `task measure` runs three `cargo test --release -- --ignored` suites (Taskfile.yml:334-340); that needs a checkout and a toolchain, so the operator's equivalent is a command over what the binary can measure, next to `conductor metric compute` (cli.yaml:366), not the test suites.
- `task agents:link REPO=<path>` links this checkout's `.claude/agents/` files into another repository and excludes them in `info/exclude` (Taskfile.yml:204-252); with no checkout the source is the embedded profile set of `story:profiles-embedded`.
- Docs that tell the reader to run tasks: website/docs/guides/start-conductor.md (:24, :57, :63, :68, :80, :99-101, :104, :180-190), read-the-dashboard.md (:14-15), first-task-existing-repo.md (:89, :141-142), troubleshoot.md (:66, :74, :150, :167), configure-an-instance.md (:116, :131), run-isolated.md (:55, :107), website/docs/concepts/instances-and-config.md (:101, :109), why-conductor.md (:54, :137, :167, :184-185), roles.md (:83) and getting-started.md (:35). Contributor pages are website/docs/develop/ (how-we-work.md, onboarding.md, regenerate-and-check.md), where `task` is the subject.
- The docs tests read a guide and resolve its `conductor <group> <leaf>` commands against the CLI tree (crates/conductor/tests/docs_quickstart.rs, docs_forge.rs, docs_setup.rs); none looks for `task`.
- Shared surface: `Taskfile.yml`, `crates/conductor/tests/taskfile.rs` and the guides with `story:session-commands-complete` and `story:dashboard-broker-units`, which is why this story depends on both.

## Acceptance

1. Spec first: model `conductor.host.LinkAgents` (field `repository`) with refusals (`not-a-repository`, `not-the-top`, `exists-not-a-link`) in `spec/domains/host.yaml`, add `[agents, link]` and `[measure]` to `crates/conductor/cli.yaml`, and extend the status input with `publish`; validated with `ess specify validate --path spec`.
2. `conductor agents link --repository <path>` writes `.claude/agents/<role>.md` links (or copies, where the source is embedded) into the repository and adds each to `info/exclude`, refusing as `task agents:link` does, with a case for each refusal and for the idempotent second run, run with no checkout.
3. `conductor status --publish` takes a snapshot and writes `STATUS.md` in the records directory, as `task status` does; without `--publish` it prints from the store as today.
4. `conductor measure` prints the append cost, guard latency and dashboard answer times it can measure on the installed binary and exits 1 naming a measurement that exceeds its limit; `task measure` keeps the cargo suites for contributors and says so in its `desc`.
5. `conductor trust` is unchanged and `task trust` is a one-line wrapper.
6. `Taskfile.yml` contains no `claude agents`, `jq`, `systemctl`, `nohup` or `git -C` in an operator task; a test in `crates/conductor/tests/taskfile.rs` lists the operator tasks by name and fails on any such word in their commands, and on a task of that list with more than one command line.
7. A test in `crates/conductor/tests` reads every page under `website/docs` except `website/docs/develop/` and fails, naming the page and line, when a line tells the reader to run `task <name>` or to clone the repository (`git clone`, `from a checkout`); the pages above are edited until it passes.
8. `task docs-check` passes and the generated reference lists `agents link` and `measure`.

## Out of Scope

Developer tasks (`check`, `ui`, `ui:dev`, `regen`, `docs-generate`, `docs-check`, `install`) and the contributor pages; removing the wrapper tasks (they stay for one release); the dashboard, session and profile commands (the sibling stories).

## Ambiguities

- Decided: `task status` becomes `conductor status --publish`, a flag on the existing command, so one noun serves both reads; the operator can ask for a separate board command instead.
- Decided: `measure` for an operator measures the installed binary, not the test suites; the parity asked for is the question answered, not the cargo invocation.
- Open: whether a page may name `task` for a developer inside an operator guide (a "from a checkout" aside); as written it may not.
