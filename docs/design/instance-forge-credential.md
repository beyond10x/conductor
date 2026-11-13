# Design: how an instance that runs as a pod reaches its forge

Status: proposal, not accepted; fourth revision, 2026-11-13. Nothing here is built and there is no
store record yet. The direction proposed is a forge gateway of a control plane (§ 4), which
sessions reach through the forge verbs of conductor's tool broker. The decisions asked of the
operator are in § 9.

Paths without a crate are under `crates/conductor/src/`, at tag `v1.6.0` (`700e1caa0d`). "The
guide" is `website/docs/guides/run-in-kubernetes.md` at that tag. A statement about Kubernetes, git
or a forge that was not read in this repository or run is marked **not shown**; a thing the design
rests on that nobody has run is marked **to be shown**.

Words: the **instance's Secret** is `conductor-instance-<name>`, which the daemon writes at every
pod start. The **pods' namespace** is the namespace of the instance pods, which is not the
daemon's. The **forge gateway** (the gateway) is a service of a control plane that holds forge
credentials and performs forge operations for an instance; its **git proxy** (the proxy) serves
git. The **tool broker** (the broker) is conductor's own broker of one instance
(`docs/design/roles-and-surfaces.md` § 6, `crates/conductor-broker`), and the word names nothing
else here.

## 1. Why, and the operator's requirements

A control plane stores the forge credential of the repositories a person adds, and wants an
instance to work on them. At `v1.6.0` an instance names `forge_token: secret:<name>`, and the name
resolves against the daemon's secrets source only, "never a Kubernetes Secret by name"
(`spec/domains/config.yaml:791-792`). So the credential has to be put into the daemon's own Secret,
by whoever installs the daemon, before a control plane can name it.

Every variant in this note is measured against these requirements:

- **Q1.** The operator's requirement: "conductor will never see the bot token".
- **Q2.** The operator's requirement: "while adding their own repo, that's where we would add the
  secret for that repo": this note reads it as a secret that belongs to a repository.
- **Q3.** The operator's requirement: "this 1 hour limitation, it's bad": work must not stop when a
  token expires. A GitHub App's installation token lives one hour (the forge's documentation; not
  run here).
- **Q4.** The operator's requirement: "a user needs to manage their own secrets".
- **Q5.** The operator's requirement: "when it's pushing, it does need the actual [secret] because
  it's using Git from there. Maybe it should be using a Git proxy server".
- **Q6.** The operator's decision, an answer chosen among lettered options before Q1 was stated,
  not a typed sentence: the credential is a GitHub App
  installed on chosen repositories; its private key is the secret.

| | the gateway (§ 4) | a reference with minting in the daemon (§ 6) |
|---|---|---|
| Q1 | met for a working instance from step 3 (§ 4.7): neither the daemon nor a pod holds a forge credential | not met: the daemon reads the App's key, each pod holds a forge token |
| Q2 | met: a remote and endpoints per repository | not met: one credential per instance |
| Q3 | met in conductor: the gateway renews what expires | met only by minting and refresh in the daemon |
| Q4 | no user's secret enters conductor; keeping them apart is the gateway's | one control token can name any labelled Secret for any instance |
| Q5 | met: git through the proxy, the first step (§ 4.7) | git with a forge token in the pod |
| Q6 | the key rests with the control plane | the key rests in the pods' namespace |

## 2. What is there

- An instance's own forge token comes from the instance's Secret into its pod
  (`daemon/pods.rs:339-379`). git's credential helper offers it to the forge origin of each of the
  instance's sources, `https://github.com` or the host of a `gitlab` source
  (`daemon/pods.rs:134-158`). The daemon's own GitLab token goes only to pods whose `gitlab`
  sources are all on its one host, so that no host a caller names receives it
  (`daemon/pods.rs:348-358`; the guide, "Forge credentials").
- A pod that carries no forge token does nothing against its forge: it tries no clone and reports
  each repository as a failure (`collect/checkouts.rs:249-260`), git gets no credential helper
  (`daemon/pods.rs:136-139`), the doctor says "not logged in" and the health answer is not ready
  (`doctor.rs:262-275`, `:2267-2270`).
