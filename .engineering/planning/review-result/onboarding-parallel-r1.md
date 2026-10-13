---
format: aep.planning-md/3
id: review-result:onboarding-parallel-r1
kind: review-result
status: active
title: Parallel-safety critic, round 1, onboarding
relations:
- reviews: story:setup-guide-for-agents
- reviews: story:doctor-identity
- reviews: story:setup-command
- reviews: story:onboarding-run
- reviews: story:snapshot-duration
revision: 1
---
needs-revision

Seven findings. The pairs that share files are ordered by edges, except one pair that has no edge at all. Most bodies do not say why they are ordered, and several scopes look incomplete.

**Where each pair lands**

| Pair | Shared surface | Ordered by | Named in the bodies |
|---|---|---|---|
| guide and doctor-identity | `website/docs/getting-started.md` | no edge | no |
| guide and setup-command | `SETUP.md` | direct edge | yes |
| guide and onboarding-run | `SETUP.md` | transitive, via setup-command | yes, onboarding-run Notes line 39 |
| doctor-identity and setup-command | `crates/conductor-model`, `spec/domains/config.yaml` | direct edge | yes, setup-command Notes line 56 |
| doctor-identity and snapshot-duration | `crates/conductor-model` | transitive | no |
| setup-command and onboarding-run | `SETUP.md` | direct edge | yes |
| setup-command and snapshot-duration | `crates/conductor-model`, `website/docs/reference/cli.md` | direct edge | no |
| onboarding-run and snapshot-duration | no declared surface (see the onboarding-run finding) | none | no |

The guide and onboarding-run, and doctor-identity and snapshot-duration, are ordered only through setup-command. The remaining pairs share nothing.

`crates/conductor-model` is regenerated whole by `ess` from all of `spec/` (AGENTS.md line 37, `Taskfile.yml` `regen`). So doctor-identity, setup-command and snapshot-duration all rewrite it even though they edit different spec files. The edges order all three.

**Findings**

```
story:setup-guide-for-agents — both this and story:doctor-identity land on `website/docs/getting-started.md` (cited, both scopes), no edge orders them and neither body names the other; `waves` only separates them by scope collision (wave 1 and wave 2), and `crates/conductor/tests/docs_forge.rs:166` reads the page's `conductor doctor` sample, so doctor-identity's sample rewrite and this item's pointer cannot run at once — .engineering/planning/story/setup-guide-for-agents.md:44
story:snapshot-duration — the `depends_on story:setup-command` edge is new (the working-tree diff adds line 9) but the body never names setup-command, or the shared `crates/conductor-model` and `website/docs/reference/cli.md` that the edge stands for — .engineering/planning/story/snapshot-duration.md:46
story:snapshot-duration — the scope omits `crates/conductor/src/cli.rs`, which holds `StartSnapshotArgs` (cli.rs:531) and the `LEAVES` access table (cli.rs:385); the `--budget` flag and the optional `snapshot collectors` view cannot land without it, and setup-command lands on the same file (inferred from the symbols; the `cli_tree.rs` header says the tree is derived from spec and `src/cli.rs`) — .engineering/planning/story/snapshot-duration.md:46
story:onboarding-run — the acceptance fixes every gap "in `SETUP.md` or `conductor setup` in the same wave", so it writes `crates/conductor/src/setup.rs`, `cli.yaml`, `cli.rs`, `crates/conductor-model` and `spec/domains/config.yaml`, yet its scope lists only `docs/analysis/onboarding-run.md`, `SETUP.md` and `AGENTS.md`; story:snapshot-duration sits in the same wave 4 and lands on `crates/conductor-model` and `cli.md`, no edge orders them (inferred surfaces) — .engineering/planning/story/onboarding-run.md:32
story:setup-command — Notes line 56 says it comes "after ... the sandbox wave (`doctor.rs`, `cli.yaml`, the model)", but `aep plan artifact graph` has `story:sandbox-spec -> story:onboarding-run` and `story:sandbox-spec -> story:snapshot-duration` depends_on edges, so the sandbox wave comes after this item's chain; `doctor.rs` is also not in its scope — .engineering/planning/story/setup-command.md:56
story:doctor-identity — the scope lists `website/docs/reference/config.md`, which does not render doctor's result (`git grep -c Prerequisite website/docs/reference/config.md` returns 0), and omits `website/docs/reference/cli.md`, which its own Notes say is regenerated; doctor's help text (`cli.yaml:196`, `cli.rs:320`, rendered at `cli.md:124`) describes what doctor reports, as doctor-minimum-versions edited it, so `cli.yaml` and `cli.rs` are probably missing too — those two inferred, and setup-command lands on all three — .engineering/planning/story/doctor-identity.md:56
story:setup-command — its scope edits `SETUP.md` but omits `crates/conductor/tests/docs_setup.rs`, the new test in story:setup-guide-for-agents that asserts each step's command and that every command named exists; a `conductor setup` step added to `SETUP.md` here, or the gap fixes in story:onboarding-run, runs into it (inferred from the guide's acceptance) — .engineering/planning/story/setup-command.md:62
```

