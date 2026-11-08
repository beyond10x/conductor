//! The daemon's secrets source (`story:secrets-source`, design architecture-next § 9): the config
//! names a secret as `secret:<name>` and never holds it; the daemon resolves it through the
//! `secrets` crates. `daemon.secrets` names one namespace and its backend, routed by
//! `secrets-federation`: `keychain`, the OS keychain of the control host (`secrets-keychain`),
//! or `remote`, the Secrets service, which answers `SourceUnreachable` until secrets federation
//! reaches it (decision S2). Conductor writes no secrets trait of its own: [`Unreachable`] is a
//! backend of the crates' storage port.
//!
//! - [`Secrets::resolve`] is `conductor.secret.ResolveSecrets`, the function a box opener calls:
//!   every name the instance's effective config writes ([`referenced`]) and the model token must
//!   be in the source, or the box is refused before anything starts (`SecretMissing`); the box
//!   receives one slot, the model token ([`MODEL_TOKEN_SLOT`]). Forge writes go through the gates
//!   route (decision S6), so no forge token is sealed into a box.
//! - [`Secrets::put`] is `conductor.secret.PutSecret`, `conductor secret put` ([`secret_put`]):
//!   the value comes from standard input, never from a flag, the environment or the config; a
//!   name already on record is refused unless `--replace` (decision S5).
//! - [`view`] is the rows of `GET /v1/secrets`: names, sources and the instances that use them,
//!   never a value.
//!
//! No value is formatted anywhere: a value is a [`SecretValue`], which has no `Debug`,
//! `Display` or `Serialize`, and a [`Sealed`] slot prints its name only.
//!
//! The keychain is the OS keychain, except in a debug build whose environment names
//! [`TEST_KEYCHAIN_VARIABLE`]: then it is `keyring-core`'s sample store backed by that file, so a
//! test never reaches the OS keychain. A release build has no such switch.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::io::{IsTerminal as _, Read as _};
use std::path::Path;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context as _, Result, anyhow, bail};
use async_trait::async_trait;
use conductor_model::behaviour::SecretStorage as Records;
use conductor_model::config::{InstanceName, SecretsSource};
use conductor_model::operator::{Actor, ActorKind, ActorNotHuman};
use conductor_model::placement::BoxId;
use conductor_model::primitives::Timestamp;
use conductor_model::secret::{
    Namespace, PutSecret, PutSecretOutcome, ResolveSecretsOutcome, SecretData, SecretExists,
    SecretMissing, SecretName, SecretPut, SecretSnapshot, SecretState, SecretsResolved, Slot,
    SlotName, Source, SourceUnreachable, SourceUnusable,
};
use keyring_core::CredentialStore;
use secrets_core::storage::{
    Address, BackendKind, BackendRef, Capability, NamespaceKey, Revealed, Scope, ScopeName,
    SecretMetadata, SecretName as StoredName, SecretStorage, SecretValue, StorageError, Target,
    Written,
};
use secrets_federation::{FederatedStorage, InMemoryConfig, Namespace as Mount, NamespaceConfig};
use serde::Serialize;
use serde_yaml::Value as Yaml;

use crate::cli::SecretPutArgs;
use crate::store::Store;

/// The variable a debug build reads the fake keychain's file from (module documentation).
pub const TEST_KEYCHAIN_VARIABLE: &str = "CONDUCTOR_TEST_KEYCHAIN_FILE";

/// The name of the one slot an agent's box receives: the model token (design § 9).
pub const MODEL_TOKEN_SLOT: &str = "model_token";

/// The namespace `daemon.secrets` names when it writes none: local mode's.
const DEFAULT_NAMESPACE: &str = ScopeName::DEFAULT;

/// The source as the config file writes it.
#[must_use]
pub const fn source_word(source: Source) -> &'static str {
    match source {
        Source::Keychain => "keychain",
        Source::Remote => "remote",
    }
}

/// Why `daemon.secrets` cannot be used at all (`conductor.secret.SourceUnusable`, decision S1):
/// the daemon does not start.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unusable {
    /// The declared error.
    pub error: SourceUnusable,
    /// The YAML path of the key at fault.
    pub path: &'static str,
    /// What is wrong, in words.
    pub why: String,
}

impl fmt::Display for Unusable {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SourceUnusable: {}: {}; the {} source cannot be used",
            self.path,
            self.why,
            source_word(self.error.source)
        )
    }
}

