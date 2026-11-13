---
format: aep.planning-md/3
id: story:instance-secret-409-reads-labels
kind: story
status: draft
title: At a 409 the daemon deletes only an instance Secret whose labels name it
scope:
- confidence: cited
  path: crates/conductor/src/daemon/pods.rs
revision: 2
---
## Why

Read in the code, found while reviewing `story:instance-rules-text`: at a pod's start the daemon
creates the instance's Secret `conductor-instance-<name>`, and where the API answers `409` it
deletes the Secret of that name and creates it again (`crates/conductor/src/daemon/pods.rs`,
`start_once`). It does not read the labels of the Secret it deletes.

- The daemon tells its own objects from another's by the labels `conductor/daemon` and
  `conductor/instance`; the sweep and the write in place read them.
- The delete at a `409` does not. A Secret of that name made by another daemon in the same
  namespace, or by hand, is deleted and replaced.
- The guide says nothing else belongs in the instance pods' namespace, and two daemons with
  different owner labels may share it.

The behaviour predates `story:instance-rules-text`, which added the label check for the write
in place only.

## Acceptance

- Before the delete at a `409`, the daemon reads the Secret of that name. One whose labels name
  this daemon and this instance is deleted and created anew, as today.
- One whose labels name another daemon or instance, or none, is left as it is: nothing is
  deleted, the pod is not started, and the instance's `detail` says that a Secret of that name
  is there that is not this daemon's, naming the Secret and no right.
- The start is tried again when the instance's rendering changes or the foreign Secret is gone,
  not on every pass.
- A case with two owner labels in one namespace holds that neither daemon deletes the other's
  Secret; a case with an unlabelled Secret of the name holds that it stays byte for byte.
- The guide's paragraph on what the daemon deletes says so.

## Also read

- `Pods::sweep` lists this daemon's Secrets, reads each one's instance from the label
  `conductor/instance`, and deletes by the Secret's name where the config holds no such
  instance. A Secret named for a running instance and labelled for one that is gone is deleted
  under the running instance. Found by a case of `story:instance-rules-text`, kept ignored
  there. The story takes it in: a Secret is deleted only where its name is the one its instance
  label gives.

## Open

- Whether the same holds for the pod of that name and the volume claim, which the start also
  creates. The story reads both paths and takes them in if they delete by name alone.
