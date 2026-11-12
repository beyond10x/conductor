// generated from conductor v1
// contract digest 33c96138ad3173997519bd2cc8cd9c77cc04d0021627fb06de8e7bab55e87239
// do not edit: regenerate with `ess synthesize --layout crate`

//! Agent — `conductor.agent`.
//!
//! An agent kind fills any role at any placement (design docs/design/architecture-next.md § 7, decision 3). A kind is the program that fills a role slot and the way conductor starts, feeds, reads and stops it; `claude-code` is the only kind in v1, with today's session behaviour kept exactly. A kind's capabilities are `Interactive` (a PTY to attach to, a mailbox the runtime delivers into, a hand-over at a context threshold, a terminal log) or `RunToExit` (one unit of work in; an exit code and an events file out). An agent is a role slot filled, `(role, kind, placement, profile, limits)`, started by the instance runtime and named by its instance and its name; the instance API's `agents/{name}/attach`, `deliver` and `restart` routes (§ 5.1) are the kind's trait over HTTP. `conductor.session` stays the `claude-code` kind's behaviour: its scenarios and tests are the pin, and nothing here replaces them.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `conductor.agent.Agent`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Agent<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentState {
    /// `Running`.
    Running,
    /// `Starting`.
    Starting,
    /// `Stopped`.
    Stopped,
}

/// The states of `conductor.agent.AgentKind`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `AgentKind<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentKindState {
    /// `Available`.
    Available,
}

/// AgentKindName — `conductor.agent.AgentKindName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKindName(pub String);

/// AgentName — `conductor.agent.AgentName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentName(pub String);

/// AgentSlotSelection — `conductor.agent.AgentSlotSelection`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSlotSelection {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
    /// `role` — `String`.
    pub role: String,
}

/// AgentTerminalAttach — `conductor.agent.AgentTerminalAttach`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTerminalAttach {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
    /// `role` — `String`.
    pub role: String,
    /// `watch` — `Optional<Boolean>`.
    pub watch: Option<bool>,
}

/// AgentTextDelivery — `conductor.agent.AgentTextDelivery`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentTextDelivery {
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<crate::config::InstanceName>,
    /// `role` — `String`.
    pub role: String,
    /// `text` — `String`.
    pub text: String,
}

/// Capability — `conductor.agent.Capability`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    /// `Interactive`.
    Interactive,
    /// `RunToExit`.
    RunToExit,
}

/// ExitClass — `conductor.agent.ExitClass`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitClass {
    /// `Requested`.
    Requested,
    /// `UsageLimit`.
    UsageLimit,
    /// `Exited`.
    Exited,
}

/// ShowRole — `conductor.agent.ShowRole`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowRole {
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
}

/// What Agent — `conductor.agent.Agent` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Agent<S>`], and at a boundary by [`AgentSnapshot::state`].
///
/// Every value satisfies `generation >= 1` — checked by [`AgentData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentData {
    /// The identity: `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `instance`: `conductor.agent.Agent` references one `conductor.instance.Instance`.
    pub instance: crate::config::InstanceName,
    /// `role` — `conductor.config.RoleName`.
    pub role: crate::config::RoleName,
    /// `kind` — `conductor.agent.AgentKindName`.
    ///
    /// Carries `agent_kind`: `conductor.agent.Agent` references one `conductor.agent.AgentKind`.
    pub kind: AgentKindName,
    /// `capabilities` — `List<conductor.agent.Capability>`.
    pub capabilities: Vec<Capability>,
    /// `box` — `Optional<conductor.placement.BoxId>`.
    ///
    /// Carries `box`: `conductor.agent.Agent` references one `conductor.placement.Box`.
    pub r#box: Option<crate::placement::BoxId>,
    /// `profile` — `Optional<String>`.
    pub profile: Option<String>,
    /// `limits` — `conductor.config.Limits`.
    pub limits: crate::config::Limits,
    /// `token_owner` — `Optional<conductor.operator.OperatorName>`.
    pub token_owner: Option<crate::operator::OperatorName>,
    /// `started_by` — `conductor.operator.Actor`.
    pub started_by: crate::operator::Actor,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `generation` — `Integer`.
    pub generation: i64,
    /// `stopped_at` — `Optional<Timestamp>`.
    pub stopped_at: Option<crate::primitives::Timestamp>,
    /// `exit_class` — `Optional<conductor.agent.ExitClass>`.
    pub exit_class: Option<ExitClass>,
    /// `exit_code` — `Optional<Integer>`.
    pub exit_code: Option<i64>,
}

