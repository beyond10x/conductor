// generated from conductor v1
// model digest 4b170c7cd3171a9a70f4559f0301c763b124dded6e2db1cb6cf4e4f0c84dc3f1
// contract digest 0b2e56522aeb2f6b1c9ff919e42aafb7f2fa85bcedc6e0ea4f88d89d955a6d38
// do not edit: regenerate with `ess synthesize --layout crate`

//! Session — `conductor.session`.
//!
//! The one conductor of an instance (story:single-conductor-lease). The lease, kept in the store under the instance's state directory, names the session id and the process id of the conductor session that holds it. Every conductor session started with the instance's settings claims it at its start, through the settings' SessionStart hook; a claim while another live process holds it is refused and names that holder, and a claim over a holder whose process is gone takes the lease over and records whom it took it from. Design: docs/design/conductor.md § 7.
//! It also holds how conductor starts and stops the sessions of an instance's roles in the box the instance's `confinement` asks for (story:sandbox-spec, story:session-envelope-seam): the request a provider is given, the identity of the session it started, and the measurement it takes at the box's boundary, kept per session under the instance's state directory.
//! And it holds what the run daemon does to sessions (docs/design/conductor-run.md § 3, story:mailbox-send-and-deliver, story:run-daemon): `conductor run` is one process per instance that keeps no state outside the store; each of its ticks is recorded with the lines it handled and the actions it took, and each text it puts in front of a running session with `conductor session deliver` is recorded as a delivery.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// AttachMode — `conductor.session.AttachMode`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachMode {
    /// `Attach`.
    Attach,
    /// `Watch`.
    Watch,
}

/// AttachSession — `conductor.session.AttachSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachSession {
    /// `name` — `Optional<String>`.
    pub name: Option<String>,
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
    /// `watch` — `Optional<Boolean>`.
    pub watch: Option<bool>,
}

/// AttachableSession — `conductor.session.AttachableSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttachableSession {
    /// `instance` — `String`.
    pub instance: String,
    /// `name` — `String`.
    pub name: String,
    /// `mode` — `conductor.session.AttachMode`.
    pub mode: AttachMode,
    /// `directory` — `Optional<String>`.
    pub directory: Option<String>,
    /// `busy` — `Optional<Boolean>`.
    pub busy: Option<bool>,
    /// `idle_seconds` — `Optional<Integer>`.
    pub idle_seconds: Option<i64>,
}

/// DeliverToSession — `conductor.session.DeliverToSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverToSession {
    /// `name` — `String`.
    pub name: String,
    /// `text` — `String`.
    pub text: String,
}

/// The states of `conductor.session.Delivery`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Delivery<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryState {
    /// `Delivered`.
    Delivered,
}

/// DeliveryId — `conductor.session.DeliveryId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryId(pub crate::primitives::Uuid);

/// EnvelopeMeasurement — `conductor.session.EnvelopeMeasurement`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopeMeasurement {
    /// `session` — `conductor.session.StartedSession`.
    pub session: StartedSession,
    /// `measured_at` — `Timestamp`.
    pub measured_at: crate::primitives::Timestamp,
    /// `provider` — `Optional<conductor.observation.EnvelopeProvider>`.
    pub provider: Option<crate::observation::EnvelopeProvider>,
    /// `cgroup` — `Optional<String>`.
    pub cgroup: Option<String>,
    /// `memory_max` — `Optional<Integer>`.
    pub memory_max: Option<i64>,
    /// `cpu_weight` — `Optional<Integer>`.
    pub cpu_weight: Option<i64>,
    /// `tasks_max` — `Optional<Integer>`.
    pub tasks_max: Option<i64>,
    /// `places` — `List<conductor.session.PlaceResolved>`.
    pub places: Vec<PlaceResolved>,
    /// `writable` — `Optional<String>`.
    pub writable: Option<String>,
    /// `fence_missing` — `Optional<String>`.
    pub fence_missing: Option<String>,
    /// `unboxed` — `Optional<String>`.
    pub unboxed: Option<String>,
}

/// The states of `conductor.session.Lease`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Lease<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaseState {
    /// `Held`.
    Held,
}

