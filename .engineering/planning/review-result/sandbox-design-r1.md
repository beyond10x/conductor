---
format: aep.planning-md/3
id: review-result:sandbox-design-r1
kind: review-result
status: active
title: Design critic, round 1, sandbox
relations:
- reviews: story:session-envelope-seam
- reviews: story:session-limits-systemd
- reviews: story:session-writes-bwrap
- reviews: story:session-forge-credentials
revision: 1
---
needs-revision

- story:session-envelope-seam — the `Provider` trait is stated as "builds the start command and reads back a measurement", yet the same body needs the provider to stop a session (shortfall path) and `conductor spawn` to deliver the charter. The spike shows both differ per provider: a boxed session is an interactive unit under `script`, so `claude stop` does not apply, it is stopped with `systemctl --user stop`, and its first prompt goes through the PTY FIFO. Neither this body nor `story:session-limits-systemd` says who owns stop, resume, first-prompt delivery or session identity, and the seam's ".agents/conductor.md … boxed forms" describe what the systemd story builds. State the full trait surface in the seam (start, stop, resume, prompt, id) and let the systemd story implement it. — .engineering/planning/story/session-envelope-seam.md:74, .engineering/planning/story/session-envelope-seam.md:80, .engineering/planning/story/session-envelope-seam.md:95, docs/analysis/session-box-spike.md:205-207
- story:session-envelope-seam — the request carries the role's `writable` place words, which only resolve with per-session inputs the start function holds (checkout, trees, `TMPDIR`, instance state and cache). Yet the resolver, `confine/places.rs`, is an item of `story:session-writes-bwrap`, so the seam's `envelope_writable` measurement and its fake-provider tests cannot be stated without it. The resolver is provider-neutral: `story:substrate-tool-server` already depends on the bwrap story, probably for it. Move the resolver into the seam so the request carries absolute paths, or state in the seam that it carries unresolved words and who resolves them. — .engineering/planning/story/session-envelope-seam.md:74, .engineering/planning/story/session-writes-bwrap.md:53-55, `aep plan artifact graph` (story:substrate-tool-server -> story:session-writes-bwrap depends_on)
- story:session-writes-bwrap — it calls bwrap "a layer of the `systemd` provider" and edits `systemd.rs` and `mod.rs`, but the seam defines no layer concept and `confinement.provider` has only `systemd` and `none`. It never says whether its doctor probe is a provider row or a new kind of row beside the seam's provider listing. Nor does it say what `report` mode does when systemd is available and `bwrap` is missing. `observation.yaml` says `envelope_unboxed` leaves every other envelope field absent, so a partial box (limits, no write fence) has no recorded form. Name the attachment point in `confine/mod.rs`, and state the `report` result. The other option is to keep bwrap inside `systemd.rs` and say so; `bwrap` then orders after the systemd story only because both edit `systemd.rs`. — .engineering/planning/story/session-writes-bwrap.md:51, .engineering/planning/story/session-writes-bwrap.md:61-64, spec/domains/observation.yaml:373
- story:session-forge-credentials — "the session bus (`/run/user/<uid>/bus`) is not bound" cannot hold under the bwrap story's `--ro-bind / /`, which already exposes the bus socket. Hiding it needs a tmpfs over `/run/user/<uid>` and a re-bind of `cc-socks`, which the bwrap story binds. The two bodies each state half of one argument list and its order, and the masks of `~/.config/gh` must also follow the writable `~` bind. Say how the bus is hidden and where in the order the masks sit relative to the bwrap story's binds. — .engineering/planning/story/session-forge-credentials.md:44, .engineering/planning/story/session-writes-bwrap.md:52-58
- story:session-forge-credentials — the token environment is "passed into the unit's environment", which is the `systemd-run -E` arguments the systemd provider builds, and `roles[].forge_token_env` joins the seam's request type. The scope lists only `bwrap.rs`, with no `systemd.rs` or `mod.rs`, so half the story sits in a layer its body does not name. Add those files to scope and say which layer passes the variables. — .engineering/planning/story/session-forge-credentials.md:46-47, .engineering/planning/story/session-forge-credentials.md:63-66
- story:session-forge-credentials — the `depends_on story:gitlab-workspace` edge has no reason written beside it. The body only explains the edge to the bwrap story, and `gitlab-workspace` brings the forge-neutral drafts `gitlab-source-host`, `gh-optional` and `gitlab-repositories` in front of the operator's top-priority epic. If the reason is a shared `config.rs` or `config.yaml`, write that and keep the edge. Otherwise drop it. — .engineering/planning/story/session-forge-credentials.md:10, `aep plan artifact graph` (story:session-forge-credentials -> story:gitlab-workspace)
- story:session-writes-bwrap — its docs line puts "forge and network side effects, which `story:session-forge-credentials` bounds" into the README and security pages. That is a claim the later story makes true, and the forge story's scope has no `website/docs/security/` page, so the sentence stays false after the bwrap story lands. Say there that the box does not bound forge writes, and give the forge story the same pages. — .engineering/planning/story/session-writes-bwrap.md:71-74, .engineering/planning/story/session-forge-credentials.md:63-66

