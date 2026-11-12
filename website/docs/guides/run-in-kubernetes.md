---
title: Run conductor in Kubernetes
sidebar_position: 11
description: Run conductor's daemon in a Kubernetes namespace from the release image, let a control plane list, add, replace, restart and remove instances over HTTP with a control token, and read what the daemon creates for each instance, which runs as a pod of its own.
lede: The daemon runs as a StatefulSet of one from the release image. A control plane adds instances over HTTP, and the daemon runs each instance's runtime as a pod of its own, with a Secret and a volume claim beside it.
source: crates/conductor/tests/fixtures/k8s/manifests.yaml; crates/conductor/src/daemon/pods.rs, daemon/api.rs, daemon/seed.rs, daemon/mod.rs, config/pod.rs, config/instance.rs, kube.rs, doctor.rs; crates/conductor/tests/kubernetes_guide.rs, daemon_pods.rs, instance_routes.rs
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
    verbs: [create, get, list, delete]
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
| `daemon.kubernetes.env` | none | variables of each instance pod, each a `value` or a `secret`, such as `GH_TOKEN` for an instance with a GitHub source |

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
| `secrets` | `get` | to read an adopted pod's API token back after a restart of the daemon |
| `secrets` | `list` | to find a Secret of its own that no instance holds |
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
`/v1/` and the four writes below, and nothing else. In the cluster, the daemon is the Service
`conductor` of its namespace:

```console
TOKEN=<the control token>
D=http://conductor.conductor.svc:7300
```

From outside the cluster, `kubectl -n conductor port-forward svc/conductor 7300:7300` and
`D=http://127.0.0.1:7300`.

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
| `GET /v1/instances` | `200` | a list of rows, `[]` before the first add |
| `POST /v1/instances` | `201` | `{"actor":"control-plane","commit":"","name":"alpha","revision":"<id>"}` |
| `GET /v1/instances/alpha` | `200` | one row, below |
| `PUT /v1/instances/alpha` | `200` | as for `POST` |
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

## What the daemon creates for each instance

For an instance `alpha`, in `daemon.kubernetes.namespace`:

| object | name | holds |
|---|---|---|
| Secret | `conductor-instance-alpha` | `conductor.yaml`, the instance's config file as the daemon renders it; `api_token`, the runtime's API token, new at every start; one key per secret the mapping or `daemon.kubernetes.env` names, such as `model-token`, its value from the daemon's secrets source |
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
  `daemon.kubernetes.env`, each value through the Secret;
- the git identity of the records' commits, `GIT_AUTHOR_NAME` and `GIT_COMMITTER_NAME`
  `conductor`, `GIT_AUTHOR_EMAIL` and `GIT_COMMITTER_EMAIL` `conductor@localhost`, each where
  `daemon.kubernetes.env` does not name the variable.

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
| `PUT /v1/instances/{name}` changes the mapping | replaces the pod once: deleted, then created on the new rendering |
| `daemon.kubernetes.image` changes | replaces each instance pod once, on the new image |

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
- **One control token.** It is one secret, not a person: the control token may make the calls
  above and no other write, and `PUT /v1/config` answers it `403`. Change its value in the
  Secret to rotate it; the daemon reads the value at every request, once the kubelet has updated
  the mounted file.
- **Forge credentials.** An instance with a source on a forge needs the forge's token in its pod,
  through `daemon.kubernetes.env` (`GH_TOKEN` or `GITLAB_TOKEN`, as a `secret`). Which token an
  instance gets is the operator's decision.
