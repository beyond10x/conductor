---
title: How changes are planned and shipped
sidebar_position: 2
description: The planning store and a story's lifecycle, waves, how conductor's own behaviour becomes stories, releases, and what may go into public text.
lede: Every change starts as a story in the repository's own planning store, ships in a wave behind one gate, and reaches users in a tagged release.
source: AGENTS.md (Commits, gates and public text; Releases); docs/design/conductor.md section 12; .agents/conductor-dev.md; docs/waves/; .engineering/project.yaml; aep plan artifact --help
---

## The planning store

Conductor's work is planned in an AEP store in the repository itself, under
`.engineering/planning/`: one Markdown file per artifact, with YAML front matter and a free
body. A story's id is its kind and name, `story:<name>`, and its file is
`.engineering/planning/story/<name>.md`. The store is plain files under git, so a status move is a
one-line diff with an author.

Write to it through `aep` rather than by hand: the commands keep the front matter and the
history of every move, and `aep plan artifact validate` reports a file removed by hand as a
deletion no command made. The commands you will use most:

| command | does |
|---|---|
| `aep plan artifact new story <name> --title … --from <file>` | create a draft story with the body in the file |
| `aep plan artifact scope <id> --add <path>` | record a path the story lands on |
| `aep plan artifact show <id>` | print one artifact |
| `aep plan artifact list`, `aep plan artifact board` | the plan, as lines or as status columns |
| `aep plan artifact evidence <id> --kind test_result --source …` | record evidence about an artifact |
| `aep plan artifact move <id> --to <status>` | move it, if its lifecycle permits |
| `aep plan artifact explain <id>` | what each past move was decided on |
| `aep plan artifact validate` | check every file, edge and status |

A story body has three sections: **Why** (what is wrong, with where it was seen), **Acceptance**
(what a test or a reader can check), and **Scope** (the files it lands on). The stories already in
the store are the best examples.

## A story's lifecycle

```text
draft       -> proposed, archived
proposed    -> active, draft, rejected
active      -> implemented, archived
implemented -> archived
rejected    -> archived
```

| status | means |
|---|---|
| `draft` | filed, not yet agreed; `aep plan artifact lifecycle story` prints the moves |
| `proposed` | reviewed and agreed as worth doing |
| `active` | being implemented in a tree |
| `implemented` | merged, with its gate recorded as evidence |
| `archived`, `rejected` | retired; the file and its history stay |

Before a story moves to `implemented`, record its gate on it: `aep plan artifact evidence <id>
--kind test_result --source "task check"` (Commits, gates and public text). `move` finds that
record by itself. A move made only on `--evidence <kind>=<count>` is journalled as an assertion,
and `explain` says so. A story that is no longer wanted is moved to `archived`, never deleted.

## Waves

Stories are delivered in waves. One wave is one round of implementation:

- **one tree per round.** A single managed worktree holds every change of the round; its units
  run one after the other in it.
- **units in sequence.** Each unit is one story (or a part of one), implemented test first and
  committed on its own.
- **review and gate once, before the merge.** `task check` runs in the round's tree, then the
  round merges into `main`. An adversarial or security review is added for large or risky
  changes, and for a guard rule that is loosened.
- **cleaned up.** The tree is finished (`worktree finish --discard-cache --archive`), collected
  with `worktree gc`, and the merged branches are deleted.

`aep plan artifact waves --kind story --status draft` proposes which stories may go together: no
two stories in one wave share a scope path, and a story never runs before one it depends on. A
story with no recorded scope is listed as `unassessed` and never placed, so scope a story before
you plan a wave around it. Each wave gets a page in `docs/waves/`: what was selected and why, the
units with their commits, the decisions taken, and the stories it filed.

## How conductor's own behaviour becomes stories

Conductor runs an organization's sessions; it does not build itself. A second session,
`conductor-dev`, develops it ([design section 12](https://github.com/beyond10x/conductor/blob/main/docs/design/conductor.md#12-developing-conductor-conductor-dev),
profile `.agents/conductor-dev.md`). It reads what conductor did and looks for:

- decisions later reversed, and escalations that turned out to be routine;
- the operator's corrections;
- conductor's failed tool calls, and commands it invented;
- refusals from other sessions, and messages held for the operator;
- requests conductor sends it as `[NEED conductor <id>] conductor-dev <what>`.

Each finding with a cause it can cite becomes a draft story, filed once. From there it follows
the same path as any other change: spec first, a wave, the gate, a merge. After a wave merges and
`task check` is green on `main`, conductor-dev installs the new binary with `task install`, and
restarts conductor only when something conductor reads at session start changed (its profile,
its adapter, the controller profile, the instance's `rules.md`, the design or its settings).

You do not need a conductor-dev session to contribute. A story you file by hand goes through the
same lifecycle.

## Releases

A release is a tag `v<version>` on `main` and a GitHub Release of the same name. Users install
from the tag:

```console
cargo install --git https://github.com/beyond10x/conductor --tag v<version> --locked conductor-cli
```

The version follows semver at `0.x`: a **minor** bump for a new command, config key or record, a
**patch** bump for anything else. A release usually follows a merged wave; several waves may go
into one. The steps (Releases):

1. On a branch from `main`, set `version` in the root `Cargo.toml`, refresh `Cargo.lock` with
   `cargo update -p conductor-cli --offline`, and point the README's install line at the new tag.
2. In a fresh clone of that exact commit, run `task check` with `B10X_GATES_POLICY` set, then
   `task docs-check`; each must exit 0.
3. Merge the branch through a pull request with green checks.
4. Tag the commit, push the tag, and create the GitHub Release. Its notes list the stories
   implemented since the last tag, one line each.
5. Verify: the tag points at `main`'s commit, its checks are green, the Release exists, and
   `cargo install` from the tag into a scratch root installs `conductor-cli` at the new version.

## What goes in public text

The repository is public. Everything committed is public text: code, comments, tests, the
specification, the plan, wave pages, this site and commit messages. So are pull-request and
release bodies. The rules (Commits, gates and public text):

- Committed text names no private repository, no session or decision id, no host detail and no
  person.
- The product names no organization (Rules, "No organization in the product"). Use
  `example-org` and `~/example-org/` in examples, fixtures and tests.
- Every pull-request and release body passes `b10x-gates scan-text` before it is sent.
- A `task check` that printed "the Gates scan is skipped" ran no secret scan and is no gate.
- Session hand-overs hold session ids and host state, so they never go into this repository;
  a wave page keeps only what the product's history needs.

:::note[Commits from the project's agents]
Maintainers and the project's agents send every commit, push, pull request and tag through the
bot route, `b10x-gates bot -- <git args>`, and every GitHub API write through `b10x-gates api`.
Each session commits only the files it wrote, with a pathspec.
:::
