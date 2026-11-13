---
format: aep.planning-md/3
id: story:instance-rules-text
kind: story
status: active
title: An instance mapping carries standing rules text that every session of the instance reads
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 4
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T00:16:24Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-13T00:16:24Z", actor: "human:timo", revision: 4}
---
## Why

Reported by a consumer control plane against v1.5.0 and v1.6.0, and read in the code: nothing a
control plane sends reaches every session of one instance as standing rules.

- A role's text is in the binary (`crates/conductor/src/role.rs`, `crates/conductor/roles/`),
  equal for every instance.
- `roles[].profile` names a file path, and no call puts a file into an instance pod.
- A charter is per repository, and no HTTP call writes one.
- The target repository's own `AGENTS.md` is the only text of a control plane's choosing that a
  session reads, and only where the control plane can write to that repository.

The case: a control plane starts an instance on a public repository it does not own. It needs
one rule in force for every session of that instance (the reported one: no private name in a
commit, a branch, a pull request or an issue) and cannot write to the repository before the
first session runs.

The brief of the work call is no substitute: a session the instance starts without a work call
does not carry it.

## Acceptance

- The instance mapping takes a text member (the story names it by the config domain's
  vocabulary), declared in `spec/`, bounded in size, and a mapping above the bound is refused
  with a declared error at `POST /v1/instances`.
- Every session the instance starts gets that text in its prompt, under a line that says it is
  the instance's standing rules, in a form the text cannot break out of; a test reads the
  prompt file of a controller and of the conductor role. An instance that runs as a pod is one
  of the cases.
- `GET /v1/instances/{name}` answers the text.
- A mapping update that changes only the text replaces no pod; sessions started after it read
  the new text. The story says what a session that already runs reads.
- The member is optional. An instance whose mapping does not hold it behaves as before this
  story: the same prompts, the same pod, and no documented answer of `POST /v1/instances`
  changes for it. A test holds a mapping without the member to the prompt it had.

## Notes

- Text a model reads is no refusal; the refusal is `story:pod-gates-policy`.
- Not decided: whether the text also reaches sessions of roles other than the controller and
  the conductor.
