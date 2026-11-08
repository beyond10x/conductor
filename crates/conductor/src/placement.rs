//! Placements (`story:placement-substrate-local`, design architecture-next § 6): where an
//! instance runs, a substrate daemon plus an adapter plus a profile. Conductor implements no
//! sandbox: it asks substrate for the box.
//!
//! A `conductor.config/2` instance writes `placement` as a map `{daemon, adapter}`, or as a bare
//! string read as `{daemon: local, adapter: <string>}` (decision P3) ([`read`]). `daemon` is
//! `local` (the default) or an `https://` URL; `adapter` is `systemd`, `linux`, `oci` or `pod`.
//! A local daemon refuses `oci` and `pod` at validate (decision P5): substrate 0.16.1's daemon
//! serves the host driver (`linux`) and a cluster driver only, and takes no adapter in a request.
//! The instance's `placement` name is the canonical text: the adapter for a local daemon,
//! `<adapter>@<url>` for a remote one ([`resolve`]). A bare string that names no adapter keeps
//! the path it had before substrate placements (conductor's own `systemd-run` units) and
//! resolves to nothing.
//!
//! Only `adapter: linux` on a local daemon goes through substrate today ([`boxed`]): one
//! substrate workspace per instance, the box, and the runtime (`conductor instance serve`) a
//! substrate PTY session in it, opened through the SDK against `substrate-daemon`'s socket
//! ([`Sdk`]). `adapter: systemd` stays on conductor's `systemd-run` path until substrate takes
//! the adapter on the wire (acceptance line 4 is deferred: `confine/systemd.rs` is kept).
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
};
use conductor_model::primitives::Uuid;
use serde_json::Value;

use crate::store::Store;

/// The daemon a bare placement and a map without `daemon` name.
pub const LOCAL: &str = "local";

/// The model API host every box may reach besides its forges (§ 6).
pub const MODEL_HOST: &str = "api.anthropic.com";

/// The variable naming the local substrate daemon's socket; [`default_socket`] without it.
pub const SOCKET_VARIABLE: &str = "CONDUCTOR_SUBSTRATE_SOCKET";

/// The egress aperture a box's sessions are started with. Substrate 0.16.1 declares apertures on
/// the daemon (`--egress-aperture`), not per request: the operator declares this one with the
/// forge and model hosts of [`BoxRequest::egress`].
pub const EGRESS_APERTURE: &str = "conductor";

/// The descriptor the model token's sealed slot arrives at inside the box.
pub const SLOT_FD: u32 = 3;

/// The name of the refusal when the daemon cannot be reached at all: conductor's, not
/// substrate's, since substrate said nothing.
pub const UNREACHABLE: &str = "placement.substrate-unreachable";

/// The name of the refusal of a placement this build does not open: a remote daemon (wave 6).
pub const UNSERVED: &str = "placement.daemon-unserved";

/// The adapters by the word the config writes.
const ADAPTERS: [(&str, Adapter); 4] = [
    ("systemd", Adapter::Systemd),
    ("linux", Adapter::Linux),
    ("oci", Adapter::Oci),
    ("pod", Adapter::Pod),
];

/// The keys of a placement map.
const KEYS: [&str; 2] = ["daemon", "adapter"];

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

/// Whether `instance` runs in a substrate box: any resolved placement but `systemd` on a local
/// daemon, which stays on conductor's own units.
#[must_use]
pub fn boxed(instance: &Instance) -> bool {
    resolve(&instance.placement.0).is_some_and(|resolved| {
        !(resolved.daemon == Where::Local && resolved.adapter == Adapter::Systemd)
    })
}

