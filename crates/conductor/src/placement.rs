//! Placements (`story:placement-substrate-local`, design architecture-next § 6): where an
//! instance runs, a substrate daemon plus an adapter plus a profile. Conductor implements no
//! sandbox: it asks substrate for the box.
//!
//! A `conductor.config/2` instance writes `placement` as a map `{daemon, adapter}`, or as a bare
//! string read as `{daemon: local, adapter: <string>}` (decision P3) ([`read`]). `daemon` is
//! `local` (the default) or an `https://` URL; `adapter` is `systemd`, `linux`, `oci` or `pod`.
//! A local daemon refuses `oci` and `pod` at validate (decision P5): a local daemon's session
//! route serves neither (substrate's ADR 0048 D6). The instance's `placement` name is the
//! canonical text: the adapter for a local daemon, `<adapter>@<url>` for a remote one
//! ([`resolve`]). A bare string that names no adapter resolves to nothing and keeps the
//! service-manager unit it had before substrate placements.
//!
//! Every adapter goes through substrate ([`boxed`]): one substrate workspace per instance, the
//! box, and the runtime (`conductor instance serve`) a substrate PTY session in it, opened
//! through the SDK against `substrate-daemon`'s socket ([`Sdk`]). Substrate puts no adapter name
//! on the wire (its ADR 0048): an adapter is the confinement facts the box requires
//! ([`requires`]), checked against the facts the daemon published (`/v1/machine`) before anything
//! starts ([`unmet`]), and refused with substrate's names, `confinement.family-unserved` and
//! `confinement.fact-unserved`. `systemd` is `linux` inside a delegated systemd scope: the
//! operator runs `substrate-daemon` in that scope, and the daemon then proves `limits` and
//! `whole-tree-kill`. The SDK of 0.20.0 sends no `sandbox.requires` (its builders write `None`),
//! so the daemon cannot hold the start to the check itself; the start names the capability
//! snapshot the facts were read from, so a daemon restarted on other facts refuses it
//! `exec.capability-stale`.
//!
//! The model token reaches a local box through the daemon's slot pool ([`SlotPool`],
//! `daemon.substrate_slots`): a slot is daemon configuration (substrate's design 11), so the
//! daemon writes the instance's `model_token` into a free slot's file before the box opens and
//! names that slot; the file is read when the runtime's run acquires it, and freed when the box
//! closes. A box keeps its lease past one 24-hour lease by renewing it, and its runtime past
//! substrate's hard deadline of one run by a restart in the same box ([`RUN_BOUND`]).
//!
//! The runtime writes the sessions directory under the state directory through a writable host
//! root ([`HostRoot`]): a start names a root, never its path, so the operator declares it on the
//! local daemon, `substrate-daemon --writable-root sessions-<instance>=<state>/sessions`
//! ([`writable_root_name`]).
//!
//! A remote daemon (`story:placement-substrate-remote`) is `placement: {daemon: https://…,
//! adapter, bearer: secret:<name>, trust_roots, server_identity}` ([`read`]): the box opens
//! through the SDK's remote transport ([`Sdk::remote`]), HTTPS with an Identity bearer the
//! daemon's secrets source answers once per request ([`bearer_from`]); the bearer is never in
//! the config, the environment, an argument or a log. A remote daemon that does not answer
//! within [`REMOTE_TIMEOUT`] is [`UNREACHABLE`]; conductord shows its instance `Unreachable`.
//! The runtime's instance API in a remote box is reached through one pipe session per request,
//! `conductor instance relay` on its socket ([`Exchange`], [`RemoteRoute`]); the proxy and the
//! federated views take that way unchanged.
//!
//! [`open_box`] is `conductor.placement.OpenBox`: it builds nothing itself; it asks the
//! [`Substrate`] for the box with the profile of § 6 ([`request`]) and records `BoxOpened`, or,
//! when substrate refuses, `BoxRefused` with substrate's own name, answered as
//! [`PlacementRefusal`] (`PlacementRefused` in the design). Nothing falls back. [`close_box`] is
//! `conductor.placement.CloseBox`.

use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Result, anyhow};
use conductor_model::behaviour::Generated;
use conductor_model::config::{Instance, InstanceName, Limits, Source};
pub use conductor_model::placement::Adapter;
use conductor_model::placement::obligations::{CloseBoxBehavior as _, OpenBoxBehavior as _};
use conductor_model::placement::{
    BoxId, CloseBox, CloseBoxOutcome, OpenBox, OpenBoxOutcome, PlacementName, RefusalReason,
    RemoteDaemon,
};
use conductor_model::primitives::Uuid;
use conductor_model::secret::SecretName;
use secrets_core::storage::SecretValue;
use serde_json::Value;

use crate::store::Store;

/// The daemon a bare placement and a map without `daemon` name.
pub const LOCAL: &str = "local";

/// The model API host every box may reach besides its forges (§ 6).
pub const MODEL_HOST: &str = "api.anthropic.com";

/// The variable naming the local substrate daemon's socket; [`default_socket`] without it.
pub const SOCKET_VARIABLE: &str = "CONDUCTOR_SUBSTRATE_SOCKET";

/// The egress aperture a box's sessions are started with. Substrate 0.20.0 declares apertures on
/// the daemon (`--egress-aperture`), not per request: the operator declares this one with the
/// forge and model hosts of [`BoxRequest::egress`].
pub const EGRESS_APERTURE: &str = "conductor";

/// The descriptor the model token's sealed slot arrives at inside the box.
pub const SLOT_FD: u32 = 3;

/// The name of the refusal when the daemon cannot be reached at all: conductor's, not
/// substrate's, since substrate said nothing.
pub const UNREACHABLE: &str = "placement.substrate-unreachable";

/// The adapters by the word the config writes.
const ADAPTERS: [(&str, Adapter); 4] = [
    ("systemd", Adapter::Systemd),
    ("linux", Adapter::Linux),
    ("oci", Adapter::Oci),
    ("pod", Adapter::Pod),
];

/// The keys of a placement map.
const KEYS: [&str; 5] = [
    "daemon",
    "adapter",
    "bearer",
    "trust_roots",
    "server_identity",
];

/// The word the config writes for `adapter`.
#[must_use]
pub fn adapter_word(adapter: Adapter) -> &'static str {
    ADAPTERS
        .iter()
        .find(|(_, known)| *known == adapter)
        .map_or("", |(word, _)| word)
}

fn adapter_named(word: &str) -> Option<Adapter> {
    ADAPTERS
        .iter()
        .find(|(known, _)| *known == word)
        .map(|(_, adapter)| *adapter)
}

/// The substrate daemon of a placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Where {
    /// `substrate-daemon --socket` on this host.
    Local,
    /// A remote daemon at this URL.
    Remote(String),
}

/// A placement name read as its daemon and adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    /// The daemon.
    pub daemon: Where,
    /// The adapter.
    pub adapter: Adapter,
}

/// The canonical placement name of `daemon` and `adapter`.
#[must_use]
pub fn name_of(daemon: &Where, adapter: Adapter) -> String {
    match daemon {
        Where::Local => adapter_word(adapter).to_owned(),
        Where::Remote(url) => format!("{}@{url}", adapter_word(adapter)),
    }
}

/// The daemon and adapter a placement name stands for; `None` for a name that names no adapter.
#[must_use]
pub fn resolve(name: &str) -> Option<Resolved> {
    match name.split_once('@') {
        Some((word, url)) => Some(Resolved {
            daemon: Where::Remote(url.to_owned()),
            adapter: adapter_named(word)?,
        }),
        None => Some(Resolved {
            daemon: Where::Local,
            adapter: adapter_named(name)?,
        }),
    }
}

/// Whether `instance` runs in a substrate box: any placement that names an adapter.
#[must_use]
pub fn boxed(instance: &Instance) -> bool {
    resolve(&instance.placement.0).is_some()
}

/// What a box requires of the daemon's confinement, in substrate's wire words: the families it
/// must serve and the properties it must prove (substrate's design 30 § 4.3).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Requirement {
    /// The families, in substrate's order.
    pub families: Vec<substrate_sdk::ConfinementFamily>,
    /// The properties, in substrate's order.
    pub facts: Vec<substrate_sdk::ConfinementProperty>,
}