- The remote of a repository is derived, not given: `https://github.com/{owner}/{name}.git` for a
  `github` source, `https://{host}/{group}/{path}.git` for a `gitlab` one
  (`collect/checkouts.rs:143`, `:171`). No member of a mapping names a remote per repository.
- No text of a caller reaches the pod's spec: every field comes from the daemon's configuration,
  the instance's name and the rendering (`daemon/pods.rs:31-36`; held by
  `tests/sec_instance_pods.rs:357`). One text of a caller reaches git's configuration: the host
  of a `gitlab` source, as the key of git's helper (`daemon/pods.rs:126-133`).
- The pod's digest holds the spec, the rendered file and the names of the secrets, no value
  (`daemon/pods.rs:513-530`). A changed name replaces the pod; a running pod keeps the value it
  started with (the guide, "Forge credentials").
- The daemon calls a pod's API (`daemon/reconcile.rs:1768-1777`) with a token it mints for the
  runtime at each start (`daemon/reconcile.rs:352-355`). Nothing in a pod calls the daemon, and a
  pod mounts no service account token (`daemon/pods.rs:459`). A pod has one container, which runs
  the runtime and its sessions as one user (`daemon/pods.rs:465-472`).
- The daemon's own secrets are refused in a mapping (`config.rs:2737-2753`) and again at pod start
  (`daemon/pods.rs:783-806`); the calls answer `422` `conductor.config.ConfigInvalid` with the YAML
  path (`daemon/api.rs:1855-1874`).

## 3. The forge operations of v1.6.0: the gateway's contract

This list is what an instance does against a forge today, and what the gateway and the proxy have
to serve. Call sites are in brackets.

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
`gh pr list` and a GraphQL query for tags (`collect/github.rs:96`, `:168`). Conductor's code also
runs the `worktree` tool, `finish` and `gc` among its commands (`gc/trees.rs:400-511`, `:478`,
`:503`).

Reads a session makes have no fixed list today: the guard admits every read verb of `gh` and
`glab` and every `api` call that is a GET (`guard/rules.rs:719-742`, `:3216-3225`, `:3745`), and
the controller's role text tells it to read the log of a failing check (`roles/controller.md:35`).
A session's git reaches a remote too: `git fetch`, `git pull` and `git clone` are writes to the
guard, admitted where they land in the session's checkout, its managed trees or its scratch
(`guard/rules.rs:2180-2183`, `:2256-2260`, `:2435`), and `git ls-remote` is a read (`:2365`).

Writes. The code makes one: it pushes the `main` of the instance's records where that repository
has a remote (`run/records.rs:219`). Under `forge.route: gates` that push runs as
`b10x-gates bot -- push origin main` and does not go through git's helper
(`run/records.rs:380-391`).

A session makes the others, told by its role text. The guard admits a write of `gh` or `glab` for
the session's own repository named with `-R` (`guard/rules.rs:776-784`), the bot route's writes for
it (`:767-774`), and judges a git command that writes, `push` among them, by the directory it runs
in (`:2180-2183`). Under this design the proxy's rule per repository is what confines a push.

1. push a branch (`roles/controller.md:35`, `:88-91`);
2. delete a branch: at a merge (`gh pr merge --delete-branch`; `should_remove_source_branch` on a
   GitLab merge, `guard/rules.rs:806`) and by `git push --delete`, a git write as above;
3. open a pull or merge request (`roles/controller.md:49`, `guard/rules.rs:780`, `:784`);
4. change its description (`roles/controller.md:58-60`);
5. comment on it (`guard/rules.rs:780`, `:784`);
6. mark it ready (`guard/rules.rs:780`);
7. merge it or set it to merge when its checks pass, under a charter with `merge: controller`
   (`roles/controller.md:51`, `guard/rules.rs:780`, `:784`, `:800`);
