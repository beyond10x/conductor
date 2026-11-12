//! The pod supervisor (design: instance pods, decisions D1, D7, D8): where `daemon.kubernetes` is
//! set, conductord runs each instance's runtime as a pod of its own rather than as a unit or a
//! child process ([`super::units::Manager::Pods`]). It is a runtime supervisor, not a placement:
//! the instance's placement stays `none`.
//!
//! Each instance has three objects in the namespace `daemon.kubernetes.namespace`, all carrying
//! the label set [`labels`] and no owner reference:
//! - the Secret [`pod_name`]: the instance's config file as [`pod::render`] writes it (key
//!   [`pod::CONFIG_FILE`]), the runtime's token (key [`TOKEN_KEY`]), and each secret the rendered
//!   file or a `daemon.kubernetes.env` entry names, resolved from the daemon's secrets source and
//!   never from a Kubernetes Secret by name; one the source lacks refuses the start, and nothing
//!   is created;
//! - the volume claim [`claim_name`], of `daemon.kubernetes.volume`, created once and never
//!   deleted by the supervisor;
//! - the pod [`pod_name`] ([`manifests`]): the image `daemon.kubernetes.image` running
//!   `conductor --config <data>/conductor.yaml instance serve` on TCP at `daemon.kubernetes
//!   .api_port`, its token from the Secret, the Secret mounted read-only and the claim mounted
//!   writable, `restartPolicy: Never` (the daemon is the supervisor), no service account token,
//!   restricted to a non-root user without privilege escalation or capabilities, probed on
//!   `/healthz`. A digest of the pod's spec and of the rendered file is its annotation
//!   [`DIGEST_ANNOTATION`]: a pod whose digest differs from the one the daemon would create now
//!   (a new image, a changed rendering) is restarted.
//!
//! Every field comes from `daemon.kubernetes`, the instance's name and the rendering (decision
//! D8). The supervisor lists its pods once per tick by the first three labels ([`Pods::refresh`]):
//! the reconciler reads each instance's state, address and failure from that list
//! ([`Pods::seen`], [`Pods::detail`]). A restarted daemon adopts the pods it finds there, the
//! token read back from the instance's Secret ([`Pods::token`]); a pod or Secret of its owner
//! whose instance the config no longer holds is deleted ([`Pods::sweep`]). Objects of another
//! owner are never named.

use std::collections::{BTreeMap, BTreeSet};
use std::net::IpAddr;
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};
use conductor_model::config::{Config, Instance, Kubernetes, KubernetesQuantities};
use conductor_model::secret::SecretName;
use secrets_core::storage::SecretValue;
use serde_json::{Map, Value, json};

use super::units::Units;
use crate::config::pod;
use crate::kube::{Client, ContainerState, Kind, Meta, Pod};
use crate::systemd::Service;

/// The label every object of the supervisor carries, naming it as their manager.
pub const MANAGED_BY: (&str, &str) = ("app.kubernetes.io/managed-by", "conductord");

/// The label naming what the object is part of.
pub const COMPONENT: (&str, &str) = ("app.kubernetes.io/component", "instance-runtime");

/// The label naming the daemon that owns the object: `daemon.kubernetes.owner`.
pub const DAEMON_LABEL: &str = "conductor/daemon";

/// The label naming the instance of the object.
pub const INSTANCE_LABEL: &str = "conductor/instance";

/// The pod's annotation that holds the digest of its spec and its rendered file ([`digest`]).
pub const DIGEST_ANNOTATION: &str = "conductor/digest";

/// The Secret's key of the runtime's API token, which the pod takes as
/// [`crate::instance::TOKEN_VARIABLE`] by `secretKeyRef`.
pub const TOKEN_KEY: &str = crate::secret::API_TOKEN_SLOT;

/// The one container of an instance pod.
pub const CONTAINER: &str = "runtime";

/// The user and group the runtime runs as: the release image's user.
pub const USER: i64 = 10001;

/// How long a deleted pod may take to be gone before a start or a stop gives up.
const GONE_BOUND: Duration = Duration::from_secs(60);

/// How often a deleted pod is looked for while it goes.
const GONE_POLL: Duration = Duration::from_millis(200);