/// The facts `adapter` stands for. `linux`: `linux`'s expected set of substrate's design 30
/// § 4.3 with the egress aperture conductor declares, so every family but `network-ingress` and
/// `limits`, and the two IPC properties. `systemd`: `linux` inside a delegated systemd scope,
/// which adds `limits` and `whole-tree-kill` (substrate-host's `confinement/systemd.rs`: around
/// `linux` the scope is its delegated cgroup root). `oci` and `pod`: the six facts a namespace
/// driver admits an exec on (substrate's ADR 0048), as far as the wire names them.
#[must_use]
pub fn requires(adapter: Adapter) -> Requirement {
    use substrate_sdk::{ConfinementFamily as F, ConfinementProperty as P};
    let linux = [
        F::ProcessRun,
        F::ProcessWatch,
        F::OutputRead,
        F::Wait,
        F::Workspace,
        F::FilesystemView,
        F::NetworkEgress,
        F::Audit,
    ];
    match adapter {
        Adapter::Linux => Requirement {
            families: linux.to_vec(),
            facts: vec![P::IpcSocketAllowlist, P::IpcSignalsContained],
        },
        Adapter::Systemd => Requirement {
            families: F::ALL
                .into_iter()
                .filter(|family| linux.contains(family) || *family == F::Limits)
                .collect(),
            facts: vec![
                P::WholeTreeKill,
                P::IpcSocketAllowlist,
                P::IpcSignalsContained,
            ],
        },
        Adapter::Oci | Adapter::Pod => Requirement {
            families: vec![F::ProcessRun, F::OutputRead, F::Wait, F::Limits],
            facts: vec![P::WholeTreeKill],
        },
    }
}

/// Substrate's name for a family the daemon does not serve.
pub const FAMILY_UNSERVED: &str = "confinement.family-unserved";

/// Substrate's name for a property the daemon does not prove.
pub const FACT_UNSERVED: &str = "confinement.fact-unserved";

/// The refusal `requires` earns against what a daemon serves and proves, as substrate's daemon
/// words it: the first unserved family in substrate's order, then the first unproved property;
/// `None` when every requirement holds.
#[must_use]
pub fn unmet(
    requires: &Requirement,
    served: &[substrate_sdk::ConfinementFamily],
    proved: &[substrate_sdk::ConfinementProperty],
) -> Option<Refused> {
    if let Some(family) = substrate_sdk::ConfinementFamily::ALL
        .into_iter()
        .find(|family| requires.families.contains(family) && !served.contains(family))
    {
        return Some(Refused {
            name: FAMILY_UNSERVED.to_owned(),
            detail: format!(
                "Confinement family {} is not served by this driver.",
                family.name()
            ),
        });
    }
    substrate_sdk::ConfinementProperty::ALL
        .into_iter()
        .find(|property| requires.facts.contains(property) && !proved.contains(property))
        .map(|property| Refused {
            name: FACT_UNSERVED.to_owned(),
            detail: format!(
                "Confinement fact {} is not proved by this driver.",
                property.name()
            ),
        })
}

/// A placement as the config writes it: its canonical name and, for a remote daemon, the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    /// The canonical name ([`name_of`]).
    pub name: String,
    /// The remote daemon; `None` for a local one.
    pub daemon: Option<RemoteDaemon>,
}

/// The host of an `https://` URL: what its certificate is checked for when the placement writes
/// no `server_identity`.
fn url_host(url: &str) -> String {
    let authority = url
        .strip_prefix("https://")
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or_default();
    let authority = authority
        .rsplit_once('@')
        .map_or(authority, |(_, host)| host);
    match authority.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or_default().to_owned(),
        None => authority
            .rsplit_once(':')
            .map_or(authority, |(host, _)| host)
            .to_owned(),
    }
}

/// Reads an instance's `placement` at `path` (decisions P3, P5; `story:placement-substrate-remote`):
/// its canonical name and its remote daemon, or each problem as its YAML path and what is wrong
/// there. A remote daemon writes `bearer: secret:<name>` and `trust_roots`, the PEM file of the
/// roots its certificate is verified against (absolute or under `~`, which is `home`), and may
/// write `server_identity`, the DNS name its certificate carries, the URL's host by default; a
/// local daemon writes none of the three. A `bearer` that is not `secret:<name>` is the config's
/// generic secret-bearing check's problem, at the same path, and is not reported twice.
///
/// # Errors
///
/// Each problem of the value, by path.
pub fn read(value: &Value, path: &str, home: &Path) -> Result<Read, Vec<(String, String)>> {
    let local_refusal = |at: String, word: &str| {
        (
            at,
            format!(
                "`{word}` is refused on a local daemon: a local substrate daemon serves `linux` \
                 and `systemd`"
            ),
        )
    };
    let local = |name: String| Read { name, daemon: None };
    match value {
        Value::String(text) if text.trim().is_empty() => {
            Err(vec![(path.to_owned(), "is empty".to_owned())])
        }
        Value::String(text) => match adapter_named(text) {
            Some(adapter @ (Adapter::Oci | Adapter::Pod)) => {
                Err(vec![local_refusal(path.to_owned(), adapter_word(adapter))])
            }
            _ => Ok(local(text.clone())),
        },
        Value::Object(map) => {
            let mut problems = Vec::new();
            for key in map.keys().filter(|key| !KEYS.contains(&key.as_str())) {
                problems.push((
                    format!("{path}.{key}"),
                    format!(
                        "`{key}` is no key of a placement: write daemon, adapter and, for a \
                         remote daemon, bearer, trust_roots and server_identity"
                    ),
                ));
            }
            let daemon = match map.get("daemon") {
                None => Some(Where::Local),
                Some(Value::String(word)) if word == LOCAL => Some(Where::Local),
                Some(Value::String(url)) if url.starts_with("https://") => {
                    Some(Where::Remote(url.clone()))
                }
                Some(other) => {
                    problems.push((
                        format!("{path}.daemon"),
                        format!(
                            "{} is no daemon: write `local` or an https:// URL",
                            shown(other)
                        ),
                    ));
                    None
                }
            };
            let adapter = match map.get("adapter") {
                None => {
                    problems.push((format!("{path}.adapter"), "missing".to_owned()));
                    None
                }
                Some(Value::String(word)) => {
                    let adapter = adapter_named(word);
                    if adapter.is_none() {
                        problems.push((
                            format!("{path}.adapter"),
                            format!("`{word}` is no adapter: write systemd, linux, oci or pod"),
                        ));
                    }
                    adapter
                }
                Some(other) => {
                    problems.push((
                        format!("{path}.adapter"),
                        format!("{} is not text", shown(other)),
                    ));
                    None
                }
            };
            if let (Some(Where::Local), Some(adapter @ (Adapter::Oci | Adapter::Pod))) =
                (&daemon, adapter)
            {
                problems.push(local_refusal(
                    format!("{path}.adapter"),
                    adapter_word(adapter),
                ));
            }
            let text = |key: &str, problems: &mut Vec<(String, String)>| match map.get(key) {
                None => None,
                Some(Value::String(text)) if !text.trim().is_empty() => Some(text.clone()),
                Some(other) => {
                    problems.push((
                        format!("{path}.{key}"),
                        format!("{} is not text", shown(other)),
                    ));
                    None
                }
            };
            let bearer = text("bearer", &mut problems);
            let roots = text("trust_roots", &mut problems);
            let identity = text("server_identity", &mut problems);
            let remote = match &daemon {
                Some(Where::Remote(url)) => {
                    let bearer = match bearer {
                        None => {
                            problems.push((
                                format!("{path}.bearer"),
                                "missing: a remote daemon is reached with a bearer, written \
                                 secret:<name>"
                                    .to_owned(),
                            ));
                            None
                        }
                        Some(text) => text
                            .strip_prefix("secret:")
                            .filter(|_| crate::config::secret_name(&text))
                            .map(|name| SecretName(name.to_owned())),
                    };
                    let roots = match roots {
                        None => {
                            problems.push((
                                format!("{path}.trust_roots"),
                                "missing: a remote daemon's certificate is verified against the \
                                 PEM roots this file names, and against no others"
                                    .to_owned(),
                            ));
                            None
                        }
                        Some(text) if text == "~" => Some(home.display().to_string()),
                        Some(text) => match text.strip_prefix("~/") {
                            Some(rest) => Some(home.join(rest).display().to_string()),
                            None if Path::new(&text).is_absolute() => Some(text),
                            None => {
                                problems.push((
                                    format!("{path}.trust_roots"),
                                    format!(
                                        "`{text}` is not absolute: write it absolute or under ~"
                                    ),
                                ));
                                None
                            }
                        },
                    };
                    match (bearer, roots) {
                        (Some(bearer), Some(trust_roots)) => Some(RemoteDaemon {
                            url: url.clone(),
                            bearer,
                            trust_roots,
                            server_identity: identity.unwrap_or_else(|| url_host(url)),
                        }),
                        _ => None,
                    }
                }
                _ => {
                    for (key, written) in [
                        ("bearer", &bearer),
                        ("trust_roots", &roots),
                        ("server_identity", &identity),
                    ] {
                        if written.is_some() {
                            problems.push((
                                format!("{path}.{key}"),
                                format!("`{key}` is for a remote daemon: a local one takes none"),
                            ));
                        }
                    }
                    None
                }
            };
            match (daemon, adapter) {
                (Some(daemon), Some(adapter)) if problems.is_empty() => {
                    if matches!(daemon, Where::Remote(_)) && remote.is_none() {
                        // The bearer is not `secret:<name>`: the generic check names it.
                        return Err(problems);
                    }
                    Ok(Read {
                        name: name_of(&daemon, adapter),
                        daemon: remote,
                    })
                }
                _ => Err(problems),
            }
        }
        other => Err(vec![(
            path.to_owned(),
            format!(
                "{} is no placement: write {{daemon, adapter}} or an adapter",
                shown(other)
            ),
        )]),
    }
}

