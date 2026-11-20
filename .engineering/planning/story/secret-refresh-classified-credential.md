---
format: aep.planning-md/3
id: story:secret-refresh-classified-credential
kind: story
status: implemented
title: A request to refresh or update secrets is class H, rule credential
summary: The credential rule's verbs take refresh, update, replace, reissue and upload, so decision handle escalates such a request as class H
refs:
- provider: github
  reference: beyond10x/conductor#246
relations:
- decomposes: epic:architecture-next
- informed_by: story:classify-tightened
scope:
- confidence: cited
  path: crates/conductor/src/decide/handle.rs
- confidence: cited
  path: crates/conductor/tests/decide_class.rs
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-11-20T14:13:22Z", actor: "human:unknown", revision: 5}
- {from: "proposed", to: "active", at: "2026-11-20T14:13:23Z", actor: "human:unknown", revision: 6}
- {from: "active", to: "implemented", at: "2026-11-20T17:26:43Z", actor: "human:unknown", revision: 7, decided_on: {"recorded":{"test_result":1}}}
---
## Why

Issue #246 (filed against 1.19.2; unchanged on main at 1.20.1). `conductor decision handle
--request <file> --decider fake` classified a `[DECISION-REQUEST]` that asked a person to change a
private policy repository and refresh the organization's secrets as class C, rule `none`, and the
decider answered it. A request to set a credential is class H and escalated, never answered by a
decider (design § 5).

Read in the code on main:

- The `credential` rule is a verb of `KEEP_SECRET` followed within 4 words by an object of `SECRET`
  (`crates/conductor/src/decide/handle.rs:667-677`). `KEEP_SECRET`
  (`decide/handle.rs:421-462`) holds set, enter, rotate, create, store, add, paste, provide,
  generate, mint, save, renew, revoke, reset, change, put, give, but not `refresh`, `update`,
  `replace`, `reissue` or `upload`: "refresh the organization secrets" matches no rule.
- `classify` then answers class C, rule `none` (`decide/handle.rs:785-805`), and `handle_request`
  answers class C through the `Decider` adapter (module documentation,
  `decide/handle.rs:31-45`).

## Outcome

- The `credential` rule's verbs (`KEEP_SECRET`) also hold `refresh`, `refreshes`, `refreshing`,
  `update`, `updates`, `updating`, `replace`, `replaces`, `replacing`, `reissue`, `reissues`,
  `re issue`, `re issues` (as `re-issue` reads), `upload`, `uploads` and `uploading`; its objects
  keep their exclusions (`NOT_SECRET`: token budgets, counts, usage, login pages), so "update the
  token budget" stays class C.
- A request whose question or lettered options ask to refresh or update secrets is class H, rule
  `credential`, its phrase the matched words; `decision handle` escalates it as class H and records
  no conductor decision, as any credential request.
- The module documentation's rule list (`decide/handle.rs:12-30`) names the new verbs.

Spec: no change; `ClassificationRule` and `DecisionClass` keep their values.

## Acceptance (tests that fail on the current code)

- `tests/decide_class.rs` `a_request_to_refresh_organization_secrets_is_credential`: the request
  "`[DECISION-REQUEST example-repo req-41] Ask a person to change the private policy repository and
  refresh the organization secrets?`" with two lettered options and a recommendation is class H,
  rule `credential`, phrase `refresh the organization secrets`.
- `tests/decide_class.rs` `each_added_credential_verb_on_a_secret_is_class_h`: one request per
  verb, `refresh`, `update`, `replace`, `reissue`, `re-issue` and `upload`, each with the object
  `the deploy key`; each is class H, rule `credential`.
- `tests/decide_class.rs` `decision_handle_escalates_a_secret_refresh_and_the_decider_answers_none`:
  `decision handle --decider fake` on the first request prints rule `credential`, leaves the
  request escalated class H and records no conductor decision.
- `tests/decide_class.rs` `updating_a_token_budget_stays_class_c` (new; passes on today's code, it
  guards the exclusions).
- `tests/decide_class.rs` `each_of_the_thirty_requests_has_the_class_rule_and_phrase_of_its_row`
  passes unchanged.

## Boundaries

- The rules still read only the question and the lettered options
  (`story:classify-tightened`); a credential named only in another line of the request is not
  read, as today.
- A verb on a credential's name that is not the credential is read as one, as `change` already
  is: "update the deploy key docs" becomes class H. Over-escalating to H is the safe side; the
  matcher's window (4 words) and `NOT_SECRET` are unchanged.
- No other verb (`regenerate`, `recreate`) is added: the issue and the fixtures name none.
- A change to a private repository is no rule of its own here: what makes this request class H is
  its credential.
- `--input-json` of the decision commands is `story:decision-request-input-json`.