/// A backend that does not answer: the remote source until secrets federation reaches the
/// Secrets service (decision S2), and a keychain that does not open.
struct Unreachable(BackendKind);

#[async_trait]
impl SecretStorage for Unreachable {
    fn capabilities(&self) -> &[Capability] {
        self.0.capabilities()
    }
    async fn read(&self, _: &Target) -> Result<Revealed, StorageError> {
        Err(StorageError::Unavailable)
    }
    async fn write(&self, _: &Target, _: SecretValue) -> Result<Written, StorageError> {
        Err(StorageError::Unavailable)
    }
    async fn list(&self, _: &Scope) -> Result<Vec<SecretMetadata>, StorageError> {
        Err(StorageError::Unavailable)
    }
}

/// The keychain's credential store, and the sample store to save when it is the test's.
type Keychain = (
    Arc<CredentialStore>,
    Option<Arc<keyring_core::sample::Store>>,
);

/// The keychain this process uses (module documentation), or `None` when it does not open.
fn keychain() -> Option<Keychain> {
    #[cfg(debug_assertions)]
    if let Some(path) = std::env::var_os(TEST_KEYCHAIN_VARIABLE) {
        let store = keyring_core::sample::Store::new_with_backing(path.to_str()?).ok()?;
        return Some((store.clone() as Arc<CredentialStore>, Some(store)));
    }
    secrets_keychain::native_store()
        .ok()
        .map(|store| (store, None))
}

/// Runs one call of the `secrets` crates to its end on a runtime of its own.
fn block<T>(call: impl Future<Output = Result<T, StorageError>>) -> Result<T, StorageError> {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .map_err(|_| StorageError::Unavailable)?
        .block_on(call)
}

/// One sealed slot for a box: where the value goes, and the value. Its `Debug` names the slot
/// and never the value.
pub struct Sealed {
    /// The slot, as the `SecretsResolved` event names it.
    pub slot: Slot,
    value: SecretValue,
}

impl Sealed {
    /// The value's bytes, for the box opener to write into the slot's sealed memfd.
    #[must_use]
    pub fn value(&self) -> &[u8] {
        self.value.expose()
    }
}

impl fmt::Debug for Sealed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sealed")
            .field("slot", &self.slot)
            .finish_non_exhaustive()
    }
}

/// The input of [`Secrets::resolve`]: `conductor.secret.ResolveSecrets` before its slots are
/// known. `referenced` is every name the instance's effective config writes ([`referenced`]);
/// `model_token` the secret of the token the agent runs on, when it runs on one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolve {
    /// Who resolves: the daemon, `{kind: Daemon}`.
    pub actor: Actor,
    /// The instance the box is opened for.
    pub instance: InstanceName,
    /// The box being opened.
    pub r#box: BoxId,
    /// Every `secret:<name>` of the instance's effective config.
    pub referenced: Vec<SecretName>,
    /// The model token's secret, sealed as the box's one slot.
    pub model_token: Option<SecretName>,
    /// When.
    pub at: Timestamp,
}

/// What [`Secrets::resolve`] answers: the outcome, and the sealed slots when it is `Resolved`.
#[derive(Debug)]
pub struct Resolution {
    /// The declared outcome; `Resolved`'s event names the slots, never a value.
    pub outcome: ResolveSecretsOutcome,
    /// The slots to seal into the box: empty unless resolved.
    pub slots: Vec<Sealed>,
}

/// The daemon's secrets source: one namespace on one backend, routed by `secrets-federation`.
pub struct Secrets {
    source: Source,
    namespace: String,
    storage: FederatedStorage,
    scope: Scope,
    sample: Option<Arc<keyring_core::sample::Store>>,
}

impl fmt::Debug for Secrets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Secrets")
            .field("source", &self.source)
            .field("namespace", &self.namespace)
            .finish_non_exhaustive()
    }
}

impl Drop for Secrets {
    fn drop(&mut self) {
        if let Some(sample) = &self.sample {
            let _ = sample.save();
        }
    }
}