fn shown(value: &Value) -> String {
    match value {
        Value::String(text) => format!("`{text}`"),
        other => format!("`{other}`"),
    }
}

// ---------------------------------------------------------------------------------------------
// The profile of § 6
// ---------------------------------------------------------------------------------------------

/// What conductord asks substrate for, for one instance (§ 6).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoxRequest {
    /// The instance.
    pub instance: InstanceName,
    /// Its placement's name.
    pub placement: PlacementName,
    /// `conductor --config <file> instance serve`.
    pub argv: Vec<String>,
    /// The instance's state directory, where the runtime starts.
    pub cwd: String,
    /// Variables set for the runtime; never a credential.
    pub env: Vec<(String, String)>,
    /// Host directories mounted read-only at their own path: the conductor binary's directory
    /// (it carries the role profiles), the harness's directory, the checkouts root when the
    /// trees are elsewhere.
    pub read_only_roots: Vec<String>,
    /// The six writable places, each a subdirectory of the one workspace.
    pub places: Vec<String>,
    /// The instance's limits.
    pub limits: Limits,
    /// The only hosts the box may reach: the forges of its sources and [`MODEL_HOST`].
    pub egress: Vec<String>,
    /// The sealed slots the box receives: the model token (`story:secrets-source`). [`request`]
    /// names [`crate::secret::MODEL_TOKEN_SLOT`]; for a local daemon the reconciler replaces it
    /// with the pool slot it wrote the token into ([`SlotPool`]), or with none.
    pub secret_slots: Vec<String>,
    /// The confinement facts the box requires ([`requires`]).
    pub requires: Requirement,
    /// The host directories the box must write outside its workspace: the sessions directory
    /// under the state directory, where the terminal log and the envelope are written. [`Sdk`]
    /// names each in the runtime's start, mounted at its own path, in this order.
    pub writable_roots: Vec<HostRoot>,
}

/// A writable host root (substrate's ADR 0038 kind 1): a start names it, never its path, so
/// the daemon must declare it, `substrate-daemon --writable-root <name>=<path>`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRoot {
    /// The name the daemon declares ([`writable_root_name`]).
    pub name: String,
    /// The host directory, mounted at the same path in the box.
    pub path: String,
}

/// The name of `instance`'s sessions root: `sessions-<instance>`, each byte outside letters,
/// digits, `_` and `-` written `_`, so it is in substrate's root-name set.
#[must_use]
pub fn writable_root_name(instance: &str) -> String {
    let name: String = instance
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!("sessions-{name}")
}

/// The sessions directory of the instance whose state directory is `state`, the box's writable
/// host root: `<state>/sessions`.
#[must_use]
pub fn sessions_dir(state: &Path) -> PathBuf {
    state.join("sessions")
}

/// The runtime's log file in the sessions directory ([`crate::instance::LOG_VARIABLE`]):
/// substrate ends a PTY session whose output reaches its bound (`session.output-limit`), so the
/// runtime in a box writes what it says here and its session stays quiet.
pub const RUNTIME_LOG: &str = "runtime.log";

/// Creates the sessions directory of `state` ([`sessions_dir`]) mode 0700, or makes an existing
/// one 0700, so the box's writable host root is there before the box is asked for.
///
/// # Errors
///
/// The directory cannot be created or restricted.
pub fn prepare_sessions(state: &Path) -> Result<()> {
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    let dir = sessions_dir(state);
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&dir)
        .map_err(|error| anyhow!("create {}: {error}", dir.display()))?;
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
        .map_err(|error| anyhow!("restrict {}: {error}", dir.display()))
}

/// Substrate's name for a start naming a writable host root the daemon does not serve
/// (`substrate_wire::EXEC_WRITABLE_ROOTS_HOST_UNSERVED`, which the SDK does not re-export).
pub const ROOT_UNSERVED: &str = "exec.writable-roots-host-unserved";

/// The refusal `roots` earn against the writable host roots a daemon serves, `served` its
/// `exec.writable-roots.host` fact (`None` when it publishes none): the first root it does not
/// list, refused before anything starts as substrate's daemon would at admission.
#[must_use]
pub fn roots_unserved(roots: &[HostRoot], served: Option<&[String]>) -> Option<Refused> {
    let served = served.unwrap_or_default();
    roots
        .iter()
        .find(|root| !served.contains(&root.name))
        .map(|root| Refused {
            name: ROOT_UNSERVED.to_owned(),
            detail: format!(
                "the daemon serves no writable host root {}: declare it with \
                 substrate-daemon --writable-root {}={}",
                root.name, root.name, root.path
            ),
        })
}

/// The box `instance` asks for, its runtime the binary at `binary` reading `config_file`, with
/// `env` set beside nothing inherited, and the runtime's log file ([`RUNTIME_LOG`]) in its
/// writable root.
#[must_use]
pub fn request(
    instance: &Instance,
    config_file: &Path,
    binary: &Path,
    mut env: Vec<(String, String)>,
) -> BoxRequest {
    let sessions = sessions_dir(Path::new(&instance.state));
    env.push((
        crate::instance::LOG_VARIABLE.to_owned(),
        sessions.join(RUNTIME_LOG).display().to_string(),
    ));
    let mut read_only_roots: Vec<String> = Vec::new();
    let mut add = |path: Option<PathBuf>| {
        if let Some(path) = path {
            let text = path.display().to_string();
            if !text.is_empty() && !read_only_roots.contains(&text) {
                read_only_roots.push(text);
            }
        }
    };
    add(binary.parent().map(Path::to_owned));
    add(crate::doctor::find(crate::confine::CLAUDE)
        .and_then(|path| path.parent().map(Path::to_owned)));
    if !Path::new(&instance.checkouts.trees).starts_with(&instance.checkouts.root) {
        add(Some(PathBuf::from(&instance.checkouts.root)));
    }
    let mut egress: Vec<String> = Vec::new();
    for source in &instance.sources {
        let host = match source {
            Source::GitHub(_) => crate::doctor::GITHUB_HOST.to_owned(),
            Source::GitLab(gitlab) => gitlab.host.clone(),
            Source::Local(_) => continue,
        };
        if !egress.contains(&host) {
            egress.push(host);
        }
    }
    egress.push(MODEL_HOST.to_owned());
    BoxRequest {
        instance: instance.name.clone(),
        placement: PlacementName(instance.placement.0.clone()),
        argv: vec![
            binary.display().to_string(),
            "--config".to_owned(),
            config_file.display().to_string(),
            "instance".to_owned(),
            "serve".to_owned(),
        ],
        cwd: instance.state.clone(),
        env,
        read_only_roots,
        places: crate::config::PLACES.map(str::to_owned).to_vec(),
        limits: instance.limits.clone(),
        egress,
        secret_slots: vec![crate::secret::MODEL_TOKEN_SLOT.to_owned()],
        requires: resolve(&instance.placement.0)
            .map(|resolved| requires(resolved.adapter))
            .unwrap_or_default(),
        writable_roots: vec![HostRoot {
            name: writable_root_name(&instance.name.0),
            path: sessions.display().to_string(),
        }],
    }
}

