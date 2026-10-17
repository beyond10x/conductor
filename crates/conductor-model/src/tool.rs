// generated from conductor v1
// model digest 90be4e2c6c23f286962f8f65af3ffd942d3f17e3bd9e0cba69fb2f3c58630472
// contract digest d909f3155a8d82e6f61f9b63b21eb3a57fb7230ca37cf7a9932e5762eeb17f5a
// do not edit: regenerate with `ess synthesize --layout crate`

//! Tool — `conductor.tool`.
//!
//! The confined tool surface of an instance's sessions (story:tool-broker-read, design docs/design/roles-and-surfaces.md § 6). A session acts through the `tool` binary, a thin client with no store access and no admin command, or through `tool mcp`, the same verbs as an MCP server over stdio. Either sends each call over the session's own socket to the instance's broker (`conductor broker serve`), which holds what the session must not: the store, and the session's grant. The broker identifies the caller by the socket the call arrives on, never by a token the caller passes; it checks the call against the session's grant, runs it, records it as a `ToolCall`, and answers.
//! One broker per instance, keyed by the instance's name; it does not outlive the instance. Its directory is `broker/` under the instance's state directory: `sessions/<session name>.json`, the registration of each session it serves (its instance, the directory it reads in, and its grant), and `sockets/<session name>.sock`, the session's socket. `conductor spawn` and `conductor session start` register a session before it starts and name its socket in the session's environment as `TOOL_SOCKET`; `conductor session stop` removes the registration, and a call on the socket of a session that is no longer registered is refused `UnknownSession`. A registration that names another instance than the broker's is refused `ForeignInstance`.
//! Every verb answers in a bounded form: on success, its result bounded by the grant's `max_output_bytes` (and a verb's own `--max`), then one summary line; on failure, at most 40 lines and the path of a log under `broker/logs/` that holds the whole output. A refusal is one line, `<Error>: <detail>`, and exit status 1.
//! The build, version-control and team verbs (story:tool-build-commit-report, design § 6.3) run what the session must not run itself. `test`, `check` and `fmt` run cargo in the session's working directory through the host ledger, at nice 19, as `conductor build run --reserve auto` runs a command: the reservation is the measured peak of that command, and a run waits in the ledger's queue for its turn. `commit` stages the named paths and commits them through the bot route (`b10x-gates bot`) with the instance's Gates policy, which only the broker holds. `send` and `report` record a `conductor.dispatch.Message`: `send` to a session of the caller's team, and `report` to the session one level above the caller, moving the caller's open dispatch as `conductor dispatch report-*` does when the caller is a team's lead. The broker cannot drive the harness's own channel (SendMessage), so it records the message and answers it for the caller to deliver.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `conductor.tool.Broker`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Broker<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrokerState {
    /// `Serving`.
    Serving,
}

/// CallAnswer — `conductor.tool.CallAnswer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallAnswer {
    /// `text` — `String`.
    pub text: String,
    /// `exit` — `Integer`.
    pub exit: i64,
}

/// CallOutcome — `conductor.tool.CallOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallOutcome {
    /// `Done`.
    Done,
    /// `Failed`.
    Failed,
    /// `NotGranted`.
    NotGranted,
    /// `GrantExpired`.
    GrantExpired,
    /// `ForeignInstance`.
    ForeignInstance,
    /// `UnknownSession`.
    UnknownSession,
    /// `OutsideTeam`.
    OutsideTeam,
    /// `NotOneLevelUp`.
    NotOneLevelUp,
}

/// CallRefused — `conductor.tool.CallRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRefused {
    /// `refusal` — `conductor.tool.CallOutcome`.
    pub refusal: CallOutcome,
    /// `detail` — `String`.
    pub detail: String,
}

/// CheckCall — `conductor.tool.CheckCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CheckCall {
    /// `package` — `Optional<String>`.
    pub package: Option<String>,
}

/// CommitCall — `conductor.tool.CommitCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitCall {
    /// `message` — `String`.
    pub message: String,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
}

/// DiffCall — `conductor.tool.DiffCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffCall {
    /// `stat` — `Optional<Boolean>`.
    pub stat: Option<bool>,
    /// `patch` — `Optional<Boolean>`.
    pub patch: Option<bool>,
}

/// FmtCall — `conductor.tool.FmtCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FmtCall {
    /// `package` — `Optional<String>`.
    pub package: Option<String>,
}

