//! Agent kinds (`conductor.agent`, design architecture-next § 7, `story:agent-kinds`): the program
//! that fills a role slot and the way conductor starts, feeds, reads and stops it.
//!
//! [`Kind`] is the trait: `start(slot) -> Handle`, `deliver(handle, text)`, `attach(handle) ->
//! PtyStream` (interactive kinds), `reports(handle)`, `stop(handle)` and `capabilities()`. A role
//! slot ([`Slot`]) is `(role, kind, placement, profile, limits)` read from the effective settings
//! ([`slot`]); `roles.<role>.kind` defaults to `claude-code`.
//!
//! [`claude_code::ClaudeCode`] is the one kind this binary implements: today's launcher behind the
//! trait, every byte it writes unchanged (it calls `conductor session start`, `stop`, `deliver`
//! and `attach`'s own code). A run-to-exit kind is registered by a caller, as the tests do
//! ([`Runtime::register`]).
//!
//! [`Runtime`] holds the kinds and the handles of the agents it started, as the instance runtime
//! will: it refuses by the specification's error names ([`Refused`]), turns a run-to-exit kind's
//! exit into one `[REPORT]` line ([`report_line`]) written to its [`ReportSink`], and starts an
//! exit the kind classes as stopped again as a new generation. The instance API's `agents/{name}/
//! attach`, `deliver` and `restart` routes call [`Runtime::attach`], [`Runtime::deliver`] and
//! [`Runtime::restart`]; [`Runtime::routes`] names the ones an agent's capabilities offer.
//!
//! The CLI verbs `conductor agent start|deliver|attach|stop --instance <i> --role <r>` ([`cli`])
//! go through the role's kind; `conductor session start` and `session stop` are their aliases for
//! this release, and go through the same path.

pub mod claude_code;
pub mod cli;

use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use conductor_model::agent::{
    AgentExists, AgentLaunchFailed, AgentNotFound, AgentNotReachable, AgentNotRunning,
    CapabilityMissing, UnknownAgentKind,
};
pub use conductor_model::agent::{AgentKindName, AgentName, AgentState, Capability, ExitClass};
use conductor_model::config::{Instance, InstanceName, Limits, PlacementName};
use conductor_model::operator::OperatorName;

use crate::config::{self, BuiltInRole};
use crate::confine::Providers;

/// What a kind's calls run with: the global `--config` and `--state-dir`, and the confinement
/// providers a session starts and stops through.
pub struct Context<'a> {
    /// The global `--config`.
    pub config: Option<&'a Path>,
    /// The global `--state-dir`.
    pub state: Option<&'a Path>,
    /// The providers `claude-code` starts and stops its sessions through.
    pub providers: &'a dyn Providers,
}

/// A role slot (§ 7): `(role, kind, placement, profile, limits)`, and the agent's name and
/// instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    /// The agent's name inside its instance: the role's session name.
    pub name: AgentName,
    /// The instance the slot belongs to.
    pub instance: InstanceName,
    /// The role, as the config writes it.
    pub role: String,
    /// The kind that fills the slot.
    pub kind: AgentKindName,
    /// The placement the agent runs at: the instance's.
    pub placement: PlacementName,
    /// The role's profile, appended to the harness; none where the role names none.
    pub profile: Option<String>,
    /// The role's limits.
    pub limits: Limits,
}

/// What a kind's `start` answers: the agent it started, and the kind's own id for it (a session
/// id, a process id), when it has one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Handle {
    /// The agent.
    pub name: AgentName,
    /// The kind that started it.
    pub kind: AgentKindName,
    /// The kind's own id of what it started.
    pub id: Option<String>,
}

/// An interactive agent's PTY, as `attach` answers it: for `claude-code`, the session's tmux
/// server on its launcher socket. The instance API's `agents/{name}/attach` bridges it to a
/// WebSocket.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyStream {
    /// A tmux session on a socket.
    Tmux {
        /// The launcher socket, `<state>/sessions/<name>.tmux`.
        socket: PathBuf,
        /// The tmux session's name.
        session: String,
        /// Read-only.
        watch: bool,
    },
}