/// Reads an instance's `placement` at `path` (decisions P3, P5): its canonical name, or each
/// problem as its YAML path and what is wrong there.
///
/// # Errors
///
/// Each problem of the value, by path.
pub fn read(value: &Value, path: &str) -> Result<String, Vec<(String, String)>> {
    let local_refusal = |at: String, word: &str| {
        (
            at,
            format!(
                "`{word}` is refused on a local daemon: a local substrate daemon serves `linux`, \
                 and `systemd` runs on conductor's own units"
            ),
        )
    };
    match value {
        Value::String(text) if text.trim().is_empty() => {
            Err(vec![(path.to_owned(), "is empty".to_owned())])
        }
        Value::String(text) => match adapter_named(text) {
            Some(adapter @ (Adapter::Oci | Adapter::Pod)) => {
                Err(vec![local_refusal(path.to_owned(), adapter_word(adapter))])
            }
            _ => Ok(text.clone()),
        },
        Value::Object(map) => {
            let mut problems = Vec::new();
            for key in map.keys().filter(|key| !KEYS.contains(&key.as_str())) {
                problems.push((
                    format!("{path}.{key}"),
                    format!("`{key}` is no key of a placement: write daemon and adapter"),
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
            match (daemon, adapter) {
                (Some(daemon), Some(adapter)) if problems.is_empty() => {
                    Ok(name_of(&daemon, adapter))
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
    /// Host directories mounted read-only at their own path: the conductor binary's directory,
    /// the profiles, the harness's directory, the checkouts root when the trees are elsewhere.
    pub read_only_roots: Vec<String>,
    /// The six writable places, each a subdirectory of the one workspace.
    pub places: Vec<String>,
    /// The instance's limits.
    pub limits: Limits,
    /// The only hosts the box may reach: the forges of its sources and [`MODEL_HOST`].
    pub egress: Vec<String>,
    /// The sealed slots the box receives: the model token (`story:secrets-source`).
    pub secret_slots: Vec<String>,
}

/// The box `instance` asks for, its runtime the binary at `binary` reading `config_file`, with
/// `env` set beside nothing inherited.
#[must_use]
pub fn request(
    instance: &Instance,
    config_file: &Path,
    binary: &Path,
    env: Vec<(String, String)>,
) -> BoxRequest {
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
    add(std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".b10x/conductor/profiles")));
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

/// The local substrate daemon over its Unix socket, through the substrate SDK.
#[derive(Debug, Clone)]
pub struct Sdk {
    /// The daemon's socket.
    pub socket: PathBuf,
}

impl Default for Sdk {
    fn default() -> Self {
        Self {
            socket: default_socket(),
        }
    }
}

/// How long a box's lease and its runtime's run last before substrate ends them: substrate
/// 0.16.1 bounds one exec at 24 hours (`MAX_EXEC_DURATION`).
const RUN_BOUND: Duration = substrate_sdk::MAX_EXEC_DURATION;

/// The memory a box gets when the instance sets none: 8 GiB.
const DEFAULT_MEMORY: u64 = 8 << 30;

/// Each output stream's bound.
const OUTPUT_BYTES: u64 = 64 << 20;

fn refused(error: substrate_sdk::SdkError) -> Refused {
    match error {
        substrate_sdk::SdkError::Refusal(refusal) => Refused {
            name: refusal.code,
            detail: refusal.message,
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
        runtime.block_on(async {
            let client = substrate_sdk::Client::builder()
                .unix_socket(&self.socket)
                .connect()
                .await
                .map_err(refused)?;
            work(client).await.map_err(refused)
        })
    }
}

impl Substrate for Sdk {
    fn open(&mut self, request: &BoxRequest) -> Result<Opened, Refused> {
        let request = request.clone();
        self.block_on(async move |client| {
            let workspace = client
                .workspace()
                .label("conductor.instance", request.instance.0.clone())
                .label("conductor.placement", request.placement.0.clone())
                .lease(RUN_BOUND)
                .create()
                .await?;
            let memory = request
                .limits
                .memory_max
                .as_ref()
                .map_or(DEFAULT_MEMORY, |size| {
                    u64::try_from(size.0).unwrap_or(0) << 30
                });
            let processes = request
                .limits
                .tasks_max
                .map_or(substrate_sdk::MAX_EXEC_PROCESSES, |tasks| {
                    u32::try_from(tasks).unwrap_or(substrate_sdk::MAX_EXEC_PROCESSES)
                });
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
            for slot in &request.secret_slots {
                session = session.secret_slot(substrate_sdk::SecretSlotRequest {
                    slot: slot.clone(),
                    fd: SLOT_FD,
                });
            }
            let started = session.start().await?;
            Ok(Opened {
                workspace: workspace.id().to_owned(),
                session: started.id().to_owned(),
            })
        })
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
}

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
    let answer = match resolve(&request.placement.0).map(|resolved| resolved.daemon) {
        Some(Where::Remote(url)) => Err(Refused {
            name: UNSERVED.to_owned(),
            detail: format!("a remote substrate daemon ({url}) is not opened by this build"),
        }),
        _ => substrate.open(request),
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
