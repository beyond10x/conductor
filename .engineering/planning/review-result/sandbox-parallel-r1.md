---
format: aep.planning-md/3
id: review-result:sandbox-parallel-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, sandbox
relations:
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
needs-revision

Seven collisions or undeclared surfaces need a change in the bodies. The chain inside the set is sound. `story:session-limits-systemd` depends on `story:session-envelope-seam`, `story:session-writes-bwrap` on limits, and `story:session-forge-credentials` on writes. That orders every pair inside the set, and the bodies name the order. The gaps are against wave 17 and in undeclared scope.

- `story:session-writes-bwrap` — lands on `README.md`, `docs/design/conductor.md` and `website/docs/security/the-guard.md`, all three of which `story:gitlab-guide` also lands on (cited, both scopes). `gitlab-guide` says it is "the only story on the hand-written pages". Writes has no `depends_on` edge to `gitlab-guide` and does not name it, and the seam chain does not reach `gitlab-guide`. `gitlab-guide`'s new `docs_forge.rs` test also reads those pages. Fix: add the edge with the shared pages as its reason, or split the pages. — .engineering/planning/story/session-writes-bwrap.md:81-88, .engineering/planning/story/gitlab-guide.md:74-78
- `story:session-envelope-seam` — its spec change regenerates `crates/conductor-model` (cited, both scopes). `story:gitlab-repositories` and `story:gitlab-workspace` do the same from `config.yaml`. `PLAN.md` and `plan.json` there carry whole-spec digests, and `task check` diffs the whole tree (`Taskfile.yml:316`). Seam has no edge to either story and its Notes do not name them. Its edges reach only `gitlab-source-host`, through `gh-optional`. Fix: add the edges, or split the surface. — .engineering/planning/story/session-envelope-seam.md:98-116
- `story:session-envelope-seam` — it lands on `crates/conductor/tests/watch.rs` (the Taskfile reader at `tests/watch.rs:2352-2501`), which `story:watch-gitlab-ci` also lands on (cited, both scopes). No edge runs between them and neither body names the other. The wave-17 tree already carries uncommitted edits to that file, so the collision is live. Fix: add an edge from seam to `watch-gitlab-ci` with this file as its reason, or split. — .engineering/planning/story/session-envelope-seam.md:48, .engineering/planning/story/watch-gitlab-ci.md:37-39
- `story:session-envelope-seam` — its Notes list `doctor.rs`, `tests/doctor.rs`, `Taskfile.yml`, `tests/taskfile.rs` and `.agents/conductor.md` as shared with wave 17. The edges to `version-flag` and `doctor-minimum-versions` actually cover `cli.rs`, `cli_tree.rs`, `cli.yaml` and the model. No wave-17 story lists `Taskfile.yml` or `tests/taskfile.rs`, so those two names are wrong. Fix: correct the Notes to the real shared set. — .engineering/planning/story/session-envelope-seam.md:104
- `story:session-envelope-seam` — its scope omits `website/docs/reference/cli.md`. The new `[session, start]` binding and the `cli.rs` change regenerate it, and `task docs-check` (`Taskfile.yml:327-330`) fails on a stale page. `doctor-minimum-versions` and `version-flag` both list it (cited, `crates/conductor-docs/src/cli.rs:9`). Fix: add the page to the scope. — .engineering/planning/story/session-envelope-seam.md:108-116
- `story:session-envelope-seam` — its scope omits three hand-written pages that go stale when the start path moves. `website/docs/security/confinement.md:14-23` states the `claude --bg` start form and says `task conductor:start` runs it. `website/docs/guides/start-conductor.md:94,131` show the same forms. `docs/config.md:89-90,130-135` marks `confinement.mode` and `provider` as "nothing yet" and says "nothing starts a session in a box yet". Only `story:session-writes-bwrap` lists `docs/config.md`, and no story lists the other two pages. Fix: add them to the scope, or say whose they are. — .engineering/planning/story/session-envelope-seam.md:108-116
- `story:session-limits-systemd` — its scope omits `docs/config.md`. Rows 84-86 and 91-93 (the `limits.*` keys) read "nothing yet" under "Read by", and that is false once this story lands. Writes lists `docs/config.md`, so the two collide on a file that limits does not declare. Fix: add the file and name the order. — .engineering/planning/story/session-limits-systemd.md:63-70
- `story:session-forge-credentials` — it adds `roles[].forge_token_env`, but its scope omits the hand-written `docs/config.md`, which holds the key table. Writes already lists that file. Fix: add the file. — .engineering/planning/story/session-forge-credentials.md:63-70
- `story:session-forge-credentials` — it depends on `story:gitlab-workspace` and says nothing about why. It shares `config.yaml`, the model, `reference/config.md` and `config.rs` with the `gitlab-*` stories and `gh-optional`. It shares `doctor.rs` and `tests/doctor.rs` with `gh-optional`, `doctor-verdict` and `doctor-minimum-versions`. Transitive edges through seam and `gitlab-workspace` order all of these (cited), but its Notes name none of them. Fix: state the shared files beside the edge. — .engineering/planning/story/session-forge-credentials.md:57-61

