# Wave 47 (controllers retry their own pipelines)

Under `forge.route: direct` a controller on a GitLab instance could open and merge its own merge
requests, but the guard denied `glab ci retry`, `glab ci run` and a pipeline start through `glab
api`. On 2026-11-07 three controller requests on two instances asked a person to start or retry a
pipeline, and each controller waited until one did. One tree, one release.

## Units

| unit | story | note |
|---|---|---|
| U1 | guard-admits-own-pipelines | under `forge.route: direct` the guard admits `glab ci retry\|run` of the session's own repository, named with `-R` or by the checkout `glab` runs in, and `glab api -X POST` to `projects/<own path>/pipeline`, `pipelines/<id>/retry` and `jobs/<id>/retry`; the same naming another repository, another host or another endpoint stays denied |

A numeric project id (`projects/42/…`) is denied: the guard cannot tell whose project it is
without a forge call, and the guard answers without one (`AGENTS.md` rule 6). `:fullpath` and
`:id`, which `glab` fills from the checkout it runs in, are admitted.

## Metric

Guard denials of a session's own pipeline retry or start (`conductor guard guard-decisions`, a
`target` matching `glab … ci retry|run` or `glab api … pipeline`), summed over the GitLab
instances on the host.

| | before (2026-11-08) | after |
|---|---|---|
| denials recorded, all time | 9 (7 and 2 on two instances) | |
| new denials in the 7 days after the release | | |