/// What a kind's `reports` answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Report {
    /// One line of the report vocabulary (`[REPORT]`, `[NEED]`, `[DECISION-REQUEST]`) the agent
    /// wrote.
    Line(String),
    /// A run-to-exit program ended.
    Exited {
        /// Its exit code.
        code: i32,
        /// The events file it wrote.
        events: Option<PathBuf>,
        /// Whether the kind classes the exit as stopped: resumable, so a new generation.
        stopped: bool,
    },
}

/// A route of the instance API's `agents/{name}/*` an agent offers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    /// `agents/{name}/attach`, interactive kinds only.
    Attach,
    /// `agents/{name}/deliver`, interactive kinds only.
    Deliver,
    /// `agents/{name}/restart`, every kind.
    Restart,
}

/// The routes `capabilities` offer: attach and deliver for an interactive kind, restart for
/// every kind (how a run-to-exit kind takes a message, § 7).
#[must_use]
pub fn routes(capabilities: &[Capability]) -> Vec<Route> {
    let mut routes = Vec::new();
    if capabilities.contains(&Capability::Interactive) {
        routes.extend([Route::Attach, Route::Deliver]);
    }
    routes.push(Route::Restart);
    routes
}

/// A refusal, by the specification's error name (`conductor.agent`), or a failure of the kind
/// itself, whose message is the kind's own (for `claude-code`, today's `session` refusals, byte
/// for byte).
#[derive(Debug)]
pub enum Refused {
    /// The binary implements no such kind.
    UnknownAgentKind(UnknownAgentKind),
    /// An agent of that name is on record.
    AgentExists(AgentExists),
    /// The kind's start failed.
    AgentLaunchFailed(AgentLaunchFailed, String),
    /// No agent of that name is on record.
    AgentNotFound(AgentNotFound),
    /// The agent is stopped.
    AgentNotRunning(AgentNotRunning),
    /// The agent's PTY does not answer.
    AgentNotReachable(AgentNotReachable, String),
    /// The agent's kind lacks the capability.
    CapabilityMissing(CapabilityMissing),
    /// The kind failed, with its own message.
    Failed(anyhow::Error),
}

impl Refused {
    /// `AgentLaunchFailed` for `name`, with why.
    #[must_use]
    pub fn launch_failed(name: &AgentName, detail: String) -> Self {
        Self::AgentLaunchFailed(AgentLaunchFailed { name: name.clone() }, detail)
    }

    /// `CapabilityMissing` for `name`.
    #[must_use]
    pub fn capability_missing(name: &AgentName, capability: Capability) -> Self {
        Self::CapabilityMissing(CapabilityMissing {
            name: name.clone(),
            capability,
        })
    }

    /// `UnknownAgentKind` for `kind`.
    #[must_use]
    pub fn unknown_kind(kind: &AgentKindName) -> Self {
        Self::UnknownAgentKind(UnknownAgentKind { kind: kind.clone() })
    }
}

impl fmt::Display for Refused {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownAgentKind(error) => write!(
                f,
                "refused: UnknownAgentKind: kind {:?}: this binary implements {}; nothing was \
                 started",
                error.kind.0,
                claude_code::NAME
            ),
            Self::AgentExists(error) => write!(
                f,
                "refused: AgentExists: agent {}: it is on record; restart it instead",
                error.name.0
            ),
            Self::AgentLaunchFailed(error, detail) => write!(
                f,
                "refused: AgentLaunchFailed: agent {}: {detail}",
                error.name.0
            ),
            Self::AgentNotFound(error) => write!(
                f,
                "refused: AgentNotFound: agent {}: no agent of that name is on record",
                error.name.0
            ),
            Self::AgentNotRunning(error) => write!(
                f,
                "refused: AgentNotRunning: agent {}: it is stopped; nothing reached it",
                error.name.0
            ),
            Self::AgentNotReachable(error, detail) => write!(
                f,
                "refused: AgentNotReachable: agent {}: {detail}",
                error.name.0
            ),
            Self::CapabilityMissing(error) => write!(
                f,
                "refused: CapabilityMissing: agent {}: its kind lacks {:?}; nothing was done",
                error.name.0, error.capability
            ),
            Self::Failed(error) => write!(f, "{error:#}"),
        }
    }
}

