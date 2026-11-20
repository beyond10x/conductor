---
format: aep.planning-md/3
id: story:guard-repo-slug-not-program
kind: story
status: active
title: 'Guard: a repository slug ending in /conductor is no conductor run'
summary: conductor_run reads an argument word whose basename is conductor as the program, so the bot route's run rerun of this repository is denied as conductor run
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T01:21:17Z", actor: "human:unknown", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-20T01:21:19Z", actor: "human:unknown", revision: 4}
---
## Problem

`b10x-gates gh --repository beyond10x/conductor -- run rerun <id> --failed`, the bot route's own
form for re-running a failed workflow run, is denied: "`conductor run` writes conductor's records;
only conductor runs it". `conductor_run` (crates/conductor/src/guard/rules.rs, `fn conductor_run`)
takes `shell::program(word) == "conductor"` for any word, so the slug `beyond10x/conductor`, an
argument and not the program, is read as a conductor invocation and the next word `run` as its leaf
(`leaf_after(&words[at..], false, false)` in the not-a-program-word branch).

## Outcome

A word in argument position is a conductor invocation only when it is a path to the program, not a
`<owner>/<repo>` slug the command takes as a value (`--repository`, `-R`, `--repo`); the denial of a
real `conductor run`, by name or path, stands.

## Acceptance (fails on main)

- The guard allows `b10x-gates gh --repository example-org/conductor -- run rerun 1 --failed`.
- It still denies `conductor run`, `~/.cargo/bin/conductor run` and `env X=1 conductor run --once`.

## Boundaries

Guard rules only; latency stays within `tests/guard_latency.rs`.