What I read: 4 set stories and 8 wave-17 stories in full, plus `epic:forge-neutral`. Commands run:
- `aep plan artifact waves --kind story --status draft`
- `aep plan artifact show` on each story
- `aep plan artifact graph`
- `git grep` of the tree: `Taskfile`, `claude --bg`, `docs/config.md` and the `conductor-docs` generators

Surfaces: 4 items cited, 0 inferred, 0 unplaceable. I traced 6 acceptance reads to a producer, and a `depends_on` edge records all 6:
- limits reads the provider trait and the doctor listing from seam;
- writes reads limits' unit and slice;
- forge reads writes' bwrap layer;
- seam reads `gh-optional`'s and `doctor-verdict`'s doctor rows;
- seam reads `profiles-forge-neutral`'s `.agents/conductor.md`;
- seam reads `version-flag`'s `cli.rs` and `cli_tree.rs`.

What I could not establish:
- Whether `crates/conductor/src/confine/{mod,systemd,bwrap,places}.rs` and `tests/confine*.rs` collide is only a claim about files that do not exist yet. Seam, limits and writes would all create or extend them, and edges order them.
- Outside my lane: seam's doctor rows go "after the program rows", but `doctor-verdict`'s test and `gitlab-guide`'s sample read the last line as `ready` or `not ready:`. That ordering needs the acceptance critic's eye.
- Outside my lane: forge is blocked by `decision-blocker:sandbox-forge-tokens` but sits in the same worktree chain, so the design critic should judge that.
- Outside my lane: `website/docs/status.md:88` still shows "Bash writes bounded by the sandbox" as planned. The page is already stale elsewhere (it shows "One conductor per instance" as planned although `session claim` exists), so I did not count it.

```findings
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 81
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:session-writes-bwrap and story:gitlab-guide both land on README.md, docs/design/conductor.md and website/docs/security/the-guard.md (cited, both scopes); gitlab-guide claims to be the only story on those pages, no depends_on edge or body names the pair, and the seam chain does not reach gitlab-guide; add the edge naming the shared pages as its reason, or split the pages"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 108
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:session-envelope-seam regenerates crates/conductor-model (PLAN.md and plan.json carry whole-spec digests, cited Taskfile.yml:316) beside story:gitlab-repositories and story:gitlab-workspace, which also regenerate it from config.yaml; seam has no edge to either and its Notes do not name them; add the edges naming the model as the reason, or split the surface"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 48
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "story:session-envelope-seam and story:watch-gitlab-ci both land on crates/conductor/tests/watch.rs (cited, both scopes) with no depends_on edge and no mention in either body; add the edge naming the file as its reason, or split the file"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 104
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the Notes name doctor.rs, tests/doctor.rs, Taskfile.yml, tests/taskfile.rs and .agents/conductor.md as shared with wave 17, but omit cli.yaml, cli.rs, cli_tree.rs and the model shared with doctor-minimum-versions, version-flag and the gitlab stories, and no wave-17 story lists Taskfile.yml or tests/taskfile.rs; correct the shared set to what the scopes show"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 108
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the scope omits website/docs/reference/cli.md, which the new [session, start] binding and the cli.rs change regenerate and which task docs-check fails on when stale (cited crates/conductor-docs/src/cli.rs:9); doctor-minimum-versions and version-flag both list it; add it to the scope"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 108
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the scope omits the hand-written pages that go stale when the start path moves: website/docs/security/confinement.md:14-23 and website/docs/guides/start-conductor.md:94,131 state the claude --bg start forms, and docs/config.md:89-90,130-135 mark confinement.mode and provider as read by nothing; docs/config.md is listed only by session-writes-bwrap; add the pages to the scope or say whose they are"
- file: .engineering/planning/story/session-limits-systemd.md
  line: 63
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the scope omits docs/config.md, whose Read-by column reads 'nothing yet' for the limits keys (rows 84-86, 91-93) and is false once this story lands; session-writes-bwrap lists the same file, so the two collide on a file limits does not declare; add the file and name the order"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 63
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the story adds roles[].forge_token_env but its scope omits the hand-written key table in docs/config.md, which session-writes-bwrap already lists; add the file to the scope"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 57
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the depends_on edge to story:gitlab-workspace has no stated reason, and the Notes name none of the files shared with the gitlab stories and gh-optional (spec/domains/config.yaml, conductor-model, reference/config.md, config.rs) or with gh-optional and the doctor stories (doctor.rs, tests/doctor.rs); they are ordered transitively but unnamed; state the shared files beside the edge"
```