/// Opens the source `daemon.secrets` names: the keychain of this process (module
/// documentation) for `keychain`; for `remote`, the Secrets service at `url`, which does not
/// answer yet. A keychain that does not open is a source that does not answer.
///
/// # Errors
///
/// [`Unusable`]: `remote` writes no `url`, or the namespace is not one the crates accept.
pub fn open(config: &SecretsSource) -> Result<Secrets, Unusable> {
    let (backend, sample): (Arc<dyn SecretStorage>, _) = match config.source {
        Source::Keychain => match keychain() {
            Some((store, sample)) => (
                Arc::new(secrets_keychain::KeychainBackend::new(store)),
                sample,
            ),
            None => (Arc::new(Unreachable(BackendKind::Keychain)), None),
        },
        Source::Remote => (Arc::new(Unreachable(BackendKind::Remote)), None),
    };
    with_backend(config, backend, sample)
}

fn with_backend(
    config: &SecretsSource,
    backend: Arc<dyn SecretStorage>,
    sample: Option<Arc<keyring_core::sample::Store>>,
) -> Result<Secrets, Unusable> {
    let unusable = |path, why: &str| Unusable {
        error: SourceUnusable {
            source: config.source,
        },
        path,
        why: why.to_owned(),
    };
    if config.source == Source::Remote && config.url.as_deref().is_none_or(str::is_empty) {
        return Err(unusable(
            "daemon.secrets.url",
            "the remote source names no Secrets service: write url: <https address>",
        ));
    }
    let namespace = config
        .namespace
        .clone()
        .unwrap_or_else(|| DEFAULT_NAMESPACE.to_owned());
    let Ok(name) = ScopeName::parse(&namespace) else {
        return Err(unusable(
            "daemon.secrets.namespace",
            "is not one segment of lower-case letters, digits, '.', '_' and '-'",
        ));
    };
    let mount = BackendRef {
        kind: match config.source {
            Source::Keychain => BackendKind::Keychain,
            Source::Remote => BackendKind::Remote,
        },
        label: ScopeName::default_name(),
    };
    let key = NamespaceKey {
        tenant: ScopeName::default_name(),
        namespace: name.clone(),
    };
    let routes = Arc::new(InMemoryConfig::new());
    block(routes.insert_namespace(Mount {
        key,
        mount: Some(mount.clone()),
    }))
    .map_err(|_| unusable("daemon.secrets", "the namespace could not be routed"))?;
    let storage = FederatedStorage::new(routes)
        .with_backend(mount, backend)
        .map_err(|_| unusable("daemon.secrets", "the backend could not be registered"))?;
    Ok(Secrets {
        source: config.source,
        namespace,
        storage,
        scope: Scope {
            tenant: ScopeName::default_name(),
            namespace: name,
            user: ScopeName::default_name(),
        },
        sample,
    })
}

/// How a read of one name went.
enum Read {
    Found(Revealed),
    Missing,
    Unreachable,
}

impl Secrets {
    /// The backend kind.
    #[must_use]
    pub const fn source(&self) -> Source {
        self.source
    }

    /// The namespace the names live in.
    #[must_use]
    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    fn unreachable(&self) -> SourceUnreachable {
        SourceUnreachable {
            source: self.source,
        }
    }

    fn address(&self, name: &SecretName) -> Option<Address> {
        Some(Address {
            scope: self.scope.clone(),
            name: StoredName::parse(&name.0).ok()?,
        })
    }

    fn read(&self, name: &SecretName) -> Read {
        // A name the crates refuse is held by no source.
        let Some(address) = self.address(name) else {
            return Read::Missing;
        };
        match block(self.storage.read(&Target::unbound(address))) {
            Ok(revealed) => Read::Found(revealed),
            Err(StorageError::NotFound | StorageError::InvalidName) => Read::Missing,
            Err(_) => Read::Unreachable,
        }
    }

    /// The value the source holds under `name`, if any (`story:token-pool`): only `conductor
    /// token reveal` asks for one, for the harness that runs on the token; no value leaves this
    /// function but inside a [`SecretValue`], which formats nowhere.
    ///
    /// # Errors
    ///
    /// `SourceUnreachable` naming the source.
    pub fn reveal(&self, name: &SecretName) -> Result<Option<SecretValue>, SourceUnreachable> {
        match self.read(name) {
            Read::Found(revealed) => Ok(Some(revealed.value)),
            Read::Missing => Ok(None),
            Read::Unreachable => Err(self.unreachable()),
        }
    }

    /// Whether the source answers: it lists the namespace.
    ///
    /// # Errors
    ///
    /// `SourceUnreachable` naming the source.
    pub fn answers(&self) -> Result<(), SourceUnreachable> {
        block(self.storage.list(&self.scope))
            .map(drop)
            .map_err(|_| self.unreachable())
    }

