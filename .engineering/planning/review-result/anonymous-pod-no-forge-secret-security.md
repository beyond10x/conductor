---
format: aep.planning-md/3
id: review-result:anonymous-pod-no-forge-secret-security
kind: review-result
status: active
title: 'forge_access: anonymous without a forge credential, eight security reads'
relations:
- reviews: story:anonymous-pod-no-forge-secret
revision: 1
---
approve

Recorder's note: eight reads by independent security reviewers (one for reads 1 to 6, a second
for reads 7 and 8) of the pull request of `story:anonymous-pod-no-forge-secret`, each on a head
of its own, with one case file carried through (75 cases at the end). Reads 1 to 4, 6 and 7 ended
needs-change; each finding was taken into the next head or into a story. Between reads 5 and 7
two independent source reads named seven more points, which heads `facb68cab9` and `fbb59dfdf7`
took. The verdict above is the eighth read's, on head `df7bc07746`. The sections up to "Not
tested" are the reviewers'; "Where each finding went" is the recorder's.

## What was reviewed

The change that keeps every forge credential out of the pod of an instance with `forge_access:
anonymous`, sends none on its clone and fetch, and denies its forge writes. Eight invariants were
held or broken by a run on each head: no forge credential of the file in the pod; no variable
that points git, ssh or a forge's command line at a credential or a configuration file; no
credential on the clone and fetch, and no other git command that talks to a remote; the guard's
denials and no read denied; `secret_syncs` refused beside the member only; nothing changed
without the member but the guard decisions the pull request names; no forge command line call by
the watch and the start's report; no private text.

## The reads

1. Head `7bea8b6478`: needs-change. Sixteen cases, seven failed. Another instance's `forge_token`
   secret under a second entry name reached the pod; a token written as a plain value under a
   second name; an extra header of the checkout's own configuration on the fetch; `NETRC`,
   `GIT_TEMPLATE_DIR`, `GIT_SSL_KEY`, `GIT_SSL_CERT`; fifteen push spellings.
2. Head `0c519fc0b3`: needs-change. `22 passed; 8 failed`. A key of the anonymous instance's own
   mapping that names another instance's `forge_token` secret put it into the pod's Secret; a
   padded or embedded copy of a token value; a header of an included file and `core.askPass` on
   the fetch; five push spellings inside the guide's own list; a read alias given with
   `--config-env` denied.
3. Head `5e39898f4a`: needs-change. `29 passed; 9 failed`. The blockers collector fetched each
   checkout with a plain `git fetch` on every snapshot, outside the hardened route: `sends the
   header: true, a netrc login: true`. Also: a header in the worktree's configuration file; a
   login in the URL of `origin`; a short token value that removed unrelated entries;
   `env -- NAME=1 git push` allowed.
4. Head `5b97ce3a46`: needs-change. `39 passed; 7 failed`. On a partial-clone checkout the
   blockers collector's `git archive` fetched missing blobs by itself: `6 requests; the home's
   header: true, a netrc login: true`.
5. Head `9879940ed3`: approve. `44 passed; 6 failed`; none of the six a finding of the unit.
6. Head `facb68cab9`: needs-change. `54 passed; 8 failed`. A login in the legacy file
   `.git/remotes/origin` or `.git/branches/origin` was sent: `remotes: 4 requests, a login: true`.
7. Head `fbb59dfdf7`: needs-change. `60 passed; 11 failed`. A rewrite in the checkout's own
   configuration hid the login from the URL read, and the clean-up after it removed the rewrite:
   `config: 4 requests, a login: true`. Also `exec -c -a git-push git origin main` allowed.
8. Head `df7bc07746`: approve. `64 passed; 11 failed`; the eleven are three documented
   spellings, the stated denial of a `--config-env` alias, a route key variable the guide tells
   not to give, four preconditions of earlier cases where the fetch is now refused or runs with
   no login, the case of the worktree's configuration file and the `taskset` mask (both left for
   stories), and one case that is red only beside others in one process (the mark below).

## Suites on the last head, as printed

- collect_github: `test result: ok. 16 passed; 0 failed; 0 ignored`
- collect_stores: `test result: ok. 23 passed; 0 failed; 0 ignored`
- daemon_pods: `test result: ok. 31 passed; 0 failed; 0 ignored`
- forge_access_anonymous: `test result: ok. 13 passed; 0 failed; 0 ignored`
- forge_access_anonymous_leaks: `test result: ok. 12 passed; 0 failed; 0 ignored`
- forge_access_anonymous_review: `test result: ok. 60 passed; 0 failed; 0 ignored`
- guard_hook: `test result: ok. 34 passed; 0 failed; 0 ignored`
- guard_rules: `test result: ok. 179 passed; 0 failed; 4 ignored`
- instance_sources: `test result: ok. 23 passed; 0 failed; 0 ignored`
- kubernetes_guide: `test result: ok. 17 passed; 0 failed; 0 ignored`
- member_failures: `test result: ok. 4 passed; 0 failed; 0 ignored`
- placement: `test result: ok. 93 passed; 0 failed; 0 ignored`
- pod_checkouts: `test result: ok. 21 passed; 0 failed; 0 ignored`

## Held on the last head

- All eight invariants, each by a run.
- A checkout whose rewrite hides the origin's login: `config: 0 requests, a login: false`; the
  same for the legacy `remotes` file, for a rewrite in an included file, and for a configuration
  that cannot be read.
- A whole anonymous snapshot: every git run carries plain `ssh` as `GIT_SSH_COMMAND`; without
  the member `2 git runs, 2 with the caller's key, 0 plain`.