impl AgentData {
    /// The first declared invariant of `conductor.agent.Agent` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.generation)), iv::Op::Ge, iv::Fact::number("1"), false, true)) {
            return Some("generation >= 1");
        }
        None
    }
}

/// The states of `conductor.agent.Agent`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](agent_state::Marker), so [`Agent<S>`](Agent) can only ever rest in a real state.
pub mod agent_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Running {}
        impl Sealed for super::Starting {}
        impl Sealed for super::Stopped {}
    }

    /// A declared state of `Agent`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::AgentState;
    }

    /// `Running`.
    pub struct Running;

    impl Marker for Running {
        const STATE: super::AgentState = super::AgentState::Running;
    }

    /// `Starting`. Where a new instance starts.
    pub struct Starting;

    impl Marker for Starting {
        const STATE: super::AgentState = super::AgentState::Starting;
    }

    /// `Stopped`.
    pub struct Stopped;

    impl Marker for Stopped {
        const STATE: super::AgentState = super::AgentState::Stopped;
    }
}

/// Agent — `conductor.agent.Agent` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Starting`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`AgentSnapshot`]
/// and [`AgentSnapshot::refine`].
pub struct Agent<S: agent_state::Marker> {
    data: AgentData,
    state: core::marker::PhantomData<S>,
}

impl<S: agent_state::Marker> Agent<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> AgentState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &AgentData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> AgentData {
        self.data
    }
}