/// ListSessions — `conductor.session.ListSessions`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListSessions {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// PlaceResolved — `conductor.session.PlaceResolved`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaceResolved {
    /// `place` — `conductor.config.Writable`.
    pub place: crate::config::Writable,
    /// `paths` — `List<String>`.
    pub paths: Vec<String>,
}

/// The states of `conductor.session.RunTick`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `RunTick<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunTickState {
    /// `Recorded`.
    Recorded,
}

/// SessionAttachRefusal — `conductor.session.SessionAttachRefusal`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionAttachRefusal {
    /// `InsideSession`.
    InsideSession,
    /// `UnknownSession`.
    UnknownSession,
    /// `NotAttachable`.
    NotAttachable,
    /// `Ambiguous`.
    Ambiguous,
}

/// SessionAttachRefused — `conductor.session.SessionAttachRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionAttachRefused {
    /// `name` — `String`.
    pub name: String,
    /// `refusal` — `conductor.session.SessionAttachRefusal`.
    pub refusal: SessionAttachRefusal,
    /// `detail` — `String`.
    pub detail: String,
}

/// SessionId — `conductor.session.SessionId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionId(pub String);

/// SessionRequest — `conductor.session.SessionRequest`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRequest {
    /// `role` — `String`.
    pub role: String,
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
    /// `cwd` — `String`.
    pub cwd: String,
    /// `limits` — `conductor.config.Limits`.
    pub limits: crate::config::Limits,
    /// `writable` — `List<conductor.session.PlaceResolved>`.
    pub writable: Vec<PlaceResolved>,
    /// `network` — `conductor.config.Network`.
    pub network: crate::config::Network,
    /// `forge_token_env` — `List<String>`.
    pub forge_token_env: Vec<String>,
}

/// SessionStartRefusal — `conductor.session.SessionStartRefusal`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStartRefusal {
    /// `InstanceNotPinned`.
    InstanceNotPinned,
    /// `ProviderUnavailable`.
    ProviderUnavailable,
    /// `MeasurementShort`.
    MeasurementShort,
    /// `UnknownRole`.
    UnknownRole,
    /// `SessionRunning`.
    SessionRunning,
    /// `LaunchFailed`.
    LaunchFailed,
}

/// SessionStartRefused — `conductor.session.SessionStartRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStartRefused {
    /// `role` — `String`.
    pub role: String,
    /// `refusal` — `conductor.session.SessionStartRefusal`.
    pub refusal: SessionStartRefusal,
    /// `detail` — `String`.
    pub detail: String,
}

/// SessionStopRefusal — `conductor.session.SessionStopRefusal`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStopRefusal {
    /// `UnknownRole`.
    UnknownRole,
    /// `NotRunning`.
    NotRunning,
}

/// SessionStopRefused — `conductor.session.SessionStopRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionStopRefused {
    /// `role` — `String`.
    pub role: String,
    /// `refusal` — `conductor.session.SessionStopRefusal`.
    pub refusal: SessionStopRefusal,
    /// `detail` — `String`.
    pub detail: String,
}

/// StartSession — `conductor.session.StartSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartSession {
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
}

/// StartedSession — `conductor.session.StartedSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartedSession {
    /// `role` — `String`.
    pub role: String,
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
    /// `provider` — `String`.
    pub provider: String,
    /// `unit` — `Optional<String>`.
    pub unit: Option<String>,
    /// `session_id` — `Optional<String>`.
    pub session_id: Option<String>,
}

/// StopSession — `conductor.session.StopSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopSession {
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
}

/// TickId — `conductor.session.TickId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TickId(pub crate::primitives::Uuid);

/// TickKind — `conductor.session.TickKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickKind {
    /// `watch`.
    Watch,
    /// `cycle`.
    Cycle,
    /// `daily`.
    Daily,
}

/// What Delivery — `conductor.session.Delivery` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Delivery<S>`], and at a boundary by [`DeliverySnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryData {
    /// The identity: `delivery_id` — `conductor.session.DeliveryId`.
    pub delivery_id: DeliveryId,
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
    /// `delivered_at` — `Timestamp`.
    pub delivered_at: crate::primitives::Timestamp,
}

