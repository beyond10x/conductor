---
format: aep.planning-md/3
id: story:decision-request-input-json
kind: story
status: implemented
title: raise-decision-request and escalate-request read --input-json
summary: The two decision commands read their input fields from one JSON object on standard input; every other command still answers not implemented
refs:
- provider: github
  reference: beyond10x/conductor#246
relations:
- decomposes: epic:architecture-next
- informed_by: story:secret-refresh-classified-credential
scope:
- confidence: cited
  path: crates/conductor/src/decide.rs
- confidence: cited
  path: crates/conductor/tests/decision.rs
- confidence: cited
  path: docs/design/conductor.md
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T14:13:24Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-20T14:13:26Z", actor: "human:unknown", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-20T17:26:43Z", actor: "human:unknown", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #246, its second part (filed against 1.19.2; unchanged on main at 1.20.1): a controller
raising or escalating a request by hand reached for `--input-json -`, which every command's
`--help` offers ("Read the input fields from standard input, as one JSON object"), and got `not
implemented`.

Read in the code on main:

- `decision raise-decision-request` refuses `--input-json` before anything else
  (`crates/conductor/src/decide.rs:205-208`, `flags_only` at `decide.rs:2657-2662`, exit 2 through
  `crate::not_implemented`, `crates/conductor/src/lib.rs:91-93`); `decision escalate-request` the
  same (`decide.rs:1178`). The design lists `--input-json -` as not implemented on every command
  (`docs/design/conductor.md:494`).
- The issue's other point, that `--help` names the shape of `--lettered-options` nowhere, does not
  hold on main nor on 1.19.2: the flag's help is "list of RequestOption, a JSON array of `{letter,
  label, cost}`; `Y` and `N` for `yes-no`" (`crates/conductor/src/cli.rs:4219-4222`, unchanged
  since before 1.19.2), and an option missing a field is refused naming the three
  (`decide.rs:1496-1522`). Nothing of it is changed here. What is changed for `escalate-request` is
  that its whole input, the options array with it, can be given as one JSON object instead of a
  JSON string inside a flag, the same remedy as for `raise-decision-request`.

## Outcome

- `decision raise-decision-request --input-json -` and `decision escalate-request --input-json -`
  read the command's input from one JSON object on standard input, keyed by the specification's
  input field names (`spec/domains/decision.yaml`, `RaiseDecisionRequest` and `EscalateRequest`):
  `request_id`, `repository`, `question`, `options`, `recommendation`, `raised_at`, `blocker`,
  `classification_rule`; and `request_id`, `escalation_class`, `reason`, `dry_run`, `no_dry_run`,
  `kind`, `lettered_options` (an array of `{letter, label, cost}`), `recommended_letters`,
  `default_letters`, `deadline`; and `actor` for both, in the forms `--actor` takes (its default
  when absent). A value is what the flag of that field takes, in the flags' spellings (`O`,
  `one-of`, the rule names), and passes the same checks the flags' parsers make (an RFC 3339
  `raised_at` and `deadline`, the allowed values, `lettered_options` a JSON array, re-encoded to
  the text the flag takes); `dry_run` is the dry run's text, not a file.
- The input then takes the flags' path: the same checks, refusals, store writes and records lines,
  so a request raised from JSON and one raised from flags leave the same row and line.
- Refused before the store opens, one line on standard error, exit 1: a field flag given beside
  `--input-json`, `--actor` included (naming the flag), a key that is no input field (naming it),
  a value of the wrong JSON type (naming its key), standard input that is no JSON object; a
  missing field is named by its key.
- Every other command keeps answering `--input-json` not implemented (exit 2);
  `docs/design/conductor.md` names these two as the exceptions.

Spec: no change. The input fields are the specification's; the generated input types are built from
them in the handler, as from the flags.

## Acceptance (tests that fail on the current code)

- `tests/decision.rs` `raise_decision_request_reads_its_input_from_input_json`: a request raised with
  `--input-json -` holding all eight fields shows in `decision requests` with the same fields as one
  raised from the same values as flags in a second store, and the decision log holds its
  `requested` line.
- `tests/decision.rs` `escalate_request_reads_lettered_options_from_input_json`: a class O
  escalation with three options from JSON shows them in `decision requests`.
- `tests/decision.rs` `input_json_beside_a_field_flag_or_with_an_unknown_key_is_refused_naming_it`:
  exit 1, naming `--question`, then naming the key `questions`; no store is created.
- `tests/decision.rs` `input_json_that_is_no_object_a_wrong_type_or_a_missing_key_is_refused_naming_it`:
  exit 1 for `[]` on standard input; for `"raised_at": 5`, naming `raised_at`; for an object
  without `question`, naming `question`; no store is created.
- `tests/decision.rs` `another_command_s_input_json_still_answers_not_implemented` (passes today;
  it guards the rule): `decision answer-request --input-json -` exits 2.

## Boundaries

- `--input-json` of any other command, and a shared reader for all of them, are later work: the
  reader of these two commands stays in `decide.rs`, and each other module keeps its own
  `flags_only` (`controller.rs:703-709` and the like).
- `crates/conductor/src/lib.rs` (`not_implemented`) is not changed.
- `crates/conductor/src/cli.rs` is not changed: the flags and their help stay as they are.
- The classification of a secret refresh is `story:secret-refresh-classified-credential`.