impl Agent<agent_state::Starting> {
    /// A new instance, resting in `Starting` — the only state the lifecycle starts one in.
    pub fn new(data: AgentData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Agent<agent_state::Running> {
    /// `stop` — `Running` → `Stopped`. Taken by the `stopped` outcome of `conductor.agent.StopAgent`.
    pub fn stop(self) -> Agent<agent_state::Stopped> {
        Agent {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `restart` — `Running` → `Starting`. Taken by the `restarted` outcome of `conductor.agent.RestartAgent`.
    pub fn restart(self) -> Agent<agent_state::Starting> {
        Agent {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Agent<agent_state::Starting> {
    /// `run` — `Starting` → `Running`. Taken by the `running` outcome of `conductor.agent.MarkAgentRunning`.
    pub fn run(self) -> Agent<agent_state::Running> {
        Agent {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stop` — `Starting` → `Stopped`. Taken by the `stopped` outcome of `conductor.agent.StopAgent`.
    pub fn stop(self) -> Agent<agent_state::Stopped> {
        Agent {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `restart` — `Starting` → `Starting`. Taken by the `restarted` outcome of `conductor.agent.RestartAgent`.
    pub fn restart(self) -> Agent<agent_state::Starting> {
        Agent {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Agent<agent_state::Stopped> {
    /// `restart` — `Stopped` → `Starting`. Taken by the `restarted` outcome of `conductor.agent.RestartAgent`.
    pub fn restart(self) -> Agent<agent_state::Starting> {
        Agent {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.agent.Agent` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`AgentSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: AgentState,
    /// What it holds.
    pub data: AgentData,
}

/// An `Agent` in whichever declared state it was found.
pub enum AnyAgent {
    /// Resting in `Running`.
    Running(Agent<agent_state::Running>),
    /// Resting in `Starting`.
    Starting(Agent<agent_state::Starting>),
    /// Resting in `Stopped`.
    Stopped(Agent<agent_state::Stopped>),
}

impl AgentSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `AgentState` cannot spell one.
    pub fn refine(self) -> AnyAgent {
        match self.state {
            AgentState::Running => AnyAgent::Running(Agent {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            AgentState::Starting => AnyAgent::Starting(Agent {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            AgentState::Stopped => AnyAgent::Stopped(Agent {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyAgent {
    /// The state, as the runtime value.
    pub fn state(&self) -> AgentState {
        match self {
            Self::Running(_) => AgentState::Running,
            Self::Starting(_) => AgentState::Starting,
            Self::Stopped(_) => AgentState::Stopped,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> AgentSnapshot {
        match self {
            Self::Running(instance) => AgentSnapshot {
                state: AgentState::Running,
                data: instance.into_data(),
            },
            Self::Starting(instance) => AgentSnapshot {
                state: AgentState::Starting,
                data: instance.into_data(),
            },
            Self::Stopped(instance) => AgentSnapshot {
                state: AgentState::Stopped,
                data: instance.into_data(),
            },
        }
    }
}

/// What AgentKind — `conductor.agent.AgentKind` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`AgentKind<S>`], and at a boundary by [`AgentKindSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKindData {
    /// The identity: `name` — `conductor.agent.AgentKindName`.
    pub name: AgentKindName,
    /// `capabilities` — `List<conductor.agent.Capability>`.
    pub capabilities: Vec<Capability>,
    /// `binary` — `String`.
    pub binary: String,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
}

/// The states of `conductor.agent.AgentKind`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](agent_kind_state::Marker), so [`AgentKind<S>`](AgentKind) can only ever rest in a real state.
pub mod agent_kind_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Available {}
    }

    /// A declared state of `AgentKind`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::AgentKindState;
    }

    /// `Available`. Where a new instance starts.
    pub struct Available;

    impl Marker for Available {
        const STATE: super::AgentKindState = super::AgentKindState::Available;
    }
}

/// AgentKind — `conductor.agent.AgentKind` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Available`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`AgentKindSnapshot`]
/// and [`AgentKindSnapshot::refine`].
pub struct AgentKind<S: agent_kind_state::Marker> {
    data: AgentKindData,
    state: core::marker::PhantomData<S>,
}

impl<S: agent_kind_state::Marker> AgentKind<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> AgentKindState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &AgentKindData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> AgentKindData {
        self.data
    }
}

impl AgentKind<agent_kind_state::Available> {
    /// A new instance, resting in `Available` — the only state the lifecycle starts one in.
    pub fn new(data: AgentKindData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.agent.AgentKind` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`AgentKindSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKindSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: AgentKindState,
    /// What it holds.
    pub data: AgentKindData,
}

/// An `AgentKind` in whichever declared state it was found.
pub enum AnyAgentKind {
    /// Resting in `Available`.
    Available(AgentKind<agent_kind_state::Available>),
}

impl AgentKindSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `AgentKindState` cannot spell one.
    pub fn refine(self) -> AnyAgentKind {
        match self.state {
            AgentKindState::Available => AnyAgentKind::Available(AgentKind {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyAgentKind {
    /// The state, as the runtime value.
    pub fn state(&self) -> AgentKindState {
        match self {
            Self::Available(_) => AgentKindState::Available,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> AgentKindSnapshot {
        match self {
            Self::Available(instance) => AgentKindSnapshot {
                state: AgentKindState::Available,
                data: instance.into_data(),
            },
        }
    }
}

/// Attach to an agent — the input of `conductor.agent.AttachAgent`.
///
/// Everything it can result in is [`AttachAgentOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachAgent {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `watch` — `Optional<Boolean>`.
    pub watch: Option<bool>,
}

/// Everything `conductor.agent.AttachAgent` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachAgentOutcome {
    /// `not-interactive` — when the existing subject's stored fields satisfy `not (exists c in capabilities: (c == Interactive))`.
    ///
    /// The agent's kind has no PTY to attach to; nothing was opened.
    NotInteractive {
        /// Why it was refused: `conductor.agent.CapabilityMissing`.
        error: CapabilityMissing,
    },
    /// `not-running` — when the existing subject's stored fields satisfy `state == Stopped`.
    ///
    /// The agent is stopped; nothing was opened.
    NotRunning {
        /// Why it was refused: `conductor.agent.AgentNotRunning`.
        error: AgentNotRunning,
    },
    /// `unreachable` — externally decided (the agent's PTY does not answer).
    ///
    /// Nothing was opened; the caller names the agent.
    Unreachable {
        /// Why it was refused: `conductor.agent.AgentNotReachable`.
        error: AgentNotReachable,
    },
    /// `attached` — otherwise.
    ///
    /// The caller holds the agent's PTY stream, read-only with `watch`.
    Attached,
    /// `no-such-agent` — for an identity no record carries.
    ///
    /// No agent of that name; nothing was opened.
    NoSuchAgent {
        /// Why it was refused: `conductor.agent.AgentNotFound`.
        error: AgentNotFound,
    },
}

/// Declare an agent kind — the input of `conductor.agent.DeclareAgentKind`.
///
/// Everything it can result in is [`DeclareAgentKindOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclareAgentKind {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentKindName`.
    pub name: AgentKindName,
    /// `capabilities` — `List<conductor.agent.Capability>`.
    pub capabilities: Vec<Capability>,
    /// `binary` — `String`.
    pub binary: String,
}

/// Everything `conductor.agent.DeclareAgentKind` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclareAgentKindOutcome {
    /// `declared` — for an identity no record carries.
    ///
    /// The kind is on record with its capabilities and its program.
    Declared {
        /// The `conductor.agent.AgentKindDeclared` this outcome publishes.
        agent_kind_declared: AgentKindDeclared,
    },
    /// `redeclared` — otherwise.
    ///
    /// The kind was on record; it now holds what the binary says.
    Redeclared {
        /// The `conductor.agent.AgentKindDeclared` this outcome publishes.
        agent_kind_declared: AgentKindDeclared,
    },
}

/// Deliver a text to an agent — the input of `conductor.agent.DeliverToAgent`.
///
/// Everything it can result in is [`DeliverToAgentOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverToAgent {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
}

/// Everything `conductor.agent.DeliverToAgent` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliverToAgentOutcome {
    /// `not-interactive` — when the existing subject's stored fields satisfy `not (exists c in capabilities: (c == Interactive))`.
    ///
    /// The agent's kind is not interactive; nothing was delivered.
    NotInteractive {
        /// Why it was refused: `conductor.agent.CapabilityMissing`.
        error: CapabilityMissing,
    },
    /// `not-running` — when the existing subject's stored fields satisfy `state == Stopped`.
    ///
    /// The agent is stopped; nothing was delivered.
    NotRunning {
        /// Why it was refused: `conductor.agent.AgentNotRunning`.
        error: AgentNotRunning,
    },
    /// `unreachable` — externally decided (the agent's PTY does not answer).
    ///
    /// Nothing was typed; the caller names the agent and does not retry in a loop.
    Unreachable {
        /// Why it was refused: `conductor.agent.AgentNotReachable`.
        error: AgentNotReachable,
    },
    /// `delivered` — otherwise.
    ///
    /// The text is in front of the agent.
    Delivered,
    /// `no-such-agent` — for an identity no record carries.
    ///
    /// No agent of that name; nothing was delivered.
    NoSuchAgent {
        /// Why it was refused: `conductor.agent.AgentNotFound`.
        error: AgentNotFound,
    },
}

/// Mark an agent running — the input of `conductor.agent.MarkAgentRunning`.
///
/// Everything it can result in is [`MarkAgentRunningOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MarkAgentRunning {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `running_at` — `Timestamp`.
    pub running_at: crate::primitives::Timestamp,
}

/// Everything `conductor.agent.MarkAgentRunning` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkAgentRunningOutcome {
    /// `running` — otherwise.
    ///
    /// The agent's program is up; the agent runs.
    Running {
        /// The `conductor.agent.AgentRunning` this outcome publishes.
        agent_running: AgentRunning,
    },
    /// `no-such-agent` — for an identity no record carries.
    ///
    /// No agent of that name; nothing changed.
    NoSuchAgent {
        /// Why it was refused: `conductor.agent.AgentNotFound`.
        error: AgentNotFound,
    },
    /// `not-starting` — from a state no declared move starts in.
    ///
    /// The agent is not starting (running already, or stopped); nothing changed.
    NotStarting {
        /// Why it was refused: `conductor.agent.AgentNotRunning`.
        error: AgentNotRunning,
    },
}

/// Record an agent's report — the input of `conductor.agent.RecordAgentReport`.
///
/// Everything it can result in is [`RecordAgentReportOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordAgentReport {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `line` — `String`.
    pub line: String,
    /// `exit_code` — `Optional<Integer>`.
    pub exit_code: Option<i64>,
    /// `events_path` — `Optional<String>`.
    pub events_path: Option<String>,
    /// `reported_at` — `Timestamp`.
    pub reported_at: crate::primitives::Timestamp,
}

/// Everything `conductor.agent.RecordAgentReport` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordAgentReportOutcome {
    /// `reported` — otherwise.
    ///
    /// The report is on record against the agent.
    Reported {
        /// The `conductor.agent.AgentReported` this outcome publishes.
        agent_reported: AgentReported,
    },
    /// `no-such-agent` — for an identity no record carries.
    ///
    /// No agent of that name; nothing was recorded.
    NoSuchAgent {
        /// Why it was refused: `conductor.agent.AgentNotFound`.
        error: AgentNotFound,
    },
}

/// Restart an agent — the input of `conductor.agent.RestartAgent`.
///
/// Everything it can result in is [`RestartAgentOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RestartAgent {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `token_owner` — `Optional<conductor.operator.OperatorName>`.
    pub token_owner: Option<crate::operator::OperatorName>,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
}

/// Everything `conductor.agent.RestartAgent` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RestartAgentOutcome {
    /// `launch-failed` — externally decided (the kind's start in the box failed, or its first prompt could not be delivered).
    ///
    /// The program did not come up again; the record is unchanged.
    LaunchFailed {
        /// Why it was refused: `conductor.agent.AgentLaunchFailed`.
        error: AgentLaunchFailed,
    },
    /// `restarted` — otherwise.
    ///
    /// The agent runs again as its next generation.
    Restarted {
        /// The `conductor.agent.AgentStarted` this outcome publishes.
        agent_started: AgentStarted,
    },
    /// `no-such-agent` — for an identity no record carries.
    ///
    /// No agent of that name; nothing was started.
    NoSuchAgent {
        /// Why it was refused: `conductor.agent.AgentNotFound`.
        error: AgentNotFound,
    },
}

/// Start an agent — the input of `conductor.agent.StartAgent`.
///
/// Everything it can result in is [`StartAgentOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartAgent {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `role` — `conductor.config.RoleName`.
    pub role: crate::config::RoleName,
    /// `kind` — `conductor.agent.AgentKindName`.
    pub kind: AgentKindName,
    /// `capabilities` — `List<conductor.agent.Capability>`.
    pub capabilities: Vec<Capability>,
    /// `box` — `Optional<conductor.placement.BoxId>`.
    pub r#box: Option<crate::placement::BoxId>,
    /// `profile` — `Optional<String>`.
    pub profile: Option<String>,
    /// `limits` — `conductor.config.Limits`.
    pub limits: crate::config::Limits,
    /// `token_owner` — `Optional<conductor.operator.OperatorName>`.
    pub token_owner: Option<crate::operator::OperatorName>,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
}

