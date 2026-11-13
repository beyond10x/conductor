---
format: aep.planning-md/3
id: story:config-reference-prints-enum-words-as-written
kind: story
status: draft
title: The config reference prints each enumeration's values as the config file takes them
tags:
- from-finding
revision: 1
---
## Why

The generated config reference prints an enumeration's values as its variant names in lower
case, which is not what the config file takes for a name of more than one word. Read at
`v1.7.0` (`951330f4f4`).

- The page: `model_token_kind` is one of `apikey` and `subscriptiontoken`
  (`website/docs/reference/config.md:251`, `:560`); the prose of the same type says `api_key` and
  `subscription_token` (`config.md:558`).
- The config takes `api_key` and `subscription_token` (`crates/conductor/src/config.rs:803-806`)
  and refuses any other word (`config.rs:4288`, `config.rs:2691-2703`).
- The generator: `Spec::variants` lower-cases each variant name of the specification
  (`crates/conductor-docs/src/config.rs:466-474`), `Spec::shown` joins them for a table cell
  (`conductor-docs/src/config.rs:482-487`), and `kind_line` writes the `One of` line
  (`conductor-docs/src/config.rs:957`). The specification declares `ApiKey`, `SubscriptionToken`
  (`spec/domains/config.yaml:920-922`).
- The same defect, counted: the page has 11 enumerations (its 12 `One of` lines but the union's
  at `config.md:298`). Two have variant
  names of more than one word, and both print words the config refuses:
  - `conductor.config.ModelTokenKind` (`config.md:560`, table `config.md:251`).
  - `conductor.config.OperatorGrant`: the page prints `answerdecisions`, `changesettings`,
    `startinstances`, `stopinstances`, `upgradeinstances` (`config.md:482`, table
    `config.md:52`); the config takes `answer-decisions`, `change-settings`, `start-instances`,
    `stop-instances`, `upgrade-instances` (`crates/conductor/src/config.rs:747-755`), and the
    page's own prose says kebab case (`config.md:480`).
  - The other nine are one word each, where the lower-cased name is the word; checked against
    the config's word table for `RepoCache`, `secret.Source`, `ForgeRoute`, `CatalogNames` and
    `Network` (`config.rs:381`, `:741`, `:798`, `:833`, `:931`); for `Harness`, `tool.Verb`,
    `direction.Activity` and `decision.Decider`: not read.
- The union keys (`github`, `local`, `gitlab`) are lower-cased the same way
  (`conductor-docs/src/config.rs:877-881`); each is one word.

## Acceptance

- The page prints each enumeration's values as the config file takes them: `api_key`,
  `subscription_token`, and the five grants in kebab case.
- A test compares the words the page prints for `ModelTokenKind` and `OperatorGrant` with the
  config's word tables (`MODEL_TOKEN_KINDS`, `OPERATOR_GRANTS`) and fails on a difference.
