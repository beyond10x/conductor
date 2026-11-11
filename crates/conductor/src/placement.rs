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
//! The token conductord proxies writes with reaches the runtime in a local box through a sealed
//! slot of its own (`story:box-reaches-forge-and-daemon`, [`HostSlot`], [`api_token_slot`]),
//! never through the box's environment: conductord writes it into the slot's file under its
//! own directory before each start ([`seal_api_token`]) where the local daemon declares the
//! slot ([`Substrate::declares`]), which the operator does. The slot is optional: a box whose
//! daemon does not declare it is asked for without it, as before there was one, and nothing is
//! refused; its runtime mints a token of its own, and `conductor doctor` says so with the flag
//! ([`slot_undeclared`]). The runtime reads the token at the descriptor substrate names
//! ([`crate::instance::SLOTS_VARIABLE`]). A box on a remote daemon gets no such slot.
//!
//! What the box sees of the host is one list ([`mounts`], `story:box-runtime-starts`): what the
//! runtime and the loop write (the state directory, the records, the cache, the directory of a
//! sqlite store) and what they read (the binary's and the harness's directories, the checkouts root,
//! the config file, the harness binary, each forge's CLI and its login). The runtime writes
//! through writable host roots ([`HostRoot`]): a start names a root, never its path, so the
//! operator declares each on the local daemon, `substrate-daemon --writable-root
//! state-<instance>=<state>` ([`writable_root_name`]); the refusal of an undeclared one prints
//! the flag ([`roots_unserved`]). A box writes nothing else: a layout whose writable directory
//! would hold the config file or conductord's own directory is refused before substrate is
//! asked ([`unmountable`], [`ROOT_SHARED`]), and so is a sqlite store written without an
//! absolute path ([`STORE_NOT_ABSOLUTE`]). The box's `HOME` is the daemon's.
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
/// the daemon (`--egress-aperture <name>=<host>:<port>/tcp`), not per request, and one aperture
/// is one destination. The SDK of 0.20.0 names one aperture in a start (its builders keep one
/// name and send no list), so the box reaches the one destination the operator declares this
/// aperture with, the model host, and none of the forges of [`BoxRequest::egress`].
pub const EGRESS_APERTURE: &str = "conductor";

/// The descriptor the model token's sealed slot arrives at inside the box.
pub const SLOT_FD: u32 = 3;

/// The descriptor the sealed slot of the daemon's token ([`HostSlot`]) arrives at inside the box.
pub const API_TOKEN_FD: u32 = 4;

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
    /// The instance's state directory, where a runtime started as a unit starts. No start
    /// carries it: the SDK of 0.20.0 has no working directory to set (its builders send none),
    /// so substrate starts the runtime in the box's workspace.
    pub cwd: String,
    /// Variables set for the runtime; never a credential.
    pub env: Vec<(String, String)>,
    /// Host directories mounted read-only at their own path: the read-only roots of [`mounts`].
    pub read_only_roots: Vec<String>,
    /// Single host files mounted read-only: the read-only files of [`mounts`]. [`Sdk`] names
    /// each in the runtime's start.
    pub read_only_files: Vec<HostFile>,
    /// The six writable places, each a subdirectory of the one workspace.
    pub places: Vec<String>,
    /// The instance's limits.
    pub limits: Limits,
    /// The hosts the runtime asks for: the forges of its sources and [`MODEL_HOST`]. No start
    /// carries them ([`EGRESS_APERTURE`]).
    pub egress: Vec<String>,
    /// The sealed slots the box receives: the model token (`story:secrets-source`). [`request`]
    /// names [`crate::secret::MODEL_TOKEN_SLOT`]; for a local daemon the reconciler replaces it
    /// with the pool slot it wrote the token into ([`SlotPool`]), or with none.
    pub secret_slots: Vec<String>,
    /// The sealed slot the runtime takes the daemon's token through
    /// (`story:box-reaches-forge-and-daemon`): the token conductord proxies writes with, never
    /// in [`Self::env`]. `None` for a box on a remote daemon, whose slots are files of that
    /// host, and where conductord's directory is not known ([`Host::daemon`]). [`request_on`]
    /// names the slot the instance would take; the reconciler keeps it only where the local
    /// daemon declares it ([`Substrate::declares`]), so a start never names one it does not.
    pub api_token: Option<HostSlot>,
    /// The confinement facts the box requires ([`requires`]).
    pub requires: Requirement,
    /// The host directories the box must write outside its workspace: the writable roots of
    /// [`mounts`], the state directory first. [`Sdk`] names each in the runtime's start, mounted
    /// at its own path, in this order.
    pub writable_roots: Vec<HostRoot>,
    /// Why conductor cannot ask for this box at all ([`unmountable`]): [`open_box`] and a
    /// restart refuse with it before substrate is asked.
    pub refused: Option<Refused>,
}

/// A single host file the box reads (substrate's ADR 0033): the regular file at `host_path`,
/// reached through no symlink, appears at `mount`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostFile {
    /// The host file, every symlink on its way resolved.
    pub host_path: String,
    /// Where the box sees it.
    pub mount: String,
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

