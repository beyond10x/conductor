---
format: aep.planning-md/3
id: review-result:forge-token-security-round-1
kind: review-result
status: active
title: Instance forge token and git wiring, security review round 1
relations:
- reviews: story:instance-forge-token
- reviews: story:pod-git-reaches-forge
revision: 1
---
needs-change

unit: commits `c9cfc2023` and `75cac4361` (range `38ee8eaea..62c8d2a00`), working tree at head `62c8d2a00` plus my test additions
verdict: NEEDS-CHANGE (1 red case)
cases: executed 9→12, red 1 (the 9 come from the run of my 3 cases alone, which skipped the 9 existing ones)
origin: introduced 1 / pre-existing 1 / undecided 0
wrote-outside-worktree: 1 directory, `<the review scratch directory>/` (logs `run-new.log` and `run-suite.log`, probe scripts `probe.sh`, `glab-probe.sh` and `listen.py`, isolated homes `home/` and `ghome/`)
needs-coordinator: a policy decision. Should a host the caller chooses get the operator's daemon-wide GitLab token?

1. `git diff --stat`: `crates/conductor/tests/sec_instance_pods.rs | 218 +` (1 file, a test file only; no implementation file touched).

**Invariants**
1. held by `crates/conductor/tests/daemon_pods.rs:1097` and `:1240`, and by `crates/conductor/tests/sec_instance_pods.rs:1023` (new, green).
2. held by `crates/conductor/tests/sec_instance_pods.rs:401`. I also ran git itself: case, the default port 443, ports with leading zeros, a trailing dot, userinfo, subdomains, `http`, another port, `%2e`, `@` and path tricks all fail closed or match the same origin. The host check at `src/config.rs:4236` refuses `*`, `%`, `$`, IDN and empty labels. Kubernetes' `$(VAR)` expansion cannot fire, because the helper holds only `$1` and `$GITLAB_TOKEN`, never `$(`.
3. NOT held for the control-token caller: the operator's daemon-wide `GITLAB_TOKEN` gets a git helper for any `gitlab` host the caller names, shown by `the_daemons_gitlab_token_gets_no_git_helper_for_a_host_the_caller_names` (red). The other attacks are held by `crates/conductor/tests/daemon_pods.rs:1492` and by git itself (see finding 1).
4. held by `crates/conductor/tests/sec_instance_pods.rs:276` (the control token) and `:958` (the store's password, new, green).
5. held by `crates/conductor/tests/daemon_pods.rs:1240` and `:1801`. A PUT cannot rename an instance (`src/config/instance.rs` `named`). Every write validates the whole file again, so changing sources after creation is caught.
6. held by `crates/conductor/tests/sec_instance_pods.rs:1023` (new, green). It checks the pod, the claim, the rendered file in the Secret, the config file, the HTTP answer and the pass result. The digest takes secret names only (`src/daemon/pods.rs:452`).
7. held by `crates/conductor/tests/config.rs:3696`. Every path validates the whole file through `parse`. The daemon-config merge (`src/daemon/seed.rs`) writes the file and then validates it, so a merge that breaks a rule stops the start rather than slipping past it. A config written before the change cannot hold the key, because the old reader refused unknown keys.
8. held by `crates/conductor/tests/kubernetes_guide.rs:335` and `:413`. One wording gap is note 3 below.

**Findings, by severity**
1. warning, NEEDS-CHANGE, introduced: `crates/conductor/src/daemon/pods.rs:140`.
   - **What the caller gains:** the caller POSTs a `gitlab` source with `host: <its server>` and no `forge_token`. The pod then holds `credential.https://<its server>.helper`, which reads the daemon-wide `GITLAB_TOKEN`. The next HTTPS fetch by the runtime or a session sends the operator's token to the caller's server once it answers 401.
   - **Already exposed before the change:** `src/collect/gitlab.rs:519` already runs `glab api --hostname <host>` with that variable set. From glab's documented env handling, glab sends `GITLAB_TOKEN` to any host. I did not confirm that by a run (see the end). So in practice this is a second channel for an existing exposure.
   - **Fixed elsewhere:** redirects, plain `http`, `insteadOf`, `.gitmodules` URLs and `useHttpPath` cannot redirect the token. git picks the helper by the final URL, and the key has no path.
   - **Not new:** a session in the pod can read the variable anyway. Repository-local `http.proxy` plus `sslVerify=false` needs write access inside the pod, which already means holding the variable.
   - **Smallest fix:** write the gitlab helper only when `GITLAB_TOKEN` is the instance's own `forge_token`, or only for hosts the operator lists in `daemon.kubernetes`. Closing the glab channel as well means refusing, at validation, a `gitlab` host outside that list while the daemon's `env` gives `GITLAB_TOKEN`.
2. note, CONFIRMED, pre-existing: `crates/conductor/src/daemon/pods.rs:297`.
   - `forge_token` may name any secret in the daemon's source except the daemon's own: another instance's token, or the daemon-wide forge secret itself.
   - Naming the daemon-wide secret gives the pod what it gets by default anyway, so nothing is gained.
   - `model_token: secret:<any>` already did all of this before the change, and one control plane owns every instance. Fix: none needed in this unit.
3. note, CONFIRMED, introduced (contract drift): `spec/domains/config.yaml:946` and `website/docs/guides/run-in-kubernetes.md:569`.
   - The text "carries none of the daemon's forge tokens" holds only for `GH_TOKEN` and `GITLAB_TOKEN`.
   - An operator `env` entry `GITHUB_TOKEN` (which gh also reads) stays in a pod that has its own token.
   - The rendered file can also bring the daemon's forge secret back into the Secret (for example `model_token: secret:forge-token`).
   - Fix: narrow the wording, or drop the alternate variable names too.
4. note, CONFIRMED, introduced (contract drift): `docs/design/architecture-next.md:210` and the decision D8 comment at `crates/conductor/tests/sec_instance_pods.rs:347`.
   - They still say env sources come only from `daemon.kubernetes`, the name and the rendered file, with `forge_token` as the one exception.
   - In fact the kinds of `sources` and a gitlab `host` now also change the pod's `GIT_CONFIG_*` env.
   - Fix: update both texts.

**Test run** (`conductor build run --reserve 8G -- cargo nextest run --locked -p conductor-cli --test sec_instance_pods`, exit 100):
`     Summary [   1.434s] 12 tests run: 11 passed, 1 failed, 0 skipped`
The 3 new cases alone, run first, before the suite: `3 tests run: 2 passed, 1 failed, 9 skipped`. Failure text: `git offers the operator's daemon-wide GITLAB_TOKEN (secret forge-token) to forge.elsewhere.example, a host only the control plane named: ["credential.https://forge.elsewhere.example.helper"]`

**Cases added** (all in `crates/conductor/tests/sec_instance_pods.rs`):
- red: `the_daemons_gitlab_token_gets_no_git_helper_for_a_host_the_caller_names` (:901)
- green: `a_forge_token_naming_the_stores_password_reaches_no_pod` (:958)
- green: `an_own_forge_tokens_value_is_only_in_its_pods_secret` (:1023)

**Not checked**
- I ran no `aep plan artifact show`: the security-reviewer procedure forbids every `aep plan artifact` command, read-only verbs included. I read section 4.1 of the design and the code instead.
- glab's handling of the token is not confirmed by a run. My local glab probe never reached the loopback listener because the `config set` flags were wrong. While it ran, glab's built-in update check made one outbound version call to its release endpoint; nothing secret was sent.
- What a token containing a newline does to git's credential protocol.
- The role key `forge_token_env` inside an instance's `roles`.
- Behaviour against a real cluster: no `$(VAR)` run on a real API server, no Secret key validation by a real API server.

```findings
[
  {"file": "crates/conductor/src/daemon/pods.rs", "line": 140, "category": "acceptance", "severity": "warning", "verdict": "NEEDS-CHANGE", "origin": "introduced", "message": "the daemon-wide GITLAB_TOKEN gets a git credential helper for any gitlab host the control-plane caller names, so its fetches send the operator's token to that host"},
  {"file": "crates/conductor/src/daemon/pods.rs", "line": 297, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "forge_token may name any non-own secret of the daemon's source, as model_token already could"},
  {"file": "spec/domains/config.yaml", "line": 946, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "\"carries none of the daemon's forge tokens\" holds only for GH_TOKEN and GITLAB_TOKEN entries, not for GITHUB_TOKEN or a secret the rendered file names"},
  {"file": "docs/design/architecture-next.md", "line": 210, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the design and the D8 test comment omit that sources and a gitlab host now shape the pod's env"}
]
```