/// How long the runtime has to end on `SIGTERM` before the kubelet kills it.
const GRACE_SECONDS: i64 = 10;

/// The lines of a pod's log a failure's detail carries, when its container says nothing.
const LOG_LINES: u32 = 8;

/// The waiting reasons that hold a pod Pending until something changes: its row says so.
pub const STUCK: [&str; 6] = [
    "ImagePullBackOff",
    "ErrImagePull",
    "InvalidImageName",
    "CreateContainerConfigError",
    "CreateContainerError",
    "CrashLoopBackOff",
];

/// The pod and the Secret of `instance`: `conductor-instance-<instance>` (decision D7).
#[must_use]
pub fn pod_name(instance: &str) -> String {
    format!("conductor-instance-{instance}")
}

/// The volume claim of `instance`: `conductor-instance-<instance>-data` (decision D7).
#[must_use]
pub fn claim_name(instance: &str) -> String {
    format!("{}-data", pod_name(instance))
}

/// The selector of every object of the daemon whose owner is `owner`: the label set's first
/// three labels.
#[must_use]
pub fn selector(owner: &str) -> String {
    format!(
        "{}={},{}={},{DAEMON_LABEL}={owner}",
        MANAGED_BY.0, MANAGED_BY.1, COMPONENT.0, COMPONENT.1
    )
}

/// The label set of the objects of `instance` of the daemon whose owner is `owner`.
#[must_use]
pub fn labels(owner: &str, instance: &str) -> Value {
    Value::Object(
        [
            MANAGED_BY,
            COMPONENT,
            (DAEMON_LABEL, owner),
            (INSTANCE_LABEL, instance),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), json!(value)))
        .collect(),
    )
}

/// The version `image` names: its tag (`registry/conductor:1.2.3` is `1.2.3`), `latest` when it
/// names none, its digest's first characters when it names a digest only.
#[must_use]
pub fn version_of(image: &str) -> String {
    let (named, digest) = image.split_once('@').unwrap_or((image, ""));
    let last = named.rsplit('/').next().unwrap_or(named);
    match last.split_once(':') {
        Some((_, tag)) if !tag.is_empty() => tag.to_owned(),
        _ if !digest.is_empty() => digest.chars().take(19).collect(),
        _ => "latest".to_owned(),
    }
}

/// What the supervisor knows of an instance's pod at its last list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seen {
    /// The list failed, as said: the pod's state is not known.
    Unlisted(String),
    /// No pod of the instance was listed.
    Absent,
    /// The pod is being deleted.
    Deleting,
    /// The pod is Pending; with the reason it waits for, when that holds it there ([`STUCK`],
    /// or it cannot be scheduled).
    Pending { stuck: Option<String> },
    /// The pod runs, at its address; its container ready or not.
    Running { ready: bool, ip: Option<IpAddr> },
    /// The pod ended: Failed or Succeeded.
    Ended,
}

/// The three objects of one instance, and what the Secret must hold from the secrets source.
#[derive(Debug, Clone, PartialEq)]
pub struct Manifests {
    /// The pod, its digest annotation set.
    pub pod: Value,
    /// The volume claim.
    pub claim: Value,
    /// The rendered config file, the Secret's [`pod::CONFIG_FILE`].
    pub rendered: String,
    /// Each secret of the source the Secret holds, by name, which is its key.
    pub secrets: BTreeSet<String>,
    /// The digest of the pod's spec and the rendered file ([`DIGEST_ANNOTATION`]).
    pub digest: String,
}

/// A quantity mapping, the keys that are set.
fn quantities(quantities: &KubernetesQuantities) -> Map<String, Value> {
    let mut map = Map::new();
    if let Some(cpu) = &quantities.cpu {
        map.insert("cpu".to_owned(), json!(cpu));
    }
    if let Some(memory) = &quantities.memory {
        map.insert("memory".to_owned(), json!(memory));
    }
    map
}