8. run a workflow or a pipeline again, or start one (`guard/rules.rs:770-773`, `:789`, `:794`);
9. create a tag and a release, upload its assets, edit and view it through the bot route
   (`roles/controller.md:41`, `guard/rules.rs:770-771`);
10. push a change to workflow files, for which the forge asks a permission of its own (**not
    shown**); the gateway refuses it unless it admits the instance for releases;
11. close a request (role texts of the host roles; not a pod role today); the gateway admits it
    for a request the instance opened.

Host roles, not pod roles today, also read across a whole group
(`roles/supervisor.md:270`, `roles/teacher.md:66`) and start pipelines through `glab api`
(`roles/supervisor.md:459`). A gateway per repository does not serve a read across a group.

Not determined in this repository: what `b10x-gates` calls and with which credential under
`forge.route: gates`; whether the `worktree` tool reaches a remote; which forge calls a session's
loaded skills make.

## 4. The direction: a forge gateway of a control plane

### 4.1 The shape

The control plane keeps every forge secret. An instance's definition lists its repositories and
gives, per repository, a git remote at the gateway's proxy and the address of the gateway's
endpoints for it. No forge secret and no reference to one reaches the daemon or a pod (Q1). A
secret belongs to the repository a person adds (Q2, Q6); whatever short-lived token the forge asks
for, the gateway mints and renews, so nothing in a pod expires with it (Q3).

