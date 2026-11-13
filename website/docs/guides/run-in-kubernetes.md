---
title: Run conductor in Kubernetes
sidebar_position: 11
description: Run conductor's daemon in a Kubernetes namespace from the release image, let a control plane list, add, replace, restart and remove instances over HTTP with a control token, and read what the daemon creates for each instance, which runs as a pod of its own.
lede: The daemon runs as a StatefulSet of one from the release image. A control plane adds instances over HTTP, and the daemon runs each instance's runtime as a pod of its own, with a Secret and a volume claim beside it.
source: crates/conductor/tests/fixtures/k8s/manifests.yaml; crates/conductor/src/daemon/pods.rs, daemon/api.rs, daemon/seed.rs, daemon/mod.rs, config/pod.rs, config/instance.rs, kube.rs, doctor.rs, dashboard/work.rs, run.rs, run/dispatch.rs, run/mailbox.rs, spawn.rs, spawn/charter.md; crates/conductor/tests/kubernetes_guide.rs, daemon_pods.rs, instance_routes.rs, work_route.rs, work_start.rs
---

## What runs where

| what | runs as | made by |
|---|---|---|
| the daemon, `conductor daemon serve` | a StatefulSet of one replica, from the release image, with a volume of its own | the manifests below, which you apply |
| each instance's runtime, `conductor instance serve` | a pod of its own, `conductor-instance-<name>`, from the image `daemon.kubernetes.image` names | the daemon, through the Kubernetes API |
| the instance's config file, its API token, its secrets | a Secret `conductor-instance-<name>` | the daemon |
| the instance's checkouts, records, state, cache and store | a volume claim `conductor-instance-<name>-data` | the daemon |

A control plane calls the daemon over HTTP on its Service with a control token. It never calls
the Kubernetes API for an instance and never shapes the pod: the daemon builds the pod, the
Secret and the claim from its own config and the instance's name.

A pod is one more way the daemon supervises a runtime, beside the systemd unit and the child
process. It is not a placement and not a sandbox: every instance of such a daemon writes
`placement: none`, and a placement that opens a box is refused.

## The image

The daemon and every instance pod run the release's container image, the one
[Run conductor in a container](./run-in-a-container.md) describes:
`ghcr.io/beyond10x/conductor:<version>`, or by digest. The examples below write
`registry.example.com/conductor:<tag>`; put the reference of your registry there, twice: once as
the StatefulSet's image, once as `daemon.kubernetes.image`.

The daemon passes `daemon.kubernetes.image` to each instance pod as written, so a reference by
digest (`registry.example.com/conductor:<tag>@sha256:<digest>` or
`registry.example.com/conductor@sha256:<digest>`) starts the pods from that digest. An instance's
`binary_version` is read from the reference, not from the running binary: the tag where the
reference has one, the first 19 characters of the digest (`sha256:` and 12 more) where it has a
digest only, `latest` where it has neither.

## The manifests

Conductor ships no chart and no install manifests. These are an example a control plane's own
chart can follow. They use two namespaces, each enforcing the `restricted` pod security level:

- `conductor` holds the daemon: its service account, its Secret, its config, a Service and the
  StatefulSet;
- `conductor-instances` holds the instance pods and nothing else besides what the daemon makes
  there: the Role and RoleBinding that give the daemon's service account its rights there, and a
  network policy for the instance pods.

Keep the instances' namespace for them alone: [the rights](#the-rights-and-why-each-verb) say why.

```yaml
apiVersion: v1
kind: Namespace
metadata:
  name: conductor
  labels:
    pod-security.kubernetes.io/enforce: restricted
---
apiVersion: v1
kind: Namespace
metadata:
  name: conductor-instances
  labels:
    pod-security.kubernetes.io/enforce: restricted
---
apiVersion: v1
kind: ServiceAccount
metadata:
  name: conductor
  namespace: conductor
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: conductor-instances
  namespace: conductor-instances
rules:
  - apiGroups: [""]
    resources: [pods]
    verbs: [create, get, list, delete]
  - apiGroups: [""]
    resources: [pods/log]
    verbs: [get]
  - apiGroups: [""]
    resources: [secrets]
    verbs: [create, get, list, update, delete]
  - apiGroups: [""]
    resources: [persistentvolumeclaims]
    verbs: [create]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: conductor-instances
  namespace: conductor-instances
subjects:
  - kind: ServiceAccount
    name: conductor
    namespace: conductor
roleRef:
  apiGroup: rbac.authorization.k8s.io
  kind: Role
  name: conductor-instances
---
apiVersion: v1
kind: Secret
metadata:
  name: conductor-secrets
  namespace: conductor
stringData:
  control-token: <a long random value>
  model-token: <the model token>
---
apiVersion: v1
kind: ConfigMap
metadata:
  name: conductor-daemon-config
  namespace: conductor
data:
  daemon.yaml: |
    version: conductor.config/2
    daemon:
      address: 0.0.0.0
      port: 7300
      store: sqlite:/var/lib/conductor/data/conductor.sqlite
      secrets: {source: files, dir: /run/secrets/conductor}
      control_token: secret:control-token
      kubernetes:
        namespace: conductor-instances
        image: registry.example.com/conductor:<tag>
        owner: example-org
        api_port: 7301
        volume: {size: 10G}
---
apiVersion: v1
kind: Service
metadata:
  name: conductor
  namespace: conductor
spec:
  selector:
    app.kubernetes.io/name: conductord
  ports:
    - {name: http, port: 7300, targetPort: http}
---
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: conductor
  namespace: conductor
spec:
  serviceName: conductor
  replicas: 1
  selector:
    matchLabels:
      app.kubernetes.io/name: conductord
  template:
    metadata:
      labels:
        app.kubernetes.io/name: conductord
    spec:
      serviceAccountName: conductor
      shareProcessNamespace: true
      securityContext:
        runAsNonRoot: true
        runAsUser: 10001
        runAsGroup: 10001
        fsGroup: 10001
        seccompProfile: {type: RuntimeDefault}
      containers:
        - name: conductor
          image: registry.example.com/conductor:<tag>
          command: [conductor, daemon, serve, --daemon-config, /config/daemon.yaml]
          env:
            - {name: CONDUCTOR_CONFIG, value: /var/lib/conductor/data/config/conductor.yaml}
          ports:
            - {name: http, containerPort: 7300}
          livenessProbe:
            httpGet: {path: /healthz, port: http}
          readinessProbe:
            httpGet: {path: /readyz, port: http}
          securityContext:
            allowPrivilegeEscalation: false
            capabilities: {drop: [ALL]}
          volumeMounts:
            - {name: data, mountPath: /var/lib/conductor/data}
            - {name: config, mountPath: /config, readOnly: true}
            - {name: secrets, mountPath: /run/secrets/conductor, readOnly: true}
      volumes:
        - name: config
          configMap: {name: conductor-daemon-config}
        - name: secrets
          secret: {secretName: conductor-secrets}
  volumeClaimTemplates:
    - metadata:
        name: data
      spec:
        accessModes: [ReadWriteOnce]
        resources:
          requests: {storage: 1Gi}
---
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: conductor-instances
  namespace: conductor-instances
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/managed-by: conductord
      app.kubernetes.io/component: instance-runtime
      conductor/daemon: example-org
  policyTypes: [Ingress]
  ingress:
    - from:
        - namespaceSelector:
            matchLabels:
              kubernetes.io/metadata.name: conductor
          podSelector:
            matchLabels:
              app.kubernetes.io/name: conductord
      ports:
        - {port: 7301, protocol: TCP}
```