    /// The names of `names` the source holds no value under, in order, each once.
    ///
    /// # Errors
    ///
    /// `SourceUnreachable` naming the source.
    pub fn missing(&self, names: &[SecretName]) -> Result<Vec<SecretName>, SourceUnreachable> {
        let mut missing = Vec::new();
        for name in names {
            match self.read(name) {
                Read::Found(_) => {}
                Read::Missing => {
                    if !missing.contains(name) {
                        missing.push(name.clone());
                    }
                }
                Read::Unreachable => return Err(self.unreachable()),
            }
        }
        Ok(missing)
    }

    /// `conductor.secret.ResolveSecrets` (module documentation). No value leaves this function
    /// but inside a [`Sealed`].
    #[must_use]
    pub fn resolve(&self, input: &Resolve) -> Resolution {
        let refused = |outcome| Resolution {
            outcome,
            slots: Vec::new(),
        };
        let mut names = input.referenced.clone();
        names.extend(input.model_token.iter().cloned());
        let mut missing = Vec::new();
        let mut token = None;
        for name in &names {
            match self.read(name) {
                Read::Found(revealed) => {
                    if input.model_token.as_ref() == Some(name) {
                        token = Some(revealed.value);
                    }
                }
                Read::Missing => {
                    if !missing.contains(name) {
                        missing.push(name.clone());
                    }
                }
                Read::Unreachable => {
                    return refused(ResolveSecretsOutcome::Unreachable {
                        error: self.unreachable(),
                    });
                }
            }
        }
        if !missing.is_empty() {
            return refused(ResolveSecretsOutcome::Missing {
                error: SecretMissing {
                    instance: input.instance.clone(),
                    r#box: input.r#box.clone(),
                    names: missing,
                },
            });
        }
        let slots: Vec<Sealed> = input
            .model_token
            .iter()
            .zip(token)
            .map(|(name, value)| Sealed {
                slot: Slot {
                    secret: name.clone(),
                    r#box: input.r#box.clone(),
                    slot_name: SlotName(MODEL_TOKEN_SLOT.to_owned()),
                    sealed_at: input.at.clone(),
                },
                value,
            })
            .collect();
        Resolution {
            outcome: ResolveSecretsOutcome::Resolved {
                secrets_resolved: SecretsResolved {
                    actor: input.actor.clone(),
                    instance: input.instance.clone(),
                    r#box: input.r#box.clone(),
                    slots: slots.iter().map(|sealed| sealed.slot.clone()).collect(),
                    resolved_at: input.at.clone(),
                },
            },
            slots,
        }
    }

    /// `conductor.secret.PutSecret`: `value` into the source under `input.name`, and the name
    /// on record in `records`, the daemon's store.
    ///
    /// # Errors
    ///
    /// The name is not one the `secrets` crates accept, or the record cannot be written.
    pub fn put(
        &self,
        records: &mut Store,
        input: &PutSecret,
        value: SecretValue,
    ) -> Result<PutSecretOutcome> {
        if input.actor.kind != ActorKind::Human {
            return Ok(PutSecretOutcome::NotHuman {
                error: ActorNotHuman {
                    actor: input.actor.clone(),
                },
            });
        }
        let Some(address) = self.address(&input.name) else {
            bail!(
                "{:?} is not a secret name the secrets crates keep: '/'-separated segments of \
                 lower-case letters, digits, '.', '_' and '-', each starting with a letter or \
                 digit",
                input.name.0
            );
        };
        let unreachable = || PutSecretOutcome::Unreachable {
            error: self.unreachable(),
        };
        let on_record = Records::get(records, &input.name).is_some();
        let held = match self.read(&input.name) {
            Read::Found(_) => true,
            Read::Missing => false,
            Read::Unreachable => return Ok(unreachable()),
        };
        if (held || on_record) && !input.replace {
            return Ok(PutSecretOutcome::Exists {
                error: SecretExists {
                    name: input.name.clone(),
                },
            });
        }
        if block(self.storage.write(&Target::unbound(address), value)).is_err() {
            return Ok(unreachable());
        }
        if let Some(sample) = &self.sample {
            sample
                .save()
                .map_err(|_| anyhow!("the test keychain file could not be written"))?;
        }
        let secret_put = SecretPut {
            actor: input.actor.clone(),
            name: input.name.clone(),
            source: self.source,
            put_at: input.put_at.clone(),
        };
        if on_record {
            return Ok(PutSecretOutcome::Replaced { secret_put });
        }
        Records::put(
            records,
            SecretSnapshot {
                state: SecretState::Stored,
                data: SecretData {
                    name: input.name.clone(),
                    source: self.source,
                    namespace: (self.namespace != DEFAULT_NAMESPACE)
                        .then(|| Namespace(self.namespace.clone())),
                    instances: Vec::new(),
                    created_by: input.actor.clone(),
                    created_at: input.put_at.clone(),
                    last_resolved_at: None,
                },
            },
        );
        records
            .check()
            .map_err(|error| anyhow!("record the secret: {error}"))?;
        Ok(PutSecretOutcome::Stored { secret_put })
    }

