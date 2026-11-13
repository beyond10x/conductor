---
format: aep.planning-md/3
id: story:instance-git-identity
kind: story
status: active
title: An instance sets the git identity its pod commits with
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-11-13T01:11:51Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-13T01:11:52Z", actor: "human:timo", revision: 3}
---
## Why

Read in the code and the Kubernetes guide at v1.6.0: every instance pod commits as
`conductor <conductor@localhost>`. The four variables `GIT_AUTHOR_NAME`, `GIT_AUTHOR_EMAIL`,
`GIT_COMMITTER_NAME` and `GIT_COMMITTER_EMAIL` are fixed in
`crates/conductor/src/daemon/pods.rs`, and no member of the instance mapping changes them.

A repository whose rules admit only commits of one author and committer e-mail refuses every
push of such a pod, whatever forge credential the pod holds. A forge that ties a commit to an
account by its e-mail shows the pod's commits as nobody's.

## Acceptance

- The instance mapping takes an optional git identity, a name and an e-mail (the story names
  the member by the config domain's vocabulary), declared in `spec/`. Each is one line, bounded
  in size, without a control character, `<`, `>` or a newline; a mapping that breaks the rule is
  refused with a declared error at `POST /v1/instances` and `PUT /v1/instances/{name}`.
- A pod of an instance that holds the member gets the four variables from it. A session's
  commit in a checkout of that pod carries the name and the e-mail as author and as committer;
  a test reads both from a commit made in the pod's environment.
- The commits conductor makes in the instance's records carry the same identity, or the story
  says why they keep the fixed one and the guide says which commits carry which.
- `GET /v1/instances/{name}` answers the identity.
- A mapping update that changes only the identity: the story says whether the pod is replaced,
  as for any other change of the rendering, and a test holds it.
- The member is optional. An instance whose mapping does not hold it behaves as before this
  story: the four variables keep `conductor` and `conductor@localhost`, the pod is the same,
  and no documented answer of `POST /v1/instances` changes for it. A test that failed before
  the change holds a mapping with the member; a second test holds a mapping without it to the
  pod it had.
- `website/docs/guides/run-in-kubernetes.md` names the member where it lists the pod's
  environment.

## Notes

- Which identity an instance carries is the operator's choice per instance; the story sets no
  default other than today's.
- Not in this story: where the forge credential comes from, and an instance that runs outside
  a pod (the story says what the member does there: applied, or refused as not supported).
