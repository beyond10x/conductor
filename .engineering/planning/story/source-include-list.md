---
format: aep.planning-md/3
id: story:source-include-list
kind: story
status: draft
title: A forge source names the repositories it stands for
relations:
- decomposes: epic:deployable-in-kubernetes
revision: 1
---
## Why

Read on 2026-11-12 at v1.4.0: a `github` source names an owner and a `gitlab` source names a
group; `exclude` on a `gitlab` or `local` source removes repositories by leading path steps.
There is no way to say "these N repositories and no others". A caller that is asked for N
repositories of a group has to list every other project of the group under `exclude`, and a
project added to the group later is in scope until the caller writes again.

A consumer control plane that holds "the repositories an instance works on" as a list of
projects asked for this on 2026-11-12.

## Outcome

A forge source can name the repositories it stands for; every other repository of the owner or
group is not the instance's.

## Decisions (2026-11-12, reversible)

- One new key `include: [<path under the owner or group>]` on `github` and `gitlab` sources,
  matched as `exclude` is: by leading path steps, no wildcard, so an entry may name a subgroup.
  Not a new source kind: the source's `gitlab` and `host` keys keep their meaning.
- `include` and `exclude` on one source do not validate; nor does `include` on a `local`
  source, nor an empty `include` list.
- Without `include` a source behaves as in v1.4.0.

## Acceptance

- A source with `include` yields the repositories under its entries and no others: rows,
  merge requests, pipelines, releases and every other collected fact of a repository outside it
  are absent, as for an excluded one.
- A project added to the group later is not the instance's until an entry covers it.
- An entry no repository of the forge answers to is reported per entry (`conductor doctor` and
  where a control plane reads it), and does not fail the source.
- The instance routes accept and echo `include`; the refusals above answer `422` with the YAML
  path of the key.
- The specification declares the key; the generated reference and the GitLab guide say it.