/// Everything `conductor.agent.StartAgent` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartAgentOutcome {
    /// `unknown-kind` — when no `conductor.agent.AgentKind` carries the identity `input.kind` names.
    ///
    /// The binary implements no such kind; nothing was started.
    UnknownKind {
        /// Why it was refused: `conductor.agent.UnknownAgentKind`.
        error: UnknownAgentKind,
    },
    /// `launch-failed` — externally decided (the kind's start in the box failed, or its first prompt could not be delivered).
    ///
    /// The program did not come up; no agent is on record.
    LaunchFailed {
        /// Why it was refused: `conductor.agent.AgentLaunchFailed`.
        error: AgentLaunchFailed,
    },
    /// `started` — otherwise.
    ///
    /// The agent runs in its role, generation 1, naming who started it and whose token it runs on.
    Started {
        /// The `conductor.agent.AgentStarted` this outcome publishes.
        agent_started: AgentStarted,
    },
    /// `exists` — for an identity a record already carries.
    ///
    /// An agent of that name is on record; nothing was started, the record unchanged.
    Exists {
        /// Why it was refused: `conductor.agent.AgentExists`.
        error: AgentExists,
    },
}

/// Stop an agent — the input of `conductor.agent.StopAgent`.
///
/// Everything it can result in is [`StopAgentOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopAgent {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `exit_class` — `conductor.agent.ExitClass`.
    pub exit_class: ExitClass,
    /// `exit_code` — `Optional<Integer>`.
    pub exit_code: Option<i64>,
    /// `stopped_at` — `Timestamp`.
    pub stopped_at: crate::primitives::Timestamp,
}