- Pushes under a name given in `argv[0]` (`env -a`, `--argv0`, `exec -a` and its clusters) are
  denied with the member; no read is denied. Without the member only builds named that way
  change, to a denial as an unreserved build.

## Not tested

- A pod on a cluster and a real forge.
- A git older than 2.45, which does not read `GIT_NO_LAZY_FETCH`; the runs used git 2.56.
- A configuration changed between the URL read and the fetch; a live ssh server.
- `reference-transaction` hooks run by a fetch; the GitLab sources of an anonymous instance.

## Where each finding went

- Into the pull request: every blocker of reads 1 to 7 and the seven points of the two source
  reads.
- `story:anonymous-fetch-worktree-config` (draft): a header in `.git/config.worktree`.
- `story:guard-launcher-spellings` (draft): `taskset <mask>`, `env -S`, a launcher `parallel`,
  env's abbreviated long options, zsh's `ARGV0=`, a multi-call program that reads its name.
- The findings recorded with this result are the notes of the last reads.

```findings
[
{"file":"crates/conductor/src/instance.rs","line":74,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the mark of a process that works for an anonymous instance is process-wide and never taken back: git started for another instance in the same process carries GIT_NO_LAZY_FETCH=1 and plain ssh in place of the caller's ssh command; the member validates only inside a pod, and a pod runs one instance, so no production process was shown to serve both"},
{"file":"crates/conductor/src/config.rs","line":1368,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"the config marks the process only when it fixes the active instance first; the collectors' sources mark anyway and the pod's environment carries the variable; not run"},
{"file":"website/docs/guides/run-in-kubernetes.md","line":924,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"GIT_NO_LAZY_FETCH is read by git from 2.45.0 and the guide names no minimum git version; measured on git 2.56 only"},
{"file":"crates/conductor/src/collect/blockers.rs","line":860,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"for every instance a member failure whose standard error starts with warning lines followed by another line records that line instead of the warning; read from the code, not run"},
{"file":"crates/conductor/src/collect/checkouts.rs","line":452,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a login only in a second remote.origin.url is not refused; the fetch uses the first URL and sends no login (2 requests, a login: false)"},
{"file":"crates/conductor/src/collect/checkouts.rs","line":346,"category":"integrity","severity":"note","verdict":"INFEASIBLE","origin":"pre-existing","message":"the read before an anonymous fetch does not cover .git/config.worktree, so an extra header there is sent; nothing is shown to write a checkout's worktree configuration; left for story:anonymous-fetch-worktree-config"},
{"file":"crates/conductor/src/guard/rules.rs","line":2423,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"taskset with a mask, env's abbreviated --argv= and --a=, and zsh's ARGV0= before git-push are allowed for an anonymous instance, as before the story; the pod holds no credential for the push; left for story:guard-launcher-spellings"}
]
```