/// The objects of `instance`'s pod in the daemon of `config`, whose `daemon.kubernetes` is
/// `kubernetes`, as the module documentation says. Nothing in them but from `kubernetes`, the
/// instance's name and the rendering (decision D8).
#[must_use]
pub fn manifests(kubernetes: &Kubernetes, config: &Config, instance: &Instance) -> Manifests {
    let name = &instance.name.0;
    let owner = &kubernetes.owner.0;
    let pod_name = pod_name(name);
    let port = kubernetes.api_port;
    let rendered = pod::render(config, instance);
    let mut secrets: BTreeSet<String> = crate::secret::referenced(&rendered)
        .remove(name)
        .unwrap_or_default();
    let mut env = vec![
        json!({"name": crate::config::INSTANCE_VARIABLE, "value": name}),
        json!({
            "name": crate::instance::CONFIG_SOURCE_VARIABLE,
            "value": format!("{}/{}", pod::SECRET_MOUNT, pod::CONFIG_FILE),
        }),
        json!({"name": crate::instance::LISTEN_VARIABLE, "value": format!("0.0.0.0:{port}")}),
        json!({
            "name": crate::instance::TOKEN_VARIABLE,
            "valueFrom": {"secretKeyRef": {"name": pod_name, "key": TOKEN_KEY}},
        }),
    ];
    for variable in &kubernetes.env {
        match (&variable.value, &variable.secret) {
            (_, Some(secret)) => {
                secrets.insert(secret.0.clone());
                env.push(json!({
                    "name": variable.name,
                    "valueFrom": {"secretKeyRef": {"name": pod_name, "key": secret.0}},
                }));
            }
            (Some(value), None) => env.push(json!({"name": variable.name, "value": value})),
            (None, None) => {}
        }
    }
    let mut resources = Map::new();
    for (key, set) in [
        ("requests", &kubernetes.resources.requests),
        ("limits", &kubernetes.resources.limits),
    ] {
        let set = quantities(set);
        if !set.is_empty() {
            resources.insert(key.to_owned(), Value::Object(set));
        }
    }
    let probe = |period: u64| {
        json!({
            "httpGet": {"path": "/healthz", "port": port},
            "periodSeconds": period,
            "failureThreshold": 3,
        })
    };
    let mut container = json!({
        "name": CONTAINER,
        "image": kubernetes.image,
        "command": [
            "conductor",
            "--config",
            format!("{}/{}", pod::DATA_MOUNT, pod::CONFIG_FILE),
            "instance",
            "serve",
        ],
        "env": env,
        "ports": [{"name": "api", "containerPort": port, "protocol": "TCP"}],
        "livenessProbe": probe(10),
        "readinessProbe": probe(5),
        "securityContext": {
            "allowPrivilegeEscalation": false,
            "capabilities": {"drop": ["ALL"]},
            "runAsNonRoot": true,
        },
        "volumeMounts": [
            {"name": "secret", "mountPath": pod::SECRET_MOUNT, "readOnly": true},
            {"name": "data", "mountPath": pod::DATA_MOUNT},
        ],
    });
    if !resources.is_empty() {
        container["resources"] = Value::Object(resources);
    }
    let spec = json!({
        "restartPolicy": "Never",
        "automountServiceAccountToken": false,
        "enableServiceLinks": false,
        "terminationGracePeriodSeconds": GRACE_SECONDS,
        "securityContext": {
            "runAsNonRoot": true,
            "runAsUser": USER,
            "runAsGroup": USER,
            "fsGroup": USER,
            "seccompProfile": {"type": "RuntimeDefault"},
        },
        "containers": [container],
        "volumes": [
            {"name": "secret", "secret": {"secretName": pod_name, "defaultMode": 0o440}},
            {"name": "data", "persistentVolumeClaim": {"claimName": claim_name(name)}},
        ],
    });
    let digest = digest(&spec, &rendered, &secrets);
    let mut annotations = Map::new();
    annotations.insert(DIGEST_ANNOTATION.to_owned(), json!(digest));
    let pod = json!({
        "apiVersion": "v1",
        "kind": "Pod",
        "metadata": {
            "name": pod_name,
            "labels": labels(owner, name),
            "annotations": annotations,
        },
        "spec": spec,
    });
    let mut claim_spec = json!({
        "accessModes": ["ReadWriteOnce"],
        "resources": {"requests": {"storage": format!("{}Gi", kubernetes.volume.size.0)}},
    });
    if let Some(class) = &kubernetes.volume.storage_class {
        claim_spec["storageClassName"] = json!(class);
    }
    let claim = json!({
        "apiVersion": "v1",
        "kind": "PersistentVolumeClaim",
        "metadata": {"name": claim_name(name), "labels": labels(owner, name)},
        "spec": claim_spec,
    });
    Manifests {
        pod,
        claim,
        rendered,
        secrets,
        digest,
    }
}