// ---------------------------------------------------------------------------------------------
// The model-token slot pool
// ---------------------------------------------------------------------------------------------

/// The most slots `daemon.substrate_slots` may name.
pub const MAX_SLOTS: i64 = 64;

/// The directory of the pool's slot files, under the daemon's own directory.
pub const SLOTS_DIR: &str = "slots";

/// The name of the refusal when every slot of the pool holds another box's token.
pub const SLOTS_EXHAUSTED: &str = "placement.secret-slots-exhausted";

/// The name of the refusal when the instance's model token cannot be had from the secrets source.
pub const MODEL_TOKEN_UNAVAILABLE: &str = "placement.model-token-unavailable";

/// What a free slot's file holds: no secret, and not empty, since substrate's daemon refuses to
/// start on an empty slot file.
const FREE_SLOT: &[u8] = b"free\n";

/// Where the daemon's secret values come from: the secrets source and a name, the value or
/// `None`. A seam, so a test answers without a keychain.
pub type TokenSource = std::sync::Arc<
    dyn Fn(&conductor_model::config::SecretsSource, &SecretName) -> Option<SecretValue>
        + Send
        + Sync,
>;

/// The daemon's secrets source ([`crate::secret::open`]).
#[must_use]
pub fn token_source() -> TokenSource {
    std::sync::Arc::new(|source, name| crate::secret::open(source).ok()?.reveal(name).ok()?)
}

/// The local substrate daemon's model-token slots: `model_token_<i>` for each `i` below `size`,
/// each one file `<dir>/model_token_<i>` the operator declares with `--secret-slot
/// model_token_<i>=<file>`. A slot is held by the box whose token its file holds.
#[derive(Debug, Clone)]
pub struct SlotPool {
    /// The directory of the slot files.
    pub dir: PathBuf,
    /// How many slots.
    pub size: usize,
}

impl SlotPool {
    /// The name of slot `index`.
    #[must_use]
    pub fn slot(index: usize) -> String {
        format!("{}_{index}", crate::secret::MODEL_TOKEN_SLOT)
    }

    /// Creates the directory (mode 0700) and every slot file that is missing, each holding no
    /// secret (mode 0600), so the substrate daemon that declares them can start.
    ///
    /// # Errors
    ///
    /// A directory or file cannot be written.
    pub fn prepare(&self) -> Result<()> {
        use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
        if self.size == 0 {
            return Ok(());
        }
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(&self.dir)
            .map_err(|error| anyhow!("create {}: {error}", self.dir.display()))?;
        std::fs::set_permissions(&self.dir, std::fs::Permissions::from_mode(0o700))
            .map_err(|error| anyhow!("restrict {}: {error}", self.dir.display()))?;
        for index in 0..self.size {
            let file = self.dir.join(Self::slot(index));
            if !file.exists() {
                self.write(&file, FREE_SLOT)?;
            }
        }
        Ok(())
    }

    fn write(&self, file: &Path, bytes: &[u8]) -> Result<()> {
        use std::io::Write as _;
        use std::os::unix::fs::{OpenOptionsExt as _, PermissionsExt as _};
        let mut out = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(file)
            .map_err(|error| anyhow!("open the slot file {}: {error}", file.display()))?;
        out.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| anyhow!("restrict the slot file {}: {error}", file.display()))?;
        out.write_all(bytes)
            .map_err(|error| anyhow!("write the slot file {}: {error}", file.display()))
    }

    /// Writes `value` into the first slot not in `held` and answers its name.
    ///
    /// # Errors
    ///
    /// [`SLOTS_EXHAUSTED`] when every slot is held, or the file cannot be written.
    pub fn acquire(
        &self,
        held: &std::collections::BTreeSet<String>,
        value: &SecretValue,
    ) -> Result<String, Refused> {
        let slot = (0..self.size)
            .map(Self::slot)
            .find(|slot| !held.contains(slot))
            .ok_or_else(|| Refused {
                name: SLOTS_EXHAUSTED.to_owned(),
                detail: format!(
                    "all {} slots of daemon.substrate_slots hold another box's model token",
                    self.size
                ),
            })?;
        self.prepare()
            .and_then(|()| self.write(&self.dir.join(&slot), value.expose()))
            .map_err(|error| Refused {
                name: SLOTS_EXHAUSTED.to_owned(),
                detail: format!("{error:#}"),
            })?;
        Ok(slot)
    }

    /// Frees `slot`: its file holds no secret again.
    ///
    /// # Errors
    ///
    /// The file cannot be written.
    pub fn free(&self, slot: &str) -> Result<()> {
        self.write(&self.dir.join(slot), FREE_SLOT)
    }
}

/// A box opener that refuses every box with `0`, before substrate is asked: what [`open_box`]
/// records when conductor cannot make the request at all (no free slot, no model token).
#[derive(Debug, Clone)]
pub struct Refusing(pub Refused);

impl Substrate for Refusing {
    fn open(&mut self, _request: &BoxRequest) -> Result<Opened, Refused> {
        Err(self.0.clone())
    }
    fn running(&mut self, _session: &str) -> Result<bool> {
        Ok(false)
    }
    fn stop(&mut self, _session: &str) -> Result<()> {
        Ok(())
    }
    fn close(&mut self, _workspace: &str) -> Result<()> {
        Ok(())
    }
}

// ---------------------------------------------------------------------------------------------
// The substrate seam
// ---------------------------------------------------------------------------------------------

/// A box substrate opened: its workspace and the runtime's PTY session in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    /// The substrate workspace, the box.
    pub workspace: String,
    /// The runtime's session.
    pub session: String,
}

/// Substrate's refusal, by its own name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// Substrate's name, such as `exec.sandbox-unavailable`.
    pub name: String,
    /// What it said beside the name.
    pub detail: String,
}

/// What conductord asks of a substrate daemon, behind a trait so a test runs against a fake.
pub trait Substrate {
    /// Opens the box `request` asks for and starts its runtime in it.
    ///
    /// # Errors
    ///
    /// Substrate's refusal, by name.
    fn open(&mut self, request: &BoxRequest) -> Result<Opened, Refused>;

    /// Whether the session `session` runs.
    ///
    /// # Errors
    ///
    /// The daemon cannot be asked.
    fn running(&mut self, session: &str) -> Result<bool>;

    /// Ends the session `session`.
    ///
    /// # Errors
    ///
    /// The daemon refused or cannot be asked.
    fn stop(&mut self, session: &str) -> Result<()>;

    /// Destroys the workspace `workspace`.
    ///
    /// # Errors
    ///
    /// The daemon refused or cannot be asked.
    fn close(&mut self, workspace: &str) -> Result<()>;

    /// Renews the leases of the workspace `workspace` and of its runtime's session `session` by
    /// `ttl` (substrate 0.20.0's `renew_lease`).
    ///
    /// # Errors
    ///
    /// The daemon refused or cannot be asked.
    fn renew(&mut self, workspace: &str, session: &str, ttl: Duration) -> Result<()> {
        let _ = (session, ttl);
        Err(anyhow!(
            "{RENEW_UNSERVED}: no lease renewal for the box {workspace}"
        ))
    }

    /// Starts the runtime `request` asks for anew in the open workspace `workspace` and answers
    /// its session: the runtime's run nears substrate's hard deadline ([`RUN_BOUND`]).
    ///
    /// # Errors
    ///
    /// Substrate's refusal, by name.
    fn restart(&mut self, workspace: &str, request: &BoxRequest) -> Result<String, Refused> {
        let _ = request;
        Err(Refused {
            name: RENEW_UNSERVED.to_owned(),
            detail: format!("no runtime restart in the box {workspace}"),
        })
    }

