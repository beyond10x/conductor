---
format: aep.planning-md/3
id: story:pod-gates-policy
kind: story
status: active
title: A control plane gives a pod instance a gates policy, and a matching commit or push is refused
relations:
- decomposes: epic:deployable-in-kubernetes
scope:
- confidence: inferred
  path: crates/conductor-docs/src/config.rs
- confidence: cited
  path: crates/conductor-model
- confidence: cited
  path: crates/conductor/http.yaml
- confidence: cited
  path: crates/conductor/src/collect/checkouts.rs
- confidence: cited
  path: crates/conductor/src/config.rs
- confidence: cited
  path: crates/conductor/src/config/pod.rs
- confidence: cited
  path: crates/conductor/src/daemon/api.rs
- confidence: cited
  path: crates/conductor/src/daemon/pods.rs
- confidence: inferred
  path: crates/conductor/src/dashboard/health.rs
- confidence: inferred
  path: crates/conductor/src/doctor.rs
- confidence: inferred
  path: crates/conductor/src/run/records.rs
- confidence: inferred
  path: crates/conductor/src/secret.rs
- confidence: cited
  path: crates/conductor/src/spec_api.rs
- confidence: inferred
  path: crates/conductor/tests
- confidence: cited
  path: spec/domains/config.yaml
- confidence: cited
  path: spec/domains/instance.yaml
- confidence: cited
  path: website/docs/guides/run-in-kubernetes.md
- confidence: inferred
  path: website/docs/reference/config.md
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T00:16:24Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-11-13T00:16:25Z", actor: "human:timo", revision: 4}
---
## Why

Reported by a consumer control plane against v1.5.0 and v1.6.0, and read in the code and the
guide: an instance that runs as a pod has no refusal of a private name at commit or push.

- `gates_policy` of an instance names a file path; no call puts a policy file into an instance
  pod.
- With `forge.route: direct`, which the Kubernetes guide's examples use, nothing scans a commit
  or a push.
- `forge.route: gates` in an instance pod is shown nowhere: no test and no run on a cluster.

The case: an instance on a public repository. A rule that is only text can be ignored by a
model; a public repository needs the refusal of a scan. Until this exists, such an instance
relies on text alone, and the guide should say so as a limit.

## Acceptance

- The instance mapping carries a gates policy a control plane can give without a file on the
  pod: the story decides between the policy inline, a `secret:<name>`, and a name the daemon
  mounts, and says why. It is declared in `spec/`.
- An instance pod with `forge.route: gates` and that policy refuses a commit and a push that
  match a rule of the policy; a test on a cluster with a forge double shows both refusals and
  one push that passes.
- The refusal names the rule and never the matched text: the pod's log, the health answer and
  the work row are read in the test for the matched text and hold none.
- A policy the instance cannot read stops sessions from pushing and is a member failure in
  `GET /v1/instances/{name}/health`; it is not a silent fall back to `direct`.
- `website/docs/guides/run-in-kubernetes.md` says, under Limits, what an instance without a
  policy relies on.
- The policy is optional. An instance whose mapping holds none behaves as before this story:
  `forge.route: direct` pushes as it does today, the pod is the same, and no documented answer
  of `POST /v1/instances` changes for it. A test holds a mapping without a policy to the pod it
  had.

## Notes

- After `story:instance-rules-text` in the reporter's order.
- The release image holds `b10x-gates`; which policy format and version the pod's copy reads is
  for the story to state.