/// SHA-256, in hex, of `spec`, `rendered` and the names of `secrets`: what a pod is restarted
/// for when it changes. No secret's value and no token enters it.
fn digest(spec: &Value, rendered: &str, secrets: &BTreeSet<String>) -> String {
    let mut context = ring::digest::Context::new(&ring::digest::SHA256);
    context.update(spec.to_string().as_bytes());
    context.update(b"\0");
    context.update(rendered.as_bytes());
    for secret in secrets {
        context.update(b"\0");
        context.update(secret.as_bytes());
    }
    context
        .finish()
        .as_ref()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The pod supervisor over `client`: what its last list saw, by instance.
#[derive(Debug, Clone)]
pub struct Pods {
    client: Client,
    /// Each listed pod of the owner, by its instance label.
    seen: BTreeMap<String, Value>,
    /// Why the last list failed, while it did.
    unlisted: Option<String>,
    /// Why the last start of an instance was refused, until one succeeds.
    refused: BTreeMap<String, String>,
    /// What the last failure of an instance's pod said ([`Pods::said`]), until its next start.
    said: BTreeMap<String, String>,
}

impl Pods {
    /// The supervisor of the pods `client` reaches.
    #[must_use]
    pub const fn new(client: Client) -> Self {
        Self {
            client,
            seen: BTreeMap::new(),
            unlisted: None,
            refused: BTreeMap::new(),
            said: BTreeMap::new(),
        }
    }

    /// The client it calls.
    #[must_use]
    pub const fn client(&self) -> &Client {
        &self.client
    }

    /// Lists the pods of `kubernetes.owner` in its namespace, once, by [`selector`]. A list that
    /// fails is kept as the state of every pod ([`Seen::Unlisted`]).
    pub fn refresh(&mut self, kubernetes: &Kubernetes) {
        match self.client.list(
            &kubernetes.namespace.0,
            Kind::Pod,
            &selector(&kubernetes.owner.0),
        ) {
            Ok(pods) => {
                self.unlisted = None;
                self.seen = pods
                    .into_iter()
                    .filter_map(|pod| {
                        let instance = Meta::of(&pod).labels.get(INSTANCE_LABEL)?.clone();
                        Some((instance, pod))
                    })
                    .collect();
            }
            Err(error) => {
                self.unlisted = Some(format!(
                    "the pods of {} in {} do not list: {error}",
                    kubernetes.owner.0, kubernetes.namespace.0
                ));
            }
        }
    }

    /// The instances whose pod the last list saw.
    #[must_use]
    pub fn listed(&self) -> Vec<String> {
        self.seen.keys().cloned().collect()
    }

    /// The state of `instance`'s pod at the last list, or at its start since.
    #[must_use]
    pub fn seen(&self, instance: &str) -> Seen {
        if let Some(why) = &self.unlisted {
            return Seen::Unlisted(why.clone());
        }
        let Some(object) = self.seen.get(instance) else {
            return Seen::Absent;
        };
        let pod = Pod::of(object);
        if pod.meta.deletion_timestamp.is_some() {
            return Seen::Deleting;
        }
        match pod.phase.as_deref() {
            Some("Running") => Seen::Running {
                ready: object["status"]["containerStatuses"]
                    .as_array()
                    .is_some_and(|containers| {
                        !containers.is_empty()
                            && containers
                                .iter()
                                .all(|container| container["ready"] == Value::Bool(true))
                    }),
                ip: pod.ip.as_deref().and_then(|ip| ip.parse().ok()),
            },
            Some("Failed" | "Succeeded") => Seen::Ended,
            _ => Seen::Pending {
                stuck: stuck(object, &pod),
            },
        }
    }

    /// The image `instance`'s listed pod runs.
    #[must_use]
    pub fn image(&self, instance: &str) -> Option<String> {
        self.seen.get(instance)?["spec"]["containers"][0]["image"]
            .as_str()
            .map(str::to_owned)
    }

    /// The digest annotation of `instance`'s listed pod.
    #[must_use]
    pub fn digest(&self, instance: &str) -> Option<String> {
        self.seen.get(instance)?["metadata"]["annotations"][DIGEST_ANNOTATION]
            .as_str()
            .map(str::to_owned)
    }

    /// What `instance`'s row says of it besides its state: why the list failed, what holds its
    /// pod Pending, what its last failure said, why its start was refused; none while it runs or
    /// starts as it should.
    #[must_use]
    pub fn detail(&self, instance: &str) -> Option<String> {
        match self.seen(instance) {
            Seen::Unlisted(why) => Some(why),
            Seen::Pending { stuck } => stuck,
            Seen::Running { .. } => None,
            Seen::Ended | Seen::Absent | Seen::Deleting => self
                .said
                .get(instance)
                .cloned()
                .or_else(|| self.seen.get(instance).and_then(container_said))
                .or_else(|| self.refused.get(instance).cloned()),
        }
    }

    /// Why the last start of `instance` was refused, until one succeeds.
    #[must_use]
    pub fn refused(&self, instance: &str) -> Option<String> {
        self.refused.get(instance).cloned()
    }

    /// What a failure of `instance`'s pod is recorded with: its container's terminated or
    /// waiting reason and message, else the last lines of its log; that the pod is gone when the
    /// list saw none. Kept for its row until its next start.
    pub fn said(&mut self, kubernetes: &Kubernetes, instance: &str) -> String {
        let name = pod_name(instance);
        let said = match self.seen.get(instance) {
            None => format!("the pod {name} is gone"),
            Some(object) => match container_said(object).filter(|said| said.contains(": ")) {
                Some(said) => said,
                None => {
                    let reason = container_said(object);
                    let log = self
                        .client
                        .log_tail(&kubernetes.namespace.0, &name, LOG_LINES)
                        .map(|text| {
                            text.lines()
                                .map(|line| line.trim().replace(char::is_control, "\u{fffd}"))
                                .filter(|line| !line.is_empty())
                                .collect::<Vec<_>>()
                                .join(" | ")
                        });
                    match (reason, log) {
                        (Some(reason), Ok(log)) if !log.is_empty() => format!("{reason}: {log}"),
                        (None, Ok(log)) if !log.is_empty() => log,
                        (Some(reason), _) => reason,
                        (None, Ok(_)) => format!("the pod {name} ended and its log is empty"),
                        (None, Err(error)) => {
                            format!("the pod {name} ended; its log does not read: {error}")
                        }
                    }
                }
            },
        };
        self.said.insert(instance.to_owned(), said.clone());
        said
    }

    /// The token `instance`'s Secret holds, read back for a pod a restarted daemon adopts; none
    /// when there is no such Secret or it holds no token.
    ///
    /// # Errors
    ///
    /// The Secret does not read for another reason than its absence.
    pub fn token(&self, kubernetes: &Kubernetes, instance: &str) -> Result<Option<String>> {
        let secret =
            match self
                .client
                .get(&kubernetes.namespace.0, Kind::Secret, &pod_name(instance))
            {
                Ok(secret) => secret,
                Err(error) if error.is_not_found() => return Ok(None),
                Err(error) => return Err(anyhow!("read the Secret of {instance}: {error}")),
            };
        let token = secret["data"][TOKEN_KEY]
            .as_str()
            .and_then(base64_decode)
            .and_then(|bytes| String::from_utf8(bytes).ok())
            .or_else(|| secret["stringData"][TOKEN_KEY].as_str().map(str::to_owned));
        Ok(token.filter(|token| !token.trim().is_empty()))
    }

    /// Starts `instance`'s pod in the daemon of `config` with `token`: each secret it needs read
    /// from `resolve` first, so a secret the source lacks refuses the start before anything is
    /// created; then a pod of its name that is still there deleted and waited for, the Secret
    /// written anew, the claim created where there is none, and the pod created. Answers the
    /// pod's name.
    ///
    /// # Errors
    ///
    /// A secret is missing, or the API refuses a call.
    pub fn start(
        &mut self,
        kubernetes: &Kubernetes,
        config: &Config,
        instance: &Instance,
        token: &str,
        resolve: &dyn Fn(&SecretName) -> Option<SecretValue>,
    ) -> Result<String> {
        let name = instance.name.0.clone();
        let started = self.start_once(kubernetes, config, instance, token, resolve);
        match &started {
            Ok(_) => {
                self.refused.remove(&name);
                self.said.remove(&name);
            }
            Err(error) => {
                self.refused.insert(name, format!("{error:#}"));
            }
        }
        started
    }

    fn start_once(
        &mut self,
        kubernetes: &Kubernetes,
        config: &Config,
        instance: &Instance,
        token: &str,
        resolve: &dyn Fn(&SecretName) -> Option<SecretValue>,
    ) -> Result<String> {
        let name = &instance.name.0;
        let namespace = &kubernetes.namespace.0;
        let pod_name = pod_name(name);
        let made = manifests(kubernetes, config, instance);
        let mut data = Map::new();
        data.insert(
            pod::CONFIG_FILE.to_owned(),
            json!(base64_encode(made.rendered.as_bytes())),
        );
        data.insert(TOKEN_KEY.to_owned(), json!(base64_encode(token.as_bytes())));
        let mut missing = Vec::new();
        for secret in &made.secrets {
            if secret == pod::CONFIG_FILE || secret == TOKEN_KEY {
                bail!(
                    "the secret {secret} of {name} has the name of a key the daemon writes into \
                     the pod's Secret itself: name it otherwise; nothing is created"
                );
            }
            match resolve(&SecretName(secret.clone())) {
                Some(value) => {
                    data.insert(secret.clone(), json!(base64_encode(value.expose())));
                }
                None => missing.push(secret.as_str()),
            }
        }
        if !missing.is_empty() {
            bail!(
                "SecretMissing: the daemon's secrets source holds no {} that the pod of {name} \
                 needs; nothing is created",
                missing.join(", ")
            );
        }
        // A pod of the name that is still there, failed or going, goes first.
        match self.client.get(namespace, Kind::Pod, &pod_name) {
            Ok(_) => self.delete_pod(namespace, &pod_name)?,
            Err(error) if error.is_not_found() => {}
            Err(error) => bail!("read the pod {pod_name} in {namespace}: {error}"),
        }
        let secret = json!({
            "apiVersion": "v1",
            "kind": "Secret",
            "metadata": {"name": pod_name, "labels": labels(&kubernetes.owner.0, name)},
            "type": "Opaque",
            "data": data,
        });
        match self.client.create(namespace, Kind::Secret, &secret) {
            Ok(_) => {}
            Err(error) if error.is_conflict() => {
                self.delete_quietly(namespace, Kind::Secret, &pod_name)?;
                self.client
                    .create(namespace, Kind::Secret, &secret)
                    .map_err(|error| {
                        anyhow!("create the Secret {pod_name} in {namespace}: {error}")
                    })?;
            }
            Err(error) => bail!("create the Secret {pod_name} in {namespace}: {error}"),
        }
        match self
            .client
            .create(namespace, Kind::PersistentVolumeClaim, &made.claim)
        {
            Ok(_) => {}
            Err(error) if error.is_conflict() => {}
            Err(error) => bail!(
                "create the volume claim {} in {namespace}: {error}",
                claim_name(name)
            ),
        }
        let created = self
            .client
            .create(namespace, Kind::Pod, &made.pod)
            .map_err(|error| anyhow!("create the pod {pod_name} in {namespace}: {error}"))?;
        self.seen.insert(name.clone(), created);
        Ok(pod_name)
    }

    /// Stops `instance`'s pod: deletes it and waits until it is gone, then deletes its Secret.
    /// Its claim is kept. One already gone is no error.
    ///
    /// # Errors
    ///
    /// The API refuses a delete, or the pod is not gone within [`GONE_BOUND`].
    pub fn stop(&mut self, kubernetes: &Kubernetes, instance: &str) -> Result<()> {
        let namespace = &kubernetes.namespace.0;
        let name = pod_name(instance);
        self.delete_pod(namespace, &name)?;
        self.delete_quietly(namespace, Kind::Secret, &name)?;
        self.seen.remove(instance);
        Ok(())
    }

    /// Deletes each pod and Secret of the owner whose instance `keep` does not hold, by the
    /// last list of pods and one list of Secrets now; answers what was done for each such
    /// instance. Claims are kept; objects of another owner are never named.
    ///
    /// # Errors
    ///
    /// The Secrets do not list.
    pub fn sweep(
        &mut self,
        kubernetes: &Kubernetes,
        keep: &BTreeSet<String>,
    ) -> Result<Vec<(String, Result<()>)>> {
        let namespace = &kubernetes.namespace.0;
        let mut orphans: BTreeMap<String, (Vec<String>, Vec<String>)> = BTreeMap::new();
        if self.unlisted.is_none() {
            for (instance, pod) in &self.seen {
                if !keep.contains(instance) {
                    orphans
                        .entry(instance.clone())
                        .or_default()
                        .0
                        .push(Meta::of(pod).name);
                }
            }
        }
        let secrets = self
            .client
            .list(namespace, Kind::Secret, &selector(&kubernetes.owner.0))
            .map_err(|error| anyhow!("list the Secrets in {namespace}: {error}"))?;
        for secret in &secrets {
            let meta = Meta::of(secret);
            if let Some(instance) = meta.labels.get(INSTANCE_LABEL)
                && !keep.contains(instance)
            {
                orphans
                    .entry(instance.clone())
                    .or_default()
                    .1
                    .push(meta.name);
            }
        }
        let mut done = Vec::new();
        for (instance, (pods, secrets)) in orphans {
            let mut outcome = Ok(());
            for name in &pods {
                outcome = outcome.and_then(|()| self.delete_pod(namespace, name));
            }
            for name in &secrets {
                outcome = outcome.and_then(|()| self.delete_quietly(namespace, Kind::Secret, name));
            }
            if outcome.is_ok() {
                self.seen.remove(&instance);
            }
            done.push((instance, outcome));
        }
        Ok(done)
    }

    /// Deletes `kind`'s `name`; one that is not there is no error.
    fn delete_quietly(&self, namespace: &str, kind: Kind, name: &str) -> Result<()> {
        match self.client.delete(namespace, kind, name) {
            Ok(()) => Ok(()),
            Err(error) if error.is_not_found() => Ok(()),
            Err(error) => Err(anyhow!(
                "delete the {} {name} in {namespace}: {error}",
                kind.kind()
            )),
        }
    }

    /// Deletes the pod `name` and waits until the API no longer has it.
    fn delete_pod(&self, namespace: &str, name: &str) -> Result<()> {
        self.delete_quietly(namespace, Kind::Pod, name)?;
        let asked = Instant::now();
        loop {
            match self.client.get(namespace, Kind::Pod, name) {
                Err(error) if error.is_not_found() => return Ok(()),
                Err(error) => bail!("read the pod {name} in {namespace}: {error}"),
                Ok(_) if asked.elapsed() >= GONE_BOUND => bail!(
                    "the pod {name} in {namespace} was not gone {} s after its delete",
                    GONE_BOUND.as_secs()
                ),
                Ok(_) => std::thread::sleep(GONE_POLL),
            }
        }
    }
}

/// What holds the Pending pod `object` there: a waiting reason of [`STUCK`] with its message, or
/// that it cannot be scheduled.
fn stuck(object: &Value, pod: &Pod) -> Option<String> {
    for container in &pod.containers {
        if let ContainerState::Waiting {
            reason: Some(reason),
            message,
        } = &container.state
            && STUCK.contains(&reason.as_str())
        {
            return Some(match message {
                Some(message) => format!("{reason}: {message}"),
                None => reason.clone(),
            });
        }
    }
    object["status"]["conditions"]
        .as_array()?
        .iter()
        .find(|condition| {
            condition["type"] == "PodScheduled"
                && condition["status"] == "False"
                && condition["reason"] == "Unschedulable"
        })
        .map(|condition| match condition["message"].as_str() {
            Some(message) => format!("Unschedulable: {message}"),
            None => "Unschedulable".to_owned(),
        })
}

/// What the container of the pod `object` says of its end or its wait: `<reason> (exit
/// <code>): <message>`, `<reason>: <message>`, its reason alone, or none.
fn container_said(object: &Value) -> Option<String> {
    let pod = Pod::of(object);
    pod.containers
        .iter()
        .find_map(|container| match &container.state {
            ContainerState::Terminated {
                exit_code,
                reason,
                message,
            } => {
                let reason = format!(
                    "{} (exit {exit_code})",
                    reason.as_deref().unwrap_or("Terminated")
                );
                Some(match message {
                    Some(message) => format!("{reason}: {}", message.trim()),
                    None => reason,
                })
            }
            ContainerState::Waiting {
                reason: Some(reason),
                message,
            } => Some(match message {
                Some(message) => format!("{reason}: {}", message.trim()),
                None => reason.clone(),
            }),
            _ => None,
        })
}

impl Units for Pods {
    /// A pod is started from its instance ([`Pods::start`]), never from a unit.
    fn start(&mut self, service: &Service) -> Result<()> {
        bail!(
            "the pod supervisor starts a pod from its instance, not the unit {}",
            service.unit
        )
    }

    /// A pod is stopped through its instance ([`Pods::stop`]), never as a unit.
    fn stop(&mut self, unit: &str) -> Result<()> {
        bail!("the pod supervisor stops a pod through its instance, not the unit {unit}")
    }

    /// No unit runs here: a pod's state is [`Pods::seen`].
    fn active(&mut self, _unit: &str) -> Result<bool> {
        Ok(false)
    }

    fn enable(&mut self, unit: &str) -> Result<()> {
        bail!("the pod supervisor enables no unit {unit}")
    }

    fn disable(&mut self, unit: &str) -> Result<()> {
        bail!("the pod supervisor disables no unit {unit}")
    }

    fn pods(&self) -> Option<&Self> {
        Some(self)
    }

    fn pods_mut(&mut self) -> Option<&mut Self> {
        Some(self)
    }
}

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// `bytes` as standard base64 with padding, as a Secret's `data` holds them.
#[must_use]
pub fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let bits = chunk.iter().enumerate().fold(0_u32, |bits, (index, byte)| {
            bits | (u32::from(*byte) << (16 - 8 * index))
        });
        for index in 0..4 {
            if index <= chunk.len() {
                out.push(char::from(
                    ALPHABET[((bits >> (18 - 6 * index)) & 63) as usize],
                ));
            } else {
                out.push('=');
            }
        }
    }
    out
}