    /// Runs `exchange` as a pipe session in its box, writes `input` to its standard input,
    /// closes it, and answers everything it wrote to its standard output by the time it exited
    /// (`story:placement-substrate-remote`: one instance API exchange through
    /// `conductor instance relay`).
    ///
    /// # Errors
    ///
    /// Substrate's refusal, by name, or [`REMOTE_API_UNSERVED`] where the daemon runs no session.
    fn exchange(&mut self, exchange: &Exchange, input: &[u8]) -> Result<Vec<u8>, Refused> {
        let _ = input;
        Err(Refused {
            name: REMOTE_API_UNSERVED.to_owned(),
            detail: format!("no pipe session in the box {}", exchange.workspace),
        })
    }
}

/// One relay session in a box (`story:placement-substrate-remote`): `conductor --config <file>
/// instance relay` with the instance's socket in its environment and the box's read-only roots,
/// so the box's own binary bridges one exchange to the runtime's `api.sock`. Never a credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exchange {
    /// The box's substrate workspace.
    pub workspace: String,
    /// The relay's argument vector, the box's conductor first.
    pub argv: Vec<String>,
    /// The relay's environment: the instance's socket ([`crate::instance::SOCKET_VARIABLE`]).
    pub env: Vec<(String, String)>,
    /// Host directories the relay sees read-only, as the box's runtime does.
    pub read_only_roots: Vec<String>,
}

impl Exchange {
    /// The relay of the box `workspace` opened for `request`, reaching the runtime at `socket`.
    #[must_use]
    pub fn relay(workspace: &str, request: &BoxRequest, socket: &Path) -> Self {
        let mut argv: Vec<String> = request.argv.iter().take(3).cloned().collect();
        argv.extend(["instance".to_owned(), "relay".to_owned()]);
        Self {
            workspace: workspace.to_owned(),
            argv,
            env: vec![(
                crate::instance::SOCKET_VARIABLE.to_owned(),
                socket.display().to_string(),
            )],
            read_only_roots: request.read_only_roots.clone(),
        }
    }
}

/// The route of a runtime in a box on a remote daemon: the daemon, its bearer, what opens it,
/// and the relay session an exchange runs as.
#[derive(Clone)]
pub struct RemoteRoute {
    /// The daemon.
    pub at: RemoteAt,
    /// Its bearer, asked once per request.
    pub bearer: Bearer,
    /// What opens the daemon.
    pub opener: RemoteSubstrate,
    /// The relay session.
    pub exchange: Exchange,
}

impl fmt::Debug for RemoteRoute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RemoteRoute")
            .field("at", &self.at)
            .field("exchange", &self.exchange)
            .finish_non_exhaustive()
    }
}

impl PartialEq for RemoteRoute {
    fn eq(&self, other: &Self) -> bool {
        self.at == other.at && self.exchange == other.exchange
    }
}

impl Eq for RemoteRoute {}

impl RemoteRoute {
    /// Sends `request`, one whole HTTP request, to the runtime through one relay session, and
    /// answers what it answered, byte for byte.
    ///
    /// # Errors
    ///
    /// The daemon refused the session or does not answer, or the relay wrote nothing.
    pub fn send(&self, request: &[u8]) -> Result<Vec<u8>> {
        let mut substrate = (self.opener)(&self.at, self.bearer.clone());
        let answer = substrate
            .exchange(&self.exchange, request)
            .map_err(|why| anyhow!("{}: {}", why.name, why.detail))?;
        if answer.is_empty() {
            return Err(anyhow!(
                "the relay in the box {} answered nothing",
                self.exchange.workspace
            ));
        }
        Ok(answer)
    }
}

/// The default socket of the local substrate daemon: `$XDG_RUNTIME_DIR/substrate/daemon.sock`,
/// else `~/.cache/b10x/conductor/substrate.sock`.
#[must_use]
pub fn default_socket() -> PathBuf {
    if let Some(path) = std::env::var_os(SOCKET_VARIABLE).filter(|path| !path.is_empty()) {
        return PathBuf::from(path);
    }
    if let Some(dir) = std::env::var_os("XDG_RUNTIME_DIR").filter(|dir| !dir.is_empty()) {
        return PathBuf::from(dir).join("substrate/daemon.sock");
    }
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default()
        .join(".cache/b10x/conductor/substrate.sock")
}

/// A remote substrate daemon as conductord keeps it beside a box it opened there
/// (`conductor.placement.RemoteDaemon`): its URL, the name of its bearer's secret, its trust
/// roots' file and the DNS name its certificate carries. Never a credential.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RemoteAt {
    /// The daemon's `https://` origin.
    pub url: String,
    /// The secret the bearer is kept under, without `secret:`.
    pub bearer: String,
    /// The PEM file of the roots its certificate is verified against.
    pub trust_roots: String,
    /// The DNS name its certificate carries.
    pub server_identity: String,
}

impl From<&RemoteDaemon> for RemoteAt {
    fn from(daemon: &RemoteDaemon) -> Self {
        Self {
            url: daemon.url.clone(),
            bearer: daemon.bearer.0.clone(),
            trust_roots: daemon.trust_roots.clone(),
            server_identity: daemon.server_identity.clone(),
        }
    }
}

/// Where a remote daemon's bearer comes from: asked once per request, so the value is held only
/// while that request is sent, and never in the config, the environment, an argument or a log.
pub type Bearer = std::sync::Arc<dyn Fn() -> Option<SecretValue> + Send + Sync>;

/// The bearer of `at` from the daemon's secrets source `source` ([`crate::secret`]): `None`
/// when the source does not open, does not answer or holds no such secret.
#[must_use]
pub fn bearer_from(source: &conductor_model::config::SecretsSource, at: &RemoteAt) -> Bearer {
    let source = source.clone();
    let name = conductor_model::secret::SecretName(at.bearer.clone());
    std::sync::Arc::new(move || {
        crate::secret::open(&source)
            .ok()?
            .reveal(&name)
            .ok()
            .flatten()
    })
}

/// The name of the refusal when a remote daemon's bearer cannot be had: the secrets source does
/// not hold it, or it is not an Identity access credential.
pub const BEARER_UNAVAILABLE: &str = "placement.bearer-unavailable";

/// The name of the refusal of an instance API exchange by a box opener that runs no pipe session
/// ([`Substrate::exchange`]'s default): substrate 0.20.0 offers no stream into a box but a
/// session's own input and output, so an exchange with a runtime in a remote box is a relay
/// session ([`Exchange`]).
pub const REMOTE_API_UNSERVED: &str = "placement.remote-api-unserved";

/// How long one question to a remote daemon may take before the daemon counts as silent.
pub const REMOTE_TIMEOUT: Duration = Duration::from_secs(10);

/// A remote daemon to reach: where, and its bearer.
#[derive(Clone)]
struct Remote {
    at: RemoteAt,
    bearer: Bearer,
}

impl fmt::Debug for Remote {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Remote")
            .field("at", &self.at)
            .finish_non_exhaustive()
    }
}

/// The substrate daemon of a box, through the substrate SDK: the local one over its Unix socket,
/// or a remote one over HTTPS with its bearer ([`Sdk::remote`]).
#[derive(Debug, Clone)]
pub struct Sdk {
    /// The local daemon's socket.
    pub socket: PathBuf,
    /// The remote daemon, when the box is opened on one.
    remote: Option<Remote>,
    /// How long one question to a remote daemon may take ([`REMOTE_TIMEOUT`]).
    pub timeout: Duration,
}

impl Default for Sdk {
    fn default() -> Self {
        Self {
            socket: default_socket(),
            remote: None,
            timeout: REMOTE_TIMEOUT,
        }
    }
}

/// What makes the box opener of a remote daemon, given where it is and its bearer.
/// Shared by the reconciler and every route the API proxies through ([`RemoteRoute`]).
pub type RemoteSubstrate =
    std::sync::Arc<dyn Fn(&RemoteAt, Bearer) -> Box<dyn Substrate + Send> + Send + Sync>;

/// The remote daemons through the SDK's remote transport ([`Sdk::remote`]).
pub fn sdk_remote() -> RemoteSubstrate {
    std::sync::Arc::new(|at, bearer| Box::new(Sdk::remote(at.clone(), bearer)))
}