/// The states of `conductor.session.Delivery`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](delivery_state::Marker), so [`Delivery<S>`](Delivery) can only ever rest in a real state.
pub mod delivery_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Delivered {}
    }

    /// A declared state of `Delivery`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DeliveryState;
    }

    /// `Delivered`. Where a new instance starts.
    pub struct Delivered;

    impl Marker for Delivered {
        const STATE: super::DeliveryState = super::DeliveryState::Delivered;
    }
}

/// Delivery — `conductor.session.Delivery` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Delivered`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DeliverySnapshot`]
/// and [`DeliverySnapshot::refine`].
pub struct Delivery<S: delivery_state::Marker> {
    data: DeliveryData,
    state: core::marker::PhantomData<S>,
}

impl<S: delivery_state::Marker> Delivery<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DeliveryState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DeliveryData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DeliveryData {
        self.data
    }
}

impl Delivery<delivery_state::Delivered> {
    /// A new instance, resting in `Delivered` — the only state the lifecycle starts one in.
    pub fn new(data: DeliveryData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.session.Delivery` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DeliverySnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverySnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DeliveryState,
    /// What it holds.
    pub data: DeliveryData,
}

/// An `Delivery` in whichever declared state it was found.
pub enum AnyDelivery {
    /// Resting in `Delivered`.
    Delivered(Delivery<delivery_state::Delivered>),
}

impl DeliverySnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DeliveryState` cannot spell one.
    pub fn refine(self) -> AnyDelivery {
        match self.state {
            DeliveryState::Delivered => AnyDelivery::Delivered(Delivery {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDelivery {
    /// The state, as the runtime value.
    pub fn state(&self) -> DeliveryState {
        match self {
            Self::Delivered(_) => DeliveryState::Delivered,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DeliverySnapshot {
        match self {
            Self::Delivered(instance) => DeliverySnapshot {
                state: DeliveryState::Delivered,
                data: instance.into_data(),
            },
        }
    }
}

/// What Lease — `conductor.session.Lease` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Lease<S>`], and at a boundary by [`LeaseSnapshot::state`].
///
/// Every value satisfies `pid > 0` — checked by [`LeaseData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseData {
    /// The identity: `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_at` — `Timestamp`.
    pub taken_at: crate::primitives::Timestamp,
    /// `taken_over_session_id` — `Optional<conductor.session.SessionId>`.
    pub taken_over_session_id: Option<SessionId>,
    /// `taken_over_pid` — `Optional<Integer>`.
    pub taken_over_pid: Option<i64>,
}

impl LeaseData {
    /// The first declared invariant of `conductor.session.Lease` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.pid)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("pid > 0");
        }
        None
    }
}

/// The states of `conductor.session.Lease`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](lease_state::Marker), so [`Lease<S>`](Lease) can only ever rest in a real state.
pub mod lease_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Held {}
    }

    /// A declared state of `Lease`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::LeaseState;
    }

    /// `Held`. Where a new instance starts.
    pub struct Held;

    impl Marker for Held {
        const STATE: super::LeaseState = super::LeaseState::Held;
    }
}

/// Lease — `conductor.session.Lease` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Held`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`LeaseSnapshot`]
/// and [`LeaseSnapshot::refine`].
pub struct Lease<S: lease_state::Marker> {
    data: LeaseData,
    state: core::marker::PhantomData<S>,
}

impl<S: lease_state::Marker> Lease<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> LeaseState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &LeaseData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> LeaseData {
        self.data
    }
}

impl Lease<lease_state::Held> {
    /// A new instance, resting in `Held` — the only state the lifecycle starts one in.
    pub fn new(data: LeaseData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.session.Lease` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`LeaseSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: LeaseState,
    /// What it holds.
    pub data: LeaseData,
}

/// An `Lease` in whichever declared state it was found.
pub enum AnyLease {
    /// Resting in `Held`.
    Held(Lease<lease_state::Held>),
}

impl LeaseSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `LeaseState` cannot spell one.
    pub fn refine(self) -> AnyLease {
        match self.state {
            LeaseState::Held => AnyLease::Held(Lease {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyLease {
    /// The state, as the runtime value.
    pub fn state(&self) -> LeaseState {
        match self {
            Self::Held(_) => LeaseState::Held,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> LeaseSnapshot {
        match self {
            Self::Held(instance) => LeaseSnapshot {
                state: LeaseState::Held,
                data: instance.into_data(),
            },
        }
    }
}

/// What RunTick — `conductor.session.RunTick` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`RunTick<S>`], and at a boundary by [`RunTickSnapshot::state`].
///
/// Every value satisfies `lines_handled >= 0` — checked by [`RunTickData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTickData {
    /// The identity: `tick_id` — `conductor.session.TickId`.
    pub tick_id: TickId,
    /// `kind` — `conductor.session.TickKind`.
    pub kind: TickKind,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `lines_handled` — `Integer`.
    pub lines_handled: i64,
    /// `actions` — `List<conductor.dispatch.DispatchLogEvent>`.
    pub actions: Vec<crate::dispatch::DispatchLogEvent>,
}

impl RunTickData {
    /// The first declared invariant of `conductor.session.RunTick` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.lines_handled)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("lines_handled >= 0");
        }
        None
    }
}

/// The states of `conductor.session.RunTick`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](run_tick_state::Marker), so [`RunTick<S>`](RunTick) can only ever rest in a real state.
pub mod run_tick_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `RunTick`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::RunTickState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::RunTickState = super::RunTickState::Recorded;
    }
}

/// RunTick — `conductor.session.RunTick` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`RunTickSnapshot`]
/// and [`RunTickSnapshot::refine`].
pub struct RunTick<S: run_tick_state::Marker> {
    data: RunTickData,
    state: core::marker::PhantomData<S>,
}

impl<S: run_tick_state::Marker> RunTick<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> RunTickState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &RunTickData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> RunTickData {
        self.data
    }
}

impl RunTick<run_tick_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: RunTickData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.session.RunTick` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`RunTickSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTickSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: RunTickState,
    /// What it holds.
    pub data: RunTickData,
}

/// An `RunTick` in whichever declared state it was found.
pub enum AnyRunTick {
    /// Resting in `Recorded`.
    Recorded(RunTick<run_tick_state::Recorded>),
}

impl RunTickSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `RunTickState` cannot spell one.
    pub fn refine(self) -> AnyRunTick {
        match self.state {
            RunTickState::Recorded => AnyRunTick::Recorded(RunTick {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyRunTick {
    /// The state, as the runtime value.
    pub fn state(&self) -> RunTickState {
        match self {
            Self::Recorded(_) => RunTickState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> RunTickSnapshot {
        match self {
            Self::Recorded(instance) => RunTickSnapshot {
                state: RunTickState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// Claim the conductor lease — the input of `conductor.session.ClaimLease`.
///
/// Everything it can result in is [`ClaimLeaseOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaimLease {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_at` — `Timestamp`.
    pub taken_at: crate::primitives::Timestamp,
}

/// Everything `conductor.session.ClaimLease` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClaimLeaseOutcome {
    /// `taken` — when, of the `conductor.session.Lease` rows satisfying `instance == input.instance`, no row is selected.
    ///
    /// No session held the instance's lease; the claiming one holds it now.
    Taken {
        /// The `conductor.session.LeaseTaken` this outcome publishes.
        lease_taken: LeaseTaken,
    },
    /// `renewed` — when, of the `conductor.session.Lease` rows satisfying `(instance == input.instance and pid == input.pid)`, at least one row is selected.
    ///
    /// The holder's own process claims again (a resumed, cleared or compacted session); it keeps the lease under the session id it gives now.
    Renewed {
        /// The `conductor.session.LeaseTaken` this outcome publishes.
        lease_taken: LeaseTaken,
    },
    /// `held` — externally decided (the process the lease names lives and is not the claiming one).
    ///
    /// A second conductor is refused, told who holds the lease, and must stop; the lease is unchanged.
    Held {
        /// Why it was refused: `conductor.session.LeaseHeld`.
        error: LeaseHeld,
    },
    /// `taken-over` — otherwise.
    ///
    /// The holder's process is gone, so the claiming session takes the lease and the lease records whom it was taken from.
    TakenOver {
        /// The `conductor.session.LeaseTakenOver` this outcome publishes.
        lease_taken_over: LeaseTakenOver,
    },
}

/// Deliver a message to a session — the input of `conductor.session.DeliverMessage`.
///
/// Everything it can result in is [`DeliverMessageOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliverMessage {
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
    /// `delivered_at` — `Timestamp`.
    pub delivered_at: crate::primitives::Timestamp,
}

/// Everything `conductor.session.DeliverMessage` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeliverMessageOutcome {
    /// `unreachable` — externally decided (no tmux server answers on the session's launcher socket).
    ///
    /// The session has no launcher socket; nothing was typed, nothing recorded, and the caller names the session and does not retry in a loop.
    Unreachable {
        /// Why it was refused: `conductor.session.SessionNotReachable`.
        error: SessionNotReachable,
    },
    /// `delivered` — otherwise.
    ///
    /// The text is in front of the session, and the delivery is on record with its time.
    Delivered {
        /// The `conductor.session.MessageDelivered` this outcome publishes.
        message_delivered: MessageDelivered,
    },
}

/// Record a tick of the run daemon — the input of `conductor.session.RecordRunTick`.
///
/// Everything it can result in is [`RecordRunTickOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordRunTick {
    /// `kind` — `conductor.session.TickKind`.
    pub kind: TickKind,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `lines_handled` — `Integer`.
    pub lines_handled: i64,
    /// `actions` — `List<conductor.dispatch.DispatchLogEvent>`.
    pub actions: Vec<crate::dispatch::DispatchLogEvent>,
}

/// Everything `conductor.session.RecordRunTick` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordRunTickOutcome {
    /// `negative-count` — when `lines_handled < 0`.
    ///
    /// A count of lines is never negative; nothing was recorded.
    NegativeCount {
        /// Why it was refused: `conductor.session.InvalidLineCount`.
        error: InvalidLineCount,
    },
    /// `recorded` — otherwise.
    ///
    /// The tick is on record with what it handled and the lines it wrote.
    Recorded {
        /// The `conductor.session.RunTickRecorded` this outcome publishes.
        run_tick_recorded: RunTickRecorded,
    },
}

/// LeaseTaken — the event `conductor.session.LeaseTaken`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseTaken {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
}

/// LeaseTakenOver — the event `conductor.session.LeaseTakenOver`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseTakenOver {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_over_session_id` — `conductor.session.SessionId`.
    pub taken_over_session_id: SessionId,
    /// `taken_over_pid` — `Integer`.
    pub taken_over_pid: i64,
}

/// MessageDelivered — the event `conductor.session.MessageDelivered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageDelivered {
    /// `delivery_id` — `conductor.session.DeliveryId`.
    pub delivery_id: DeliveryId,
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
    /// `delivered_at` — `Timestamp`.
    pub delivered_at: crate::primitives::Timestamp,
}

/// RunTickRecorded — the event `conductor.session.RunTickRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTickRecorded {
    /// `tick_id` — `conductor.session.TickId`.
    pub tick_id: TickId,
    /// `kind` — `conductor.session.TickKind`.
    pub kind: TickKind,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `lines_handled` — `Integer`.
    pub lines_handled: i64,
}

/// The declared error `conductor.session.InvalidLineCount`.
///
/// A count of lines handled is never negative, so no tick was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidLineCount {
    /// `lines_handled` — `Integer`.
    pub lines_handled: i64,
}