/// The states of `conductor.tool.Grant`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Grant<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrantState {
    /// `Active`.
    Active,
}

/// Level — `conductor.tool.Level`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `Director`.
    Director,
    /// `Lead`.
    Lead,
    /// `Worker`.
    Worker,
}

/// ReportCall — `conductor.tool.ReportCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportCall {
    /// `status` — `conductor.tool.ReportStatus`.
    pub status: ReportStatus,
    /// `line` — `String`.
    pub line: String,
    /// `to` — `Optional<conductor.config.SessionName>`.
    pub to: Option<crate::config::SessionName>,
}

/// ReportStatus — `conductor.tool.ReportStatus`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportStatus {
    /// `Started`.
    Started,
    /// `Progress`.
    Progress,
    /// `Blocked`.
    Blocked,
    /// `Unblocked`.
    Unblocked,
    /// `Done`.
    Done,
    /// `Failed`.
    Failed,
}

/// RoleGrant — `conductor.tool.RoleGrant`.
///
/// Every value satisfies `max_output_bytes >= 1` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleGrant {
    /// `tools` — `List<conductor.tool.Verb>`.
    pub tools: Vec<Verb>,
    /// `repositories` — `List<String>`.
    pub repositories: Vec<String>,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `max_output_bytes` — `Integer`.
    pub max_output_bytes: i64,
    /// `ttl` — `Optional<Duration>`.
    pub ttl: Option<crate::primitives::Duration>,
}

/// SearchCall — `conductor.tool.SearchCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchCall {
    /// `pattern` — `String`.
    pub pattern: String,
    /// `path` — `Optional<String>`.
    pub path: Option<String>,
}

/// SendCall — `conductor.tool.SendCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendCall {
    /// `to` — `String`.
    pub to: String,
    /// `text` — `String`.
    pub text: String,
}

/// ShowCall — `conductor.tool.ShowCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowCall {
    /// `path` — `String`.
    pub path: String,
    /// `lines` — `Optional<String>`.
    pub lines: Option<String>,
    /// `symbol` — `Optional<String>`.
    pub symbol: Option<String>,
    /// `max` — `Optional<Integer>`.
    pub max: Option<i64>,
}

/// The states of `conductor.tool.Team`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Team<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeamState {
    /// `Formed`.
    Formed,
}

/// TeamName — `conductor.tool.TeamName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamName(pub String);

/// TestCall — `conductor.tool.TestCall`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestCall {
    /// `package` — `Optional<String>`.
    pub package: Option<String>,
    /// `filter` — `Optional<String>`.
    pub filter: Option<String>,
}

/// The states of `conductor.tool.ToolCall`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ToolCall<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallState {
    /// `Recorded`.
    Recorded,
}

/// ToolCallId — `conductor.tool.ToolCallId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallId(pub crate::primitives::Uuid);

/// The states of `conductor.tool.ToolSession`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ToolSession<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolSessionState {
    /// `Registered`.
    Registered,
}

/// Verb — `conductor.tool.Verb`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    /// `Show`.
    Show,
    /// `Search`.
    Search,
    /// `Diff`.
    Diff,
    /// `Test`.
    Test,
    /// `Check`.
    Check,
    /// `Fmt`.
    Fmt,
    /// `Commit`.
    Commit,
    /// `Send`.
    Send,
    /// `Report`.
    Report,
}

/// What Broker — `conductor.tool.Broker` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Broker<S>`], and at a boundary by [`BrokerSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerData {
    /// The identity: `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
}

/// The states of `conductor.tool.Broker`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](broker_state::Marker), so [`Broker<S>`](Broker) can only ever rest in a real state.
pub mod broker_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Serving {}
    }

    /// A declared state of `Broker`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::BrokerState;
    }

    /// `Serving`. Where a new instance starts.
    pub struct Serving;

    impl Marker for Serving {
        const STATE: super::BrokerState = super::BrokerState::Serving;
    }
}

/// Broker — `conductor.tool.Broker` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Serving`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`BrokerSnapshot`]
/// and [`BrokerSnapshot::refine`].
pub struct Broker<S: broker_state::Marker> {
    data: BrokerData,
    state: core::marker::PhantomData<S>,
}

impl<S: broker_state::Marker> Broker<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> BrokerState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &BrokerData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> BrokerData {
        self.data
    }
}

