# Design: how an instance that runs as a pod reaches its forge

Status: proposal, not accepted, 2026-11-13. Two variants are described and neither is built. The
first decision asked of the operator is which variant (§ 9, V); the five decisions D-1 to D-5
apply only under variant R. No store record yet.

- **Variant R, a reference:** the instance mapping names a Kubernetes Secret as its forge
  credential; for a GitHub App key the daemon mints one-hour tokens for the pod (§ 4).
- **Variant B, a forge broker of the control plane:** the control plane keeps every forge secret;
  an instance gets, per repository, a git remote and endpoints for forge operations, and neither
  the daemon nor a pod ever holds a forge credential (§ 5).

Paths without a crate are under `crates/conductor/src/`, at tag `v1.6.0` (`700e1caa0d`). "The
guide" is `website/docs/guides/run-in-kubernetes.md` at that tag. A statement about Kubernetes, git
or a forge that was not read in this repository or run is marked **not shown**; a thing a variant
rests on that nobody has run is marked **to be shown**.

Words: the **instance's Secret** is `conductor-instance-<name>`, which the daemon writes at every
pod start. A **referenced Secret** is one a control plane wrote and an instance mapping names. The
**pods' namespace** is the namespace of the instance pods, which is not the daemon's. A **broker**
is a service of the control plane that performs forge operations for an instance.

## 1. Why

A control plane stores a person's forge credential and wants an instance to push with it. At
`v1.6.0` an instance names `forge_token: secret:<name>`, and the name resolves against the
daemon's secrets directory only, "never a Kubernetes Secret by name"
(`spec/domains/config.yaml:791-792`). So the credential has to be put into the daemon's own
Secret, by whoever installs the daemon, before a control plane can name it.

The second case is a credential that does not last. A GitHub App's installation token lives one
hour (**not shown**: the forge's documentation). A pod reads its forge token from its environment,
fixed at pod start (`daemon/pods.rs:377`), so an instance on such a token stops pushing after an
hour, and the only way to give it a new one is to replace the pod, which ends its sessions.

The third case is the one variant B answers: a pod that holds a forge token can leak it, and
whoever has it can do on the forge whatever the token allows, with no record on the control
plane's side.

## 2. What is there

- The pod's forge token: one environment variable, `GH_TOKEN` or `GITLAB_TOKEN` by the kind of
  the instance's sources, through `secretKeyRef` on the instance's Secret (`daemon/pods.rs:373-378`).
  git's credential helper prints that variable when git asks (`daemon/pods.rs:120-123`); `gh` and
  `glab` read the variable themselves (`config.rs:495-498`); every process the runtime starts
  inherits it (`instance.rs:57-63`).
- A pod without that variable does nothing against its forge: it tries no clone and reports each
  repository as a failure (`collect/checkouts.rs:250-270`), git gets no credential helper
  (`daemon/pods.rs:136-158`), the doctor says "not logged in" and the health answer is not ready
  (`doctor.rs:262-275`, `:2267-2270`).
- The remote of a repository is derived, not given: `https://github.com/{owner}/{name}.git` for a
  `github` source, `https://{host}/{group}/{path}.git` for a `gitlab` one
  (`collect/checkouts.rs:143`, `:171`). No member of a mapping names a remote per repository.
- Among the keys of the instance's Secret are the rendered config file and one key per
  secret the mapping names (`daemon/pods.rs:774-820`). It is also mounted whole, read-only, at
  `/run/secrets/conductor`, with no `subPath` (`pod.rs:46`, `daemon/pods.rs:450`). The runtime's
  secrets source reads a file of that mount at every read (`secret.rs:145-149`), which is how the
  model token reaches the harness per use (`token.rs:663-668`).
- The daemon's Kubernetes client creates, gets, lists and deletes Secrets and has no call that
  changes one (`daemon/kube.rs:340-382`). The Role of the guide grants those four verbs on every
  Secret of the pods' namespace. The doctor asks for those rights (`doctor.rs:2113-2124`) and a
  test holds that each one asked for is used (`tests/sec_instance_pods.rs:731`).
- No text of a caller reaches the pod's spec: every field comes from the daemon's configuration,
  the instance's name and the rendering (`daemon/pods.rs:31-36`; held by
  `tests/sec_instance_pods.rs:357`).