/// The declared error `conductor.session.LeaseHeld`.
///
/// Another session's live process holds the instance's lease: the claiming session is a second conductor and must stop. Nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeaseHeld {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
}

/// The declared error `conductor.session.SessionNotReachable`.
///
/// No tmux server answers on the session's launcher socket, so nothing was typed into it and nothing recorded; the caller is told which session, and does not retry in a loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionNotReachable {
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
}

/// Deliveries to sessions — one row of the view `conductor.session.Deliveries`.
///
/// Projects `conductor.session.Delivery` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deliveries {
    /// `delivery_id` — `conductor.session.DeliveryId`.
    pub delivery_id: DeliveryId,
    /// `state` — `conductor.session.Delivery.State`.
    pub state: DeliveryState,
    /// `session_name` — `conductor.config.SessionName`.
    pub session_name: crate::config::SessionName,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
    /// `delivered_at` — `Timestamp`.
    pub delivered_at: crate::primitives::Timestamp,
}

/// Conductor leases — one row of the view `conductor.session.Leases`.
///
/// Projects `conductor.session.Lease` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leases {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: crate::config::InstanceName,
    /// `state` — `conductor.session.Lease.State`.
    pub state: LeaseState,
    /// `session_id` — `conductor.session.SessionId`.
    pub session_id: SessionId,
    /// `pid` — `Integer`.
    pub pid: i64,
    /// `taken_at` — `Timestamp`.
    pub taken_at: crate::primitives::Timestamp,
    /// `taken_over_session_id` — `Optional<conductor.session.SessionId>`.
    pub taken_over_session_id: Option<SessionId>,
    /// `taken_over_pid` — `Optional<Integer>`.
    pub taken_over_pid: Option<i64>,
}