/// Everything `conductor.agent.StopAgent` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopAgentOutcome {
    /// `stopped` — otherwise.
    ///
    /// The agent is stopped, with why and, for an exit, its code.
    Stopped {
        /// The `conductor.agent.AgentStopped` this outcome publishes.
        agent_stopped: AgentStopped,
    },
    /// `no-such-agent` — for an identity no record carries.
    ///
    /// No agent of that name; nothing was stopped.
    NoSuchAgent {
        /// Why it was refused: `conductor.agent.AgentNotFound`.
        error: AgentNotFound,
    },
    /// `not-running` — from a state no declared move starts in.
    ///
    /// The agent is stopped already; nothing changed.
    NotRunning {
        /// Why it was refused: `conductor.agent.AgentNotRunning`.
        error: AgentNotRunning,
    },
}

/// AgentKindDeclared — the event `conductor.agent.AgentKindDeclared`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKindDeclared {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentKindName`.
    pub name: AgentKindName,
    /// `capabilities` — `List<conductor.agent.Capability>`.
    pub capabilities: Vec<Capability>,
    /// `binary` — `String`.
    pub binary: String,
}

/// AgentReported — the event `conductor.agent.AgentReported`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentReported {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `line` — `String`.
    pub line: String,
    /// `exit_code` — `Optional<Integer>`.
    pub exit_code: Option<i64>,
    /// `events_path` — `Optional<String>`.
    pub events_path: Option<String>,
    /// `reported_at` — `Timestamp`.
    pub reported_at: crate::primitives::Timestamp,
}

