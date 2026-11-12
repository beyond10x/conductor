---
format: aep.planning-md/3
id: review-result:forge-token-security-round-2
kind: review-result
status: active
title: Instance forge token and git wiring, security review round 2
relations:
- reviews: story:instance-forge-token
- reviews: story:pod-git-reaches-forge
revision: 1
---
pass

unit: fix commit `6a5dd6758` (head `3f869881f`) plus one test I added in the working tree
verdict: nothing found that blocks the fix; 3 notes
cases: executed 13→14 (the 13 are the cases my run of the new case alone skipped), red 0
origin: introduced 2 / pre-existing 1 / undecided 0
wrote-outside-worktree: logs `r2-new.log` and `r2-suite.log` in `<the review scratch directory>/`
needs-coordinator: none

1. `git diff --stat`: `crates/conductor/tests/sec_instance_pods.rs | 77 +` (a test file only).

**Claims**
- **A. held** by `crates/conductor/tests/daemon_pods.rs:1728`, `crates/conductor/tests/sec_instance_pods.rs:905` and `crates/conductor/tests/sec_instance_pods.rs:1260` (new, green).
  - **Host comparison:** `GITLAB_HOST` goes through the same host check as a source. Trailing dots, whitespace, IDN, a scheme and `*` are refused. Both sides are lowercased. Port forms that differ only fail closed (`host:0443` and `host:443` are different hosts, so neither gets the token).
  - **Duplicates and secrets:** a second `GITLAB_HOST` or `GITLAB_TOKEN` entry is refused by the duplicate-name check (`src/config.rs:3121`). `GITLAB_HOST` written as a secret is refused (`crates/conductor/tests/config.rs:3789`).
  - **PUT after the pod exists:** a PUT that moves a source off the daemon's host replaces the pod at the next pass. The old pod runs on the config copied at its start, so it never sees the new host.
  - **Instance with no `gitlab` source:** it gets no GitLab helper, and the runtime runs `glab` only for `gitlab` sources (`src/collect/gitlab.rs:499`). The control-token caller has no channel from such a pod to a host of its choice; a session inside the pod could already read the variable.
- **B. held, pre-existing:** the code never names a GitHub host but `github.com` (`src/doctor.rs:156`, `src/daemon/pods.rs:145`). A mapping cannot set `GH_HOST`, and a `github` source has no host.
  - `gh help environment` (gh 2.102.0) says `GH_TOKEN` is also sent to subdomains of `ghe.com`. Only a session inside the pod can target those; this is not new.
- **C. held** by `crates/conductor/tests/sec_instance_pods.rs:1137` and `crates/conductor/tests/config.rs:3832`.
  - The check walks every string under `instances` and `globals`. A PUT re-validates the whole file. The daemon-config merge validates after writing. The supervisor's check covers a file written before the fix.
  - A `secret:` inside a URL is never resolved into a pod, so it is no way around the check either.
- **D. held, with one gap** (note 2), by `crates/conductor/tests/daemon_pods.rs:1905`. What I read:
  - gh: the list matches `gh help environment` of gh 2.102.0.
  - glab: I read the strings of the glab 1.115.0 binary, without running it. They contain `GITLAB_TOKEN`, `GITLAB_ACCESS_TOKEN` and `OAUTH_TOKEN`, and also `CI_JOB_TOKEN` with `GLAB_ENABLE_CI_AUTOLOGIN`. The host variables there are `GITLAB_HOST`, `GL_HOST`, `GITLAB_URI` and `GITLAB_URL`.
  - I could not read the versions the image pins, nor glab's source.
- **E. held**, except one guide sentence (note 1), by `crates/conductor/tests/kubernetes_guide.rs:477`. Design section 4.1, the spec comments and the restart table match the code.

**Findings, by severity**
1. note, CONFIRMED, introduced: `website/docs/guides/run-in-kubernetes.md:597`.
   - The guide says an instance "keeps what `daemon.kubernetes.env` gives every pod on its forge and host".
   - In the code, an instance with only `github` or `local` sources also keeps the daemon's GitLab token entries (`src/daemon/pods.rs:321`: no gitlab source means "on the host").
   - Fix: say so in the guide, or, for least privilege, drop the GitLab entries for a pod with no `gitlab` source.