/// Ticks of the run daemon — one row of the view `conductor.session.RunTicks`.
///
/// Projects `conductor.session.RunTick` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTicks {
    /// `tick_id` — `conductor.session.TickId`.
    pub tick_id: TickId,
    /// `state` — `conductor.session.RunTick.State`.
    pub state: RunTickState,
    /// `kind` — `conductor.session.TickKind`.
    pub kind: TickKind,
    /// `started_at` — `Timestamp`.
    pub started_at: crate::primitives::Timestamp,
    /// `lines_handled` — `Integer`.
    pub lines_handled: i64,
    /// `actions` — `List<conductor.dispatch.DispatchLogEvent>`.
    pub actions: Vec<crate::dispatch::DispatchLogEvent>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.session.ClaimLease` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by an unknown identity, which reaches no declared outcome (neither `unknown_instance:` nor `wrong_state:`).
    ///
    /// Contract: given `conductor.session.ClaimLease` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `taken` when, of the `conductor.session.Lease` rows satisfying `instance == input.instance`, no row is selected, creates `conductor.session.Lease`, emits `conductor.session.LeaseTaken`; `renewed` when, of the `conductor.session.Lease` rows satisfying `(instance == input.instance and pid == input.pid)`, at least one row is selected, updates `conductor.session.Lease`, emits `conductor.session.LeaseTaken`; `held` externally decided (the process the lease names lives and is not the claiming one), error `conductor.session.LeaseHeld`; `taken-over` otherwise, updates `conductor.session.Lease`, emits `conductor.session.LeaseTakenOver`.
    pub trait ClaimLeaseBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.session.ClaimLease`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn claim_lease(&mut self, input: super::ClaimLease) -> Result<super::ClaimLeaseOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.session.DeliverMessage` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait DeliverMessageBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.session.DeliverMessage`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn deliver_message(&mut self, input: super::DeliverMessage) -> Result<super::DeliverMessageOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.session.RecordRunTick` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordRunTickBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.session.RecordRunTick`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_run_tick(&mut self, input: super::RecordRunTick) -> Result<super::RecordRunTickOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.session.Deliveries` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait DeliveriesQuery {
        /// Serves `conductor.session.Deliveries` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn deliveries(&self) -> Result<Vec<super::Deliveries>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.session.Leases` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait LeasesQuery {
        /// Serves `conductor.session.Leases` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn leases(&self) -> Result<Vec<super::Leases>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.session.RunTicks` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RunTicksQuery {
        /// Serves `conductor.session.RunTicks` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn run_ticks(&self) -> Result<Vec<super::RunTicks>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl ClaimLeaseBehavior for Unimplemented {
        fn claim_lease(&mut self, _input: super::ClaimLease) -> Result<super::ClaimLeaseOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.session.ClaimLease" })
        }
    }
}