/// AgentRunning — the event `conductor.agent.AgentRunning`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentRunning {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `running_at` — `Timestamp`.
    pub running_at: crate::primitives::Timestamp,
}

/// AgentStarted — the event `conductor.agent.AgentStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStarted {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `role` — `conductor.config.RoleName`.
    pub role: crate::config::RoleName,
    /// `kind` — `conductor.agent.AgentKindName`.
    pub kind: AgentKindName,
    /// `box` — `Optional<conductor.placement.BoxId>`.
    pub r#box: Option<crate::placement::BoxId>,
    /// `token_owner` — `Optional<conductor.operator.OperatorName>`.
    pub token_owner: Option<crate::operator::OperatorName>,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
}

/// AgentStopped — the event `conductor.agent.AgentStopped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStopped {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `exit_class` — `conductor.agent.ExitClass`.
    pub exit_class: ExitClass,
    /// `exit_code` — `Optional<Integer>`.
    pub exit_code: Option<i64>,
    /// `stopped_at` — `Timestamp`.
    pub stopped_at: crate::primitives::Timestamp,
}

/// The declared error `conductor.agent.AgentExists`.
///
/// An agent of that name is on record already; a stopped one is started again with `RestartAgent`. Nothing was started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExists {
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
}

/// The declared error `conductor.agent.AgentLaunchFailed`.
///
/// The kind's start failed (the program did not come up in the box, or its first prompt could not be delivered), so no agent is on record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentLaunchFailed {
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
}

/// The declared error `conductor.agent.AgentNotFound`.
///
/// No agent of that name is on record; nothing was done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNotFound {
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
}