/// The name of `instance`'s writable root for `what` (`state`, `records`, `cache`): `<what>-<instance>`,
/// each byte of the instance outside letters, digits, `_` and `-` written `_`, so it is in
/// substrate's root-name set.
#[must_use]
pub fn writable_root_name(what: &str, instance: &str) -> String {
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
    format!("{what}-{name}")
}

/// The name of the writable root that holds a sqlite `daemon.store`: one database for every
/// instance of the file, so one root, declared once.
pub const STORE_ROOT: &str = "store";

/// The sessions directory of the instance whose state directory is `state`, in the box's
/// writable state root: `<state>/sessions`.
#[must_use]
pub fn sessions_dir(state: &Path) -> PathBuf {
    state.join("sessions")
}

/// The runtime's log file in the sessions directory ([`crate::instance::LOG_VARIABLE`]):
/// substrate ends a PTY session whose output reaches its bound (`session.output-limit`), so the
/// runtime in a box writes what it says here and its session stays quiet.
pub const RUNTIME_LOG: &str = "runtime.log";

/// Creates the sessions directory of `state` ([`sessions_dir`]) mode 0700, or makes an existing
/// one 0700, so it and the state root around it are there before the box is asked for.
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

/// Conductor's name for a writable root whose directory is not there: substrate refuses it,
/// and `conductor doctor` names it before any box is asked for.
pub const ROOT_MISSING: &str = "placement.writable-root-missing";

/// The refusal `roots` earn on this host: the first whose path is no directory.
#[must_use]
pub fn roots_missing(roots: &[HostRoot]) -> Option<Refused> {
    roots
        .iter()
        .find(|root| !Path::new(&root.path).is_dir())
        .map(|root| Refused {
            name: ROOT_MISSING.to_owned(),
            detail: format!(
                "the writable root {} is no directory: create {}",
                root.name, root.path
            ),
        })
}

/// A sealed slot of the local substrate daemon on a file of conductord's (substrate's ADR
/// 0012): a start names the slot and a descriptor, never the file or its content, so the
/// operator declares it, `substrate-daemon --secret-slot <name>=<file>`, and substrate hands
/// the runtime what the file holds at the start, sealed, at [`Self::fd`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostSlot {
    /// The name the daemon declares ([`api_token_slot`]).
    pub name: String,
    /// The file on the daemon's host that conductord writes.
    pub file: String,
    /// The descriptor the slot arrives at inside the box.
    pub fd: u32,
}

/// The most bytes a slot name holds (substrate-wire's slot-name rule, `[a-z][a-z0-9_]{0,63}`).
const SLOT_NAME_BOUND: usize = 64;

/// The name of the sealed slot `instance` takes the daemon's token through:
/// [`crate::secret::API_TOKEN_SLOT`], `_`, and the instance in lower case with each byte outside
/// letters and digits written `_`, cut at substrate's bound, so it is in substrate's slot-name
/// set. Two instances whose names differ only in such bytes share the slot: conductord writes
/// the file just before each start, and substrate reads it at that start.
#[must_use]
pub fn api_token_slot(instance: &str) -> String {
    let mut name = format!("{}_", crate::secret::API_TOKEN_SLOT);
    name.extend(instance.chars().map(|c| {
        if c.is_ascii_alphanumeric() {
            c.to_ascii_lowercase()
        } else {
            '_'
        }
    }));
    name.truncate(SLOT_NAME_BOUND);
    name
}

/// Whether `name` is the sealed slot of a daemon's token ([`api_token_slot`]).
#[must_use]
pub fn is_api_token_slot(name: &str) -> bool {
    name.strip_prefix(crate::secret::API_TOKEN_SLOT)
        .is_some_and(|rest| rest.starts_with('_'))
}

/// What follows for a box when the daemon does not declare `slot`, `served` its `secrets.slots`
/// fact (`None` when it publishes none), with the flag that declares it; `None` when the daemon
/// declares it or the box has none. It is no refusal: the box is asked for without the slot.
#[must_use]
pub fn slot_undeclared(slot: Option<&HostSlot>, served: Option<&[String]>) -> Option<String> {
    let slot = slot?;
    if declared(served, &slot.name) {
        return None;
    }
    Some(format!(
        "the daemon's token does not reach the box, so its runtime refuses each write \
         conductord proxies to it: the substrate daemon declares no secret slot {}; declare it \
         with substrate-daemon --secret-slot {}={}, the file there, not empty and mode 600, \
         before that daemon starts",
        slot.name, slot.name, slot.file
    ))
}

/// What a daemon's facts say of one box before anything is asked for ([`Sdk::admits`]).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Admission {
    /// Why the daemon would refuse the box; `None` when its facts serve it.
    pub refused: Option<Refused>,
    /// Why the daemon's token would not reach the runtime of a box the daemon would open
    /// ([`slot_undeclared`]); `None` when it would, and for a box it would refuse.
    pub unsealed: Option<String>,
}

/// Conductor's name for a daemon's token that could not be written into its slot's file.
pub const TOKEN_UNSEALED: &str = "placement.api-token-unsealed";

/// Whether `served`, a daemon's `secrets.slots` fact (`None` when it publishes none), declares
/// the slot `name`: what conductord asks before it names the slot in a start
/// ([`Substrate::declares`]) and what doctor's dry run asks ([`Sdk::admits`]).
fn declared(served: Option<&[String]>, name: &str) -> bool {
    served.is_some_and(|served| served.iter().any(|one| one == name))
}