impl Broker<broker_state::Serving> {
    /// A new instance, resting in `Serving` — the only state the lifecycle starts one in.
    pub fn new(data: BrokerData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.tool.Broker` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`BrokerSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrokerSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: BrokerState,
    /// What it holds.
    pub data: BrokerData,
}

/// An `Broker` in whichever declared state it was found.
pub enum AnyBroker {
    /// Resting in `Serving`.
    Serving(Broker<broker_state::Serving>),
}

impl BrokerSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `BrokerState` cannot spell one.
    pub fn refine(self) -> AnyBroker {
        match self.state {
            BrokerState::Serving => AnyBroker::Serving(Broker {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyBroker {
    /// The state, as the runtime value.
    pub fn state(&self) -> BrokerState {
        match self {
            Self::Serving(_) => BrokerState::Serving,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> BrokerSnapshot {
        match self {
            Self::Serving(instance) => BrokerSnapshot {
                state: BrokerState::Serving,
                data: instance.into_data(),
            },
        }
    }
}

/// What Grant — `conductor.tool.Grant` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Grant<S>`], and at a boundary by [`GrantSnapshot::state`].
///
/// Every value satisfies `max_output_bytes >= 1` — checked by [`GrantData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantData {
    /// The identity: `session` — `conductor.config.SessionName`.
    ///
    /// Carries `holder`: `conductor.tool.Grant` references one `conductor.tool.ToolSession`.
    pub session: crate::config::SessionName,
    /// `tools` — `List<conductor.tool.Verb>`.
    pub tools: Vec<Verb>,
    /// `repositories` — `List<String>`.
    pub repositories: Vec<String>,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `max_output_bytes` — `Integer`.
    pub max_output_bytes: i64,
    /// `expires_at` — `Optional<Timestamp>`.
    pub expires_at: Option<crate::primitives::Timestamp>,
}

impl GrantData {
    /// The first declared invariant of `conductor.tool.Grant` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.max_output_bytes)), iv::Op::Ge, iv::Fact::number("1"), false, true)) {
            return Some("max_output_bytes >= 1");
        }
        None
    }
}

/// The states of `conductor.tool.Grant`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](grant_state::Marker), so [`Grant<S>`](Grant) can only ever rest in a real state.
pub mod grant_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Active {}
    }

    /// A declared state of `Grant`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::GrantState;
    }

    /// `Active`. Where a new instance starts.
    pub struct Active;

    impl Marker for Active {
        const STATE: super::GrantState = super::GrantState::Active;
    }
}

/// Grant — `conductor.tool.Grant` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Active`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`GrantSnapshot`]
/// and [`GrantSnapshot::refine`].
pub struct Grant<S: grant_state::Marker> {
    data: GrantData,
    state: core::marker::PhantomData<S>,
}

impl<S: grant_state::Marker> Grant<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> GrantState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &GrantData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> GrantData {
        self.data
    }
}