**What I read:** 8 artifacts (epic, 4 stories, approval-record, decision-blocker, ADR) via `aep plan artifact show`, plus `relations`, `graph` and `validate`. I also read `session-envelope-model`, `gitlab-workspace`, `spec/domains/{config,observation}.yaml`, `spawn.rs`, `collect/sessions.rs`, `doctor.rs`, `Taskfile.yml`, `.agents/conductor.md` and the whole spike.

**Edges and reads:** I walked every non-review edge in the graph (about 150, beyond the set, including the forge-neutral and substrate chains) and found no cycle. The set is a chain, seam -> systemd -> bwrap -> forge, and each link has its reason written, so I report the layering trade-off above rather than the chain. I traced 14 acceptance reads to a producer in the set or already implemented, and all 14 are covered by an edge. `validate` reported only 5 old stories closed on an assertion, which is not mine.

**What I could not establish:**
- Whether `bwrap` can hide a Unix socket from the unit's own `systemd-run`. I judged only the ordering, not the mechanics.
- Whether `session start` returns the same session id and exit status for an interactive unit as `claude --bg` does today. The bodies are silent, and this belongs to the first finding.

**Out of my lane:**
- Scope or acceptance: the seam's Why names three start sites (`conductor:start`, `conductor:restart`, `dev:start`) but its acceptance converts only two. `conductor:restart` (`Taskfile.yml:98-149`) still stops with `claude stop` and starts with `claude --bg`, so a boxed conductor cannot be restarted.
- Acceptance: the forge story's title says "only read-only forge credentials", but `--ro-bind /` leaves `~/.ssh`, `~/.git-credentials` and `~/.netrc` readable, so a `git push` over SSH is not bounded.
- Acceptance: `systemd-run -E NAME=value` puts the token in the unit's properties, where `--setenv=NAME` would not.
- Acceptance: the bwrap story's `~` bind "limited by the live run's finding" makes its argument-list test depend on its own live run.

```findings
- file: .engineering/planning/story/session-envelope-seam.md
  line: 74
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the Provider trait is stated as build-start-command plus measurement, yet the body needs the provider to stop a session and spawn to deliver the charter, and the spike shows boxed sessions are interactive units with no claude stop, first prompt through the PTY and stop/resume through systemctl; no body says who owns stop, resume, prompt delivery or session identity, and the seam's boxed forms in .agents/conductor.md describe what story:session-limits-systemd builds"
- file: .engineering/planning/story/session-envelope-seam.md
  line: 74
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the request carries writable place words that resolve only with per-session inputs the start function holds, yet the resolver confine/places.rs is an item of story:session-writes-bwrap, so the seam's envelope_writable measurement and fake-provider tests cannot be stated without it and the resolver is provider-neutral; move it into the seam or state who resolves"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 51
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "bwrap is called a layer of the systemd provider but the seam defines no layer concept and confinement.provider has no such value; the body does not say whether its doctor probe is a provider row or a new kind of row, nor what report mode records when systemd is available and bwrap is missing (a partial box that observation.yaml's envelope_unboxed cannot express); name the attachment point in confine/mod.rs and the report result, or keep bwrap inside systemd.rs and say the order is for that shared file"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 44
  category: design
  severity: blocker
  verdict: needs-revision
  origin: introduced
  message: "the session bus is said to be not bound, but the bwrap story's --ro-bind / / already exposes it and hiding it needs a tmpfs over /run/user/<uid> with cc-socks re-bound, which the bwrap story binds; the two bodies each state half of one argument list and its order, and the ~/.config/gh masks must follow the writable ~ bind, so say how the bus is hidden and where the masks sit in the order"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 46
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the token environment is passed into the unit's environment, which is the systemd-run arguments the systemd provider builds, and forge_token_env joins the seam's request type, yet the scope lists only bwrap.rs and not systemd.rs or confine/mod.rs; name them and say which layer passes the variables"
- file: .engineering/planning/story/session-forge-credentials.md
  line: 10
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the depends_on story:gitlab-workspace edge has no reason written in the body and pulls the forge-neutral drafts gitlab-source-host, gh-optional and gitlab-repositories in front of the priority epic; state the shared file it records or drop it"
- file: .engineering/planning/story/session-writes-bwrap.md
  line: 71
  category: design
  severity: warning
  verdict: needs-revision
  origin: introduced
  message: "the docs line writes into the README and security pages that forge side effects are bounded by story:session-forge-credentials, a claim that story makes true later and whose scope has no security page, so the sentence stays false after this story lands; say the box does not bound forge writes and give the forge story the same pages"
```