impl<T: Substrate + ?Sized> Substrate for &mut T {
    fn open(&mut self, request: &BoxRequest) -> Result<Opened, Refused> {
        (**self).open(request)
    }
    fn running(&mut self, session: &str) -> Result<bool> {
        (**self).running(session)
    }
    fn stop(&mut self, session: &str) -> Result<()> {
        (**self).stop(session)
    }
    fn close(&mut self, workspace: &str) -> Result<()> {
        (**self).close(workspace)
    }
    fn renew(&mut self, workspace: &str, session: &str, ttl: Duration) -> Result<()> {
        (**self).renew(workspace, session, ttl)
    }
    fn restart(&mut self, workspace: &str, request: &BoxRequest) -> Result<String, Refused> {
        (**self).restart(workspace, request)
    }
    fn exchange(&mut self, exchange: &Exchange, input: &[u8]) -> Result<Vec<u8>, Refused> {
        (**self).exchange(exchange, input)
    }
}

impl Sdk {
    /// The remote daemon `at`, reached with the bearer `bearer` gives.
    #[must_use]
    pub fn remote(at: RemoteAt, bearer: Bearer) -> Self {
        Self {
            socket: PathBuf::new(),
            remote: Some(Remote { at, bearer }),
            timeout: REMOTE_TIMEOUT,
        }
    }
}

/// How long one lease and one run of the runtime last before substrate ends them: substrate
/// 0.20.0 bounds one lease and one run's hard deadline (`limits.timeout_ms`) at 24 hours
/// (`MAX_EXEC_DURATION`). A lease is renewed every [`RENEW_EVERY`]; a run is restarted in the
/// same box [`RESTART_BEFORE`] its deadline.
pub const RUN_BOUND: Duration = substrate_sdk::MAX_EXEC_DURATION;

/// How often the daemon renews a box's leases.
pub const RENEW_EVERY: Duration = Duration::from_secs(3600);

/// How long before [`RUN_BOUND`] the daemon restarts a box's runtime.
pub const RESTART_BEFORE: Duration = Duration::from_secs(3600);

/// The name of the refusal of a renewal or restart by a box opener that serves neither.
pub const RENEW_UNSERVED: &str = "placement.renewal-unserved";

/// The memory a box gets when the instance sets none: 8 GiB.
const DEFAULT_MEMORY: u64 = 8 << 30;

/// Each output stream's bound: the most the SDK admits (`MAX_IO_BYTES`, 1 MiB); a larger one is
/// refused before anything is sent.
const OUTPUT_BYTES: u64 = substrate_sdk::MAX_IO_BYTES;

fn refused(error: substrate_sdk::SdkError) -> Refused {
    match error {
        substrate_sdk::SdkError::Refusal(refusal) => Refused {
            name: refusal.code,
            detail: refusal.message,
        },
        substrate_sdk::SdkError::TokenUnavailable => Refused {
            name: BEARER_UNAVAILABLE.to_owned(),
            detail: "the secrets source holds no Identity access credential under the \
                     placement's bearer"
                .to_owned(),
        },
        substrate_sdk::SdkError::Transport(detail) => Refused {
            name: UNREACHABLE.to_owned(),
            detail,
        },
        other => Refused {
            name: UNREACHABLE.to_owned(),
            detail: other.to_string(),
        },
    }
}

impl Sdk {
    /// A client of this daemon: the local one over its socket, or the remote one over HTTPS,
    /// trusting only its trust roots, its certificate checked for its server identity, and its
    /// bearer asked of [`Bearer`] for each request.
    async fn client(&self) -> Result<substrate_sdk::Client, substrate_sdk::SdkError> {
        let Some(remote) = &self.remote else {
            return substrate_sdk::Client::builder()
                .unix_socket(&self.socket)
                .connect()
                .await;
        };
        // The SDK's TLS takes the process's provider; conductor's is ring.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let origin = if remote.at.url.ends_with('/') {
            remote.at.url.clone()
        } else {
            format!("{}/", remote.at.url)
        };
        let bearer = remote.bearer.clone();
        substrate_sdk::Client::builder()
            .https_endpoint(origin)
            .trust_roots(&remote.at.trust_roots)
            .server_identity(remote.at.server_identity.clone())
            .token_provider(move |_reason| {
                let value = bearer();
                async move {
                    let value = value.ok_or(substrate_sdk::SdkError::TokenUnavailable)?;
                    let text = std::str::from_utf8(value.expose())
                        .map_err(|_| substrate_sdk::SdkError::TokenUnavailable)?;
                    substrate_sdk::AccessToken::new(text.trim().to_owned())
                }
            })
            .connect()
            .await
    }

    fn block_on<T>(
        &self,
        work: impl AsyncFnOnce(substrate_sdk::Client) -> Result<T, substrate_sdk::SdkError>,
    ) -> Result<T, Refused> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| Refused {
                name: UNREACHABLE.to_owned(),
                detail: format!("start the SDK's runtime: {error}"),
            })?;
        let asked = async {
            let client = self.client().await.map_err(refused)?;
            work(client).await.map_err(refused)
        };
        runtime.block_on(async {
            if self.remote.is_none() {
                return asked.await;
            }
            tokio::time::timeout(self.timeout, asked)
                .await
                .unwrap_or_else(|_| {
                    Err(Refused {
                        name: UNREACHABLE.to_owned(),
                        detail: format!("the daemon did not answer within {:?}", self.timeout),
                    })
                })
        })
    }
}

impl Sdk {
    /// Starts the runtime `request` asks for as a PTY session in `workspace` and answers its id.
    async fn runtime(
        workspace: &substrate_sdk::Workspace,
        request: &BoxRequest,
    ) -> Result<String, substrate_sdk::SdkError> {
        let memory = request
            .limits
            .memory_max
            .as_ref()
            .map_or(DEFAULT_MEMORY, |size| {
                u64::try_from(size.0).unwrap_or(0).saturating_mul(1 << 30)
            })
            .min(substrate_sdk::MAX_EXEC_MEMORY_BYTES);
        let processes = request
            .limits
            .tasks_max
            .map_or(substrate_sdk::MAX_EXEC_PROCESSES, |tasks| {
                u32::try_from(tasks).unwrap_or(substrate_sdk::MAX_EXEC_PROCESSES)
            })
            .min(substrate_sdk::MAX_EXEC_PROCESSES);
        let policy = substrate_sdk::ExecutionPolicy::builder()
            .timeout(RUN_BOUND)
            .cpu_time(RUN_BOUND)
            .memory_bytes(memory)
            .processes(processes)
            .output_bytes(OUTPUT_BYTES)
            .build()?;
        let (program, args) = request
            .argv
            .split_first()
            .ok_or(substrate_sdk::SdkError::Builder { field: "program" })?;
        let mut session = workspace
            .pty_session(
                program.clone(),
                substrate_sdk::PtyWindow {
                    columns: 200,
                    rows: 50,
                },
            )
            .args(args.iter().cloned())
            .policy(policy)
            .lease(RUN_BOUND)
            .input_limit_bytes(substrate_sdk::MAX_SESSION_INPUT_BYTES)
            .frame_limit_bytes(substrate_sdk::MAX_SESSION_FRAME_BYTES)
            .queued_frames(substrate_sdk::MAX_SESSION_QUEUED_FRAMES)
            .aperture(EGRESS_APERTURE);
        for (name, value) in &request.env {
            session = session.env(name.clone(), value.clone());
        }
        for root in &request.read_only_roots {
            session = session.read_only_root(substrate_sdk::ReadOnlyRoot {
                host_path: root.clone(),
                mount: root.clone(),
            });
        }
        for root in &request.writable_roots {
            session = session.writable_root(substrate_sdk::WritableRoot::Host(
                substrate_sdk::WritableHostRoot {
                    host: root.name.clone(),
                    mount: root.path.clone(),
                },
            ));
        }
        for slot in &request.secret_slots {
            session = session.secret_slot(substrate_sdk::SecretSlotRequest {
                slot: slot.clone(),
                fd: SLOT_FD,
            });
        }
        let started = session.start().await?;
        Ok(started.id().to_owned())
    }
}