impl std::error::Error for Refused {}

impl From<anyhow::Error> for Refused {
    fn from(error: anyhow::Error) -> Self {
        Self::Failed(error)
    }
}

/// An agent kind (§ 7).
pub trait Kind: Send + Sync {
    /// The kind's name, as `roles.<role>.kind` writes it.
    fn name(&self) -> AgentKindName;

    /// The program the kind starts, as `conductor doctor` looks it up.
    fn binary(&self) -> String;

    /// What the kind can do.
    fn capabilities(&self) -> Vec<Capability>;

    /// Starts the slot's agent.
    ///
    /// # Errors
    ///
    /// The kind's refusal.
    fn start(&self, context: &Context<'_>, slot: &Slot) -> Result<Handle, Refused>;

    /// Puts `text` in front of the agent; an interactive kind's only.
    ///
    /// # Errors
    ///
    /// `CapabilityMissing` unless the kind overrides it, else the kind's refusal.
    fn deliver(&self, _context: &Context<'_>, handle: &Handle, _text: &str) -> Result<(), Refused> {
        Err(Refused::capability_missing(
            &handle.name,
            Capability::Interactive,
        ))
    }

    /// The agent's PTY; an interactive kind's only.
    ///
    /// # Errors
    ///
    /// `CapabilityMissing` unless the kind overrides it, else the kind's refusal.
    fn attach(
        &self,
        _context: &Context<'_>,
        handle: &Handle,
        _watch: bool,
    ) -> Result<PtyStream, Refused> {
        Err(Refused::capability_missing(
            &handle.name,
            Capability::Interactive,
        ))
    }

    /// What the agent reported since the last call: lines of the report vocabulary, and a
    /// run-to-exit program's exit.
    ///
    /// # Errors
    ///
    /// The kind cannot read the agent.
    fn reports(&self, context: &Context<'_>, handle: &Handle) -> Result<Vec<Report>, Refused>;

    /// Stops the agent.
    ///
    /// # Errors
    ///
    /// The kind's refusal.
    fn stop(&self, context: &Context<'_>, handle: &Handle) -> Result<(), Refused>;
}

/// The kinds this binary implements: `claude-code`.
#[must_use]
pub fn built_in() -> Vec<Box<dyn Kind>> {
    vec![Box::new(claude_code::ClaudeCode)]
}

/// The built-in kind named `name`, if this binary implements one.
#[must_use]
pub fn built_in_kind(name: &str) -> Option<Box<dyn Kind>> {
    built_in().into_iter().find(|kind| kind.name().0 == name)
}

/// The role `role` of `instance`, under its name or its old one (`director` or `conductor`,
/// `maintainer` or `conductor-dev`); the maintainer is the host's role
/// ([`config::host_maintainer`]).
#[must_use]
pub fn find_role(instance: &Instance, role: &str) -> Option<conductor_model::config::Role> {
    match config::role_name(role) {
        Some(BuiltInRole::Maintainer) => config::host_maintainer(instance),
        Some(wanted) => instance
            .roles
            .iter()
            .find(|named| config::is_role(&named.role, wanted))
            .cloned(),
        None => instance
            .roles
            .iter()
            .find(|named| named.role == role)
            .cloned(),
    }
}

/// The slot of the role `role` of `instance`, from the effective settings: its session name, its
/// kind (`claude-code` by default), the instance's placement, its profile and its limits.
///
/// # Errors
///
/// The instance names no such role.
pub fn slot(instance: &Instance, role: &str) -> Result<Slot, String> {
    let found = find_role(instance, role)
        .ok_or_else(|| format!("the instance names no role {role} in conductor config show"))?;
    let name = found
        .session_name
        .clone()
        .unwrap_or_else(|| config::session_name(instance, &found.role));
    Ok(Slot {
        name: AgentName(name.0),
        instance: instance.name.clone(),
        role: found.role.clone(),
        kind: found.kind.clone(),
        placement: instance.placement.clone(),
        profile: found.profile.as_ref().map(|profile| profile.0.clone()),
        limits: found.limits.clone(),
    })
}

/// Where the runtime writes the report lines it makes: the instance runtime's mailbox to the
/// controller, a test's list.
pub trait ReportSink: Send {
    /// Writes `line`, reported by `agent`.
    ///
    /// # Errors
    ///
    /// The line could not be written.
    fn write(&mut self, agent: &AgentName, line: &str) -> Result<()>;
}

impl<F> ReportSink for F
where
    F: FnMut(&AgentName, &str) -> Result<()> + Send,
{
    fn write(&mut self, agent: &AgentName, line: &str) -> Result<()> {
        self(agent, line)
    }
}

/// An agent the runtime started: its slot, its handle and where its lifecycle is
/// (`conductor.agent.Agent`).
#[derive(Debug, Clone)]
pub struct Agent {
    /// The slot it fills.
    pub slot: Slot,
    /// The kind's capabilities at the start.
    pub capabilities: Vec<Capability>,
    /// The kind's handle of the current generation; none once stopped.
    pub handle: Option<Handle>,
    /// `Starting`, `Running` or `Stopped`.
    pub state: AgentState,
    /// The starts of the slot: 1, one more at each restart.
    pub generation: i64,
    /// Why it stopped.
    pub exit_class: Option<ExitClass>,
    /// A run-to-exit program's exit code.
    pub exit_code: Option<i64>,
    /// The owner of the pooled token the agent runs on (`story:token-pool`); none for an agent on
    /// no pooled token.
    pub token_owner: Option<OperatorName>,
}

/// The kinds and the agents of one instance runtime.
pub struct Runtime {
    kinds: Vec<Box<dyn Kind>>,
    agents: BTreeMap<String, Agent>,
    sink: Box<dyn ReportSink>,
}

impl Runtime {
    /// A runtime with the built-in kinds ([`built_in`]) writing its report lines to `sink`.
    #[must_use]
    pub fn new(sink: Box<dyn ReportSink>) -> Self {
        Self {
            kinds: built_in(),
            agents: BTreeMap::new(),
            sink,
        }
    }

    /// Adds `kind`, replacing a kind of its name.
    pub fn register(&mut self, kind: Box<dyn Kind>) {
        self.kinds.retain(|known| known.name() != kind.name());
        self.kinds.push(kind);
    }

    /// The kind named `name`.
    #[must_use]
    pub fn kind(&self, name: &str) -> Option<&dyn Kind> {
        self.kinds
            .iter()
            .find(|kind| kind.name().0 == name)
            .map(AsRef::as_ref)
    }

    /// The agent named `name`.
    #[must_use]
    pub fn agent(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    /// Every agent, by name.
    pub fn agents(&self) -> impl Iterator<Item = &Agent> {
        self.agents.values()
    }

    fn known(&self, name: &str) -> Result<&Agent, Refused> {
        self.agents.get(name).ok_or_else(|| {
            Refused::AgentNotFound(AgentNotFound {
                name: AgentName(name.to_owned()),
            })
        })
    }

    fn kind_of(&self, agent: &Agent) -> Result<&dyn Kind, Refused> {
        self.kind(&agent.slot.kind.0)
            .ok_or_else(|| Refused::unknown_kind(&agent.slot.kind))
    }

    /// `StartAgent`: refuses a kind it does not hold and a name on record; else the agent is
    /// `Starting` while the kind starts it and `Running` once the kind answered its handle
    /// (`MarkAgentRunning`); a kind that fails leaves nothing on record.
    ///
    /// # Errors
    ///
    /// `UnknownAgentKind`, `AgentExists`, or the kind's refusal.
    pub fn start(&mut self, context: &Context<'_>, slot: Slot) -> Result<&Agent, Refused> {
        let kind = self
            .kind(&slot.kind.0)
            .ok_or_else(|| Refused::unknown_kind(&slot.kind))?;
        if self.agents.contains_key(&slot.name.0) {
            return Err(Refused::AgentExists(AgentExists {
                name: slot.name.clone(),
            }));
        }
        let capabilities = kind.capabilities();
        let handle = kind.start(context, &slot)?;
        let name = slot.name.0.clone();
        self.agents.insert(
            name.clone(),
            Agent {
                slot,
                capabilities,
                handle: Some(handle),
                state: AgentState::Running,
                generation: 1,
                exit_class: None,
                exit_code: None,
                token_owner: crate::token::owner_of(&AgentName(name.clone())),
            },
        );
        Ok(&self.agents[&name])
    }

    /// The routes the agent `name` offers ([`routes`]).
    ///
    /// # Errors
    ///
    /// `AgentNotFound`.
    pub fn routes(&self, name: &str) -> Result<Vec<Route>, Refused> {
        Ok(routes(&self.known(name)?.capabilities))
    }

    fn interactive(&self, name: &str) -> Result<(&Agent, &Handle), Refused> {
        let agent = self.known(name)?;
        if !agent.capabilities.contains(&Capability::Interactive) {
            return Err(Refused::capability_missing(
                &agent.slot.name,
                Capability::Interactive,
            ));
        }
        match (&agent.handle, agent.state) {
            (Some(handle), AgentState::Starting | AgentState::Running) => Ok((agent, handle)),
            _ => Err(Refused::AgentNotRunning(AgentNotRunning {
                name: agent.slot.name.clone(),
            })),
        }
    }

    /// `DeliverToAgent`, the `agents/{name}/deliver` route.
    ///
    /// # Errors
    ///
    /// `AgentNotFound`, `CapabilityMissing`, `AgentNotRunning`, or the kind's refusal.
    pub fn deliver(&self, context: &Context<'_>, name: &str, text: &str) -> Result<(), Refused> {
        let (agent, handle) = self.interactive(name)?;
        self.kind_of(agent)?.deliver(context, handle, text)
    }

    /// `AttachAgent`, the `agents/{name}/attach` route.
    ///
    /// # Errors
    ///
    /// `AgentNotFound`, `CapabilityMissing`, `AgentNotRunning`, or the kind's refusal.
    pub fn attach(
        &self,
        context: &Context<'_>,
        name: &str,
        watch: bool,
    ) -> Result<PtyStream, Refused> {
        let (agent, handle) = self.interactive(name)?;
        self.kind_of(agent)?.attach(context, handle, watch)
    }

    /// `StopAgent` with the class `Requested`.
    ///
    /// # Errors
    ///
    /// `AgentNotFound`, `AgentNotRunning`, or the kind's refusal.
    pub fn stop(&mut self, context: &Context<'_>, name: &str) -> Result<&Agent, Refused> {
        let agent = self.known(name)?;
        let Some(handle) = agent
            .handle
            .clone()
            .filter(|_| agent.state != AgentState::Stopped)
        else {
            return Err(Refused::AgentNotRunning(AgentNotRunning {
                name: agent.slot.name.clone(),
            }));
        };
        self.kind_of(agent)?.stop(context, &handle)?;
        let agent = self.agents.get_mut(name).expect("known above");
        agent.state = AgentState::Stopped;
        agent.handle = None;
        agent.exit_class = Some(ExitClass::Requested);
        agent.exit_code = None;
        Ok(agent)
    }

    /// `RestartAgent`, the `agents/{name}/restart` route: a running agent is stopped by its kind
    /// first, then the slot starts again as its next generation, its stop cleared.
    ///
    /// # Errors
    ///
    /// `AgentNotFound`, or the kind's refusal; a failed start leaves the record as it was.
    pub fn restart(&mut self, context: &Context<'_>, name: &str) -> Result<&Agent, Refused> {
        let agent = self.known(name)?;
        let kind = self.kind_of(agent)?;
        if let Some(handle) = agent
            .handle
            .as_ref()
            .filter(|_| agent.state != AgentState::Stopped)
        {
            kind.stop(context, handle)?;
        }
        let handle = kind.start(context, &agent.slot)?;
        let agent = self.agents.get_mut(name).expect("known above");
        agent.handle = Some(handle);
        agent.state = AgentState::Running;
        agent.generation += 1;
        agent.exit_class = None;
        agent.exit_code = None;
        agent.token_owner = crate::token::owner_of(&agent.slot.name);
        Ok(agent)
    }

    /// Reads every running agent's reports: a line is written to the sink as it is; a
    /// run-to-exit exit is written as one `[REPORT]` line ([`report_line`]), the agent stopped as
    /// `Exited` with its code, and started again as a new generation when the kind classes the
    /// exit as stopped. Answers the lines written, by agent.
    ///
    /// # Errors
    ///
    /// A kind cannot read its agent, a restart is refused, or the sink fails.
    pub fn poll(&mut self, context: &Context<'_>) -> Result<Vec<(AgentName, String)>, Refused> {
        let mut written = Vec::new();
        let names: Vec<String> = self.agents.keys().cloned().collect();
        for name in names {
            let agent = &self.agents[&name];
            let Some(handle) = agent.handle.clone() else {
                continue;
            };
            let reports = self.kind_of(agent)?.reports(context, &handle)?;
            for report in reports {
                match report {
                    Report::Line(line) => {
                        self.sink.write(&handle.name, &line)?;
                        written.push((handle.name.clone(), line));
                    }
                    Report::Exited {
                        code,
                        events,
                        stopped,
                    } => {
                        let line = report_line(&handle.name, code, events.as_deref(), stopped);
                        self.sink.write(&handle.name, &line)?;
                        written.push((handle.name.clone(), line));
                        let agent = self.agents.get_mut(&name).expect("listed above");
                        agent.state = AgentState::Stopped;
                        agent.handle = None;
                        agent.exit_class = Some(ExitClass::Exited);
                        agent.exit_code = Some(i64::from(code));
                        if stopped {
                            self.restart(context, &name)?;
                        }
                    }
                }
            }
        }
        Ok(written)
    }
}

/// The one `[REPORT]` line a run-to-exit exit becomes: `[REPORT <agent> -] <verb>: exit <code>,
/// <n> events in <file>[: <last event's summary>]`, the verb `done` for 0, `blocked` for an exit
/// of the stopped class and `failed` otherwise.
#[must_use]
pub fn report_line(agent: &AgentName, code: i32, events: Option<&Path>, stopped: bool) -> String {
    let verb = match (code, stopped) {
        (0, _) => "done",
        (_, true) => "blocked",
        _ => "failed",
    };
    let mut line = format!("[REPORT {} -] {verb}: exit {code}", agent.0);
    if let Some(file) = events {
        let text = fs::read_to_string(file).unwrap_or_default();
        let lines: Vec<&str> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .collect();
        line.push_str(&format!(", {} events in {}", lines.len(), file.display()));
        let summary = lines
            .last()
            .and_then(|last| serde_json::from_str::<serde_json::Value>(last).ok())
            .and_then(|event| {
                ["summary", "message"]
                    .iter()
                    .find_map(|key| event[key].as_str().map(str::to_owned))
            });
        if let Some(summary) = summary {
            let summary = summary.split_whitespace().collect::<Vec<_>>().join(" ");
            if !summary.is_empty() {
                line.push_str(&format!(": {summary}"));
            }
        }
    }
    line
}