/// What keeps conductord from writing the slot file `file` as [`seal_api_token`] writes it,
/// found without writing anything: an ancestor of the file that is there and is no directory;
/// where the slot's directory is not there, the nearest ancestor that is not writable by this
/// user; a slot directory that is not this user's (it is made 0700) or, for a file not there
/// yet, not writable; a file there that is no regular file, is not this user's (it is made
/// 0600) or is not writable. [`seal_api_token`] asks it before it writes and `conductor doctor`'s
/// dry run asks it ([`api_token_unsealable`]), so the two refuse a box alike; what only a write
/// meets, a full disk, only conductord sees.
fn slot_unwritable(file: &Path) -> Result<()> {
    use nix::unistd::{AccessFlags, access};
    use std::os::unix::fs::MetadataExt as _;
    let Some(dir) = file.parent() else {
        return Ok(());
    };
    // The owner of `/proc/self` is the effective user of this process; root restricts any file.
    let me = std::fs::metadata("/proc/self")
        .map(|own| own.uid())
        .ok()
        .filter(|uid| *uid != 0);
    let mine = |path: &Path, meta: &std::fs::Metadata| match me {
        Some(me) if meta.uid() != me => Err(anyhow!("{} is not this user's", path.display())),
        _ => Ok(()),
    };
    let writable = |path: &Path, flags: AccessFlags| {
        access(path, flags).map_err(|errno| anyhow!("{} is not writable: {errno}", path.display()))
    };
    // The nearest of the directory and its ancestors that is there, followed as `create` does.
    let mut there = dir;
    while std::fs::symlink_metadata(there).is_err() {
        match there.parent() {
            Some(up) => there = up,
            None => return Ok(()),
        }
    }
    let Some(meta) = std::fs::metadata(there)
        .ok()
        .filter(std::fs::Metadata::is_dir)
    else {
        return Err(anyhow!(
            "create {}: {} is not a directory",
            dir.display(),
            there.display()
        ));
    };
    if there != dir {
        return writable(there, AccessFlags::W_OK | AccessFlags::X_OK)
            .map_err(|error| anyhow!("create {}: {error:#}", dir.display()));
    }
    mine(dir, &meta).map_err(|error| anyhow!("restrict {}: {error:#}", dir.display()))?;
    if std::fs::symlink_metadata(file).is_err() {
        return writable(dir, AccessFlags::W_OK | AccessFlags::X_OK)
            .map_err(|error| anyhow!("create the slot file {}: {error:#}", file.display()));
    }
    let meta = std::fs::metadata(file)
        .map_err(|error| anyhow!("open the slot file {}: {error}", file.display()))?;
    if !meta.is_file() {
        return Err(anyhow!(
            "open the slot file {}: it is not a regular file",
            file.display()
        ));
    }
    mine(file, &meta)
        .and_then(|()| writable(file, AccessFlags::W_OK))
        .map_err(|error| anyhow!("open the slot file {}: {error:#}", file.display()))
}

/// The refusal of a box whose daemon's token is not in its slot `slot`, for `error`.
fn unsealed(slot: &HostSlot, error: &anyhow::Error) -> Refused {
    Refused {
        name: TOKEN_UNSEALED.to_owned(),
        detail: format!(
            "the daemon's token for the runtime is not in its slot {}: {error:#}",
            slot.name
        ),
    }
}

/// The refusal [`seal_api_token`] would answer for `slot` before it writes anything
/// ([`slot_unwritable`]), found without writing: `conductor doctor`'s dry run of a box whose
/// daemon declares the slot.
fn api_token_unsealable(slot: &HostSlot) -> Option<Refused> {
    slot_unwritable(Path::new(&slot.file))
        .err()
        .map(|error| unsealed(slot, &error))
}

/// Writes `token` into the file of `slot` (mode 0600, its directory 0700), which substrate
/// reads when the runtime's run acquires the slot, once [`slot_unwritable`] finds nothing in its
/// way.
///
/// # Errors
///
/// [`TOKEN_UNSEALED`] when the directory or the file cannot be written.
pub fn seal_api_token(slot: &HostSlot, token: &str) -> Result<(), Refused> {
    use std::os::unix::fs::{DirBuilderExt as _, PermissionsExt as _};
    let file = Path::new(&slot.file);
    let written = slot_unwritable(file)
        .and_then(|()| match file.parent() {
            Some(dir) => std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(dir)
                .and_then(|()| {
                    std::fs::set_permissions(dir, std::fs::Permissions::from_mode(0o700))
                })
                .map_err(|error| anyhow!("create {}: {error}", dir.display())),
            None => Ok(()),
        })
        .and_then(|()| write_slot_file(file, format!("{token}\n").as_bytes()));
    written.map_err(|error| unsealed(slot, &error))
}

/// Writes `bytes` as all the slot file `file` holds, mode 0600.
fn write_slot_file(file: &Path, bytes: &[u8]) -> Result<()> {
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

/// How the box sees one host path of [`mounts`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Bind {
    /// A directory the box reads (substrate's ADR 0010).
    ReadOnlyRoot,
    /// A single file the box reads (substrate's ADR 0033).
    ReadOnlyFile,
    /// A directory the box writes, by the name the daemon declares it under (substrate's ADR
    /// 0038 kind 1).
    WritableRoot(String),
}