**What you read:** five stories in full, plus `epic:onboarding-from-url`, `story:sandbox-spec` and `story:doctor-minimum-versions`. I ran `aep plan artifact waves --kind story --status draft`, `show` on each story, `graph`, `relations`, and `git grep` and `sed` on `crates/conductor/src/cli.rs`, `crates/conductor/cli.yaml`, `crates/conductor/tests/{cli_tree,docs_forge,guard_rules,no_organization}.rs`, `crates/conductor-docs/src/lib.rs` and `website/docs/reference/`.

**What I could not establish**
- Surfaces: five of five items have a cited surface, none are only inferred, none are unplaceable. The findings above that rest on inferred surfaces say so.
- Acceptance reads: I traced four to a producer in the set. Three have a direct `depends_on` edge: setup-command reads doctor-identity's forge-identity reading and the guide's `SETUP.md`, and onboarding-run reads `conductor setup`. One is covered only transitively: onboarding-run reads the guide's `SETUP.md` and README pointer through setup-command. I did not make that a finding.
- Whether doctor-identity's help text changes is inferred from how doctor-minimum-versions did it, so the `cli.yaml` and `cli.rs` omission is weaker than the `cli.md` one.
- Out of my lane, for the scope or design critics:
  - `SETUP.md` at the repository root is outside the files the public-text check in `crates/conductor/tests/no_organization.rs` reads (`docs/`, `website/docs/`, `README.md`, `AGENTS.md`, `.agents/`).
  - `conductor setup --apply` both reads and writes, but `LEAVES` takes one `Access` value per leaf.

```findings
- file: .engineering/planning/story/setup-guide-for-agents.md
  line: 44
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "both this and story:doctor-identity land on website/docs/getting-started.md (cited, both scopes), no edge orders them and neither body names the other; waves separates them only by scope collision, and crates/conductor/tests/docs_forge.rs:166 reads the page's doctor sample; either an ordering edge recording the shared file or splitting the page so they no longer share it"
- file: .engineering/planning/story/snapshot-duration.md
  line: 46
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the new depends_on story:setup-command edge is not explained in the body, which names neither setup-command nor the shared crates/conductor-model and website/docs/reference/cli.md"
- file: .engineering/planning/story/snapshot-duration.md
  line: 46
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the scope omits crates/conductor/src/cli.rs (StartSnapshotArgs at cli.rs:531, LEAVES at cli.rs:385) which the --budget flag and the snapshot collectors view need, and which story:setup-command also lands on (inferred from symbols, not cited in either body)"
- file: .engineering/planning/story/onboarding-run.md
  line: 32
  category: parallel-safety
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the acceptance fixes gaps in conductor setup in the same wave, so it writes setup.rs, cli.yaml, cli.rs, crates/conductor-model and spec/domains/config.yaml, yet the scope lists only docs/analysis/onboarding-run.md, SETUP.md and AGENTS.md; story:snapshot-duration is in the same wave 4 on crates/conductor-model and cli.md with no edge between them (inferred surfaces)"
- file: .engineering/planning/story/setup-command.md
  line: 56
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "Notes say the item comes after the sandbox wave, but the graph has story:sandbox-spec depends_on story:onboarding-run and story:snapshot-duration, which sit after this item; doctor.rs, named in that note, is not in the scope"
- file: .engineering/planning/story/doctor-identity.md
  line: 56
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: pre-existing
  message: "the scope lists reference/config.md, which does not render doctor's result (Prerequisite appears 0 times), and omits reference/cli.md that its own Notes regenerate; cli.yaml:196 and cli.rs:320 (doctor's help text) are probably missing too (inferred from how doctor-minimum-versions edited them), and setup-command lands on the same files"
- file: .engineering/planning/story/setup-command.md
  line: 62
  category: parallel-safety
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the scope edits SETUP.md but omits crates/conductor/tests/docs_setup.rs, the guide's test over SETUP.md's step commands, so a step added here or by story:onboarding-run meets that test unplanned (inferred from the guide's acceptance)"
```