/// The declared error `conductor.agent.AgentNotReachable`.
///
/// The agent is on record as running, but its PTY does not answer (for the `claude-code` kind: no tmux server on its launcher socket, and, for a delivery, no live background session of its name whose attach client took the text), so nothing was typed or opened; the caller is told which agent, and does not retry in a loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNotReachable {
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
}

/// The declared error `conductor.agent.AgentNotRunning`.
///
/// The agent is stopped, so nothing reached it and nothing changed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentNotRunning {
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
}

/// The declared error `conductor.agent.CapabilityMissing`.
///
/// The agent's kind lacks the capability the command needs, named here (`Interactive` for attach and deliver), so nothing was done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityMissing {
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `capability` — `conductor.agent.Capability`.
    pub capability: Capability,
}

/// The declared error `conductor.agent.UnknownAgentKind`.
///
/// The binary implements no kind of that name, so nothing was started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownAgentKind {
    /// `kind` — `conductor.agent.AgentKindName`.
    pub kind: AgentKindName,
}

/// Agent kinds — one row of the view `conductor.agent.AgentKinds`.
///
/// Projects `conductor.agent.AgentKind` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentKinds {
    /// `name` — `conductor.agent.AgentKindName`.
    pub name: AgentKindName,
    /// `state` — `conductor.agent.AgentKind.State`.
    pub state: AgentKindState,
    /// `capabilities` — `List<conductor.agent.Capability>`.
    pub capabilities: Vec<Capability>,
    /// `binary` — `String`.
    pub binary: String,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
}