- The pod's digest holds the spec, the rendered file and the names of the secrets, no value
  (`daemon/pods.rs:513-530`). A changed name replaces the pod; a changed value does nothing until
  the next start.
- The daemon calls a pod over plain TCP with the runtime's API token in a header
  (`daemon/reconcile.rs:1770-1777`, `dashboard.rs:961-966`). No runtime route takes a credential.
  Nothing in a pod calls the daemon, and a pod mounts no service account token
  (`daemon/pods.rs:459`).
- The daemon's own secrets are refused in a mapping (`config.rs:2737-2753`) and again at pod
  start (`daemon/pods.rs:783-806`); the calls answer `422` `conductor.config.ConfigInvalid` with
  the YAML path (`daemon/api.rs:1855-1874`).
- The workspace has `ring` and `rustls`, no JWT crate and no HTTPS client that trusts public
  certificate authorities; the daemon reaches a forge's API today by running `curl`
  (`upgrade.rs:319-366`).

## 3. The forge operations of v1.6.0

This list is what an instance does against a forge today. Under variant B it is the contract of
the broker; under variant R it is what the pod's token must allow. Call sites are in brackets.

Reads conductor's own code makes, all inside the pod, through `gh`, `glab api` or git:

1. list the repositories of an owner or the projects of a group [4: `collect/repositories.rs:240`,
   `collect/github.rs:158`, `collect/specifications.rs:187`, `collect/gitlab.rs:614`];
2. list open pull or merge requests with the state of their checks [4: `collect/github.rs:168`,
   `watch.rs:419`, `collect/gitlab.rs:250`, `watch.rs:2744`];
3. read one merge request (head pipeline, merge status) [2: `collect/gitlab.rs:256`, `watch.rs:2750`];
4. list requests merged since a date [2: `collect/github.rs:257`, `collect/gitlab.rs:294`];
5. list the runs or pipelines of the default branch [4: `collect/github.rs:181`, `watch.rs:406`,
   `collect/gitlab.rs:427`, `watch.rs:2787`];
6. list releases [4: `collect/repositories.rs:263`, `collect/github.rs:245`,
   `collect/gitlab.rs:324`, `:1067`];
7. list tags with their dates [2: `collect/github.rs:280`, `collect/gitlab.rs:367`];
8. list open issues, or the oldest one [2: `collect/repositories.rs:250`, `collect/gitlab.rs:1096`];
9. read the head commit of the default branch [2: `collect/repositories.rs:275`,
   `collect/gitlab.rs:1028`];
10. list commits since a date [2: `collect/repositories.rs:280`, `collect/gitlab.rs:1039`];
11. read the files or the diff of a commit [2: `collect/repositories.rs:286`,
    `collect/gitlab.rs:1057`];
12. compare a tag with a branch [1: `collect/gitlab.rs:1077`];
13. read when an Actions secret was last updated [1: `collect/secrets.rs:62`];
14. check the login [1: `doctor.rs:262`];
15. `git fetch` of a checkout [4: `collect/checkouts.rs:330`, `collect/repositories.rs:962`,
    `:1567`, `collect/blockers.rs:179`];
16. `git clone` of a repository an `include` entry names [1: `collect/checkouts.rs:297`];
17. read conductor's own release and download its assets, with no token, by the daemon and the
    `upgrade` command only [`upgrade.rs:319-366`].

The collectors read the forge CLI's own output shapes, among them `statusCheckRollup` of
`gh pr list` and a GraphQL query for tags (`collect/github.rs:96`, `:168`).

