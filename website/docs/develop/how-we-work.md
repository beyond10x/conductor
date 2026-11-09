---
title: How changes are planned and shipped
sidebar_position: 2
description: The planning store, a story's lifecycle, waves, the maintainer, the release steps and public text.
lede: Every change starts as a story in the repository's own planning store, ships in a wave behind one gate, and reaches users in a tagged release.
source: AGENTS.md (Commits, gates and public text; Releases); docs/design/conductor.md section 12; crates/conductor/roles/maintainer.md; crates/conductor/src/config.rs; crates/conductor/src/daemon/reconcile.rs; docs/waves/; aep plan artifact --help
---

## The planning store

Conductor's work is planned in an AEP store under `.engineering/planning/`: one Markdown file per
artifact, with YAML front matter and a free body. The story `story:<name>` is the file
`.engineering/planning/story/<name>.md`. A status move is a one-line diff with an author.

Write to the store with `aep plan artifact` (`new`, `scope`, `show`, `list`, `board`, `evidence`,
`move`, `explain`, `validate`), not by hand: `validate` reports a file removed by hand as a
deletion no command made. A story body has three sections: Why (what is wrong, and where it was
seen), Acceptance (what a test or a reader can check) and Scope (the files it lands on).

## A story's lifecycle

```text
draft       -> proposed, archived
proposed    -> active, draft, rejected
active      -> implemented, archived
implemented -> archived
rejected    -> archived
```

`aep plan artifact lifecycle story` prints these moves. Before `implemented`, the story's gate is
recorded with `aep plan artifact evidence <id> --kind test_result`. A story no longer wanted is
moved to `archived`, not deleted.

## Waves

`aep plan artifact waves --kind story --status draft` proposes which stories may go together: no
two share a scope path, and none runs before one it depends on. A story with no recorded scope is
listed as `unassessed` and is not placed.

A wave is one round in one managed worktree, its units implemented test first and committed one
by one. The gate is the pull request's `task check` job and Gates job. A large or risky change, or
a loosened guard rule, also gets an adversarial or security review. The round ends with
`worktree finish --discard-cache --archive` and `worktree gc`, and gets a page in `docs/waves/`.

## The maintainer

The maintainer is the role that develops conductor
([design section 12](https://github.com/beyond10x/conductor/blob/main/docs/design/conductor.md#12-developing-conductor-conductor-dev)),
with the profile `conductor role show --role maintainer` prints. The config writes the role as
`maintainer`, and its old name `conductor-dev` is read for this release with a note. The session
is named after the role as the file writes it: `<session_prefix>-maintainer`, or
`<session_prefix>-conductor-dev`, both accepted for this release. `task maintainer:start` starts
it.

The maintainer reads `conductor status`, the `dispatches/` and `decisions/` logs and
`conductor metric kpi-readings`. A reversed decision, an escalation that was class C or a failed
dispatch with a cause it can cite becomes one draft story.

After a wave merges, `task install` installs the new binary on the `PATH`. A running instance stays
on the binary conductord pinned for it until `conductor instance upgrade --tag v<version>`, which
rolls back unless the runtime answers within `daemon.upgrade_timeout`. A story filed by hand, with
no maintainer session, goes through the same lifecycle.

## Releases

A release is a tag `v<version>` on `main` and a GitHub Release of the same name
([`AGENTS.md`](https://github.com/beyond10x/conductor/blob/main/AGENTS.md#releases)):

1. On a branch from `main`, set `version` in the root `Cargo.toml`: at `0.x`, a minor bump for a
   new command, config key or record, a patch bump otherwise. Refresh `Cargo.lock` with
   `cargo update -p conductor-cli --offline`. Point the install lines of the README and
   `website/docs/getting-started.md` (`cargo install --tag` and the archive download) at the new
   version.
2. The `task check` job and the Gates job are green on that exact commit, on the pull request of
   step 4. Read each job's own conclusion. Only where CI cannot run: `task check` in a fresh clone
   at that commit with `B10X_GATES_POLICY` set, exit 0.
3. When the release changes anything onboarding runs (`SETUP.md`, the README pointer to it,
   `conductor setup`, `conductor doctor`, the release archives or the prerequisites), repeat both
   onboarding runs of `docs/analysis/onboarding-run.md` on that commit and append them there. One
   repository has a GitHub `origin`, the other a self-hosted GitLab `origin`, each with a planning
   store one format old. Both end `ready`, with questions only for logins and the store
   migration. A gap either run hits is fixed on the branch and the runs repeated.
4. Open a pull request through the bot route, and fast-forward `main` to it once it is green.
5. `b10x-gates bot -- tag -a v<version> -m <title> <commit>`, then
   `b10x-gates bot -- push origin v<version>`. Create the GitHub Release through `b10x-gates api`
   (`POST /repos/beyond10x/conductor/releases` with `tag_name`, `name` and a `body` that lists the
   stories implemented since the last tag, one line each). The tag push started the Release
   workflow, which attaches the archives when its builds finish. If it finishes first, it creates
   the Release with the tag's message as notes, and the body goes on with
   `PATCH /repos/beyond10x/conductor/releases/<id>`. A failed run is re-run, or an existing tag's
   archives backfilled, with `workflow_dispatch` and `tag: v<version>`.
6. Verify that the tag points at `main`'s commit, the tag's checks and the Release workflow's run
   are green, and the Release lists five assets: the four archives and `SHA256SUMS`. An archive
   downloaded from it passes `sha256sum --check` against `SHA256SUMS`, and its `conductor --help`
   exits 0. `cargo install` from the tag with `--root <scratch dir>` succeeds, and
   `cargo install --list --root <scratch dir>` lists `conductor-cli v<version>`.

## What goes in public text

The repository is public, and so are pull-request and release bodies.

- Committed text names no private repository, no session or decision id, no host detail and no
  person.
- The product names no organization (`AGENTS.md` Rules, "No organization in the product"). Use
  `example-org` and `~/example-org/` in examples, fixtures and tests.
- Every pull-request and release body passes `b10x-gates scan-text` before it is sent.
- Session hand-overs, which hold session ids and host state, stay out of this repository.

:::note[Commits from the project's agents]
Maintainers and the project's agents send every commit, push, pull request and tag through the
bot route, `b10x-gates bot -- <git args>`, and every GitHub API write through `b10x-gates api`.
Each session commits only the files it wrote, with a pathspec.
:::