/// The bytes of `text`, standard base64 with padding, as a Secret's `data` writes them; none
/// when it is no base64.
#[must_use]
pub fn base64_decode(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0_u32;
    let mut held = 0_u32;
    let mut out = Vec::with_capacity(text.len() / 4 * 3);
    for byte in text.bytes().filter(|byte| !byte.is_ascii_whitespace()) {
        if byte == b'=' {
            break;
        }
        let value = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        };
        bits = (bits << 6) | u32::from(value);
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push(u8::try_from((bits >> held) & 0xff).unwrap_or_default());
        }
        bits &= (1 << held) - 1;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_goes_there_and_back() {
        for text in ["", "a", "ab", "abc", "abcd", "token-0123456789"] {
            let encoded = base64_encode(text.as_bytes());
            assert_eq!(
                base64_decode(&encoded).as_deref(),
                Some(text.as_bytes()),
                "{encoded}"
            );
        }
        assert_eq!(base64_encode(b"abcd"), "YWJjZA==");
    }

    #[test]
    fn the_version_is_the_images_tag() {
        assert_eq!(version_of("registry.example.com/conductor:1.2.3"), "1.2.3");
        assert_eq!(version_of("registry.example.com:5000/conductor"), "latest");
        assert_eq!(version_of("conductor:2.0.0@sha256:abc"), "2.0.0");
    }
}