The gateway offers the operations of § 3 as **typed endpoints**, by name ("open a request for this
branch", "merge this request"). It decides each by its meaning, can refuse a merge into a protected
branch, and records who asked for what. It appends a footer that names the instance to a
request's description and keeps the session's text. The proxy decides per ref: a push lists each ref's old and
new value before the pack, so the proxy can refuse a push to the default branch, to a tag, or a
deletion (**not shown**).

Not proposed: a proxy that speaks the forge's own API, decided by path and method. On GitHub one
GraphQL endpoint carries reads and writes for every repository of the credential, and conductor's
collectors use it (`collect/github.rs:96`, `:280`); such a proxy cannot keep a pod inside its
repositories there (**not shown** for `gh`'s own use of that endpoint).

### 4.2 The session side: the forge verbs of the tool broker

A session reaches the gateway through the tool broker, not through a new command and not through
a second broker.

- **The approved design.** `docs/design/roles-and-surfaces.md` is approved (`:3`). Its broker
  "holds what the session must not have: the forge credentials, the bot route" (`:123`), knows the
  caller by the session's own socket (`:128`, § 6.2), checks each call against a grant that names
  the session's repositories (`:127`), and plans the verbs `push` (through the gates) and
  `pr open|checks|comment`, `ci rerun`, `release` (`:143-144`).
- **What is built.** Nine verbs: `show`, `search`, `diff`, `test`, `check`, `fmt`, `commit`,
  `send`, `report` (`crates/conductor-broker/src/lib.rs:53-63`). `push`, `pr`, `ci` and `release`
  are not built. A controller session is registered with the broker and given its socket at its
  start (`spawn.rs:439-454`); `conductor broker serve` serves the sockets (`broker.rs:55-61`).
- **Cost a.** Nothing in the code starts `conductor broker serve`: its one caller is the command
  line (`cli.rs:524-526`). On a host the `broker` task of `Taskfile.yml` starts it; nothing does
  in a pod. Under this design one runs in the pod beside the runtime.
- **Cost b.** Sessions call `gh`, `glab` and `git` through the harness's shell today: the
  controller's role text names `gh pr`/`glab mr -R <origin>` and `git push` for
  `forge.route: direct` (`roles/controller.md:88-89`, `:112-113`), and the guard reads those
  commands on a shell line (`guard/rules.rs:719-806`). Moving forge calls to `tool` verbs changes
  the role texts and the guard.
- **Against § 6.1 of that design.** There the broker holds the forge credentials. Here it holds
  only the pod's proof for the gateway (§ 4.4), and the gateway holds the credentials. The broker
  refuses a verb outside the session's grant before it leaves the pod; the gateway's policy per
  instance decides again.
- **Two rules for a merge, a tag and a release.** A merge needs the charter's `merge: controller`
  on conductor's side, as today (`roles/controller.md:51`), and the gateway's own admission of a
  merge for that instance. A tag and a release need the gateway's admission for the instance,
  which it refuses unless the control plane says otherwise.

### 4.3 What conductor needs on its side

1. **A remote per repository in the instance definition.** A new kind of source lists
   repositories by name, each with its remote at the proxy and its forge kind, so nothing is
   listed from the forge at start. Declared in `spec/`; an instance without it is unchanged.
2. **Its own reads through the gateway.** A third collector backend beside the `github` and
   `gitlab` ones, for reads 1 to 12 and 14 of § 3; 15 and 16 go to the proxy; 17 needs no
   credential. Read 13 is left out of the first contract: it serves only an instance that syncs
   secrets.
3. **The broker's forge verbs,** each handing its call to the gateway: the design's `push`,
   `pr open|checks|comment`, `ci rerun` and `release`, and what § 3 needs beyond them (a
   request's description, ready, merge, a branch's deletion).
4. **The session's reads: a fixed list,** agreed with the control plane's side: the instance's
   repositories, reads 2 to 12 of § 3 for each of them, a request's view, diff, comments and
   reviews (under `merge: human` a person reviews the request, `roles/controller.md:48-51`), and
   the log of a failing check (`roles/controller.md:35`). A read outside the list is refused and
   the session reports `blocked`; the list grows by agreement.
5. **Role texts and guard narrowed to the broker's verbs:** no `gh` or `glab` write and no push to
   a remote other than the proxy's.
6. **Everything that takes a missing token for "no forge".** The clone at start
   (`collect/checkouts.rs:249-260`), git's helper (`daemon/pods.rs:136-139`), the doctor's login
   row and the health answer (`doctor.rs:262-275`, `:2267-2270`) each get a branch for an
   instance on the gateway; what the login row reads then is open (§ 8).
7. **The records' push.** The first contract leaves it out: an instance on the gateway keeps its
   records without a remote. Where they get one later, under `forge.route: direct` the push goes
   to the proxy like any push; under `gates` it runs through `b10x-gates` and not git's helper
   (`run/records.rs:380-391`), and how that route reaches the proxy is open (§ 8).
8. **The git identity of an instance's commits:** commits are made in the pod. It is a story
   of its own, accepted apart from this note and outside the steps of § 4.7.

Item 1 is step 1 of § 4.7. Of item 6, the clone at start and git's helper are step 1, the
doctor's row and the health answer step 2. Item 7 is outside the four steps.

### 4.4 How the pod proves which instance it is

The gateway and the proxy must know which instance calls, or any pod could use any repository. So
something that identifies the instance reaches the pod, and that thing is what a pod can leak in
place of a forge token. It is worth what the gateway allows that instance, for as long as the
control plane lets it live; the control plane can end it without touching a forge and sees every
use.

The requirement: **the proof rests where no session can read it.** A pod runs the runtime and its
sessions in one container as one user (`daemon/pods.rs:465-472`), so this needs a separation: the
tool broker as its own container or user, with the proof mounted only there (**not shown**). The
requirement is met from step 3 of § 4.7, not before:

- **Steps 1 and 2.** git in the pod reaches the proxy with the proof through a credential helper
  that reads the proof's file at every call (the proof rotates; today's helper prints a
  variable, `daemon/pods.rs:134-158`). So the proof is within a session's reach, as a forge
  token would be. What bounds it: the proxy's rule per ref, the proof's own lifetime, and this
  rule for the gateway: **until step 3 the gateway admits only git and reads for an instance,**
  no endpoint that writes. Otherwise a session could call a merge with the proof and pass by the
  charter's rule of § 4.2.
- **From step 3.** The proof is mounted only into the broker's container, and the gateway's
  writing endpoints open for it. git in the runtime's container, the collectors' and a
  session's alike, then reaches the proxy through a local endpoint of the broker's container on
  the pod's loopback, which adds the proof and forwards. That endpoint knows the instance, not
  the session, so a session's grant does not narrow git: the proxy's rule per ref does. **To be
  shown** (§ 4.8).

Which proof it is has to be decided before step 1, since git uses it from step 1.

| proof | lifetime | rotation | tie to the pod | kept from a session |
|---|---|---|---|---|
| a. a bearer value per instance, named as `forge_token` is (`secret:<name>`) | as long as the control plane lets it | through whoever installs the daemon; reaches the pod at its next start (`daemon/pods.rs:513-530`) | none: it works from anywhere until revoked | only with the separation |
| b. the same value by a reference to a labelled Secret of the pods' namespace | as a | the control plane writes it; reaches the pod at its next start | none | only with the separation; brings back the label rule and the reference of § 6 |
| c. the runtime's own token, minted at each start (`daemon/reconcile.rs:352-355`) | one start of the pod | at each start | to one start, by a new call that asks the daemon | from step 3: requires that only the runtime and the broker's container hold it |
| d. a token Kubernetes signs for the pod, with the gateway as its audience | expiring (**not shown**) | by Kubernetes (**not shown**) | invalid once the pod is gone (**not shown**) | from step 3: mounted only into the broker's container |

Proposed: **d**, checked by the gateway with a TokenReview, which answers the pod, whose name is
`conductor-instance-<name>` (`daemon/pods.rs:219-221`) (**not shown**). Its costs: the rule of
`daemon/pods.rs:459` is lifted for one projected token; the pod gets a projected volume and the
file-reading helper above; and the gateway needs access into the pods' cluster, the right to
create a TokenReview there or a way to reach that cluster's token issuer, which the control plane
has to be given (**not shown**).

### 4.5 When the gateway or the proxy does not answer

No fallback to the forge: a pod on the gateway holds nothing it could fall back with.

- Collectors: a read that fails is asked once more and then fails its collector with an error
  that names the repository and the command, and that collector records nothing for the pass,
  as for a forge that does not answer today (`collect/github.rs:53-57`).
- A session's verb fails with the gateway's status; the session reports `blocked` with that line,
  as it does for a refused push today. Nothing is queued inside the pod.
- The health answer names the gateway as the member that failed, so a control plane can tell its
  own outage from a forge's.

### 4.6 An installation without a control plane

Conductor also runs with no control plane: on a workstation, and as a daemon whose instances
push with `forge_token`. The gateway adds a route; it removes none. `forge.route: direct` and
`gates`, the `github` and `gitlab` sources, `forge_token` and the forge CLIs in the image stay as
they are, and an instance that names no gateway is unchanged.

### 4.7 Steps

Each step is one story. After each, an instance on the gateway, which names no forge token, can
do this much:

1. **A git remote per repository through the gateway's proxy,** the proxy deciding per ref
   (§ 4.1; § 4.3 item 1 and the git half of item 6; the proof in the pod with its helper, § 4.4).
   *Then:* it clones, fetches and pushes branches. Its collectors read nothing from the forge,
   its health answer is not ready, and a session opens no request. Q5 holds.
2. **Conductor's own forge reads through the gateway** (§ 4.3 item 2 and the rest of item 6).
   *Then:* its collectors, doctor and health answer work. A session still opens and merges no
   request.
3. **The tool broker's forge verbs, handing calls to the gateway,** with the broker in its own
   container, the proof only there, and the local git endpoint (§ 4.2, § 4.3 item 3, § 4.4).
   *Then:* a session opens, describes and merges a request. This is the first step after which
   an instance does its whole work with no forge token, so Q1 holds for a working instance from
   here, and from here no session can use the proof.
4. **Role texts and guard narrowed to those verbs** (§ 4.3 items 4 and 5). *Then:* a session has
   no forge command line left to call.

Between steps 1 and 3 an instance could run on both: git through the proxy and `forge_token`
for `gh` and `glab`. That is against Q1, it is not proposed, and it does not help a repository
that only a GitHub App may push to (§ 5).

The control plane's side is not in this list and is the larger part: the gateway, the proxy,
minting, the policy per instance, and their operation.

### 4.8 To be shown before the gateway is built

Nothing below has been run. Each is a test with a forge double and a proxy double before the step
that rests on it is accepted:

- `git clone`, `git fetch` and `git push` through a proxy that adds the credential, with the proof
  of § 4.4 as the only thing the pod holds; and the proxy's refusal per ref;
- a push of the size the forge accepts, through the proxy, and what the proxy's own limit is;
- LFS, if a repository of an instance uses it (the image holds no `git-lfs` and no code calls it);
- which account the forge records as the pusher and as the author of a request, and whether the
  forge's rules on commit identity still pass;
- a credential helper that reads a rotating token from a file at every call;
- the broker in its own container or as its own user, with the projected token mounted only there;
- git in the runtime's container through the broker's local endpoint: clone, fetch and push, by
  the collectors and by a session;
- what a session can still reach on the forge directly, without the gateway, from inside the pod
  (an anonymous clone of a public repository is always possible).

## 5. Until step 3 exists

- At `v1.6.0` an instance pod pushes with `forge_token: secret:<name>` (the guide, "One token per
  instance"; `daemon/pods.rs:339-379`). It needs no new code: the installer puts the value into the
  daemon's Secret, and the value is fixed at pod start (`daemon/pods.rs:513-530`; the guide,
  "Forge credentials").
- A repository whose rules let only a GitHub App create and update branches can be pushed only
  with an installation token of that App, which lasts one hour (the forge's documentation; not
  run here). `forge_token` cannot serve such a repository beyond an hour.
- An instance on the gateway does its whole work only after step 3 (§ 4.7). So before step 3
  exists, such a repository has four ways, set out with their costs in § 9. Proposed: A, to
  wait for step 3.

## 6. Alternative not proposed: a credential by reference with minting in the daemon

What it is. A mapping names a Kubernetes Secret that a control plane wrote into the pods'
namespace with a label the daemon checks. The daemon copies its value into the instance's Secret
under a fixed key, so no caller's text reaches the pod's spec. For a GitHub App the referenced
Secret holds the App's private key: the daemon signs a JWT, mints an installation token for the
instance's repositories, and before the hour is out mints again and replaces the instance's Secret
in place, so the file the pod mounts (`config/pod.rs:44-46`) follows. That needs `update` on
Secrets, a call the daemon's client lacks (`kube.rs:340-382`), and a git helper and a wrapper for
`gh` and `glab` that read that file.

This is the shape the operator chose first, with Q6: the instance makes its own one-hour tokens
from the App's key. Requirement Q1, stated later, rules it out except as an interim.

Why it is not proposed:

- **Q1.** The daemon reads the App's private key, and each pod holds a forge token.
- **Q2.** One forge credential per instance; a secret per repository has no place in it.
- **Q4.** Whoever holds the control call can name any labelled Secret for any instance. The label
  is a rule the daemon keeps, not a boundary Kubernetes enforces. With a GitLab or raw value and a
  `gitlab` source on a host of the caller's choice, git's helper offers the value to that host
  (`daemon/pods.rs:134-158`): the path the guide's `GITLAB_HOST` rule closes for the daemon's own
  token (`daemon/pods.rs:348-358`; the guide, "Forge credentials").
- **Q6.** The App's key would rest in the pods' namespace, where the daemon's `create` on pods
  starts a pod that may name any Secret of the namespace (the guide, "These rights reach
  further").
- **The pod's own check.** `carries_token` reads the runtime's own environment
  (`collect/checkouts.rs:263-271`); an instance whose token comes from a file would clone nothing
  until that check changes.
- **Lifetime.** The last minted token outlives the pod by up to an hour, and no revocation is
  described.
- **What remains of it.** Under the gateway its minting, refresh, wrapper and label rule are all
  dropped; it would be built to be removed.

Its five decisions of the second revision, D-1 to D-5 (the namespace rule, the refresh route, the
Role's `update`, the minted token's permissions, the wrapper), leave the operator's list. They
return with interim B or D of § 9, should the operator choose one.

Whether a reference to a labelled Secret is built for another kind of secret, a model token
among them, is a separate question, open, and not decided by this note.

## 7. What does not change

An instance whose mapping names no gateway gets the pod it gets today: the same variables, the
same helper text, the same Secret keys, the same digest. `forge_token` keeps its meaning and its
refusals. The daemon's own secrets source is untouched, and the guide's Role gains no verb on
Secrets.

## 8. Not shown, and not in this design

- Not shown: everything in § 4.8; the per-ref decision of the proxy; a projected token mounted
  into one container only and checked by a TokenReview; git through the broker's local endpoint;
  the forge's own permission for workflow files.
- From the forge's documentation, not run here: the one-hour lifetime of an installation token,
  and branch rules that admit only an App as their bypass actor.
- `gh auth status` behind the gateway is moot under typed endpoints: a pod on the gateway has no
  forge login to check. What the doctor's login row and the health answer read under the gateway
  instead (`doctor.rs:262-275`, `:2267-2270`) is open, and part of step 2.
- How the records' push of `forge.route: gates` (`run/records.rs:380-391`) reaches the proxy, and
  whether the `worktree` tool reaches a remote.
- Not here: creating an App, installing it, or any rule of a forge that admits its pushes (the
  operator's acts, outside conductor); the gateway's and the proxy's own design, which is the
  control plane's; reads across a group for host roles; several gateways per instance.

## 9. Decisions

Asked of the operator:

- **V, the direction.** Proposed: the gateway (§ 4), built in the steps of § 4.7. With it, the
  question of § 5, for a repository that only a GitHub App may push to, before step 3 exists:
  - **A.** Wait for step 3. Cost: no working instance on such a repository until three of the
    four stories of § 4.7 and the control plane's side of them are built; every other repository
    can run on `forge_token` meanwhile. Proposed.
  - **B.** Build the minting and refresh of § 6 as an interim: the shape chosen first, with Q6.
    Cost: JWT signing, minting, a new Role verb, a helper and a wrapper that read a file; the
    daemon reads the App's key and the pod holds a forge token, against Q1; removed once step 3
    exists.
  - **C.** A long-lived token on `forge_token`, with the repository's owner changing its rules to
    admit it. Cost: no code; the pod holds a long-lived forge token, against Q1 for that
    repository; the owner admits a token other than the App's, in place of Q6 there.
  - **D.** The control plane keeps the App's key, mints, and writes only the one-hour token for
    the instance's repositories into a labelled Secret of the pods' namespace; the daemon copies
    it into the instance's Secret. Cost on conductor's side: the reference with its label rule
    and the whole refresh route of § 6 (`update` on Secrets, without which such an instance is
    refused at start; a helper and a wrapper that read a file), everything of § 6 but the
    minting; the pod holds a one-hour forge token and the daemon handles it, against Q1 as
    worded, though neither sees the key; removed once step 3 exists. Not proposed.
- **P, the pod's proof** (§ 4.4), to be decided before step 1. Proposed: d, a projected token
  with the gateway as its audience. Cost: the control plane is given access into the pods'
  cluster to check the token, and the pod gets one projected token. a and b are replayable from
  anywhere and need the installer or the label rule; c needs a new call to the daemon.

Settled in this note:

- Typed endpoints, not a proxy of the forge's API: GraphQL on GitHub defeats a rule by path (§ 4.1).
- The session side is the tool broker's forge verbs: it is the approved design (§ 4.2).
- Session reads are a fixed list that grows by agreement: the gateway's contract stays enumerable
  (§ 4.3 item 4).
- The steps of § 4.7, in that order: git through the proxy answers Q5 first and needs no new verb.
- D-1 to D-5 of the second revision leave: they belong to the alternative of § 6.
- No count of work is given beyond the stories of § 4.7: the control plane's side is not sized here.