impl Grant<grant_state::Active> {
    /// A new instance, resting in `Active` — the only state the lifecycle starts one in.
    pub fn new(data: GrantData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.tool.Grant` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`GrantSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: GrantState,
    /// What it holds.
    pub data: GrantData,
}

/// An `Grant` in whichever declared state it was found.
pub enum AnyGrant {
    /// Resting in `Active`.
    Active(Grant<grant_state::Active>),
}

impl GrantSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `GrantState` cannot spell one.
    pub fn refine(self) -> AnyGrant {
        match self.state {
            GrantState::Active => AnyGrant::Active(Grant {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyGrant {
    /// The state, as the runtime value.
    pub fn state(&self) -> GrantState {
        match self {
            Self::Active(_) => GrantState::Active,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> GrantSnapshot {
        match self {
            Self::Active(instance) => GrantSnapshot {
                state: GrantState::Active,
                data: instance.into_data(),
            },
        }
    }
}

/// What Team — `conductor.tool.Team` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Team<S>`], and at a boundary by [`TeamSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamData {
    /// The identity: `team` — `conductor.tool.TeamName`.
    pub team: TeamName,
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `broker`: `conductor.tool.Team` references one `conductor.tool.Broker`.
    pub instance: crate::config::InstanceName,
    /// `lead` — `Optional<conductor.config.SessionName>`.
    ///
    /// Carries `led_by`: `conductor.tool.Team` references one `conductor.tool.ToolSession`.
    pub lead: Option<crate::config::SessionName>,
}

/// The states of `conductor.tool.Team`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](team_state::Marker), so [`Team<S>`](Team) can only ever rest in a real state.
pub mod team_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Formed {}
    }

    /// A declared state of `Team`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::TeamState;
    }

    /// `Formed`. Where a new instance starts.
    pub struct Formed;

    impl Marker for Formed {
        const STATE: super::TeamState = super::TeamState::Formed;
    }
}

/// Team — `conductor.tool.Team` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Formed`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`TeamSnapshot`]
/// and [`TeamSnapshot::refine`].
pub struct Team<S: team_state::Marker> {
    data: TeamData,
    state: core::marker::PhantomData<S>,
}

impl<S: team_state::Marker> Team<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> TeamState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &TeamData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> TeamData {
        self.data
    }
}

impl Team<team_state::Formed> {
    /// A new instance, resting in `Formed` — the only state the lifecycle starts one in.
    pub fn new(data: TeamData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.tool.Team` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`TeamSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: TeamState,
    /// What it holds.
    pub data: TeamData,
}

/// An `Team` in whichever declared state it was found.
pub enum AnyTeam {
    /// Resting in `Formed`.
    Formed(Team<team_state::Formed>),
}

impl TeamSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `TeamState` cannot spell one.
    pub fn refine(self) -> AnyTeam {
        match self.state {
            TeamState::Formed => AnyTeam::Formed(Team {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyTeam {
    /// The state, as the runtime value.
    pub fn state(&self) -> TeamState {
        match self {
            Self::Formed(_) => TeamState::Formed,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> TeamSnapshot {
        match self {
            Self::Formed(instance) => TeamSnapshot {
                state: TeamState::Formed,
                data: instance.into_data(),
            },
        }
    }
}

/// What ToolCall — `conductor.tool.ToolCall` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ToolCall<S>`], and at a boundary by [`ToolCallSnapshot::state`].
///
/// Every value satisfies `duration_ms >= 0` — checked by [`ToolCallData::broken_invariant`].
/// Every value satisfies `output_bytes >= 0` — checked by [`ToolCallData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallData {
    /// The identity: `tool_call_id` — `conductor.tool.ToolCallId`.
    pub tool_call_id: ToolCallId,
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `calls`: `conductor.tool.Broker` owns many `conductor.tool.ToolCall`.
    pub instance: crate::config::InstanceName,
    /// `session` — `conductor.config.SessionName`.
    ///
    /// Carries `caller`: `conductor.tool.ToolCall` references one `conductor.tool.ToolSession`.
    pub session: crate::config::SessionName,
    /// `verb` — `conductor.tool.Verb`.
    pub verb: Verb,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `outcome` — `conductor.tool.CallOutcome`.
    pub outcome: CallOutcome,
    /// `rule` — `Optional<String>`.
    pub rule: Option<String>,
    /// `duration_ms` — `Integer`.
    pub duration_ms: i64,
    /// `output_bytes` — `Integer`.
    pub output_bytes: i64,
    /// `called_at` — `Timestamp`.
    pub called_at: crate::primitives::Timestamp,
}

impl ToolCallData {
    /// The first declared invariant of `conductor.tool.ToolCall` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.duration_ms)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("duration_ms >= 0");
        }
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.output_bytes)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("output_bytes >= 0");
        }
        None
    }
}

/// The states of `conductor.tool.ToolCall`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](tool_call_state::Marker), so [`ToolCall<S>`](ToolCall) can only ever rest in a real state.
pub mod tool_call_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `ToolCall`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ToolCallState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::ToolCallState = super::ToolCallState::Recorded;
    }
}

/// ToolCall — `conductor.tool.ToolCall` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ToolCallSnapshot`]
/// and [`ToolCallSnapshot::refine`].
pub struct ToolCall<S: tool_call_state::Marker> {
    data: ToolCallData,
    state: core::marker::PhantomData<S>,
}

impl<S: tool_call_state::Marker> ToolCall<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ToolCallState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ToolCallData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ToolCallData {
        self.data
    }
}

impl ToolCall<tool_call_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: ToolCallData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.tool.ToolCall` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ToolCallSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ToolCallState,
    /// What it holds.
    pub data: ToolCallData,
}

/// An `ToolCall` in whichever declared state it was found.
pub enum AnyToolCall {
    /// Resting in `Recorded`.
    Recorded(ToolCall<tool_call_state::Recorded>),
}

impl ToolCallSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ToolCallState` cannot spell one.
    pub fn refine(self) -> AnyToolCall {
        match self.state {
            ToolCallState::Recorded => AnyToolCall::Recorded(ToolCall {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyToolCall {
    /// The state, as the runtime value.
    pub fn state(&self) -> ToolCallState {
        match self {
            Self::Recorded(_) => ToolCallState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ToolCallSnapshot {
        match self {
            Self::Recorded(instance) => ToolCallSnapshot {
                state: ToolCallState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What ToolSession — `conductor.tool.ToolSession` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ToolSession<S>`], and at a boundary by [`ToolSessionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSessionData {
    /// The identity: `session` — `conductor.config.SessionName`.
    pub session: crate::config::SessionName,
    /// `instance` — `conductor.config.InstanceName`.
    ///
    /// Carries `broker`: `conductor.tool.ToolSession` references one `conductor.tool.Broker`.
    pub instance: crate::config::InstanceName,
    /// `socket` — `String`.
    pub socket: String,
    /// `workdir` — `String`.
    pub workdir: String,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
    /// `level` — `Optional<conductor.tool.Level>`.
    pub level: Option<Level>,
    /// `team` — `Optional<conductor.tool.TeamName>`.
    ///
    /// Carries `member_of`: `conductor.tool.ToolSession` references one `conductor.tool.Team`.
    pub team: Option<TeamName>,
    /// `above` — `Optional<conductor.config.SessionName>`.
    ///
    /// Carries `reports_to`: `conductor.tool.ToolSession` references one `conductor.tool.ToolSession`.
    pub above: Option<crate::config::SessionName>,
}

/// The states of `conductor.tool.ToolSession`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](tool_session_state::Marker), so [`ToolSession<S>`](ToolSession) can only ever rest in a real state.
pub mod tool_session_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Registered {}
    }

    /// A declared state of `ToolSession`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ToolSessionState;
    }

    /// `Registered`. Where a new instance starts.
    pub struct Registered;

    impl Marker for Registered {
        const STATE: super::ToolSessionState = super::ToolSessionState::Registered;
    }
}

/// ToolSession — `conductor.tool.ToolSession` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Registered`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ToolSessionSnapshot`]
/// and [`ToolSessionSnapshot::refine`].
pub struct ToolSession<S: tool_session_state::Marker> {
    data: ToolSessionData,
    state: core::marker::PhantomData<S>,
}

impl<S: tool_session_state::Marker> ToolSession<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ToolSessionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ToolSessionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ToolSessionData {
        self.data
    }
}

impl ToolSession<tool_session_state::Registered> {
    /// A new instance, resting in `Registered` — the only state the lifecycle starts one in.
    pub fn new(data: ToolSessionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.tool.ToolSession` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ToolSessionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolSessionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ToolSessionState,
    /// What it holds.
    pub data: ToolSessionData,
}

/// An `ToolSession` in whichever declared state it was found.
pub enum AnyToolSession {
    /// Resting in `Registered`.
    Registered(ToolSession<tool_session_state::Registered>),
}

impl ToolSessionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ToolSessionState` cannot spell one.
    pub fn refine(self) -> AnyToolSession {
        match self.state {
            ToolSessionState::Registered => AnyToolSession::Registered(ToolSession {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyToolSession {
    /// The state, as the runtime value.
    pub fn state(&self) -> ToolSessionState {
        match self {
            Self::Registered(_) => ToolSessionState::Registered,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ToolSessionSnapshot {
        match self {
            Self::Registered(instance) => ToolSessionSnapshot {
                state: ToolSessionState::Registered,
                data: instance.into_data(),
            },
        }
    }
}

/// Record a tool call — the input of `conductor.tool.RecordToolCall`.
///
/// Everything it can result in is [`RecordToolCallOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordToolCall {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session` — `conductor.config.SessionName`.
    pub session: crate::config::SessionName,
    /// `verb` — `conductor.tool.Verb`.
    pub verb: Verb,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `outcome` — `conductor.tool.CallOutcome`.
    pub outcome: CallOutcome,
    /// `rule` — `Optional<String>`.
    pub rule: Option<String>,
    /// `duration_ms` — `Integer`.
    pub duration_ms: i64,
    /// `output_bytes` — `Integer`.
    pub output_bytes: i64,
    /// `called_at` — `Timestamp`.
    pub called_at: crate::primitives::Timestamp,
}

/// Everything `conductor.tool.RecordToolCall` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordToolCallOutcome {
    /// `recorded` — otherwise.
    ///
    /// The broker's answer to one call, run or refused, is recorded once it is answered.
    Recorded {
        /// The `conductor.tool.ToolCallRecorded` this outcome publishes.
        tool_call_recorded: ToolCallRecorded,
    },
}

/// ToolCallRecorded — the event `conductor.tool.ToolCallRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCallRecorded {
    /// `tool_call_id` — `conductor.tool.ToolCallId`.
    pub tool_call_id: ToolCallId,
    /// `session` — `conductor.config.SessionName`.
    pub session: crate::config::SessionName,
    /// `verb` — `conductor.tool.Verb`.
    pub verb: Verb,
    /// `outcome` — `conductor.tool.CallOutcome`.
    pub outcome: CallOutcome,
}

/// The declared error `conductor.tool.BuildFailed`.
///
/// `test`, `check` or `fmt` ran and failed, or the host ledger refused its reservation. The answer is the failing tests' names and the first line of each one's panic, or the compiler's errors, at most 40 lines; `log` holds the whole output. The call is recorded `Failed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildFailed {
    /// `log` — `String`.
    pub log: String,
}

/// The declared error `conductor.tool.CommitRefused`.
///
/// The bot route refused the commit, or the broker holds no Gates policy, or a path did not stage. `reason` is the route's first line; the call is recorded `Failed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRefused {
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `conductor.tool.ForeignInstance`.
///
/// The session belongs to another instance than the broker's. The call did not run; it is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForeignInstance {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
}

/// The declared error `conductor.tool.GrantExpired`.
///
/// The session's grant has passed its time to live. The call did not run; it is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantExpired {
    /// `expires_at` — `Timestamp`.
    pub expires_at: crate::primitives::Timestamp,
}

/// The declared error `conductor.tool.NotGranted`.
///
/// The call is outside the session's grant: its verb is not among the grant's tools, or a path it names lies outside the grant's paths. `rule` names the grant rule it broke, such as `tools: show` or `paths: src`. The call did not run; it is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotGranted {
    /// `rule` — `String`.
    pub rule: String,
}

/// The declared error `conductor.tool.NotOneLevelUp`.
///
/// A `report` goes to another session than the one one level above the caller, or the caller has no level above it. Nothing was recorded but the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotOneLevelUp {
    /// `recipient` — `String`.
    pub recipient: String,
}

/// The declared error `conductor.tool.OutsideTeam`.
///
/// A `send` names a recipient that is no other session of the caller's team: a session of another team, a session with no team, or none the broker serves. A recipient registered under another instance is refused `ForeignInstance` instead. Nothing was recorded but the call.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutsideTeam {
    /// `recipient` — `String`.
    pub recipient: String,
}

/// The declared error `conductor.tool.UnknownSession`.
///
/// No session the broker serves holds the socket the call came in on: the session stopped, or was never registered. The call did not run; it is recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownSession {
    /// `session` — `conductor.config.SessionName`.
    pub session: crate::config::SessionName,
}

/// Tool calls — one row of the view `conductor.tool.ToolCalls`.
///
/// Projects `conductor.tool.ToolCall` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolCalls {
    /// `tool_call_id` — `conductor.tool.ToolCallId`.
    pub tool_call_id: ToolCallId,
    /// `session` — `conductor.config.SessionName`.
    pub session: crate::config::SessionName,
    /// `verb` — `conductor.tool.Verb`.
    pub verb: Verb,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
    /// `outcome` — `conductor.tool.CallOutcome`.
    pub outcome: CallOutcome,
    /// `rule` — `Optional<String>`.
    pub rule: Option<String>,
    /// `duration_ms` — `Integer`.
    pub duration_ms: i64,
    /// `output_bytes` — `Integer`.
    pub output_bytes: i64,
    /// `called_at` — `Timestamp`.
    pub called_at: crate::primitives::Timestamp,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.tool.RecordToolCall` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordToolCallBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.tool.RecordToolCall`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_tool_call(&mut self, input: super::RecordToolCall) -> Result<super::RecordToolCallOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.tool.ToolCalls` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ToolCallsQuery {
        /// Serves `conductor.tool.ToolCalls` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn tool_calls(&self) -> Result<Vec<super::ToolCalls>, crate::obligation::UnmetObligation>;
    }

}