2. note, INFEASIBLE, introduced: `crates/conductor/src/config.rs:513`.
   - `CI_JOB_TOKEN` is not in the glab list. An operator `env` entry of it stays in an instance with its own token and in a pod on another GitLab host.
   - Nobody was shown to set it: glab reads it only when `GLAB_ENABLE_CI_AUTOLOGIN` is also set.
   - Fix: add it to the list, or say in the guide that it is not covered.
3. note, CONFIRMED, pre-existing: `crates/conductor/src/config.rs:3121`.
   - An operator who writes the self-hosted GitLab host as `GL_HOST` or `GITLAB_URI` instead of `GITLAB_HOST` makes the daemon assume `gitlab.com`. This fails closed: instances on their host get no token.
   - A `GITLAB_HOST` with a scheme, a form glab accepts, is now refused at validation. The guide's "a host as a gitlab source writes one" covers this.

**My changed case** (`crates/conductor/tests/sec_instance_pods.rs:905`): it still proves what I meant, and more strongly.
- What I meant: the operator's GitLab token must not reach a host the caller names. Asserting that the variable and its secret are absent covers git and `glab` together. Removing the `on_gitlab_host` check would turn the case red again.
- What it lost is the positive control: it no longer shows that the scenario carries the token at all, so the helper assertion after it is now vacuous. A fix that took the GitLab token from every pod would also pass it.
- `daemon_pods.rs:1728` and my new case's precondition (a pod on `gitlab.com` carries the token) restore that control.
- The doc comment of the changed case still says "a daemon whose `env` gives every pod" the token. That wording is stale but harmless.

**Round-1 finding 2** (an instance may name any other secret of the daemon's source): the fix makes its reach smaller, not larger. Forge-token secrets are now excluded, and `GITLAB_HOST` cannot be a secret.

**Test run** (`conductor build run --reserve 8G -- cargo nextest run --locked -p conductor-cli --test sec_instance_pods`, exit 0):
`     Summary [   4.269s] 14 tests run: 14 passed, 0 skipped`
The new case alone, run first: `1 test run: 1 passed, 13 skipped`.

**Case added:** green, `a_put_that_moves_a_gitlab_source_off_the_daemons_host_replaces_the_pod_without_its_token` (`crates/conductor/tests/sec_instance_pods.rs:1260`).

**Not checked**
- Whether secrets sources that ignore case in names (keychain, remote) map two names to one secret, which could get around the forge-secret check.
- glab's source and the gh and glab versions the image pins.
- A session inside a pod running `gh` or `glab` inside a checkout, where the host comes from the git remote.
- Whether the instance-API proxy refuses every control-plane write: I read only `CONTROL_PLANE_ROUTES`.
- Behaviour on a real cluster.

```findings
[
  {"file": "website/docs/guides/run-in-kubernetes.md", "line": 597, "category": "contract-drift", "severity": "note", "verdict": "CONFIRMED", "origin": "introduced", "message": "the guide says an instance keeps the env entries of its forge and host, but a pod with no gitlab source also keeps the daemon's GitLab token entries"},
  {"file": "crates/conductor/src/config.rs", "line": 513, "category": "judgement", "severity": "note", "verdict": "INFEASIBLE", "origin": "introduced", "message": "glab also reads CI_JOB_TOKEN under GLAB_ENABLE_CI_AUTOLOGIN, which FORGE_CLI_TOKEN_VARIABLES leaves out"},
  {"file": "crates/conductor/src/config.rs", "line": 3121, "category": "judgement", "severity": "note", "verdict": "CONFIRMED", "origin": "pre-existing", "message": "a self-hosted GitLab host written as GL_HOST or GITLAB_URI instead of GITLAB_HOST makes the daemon assume gitlab.com, which fails closed for that host"}
]
```