Reads a session makes have no fixed list: the guard admits every read verb of `gh` and `glab`
and every `api` call that is a GET (`guard/rules.rs:721-736`, `:3745`), and the controller's role
text tells it to read the log of a failing check (`roles/controller.md:35`).

Writes. The code makes one: it pushes the `main` of the instance's records where that repository
has a remote (`run/records.rs:219`). A session makes the others, told by its role text and
admitted by the guard for its own repository only:

1. push a branch (`roles/controller.md:35`, `:89-91`);
2. open a pull or merge request (`roles/controller.md:49`, `guard/rules.rs:780`, `:784`);
3. change its description (`roles/controller.md:58-60`);
4. comment on it (`guard/rules.rs:780`, `:784`);
5. mark it ready (`guard/rules.rs:780`);
6. merge it or set it to merge when its checks pass, under a charter with `merge: controller`
   (`roles/controller.md:51`, `guard/rules.rs:780`, `:784`, `:800`);
7. run a workflow or a pipeline again, or start one (`guard/rules.rs:770`, `:789`, `:794`);
8. create a tag and a release and upload its assets (`roles/controller.md:41`,
   `guard/rules.rs:770`);
9. close a request (role texts of the host roles; not a pod role today).

Not determined in this repository: what `b10x-gates` calls and with which credential under
`forge.route: gates`; whether the `worktree` CLI reaches a remote; which forge calls a session's
loaded skills make.

## 4. Variant R: a reference to a Secret, and minting for a GitHub App

### 4.1 What lives in the pods' namespace, and who can read it

Today's rule (guide, "These rights reach further…"): the pods' namespace holds nothing but what
the daemon makes there; no other workload, Secret or service account. The reason given there
stays true: `get` and `list` read every Secret of the namespace, and `create` on pods can start a
pod that names any Secret of it.