/// Agents — one row of the view `conductor.agent.Agents`.
///
/// Projects `conductor.agent.Agent` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Agents {
    /// `name` — `conductor.agent.AgentName`.
    pub name: AgentName,
    /// `state` — `conductor.agent.Agent.State`.
    pub state: AgentState,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `role` — `conductor.config.RoleName`.
    pub role: crate::config::RoleName,
    /// `kind` — `conductor.agent.AgentKindName`.
    pub kind: AgentKindName,
    /// `capabilities` — `List<conductor.agent.Capability>`.
    pub capabilities: Vec<Capability>,
    /// `box` — `Optional<conductor.placement.BoxId>`.
    pub r#box: Option<crate::placement::BoxId>,
    /// `profile` — `Optional<String>`.
    pub profile: Option<String>,
    /// `limits` — `conductor.config.Limits`.
    pub limits: crate::config::Limits,
    /// `token_owner` — `Optional<conductor.operator.OperatorName>`.
    pub token_owner: Option<crate::operator::OperatorName>,
    /// `started_by` — `conductor.operator.Actor`.
    pub started_by: crate::operator::Actor,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `generation` — `Integer`.
    pub generation: i64,
    /// `stopped_at` — `Optional<Timestamp>`.
    pub stopped_at: Option<crate::primitives::Timestamp>,
    /// `exit_class` — `Optional<conductor.agent.ExitClass>`.
    pub exit_class: Option<ExitClass>,
    /// `exit_code` — `Optional<Integer>`.
    pub exit_code: Option<i64>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.agent.AttachAgent` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by the guard `exists c in capabilities: (c == Interactive)`, in `not-interactive`.
    ///
    /// Contract: given `conductor.agent.AttachAgent` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `not-interactive` when the existing subject's stored fields satisfy `not (exists c in capabilities: (c == Interactive))`, error `conductor.agent.CapabilityMissing`; `not-running` when the existing subject's stored fields satisfy `state == Stopped`, error `conductor.agent.AgentNotRunning`; `unreachable` externally decided (the agent's PTY does not answer), error `conductor.agent.AgentNotReachable`; `attached` otherwise, preserves `conductor.agent.Agent`; `no-such-agent` for an identity no record carries, error `conductor.agent.AgentNotFound`.
    pub trait AttachAgentBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.AttachAgent`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn attach_agent(&mut self, input: super::AttachAgent) -> Result<super::AttachAgentOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.agent.DeclareAgentKind` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait DeclareAgentKindBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.DeclareAgentKind`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn declare_agent_kind(&mut self, input: super::DeclareAgentKind) -> Result<super::DeclareAgentKindOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.agent.DeliverToAgent` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by the guard `exists c in capabilities: (c == Interactive)`, in `not-interactive`.
    ///
    /// Contract: given `conductor.agent.DeliverToAgent` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `not-interactive` when the existing subject's stored fields satisfy `not (exists c in capabilities: (c == Interactive))`, error `conductor.agent.CapabilityMissing`; `not-running` when the existing subject's stored fields satisfy `state == Stopped`, error `conductor.agent.AgentNotRunning`; `unreachable` externally decided (the agent's PTY does not answer), error `conductor.agent.AgentNotReachable`; `delivered` otherwise, preserves `conductor.agent.Agent`; `no-such-agent` for an identity no record carries, error `conductor.agent.AgentNotFound`.
    pub trait DeliverToAgentBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.DeliverToAgent`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn deliver_to_agent(&mut self, input: super::DeliverToAgent) -> Result<super::DeliverToAgentOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.agent.MarkAgentRunning` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait MarkAgentRunningBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.MarkAgentRunning`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn mark_agent_running(&mut self, input: super::MarkAgentRunning) -> Result<super::MarkAgentRunningOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.agent.RecordAgentReport` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordAgentReportBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.RecordAgentReport`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_agent_report(&mut self, input: super::RecordAgentReport) -> Result<super::RecordAgentReportOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.agent.RestartAgent` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RestartAgentBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.RestartAgent`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn restart_agent(&mut self, input: super::RestartAgent) -> Result<super::RestartAgentOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.agent.StartAgent` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by `external:` beside `when_related:` in one command.
    ///
    /// Contract: given `conductor.agent.StartAgent` input, decide and enact exactly one outcome. Selection precedence: on commands with `when_related:`, check `existing_instance` then `exists: false` before input-guarded refusals; choose the first declared input refusal whose guard holds; then check addressed-row existence (`unknown_instance`, and `existing_instance` on commands without `when_related:`); then the held state (`when_subject_state` and `when_subject`), with `wrong_state` only if the selected branch moves from a state the row does not hold; then accepting and external branches in declaration order. An accepting branch that moves nothing answers in every state. Related-presence predicates do not precede input-guarded refusals. Declared outcomes (declaration order, not selection precedence): `unknown-kind` when no `conductor.agent.AgentKind` carries the identity `input.kind` names, error `conductor.agent.UnknownAgentKind`; `launch-failed` externally decided (the kind's start in the box failed, or its first prompt could not be delivered), error `conductor.agent.AgentLaunchFailed`; `started` otherwise, creates `conductor.agent.Agent`, emits `conductor.agent.AgentStarted`; `exists` for an identity a record already carries, error `conductor.agent.AgentExists`.
    pub trait StartAgentBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.StartAgent`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn start_agent(&mut self, input: super::StartAgent) -> Result<super::StartAgentOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.agent.StopAgent` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait StopAgentBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.agent.StopAgent`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn stop_agent(&mut self, input: super::StopAgent) -> Result<super::StopAgentOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.agent.AgentKinds` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait AgentKindsQuery {
        /// Serves `conductor.agent.AgentKinds` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn agent_kinds(&self) -> Result<Vec<super::AgentKinds>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.agent.Agents` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait AgentsQuery {
        /// Serves `conductor.agent.Agents` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn agents(&self) -> Result<Vec<super::Agents>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl AttachAgentBehavior for Unimplemented {
        fn attach_agent(&mut self, _input: super::AttachAgent) -> Result<super::AttachAgentOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.agent.AttachAgent" })
        }
    }

    impl DeliverToAgentBehavior for Unimplemented {
        fn deliver_to_agent(&mut self, _input: super::DeliverToAgent) -> Result<super::DeliverToAgentOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.agent.DeliverToAgent" })
        }
    }

    impl StartAgentBehavior for Unimplemented {
        fn start_agent(&mut self, _input: super::StartAgent) -> Result<super::StartAgentOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.agent.StartAgent" })
        }
    }
}