/// One host path the box sees.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Mount {
    /// What it is to the runtime: `state`, `records`, `cache`, `store`, `binary`, `harness`,
    /// `checkouts`, `config`, `forge-cli` or `forge-login`.
    pub what: &'static str,
    /// The path on the host; of a file, with every symlink on its way resolved.
    pub host_path: String,
    /// The path in the box.
    pub mount: String,
    /// How the box sees it.
    pub bind: Bind,
}

/// What of the daemon's host decides a box's mounts: the three things the runtime finds through
/// its own environment, and the daemon's own directory.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Host {
    /// `HOME`: where each forge CLI keeps its login.
    pub home: Option<PathBuf>,
    /// `PATH`: where the harness and the forge CLIs are found.
    pub path: Option<std::ffi::OsString>,
    /// The store URL ([`crate::store::STORE_VARIABLE`]).
    pub store: Option<String>,
    /// conductord's own directory ([`crate::daemon::DIR`] under `HOME`, unless it was started on
    /// another): the runtimes' tokens and the binaries it starts, which no box writes.
    pub daemon: Option<PathBuf>,
}

impl Host {
    /// The host as the runtime's environment `env` names it; without a `PATH` there, this
    /// process's. The daemon's directory is the default one under `HOME`.
    #[must_use]
    pub fn of(env: &[(String, String)]) -> Self {
        let named = |name: &str| {
            env.iter()
                .find(|(known, value)| known == name && !value.is_empty())
                .map(|(_, value)| value.clone())
        };
        let home = named("HOME")
            .map(PathBuf::from)
            .filter(|home| home.is_absolute());
        Self {
            daemon: home.as_ref().map(|home| home.join(crate::daemon::DIR)),
            home,
            path: named("PATH")
                .map(std::ffi::OsString::from)
                .or_else(|| std::env::var_os("PATH")),
            store: named(crate::store::STORE_VARIABLE),
        }
    }

    /// The sqlite file the store URL names, as written but `~/` read under [`Self::home`];
    /// `None` for any other store.
    fn store_path(&self) -> Option<PathBuf> {
        let path = self.store.as_deref()?.strip_prefix("sqlite:")?;
        let path = path
            .strip_prefix("//")
            .filter(|rest| rest.starts_with('/'))
            .unwrap_or(path);
        Some(match path.strip_prefix("~/") {
            Some(rest) => self.home.as_ref()?.join(rest),
            None => PathBuf::from(path),
        })
    }

    /// The sqlite file the store URL names ([`Self::store_path`]) as a mount takes it; `None`
    /// for any other store and for a file that is not absolute ([`STORE_NOT_ABSOLUTE`]).
    fn store_file(&self) -> Option<PathBuf> {
        let file = clean(&self.store_path()?);
        (file.is_absolute() && file.parent().is_some()).then_some(file)
    }

    fn find(&self, program: &str) -> Option<PathBuf> {
        crate::doctor::find_in(program, self.path.as_deref()?)
    }
}

/// The directories every box has of the host's base system, read-only (substrate's reserved
/// mounts): a path at or under one is already there, and a mount on it is refused.
const BASE: [&str; 4] = ["/usr", "/bin", "/lib", "/lib64"];

/// The files of a forge CLI's login under `HOME`, by the CLI's program.
const LOGINS: [(&str, &[&str]); 2] = [
    ("gh", &[".config/gh/hosts.yml", ".config/gh/config.yml"]),
    ("glab", &[".config/glab-cli/config.yml"]),
];

/// The most read-only directories one start names (substrate-wire's `MAX_READ_ONLY_ROOTS`).
pub const READ_ONLY_ROOTS_BOUND: usize = 4;
/// The most read-only files one start names (substrate-wire's `MAX_READ_ONLY_FILES`).
pub const READ_ONLY_FILES_BOUND: usize = 16;
/// The most writable roots one start names (substrate-wire's `MAX_WRITABLE_ROOTS`).
pub const WRITABLE_ROOTS_BOUND: usize = 8;

/// A directory the box must write holds the config file or lies with conductord's own
/// directory: a writable root is a whole directory, so the box would write them too. Conductor's
/// own refusal, before substrate is asked.
pub const ROOT_SHARED: &str = "placement.writable-root-shared";

/// The box would name more directories or files than one start may ([`READ_ONLY_ROOTS_BOUND`],
/// [`READ_ONLY_FILES_BOUND`], [`WRITABLE_ROOTS_BOUND`]). Conductor's own refusal, before
/// substrate is asked.
pub const MOUNTS_EXCEEDED: &str = "placement.mounts-exceeded";

/// The sqlite store is written without an absolute path (`sqlite:conductor.sqlite`): conductord
/// opens it in its working directory, and no directory of it can be named for the box to write,
/// so the runtime would open another database. Conductor's own refusal, before substrate is
/// asked.
pub const STORE_NOT_ABSOLUTE: &str = "placement.store-not-absolute";