    /// The line `conductor doctor` and the daemon's start print: the source, its namespace and
    /// whether it answers.
    #[must_use]
    pub fn line(&self, config: &SecretsSource) -> String {
        let at = config
            .url
            .as_deref()
            .map(|url| format!(" {url}"))
            .unwrap_or_default();
        let answer = match self.answers() {
            Ok(()) => "(answers)".to_owned(),
            Err(_) => "(does not answer: SourceUnreachable)".to_owned(),
        };
        format!(
            "secrets: {}{at} namespace {} {answer}",
            source_word(self.source),
            self.namespace
        )
    }

    /// Keeps `value` under `name`, replacing what was there, with nothing on record
    /// (`story:daemon-remote`: a client's login is no secret of the daemon's).
    ///
    /// # Errors
    ///
    /// `SourceUnreachable` naming the source; a name the crates refuse is one too.
    pub fn keep(&self, name: &SecretName, value: SecretValue) -> Result<(), SourceUnreachable> {
        let address = self.address(name).ok_or_else(|| self.unreachable())?;
        block(self.storage.write(&Target::unbound(address), value))
            .map_err(|_| self.unreachable())?;
        if let Some(sample) = &self.sample {
            sample.save().map_err(|_| self.unreachable())?;
        }
        Ok(())
    }
}

/// The namespace of a client's logins in its own keychain (`story:daemon-remote`).
pub const LOGIN_NAMESPACE: &str = "conductor-login";

/// This client's own keychain, the namespace [`LOGIN_NAMESPACE`]: where `conductor login` keeps
/// what it obtained (`story:daemon-remote`). The OS keychain, or the test's file (module
/// documentation).
///
/// # Errors
///
/// [`Unusable`]: never for the keychain in practice; a keychain that does not open answers
/// `SourceUnreachable` on use.
pub fn client() -> Result<Secrets, Unusable> {
    open(&SecretsSource {
        source: Source::Keychain,
        namespace: Some(LOGIN_NAMESPACE.to_owned()),
        url: None,
    })
}

/// The doctor's line for `config`: [`Secrets::line`], or why the source cannot be used.
#[must_use]
pub fn doctor_line(config: &SecretsSource) -> String {
    match open(config) {
        Ok(secrets) => secrets.line(config),
        Err(unusable) => format!(
            "secrets: {} (unusable: {unusable})",
            source_word(config.source)
        ),
    }
}

