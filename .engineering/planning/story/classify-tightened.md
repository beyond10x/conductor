---
format: aep.planning-md/3
id: story:classify-tightened
kind: story
status: draft
title: Classification rules match an action on an object; 30 fixture requests, 8 of them class C the word rules escalated
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

`decide::classify` (`story:decide-model-adapter`) matches single words: "token" anywhere makes a
request class H, "delete the merged branch" makes it class O. The operator's first complaint about
conductor was being asked "completely obvious" decisions (2026-10-14). A rule that over-escalates
reproduces it through the Decisions page.

## Outcome

Each classification rule matches an action on an object, not a word: data loss = a removal verb
(delete, erase, purge, drop, wipe, rm) on another's work, an archive, a store or a repository, and
not on a build directory, a cache, a merged branch or a scratch path; history rewrite = force-push,
filter-branch, rebase or amend of a pushed or published ref; repository lifecycle = create, rename,
archive or retire a repository; architecture = accept, change or write an ADR, a cross-repository
contract or a language choice; credential = set, enter, rotate, create or store a secret, token,
key, password or login, and not reading, counting or mentioning tokens; hands = a permission,
account or payment only the operator holds. A request whose recommendation is an option it names
"reversible", "can be undone" or "revertible" and that matches no rule is class C.

## Acceptance

1. The fixture set in `crates/conductor/tests/decide_class.rs` grows to 30 requests: the 12
   existing, plus 18 shaped after real requests of 2026-10-14 to 2026-10-17 (scrubbed of ids and
   names), of which at least 8 are class C requests that the old rules escalated: build-output
   deletion, merged-branch deletion, a KPI about tokens, a "token budget" wording, a scratch path
   cleanup, a dependency upgrade, a wave approval, a release go. Every row names its expected class
   and rule, and all 30 pass.
2. `classify` returns the matched phrase with the rule, and `conductor decision handle` prints it;
   the escalation's reason names it.

## Scope

`crates/conductor/src/decide.rs` (`classify`), `crates/conductor/src/decide/model.rs`,
`crates/conductor/tests/decide_class.rs`.