/// `path` as substrate takes a mount: no trailing slash, no empty and no `.` component, and each
/// `..` taken with the component before it. Every path of [`mounts`] goes through it.
fn clean(path: &Path) -> PathBuf {
    let mut cleaned = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                cleaned.pop();
            }
            other => cleaned.push(other),
        }
    }
    cleaned
}

fn at_or_under(path: &str, root: &str) -> bool {
    path == root
        || path
            .strip_prefix(root)
            .is_some_and(|rest| rest.starts_with('/'))
}

/// Adds `mount` to `kept` unless the box sees its path already, and says whether it did.
/// Substrate refuses mounts that nest, so of two writable directories one inside the other the
/// outer one stays, under its own name, a path to read at or under a mount or the base system
/// is not named again, and a directory to read that holds a writable one is never added
/// ([`beside`] names its entries instead).
fn admit(kept: &mut Vec<Mount>, mount: Mount) -> bool {
    if !Path::new(&mount.mount).is_absolute() || !Path::new(&mount.host_path).is_absolute() {
        return false;
    }
    let seen = kept.iter().any(|known| {
        known.mount == mount.mount
            || (known.bind != Bind::ReadOnlyFile && at_or_under(&mount.mount, &known.mount))
    });
    if seen {
        return false;
    }
    let inside = |known: &Mount| at_or_under(&known.mount, &mount.mount);
    match mount.bind {
        Bind::WritableRoot(_) => match kept.iter().position(inside) {
            Some(first) => {
                let mut index = 0;
                kept.retain(|known| {
                    index += 1;
                    index - 1 == first || !inside(known)
                });
                kept[first] = mount;
            }
            None => kept.push(mount),
        },
        Bind::ReadOnlyRoot | Bind::ReadOnlyFile => {
            if BASE.iter().any(|base| at_or_under(&mount.mount, base)) {
                return false;
            }
            if kept
                .iter()
                .any(|known| matches!(known.bind, Bind::WritableRoot(_)) && inside(known))
            {
                return false;
            }
            kept.retain(|known| !inside(known));
            kept.push(mount);
        }
    }
    true
}

fn text(path: &Path) -> String {
    clean(path).display().to_string()
}

fn real(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| clean(path))
}

fn read_only_root(what: &'static str, path: &Path) -> Mount {
    Mount {
        what,
        host_path: text(path),
        mount: text(path),
        bind: Bind::ReadOnlyRoot,
    }
}

fn read_only_file(what: &'static str, path: &Path, mount: &Path) -> Mount {
    Mount {
        what,
        host_path: text(path),
        mount: text(mount),
        bind: Bind::ReadOnlyFile,
    }
}