impl Substrate for Sdk {
    fn open(&mut self, request: &BoxRequest) -> Result<Opened, Refused> {
        let request = request.clone();
        self.block_on(async move |client| {
            // The facts the start's capability snapshot carries: the same client's machine.
            let machine = client.machine();
            if let Some(refused) = unmet(
                &request.requires,
                &machine.served_families(),
                &machine.proved_properties(),
            ) {
                return Ok(Err(refused));
            }
            if let Some(refused) = roots_unserved(
                &request.writable_roots,
                machine.facts.exec_writable_roots_host.as_deref(),
            ) {
                return Ok(Err(refused));
            }
            let workspace = client
                .workspace()
                .label("conductor.instance", request.instance.0.clone())
                .label("conductor.placement", request.placement.0.clone())
                .lease(RUN_BOUND)
                .create()
                .await?;
            let session = Self::runtime(&workspace, &request).await?;
            Ok(Ok(Opened {
                workspace: workspace.id().to_owned(),
                session,
            }))
        })
        .and_then(|opened| opened)
    }

    fn renew(&mut self, workspace: &str, session: &str, ttl: Duration) -> Result<()> {
        let (workspace, session) = (workspace.to_owned(), session.to_owned());
        self.block_on(async move |client| {
            client
                .get_workspace(&workspace)
                .await?
                .renew_lease(ttl)
                .await?;
            client
                .get_pipe_session(&session)
                .await?
                .renew_lease(ttl)
                .await?;
            Ok(())
        })
        .map_err(|why| anyhow!("{}: {}", why.name, why.detail))
    }

    fn restart(&mut self, workspace: &str, request: &BoxRequest) -> Result<String, Refused> {
        let (id, request) = (workspace.to_owned(), request.clone());
        self.block_on(async move |client| {
            let machine = client.machine();
            if let Some(refused) = unmet(
                &request.requires,
                &machine.served_families(),
                &machine.proved_properties(),
            ) {
                return Ok(Err(refused));
            }
            if let Some(refused) = roots_unserved(
                &request.writable_roots,
                machine.facts.exec_writable_roots_host.as_deref(),
            ) {
                return Ok(Err(refused));
            }
            let workspace = client.get_workspace(&id).await?;
            Ok(Ok(Self::runtime(&workspace, &request).await?))
        })
        .and_then(|session| session)
    }

    fn running(&mut self, session: &str) -> Result<bool> {
        let id = session.to_owned();
        self.block_on(async move |client| {
            let session = client.get_pipe_session(&id).await?;
            Ok(matches!(
                session.observation().state,
                substrate_sdk::PipeSessionState::Accepted
                    | substrate_sdk::PipeSessionState::Ready
                    | substrate_sdk::PipeSessionState::Attached
            ))
        })
        .map_err(|why| anyhow!("{}: {}", why.name, why.detail))
    }

    fn stop(&mut self, session: &str) -> Result<()> {
        let id = session.to_owned();
        self.block_on(async move |client| {
            client.get_pipe_session(&id).await?.retire().await?;
            Ok(())
        })
        .map_err(|why| anyhow!("{}: {}", why.name, why.detail))
    }

    fn close(&mut self, workspace: &str) -> Result<()> {
        let id = workspace.to_owned();
        self.block_on(async move |client| {
            client.get_workspace(&id).await?.destroy().await?;
            Ok(())
        })
        .map_err(|why| anyhow!("{}: {}", why.name, why.detail))
    }

    fn exchange(&mut self, exchange: &Exchange, input: &[u8]) -> Result<Vec<u8>, Refused> {
        let exchange = exchange.clone();
        let input = input.to_vec();
        self.block_on(async move |client| {
            let workspace = client.get_workspace(&exchange.workspace).await?;
            let (program, args) = exchange
                .argv
                .split_first()
                .ok_or(substrate_sdk::SdkError::Builder { field: "program" })?;
            let policy = substrate_sdk::ExecutionPolicy::builder()
                .timeout(RELAY_BOUND)
                .cpu_time(RELAY_BOUND)
                .memory_bytes(RELAY_MEMORY)
                .processes(RELAY_PROCESSES)
                .output_bytes(OUTPUT_BYTES)
                .build()?;
            let mut session = workspace
                .pipe_session(program.clone())
                .args(args.iter().cloned())
                .policy(policy)
                .lease(RELAY_BOUND)
                .input_limit_bytes(substrate_sdk::MAX_SESSION_INPUT_BYTES)
                .frame_limit_bytes(substrate_sdk::MAX_SESSION_FRAME_BYTES)
                .queued_frames(substrate_sdk::MAX_SESSION_QUEUED_FRAMES);
            for (name, value) in &exchange.env {
                session = session.env(name.clone(), value.clone());
            }
            for root in &exchange.read_only_roots {
                session = session.read_only_root(substrate_sdk::ReadOnlyRoot {
                    host_path: root.clone(),
                    mount: root.clone(),
                });
            }
            let started = session.start().await?;
            let mut channel = started.attach().await?;
            for chunk in input.chunks(RELAY_CHUNK) {
                channel.write(chunk).await?;
            }
            channel.close_input().await?;
            let mut answer = Vec::new();
            while let Some(frame) = channel.next_frame().await? {
                match frame {
                    substrate_sdk::PipeFrame::Output {
                        stream: substrate_sdk::OutputStream::Stdout,
                        bytes,
                        ..
                    } => answer.extend_from_slice(&bytes),
                    substrate_sdk::PipeFrame::Exit { .. } => break,
                    substrate_sdk::PipeFrame::ProtocolError { message, .. } => {
                        return Err(substrate_sdk::SdkError::Protocol(message));
                    }
                    _ => {}
                }
            }
            let _ = channel.close().await;
            let _ = started.retire().await;
            Ok(answer)
        })
    }
}

/// How long one relay session may run: one instance API exchange, as long as the daemon's proxy
/// waits for an answer.
const RELAY_BOUND: Duration = Duration::from_secs(60);

/// The memory one relay session gets: 256 MiB.
const RELAY_MEMORY: u64 = 256 << 20;

/// The processes one relay session may run.
const RELAY_PROCESSES: u32 = 16;

/// The bytes of one standard-input frame of a relay session: half the frame bound, which the
/// frame's base64 encoding stays within.
const RELAY_CHUNK: usize = 32 * 1024;

// ---------------------------------------------------------------------------------------------
// OpenBox and CloseBox
// ---------------------------------------------------------------------------------------------

/// An open box, as conductord keeps it.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct OpenedBox {
    /// The box's id on record.
    pub box_id: String,
    /// Its substrate workspace.
    pub workspace: String,
    /// Its runtime's session.
    pub session: String,
    /// The remote daemon it was opened on; `None` for the local one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteAt>,
    /// The pool slot its model token is in ([`SlotPool`]); `None` when it holds none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slot: Option<String>,
    /// When its runtime's current run started, in seconds since the epoch; 0 when not known.
    #[serde(default)]
    pub started_at: u64,
    /// When its leases were last renewed, in seconds since the epoch; 0 when not known.
    #[serde(default)]
    pub renewed_at: u64,
}

/// A box substrate refused (`PlacementRefused` in design § 6), on record as `BoxRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementRefusal {
    /// The refused box's id on record.
    pub box_id: String,
    /// The placement.
    pub placement: String,
    /// The instance.
    pub instance: String,
    /// Substrate's name.
    pub reason: String,
    /// What it said beside the name.
    pub detail: String,
}

impl fmt::Display for PlacementRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "PlacementRefused: the placement {} refused the box of {}: {}: {}",
            self.placement, self.instance, self.reason, self.detail
        )
    }
}

impl std::error::Error for PlacementRefusal {}

/// The store `OpenBox` and `CloseBox` run on, with what substrate answered.
struct Ports<'s> {
    store: &'s mut Store,
    refusal: Option<String>,
    box_id: BoxId,
}

