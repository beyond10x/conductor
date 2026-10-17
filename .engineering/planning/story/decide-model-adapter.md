---
format: aep.planning-md/3
id: story:decide-model-adapter
kind: story
status: draft
title: A decision request is classified in code and class C is answered by one bounded model call through the Decider adapter
relations:
- decomposes: epic:conductor-run
revision: 1
---
## Why

A decision request's class is chosen by the session through a flag
(`crates/conductor/src/decide.rs:239`, `:938`) and the answer is a session turn at hundreds of
thousands of tokens of context. The design (`docs/design/conductor-run.md` § 3) classifies in code
and answers class C with one bounded model call.

## Outcome

`decide::handle_request` classifies a parsed `[DECISION-REQUEST]` by rules in code, answers class C
through a `Decider` trait with a bounded prompt and records the decision, and escalates class O and
H to the Decisions page with options, default and deadline. The first `Decider` is `claude -p`.

## Acceptance

1. Spec first: `spec/domains/decision.yaml` gains the classification rule names
   (`data-loss`, `history-rewrite`, `repository-lifecycle`, `architecture`, `credential`, `hands`)
   on a request and the decider's name and prompt hash on a conductor decision's evidence;
   `ess specify validate` exits 0.
2. `classify(request) -> Class`: a request whose text or options match a rule name is O (H for
   `credential` and `hands`); otherwise C; table-driven tests in
   `crates/conductor/tests/decide_class.rs` with 12 requests from the records' history, scrubbed.
3. `Decider` trait: `decide(prompt) -> Answer { letter, reason }`; `ClaudePrint` runs `claude -p
   --model <roles[conductor].model> --output-format json --setting-sources ""` with the prompt on
   stdin and no tools, parses the answer, refuses one that names no listed option; a `Fake` decider
   for tests. The prompt holds: the request and options, the repository's accepted ADR titles
   (`aep plan artifact list --kind architecture-decision-record --status accepted` in the checkout),
   the role's rules file, the operator's decision principles from `<records>/rules.md` § "How the
   operator decides" (or `rules-v2/all.md` once switched), and the repository's decisions of the
   last 7 days from the decision log; its size is capped at 16k characters and the test holds the
   cap.
4. Class C: the answer is recorded with `record-conductor-decision` (class C, choice, reason,
   evidence `decider=<name> prompt=<sha256>`) and returned for delivery; class O/H: `escalate-request`
   with `--kind one-of`, lettered options from the request, the recommendation from the request's
   "recommendation" line, default = recommendation, deadline = now + 48 h. Tests with the `Fake`.
5. `conductor decision handle --request <file>` exposes it for the daemon and for by-hand use.

## Scope

`spec/domains/decision.yaml`, `crates/conductor-model` (regenerated), `crates/conductor/src/decide.rs`,
new `crates/conductor/src/decide/model.rs`, `crates/conductor/src/cli.rs`/`cli.yaml`,
`crates/conductor/tests/decide_class.rs`, `crates/conductor/tests/decide_model.rs`,
`website/docs/reference/`.