Replace the image twice and the two values of the Secret. To name the namespaces otherwise,
change each `namespace:`, the two Namespaces' names, `daemon.kubernetes.namespace` and the network
policy's `kubernetes.io/metadata.name`. `shareProcessNamespace` makes the pod's pause process its
first: the daemon would ignore `SIGTERM` as the first process and end only when the grace period
kills it. Then:

```console
kubectl apply -f manifests.yaml
kubectl -n conductor rollout status statefulset/conductor
```

The daemon's pod is Ready once `/readyz` answers `200`. `kubectl -n conductor exec conductor-0 --
conductor doctor` names one row per right of the Role, `kubernetes conductor-instances: <verb>
<resource> allowed`, and ends `ready`.

The daemon needs no right in its own namespace. `daemon.kubernetes.namespace` may also name the
daemon's own namespace, with the Role and the RoleBinding there. That layout works, and the
daemon then reaches every Secret of its own namespace, its control token among them: [the
rights](#the-rights-and-why-each-verb) say why to keep the two apart.

## The daemon-config file

`--daemon-config` names a file that holds `version` and `daemon` and nothing else. At every start
the daemon writes them into its config file, the one `CONDUCTOR_CONFIG` names on its volume, and
keeps the `instances` that file holds. So the ConfigMap carries the daemon's settings, and the
instances live on the volume and arrive over HTTP. A file with no instance is allowed where
`daemon.kubernetes` is set.

| key | in the example | is |
|---|---|---|
| `daemon.address` | `0.0.0.0` | the address the listener binds, so the Service and the probes reach it |
| `daemon.port` | `7300` | the listener's port |
| `daemon.store` | a sqlite file on the daemon's volume | the daemon's own store. Each instance pod opens a sqlite file on its own claim; a `postgres://` URL here is handed to every instance pod instead |
| `daemon.secrets` | the files of the mounted Secret | where every `secret:<name>` resolves, the daemon's and each instance's: see [the secrets directory](#the-secrets-directory) |
| `daemon.control_token` | `secret:control-token` | the secret whose value a control plane sends as its bearer |
| `daemon.kubernetes.namespace` | `conductor-instances` | the namespace of the instance pods |
| `daemon.kubernetes.image` | `registry.example.com/conductor:<tag>` | the image every instance pod runs; its tag is the instance's `binary_version` |
| `daemon.kubernetes.owner` | `example-org` | the value of the label `conductor/daemon` on every object the daemon makes |
| `daemon.kubernetes.api_port` | `7301` | the port the runtime serves its API on in its pod |
| `daemon.kubernetes.volume.size` | `10G` | the size of each instance's claim, a whole number of GiB written with `G`, 1G or more (`Gi`, `Mi` and other suffixes are refused); `volume.storage_class` names its class, absent the cluster's default |
| `daemon.kubernetes.resources` | none | `requests` and `limits` of `cpu` and `memory` for each instance pod |
| `daemon.kubernetes.env` | none | variables of each instance pod, each a `value` or a `secret`, such as `GH_TOKEN` for an instance with a GitHub source, or `GITLAB_TOKEN` and `GITLAB_HOST` for a self-hosted GitLab; an instance's `forge_token` takes the place of every entry of the daemon's forge token variables in its own pod, see [forge credentials](#forge-credentials) |
| `git_identity`, a key of an instance's mapping, not of this file | none | the git identity the instance's pod commits with, `{"name": "Example Bot", "email": "bot@example.com"}`: each value 1 to 200 bytes on one line, with no control character, no `<`, no `>` and no white space at either end, the `email` with exactly one `@`; a mapping that breaks the rule is refused `422` at the value's path. It takes the place of `daemon.kubernetes.env` entries of the identity's variables in the instance's pod, see [what the daemon creates](#what-the-daemon-creates-for-each-instance) |
| `model_token_kind`, a key of an instance's mapping, not of this file | none | the kind of the instance's `model_token`: `api_key`, as where the mapping names none, or `subscription_token`; a mapping that names it without a `model_token`, or names another word, is refused `422` at its path. See [the model token](#the-model-token) |
| `standing_rules`, a key of an instance's mapping, not of this file | none | text every session the instance starts reads in its prompt file, at most 16384 bytes of UTF-8 with no NUL; a mapping that breaks either is refused `422` at its path. See [standing rules](#standing-rules) |

The [config reference](../reference/config.md) has each key's rules.

## The rights, and why each verb

The daemon reaches the Kubernetes API with its service account's mounted token and calls nothing
the Role does not grant.

| resource | verbs | why |
|---|---|---|
| `pods` | `create` | to start an instance's runtime |
| `pods` | `list` | once per pass, by the label set: each instance's state, address and failure, and the pods a restarted daemon adopts |
| `pods` | `get` | to see a deleted pod gone before it starts it again |
| `pods` | `delete` | to stop, restart or remove an instance, and to remove a pod of its own that no instance holds |
| `pods/log` | `get` | the last lines of a pod that failed without a message, for the instance's `detail` |
| `secrets` | `create`, `delete` | the instance's Secret, written at every start, removed with the pod |
| `secrets` | `get` | to read an adopted pod's API token back after a restart of the daemon, and an instance's Secret before it updates it |
| `secrets` | `list` | to find a Secret of its own that no instance holds |
| `secrets` | `update` | to write an instance's `conductor.yaml` again in place when only its [standing rules](#standing-rules) changed, so its pod is not replaced |
| `persistentvolumeclaims` | `create` | the instance's claim, once; the daemon never reads or deletes it |

`conductor doctor` asks the API for each of these with a self-subject access review and fails on
one that is not granted. `kubectl auth can-i --list --as
system:serviceaccount:conductor:conductor -n conductor-instances` lists them.

These rights reach further than the daemon's own objects. `get` and `list` on Secrets read every
Secret of the namespace, and `create` on pods starts a pod that may name any Secret and any
service account of the namespace. So the namespace of the instance pods:

- holds nothing but what the daemon makes there, the Role, the RoleBinding and the network
  policy: no other workload, Secret or service account than the `default` one Kubernetes makes,
  whose token an instance pod does not mount;
- enforces the `restricted` pod security level;
- has a network policy that admits only the daemon to the instance pods' API port;
- is not the daemon's namespace: the daemon's workload, its Secret and its service account are
  in another, and the RoleBinding in the instance pods' namespace binds that service account.

## The secrets directory

`daemon.secrets` holds the daemon's own secrets, those the `daemon` section names such as the
control token, and the credentials the instances name, such as `model-token`. Every secret there
that the `daemon` section does not name can be put into an instance pod by whoever holds the
control token: an instance mapping names it as `secret:<name>`, and the daemon writes its value
into that instance's Secret. So keep nothing else there. A mapping that names a secret of the
`daemon` section is refused `422`.

## The calls

A control plane sends the control token as its bearer. With it, the daemon admits the reads of
`/v1/` and the five writes below, and nothing else. In the cluster, the daemon is the Service
`conductor` of its namespace:

```console
TOKEN=<the control token>
D=http://conductor.conductor.svc:7300
```

From outside the cluster, `kubectl -n conductor port-forward svc/conductor 7300:7300` and
`D=http://127.0.0.1:7300`.

Read which daemon answers: one row, its version and the config format it reads, the values
`conductor_build_info` carries on `/metrics`. It answers as soon as the daemon serves, before
its first pass published, while `GET /v1/instances` still answers `503`:

```console
curl -H "Authorization: Bearer $TOKEN" $D/v1/version
[{"config":"conductor.config/2","version":"<version>"}]
```

List the instances. A new daemon answers an empty list:

```console
curl -H "Authorization: Bearer $TOKEN" $D/v1/instances
```

Add one. The body holds the instance's mapping, as an instance of the config file writes it, and
a reason:

```console
curl -X POST -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"mapping":{"name":"alpha","placement":"none","session_prefix":"alpha","sources":[{"local":"/var/lib/conductor/data/checkouts"}],"forge":{"route":"direct"},"model_token":"secret:model-token"},"reason":"a new team"}' \
  $D/v1/instances
```

Read one:

```console
curl -H "Authorization: Bearer $TOKEN" $D/v1/instances/alpha
```

Replace one with a whole new mapping, the same name in it:

```console
curl -X PUT -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"mapping":{"name":"alpha","placement":"none","session_prefix":"alpha","sources":[{"local":"/var/lib/conductor/data/checkouts"}],"forge":{"route":"direct"},"model_token":"secret:model-token","thresholds":{"disk_low":"50G"}},"reason":"more disk"}' \
  $D/v1/instances/alpha
```

Read whether the secrets one names are set:

```console
curl -H "Authorization: Bearer $TOKEN" $D/v1/instances/alpha/secrets
[{"instance":"alpha","key":"model_token","modified_at":"2026-11-12T03:29:10Z","secret":"model-token","set":true}]
```

One row per key of the mapping that names a `secret:<name>`, and per key under `globals` that
does: `key` is the key's YAML path, relative to the instance's entry or starting with `globals.`,
`secret` the name, and `set` whether the daemon's secrets source holds a value under it as the
instance's start reads it, so an empty file reads `false`. Two keys that name one secret are two
rows. `modified_at` is the file's modification time, for a set secret of a `files` source only.
No row carries a value, a length or a digest. The rows list what the mapping names, not the
daemon's own secrets and not the secrets of the `daemon.kubernetes.env` entries, which the daemon
gives to pods itself. The daemon reads its source at each call: a key added to its mounted
Secret reads `set: true` once the kubelet has updated the mount, without a restart.

Restart one:

```console
curl -X POST -H "Authorization: Bearer $TOKEN" $D/v1/instances/alpha/restart
```

Remove one:

```console
curl -X DELETE -H "Authorization: Bearer $TOKEN" $D/v1/instances/alpha
```

What each call answered on a cluster:

| call | status | body |
|---|---|---|
| `GET /v1/version` | `200` | `[{"config":"conductor.config/2","version":"<version>"}]`, one row, also before the first pass published |
| `GET /v1/instances` | `200` | a list of rows, `[]` before the first add |
| `POST /v1/instances` | `201` | `{"actor":"control-plane","commit":"","name":"alpha","revision":"<id>"}` |
| `GET /v1/instances/alpha` | `200` | one row, below |
| `PUT /v1/instances/alpha` | `200` | as for `POST` |
| `GET /v1/instances/alpha/secrets` | `200` | one row per key that names a secret, above; `404` `conductor.instance.UnknownInstance` for a name the daemon's file does not hold, `503` `conductor.secret.SourceUnreachable` while the source does not answer |
| `POST /v1/instances/alpha/restart` | `202` | `{"action":"Restart","actor":"control-plane","name":"alpha","result":"Succeeded"}` |
| `DELETE /v1/instances/alpha` | `200` | as for `POST` |

`revision` names the change the daemon recorded, with the control plane as its actor. `commit` is
empty: the daemon's config file is not in a git checkout. One row:

```json
{"binary_version":"<tag>","box_id":null,"changed_by":{"kind":"Daemon"},"detail":null,
 "last_seen":"2026-11-12T03:30:48.033644837Z","name":"alpha","placement":"none",
 "roles":["conductor","conductor-dev","controller"],
 "sources":[{"kind":"Local","value":{"exclude":[],"path":"/var/lib/conductor/data/checkouts"}}],
 "state":"Running"}
```

`GET /v1/instances/alpha` adds `git_identity`, `{"name":…,"email":…}`, `model_token_kind` and
`standing_rules`, as the mapping holds each, where the mapping names one.

A restart answers once the new pod is created, and a replace that changes the rendering
replaces the pod the same way. Each waits for the old pod to be gone, so a call that comes
meanwhile waits its turn.

What a mapping must hold:

- `placement: none`. A placement that opens a box is refused.
- `session_prefix`, distinct across the instances: at most one instance of the file may go
  without one, so a second instance without it is refused `422` at
  `instances[<i>].session_prefix`.
- Every path, `checkouts`, `records`, `state`, `cache`, under the instance's data mount,
  `/var/lib/conductor/data`, or none: the daemon fills in a path the mapping leaves out, under
  the mount. A path outside it is refused `422` at its YAML path.
- A `secret:<name>` resolves against the daemon's secrets source, never against a Kubernetes
  Secret by name. The add of a name the source lacks is accepted, and the daemon creates nothing
  for it: its row reads `Stopped`, with the `detail` `SecretMissing: the daemon's secrets source
  holds no <name> that the pod of <instance> needs; nothing is created`.

What the daemon refuses:

| request | status | body |
|---|---|---|
| a bearer that is not the control token | `401` | `{"error":"the bearer is not admitted","identity":null}` |
| any other write with the control token, such as `PUT /v1/config` | `403` | `{"actor":{"agent":null,"kind":"ControlPlane","operator":null},"error":"conductor.operator.ActorNotHuman","principal":"control-plane"}` |
| `POST` of a name the daemon holds | `409` | `{"error":"conductor.config.InstanceExists","name":"beta"}` |
| a mapping that does not validate | `422` | `{"detail":"instances[1].records: \"/etc/gamma\" lies outside the instance's data mount /var/lib/conductor/data: …","error":"conductor.config.ConfigInvalid","path":"instances[1].records"}`: `path` is the first problem's YAML path in the daemon's file, where the new instance comes after the ones it holds, and `detail` names every problem in one line |
| a name the daemon does not hold | `404` | `{"error":"conductor.instance.UnknownInstance","name":"alpha"}` |

`GET /v1/instances` reads what the daemon's last pass saw, every 2 s: a write is listed after
the pass that acts on it. A pass that stops or replaces a pod waits for that pod to be gone, so
after a `DELETE` the instance stays listed until its pod is gone, about 4 s on a cluster, and a
new one may read `Starting` or `Stopped` before its pod is Ready.

### Where a checkout must be

An agent starts only in a checkout under the instance's checkouts root,
`/var/lib/conductor/data/checkouts` unless the mapping names another `checkouts.root`. A pod
clones the repositories its `github` and `gitlab` sources name under `include`, and nothing
else: a source without `include` gets no checkout from the pod, and outside a pod nothing is
cloned. The pod clones each such repository that has no checkout when it starts, before it seeds
its instance, and again at every collect pass while one still has none; it fetches each of those
checkouts at every pass. An archived repository is not cloned. A checkout is where the collector
reads it, with `origin` the forge's HTTPS URL:

| source | repository | checkout | `origin` |
|---|---|---|---|
| `{github: example-org, include: [one]}` | `one` | `<checkouts>/one` | `https://github.com/example-org/one.git` |
| `{gitlab: example-group, host: gitlab.example.com, include: [alpha, sub]}` | the project `example-group/alpha` | `<checkouts>/alpha` | `https://gitlab.example.com/example-group/alpha.git` |
| the same | the project `example-group/sub/gamma`, in the subgroup `sub` | `<checkouts>/sub/gamma` | `https://gitlab.example.com/example-group/sub/gamma.git` |

An instance on a self-hosted GitLab, with two projects of its group:

```console
curl -X POST -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"mapping":{"name":"beta","placement":"none","session_prefix":"beta","sources":[{"gitlab":"example-group","host":"gitlab.example.com","include":["alpha","sub/gamma"]}],"forge":{"route":"direct"},"model_token":"secret:model-token","forge_token":"secret:beta-forge-token"},"reason":"two projects of a group"}' \
  $D/v1/instances
```

The pod clones with git and the token it carries, through git's credential helper
([forge credentials](#forge-credentials)): no URL, `.git/config`, argument or message holds the
token. A self-hosted GitLab needs the token and its host: either `GITLAB_TOKEN` as a `secret` and
`GITLAB_HOST` as a `value` in `daemon.kubernetes.env`, for every instance on that host, or the
instance's own `forge_token`, as above. A pod that carries no token of a source's forge tries no
clone.

The pod never touches what is there: a path that holds `.git` is left as it is, with its commits
and its `origin`; a path that holds anything else is left alone and reported. A project whose
path under the group has more than two steps, or lies under a directory that is a checkout
itself, is not where the collector reads a checkout: it is not cloned, and is reported. A path
step `.git`, in any case, or one that starts with `.conductor-clone-`, where the pod makes its
clones, names no checkout: an `include` or `exclude` entry with one is refused, and a listed
project with one is not cloned and is left out of every collector. It is reported where the
source would collect it otherwise, an `include` entry covering its path or the source naming no
`include`; elsewhere it is left out without a report, as an excluded project is.

A repository an `include` entry covers is cloned and then trusted for the harness, so its
project settings and hooks apply to sessions started in it. A checkout under the root that no
entry covers, also one an earlier `include` had the pod clone, is not trusted while every source
on the root names `include`; where a `local` source or one without `include` is on the root, a
start trusts every checkout under it, so an operator's own checkout stays usable. Name in
`include` only hosts and repositories whose content may run in the pod with the pod's tokens.

The size of the instance's volume claim is the only limit of what a pod clones. Its start waits
at most 20 minutes for clones: it begins no clone after that, says how many it left, and the
collect passes clone them.

A repository the pod could not clone or fetch is one entry of `member_failures` in the
instance's health, collector `repositories`, member the repository's name, with the reason and
git's last error line, every credential taken out. A failed clone stops neither the other clones
nor the pod, and the next pass tries again. The row's `detail` does not change; read the health:

```console
curl -H "Authorization: Bearer $TOKEN" $D/v1/instances/beta/health
```

A pod takes its first collect pass at its start, after its clones and its seeding, whenever its
last cycle ran. Until that pass has ended the health answers the snapshot of the pod before;
once it has, its `snapshot` started after the pod did, and it lists each clone that failed at
this start, not the failures of the pod before. A failed clone is tried again at each collect
pass, every `cadence.cycle` (2 hours by default), and at every start of the pod. After a token
was corrected, `POST /v1/instances/{name}/restart` tries at once:

```console
curl -X POST -H "Authorization: Bearer $TOKEN" $D/v1/instances/beta/restart
```

In the pod, `conductor doctor` says how many of the included repositories have no checkout.

Which rights of the token are used, read from the calls conductor makes and not tested against
every forge version: conductor's own calls only read, the forge's API (listing the repositories,
their merge or pull requests, pipelines and releases) and the repositories (`git clone`,
`git fetch`). A session's merge request needs the right to write through the API, and a push the
right to write the repository. On GitLab those rights are called `read_api` and `read_repository`
for reading, `api` for writing through the API, and `write_repository` for a push.

### The work call

`POST /v1/instances/{name}/work` hands the instance one unit of work. The body holds
`repository`, which the instance's newest complete snapshot or its workspace file names; `brief`,
1 to 65536 bytes; `reference`, the control plane's id of the unit, 1 to 64 characters of
`A-Z a-z 0-9 . _ -` led by a letter or a digit; and optionally `start_ref` and `issue`. A body
is at most 1 MiB:

```console
curl -X POST -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"repository":"one","brief":"Add a /healthz route.","reference":"WI-1","start_ref":"main","issue":"https://github.com/example-org/one/issues/7"}' \
  $D/v1/instances/alpha/work
{"reference":"WI-1","dispatch_id":"DSP-<date>-01","repository":"one","state":"Sent","sent_at":"2026-11-13T09:00:00Z","links":[]}
```

The call sends the unit as a dispatch and starts one controller session for the repository at
once, unless the repository has a `Running` or `Paused` controller. It answers `201` with the
unit's row once the start was made or refused, and does not wait for the session. The session
runs on the instance's `controller` role, its profile the controller profile the binary carries
unless the role names one; its first message is the repository's charter, then a `## Dispatch` section that names the dispatch, its
reference, start ref and issue, and last the brief, verbatim, in a fenced block whose fence is
longer than any run of backticks in it. The same reference again answers `200` with the row and
starts nothing, whatever the other members say. `GET /v1/instances/{name}/work/{reference}`
answers the row, `404` `conductor.dispatch.WorkNotFound` for a reference no dispatch holds.

Where `<records>/charters/<repository>.md` is missing or empty, the start first writes
conductor's default charter there; a charter with text is never overwritten. The default charter
has the session work the unit and nothing else, in one pull or merge request that it reports as
`review <url>` and never merges (`merge: human`), and end with `done`, `failed` or `blocked`.
Any edit makes it the operator's charter, treated as any other. While it is unchanged, the
repository gets a controller only for a dispatch.

| request | status | `error` |
|---|---|---|
| a repository with an open dispatch | `409` | `conductor.dispatch.RepositoryBusy`, its open dispatches in `open` |
| no work request, or a `start_ref` or `issue` the rules below refuse | `422` | `conductor.dispatch.InvalidWorkRequest` |
| a reference that is none | `422` | `conductor.dispatch.InvalidReference` |
| a brief that is empty, over 65536 bytes, or holds a NUL or a run of more than 255 backticks | `422` | `conductor.dispatch.InvalidBrief` |
| a repository neither the snapshot nor the workspace file names | `422` | `conductor.dispatch.UnknownRepository` |
| a body over 1 MiB | `413` | none, the reason as text |
| a store or records that fail | `500` | `conductor.dispatch.WorkFailed`; the runtime's log says why |

`start_ref` is a git ref name of 1 to 255 bytes and `issue` 1 to 2048 bytes. Neither holds a
control character (C0 or C1), U+2028, U+2029, a bidirectional control or a backtick: each is
written in the controller's first prompt.

A start that is refused does not fail the call: it answers `201`, the row stays `Sent`, and its
member `start` says why no controller runs. `start` is there only while the row is `Sent`, until
a controller of the repository starts after it:

| `start` | means |
|---|---|
| `ControllerLimit` | `controllers.max_working` controllers run already |
| `AlreadyRunning` | the repository has a `Running` or `Paused` controller, which keeps working what it works; no second one starts |
| `StartFailed` | the start failed with no refusal of its own, a store or a file that could not be read or written; the runtime's log says why |
| any other | the refusal of `conductor spawn` by that name, such as `LaunchFailed` or `UntrustedCheckout` |

The loop tries the start again at its next cycle, every `cadence.cycle` (2 hours by default),
for each `Sent` unit whose repository runs no controller. After three starts of a unit failed at
the launch, `StartFailed` or `LaunchFailed`, it tries that unit no more, and one `cycle` line
says it is left to the operator.

`controllers.max_working` holds for the call and the cycle alike: each counts the instance's
running controllers and starts under one lock of the runtime's process, so neither starts past
it beside the other. A `conductor spawn` run by hand beside the runtime is not counted under that
lock.

The session reports its pull or merge request as `[REPORT <repo> <dispatch-id>] review <url>`,
and the loop takes the report at its next watch tick. The row's `links` then holds the URL and
`last_report` the report's first line. Only a `review` report records a link: its first word, an
`http://` or `https://` URL of at most 2048 characters with no space, control character, line or
paragraph separator or bidirectional control. A dispatch holds 32 links at most.

```console
curl -H "Authorization: Bearer $TOKEN" $D/v1/instances/alpha/work/WI-1
{"reference":"WI-1","dispatch_id":"DSP-<date>-01","repository":"one","state":"Started","sent_at":"2026-11-13T09:00:00Z","started_at":"2026-11-13T09:42:10Z","last_report":"[REPORT one DSP-<date>-01] review https://github.com/example-org/one/pull/12 opened for WI-1","links":["https://github.com/example-org/one/pull/12"]}
```

A `review` report starts a `Sent` row and moves no other: a row with a review link is not finished.
It reads `Done`, with `ended_at`, once the session reports `done`.

## What the daemon creates for each instance

For an instance `alpha`, in `daemon.kubernetes.namespace`:

| object | name | holds |
|---|---|---|
| Secret | `conductor-instance-alpha` | `conductor.yaml`, the instance's config file as the daemon renders it; `api_token`, the runtime's API token, new at every start; one key per secret the mapping or `daemon.kubernetes.env` names, such as `model-token`, its value from the daemon's secrets source, but none for an `env` entry the pod does not get ([forge credentials](#forge-credentials)) |
| volume claim | `conductor-instance-alpha-data` | `ReadWriteOnce`, of `daemon.kubernetes.volume`: the instance's checkouts, records, state, cache and its own sqlite store |
| pod | `conductor-instance-alpha` | one container, `runtime`, running `conductor --config /var/lib/conductor/data/conductor.yaml instance serve` from `daemon.kubernetes.image` |

The pod:

- restart policy `Never`: the daemon is its supervisor and starts a new pod when one ends;
- one process namespace for the pod, so its pause process is the first process: the runtime ends
  at once on `SIGTERM`, and what the agents leave behind is reaped, as `docker run --init` does
  for a container;
- no service account token mounted, and no service links;
- user 10001, not root, no privilege escalation, every capability dropped, the `RuntimeDefault`
  seccomp profile, so the `restricted` level admits it;
- the Secret mounted read-only at `/run/secrets/conductor`, the claim at
  `/var/lib/conductor/data`;
- the runtime's API on `daemon.kubernetes.api_port`, probed at `/healthz`; every request to it
  carries the token, which the daemon holds and sends when it proxies
  `/v1/instances/alpha/<route>` to the pod;
- the annotation `conductor/digest`, a digest of the pod's spec and the rendered file with no
  secret value in it: a pod whose digest is not the one the daemon would create now is replaced;
- `requests` and `limits` from `daemon.kubernetes.resources`, and the variables of
  `daemon.kubernetes.env` the pod gets ([Forge credentials](#forge-credentials)) and the
  instance's forge token, each value through the Secret;
- the git identity, `GIT_AUTHOR_NAME` and `GIT_COMMITTER_NAME` from the `name` of the
  instance's `git_identity`, `GIT_AUTHOR_EMAIL` and `GIT_COMMITTER_EMAIL` from its `email`, each
  once and in place of a `daemon.kubernetes.env` entry of the variable; for an instance without
  it, each variable `daemon.kubernetes.env` names, else `conductor` for a name and
  `conductor@localhost` for an e-mail. They are the pod's environment, so they hold for every
  commit in the pod, a session's commits in a checkout and conductor's in the records;
- git's credential helper for the HTTPS origin of the forge whose token the pod carries, in
  `GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_<n>` and `GIT_CONFIG_VALUE_<n>`, after the token and before
  the git identity ([Forge credentials](#forge-credentials));
- `GLAB_SEND_TELEMETRY` `false`, after the git identity, so no `glab` the pod runs sends usage
  data to the forge, where `daemon.kubernetes.env` does not name the variable.

The three objects carry the label set of [A network policy](#a-network-policy) and no owner
reference. At its start the runtime copies `conductor.yaml` from the Secret to its volume, writes
and commits its records skeleton where it is missing, and serves.

A row of `GET /v1/instances` reads `Starting` while the pod is Pending, `Running` once its
container is ready, `Unreachable` with a `detail` when the pod cannot pull its image or ended
with a message, and `Stopped` with a `detail` when the daemon refused to start it.

## What survives a restart of the daemon

The daemon keeps its config file, with every instance, on its volume. When its pod is deleted or
restarted, the new pod writes `version` and `daemon` from the ConfigMap into that file again,
keeps the instances, lists the pods by label and adopts each one of an instance it holds: it reads
the pod's token back from the instance's Secret, records nothing and restarts nothing. The
instance pods run on through it, with the same uid and no restart.

| event | what the daemon does |
|---|---|
| its own pod restarts | adopts every instance pod; none is replaced |
| an instance pod is deleted, by hand or by eviction | creates it again at its next pass; a pod gone within 60 s of its start counts as a failed start, and the next one waits the back-off, a few seconds the first time |
| an instance pod ends | starts it again after the failed-start back-off; the row's `detail` holds what the container said, or the last lines of its log |
| `PUT /v1/instances/{name}` changes the mapping | replaces the pod once: deleted, then created on the new rendering; a change of its `standing_rules` alone replaces no pod and writes the instance's Secret again in place, see [standing rules](#standing-rules) |
| `daemon.kubernetes.image` changes | replaces each instance pod once, on the new image |
| the daemon is upgraded to a version that changes the pod it would make | replaces each pod whose spec changed once, at its first pass, on the same image, and adopts every other pod. The version that gave git its credential helper ([Forge credentials](#forge-credentials)) replaces each pod that carries the token of a forge its instance has a source on, unless `daemon.kubernetes.env` names `GIT_CONFIG_COUNT`, and each pod of an instance with its own `forge_token` whose daemon gives a forge token variable it no longer carries. The version that gave the daemon's GitLab token to one host replaces each pod that carried the daemon's entry of `GITLAB_TOKEN`, `GITLAB_ACCESS_TOKEN` or `OAUTH_TOKEN` while a `gitlab` source of its instance is on another host than `GITLAB_HOST` names (`gitlab.com` where `env` names none), and each pod of an instance with its own `forge_token` whose daemon gives `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN`, `GITLAB_ACCESS_TOKEN` or `OAUTH_TOKEN`. That version refuses a config file whose instance names the secret of a daemon's forge token, and a `GITLAB_HOST` entry written as a `secret`: change either before the upgrade. The version that gives a pod the daemon's forge token only of a forge its instance has a source on replaces each pod that carried the daemon's entries of a forge its instance has no source on, and each pod that carried the daemon's entry of `CI_JOB_TOKEN` it no longer carries; it refuses an entry named `GL_HOST`, `GITLAB_URI` or `GITLAB_URL`, and an instance that names the secret of a daemon's `CI_JOB_TOKEN` entry |

The instances are in the daemon's config file and nowhere else. A daemon that starts on an empty
volume holds no instance, and at its first pass it deletes every pod and Secret that carries its
`conductor/daemon` label; the claims stay, and adding an instance again by the same name mounts
its claim again. So keep the daemon's volume across restarts and upgrades. A Deployment in place
of the StatefulSet works where a claim holds the directory of `CONDUCTOR_CONFIG`, the replica
count is one and the strategy is `Recreate`; the daemon uses nothing else of a StatefulSet, neither
its pod name nor its hostname.

The daemon reads the ConfigMap at its start only. After a change to it, restart the daemon's pod,
`kubectl -n conductor rollout restart statefulset/conductor`, or let the chart put a checksum of
the file into the pod template's annotations so a change rolls it.

## Remove an instance, and what stays

`DELETE /v1/instances/{name}` removes the instance from the daemon's file. The daemon then
deletes its pod and its Secret, and keeps its claim with the instance's records, checkouts and
store. An instance added again under the same name finds them there. To remove the data too:

```console
kubectl -n conductor-instances delete pvc conductor-instance-alpha-data
```

Before the daemon itself is removed, retire every instance: `DELETE` each name `GET
/v1/instances` lists, until the list is empty. A daemon removed first leaves its instance pods
running with no supervisor. The kept claims then go by the daemon's labels:

```console
kubectl -n conductor-instances delete pvc -l app.kubernetes.io/managed-by=conductord,conductor/daemon=example-org
```

## A network policy

The manifests hold one: it selects the instance pods by the daemon's labels and admits only the
daemon's pods, by their label and their namespace's name together, on
`daemon.kubernetes.api_port`. Select on these labels, which every object the daemon makes
carries:

| label | value |
|---|---|
| `app.kubernetes.io/managed-by` | `conductord` |
| `app.kubernetes.io/component` | `instance-runtime` |
| `conductor/daemon` | `daemon.kubernetes.owner` |
| `conductor/instance` | the instance's name |

With the policy applied on a cluster that enforces network policies, the daemon's pod reached an
instance pod's API port and another instance pod's request to it was refused; the kubelet's
probes still passed. The policy restricts ingress only: an instance pod reaches its forge and the
model's API, so restrict egress to those hosts in a policy of your own if your cluster needs it.

## Forge credentials

An instance with a source on a forge needs the forge's token in its pod. The daemon's forge
token variables are every variable the forge's command line reads a token from:
`GH_TOKEN`, `GITHUB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN` for `gh` and a
`github` source, and `GITLAB_TOKEN`, `GITLAB_ACCESS_TOKEN`, `OAUTH_TOKEN`, `CI_JOB_TOKEN` for
`glab` and a `gitlab` source. An instance's own token fills the first of its forge's list,
`GH_TOKEN` or `GITLAB_TOKEN`, and git's credential helper reads only that one. One daemon can
use both setups:

| setup | where the token is named | which pods carry it |
|---|---|---|
| one token per daemon | an entry of `daemon.kubernetes.env`, as a `secret` | the pods of the instances with a source on its forge or with `local` sources only; a GitLab token only the pods whose `gitlab` sources are all on its host |
| one token per instance | the instance's `forge_token: secret:<name>` | that instance's pod, in place of every entry of the daemon's forge token variables |

One token per daemon: put the token into the Secret `conductor-secrets` under `forge-token`, and
add the entry to `daemon.yaml` in the ConfigMap:

```yaml
daemon:
  kubernetes:
    env:
      - {name: GH_TOKEN, secret: secret:forge-token}
```

GitHub has one host, `github.com`, so that is all. The daemon's GitLab token belongs to one
host: the value of a `GITLAB_HOST` entry, `gitlab.com` where `env` names none. On a self-hosted
GitLab the setup per daemon is both entries:

```yaml
daemon:
  kubernetes:
    env:
      - {name: GITLAB_TOKEN, secret: secret:forge-token}
      - {name: GITLAB_HOST, value: gitlab.example.com}
```

A pod gets the daemon's entries of `GITLAB_TOKEN`, `GITLAB_ACCESS_TOKEN` and `OAUTH_TOKEN`, and
git's helper for that host, only where every `gitlab` source of its instance is on that host. The
daemon compares the hosts as git's key writes them: lowercased, with the port as written, so
`host` and `host:443` are different hosts; write a source's `host` as `GITLAB_HOST` writes it.
An instance on another GitLab host gets none of those entries, and its Secret none of their
secrets: it names a `forge_token` of its own. Otherwise a control plane could add an instance
whose `gitlab` source names a server of its choice and receive the daemon's token there with the
pod's next fetch. `GITLAB_HOST` is a `value`, a host as a `gitlab` source writes one; an entry
that writes it as a `secret` is refused at its path, since a host is no secret. The daemon reads
its GitLab host from `GITLAB_HOST` only: an entry named `GL_HOST`, `GITLAB_URI` or `GITLAB_URL`
is refused at its path, since a host written there would leave the daemon's token on
`gitlab.com`.

One token per instance: put the token into the Secret `conductor-secrets` under a name of its
own, and name it in the instance's mapping:

```console
curl -X PUT -H "Authorization: Bearer $TOKEN" -H 'content-type: application/json' \
  -d '{"mapping":{"name":"alpha","placement":"none","session_prefix":"alpha","sources":[{"github":"example-org"}],"forge":{"route":"direct"},"model_token":"secret:model-token","forge_token":"secret:alpha-forge-token"},"reason":"a forge token of its own"}' \
  $D/v1/instances/alpha
```

The daemon takes the variable from the instance's sources; a mapping names a secret, never a
variable. A mapping that names a `forge_token` is refused `422` at
`instances[<i>].forge_token` when its sources are all `local` or on both forges, and when it names
a secret of the `daemon` section. An instance that names its own `forge_token`
carries none of the daemon's forge token variables: its pod gets the daemon's entry of none of
them, and its Secret holds their secrets only where another entry names them. An instance that
names none carries the daemon's entries of a forge only where it has a source on that forge:
GitHub's where it has a `github` source, GitLab's where it has a `gitlab` source and every
`gitlab` source is on the daemon's GitLab host. An entry it does not carry keeps its secret out
of its Secret, unless an entry it carries names that secret too. An instance whose
sources are all `local` names no forge, so its pod carries the daemon's entries of both forges.

An instance cannot name the secret of a daemon's forge token, the secret an entry of one of
those variables names: not as its `forge_token`, not as its `model_token`, not under any other
key, and `globals` cannot either. The daemon gives that token to the pods it belongs to; a
mapping that names it is refused `422` at the reference's path, such as
`instances[<i>].forge_token`, and names a token of its own instead.

The daemon writes a token's value into the instance's Secret when it starts the pod, and a
running pod keeps the value it started with. To rotate a token, change its value in the Secret
`conductor-secrets`, and once the kubelet has updated the mounted file, restart each instance
that uses it with `POST /v1/instances/{name}/restart`.

git over HTTPS reaches the forge with the same token. Where a pod carries `GH_TOKEN` and its
instance has a `github` source, or carries `GITLAB_TOKEN` and has a `gitlab` source, from either
setup (the daemon's GitLab token only on its host), the daemon gives git a credential helper for that forge's origin in git's own variables
`GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_<n>` and `GIT_CONFIG_VALUE_<n>`: `https://github.com` for a
`github` source, and `https://<host>` for each host of a `gitlab` source, `https://gitlab.com`
where it names none. The helper answers git with the user name `x-access-token` for GitHub or
`oauth2` for GitLab and the token as the variable holds it when git asks; its text names the
variable and holds no token, and no file is written. git offers the token to no other host: not
to another host, not to the same host over `http`, and not on another port. So a clone, a fetch
or a push over HTTPS to the forge authenticates, for conductor's own `git fetch` and for every
session in the pod alike. git reads these variables from version 2.31; the image's git, from
Debian 13, is newer.

Where `daemon.kubernetes.env` names `GIT_CONFIG_COUNT`, the daemon writes none of the three
variables into any pod: the installer owns git's configuration, and a credential helper is then
the installer's to give.

SSH remotes are not covered: a remote such as `git@github.com:example-org/repository.git` gets
nothing from the token. Use the HTTPS remote, `https://github.com/example-org/repository.git`.

## The model token

An instance's `model_token: secret:<name>` is the credential its agents run on. Its
`model_token_kind` says which kind it is, and so how the harness gets it:

| `model_token_kind` | the secret holds | how the harness gets it |
|---|---|---|
| `api_key`, also where the mapping names none | an API key | the session's settings copy names `conductor token reveal --model-token <instance>` as its `apiKeyHelper`; the harness runs it when it needs the key, and it prints the value from the instance's secrets source at each ask |
| `subscription_token` | the long-lived token of a subscription login, as `claude setup-token` prints it | the runtime reads the value from the instance's secrets source once, at each session's start, and starts the harness with it as `CLAUDE_CODE_OAUTH_TOKEN` in the environment of the harness's process. The session's settings copy names no `apiKeyHelper`, which would outrank the variable: one the role's own settings name is left out of the copy, and the start says so in one line |

For either kind the value is in no command line, no settings file, no log line, no record and no
answer of the API; the pod holds it once, under the secret's key of the instance's Secret. The
runtime sets the variable on the harness's process as it starts it, with no unit and no shell in
between, so the value is in that process's environment, `/proc/<pid>/environ`, which its own
user and root can read, as for any variable of a process.

`subscription_token` is not supported at a placement that opens a box: a box receives its model
token sealed through a slot of the daemon's pool, as an API key. A mapping that names it there,
that names a kind without a `model_token`, or a word other than the two kinds, is refused `422`
`conductor.config.ConfigInvalid` at `instances[<i>].model_token_kind`. `GET /v1/instances/{name}`
answers `model_token_kind` as the mapping holds it, and no member where it holds none.

### A new value

A subscription token expires, and nothing refreshes it. An expired one, or one replaced, needs
a new value in the Secret `conductor-secrets` and a new session started on it. What a pod that
runs does when the value changes:

- Its secrets source is the instance's Secret, mounted at `/run/secrets/conductor`. The daemon
  writes the value into that Secret from `conductor-secrets` when it starts the pod, and not
  after: a pod that runs reads the value it started with, of either kind, whatever
  `conductor-secrets` holds now.
- `api_key`: `conductor token reveal` reads the mounted file at each ask of the harness.
- `subscription_token`: the runtime reads the mounted file once at each session's start; a
  session keeps the value it started with until it ends.

So once the kubelet has updated the daemon's mounted file, `POST /v1/instances/{name}/restart`:
the daemon writes the new value into the instance's Secret and starts a new pod, whose sessions
start on it. An instance that runs as no pod reads the daemon's secrets source itself: each
session started after the change runs on the new value.

A control plane that sends no restart can tell a pod that runs on an older value than the secret
holds: `modified_at` of the `model_token` row of `GET /v1/instances/{name}/secrets` is when the
daemon's file of the secret last changed, and a pod that started before that runs on the value
before it. No answer of the daemon carries the pod's start: read the pod's `status.startTime`,
or keep the time of the control plane's own last restart:

```console
kubectl -n conductor-instances get pod conductor-instance-alpha -o jsonpath='{.status.startTime}'
```

## Standing rules

An instance mapping may hold `standing_rules`: text that every session the instance starts
reads, whatever its role: each controller, the director and the maintainer. The daemon's
host roles, `supervisor` and `teacher`, do not read it. It is for a rule that must hold for every
session of one instance from its first one, such as no private name in a commit, a branch, a pull
request or an issue, where the repository's own `AGENTS.md` is not the control plane's to write.

```json
{"mapping": {"name": "alpha", "placement": "none", "session_prefix": "alpha",
             "sources": [{"github": "example-org"}],
             "standing_rules": "No private name in a commit, a branch, a pull request or an issue.\n"},
 "reason": "the tenant's rules"}
```

The text is at most 16384 bytes of UTF-8 and holds no NUL; a mapping that breaks either is
refused `422` `conductor.config.ConfigInvalid` at `instances[<i>].standing_rules`. The config file
holds no secret, so its secret scan reads this text as every other value: a text with a word that
looks like a token (one starting `ghp_` or `glpat-`, say) or a URL that writes a password
(`https://user:password@…`) is refused there too, and the problem quotes no part of the text.
Write such a rule without the word itself, for example "no token of a forge goes into a file". A
text that is empty or only white space is no rule: the mapping reads as one without the member.

The control plane alone writes the text. The instance's own settings write, the dashboard's
configuration page, shows `standing_rules` as fixed and refuses to change it. On a host where the
daemon starts its instances as units rather than as pods, a session runs as the daemon's user and
can edit the config file itself, as that host's trust model already says of every key.

A session gets the text in its prompt file, after one line that says these are the instance's
standing rules, verbatim inside a fenced block whose fence of backticks is longer than any run of
backticks in the text, so no line of the text closes the block:

- a role that starts from the profile the binary carries, as a controller and the maintainer do
  by default, starts from that profile with the block after it, written to
  `<state>/sessions/<session name>.profile.md`;
- a role whose config names its own `profile` file starts from a copy there that holds that
  file's text and the block; the named file is not changed;
- a role that starts from an agent, as the director does, gets a prompt file with the block alone
  beside its agent.

Without `standing_rules` every session starts as before, with the same arguments and the same
prompt file, and the pod is the one it was.

A change of `standing_rules` alone replaces no pod: the text is no part of the pod's digest. At
its next pass the daemon writes the instance's Secret again in place, its `conductor.yaml`
holding the new text, which takes the Role's `update` on Secrets. The kubelet updates the
Secret's files in the running pod after a delay of its own, by default up to about a minute, and
the runtime reads the text from the mounted file at each session's start. So a session started
after that reads the new text, and a session that already runs keeps the text it started with
until it starts again. An instance whose runtime is no pod reads the text from its config file at
each session's start. `GET /v1/instances/{name}` answers the text as the mapping holds it, and no
member where it holds none. A file that does not read or validate when a session starts leaves
the session the text the runtime last read, and the runtime writes one line to its log that names
the file and why, and no part of the text.

Where the Role lacks `update` on Secrets, the API refuses the write in place (`403`). The
instance's row then says so in its `detail`, naming the right, that the pod's sessions still read
the earlier text and that the pod's next start delivers the new one. A write that fails otherwise,
such as a `500`, a `409` conflict or an API server that does not answer, is no missing right: the
`detail` says the write did not go through and gives the status or the kind of failure, names no
right, and says the same of the sessions and the next start. Either clears once a write goes
through, the Secret holds the text already, or the pod starts again; the daemon's log gets one
line when a failure begins or changes and one when it ends. Nothing is deleted or replaced in its
place: the pod and its Secret stay as they were, and the next pass tries again. `conductor doctor`
and `conductor setup` fail on the missing right only once an instance names `standing_rules`;
until then its row says it is not needed yet.

## Limits

- **One daemon per owner in a namespace.** A daemon deletes every pod and Secret with its
  `conductor/daemon` label whose instance its file does not hold, so two daemons with the same
  `daemon.kubernetes.owner` in one namespace delete each other's. Give each daemon its own owner.
- **One replica.** The daemon keeps its file and its store on its own volume; the StatefulSet has
  one replica, and a second would be a second daemon.
- **No chart.** Conductor ships no chart and no install manifests. The manifests above are an
  example: a test holds them to the daemon's code, and the run on a cluster applies them.
- **Not built.** Drain and resume, stop and start as a desired state, capacity and heartbeat
  have no call. `POST /v1/instances/{name}/upgrade` answers `409` for an instance that runs as a
  pod: the image names its version. The token pool's join and leave answer `409`: each instance
  pod has a store of its own, which does not read the daemon's pool.
- **Not run on a cluster.** The work call's start and its review link were not shown end to
  end in an instance pod: no session ran there on a real model token, and no push reached a
  forge with a real forge token.
- **One control token.** It is one secret, not a person: the control token may make the calls
  above and no other write, and `PUT /v1/config` answers it `403`. Change its value in the
  Secret to rotate it; the daemon reads the value at every request, once the kubelet has updated
  the mounted file.