Proposed rule: the pods' namespace holds what the daemon makes there **and the Secrets a control
plane writes for instances**, each carrying the label `conductor/instance-credential: "true"`
(the prefix of the daemon's own labels, `daemon/pods.rs:68-74`). Still no other workload and no
other service account.

Who can then read a referenced Secret:

- the daemon, through its Role (as it can read every Secret of the namespace today);
- whoever holds the control token, indirectly: it can name any labelled Secret in a mapping and
  so have its value given to an instance it also controls. The label is a rule the daemon keeps,
  not a boundary Kubernetes enforces. One control token means one trust domain: two control
  planes that must not use each other's credentials need two daemons and two namespaces;
- an instance pod: only the values the daemon copied into its own Secret;
- whoever administers the cluster or the namespace, as for any Secret.

Never accepted as a reference, with a declared error at `POST /v1/instances` and
`PUT /v1/instances/{name}`: a Secret without the label; a Secret the daemon made (it carries the
daemon's owner labels); a name that is no Kubernetes object name. The `422` for a mapping that
names a secret of the `daemon` section stays as it is.

### 4.2 The reference (first unit)

```yaml
forge_credential:
  secret: <name of a labelled Secret in the pods' namespace>
  key: <key in it>
  type: github-token | gitlab-token | raw | github-app
  app_id: <number; github-app only>
```

- `forge_token: secret:<name>` stays and means what it means today. A mapping that holds both is
  refused by path.
- `github-token` and `gitlab-token`: the daemon checks that the type fits the kind of the
  instance's sources and nothing about the value's form. `raw`: no check at all. All three go the
  way `forge_token` goes: the variable of the sources' forge.
- The daemon reads the referenced Secret (`get`, a right it holds), checks the label, and copies
  the value into the instance's Secret under one fixed key, `forge-credential`. The pod's spec
  names that fixed key and never the control plane's Secret, so the rule that no text of a caller
  reaches the pod's spec holds unchanged.
- The reference's Secret name, key and type enter the digest; the value does not. A changed
  reference replaces the pod once. A changed value reaches the pod at its next start, or through
  the refresh of § 4.4 once that exists.
- The check runs at the call (a missing or unlabelled Secret is refused, so a control plane
  learns it at once) and again at every pod start (a Secret removed since then is a start
  failure of the instance, whose `detail` names the Secret and the reason and never a value).
- New declared errors, in `spec/domains/config.yaml`: the referenced Secret does not exist; it
  does not carry the label; the key is not in it; the type does not fit the instance's sources.

### 4.3 `github-app`: the daemon mints, the pod never holds the key (second unit)

- The referenced Secret holds the App's private key. The daemon reads it when it mints and keeps
  it in memory no longer than the signing. It is copied nowhere: not into the instance's Secret,
  not into a file of the daemon, not into the pod.
- Mint: an RS256 JWT of at most 10 minutes signed with `ring`; `GET /repos/{owner}/{repo}/installation`
  for the installation; `POST /app/installations/{id}/access_tokens` with the repository names
  and a fixed permission set (§ 9, D-4). The calls go through `curl` as the daemon's forge calls
  do today, with the JWT handed over on standard input, never as an argument, so no process
  list shows it. (**Not shown**: the forge's call shapes are from its documentation.)
- Narrowing: the token names the repositories the instance's rows name. A `github` source names
  an owner with an optional `include` list (`spec/domains/config.yaml:86-91`); without `include`
  the repositories are not named. So `github-app` is refused by path for an instance whose
  source has no `include`, and for an instance with sources of two owners (two installations,
  but one variable and one helper per origin, `daemon/pods.rs:144-157`).
- The minted token is written into the instance's Secret under `forge-credential` before the pod
  starts, as any other value. The first start therefore needs nothing of § 4.4.
- The expiry is kept in the daemon's state file beside the runtimes, as a time and nothing else.
  The token is not written there: that file holds the runtimes' API tokens in plain text today
  (`daemon/reconcile.rs:1893-1898`), and a forge token must not join them. After a restart the
  daemon mints again for each `github-app` instance at its first pass.
- A mint that fails (the key is refused, the App is not installed on the repository, the forge
  does not answer) is a member failure of the instance's health with a declared reason. The pod
  keeps the token it has; the daemon tries again at each pass.

### 4.4 How a refreshed token reaches a pod that runs

Two things have to change, whichever route is taken, because a variable fixed at pod start cannot
follow a refresh:

- git's helper reads a file instead of the variable. The file is a key of the mounted Secret at a
  fixed path, so the helper's text still holds no text of a caller.
- `gh` and `glab` read their variable from the environment (`config.rs:495-498`). For an instance
  with `forge_credential` the pod does not get the variable through `secretKeyRef`; the runtime
  sets it from the file for each process it starts (`instance.rs:57-63` is the one place), and a
  session's own shell gets it through a small wrapper for `gh` and `glab` on its `PATH` that
  reads the file at each call. The wrapper is part of the image. An instance on `forge_token`
  keeps today's variable.

The routes for the refresh itself:

- **A. The daemon replaces the instance's Secret in place; the kubelet updates the mounted
  file.** Needs: one new verb, `update` on Secrets, in the Role (the installer's act), in the
  client, in the doctor's list and in the rights test. Exposes: nothing the Role does not
  already allow. The daemon may delete and create any Secret of the namespace today, which
  together replace it; `update` is the same power in one call. Survives a restart of the
  container and of the daemon, because the value rests in the Secret. Cost: the kubelet's delay
  between the write and the file (**not shown**; the refresh runs 15 minutes before the expiry
  to leave room).
- **B. The daemon sends the token to the runtime; the runtime keeps it in a file of a
  memory-backed directory.** Needs: a new runtime route, declared in `spec/`, and no new
  Kubernetes right. Exposes: the token travels over plain TCP between the daemon and the pod, as
  the runtime's API token does today; a holder of the runtime's API token can overwrite the
  credential (not read it). Lost when the container restarts, so the runtime has to say in its
  health that it holds none and the daemon has to send it again.
- **C. The pod asks the daemon.** Needs: the daemon's address and a credential for it in the pod,
  and a network rule that admits pod to daemon. Exposes: a pod that can call the daemon, which no
  pod can today. Not proposed.

Proposed under variant R: **A**. It adds no power to the daemon, keeps the token where every
other value of the pod rests, and needs no state in the runtime. Where the Role lacks `update`, a
`github-app` instance is refused at start with a failure that names the missing right; it never
falls back to a token that will expire unnoticed.

The refresh has a place in the daemon's loop: `keep_boxes` runs at every 2-second tick and
already renews leases on a timer with its state kept in a file (`daemon/reconcile.rs:694-745`).

The same route serves a changed value of a `github-token`, `gitlab-token` or `raw` reference: the
daemon reads the referenced Secret at each pass of its reconciliation and writes a changed value
through. A control plane then never has to send a restart after a rotation.

### 4.5 What is logged, and what a test reads

Logged, by the daemon: the Secret's name, the key, the type, the App's number, the installation's
number, the count of repositories, the expiry, and the reason of a refusal or a failed mint.
Never: the key, a JWT, a token, or a forge's error body (it is cut to its status and its declared
message).

Each unit has a test with marker values that reads the daemon's log, the runtime's log, the pod's
spec, every answer of the instance calls, the daemon's state files and the argument lists of the
processes the daemon started, and finds no marker; and a test that failed before the unit's
change. No test uses a real key or a real token. The forge is a double.

### 4.6 What a pod can leak under variant R

A forge token: for `github-app` one that lives an hour and names the instance's repositories; for
the other types whatever the person's token allows, for as long as it lives. The App's key
cannot leak from a pod; it can leak from the daemon's memory or from the namespace.

## 5. Variant B: a forge broker of the control plane

### 5.1 The shape

The control plane keeps every forge secret. An instance's definition lists its repositories and
gives, per repository, a git remote and the address of the broker's endpoints for it. A session
that wants to push, open a request or merge calls the broker; git reaches the forge through a
proxy of the control plane that adds the credential. No forge secret and no reference to one
reaches the daemon or a pod.

Two forms of the broker, which differ in what conductor has to change:

- **B1, typed endpoints.** The broker offers the operations of § 3 by name ("open a request for
  this branch", "merge this request"). It can decide each operation by its meaning, refuse a
  merge into a protected branch, and record who asked for what. Conductor's collectors and its
  sessions call these endpoints instead of `gh` and `glab`.
- **B2, a proxy that speaks the forge's own API.** The broker forwards the forge's HTTP API for
  the instance's repositories and adds the credential. `gh`, `glab` and the collectors stay as
  they are and are pointed at the proxy. It decides by path and method, not by meaning. **To be
  shown:** that `gh` and `glab` work against a host that is not the forge (`gh` treats another
  host as an enterprise server with other API paths; **not shown**), including the GraphQL call
  of § 3.

### 5.2 What conductor needs on its side

1. **A remote per repository in the instance definition.** Today the remote is derived from the
   source (§ 2). A new kind of source lists repositories by name, each with its remote and its
   forge kind, so nothing is listed from the forge at start. Declared in `spec/`; an instance
   without it is unchanged.
2. **The reads of § 3 through the broker.** Under B1: a third collector backend beside the
   `github` and `gitlab` ones, for the 16 reads that touch a forge (17 is conductor's own
   release, which needs no credential). Under B2: none, if the pointing is shown to work.
3. **The session's writes through the broker.** `forge.route` gets a third value beside `gates`
   and `direct` (`config.rs:737`). The guard admits, for that route, the broker's calls for the
   session's own repository and refuses `gh`, `glab` and `git push` to any other remote
   (`guard/rules.rs:770-810` is where the two routes are told apart today). The controller's
   role text names the route's commands (`roles/controller.md:88-89`). Under B1 a session needs
   one command for the nine writes of § 3, most simply a `conductor forge …` command that calls
   the broker, so that the role text and the guard name one program.
4. **The session's reads.** They have no fixed list today (§ 3). Under B1 either the broker
   offers a generic read for the instance's repositories, or the list is fixed and the role text
   and the guard are narrowed to it; the log of a failing check has to be on it. Under B2 the
   proxy forwards every GET for the instance's repositories.
5. **Everything that takes a missing token for "no forge".** The clone at start, git's helper,
   the doctor's login check and the health answer (§ 2) each get a branch for a brokered
   instance: the remote is the proxy's, no helper is written, and the login check asks the
   broker whether it serves this instance.
6. **The records' push**, where the records have a remote: through the proxy like any push.

### 5.3 How the pod proves which instance it is

The broker and the proxy must know which instance calls, or any pod could use any repository.
So something that identifies the instance reaches the pod, and **that thing replaces the forge
token as what a pod can leak**. What it is worth to a thief is what the broker allows that
instance: its repositories, the operations of § 3, for as long as the control plane lets it
live. The control plane can end it without touching a forge, can limit it to the pod's network
address, and sees every use.

Ways to give it to the pod, each with what it changes here:

- **a. A bearer value per instance, named in the mapping as `forge_token` is named today**
  (`secret:<name>` of the daemon's secrets directory), exported under a variable of its own.
  Changes nothing about where secrets rest; the control plane cannot set it without whoever
  installs the daemon, which is the limit § 1 starts from.
- **b. The same value by the reference of § 4.2** (a labelled Secret of the pods' namespace).
  Then § 4.1 and § 4.2 are needed under variant B as well, for this one value.
- **c. The runtime's own API token as the proof.** The daemon mints it at every pod start and
  the control plane can ask the daemon which instance holds which token. Needs a call by which
  the broker checks a token with the daemon, and it makes the runtime's token worth more than
  it is today.
- **d. A token Kubernetes signs for the pod.** A pod mounts no service account token today
  (`daemon/pods.rs:459`), by rule; this would lift that rule for one projected token with the
  broker as its audience. **Not shown.**

git gets the proof through its credential helper for the proxy's origin, as it gets the forge
token today; a session's calls to the broker get it from the same file or variable.

### 5.4 When the broker or the proxy does not answer

No fallback to the forge: a brokered pod holds nothing it could fall back with.

- Collectors: a read that fails is asked once more and then fails its collector with an error
  that names the repository and the command, and that collector records nothing for the pass,
  as for a forge that does not answer today (`collect/github.rs:53-57`).
- A session's write: the command fails with the broker's status; the session reports `blocked`
  with that line, as it does for a refused push today. Nothing is queued inside the pod.
- The health answer names the broker as the member that failed, so a control plane can tell its
  own outage from a forge's.

### 5.5 An installation without a control plane

Conductor also runs with no control plane: on a workstation, and as a daemon whose instances
push with `forge_token`. Variant B adds a route; it removes none. `forge.route: direct` and
`gates`, the `github` and `gitlab` sources, `forge_token` and the forge CLIs in the image stay as
they are, and an instance that names no broker is unchanged.

### 5.6 What of variant R is still needed under variant B

- § 4.3, minting in the daemon: not needed. Whatever the broker does with an App's key is the
  control plane's.
- § 4.4, the refresh: not needed if the pod's proof is long-lived or is 5.3 c; needed in the
  form of route A or B if the control plane rotates a value of 5.3 a or b under a running pod.
- § 4.1 and § 4.2, the label rule and the reference: needed only with 5.3 b.
- The git identity of an instance's commits (a story of its own): needed under both variants,
  since commits are made in the pod.

### 5.7 To be shown before variant B is built

Nothing below has been run. Each is a test with a forge double and a proxy double before any
unit of variant B is accepted:

- `git clone`, `git fetch` and `git push` through a proxy that adds the credential, with the
  proof of § 5.3 as the only thing the pod holds;
- a push of the size the forge accepts, through the proxy, and what the proxy's own limit is;
- LFS, if a repository of an instance uses it (the image holds no `git-lfs` and no code calls it
  today);
- which account the forge records as the pusher and as the author of a request, and whether
  the forge's rules on commit identity still pass;
- under B2: `gh` and `glab` against the proxy for every read of § 3;
- what a session can still reach on the forge directly, without the broker, from inside the pod
  (an anonymous clone of a public repository is always possible).

## 6. The two variants side by side

Counts of units are estimates, not measured.

- **What the pod holds.** R: a forge token (one hour and the instance's repositories for an
  App; a person's token otherwise). B: a proof of which instance it is, worth what the broker
  allows.
- **Who decides an operation.** R: the forge, by the token's permissions and its own rules. B:
  the broker first, then the forge.
- **Where a record of each write is.** R: the forge. B: the broker and the forge.
- **What conductor changes.** R: 2 units (the reference; minting and refresh) and the wrapper
  in the image. B1: 5 units (the source with remotes; the collector backend; the route with
  guard, role text and command; the checks for "no token"; the proof), B2: 3 (source, route,
  checks and proof together with them) if its pointing is shown.
- **What the control plane builds.** R: writes a labelled Secret. B: a broker, a git proxy,
  their rules per instance and their operation.
- **What rests on things not shown.** R: the kubelet's delay, signing with `ring`. B: § 5.7.
- **What changes for an installation without a control plane.** Neither changes it.

The variants do not exclude each other: R is a credential for the routes of today, B is a third
route. An instance could start on R and move to B when § 5.7 is shown.

## 7. What does not change, under either variant

An instance whose mapping holds no `forge_credential` and names no broker gets the pod it gets
today: the same variables, the same helper text, the same Secret keys, the same digest.
`forge_token` keeps its meaning and its refusals. The daemon's own secrets directory is
untouched. A daemon whose Role has no `update` runs every instance it runs today.

## 8. Not shown, and not in this design

- Not shown under R: a push with a minted token against a real forge; the kubelet's delay on a
  cluster; `gh` and `glab` behind the wrapper in a real session; RS256 signing with `ring` (its
  documentation says it can; no code here uses it yet). Under B: everything in § 5.7.
- Not here: creating an App, installing it, or any rule of a forge that admits its pushes (the
  operator's acts, outside conductor); the broker's and the proxy's own design, which is the
  control plane's; a GitLab equivalent of minting; several forge credentials per instance; a
  pool of model tokens (it would use the reference and the label rule of § 4).

## 9. Decisions asked of the operator

- **V, the variant.** R: a reference to a Secret, with minting for an App (§ 4). B: a broker of
  the control plane (§ 5), as B1 or B2. R then B: the first instance runs on R, and B is built
  once § 5.7 is shown (proposed: it is the shortest way to a first instance that pushes, and it
  keeps the stronger separation as the direction).

Under R only:

- **D-1, the namespace rule** (§ 4.1). A: the pods' namespace also holds labelled Secrets of a
  control plane, with one control token as one trust domain (proposed). B: referenced Secrets
  live in a third namespace, which costs the daemon a second Role and changes nothing about who
  can name them.
- **D-2, the refresh route** (§ 4.4). A: replace the instance's Secret in place (proposed). B: a
  call to the runtime.
- **D-3, the Role** (only with D-2 A). The Role of the guide gains `update` on Secrets; an
  installer who does not grant it cannot run `github-app` instances and loses nothing else.
- **D-4, the permissions of a minted token.** A: contents write, pull requests write, metadata
  read, and nothing else (proposed). B: A plus checks read and actions read, so the instance can
  read the state of the repository's checks with the same token (reads 2 and 5 of § 3). Which of
  the instance's reads fail under A on a private repository is **not shown**.
- **D-5, `gh` and `glab`** (§ 4.4). A: the wrapper in the image (proposed). B: no wrapper; the
  guide says that a session's `gh` and `glab` stop working one hour after the pod's start on a
  `github-app` instance and that only git keeps working.

Under B only, to settle with the control plane's side before a unit is written: B1 or B2; which
of 5.3 a to d proves the instance; a fixed list of session reads or a generic one.