impl conductor_model::behaviour::BoxStorage for Ports<'_> {
    fn get(&self, identity: &BoxId) -> Option<conductor_model::placement::BoxSnapshot> {
        conductor_model::behaviour::BoxStorage::get(&*self.store, identity)
    }
    fn put(&mut self, snapshot: conductor_model::placement::BoxSnapshot) {
        conductor_model::behaviour::BoxStorage::put(self.store, snapshot);
    }
    fn delete(&mut self, identity: &BoxId) {
        conductor_model::behaviour::BoxStorage::delete(self.store, identity);
    }
    fn list(&self) -> Vec<conductor_model::placement::BoxSnapshot> {
        conductor_model::behaviour::BoxStorage::list(&*self.store)
    }
}

impl conductor_model::behaviour::BoxRefusalStorage for Ports<'_> {
    fn get(&self, identity: &BoxId) -> Option<conductor_model::placement::BoxRefusalSnapshot> {
        conductor_model::behaviour::BoxRefusalStorage::get(&*self.store, identity)
    }
    fn put(&mut self, snapshot: conductor_model::placement::BoxRefusalSnapshot) {
        conductor_model::behaviour::BoxRefusalStorage::put(self.store, snapshot);
    }
    fn delete(&mut self, identity: &BoxId) {
        conductor_model::behaviour::BoxRefusalStorage::delete(self.store, identity);
    }
    fn list(&self) -> Vec<conductor_model::placement::BoxRefusalSnapshot> {
        conductor_model::behaviour::BoxRefusalStorage::list(&*self.store)
    }
}

macro_rules! unmet {
    ($($method:ident -> $type:ty, $name:literal;)*) => {
        $(fn $method(&mut self) -> Result<$type, conductor_model::obligation::UnmetObligation> {
            Err(conductor_model::behaviour::unmet_context($name))
        })*
    };
}

impl conductor_model::behaviour::TryContext for Ports<'_> {
    unmet! {
        try_generate_conductor_config_revision_id -> conductor_model::config::RevisionId, "conductor.config.RevisionId";
        try_generate_conductor_direction_serving_id -> conductor_model::direction::ServingId, "conductor.direction.ServingId";
        try_generate_conductor_dispatch_guard_decision_id -> conductor_model::dispatch::GuardDecisionId, "conductor.dispatch.GuardDecisionId";
        try_generate_conductor_dispatch_message_id -> conductor_model::dispatch::MessageId, "conductor.dispatch.MessageId";
        try_generate_conductor_dispatch_resource_request_id -> conductor_model::dispatch::ResourceRequestId, "conductor.dispatch.ResourceRequestId";
        try_generate_conductor_host_grant_id -> conductor_model::host::GrantId, "conductor.host.GrantId";
        try_generate_conductor_host_reclaim_run_id -> conductor_model::host::ReclaimRunId, "conductor.host.ReclaimRunId";
        try_generate_conductor_instance_reconciliation_id -> conductor_model::instance::ReconciliationId, "conductor.instance.ReconciliationId";
        try_generate_conductor_observation_board_id -> conductor_model::observation::BoardId, "conductor.observation.BoardId";
        try_generate_conductor_observation_observation_id -> conductor_model::observation::ObservationId, "conductor.observation.ObservationId";
        try_generate_conductor_observation_snapshot_id -> conductor_model::observation::SnapshotId, "conductor.observation.SnapshotId";
        try_generate_conductor_session_delivery_id -> conductor_model::session::DeliveryId, "conductor.session.DeliveryId";
        try_generate_conductor_session_tick_id -> conductor_model::session::TickId, "conductor.session.TickId";
        try_generate_conductor_token_token_id -> conductor_model::token::TokenId, "conductor.token.TokenId";
        try_generate_conductor_tool_tool_call_id -> conductor_model::tool::ToolCallId, "conductor.tool.ToolCallId";
        try_generate_string -> String, "String";
    }

    fn try_generate_conductor_placement_box_id(
        &mut self,
    ) -> Result<BoxId, conductor_model::obligation::UnmetObligation> {
        Ok(self.box_id.clone())
    }

    fn try_generate_conductor_placement_refusal_reason(
        &mut self,
    ) -> Result<RefusalReason, conductor_model::obligation::UnmetObligation> {
        self.refusal.clone().map(RefusalReason).ok_or_else(|| {
            conductor_model::behaviour::unmet_context("conductor.placement.RefusalReason")
        })
    }

    fn try_external(
        &mut self,
        command: conductor_model::behaviour::ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, conductor_model::obligation::UnmetObligation> {
        match (command, outcome) {
            // Substrate answered before the command ran ([`open_box`]); a host policy's refusal
            // of the adapter comes back under substrate's own name, recorded as `refused`.
            (
                conductor_model::behaviour::ExternalCommand::ConductorPlacementOpenBox(_),
                "refused",
            ) => Ok(self.refusal.is_some()),
            (
                conductor_model::behaviour::ExternalCommand::ConductorPlacementOpenBox(_),
                "adapter-refused",
            ) => Ok(false),
            (command, _) => Err(conductor_model::behaviour::unmet_context(command.name())),
        }
    }
}

fn new_box_id() -> BoxId {
    BoxId(Uuid(eventlog_core::new_event_id()))
}

/// `conductor.placement.OpenBox` for `request`, as the module documentation says.
///
/// # Errors
///
/// [`PlacementRefusal`] when substrate refused the box, or the record could not be written (its
/// reason then names the store).
pub fn open_box(
    store: &mut Store,
    substrate: &mut dyn Substrate,
    request: &BoxRequest,
) -> Result<OpenedBox, PlacementRefusal> {
    let answer = substrate.open(request);
    let box_id = new_box_id();
    let (refusal, detail) = match &answer {
        Ok(_) => (None, String::new()),
        Err(why) => (Some(why.name.clone()), why.detail.clone()),
    };
    let input = OpenBox {
        actor: crate::actor::daemon(),
        placement: request.placement.clone(),
        instance: request.instance.clone(),
        at: crate::records::now()
            .unwrap_or_else(|_| conductor_model::primitives::Timestamp(String::new())),
    };
    let mut generated = Generated::new(Ports {
        store,
        refusal: refusal.clone(),
        box_id: box_id.clone(),
    });
    let refuse = |reason: String, detail: String| PlacementRefusal {
        box_id: box_id.0.0.clone(),
        placement: request.placement.0.clone(),
        instance: request.instance.0.clone(),
        reason,
        detail,
    };
    let outcome = generated
        .open_box(input)
        .map_err(|unmet| refuse("placement.record-failed".to_owned(), unmet.to_string()))?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| refuse("placement.record-failed".to_owned(), error.to_string()))?;
    match (outcome, answer) {
        (OpenBoxOutcome::Opened { .. }, Ok(opened)) => Ok(OpenedBox {
            box_id: box_id.0.0.clone(),
            workspace: opened.workspace,
            session: opened.session,
            remote: None,
            slot: None,
            started_at: 0,
            renewed_at: 0,
        }),
        _ => Err(refuse(refusal.unwrap_or_default(), detail)),
    }
}

/// `conductor.placement.CloseBox` for `opened`: ends its runtime's session, destroys its
/// workspace and records `BoxClosed`.
///
/// # Errors
///
/// Substrate refused the stop or the close, or the record could not be written.
pub fn close_box(
    store: &mut Store,
    substrate: &mut dyn Substrate,
    opened: &OpenedBox,
) -> Result<()> {
    if substrate.running(&opened.session).unwrap_or(true) {
        substrate.stop(&opened.session)?;
    }
    substrate.close(&opened.workspace)?;
    let input = CloseBox {
        actor: crate::actor::daemon(),
        box_id: BoxId(Uuid(opened.box_id.clone())),
        at: crate::records::now()?,
    };
    let mut generated = Generated::new(Ports {
        store,
        refusal: None,
        box_id: new_box_id(),
    });
    let outcome = generated
        .close_box(input)
        .map_err(|unmet| anyhow!("close the box {}: {unmet}", opened.box_id))?;
    generated
        .ports
        .store
        .check()
        .map_err(|error| anyhow!("record the closed box {}: {error}", opened.box_id))?;
    match outcome {
        CloseBoxOutcome::Closed { .. } => Ok(()),
        CloseBoxOutcome::NoSuchBox { .. } => {
            Err(anyhow!("BoxNotFound: no box has the id {}", opened.box_id))
        }
        CloseBoxOutcome::WrongState { .. } => Err(anyhow!(
            "BoxStateConflict: the box {} is already closed",
            opened.box_id
        )),
    }
}
