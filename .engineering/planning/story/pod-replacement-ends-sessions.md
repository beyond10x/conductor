---
format: aep.planning-md/3
id: story:pod-replacement-ends-sessions
kind: story
status: draft
title: A change of an instance's mapping does not end its sessions without saying so
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read on 2026-11-12 at v1.4.0: every change of an instance's mapping that renders differently
replaces its pod (`crates/conductor/src/daemon/pods.rs`, the digest over the pod spec, the
rendered config file and the secret names), within one tick of the `PUT`. Agent sessions are
processes of the pod and end with it; nothing resumes them (`spawn::resume` answers nothing for
a session that is not live). The home directory of the pod's user is outside the claim's mount,
so the harness's own session state is not kept either (inferred from the image's `HOME` and the
mount path; not run). After the new pod starts, the loop's first cycle tick starts new
controllers from their charters. So adding or removing one repository of an instance's source,
with `exclude` or `include`, ends every session of that instance.

A consumer control plane whose records change which repositories an instance serves asked what
a `PUT` does to running sessions (2026-11-12).

## Outcome

A change of an instance's mapping that does not need a new pod does not end the sessions that
run in it; a change that does need one says so before it is made, and sessions that can be
resumed are.

## Acceptance

- A `PUT` that changes only which repositories a source includes or excludes reaches the
  running instance without replacing its pod, or the story says why it cannot and the guide
  says which changes replace a pod.
- A caller can ask whether a `PUT` would replace the pod before sending it, or the `PUT`'s
  answer says that it did.
- After a pod was replaced, a session that ran in the old pod and can be resumed is resumed in
  the new one; what cannot be resumed is reported per session.

## Open with the operator

- Whether the harness's session state belongs on the claim (it then survives a pod and grows
  the claim), which this story needs for any resume.