/// The directory `dir` as the box reads it beside the writable roots `written`: the directory
/// itself, or, when it holds one of `written`, each of its entries but that one (a directory as
/// a directory, a file as a file, the directory on the way to a writable root by its own
/// entries), since substrate refuses a read-only root that holds a writable one. A link to a
/// directory is left out: substrate refuses a root reached through a link.
fn beside(what: &'static str, dir: &Path, written: &[String], found: &mut Vec<Mount>) {
    let at = text(dir);
    if !written
        .iter()
        .any(|root| root != &at && at_or_under(root, &at))
    {
        found.push(read_only_root(what, dir));
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries
        .filter_map(|entry| entry.ok().map(|entry| entry.path()))
        .collect();
    entries.sort();
    for entry in entries {
        let at = text(&entry);
        if written.iter().any(|root| at_or_under(&at, root)) {
            continue;
        }
        let Ok(kind) = std::fs::symlink_metadata(&entry).map(|meta| meta.file_type()) else {
            continue;
        };
        if kind.is_dir() {
            beside(what, &entry, written, found);
        } else if std::fs::metadata(&entry).is_ok_and(|meta| meta.is_file()) {
            found.push(read_only_file(what, &real(&entry), &entry));
        }
    }
}

/// What a writable directory at `mount` would hand the box to write beside its own content:
/// the config file (at `config`, a path the file is reached by) or conductord's directory
/// `daemon`, which a box never writes.
fn shares(mount: &str, config: &[String], daemon: Option<&str>) -> Option<String> {
    if let Some(file) = config.iter().find(|file| at_or_under(file, mount)) {
        return Some(format!("the config file {file}"));
    }
    daemon
        .filter(|dir| at_or_under(dir, mount) || at_or_under(mount, dir))
        .map(|dir| format!("conductord's own directory {dir}"))
}

/// Everything of the host the box of `instance` sees, in one list (`story:box-runtime-starts`):
/// what `conductor instance serve` and the loop write, then what they read. The runtime is the
/// binary at `binary` reading `config_file`, on `host`.
///
/// | what | path | the box |
/// |---|---|---|
/// | `state` | the state directory (`run/lock`, `api.sock`, `sessions/`) | writes, root `state-<instance>` |
/// | `records` | the records directory | writes, root `records-<instance>` |
/// | `cache` | the cache directory (the collectors' exports, the decider's directory) | writes, root `cache-<instance>` |
/// | `store` | the directory of a sqlite store (the database and its journal) | writes, root [`STORE_ROOT`] |
/// | `binary` | the conductor binary's directory (it carries the role profiles) | reads |
/// | `harness` | the harness's directory on `PATH` | reads |
/// | `checkouts` | the checkouts root, when the trees are elsewhere | reads |
/// | `config` | the config file | reads, one file |
/// | `harness` | the file the harness's name on `PATH` links to | reads, one file |
/// | `forge-cli` | the CLI of each forge among the sources (`gh`, `glab`) | reads, one file |
/// | `forge-login` | that CLI's login files under `HOME` | reads, one file each |
///
/// Nothing else of the host is written: a writable root is a whole directory, so one that holds
/// the config file or lies with conductord's own directory ([`Host::daemon`]) is not in the
/// list, and the box is refused ([`unmountable`]). Each path is named once, as substrate takes
/// it (no trailing slash): a writable directory inside another is the outer one's, and a path
/// to read that a mount or the box's base system already holds is left out. Substrate refuses
/// a directory to read that holds a writable one, so such a directory (the checkouts root
/// around the records) is named by its other entries ([`beside`]). A program and the config
/// file appear at the path they are named by; where that path is in the box already and is a
/// link, the file it links to appears at its own path, so the link resolves. A login file is
/// named only when it is there.
#[must_use]
pub fn mounts(instance: &Instance, config_file: &Path, binary: &Path, host: &Host) -> Vec<Mount> {
    layout(instance, config_file, binary, host).0
}

/// Why no box can hold `instance` as its config and `host` lay it out, `None` when one can:
/// [`ROOT_SHARED`] for a directory to write that would hand the box the config file or
/// conductord's directory, [`STORE_NOT_ABSOLUTE`] for a sqlite store with no absolute path,
/// [`MOUNTS_EXCEEDED`] for more mounts than one start names. Refused
/// before substrate is asked ([`BoxRequest::refused`]).
#[must_use]
pub fn unmountable(
    instance: &Instance,
    config_file: &Path,
    binary: &Path,
    host: &Host,
) -> Option<Refused> {
    layout(instance, config_file, binary, host).1
}

/// The list of [`mounts`] and the refusal of [`unmountable`], from one reading of the host.
fn layout(
    instance: &Instance,
    config_file: &Path,
    binary: &Path,
    host: &Host,
) -> (Vec<Mount>, Option<Refused>) {
    let name = &instance.name.0;
    let harness = host.find(crate::confine::CLAUDE);
    let mut kept: Vec<Mount> = Vec::new();
    let mut refused: Option<Refused> = None;

    // What the runtime and the loop write, and nothing beside it.
    let config = [text(config_file), text(&real(config_file))];
    let daemon = host.daemon.as_deref().map(text);
    let store = host.store_file();
    if let Some(path) = host.store_path().filter(|_| store.is_none()) {
        refused = Some(Refused {
            name: STORE_NOT_ABSOLUTE.to_owned(),
            detail: format!(
                "the sqlite store {} has no absolute path, so the box of instance {name} can be \
                 handed no directory of it to write: write an absolute path, such as \
                 daemon.store: {}",
                path.display(),
                crate::setup::STORE
            ),
        });
    }
    let written = [
        ("state", Some(PathBuf::from(&instance.state))),
        ("records", Some(PathBuf::from(&instance.records))),
        ("cache", Some(PathBuf::from(&instance.cache))),
        (
            "store",
            store.as_deref().and_then(Path::parent).map(Path::to_owned),
        ),
    ];
    for (what, path) in written {
        let Some(path) = path else { continue };
        let root = match what {
            "store" => STORE_ROOT.to_owned(),
            _ => writable_root_name(what, name),
        };
        let mount = Mount {
            what,
            host_path: text(&path),
            mount: text(&path),
            bind: Bind::WritableRoot(root),
        };
        let Some(held) = shares(&mount.mount, &config, daemon.as_deref()) else {
            admit(&mut kept, mount);
            continue;
        };
        let dir = &mount.mount;
        let detail = match (what, store.as_deref().and_then(Path::file_name)) {
            ("store", Some(file)) => format!(
                "the directory of the sqlite store, {dir}, holds {held}, and a box writes a \
                 whole directory: put the store in a directory of its own, such as daemon.store: \
                 sqlite:{dir}/store/{}",
                Path::new(file).display()
            ),
            _ => format!(
                "the {what} directory of instance {name}, {dir}, holds {held}, and a box writes \
                 a whole directory: name a {what} directory apart from both"
            ),
        };
        refused.get_or_insert(Refused {
            name: ROOT_SHARED.to_owned(),
            detail,
        });
    }
    let roots: Vec<String> = kept.iter().map(|mount| mount.mount.clone()).collect();

    // What they read: directories.
    let mut read: Vec<(&'static str, PathBuf)> = Vec::new();
    read.extend(binary.parent().map(|dir| ("binary", dir.to_owned())));
    read.extend(
        harness
            .as_deref()
            .and_then(Path::parent)
            .map(|dir| ("harness", dir.to_owned())),
    );
    if !Path::new(&instance.checkouts.trees).starts_with(&instance.checkouts.root) {
        read.push(("checkouts", PathBuf::from(&instance.checkouts.root)));
    }
    // The first directory named by its entries, and what of the instance it holds (`records`).
    let mut split: Option<(String, String)> = None;
    for (what, dir) in read {
        let mut found = Vec::new();
        beside(what, &dir, &roots, &mut found);
        let whole = found.len() == 1 && found[0].bind == Bind::ReadOnlyRoot;
        let mut added = false;
        for mount in found {
            added |= admit(&mut kept, mount);
        }
        if added && !whole {
            let dir = text(&dir);
            let held: Vec<&str> = kept
                .iter()
                .filter(|mount| {
                    matches!(mount.bind, Bind::WritableRoot(_)) && at_or_under(&mount.mount, &dir)
                })
                .map(|mount| mount.what)
                .collect();
            split.get_or_insert((dir, held.join(" and ")));
        }
    }

    // What they read: single files.
    let mut files: Vec<(&'static str, PathBuf)> = vec![("config", config_file.to_owned())];
    files.extend(harness.map(|found| ("harness", found)));
    let mut logins: Vec<PathBuf> = Vec::new();
    for source in &instance.sources {
        let program = match source {
            Source::GitHub(_) => "gh",
            Source::GitLab(_) => "glab",
            Source::Local(_) => continue,
        };
        files.extend(host.find(program).map(|found| ("forge-cli", found)));
        for (_, paths) in LOGINS.iter().filter(|(known, _)| *known == program) {
            if let Some(home) = &host.home {
                logins.extend(paths.iter().map(|path| home.join(path)));
            }
        }
    }
    for (what, found) in files {
        let (target, found) = (real(&found), clean(&found));
        if !admit(&mut kept, read_only_file(what, &target, &found)) && target != found {
            admit(&mut kept, read_only_file(what, &target, &target));
        }
    }
    for login in logins {
        if login.is_file() {
            admit(
                &mut kept,
                read_only_file("forge-login", &real(&login), &login),
            );
        }
    }

    // What one start may name.
    let count = |bind: fn(&Bind) -> bool| kept.iter().filter(|mount| bind(&mount.bind)).count();
    let bounds = [
        (
            "directories to read",
            count(|bind| *bind == Bind::ReadOnlyRoot),
            READ_ONLY_ROOTS_BOUND,
        ),
        (
            "files to read",
            count(|bind| *bind == Bind::ReadOnlyFile),
            READ_ONLY_FILES_BOUND,
        ),
        (
            "directories to write",
            count(|bind| matches!(bind, Bind::WritableRoot(_))),
            WRITABLE_ROOTS_BOUND,
        ),
    ];
    if let Some((what, asked, bound)) = bounds.into_iter().find(|(_, asked, bound)| asked > bound) {
        let why = split.map_or_else(String::new, |(dir, held)| {
            format!(
                ": {dir} holds the {held} directory, which the box writes, so each other entry \
                 of it is named alone; put {held} outside {dir}"
            )
        });
        refused.get_or_insert(Refused {
            name: MOUNTS_EXCEEDED.to_owned(),
            detail: format!(
                "the box of instance {name} would name {asked} {what} and one start names at \
                 most {bound}{why}"
            ),
        });
    }
    (kept, refused)
}

/// The box `instance` asks for, its runtime the binary at `binary` reading `config_file`, with
/// `env` set beside nothing inherited, the runtime's log file ([`RUNTIME_LOG`]) in its writable
/// state root, and the mounts of [`mounts`] on the host `env` names ([`Host::of`]).
#[must_use]
pub fn request(
    instance: &Instance,
    config_file: &Path,
    binary: &Path,
    env: Vec<(String, String)>,
) -> BoxRequest {
    let host = Host::of(&env);
    request_on(instance, config_file, binary, env, &host)
}

/// [`request`] on `host`: conductord names its own directory there ([`Host::daemon`]).
#[must_use]
pub fn request_on(
    instance: &Instance,
    config_file: &Path,
    binary: &Path,
    mut env: Vec<(String, String)>,
    host: &Host,
) -> BoxRequest {
    let (mounts, refused) = layout(instance, config_file, binary, host);
    let state = clean(Path::new(&instance.state));
    env.push((
        crate::instance::LOG_VARIABLE.to_owned(),
        sessions_dir(&state).join(RUNTIME_LOG).display().to_string(),
    ));
    let mut read_only_roots: Vec<String> = Vec::new();
    let mut read_only_files: Vec<HostFile> = Vec::new();
    let mut writable_roots: Vec<HostRoot> = Vec::new();
    for mount in mounts {
        match mount.bind {
            Bind::ReadOnlyRoot => read_only_roots.push(mount.mount),
            Bind::ReadOnlyFile => read_only_files.push(HostFile {
                host_path: mount.host_path,
                mount: mount.mount,
            }),
            Bind::WritableRoot(name) => writable_roots.push(HostRoot {
                name,
                path: mount.mount,
            }),
        }
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
    // The daemon's token: a slot of the local daemon on a file of conductord's own directory.
    let api_token = match (&host.daemon, &instance.remote) {
        (Some(dir), None) => {
            let name = api_token_slot(&instance.name.0);
            Some(HostSlot {
                file: text(&dir.join(SLOTS_DIR).join(&name)),
                name,
                fd: API_TOKEN_FD,
            })
        }
        _ => None,
    };
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
        cwd: state.display().to_string(),
        env,
        read_only_roots,
        read_only_files,
        places: crate::config::PLACES.map(str::to_owned).to_vec(),
        limits: instance.limits.clone(),
        egress,
        secret_slots: vec![crate::secret::MODEL_TOKEN_SLOT.to_owned()],
        api_token,
        requires: resolve(&instance.placement.0)
            .map(|resolved| requires(resolved.adapter))
            .unwrap_or_default(),
        writable_roots,
        refused,
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
        write_slot_file(file, bytes)
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

    /// Whether the daemon declares the secret slot `slot` (its `secrets.slots` fact): conductord
    /// asks before it names the slot of the daemon's token in a box request
    /// (`story:box-reaches-forge-and-daemon`). A daemon that cannot be asked, and a box opener
    /// that does not say, declares none, so the box is asked for without the slot.
    fn declares(&mut self, slot: &str) -> bool {
        let _ = slot;
        false
    }

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
    fn declares(&mut self, slot: &str) -> bool {
        (**self).declares(slot)
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
        let bound = self.remote.is_some().then_some(self.timeout);
        self.block_within(bound, work)
    }

    /// [`Self::block_on`] with the bound given: `None` waits for the daemon without one.
    fn block_within<T>(
        &self,
        bound: Option<Duration>,
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
            let Some(bound) = bound else {
                return asked.await;
            };
            tokio::time::timeout(bound, asked)
                .await
                .unwrap_or_else(|_| {
                    Err(Refused {
                        name: UNREACHABLE.to_owned(),
                        detail: if self.remote.is_some() {
                            format!("the daemon did not answer within {bound:?}")
                        } else {
                            format!(
                                "the substrate daemon at {} did not answer within {} s",
                                self.socket.display(),
                                bound.as_secs()
                            )
                        },
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
        for file in &request.read_only_files {
            session = session.read_only_file(substrate_sdk::ReadOnlyFile {
                host_path: file.host_path.clone(),
                mount: file.mount.clone(),
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
        if let Some(slot) = &request.api_token {
            session = session.secret_slot(substrate_sdk::SecretSlotRequest {
                slot: slot.name.clone(),
                fd: slot.fd,
            });
        }
        let started = session.start().await?;
        Ok(started.id().to_owned())
    }
}

impl Sdk {
    /// The refusal `request` earns against the facts `machine` publishes, before anything
    /// starts: a family or a property it requires and the daemon does not serve ([`unmet`]),
    /// then a writable root the daemon does not list ([`roots_unserved`]). The slot of the
    /// daemon's token is not among them: one the daemon does not declare refuses nothing.
    fn unserved(machine: &substrate_sdk::Machine, request: &BoxRequest) -> Option<Refused> {
        unmet(
            &request.requires,
            &machine.served_families(),
            &machine.proved_properties(),
        )
        .or_else(|| {
            roots_unserved(
                &request.writable_roots,
                machine.facts.exec_writable_roots_host.as_deref(),
            )
        })
    }

    /// Whether this daemon would take each of `requests` as far as [`Substrate::open`] checks
    /// before it asks for anything: the daemon answers, and its facts serve the box
    /// ([`Self::unserved`]); the refusal of each, in their order. For a box its facts serve,
    /// the slot of the daemon's token as conductord takes it: declared, the refusal conductord
    /// would answer before it writes the slot's file ([`api_token_unsealable`], the check
    /// [`seal_api_token`] asks, so nothing is written here), else what follows from its not
    /// being declared ([`slot_undeclared`]). It asks the daemon for its machine once, whatever
    /// the requests, and starts nothing: `conductor doctor`'s dry run. The question, its
    /// connection included, ends within `bound`.
    ///
    /// # Errors
    ///
    /// [`UNREACHABLE`] when the daemon does not answer within `bound`, naming its socket and the
    /// bound.
    pub fn admits(
        &self,
        requests: &[BoxRequest],
        bound: Duration,
    ) -> Result<Vec<Admission>, Refused> {
        self.block_within(Some(bound), async move |client| {
            let machine = client.machine();
            let served = machine.facts.secrets_slots.as_deref();
            Ok(requests
                .iter()
                .map(|request| match Self::unserved(&machine, request) {
                    Some(refused) => Admission {
                        refused: Some(refused),
                        unsealed: None,
                    },
                    None => match &request.api_token {
                        Some(slot) if declared(served, &slot.name) => Admission {
                            refused: api_token_unsealable(slot),
                            unsealed: None,
                        },
                        slot => Admission {
                            refused: None,
                            unsealed: slot_undeclared(slot.as_ref(), served),
                        },
                    },
                })
                .collect())
        })
    }
}

impl Substrate for Sdk {
    fn open(&mut self, request: &BoxRequest) -> Result<Opened, Refused> {
        let request = request.clone();
        self.block_on(async move |client| {
            // The facts the start's capability snapshot carries: the same client's machine.
            if let Some(refused) = Self::unserved(&client.machine(), &request) {
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

    fn declares(&mut self, slot: &str) -> bool {
        let slot = slot.to_owned();
        self.block_on(async move |client| {
            Ok(declared(
                client.machine().facts.secrets_slots.as_deref(),
                &slot,
            ))
        })
        .unwrap_or(false)
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
            if let Some(refused) = Self::unserved(&client.machine(), &request) {
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
    let answer = match &request.refused {
        Some(why) => Err(why.clone()),
        None => substrate.open(request),
    };
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