/// What the daemon prints at start: the source line, then one line per instance whose config
/// names a secret the source does not hold, each naming `SecretMissing` and the names; that
/// instance's box is refused when it opens.
///
/// # Errors
///
/// [`Unusable`]: the daemon does not start.
pub fn at_start(config: &SecretsSource, text: &str) -> Result<Vec<String>, Unusable> {
    let secrets = open(config)?;
    let mut lines = vec![secrets.line(config)];
    for (instance, names) in referenced(text) {
        let names: Vec<SecretName> = names.into_iter().map(SecretName).collect();
        match secrets.missing(&names) {
            Ok(missing) if missing.is_empty() => {}
            Ok(missing) => lines.push(format!(
                "secrets: {instance}: SecretMissing: {}; its box is refused until conductor \
                 secret put writes them",
                missing
                    .iter()
                    .map(|name| name.0.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            Err(_) => lines.push(format!("secrets: {instance}: SourceUnreachable")),
        }
    }
    Ok(lines)
}

/// The name `text` writes after `secret:`, when it is one (`config::secret_name`).
fn named(text: &str) -> Option<&str> {
    crate::config::secret_name(text)
        .then(|| text.strip_prefix("secret:"))
        .flatten()
}

fn collect(value: &Yaml, into: &mut BTreeSet<String>) {
    match value {
        Yaml::String(text) => {
            if let Some(name) = named(text) {
                into.insert(name.to_owned());
            }
        }
        Yaml::Sequence(items) => items.iter().for_each(|item| collect(item, into)),
        Yaml::Mapping(map) => map.values().for_each(|item| collect(item, into)),
        Yaml::Tagged(tagged) => collect(&tagged.value, into),
        _ => {}
    }
}

/// Every `secret:<name>` each instance's effective config writes, by instance: the names under
/// `globals` and under the instance's own entry. The daemon's own (`daemon.*`) are no
/// instance's. A text that is no YAML names none.
#[must_use]
pub fn referenced(text: &str) -> BTreeMap<String, BTreeSet<String>> {
    let Ok(file) = serde_yaml::from_str::<Yaml>(text) else {
        return BTreeMap::new();
    };
    let mut global = BTreeSet::new();
    if let Some(globals) = file.get("globals") {
        collect(globals, &mut global);
    }
    let mut by_instance = BTreeMap::new();
    for instance in file
        .get("instances")
        .and_then(Yaml::as_sequence)
        .into_iter()
        .flatten()
    {
        let Some(name) = instance.get("name").and_then(Yaml::as_str) else {
            continue;
        };
        let mut names = global.clone();
        collect(instance, &mut names);
        by_instance.insert(name.to_owned(), names);
    }
    by_instance
}

/// One row of `GET /v1/secrets`: a secret's name, its source as the config writes it, the
/// instances whose effective config names it, and who put it and when, absent for a name the
/// config writes and no put recorded. Never a value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SecretRow {
    /// The name.
    pub name: String,
    /// `keychain` or `remote`.
    pub source: &'static str,
    /// The instances that use it, by name, in order.
    pub instances: Vec<String>,
    /// Who put it (`human:<name>`), when a put is on record.
    pub created_by: Option<String>,
    /// When it was put, when a put is on record.
    pub created_at: Option<String>,
}

/// The rows of `GET /v1/secrets` (module documentation): every secret on record in `records`,
/// the daemon's store, and every name the config file's `text` writes, by name. A name only the
/// config writes has the daemon's `source`.
#[must_use]
pub fn view(records: &Store, text: &str, source: Source) -> Vec<SecretRow> {
    let mut rows: BTreeMap<String, SecretRow> = Records::list(records)
        .into_iter()
        .map(|secret| {
            let data = secret.data;
            (
                data.name.0.clone(),
                SecretRow {
                    name: data.name.0,
                    source: source_word(data.source),
                    instances: Vec::new(),
                    created_by: Some(crate::actor::written(&data.created_by)),
                    created_at: Some(data.created_at.0),
                },
            )
        })
        .collect();
    for (instance, names) in referenced(text) {
        for name in names {
            rows.entry(name.clone())
                .or_insert_with(|| SecretRow {
                    name,
                    source: source_word(source),
                    instances: Vec::new(),
                    created_by: None,
                    created_at: None,
                })
                .instances
                .push(instance.clone());
        }
    }
    rows.into_values().collect()
}

/// The value on standard input: one line read with echo off from a terminal, else everything up
/// to its end, one trailing line end dropped.
pub(crate) fn value_from_stdin(name: &str) -> Result<SecretValue> {
    let stdin = std::io::stdin();
    let mut bytes = Vec::new();
    if stdin.is_terminal() {
        use nix::sys::termios::{LocalFlags, SetArg, tcgetattr, tcsetattr};
        eprint!("value of {name} (not shown): ");
        let saved = tcgetattr(&stdin).context("read the terminal's settings")?;
        let mut quiet = saved.clone();
        quiet.local_flags.remove(LocalFlags::ECHO);
        tcsetattr(&stdin, SetArg::TCSANOW, &quiet).context("turn the terminal's echo off")?;
        let mut line = String::new();
        let read = stdin.read_line(&mut line);
        let _ = tcsetattr(&stdin, SetArg::TCSANOW, &saved);
        eprintln!();
        read.context("read the value")?;
        bytes = line.into_bytes();
    } else {
        stdin
            .lock()
            .read_to_end(&mut bytes)
            .context("read the value from standard input")?;
    }
    if bytes.last() == Some(&b'\n') {
        bytes.pop();
        if bytes.last() == Some(&b'\r') {
            bytes.pop();
        }
    }
    let value = SecretValue::new(bytes).map_err(|_| anyhow!("the value is too large"))?;
    if value.is_empty() {
        bail!("standard input holds no value: pipe the value in, or type it at the prompt");
    }
    Ok(value)
}

/// `conductor secret put` (module documentation): the source and the store of the config file
/// `config` names, the actor of the process, now.
///
/// # Errors
///
/// No `--name`; the config does not load; standard input holds no value; the store does not
/// open; or the record cannot be written. A refusal of the specification is one line of standard
/// error and exit 1.
pub fn secret_put(config: Option<&Path>, args: &SecretPutArgs) -> Result<ExitCode> {
    const WHAT: &str = "secret put";
    let Some(name) = args.name.clone() else {
        bail!("{WHAT}: --name is required");
    };
    let environment = crate::config::Environment::from_process().context(WHAT)?;
    let loaded = crate::config::load(config, &environment).context(WHAT)?;
    let source = &loaded.config.daemon.secrets;
    let secrets = match open(source) {
        Ok(secrets) => secrets,
        Err(unusable) => {
            eprintln!("conductor: {WHAT}: {unusable}; nothing was written");
            return Ok(ExitCode::FAILURE);
        }
    };
    let input = PutSecret {
        actor: crate::actor::chosen(),
        name: SecretName(name.clone()),
        put_at: crate::records::now().context(WHAT)?,
        replace: args.replace == Some(true),
    };
    if input.actor.kind != ActorKind::Human {
        eprintln!(
            "conductor: {WHAT}: ActorNotHuman: {}",
            crate::actor::not_human(&input.actor)
        );
        return Ok(ExitCode::FAILURE);
    }
    let value = value_from_stdin(&name).context(WHAT)?;
    let url = &loaded.config.daemon.store.0;
    if let Ok(crate::store::Location::Sqlite(path)) = crate::store::Location::parse(url)
        && let Some(parent) = path.parent()
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("{WHAT}: daemon.store: create {}", parent.display()))?;
    }
    let mut records = crate::store::open_url(url, crate::daemon::TENANT)
        .map_err(|error| anyhow!("{WHAT}: daemon.store: {error:#}"))?;
    let refusal = match secrets.put(&mut records, &input, value).context(WHAT)? {
        PutSecretOutcome::Stored { .. } => {
            println!(
                "{name}: stored in the {} source, namespace {}",
                source_word(secrets.source()),
                secrets.namespace()
            );
            return Ok(ExitCode::SUCCESS);
        }
        PutSecretOutcome::Replaced { .. } => {
            println!(
                "{name}: written again in the {} source, namespace {}",
                source_word(secrets.source()),
                secrets.namespace()
            );
            return Ok(ExitCode::SUCCESS);
        }
        PutSecretOutcome::NotHuman { error } => {
            format!("ActorNotHuman: {}", crate::actor::not_human(&error.actor))
        }
        PutSecretOutcome::Unreachable { error } => format!(
            "SourceUnreachable: the {:?} source did not answer; nothing was written",
            error.source
        ),
        PutSecretOutcome::Exists { error } => format!(
            "SecretExists: {} is on record already; give --replace to write its value again; \
             nothing was written",
            error.name.0
        ),
    };
    eprintln!("conductor: {WHAT}: {refusal}");
    Ok(ExitCode::FAILURE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use conductor_model::primitives::Uuid;

    fn keychain(namespace: Option<&str>) -> Secrets {
        let store = keyring_core::mock::Store::new().expect("a mock store");
        let config = SecretsSource {
            source: Source::Keychain,
            namespace: namespace.map(str::to_owned),
            url: None,
        };
        with_backend(
            &config,
            Arc::new(secrets_keychain::KeychainBackend::new(store)),
            None,
        )
        .expect("open the mock keychain")
    }

    fn write(secrets: &Secrets, name: &str, value: &str) {
        let address = secrets
            .address(&SecretName(name.to_owned()))
            .expect("a name");
        block(secrets.storage.write(
            &Target::unbound(address),
            SecretValue::new(value.as_bytes().to_vec()).expect("a value"),
        ))
        .expect("write");
    }

    fn request(referenced: &[&str], model_token: Option<&str>) -> Resolve {
        Resolve {
            actor: crate::actor::daemon(),
            instance: InstanceName("example-org".to_owned()),
            r#box: BoxId(Uuid("6f1c2a4e-0000-4000-8000-000000000001".to_owned())),
            referenced: referenced
                .iter()
                .map(|name| SecretName((*name).to_owned()))
                .collect(),
            model_token: model_token.map(|name| SecretName(name.to_owned())),
            at: Timestamp("2026-11-08T10:00:00Z".to_owned()),
        }
    }

    #[test]
    fn a_box_receives_one_slot_the_model_token_and_nothing_else() {
        let secrets = keychain(None);
        write(&secrets, "example-org-model", "model-value");
        write(&secrets, "example-org-forge", "forge-value");
        let resolution =
            secrets.resolve(&request(&["example-org-forge"], Some("example-org-model")));
        let ResolveSecretsOutcome::Resolved { secrets_resolved } = &resolution.outcome else {
            panic!("not resolved: {:?}", resolution.outcome);
        };
        assert_eq!(resolution.slots.len(), 1);
        assert_eq!(resolution.slots[0].slot.slot_name.0, MODEL_TOKEN_SLOT);
        assert_eq!(resolution.slots[0].slot.secret.0, "example-org-model");
        assert_eq!(resolution.slots[0].value(), b"model-value");
        assert_eq!(
            secrets_resolved.slots,
            vec![resolution.slots[0].slot.clone()]
        );
        let shown = format!("{resolution:?}");
        assert!(!shown.contains("model-value"), "{shown}");
        assert!(!shown.contains("forge-value"), "{shown}");
    }

    #[test]
    fn a_missing_name_refuses_the_box_naming_every_missing_name() {
        let secrets = keychain(Some("example-org"));
        write(&secrets, "example-org-model", "model-value");
        let resolution = secrets.resolve(&request(
            &["example-org-absent", "Not-A-Name", "example-org-absent"],
            Some("example-org-model"),
        ));
        assert!(resolution.slots.is_empty());
        let ResolveSecretsOutcome::Missing { error } = resolution.outcome else {
            panic!("not missing: {:?}", resolution.outcome);
        };
        let names: Vec<&str> = error.names.iter().map(|name| name.0.as_str()).collect();
        assert_eq!(names, ["example-org-absent", "Not-A-Name"]);
    }

    #[test]
    fn the_remote_source_answers_source_unreachable_and_needs_a_url() {
        let without = SecretsSource {
            source: Source::Remote,
            namespace: None,
            url: None,
        };
        let refused = open(&without).expect_err("a remote source without url opened");
        assert_eq!(refused.path, "daemon.secrets.url");
        assert!(
            refused.to_string().starts_with("SourceUnusable: "),
            "{refused}"
        );

        let remote = open(&SecretsSource {
            url: Some("https://secrets.example".to_owned()),
            ..without
        })
        .expect("a remote source with url opens");
        assert!(remote.answers().is_err());
        let resolution = remote.resolve(&request(&[], Some("example-org-model")));
        assert!(matches!(
            resolution.outcome,
            ResolveSecretsOutcome::Unreachable {
                error: SourceUnreachable {
                    source: Source::Remote
                }
            }
        ));
    }

    #[test]
    fn a_namespace_the_crates_refuse_is_unusable() {
        let refused = open(&SecretsSource {
            source: Source::Keychain,
            namespace: Some("Example Org".to_owned()),
            url: None,
        })
        .expect_err("a bad namespace opened");
        assert_eq!(refused.path, "daemon.secrets.namespace");
    }

    #[test]
    fn referenced_names_are_the_globals_and_the_instances_own_never_the_daemons() {
        let text = "version: conductor.config/2\n\
                    daemon: {store: \"postgres://u:secret:db-password@db.example/c\", bearer: secret:daemon-bearer}\n\
                    globals: {roles: [{role: controller, model_token: secret:shared-model}]}\n\
                    instances:\n\
                    \x20 - name: alpha\n    forge_token: secret:alpha-forge\n\
                    \x20 - name: beta\n    note: not-a-secret\n";
        let found = referenced(text);
        let alpha: Vec<&str> = found["alpha"].iter().map(String::as_str).collect();
        let beta: Vec<&str> = found["beta"].iter().map(String::as_str).collect();
        assert_eq!(alpha, ["alpha-forge", "shared-model"]);
        assert_eq!(beta, ["shared-model"]);
    }
}
