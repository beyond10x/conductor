# Wave 02

Coordinator: conductor-dev. Skill: `aep:implementing` 0.20.1, wave mode. Approved by the operator
on 2026-10-08 ("make sure we do not rely on global claude config at all"; the first user's review
items: "yes, all of that makes perfect sense").

Integration branch `wave/02` from `main` at `510ca0f`, tree `cw02-int`. Commits this approval
covers: one per unit, the merges into `wave/02`, the coordinator's docs and profile commits, the
closing store commit, and the merge into `main`.

## Units

| unit | story | outcome |
|---|---|---|
| U1 | sessions-without-global-config | merged |
| U2 | specifications-every-source | merged |
| U3 | sessions-outside-root-explained | merged (model regenerated at the merge) |
| U4 | portable-free-space | merged |
| U5 | portable-trust, first-run-seed, install-prerequisites | merged (model regenerated; `offline()` test helpers reconciled with U4; `HOME` isolated in the snapshot tests) |
| U6 | portable-dashboard-service, first-run-bypass-disclaimer | merged |
| U7 | guard-profile-files | merged |
| U8 | guard-write-forms-and-config-scope | merged |
| coordinator | profiles-from-review; README, AGENTS, `b10x.toml` | committed on `wave/02` |

Units ran in their own trees (`cw02-u1` … `cw02-u8`), each finished with `--discard-cache` after
its merge.

## Outcome

Closing gate on `wave/02`: `task check` exit 0, 579 passed, 0 failed, 3 ignored; conformance 281
of 281; `docs-check` 5 generated files current; Gates scan 364 files; site build exit 0. The first
closing run failed on `cargo fmt --check` in the merge resolution of U5 (escapes turned into
newlines by the resolving script); fixed and rerun.

Runtime check before rollout: a background `claude` session started with `--setting-sources
project,local` and a settings file holding the env, plugins and `claudeMdExcludes` ran a Bash
command without a permission prompt (bypass mode held); a print-mode session with the same flags
had the plugins and the env and no global `CLAUDE.md`.

Planned beside this wave (not implemented): epic `multi-instance-host`, its specification and nine
draft stories, two critic rounds recorded; draft domain `spec/drafts/host.yaml`.
