// generated from conductor v1
// model digest a61944b1d962028b0aff537f36be397359d5ec8be287a7db0368ad0bad816597
// contract digest cf5dcfb046d79cfcab8cd24240fae61c3aac1f945b760367fefff77c0d5a1bd1
// do not edit: regenerate with `ess synthesize --layout crate`

//! Dispatch — `conductor.dispatch`.
//!
//! How conductor directs the repository controllers. A controller is one session bound to one repository checkout, and at most one runs per repository. A dispatch is one brief to one controller in service of one goal, and it moves as the controller reports. Every message between conductor and a controller is recorded with its parsed first line, whichever harness and transport carried it. A resource request asks conductor for a build slot, disk or usage before a controller spends it. Design: docs/design/conductor.md § 3, § 6, § 7.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// The states of `conductor.dispatch.Controller`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Controller<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerState {
    /// `Paused`.
    Paused,
    /// `Running`.
    Running,
    /// `Stopped`.
    Stopped,
}

/// ControllerId — `conductor.dispatch.ControllerId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerId(pub crate::primitives::Uuid);

/// The states of `conductor.dispatch.Dispatch`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Dispatch<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DispatchState {
    /// `Blocked`.
    Blocked,
    /// `Cancelled`.
    Cancelled,
    /// `Done`.
    Done,
    /// `Failed`.
    Failed,
    /// `Sent`.
    Sent,
    /// `Started`.
    Started,
}

/// DispatchId — `conductor.dispatch.DispatchId`: a distinct wrapper around `String`.
///
/// Every value starts with `DSP-`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchId(pub String);

/// The states of `conductor.dispatch.GuardDecision`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `GuardDecision<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuardDecisionState {
    /// `Recorded`.
    Recorded,
}

/// GuardDecisionId — `conductor.dispatch.GuardDecisionId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDecisionId(pub crate::primitives::Uuid);

/// The states of `conductor.dispatch.Message`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Message<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageState {
    /// `Handled`.
    Handled,
    /// `Received`.
    Received,
    /// `Rejected`.
    Rejected,
}

/// MessageId — `conductor.dispatch.MessageId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageId(pub crate::primitives::Uuid);

/// MessageKind — `conductor.dispatch.MessageKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    /// `Dispatch`.
    Dispatch,
    /// `Report`.
    Report,
    /// `DecisionRequest`.
    DecisionRequest,
    /// `Decision`.
    Decision,
    /// `Need`.
    Need,
    /// `Resource`.
    Resource,
    /// `Escalation`.
    Escalation,
    /// `Unknown`.
    Unknown,
}

/// ResourceKind — `conductor.dispatch.ResourceKind`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceKind {
    /// `BuildSlot`.
    BuildSlot,
    /// `Disk`.
    Disk,
    /// `Usage`.
    Usage,
}

/// The states of `conductor.dispatch.ResourceRequest`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ResourceRequest<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceRequestState {
    /// `Granted`.
    Granted,
    /// `Refused`.
    Refused,
    /// `Released`.
    Released,
    /// `Requested`.
    Requested,
}

/// ResourceRequestId — `conductor.dispatch.ResourceRequestId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequestId(pub crate::primitives::Uuid);

/// Transport — `conductor.dispatch.Transport`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// `SendMessage`.
    SendMessage,
    /// `Mailbox`.
    Mailbox,
}

/// Verdict — `conductor.dispatch.Verdict`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// `Allow`.
    Allow,
    /// `Deny`.
    Deny,
}

/// What Controller — `conductor.dispatch.Controller` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Controller<S>`], and at a boundary by [`ControllerSnapshot::state`].
///
/// Every value satisfies `charter_revision > 0` — checked by [`ControllerData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerData {
    /// The identity: `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: crate::observation::Harness,
    /// `session_name` — `conductor.observation.RepositoryName`.
    pub session_name: crate::observation::RepositoryName,
    /// `charter_revision` — `Integer`.
    pub charter_revision: i64,
}

impl ControllerData {
    /// The first declared invariant of `conductor.dispatch.Controller` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.charter_revision)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("charter_revision > 0");
        }
        None
    }
}

/// The states of `conductor.dispatch.Controller`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](controller_state::Marker), so [`Controller<S>`](Controller) can only ever rest in a real state.
pub mod controller_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Paused {}
        impl Sealed for super::Running {}
        impl Sealed for super::Stopped {}
    }

    /// A declared state of `Controller`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ControllerState;
    }

    /// `Paused`.
    pub struct Paused;

    impl Marker for Paused {
        const STATE: super::ControllerState = super::ControllerState::Paused;
    }

    /// `Running`. Where a new instance starts.
    pub struct Running;

    impl Marker for Running {
        const STATE: super::ControllerState = super::ControllerState::Running;
    }

    /// `Stopped`. Terminal: an instance may rest here forever.
    pub struct Stopped;

    impl Marker for Stopped {
        const STATE: super::ControllerState = super::ControllerState::Stopped;
    }
}

/// Controller — `conductor.dispatch.Controller` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Running`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ControllerSnapshot`]
/// and [`ControllerSnapshot::refine`].
pub struct Controller<S: controller_state::Marker> {
    data: ControllerData,
    state: core::marker::PhantomData<S>,
}

impl<S: controller_state::Marker> Controller<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ControllerState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ControllerData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ControllerData {
        self.data
    }
}

impl Controller<controller_state::Running> {
    /// A new instance, resting in `Running` — the only state the lifecycle starts one in.
    pub fn new(data: ControllerData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Controller<controller_state::Paused> {
    /// `resume` — `Paused` → `Running`. Taken by the `resumed` outcome of `conductor.dispatch.ResumeController`.
    pub fn resume(self) -> Controller<controller_state::Running> {
        Controller {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stop` — `Paused` → `Stopped`. Taken by the `stopped` outcome of `conductor.dispatch.StopController`.
    pub fn stop(self) -> Controller<controller_state::Stopped> {
        Controller {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Controller<controller_state::Running> {
    /// `pause` — `Running` → `Paused`. Taken by the `paused` outcome of `conductor.dispatch.PauseController`.
    pub fn pause(self) -> Controller<controller_state::Paused> {
        Controller {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `stop` — `Running` → `Stopped`. Taken by the `stopped` outcome of `conductor.dispatch.StopController`.
    pub fn stop(self) -> Controller<controller_state::Stopped> {
        Controller {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.dispatch.Controller` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ControllerSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ControllerState,
    /// What it holds.
    pub data: ControllerData,
}

/// An `Controller` in whichever declared state it was found.
pub enum AnyController {
    /// Resting in `Paused`.
    Paused(Controller<controller_state::Paused>),
    /// Resting in `Running`.
    Running(Controller<controller_state::Running>),
    /// Resting in `Stopped`.
    Stopped(Controller<controller_state::Stopped>),
}

impl ControllerSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ControllerState` cannot spell one.
    pub fn refine(self) -> AnyController {
        match self.state {
            ControllerState::Paused => AnyController::Paused(Controller {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            ControllerState::Running => AnyController::Running(Controller {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            ControllerState::Stopped => AnyController::Stopped(Controller {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyController {
    /// The state, as the runtime value.
    pub fn state(&self) -> ControllerState {
        match self {
            Self::Paused(_) => ControllerState::Paused,
            Self::Running(_) => ControllerState::Running,
            Self::Stopped(_) => ControllerState::Stopped,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ControllerSnapshot {
        match self {
            Self::Paused(instance) => ControllerSnapshot {
                state: ControllerState::Paused,
                data: instance.into_data(),
            },
            Self::Running(instance) => ControllerSnapshot {
                state: ControllerState::Running,
                data: instance.into_data(),
            },
            Self::Stopped(instance) => ControllerSnapshot {
                state: ControllerState::Stopped,
                data: instance.into_data(),
            },
        }
    }
}

/// What Dispatch — `conductor.dispatch.Dispatch` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Dispatch<S>`], and at a boundary by [`DispatchSnapshot::state`].
///
/// Every value satisfies `priority >= 0` — checked by [`DispatchData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchData {
    /// The identity: `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `goal_id` — `conductor.direction.GoalId`.
    ///
    /// Carries `goal`: `conductor.dispatch.Dispatch` references one `conductor.direction.Goal`.
    pub goal_id: crate::direction::GoalId,
    /// `brief` — `String`.
    pub brief: String,
    /// `sent_at` — `Timestamp`.
    pub sent_at: crate::primitives::Timestamp,
    /// `priority` — `Integer`.
    pub priority: i64,
    /// `waits_on` — `Optional<conductor.dispatch.DispatchId>`.
    ///
    /// Carries `awaited_dispatch`: `conductor.dispatch.Dispatch` references one `conductor.dispatch.Dispatch`.
    pub waits_on: Option<DispatchId>,
}

impl DispatchData {
    /// The first declared invariant of `conductor.dispatch.Dispatch` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.priority)), iv::Op::Ge, iv::Fact::number("0"), false, true)) {
            return Some("priority >= 0");
        }
        None
    }
}

/// The states of `conductor.dispatch.Dispatch`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](dispatch_state::Marker), so [`Dispatch<S>`](Dispatch) can only ever rest in a real state.
pub mod dispatch_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Blocked {}
        impl Sealed for super::Cancelled {}
        impl Sealed for super::Done {}
        impl Sealed for super::Failed {}
        impl Sealed for super::Sent {}
        impl Sealed for super::Started {}
    }

    /// A declared state of `Dispatch`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DispatchState;
    }

    /// `Blocked`.
    pub struct Blocked;

    impl Marker for Blocked {
        const STATE: super::DispatchState = super::DispatchState::Blocked;
    }

    /// `Cancelled`. Terminal: an instance may rest here forever.
    pub struct Cancelled;

    impl Marker for Cancelled {
        const STATE: super::DispatchState = super::DispatchState::Cancelled;
    }

    /// `Done`. Terminal: an instance may rest here forever.
    pub struct Done;

    impl Marker for Done {
        const STATE: super::DispatchState = super::DispatchState::Done;
    }

    /// `Failed`. Terminal: an instance may rest here forever.
    pub struct Failed;

    impl Marker for Failed {
        const STATE: super::DispatchState = super::DispatchState::Failed;
    }

    /// `Sent`. Where a new instance starts.
    pub struct Sent;

    impl Marker for Sent {
        const STATE: super::DispatchState = super::DispatchState::Sent;
    }

    /// `Started`.
    pub struct Started;

    impl Marker for Started {
        const STATE: super::DispatchState = super::DispatchState::Started;
    }
}

/// Dispatch — `conductor.dispatch.Dispatch` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Sent`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DispatchSnapshot`]
/// and [`DispatchSnapshot::refine`].
pub struct Dispatch<S: dispatch_state::Marker> {
    data: DispatchData,
    state: core::marker::PhantomData<S>,
}

impl<S: dispatch_state::Marker> Dispatch<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DispatchState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DispatchData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DispatchData {
        self.data
    }
}

impl Dispatch<dispatch_state::Sent> {
    /// A new instance, resting in `Sent` — the only state the lifecycle starts one in.
    pub fn new(data: DispatchData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Dispatch<dispatch_state::Blocked> {
    /// `unblock` — `Blocked` → `Started`. Taken by the `unblocked` outcome of `conductor.dispatch.ReportUnblocked`.
    pub fn unblock(self) -> Dispatch<dispatch_state::Started> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `fail` — `Blocked` → `Failed`. Taken by the `failed` outcome of `conductor.dispatch.ReportFailed`.
    pub fn fail(self) -> Dispatch<dispatch_state::Failed> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `cancel` — `Blocked` → `Cancelled`. Taken by the `cancelled` outcome of `conductor.dispatch.CancelDispatch`.
    pub fn cancel(self) -> Dispatch<dispatch_state::Cancelled> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Dispatch<dispatch_state::Sent> {
    /// `start` — `Sent` → `Started`. Taken by the `started` outcome of `conductor.dispatch.ReportStarted`.
    pub fn start(self) -> Dispatch<dispatch_state::Started> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `cancel` — `Sent` → `Cancelled`. Taken by the `cancelled` outcome of `conductor.dispatch.CancelDispatch`.
    pub fn cancel(self) -> Dispatch<dispatch_state::Cancelled> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl Dispatch<dispatch_state::Started> {
    /// `block` — `Started` → `Blocked`. Taken by the `blocked` outcome of `conductor.dispatch.ReportBlocked`.
    pub fn block(self) -> Dispatch<dispatch_state::Blocked> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `finish` — `Started` → `Done`. Taken by the `done` outcome of `conductor.dispatch.ReportDone`.
    pub fn finish(self) -> Dispatch<dispatch_state::Done> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `fail` — `Started` → `Failed`. Taken by the `failed` outcome of `conductor.dispatch.ReportFailed`.
    pub fn fail(self) -> Dispatch<dispatch_state::Failed> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `cancel` — `Started` → `Cancelled`. Taken by the `cancelled` outcome of `conductor.dispatch.CancelDispatch`.
    pub fn cancel(self) -> Dispatch<dispatch_state::Cancelled> {
        Dispatch {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.dispatch.Dispatch` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DispatchSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DispatchState,
    /// What it holds.
    pub data: DispatchData,
}

/// An `Dispatch` in whichever declared state it was found.
pub enum AnyDispatch {
    /// Resting in `Blocked`.
    Blocked(Dispatch<dispatch_state::Blocked>),
    /// Resting in `Cancelled`.
    Cancelled(Dispatch<dispatch_state::Cancelled>),
    /// Resting in `Done`.
    Done(Dispatch<dispatch_state::Done>),
    /// Resting in `Failed`.
    Failed(Dispatch<dispatch_state::Failed>),
    /// Resting in `Sent`.
    Sent(Dispatch<dispatch_state::Sent>),
    /// Resting in `Started`.
    Started(Dispatch<dispatch_state::Started>),
}

impl DispatchSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DispatchState` cannot spell one.
    pub fn refine(self) -> AnyDispatch {
        match self.state {
            DispatchState::Blocked => AnyDispatch::Blocked(Dispatch {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DispatchState::Cancelled => AnyDispatch::Cancelled(Dispatch {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DispatchState::Done => AnyDispatch::Done(Dispatch {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DispatchState::Failed => AnyDispatch::Failed(Dispatch {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DispatchState::Sent => AnyDispatch::Sent(Dispatch {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DispatchState::Started => AnyDispatch::Started(Dispatch {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDispatch {
    /// The state, as the runtime value.
    pub fn state(&self) -> DispatchState {
        match self {
            Self::Blocked(_) => DispatchState::Blocked,
            Self::Cancelled(_) => DispatchState::Cancelled,
            Self::Done(_) => DispatchState::Done,
            Self::Failed(_) => DispatchState::Failed,
            Self::Sent(_) => DispatchState::Sent,
            Self::Started(_) => DispatchState::Started,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DispatchSnapshot {
        match self {
            Self::Blocked(instance) => DispatchSnapshot {
                state: DispatchState::Blocked,
                data: instance.into_data(),
            },
            Self::Cancelled(instance) => DispatchSnapshot {
                state: DispatchState::Cancelled,
                data: instance.into_data(),
            },
            Self::Done(instance) => DispatchSnapshot {
                state: DispatchState::Done,
                data: instance.into_data(),
            },
            Self::Failed(instance) => DispatchSnapshot {
                state: DispatchState::Failed,
                data: instance.into_data(),
            },
            Self::Sent(instance) => DispatchSnapshot {
                state: DispatchState::Sent,
                data: instance.into_data(),
            },
            Self::Started(instance) => DispatchSnapshot {
                state: DispatchState::Started,
                data: instance.into_data(),
            },
        }
    }
}

/// What GuardDecision — `conductor.dispatch.GuardDecision` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`GuardDecision<S>`], and at a boundary by [`GuardDecisionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDecisionData {
    /// The identity: `guard_decision_id` — `conductor.dispatch.GuardDecisionId`.
    pub guard_decision_id: GuardDecisionId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `session_ref` — `String`.
    pub session_ref: String,
    /// `tool` — `String`.
    pub tool: String,
    /// `target` — `String`.
    pub target: String,
    /// `verdict` — `conductor.dispatch.Verdict`.
    pub verdict: Verdict,
    /// `reason` — `String`.
    pub reason: String,
    /// `decided_at` — `Timestamp`.
    pub decided_at: crate::primitives::Timestamp,
}

/// The states of `conductor.dispatch.GuardDecision`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](guard_decision_state::Marker), so [`GuardDecision<S>`](GuardDecision) can only ever rest in a real state.
pub mod guard_decision_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Recorded {}
    }

    /// A declared state of `GuardDecision`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::GuardDecisionState;
    }

    /// `Recorded`. Where a new instance starts.
    pub struct Recorded;

    impl Marker for Recorded {
        const STATE: super::GuardDecisionState = super::GuardDecisionState::Recorded;
    }
}

/// GuardDecision — `conductor.dispatch.GuardDecision` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Recorded`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`GuardDecisionSnapshot`]
/// and [`GuardDecisionSnapshot::refine`].
pub struct GuardDecision<S: guard_decision_state::Marker> {
    data: GuardDecisionData,
    state: core::marker::PhantomData<S>,
}

impl<S: guard_decision_state::Marker> GuardDecision<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> GuardDecisionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &GuardDecisionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> GuardDecisionData {
        self.data
    }
}

impl GuardDecision<guard_decision_state::Recorded> {
    /// A new instance, resting in `Recorded` — the only state the lifecycle starts one in.
    pub fn new(data: GuardDecisionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.dispatch.GuardDecision` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`GuardDecisionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDecisionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: GuardDecisionState,
    /// What it holds.
    pub data: GuardDecisionData,
}

/// An `GuardDecision` in whichever declared state it was found.
pub enum AnyGuardDecision {
    /// Resting in `Recorded`.
    Recorded(GuardDecision<guard_decision_state::Recorded>),
}

impl GuardDecisionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `GuardDecisionState` cannot spell one.
    pub fn refine(self) -> AnyGuardDecision {
        match self.state {
            GuardDecisionState::Recorded => AnyGuardDecision::Recorded(GuardDecision {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyGuardDecision {
    /// The state, as the runtime value.
    pub fn state(&self) -> GuardDecisionState {
        match self {
            Self::Recorded(_) => GuardDecisionState::Recorded,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> GuardDecisionSnapshot {
        match self {
            Self::Recorded(instance) => GuardDecisionSnapshot {
                state: GuardDecisionState::Recorded,
                data: instance.into_data(),
            },
        }
    }
}

/// What Message — `conductor.dispatch.Message` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Message<S>`], and at a boundary by [`MessageSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageData {
    /// The identity: `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
    /// `sender` — `String`.
    pub sender: String,
    /// `recipient` — `String`.
    pub recipient: String,
    /// `kind` — `conductor.dispatch.MessageKind`.
    pub kind: MessageKind,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `transport` — `conductor.dispatch.Transport`.
    pub transport: Transport,
    /// `received_at` — `Timestamp`.
    pub received_at: crate::primitives::Timestamp,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
    /// `dispatch_id` — `Optional<conductor.dispatch.DispatchId>`.
    ///
    /// Carries `opened_dispatch`: `conductor.dispatch.Message` references one `conductor.dispatch.Dispatch`.
    pub dispatch_id: Option<DispatchId>,
}

/// The states of `conductor.dispatch.Message`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](message_state::Marker), so [`Message<S>`](Message) can only ever rest in a real state.
pub mod message_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Handled {}
        impl Sealed for super::Received {}
        impl Sealed for super::Rejected {}
    }

    /// A declared state of `Message`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::MessageState;
    }

    /// `Handled`. Terminal: an instance may rest here forever.
    pub struct Handled;

    impl Marker for Handled {
        const STATE: super::MessageState = super::MessageState::Handled;
    }

    /// `Received`. Where a new instance starts.
    pub struct Received;

    impl Marker for Received {
        const STATE: super::MessageState = super::MessageState::Received;
    }

    /// `Rejected`. Terminal: an instance may rest here forever.
    pub struct Rejected;

    impl Marker for Rejected {
        const STATE: super::MessageState = super::MessageState::Rejected;
    }
}

/// Message — `conductor.dispatch.Message` — with its lifecycle state carried by the type.
///
/// Its constructors rest in the states a creation lands in: `Received`, `Rejected`.
/// And the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`MessageSnapshot`]
/// and [`MessageSnapshot::refine`].
pub struct Message<S: message_state::Marker> {
    data: MessageData,
    state: core::marker::PhantomData<S>,
}

impl<S: message_state::Marker> Message<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> MessageState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &MessageData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> MessageData {
        self.data
    }
}

impl Message<message_state::Received> {
    /// A new instance, resting in `Received` — where a creation naming no `into:` lands it.
    pub fn new(data: MessageData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Message<message_state::Rejected> {
    /// A new instance, resting in `Rejected` — where a creation declared `into: Rejected` lands it.
    pub fn new_rejected(data: MessageData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Message<message_state::Received> {
    /// `handle` — `Received` → `Handled`. Taken by the `handled` outcome of `conductor.dispatch.HandleMessage`, the `routed` outcome of `conductor.dispatch.RouteNeed`.
    pub fn handle(self) -> Message<message_state::Handled> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `reject` — `Received` → `Rejected`. Taken by the `rejected` outcome of `conductor.dispatch.RejectMessage`.
    pub fn reject(self) -> Message<message_state::Rejected> {
        Message {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.dispatch.Message` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`MessageSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: MessageState,
    /// What it holds.
    pub data: MessageData,
}

/// An `Message` in whichever declared state it was found.
pub enum AnyMessage {
    /// Resting in `Handled`.
    Handled(Message<message_state::Handled>),
    /// Resting in `Received`.
    Received(Message<message_state::Received>),
    /// Resting in `Rejected`.
    Rejected(Message<message_state::Rejected>),
}

impl MessageSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `MessageState` cannot spell one.
    pub fn refine(self) -> AnyMessage {
        match self.state {
            MessageState::Handled => AnyMessage::Handled(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MessageState::Received => AnyMessage::Received(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            MessageState::Rejected => AnyMessage::Rejected(Message {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyMessage {
    /// The state, as the runtime value.
    pub fn state(&self) -> MessageState {
        match self {
            Self::Handled(_) => MessageState::Handled,
            Self::Received(_) => MessageState::Received,
            Self::Rejected(_) => MessageState::Rejected,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> MessageSnapshot {
        match self {
            Self::Handled(instance) => MessageSnapshot {
                state: MessageState::Handled,
                data: instance.into_data(),
            },
            Self::Received(instance) => MessageSnapshot {
                state: MessageState::Received,
                data: instance.into_data(),
            },
            Self::Rejected(instance) => MessageSnapshot {
                state: MessageState::Rejected,
                data: instance.into_data(),
            },
        }
    }
}

/// What ResourceRequest — `conductor.dispatch.ResourceRequest` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ResourceRequest<S>`], and at a boundary by [`ResourceRequestSnapshot::state`].
///
/// Every value satisfies `amount > 0` — checked by [`ResourceRequestData::broken_invariant`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequestData {
    /// The identity: `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `resource` — `conductor.dispatch.ResourceKind`.
    pub resource: ResourceKind,
    /// `amount` — `Integer`.
    pub amount: i64,
    /// `requested_at` — `Timestamp`.
    pub requested_at: crate::primitives::Timestamp,
}

impl ResourceRequestData {
    /// The first declared invariant of `conductor.dispatch.ResourceRequest` this value breaks, as the specification declares it,
    /// or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value. One that reads something
    /// absent — an empty `Optional`, a list position past the end, or `state`, which this
    /// type does not hold — decides nothing, as the conformance interpreter reads it.
    pub fn broken_invariant(&self) -> Option<&'static str> {
        use crate::primitives::invariant as iv;
        if iv::broken(iv::compare(Some(iv::Fact::integer(self.amount)), iv::Op::Gt, iv::Fact::number("0"), false, true)) {
            return Some("amount > 0");
        }
        None
    }
}

/// The states of `conductor.dispatch.ResourceRequest`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](resource_request_state::Marker), so [`ResourceRequest<S>`](ResourceRequest) can only ever rest in a real state.
pub mod resource_request_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Granted {}
        impl Sealed for super::Refused {}
        impl Sealed for super::Released {}
        impl Sealed for super::Requested {}
    }

    /// A declared state of `ResourceRequest`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ResourceRequestState;
    }

    /// `Granted`.
    pub struct Granted;

    impl Marker for Granted {
        const STATE: super::ResourceRequestState = super::ResourceRequestState::Granted;
    }

    /// `Refused`. Terminal: an instance may rest here forever.
    pub struct Refused;

    impl Marker for Refused {
        const STATE: super::ResourceRequestState = super::ResourceRequestState::Refused;
    }

    /// `Released`. Terminal: an instance may rest here forever.
    pub struct Released;

    impl Marker for Released {
        const STATE: super::ResourceRequestState = super::ResourceRequestState::Released;
    }

    /// `Requested`. Where a new instance starts.
    pub struct Requested;

    impl Marker for Requested {
        const STATE: super::ResourceRequestState = super::ResourceRequestState::Requested;
    }
}

/// ResourceRequest — `conductor.dispatch.ResourceRequest` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Requested`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ResourceRequestSnapshot`]
/// and [`ResourceRequestSnapshot::refine`].
pub struct ResourceRequest<S: resource_request_state::Marker> {
    data: ResourceRequestData,
    state: core::marker::PhantomData<S>,
}

impl<S: resource_request_state::Marker> ResourceRequest<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ResourceRequestState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ResourceRequestData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ResourceRequestData {
        self.data
    }
}

impl ResourceRequest<resource_request_state::Requested> {
    /// A new instance, resting in `Requested` — the only state the lifecycle starts one in.
    pub fn new(data: ResourceRequestData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl ResourceRequest<resource_request_state::Granted> {
    /// `release` — `Granted` → `Released`. Taken by the `released` outcome of `conductor.dispatch.ReleaseResource`.
    pub fn release(self) -> ResourceRequest<resource_request_state::Released> {
        ResourceRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl ResourceRequest<resource_request_state::Requested> {
    /// `grant` — `Requested` → `Granted`. Taken by the `granted` outcome of `conductor.dispatch.GrantResource`.
    pub fn grant(self) -> ResourceRequest<resource_request_state::Granted> {
        ResourceRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `refuse` — `Requested` → `Refused`. Taken by the `refused` outcome of `conductor.dispatch.RefuseResource`.
    pub fn refuse(self) -> ResourceRequest<resource_request_state::Refused> {
        ResourceRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.dispatch.ResourceRequest` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ResourceRequestSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequestSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ResourceRequestState,
    /// What it holds.
    pub data: ResourceRequestData,
}

/// An `ResourceRequest` in whichever declared state it was found.
pub enum AnyResourceRequest {
    /// Resting in `Granted`.
    Granted(ResourceRequest<resource_request_state::Granted>),
    /// Resting in `Refused`.
    Refused(ResourceRequest<resource_request_state::Refused>),
    /// Resting in `Released`.
    Released(ResourceRequest<resource_request_state::Released>),
    /// Resting in `Requested`.
    Requested(ResourceRequest<resource_request_state::Requested>),
}

impl ResourceRequestSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ResourceRequestState` cannot spell one.
    pub fn refine(self) -> AnyResourceRequest {
        match self.state {
            ResourceRequestState::Granted => AnyResourceRequest::Granted(ResourceRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            ResourceRequestState::Refused => AnyResourceRequest::Refused(ResourceRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            ResourceRequestState::Released => AnyResourceRequest::Released(ResourceRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            ResourceRequestState::Requested => AnyResourceRequest::Requested(ResourceRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyResourceRequest {
    /// The state, as the runtime value.
    pub fn state(&self) -> ResourceRequestState {
        match self {
            Self::Granted(_) => ResourceRequestState::Granted,
            Self::Refused(_) => ResourceRequestState::Refused,
            Self::Released(_) => ResourceRequestState::Released,
            Self::Requested(_) => ResourceRequestState::Requested,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ResourceRequestSnapshot {
        match self {
            Self::Granted(instance) => ResourceRequestSnapshot {
                state: ResourceRequestState::Granted,
                data: instance.into_data(),
            },
            Self::Refused(instance) => ResourceRequestSnapshot {
                state: ResourceRequestState::Refused,
                data: instance.into_data(),
            },
            Self::Released(instance) => ResourceRequestSnapshot {
                state: ResourceRequestState::Released,
                data: instance.into_data(),
            },
            Self::Requested(instance) => ResourceRequestSnapshot {
                state: ResourceRequestState::Requested,
                data: instance.into_data(),
            },
        }
    }
}

/// Cancel a dispatch — the input of `conductor.dispatch.CancelDispatch`.
///
/// Everything it can result in is [`CancelDispatchOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CancelDispatch {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.dispatch.CancelDispatch` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CancelDispatchOutcome {
    /// `cancelled` — otherwise.
    ///
    /// Conductor withdrew the brief.
    Cancelled {
        /// The `conductor.dispatch.DispatchCancelled` this outcome publishes.
        dispatch_cancelled: DispatchCancelled,
    },
    /// `no-such-dispatch` — for an identity no record carries.
    NoSuchDispatch {
        /// Why it was refused: `conductor.dispatch.DispatchNotFound`.
        error: DispatchNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The dispatch already ended.
    WrongState {
        /// Why it was refused: `conductor.dispatch.DispatchStateConflict`.
        error: DispatchStateConflict,
    },
}

/// Grant a resource — the input of `conductor.dispatch.GrantResource`.
///
/// Everything it can result in is [`GrantResourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrantResource {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
    /// `disk_free_bytes` — `Integer`.
    pub disk_free_bytes: i64,
}

/// Everything `conductor.dispatch.GrantResource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrantResourceOutcome {
    /// `below-disk-floor` — when the existing subject's stored fields satisfy `(resource == BuildSlot and state == Requested)` and `disk_free_bytes < 32212254720`.
    ///
    /// A build slot is not granted under 30G free; the request stays open.
    BelowDiskFloor {
        /// Why it was refused: `conductor.dispatch.DiskBelowFloor`.
        error: DiskBelowFloor,
    },
    /// `granted` — otherwise.
    ///
    /// The controller may spend it.
    Granted {
        /// The `conductor.dispatch.ResourceGranted` this outcome publishes.
        resource_granted: ResourceGranted,
    },
    /// `no-such-request` — for an identity no record carries.
    NoSuchRequest {
        /// Why it was refused: `conductor.dispatch.ResourceRequestNotFound`.
        error: ResourceRequestNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The request was already answered.
    WrongState {
        /// Why it was refused: `conductor.dispatch.ResourceRequestStateConflict`.
        error: ResourceRequestStateConflict,
    },
}

/// Handle a message — the input of `conductor.dispatch.HandleMessage`.
///
/// Everything it can result in is [`HandleMessageOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleMessage {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
}

/// Everything `conductor.dispatch.HandleMessage` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandleMessageOutcome {
    /// `need-unrouted` — when the existing subject's stored fields satisfy `(kind == Need and state == Received)`.
    ///
    /// A `[NEED]` is closed only by routing it to the dispatch it opened (`RouteNeed`); the message stays where it was.
    NeedUnrouted {
        /// Why it was refused: `conductor.dispatch.NeedNotRouted`.
        error: NeedNotRouted,
    },
    /// `handled` — otherwise.
    ///
    /// The message was acted on.
    Handled {
        /// The `conductor.dispatch.MessageHandled` this outcome publishes.
        message_handled: MessageHandled,
    },
    /// `no-such-message` — for an identity no record carries.
    NoSuchMessage {
        /// Why it was refused: `conductor.dispatch.MessageNotFound`.
        error: MessageNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The message was already handled or rejected.
    WrongState {
        /// Why it was refused: `conductor.dispatch.MessageStateConflict`.
        error: MessageStateConflict,
    },
}

/// Pause a controller — the input of `conductor.dispatch.PauseController`.
///
/// Everything it can result in is [`PauseControllerOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PauseController {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// Everything `conductor.dispatch.PauseController` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PauseControllerOutcome {
    /// `paused` — otherwise.
    ///
    /// The controller holds its work and still counts as the repository's one controller.
    Paused {
        /// The `conductor.dispatch.ControllerPaused` this outcome publishes.
        controller_paused: ControllerPaused,
    },
    /// `no-such-controller` — for an identity no record carries.
    NoSuchController {
        /// Why it was refused: `conductor.dispatch.ControllerNotFound`.
        error: ControllerNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a running controller is paused.
    WrongState {
        /// Why it was refused: `conductor.dispatch.ControllerStateConflict`.
        error: ControllerStateConflict,
    },
}

/// Receive a message — the input of `conductor.dispatch.ReceiveMessage`.
///
/// Everything it can result in is [`ReceiveMessageOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiveMessage {
    /// `sender` — `String`.
    pub sender: String,
    /// `recipient` — `String`.
    pub recipient: String,
    /// `kind` — `conductor.dispatch.MessageKind`.
    pub kind: MessageKind,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `transport` — `conductor.dispatch.Transport`.
    pub transport: Transport,
    /// `received_at` — `Timestamp`.
    pub received_at: crate::primitives::Timestamp,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
    /// `parsed` — `Boolean`.
    pub parsed: bool,
}

/// Everything `conductor.dispatch.ReceiveMessage` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiveMessageOutcome {
    /// `unparsed` — when `(parsed == false or kind == Unknown)`.
    ///
    /// The first line did not parse (AGENTS.md rule 4), or names no kind, so the message is recorded as rejected and never acted on; the sender gets the format back.
    Unparsed {
        /// The `conductor.dispatch.MessageReceived` this outcome publishes.
        message_received: MessageReceived,
    },
    /// `received` — otherwise.
    ///
    /// The message is recorded with its parsed first line.
    Received {
        /// The `conductor.dispatch.MessageReceived` this outcome publishes.
        message_received: MessageReceived,
    },
}

/// Record a guard decision — the input of `conductor.dispatch.RecordGuardDecision`.
///
/// Everything it can result in is [`RecordGuardDecisionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordGuardDecision {
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `session_ref` — `String`.
    pub session_ref: String,
    /// `tool` — `String`.
    pub tool: String,
    /// `target` — `String`.
    pub target: String,
    /// `verdict` — `conductor.dispatch.Verdict`.
    pub verdict: Verdict,
    /// `reason` — `String`.
    pub reason: String,
    /// `decided_at` — `Timestamp`.
    pub decided_at: crate::primitives::Timestamp,
}

/// Everything `conductor.dispatch.RecordGuardDecision` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordGuardDecisionOutcome {
    /// `recorded` — otherwise.
    ///
    /// The hook's verdict on one tool call is recorded before the call proceeds or is blocked.
    Recorded {
        /// The `conductor.dispatch.GuardDecisionRecorded` this outcome publishes.
        guard_decision_recorded: GuardDecisionRecorded,
    },
}

/// Refuse a resource — the input of `conductor.dispatch.RefuseResource`.
///
/// Everything it can result in is [`RefuseResourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefuseResource {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.dispatch.RefuseResource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefuseResourceOutcome {
    /// `refused` — otherwise.
    ///
    /// The controller does not spend it, and reports blocked if it needs it.
    Refused {
        /// The `conductor.dispatch.ResourceRefused` this outcome publishes.
        resource_refused: ResourceRefused,
    },
    /// `no-such-request` — for an identity no record carries.
    NoSuchRequest {
        /// Why it was refused: `conductor.dispatch.ResourceRequestNotFound`.
        error: ResourceRequestNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The request was already answered.
    WrongState {
        /// Why it was refused: `conductor.dispatch.ResourceRequestStateConflict`.
        error: ResourceRequestStateConflict,
    },
}

/// Reject a message — the input of `conductor.dispatch.RejectMessage`.
///
/// Everything it can result in is [`RejectMessageOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RejectMessage {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.dispatch.RejectMessage` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectMessageOutcome {
    /// `rejected` — otherwise.
    ///
    /// Conductor refused a message it had received and recorded, and the sender gets the reason. An unparsed first line is rejected at receipt (`ReceiveMessage` `unparsed`), not here. External senders are accepted and routed (design § 3).
    Rejected {
        /// The `conductor.dispatch.MessageRejected` this outcome publishes.
        message_rejected: MessageRejected,
    },
    /// `no-such-message` — for an identity no record carries.
    NoSuchMessage {
        /// Why it was refused: `conductor.dispatch.MessageNotFound`.
        error: MessageNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The message was already handled or rejected.
    WrongState {
        /// Why it was refused: `conductor.dispatch.MessageStateConflict`.
        error: MessageStateConflict,
    },
}

/// Release a granted resource — the input of `conductor.dispatch.ReleaseResource`.
///
/// Everything it can result in is [`ReleaseResourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleaseResource {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
}

/// Everything `conductor.dispatch.ReleaseResource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReleaseResourceOutcome {
    /// `released` — otherwise.
    ///
    /// The build slot, disk or usage is back in the ledger for the next request.
    Released {
        /// The `conductor.dispatch.ResourceReleased` this outcome publishes.
        resource_released: ResourceReleased,
    },
    /// `no-such-request` — for an identity no record carries.
    NoSuchRequest {
        /// Why it was refused: `conductor.dispatch.ResourceRequestNotFound`.
        error: ResourceRequestNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a granted request is released.
    WrongState {
        /// Why it was refused: `conductor.dispatch.ResourceRequestStateConflict`.
        error: ResourceRequestStateConflict,
    },
}

/// Report a dispatch blocked — the input of `conductor.dispatch.ReportBlocked`.
///
/// Everything it can result in is [`ReportBlockedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBlocked {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.dispatch.ReportBlocked` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportBlockedOutcome {
    /// `blocked` — otherwise.
    ///
    /// The controller stopped working and waits; waiting is not work (design P13).
    Blocked {
        /// The `conductor.dispatch.DispatchBlocked` this outcome publishes.
        dispatch_blocked: DispatchBlocked,
    },
    /// `no-such-dispatch` — for an identity no record carries.
    NoSuchDispatch {
        /// Why it was refused: `conductor.dispatch.DispatchNotFound`.
        error: DispatchNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a started dispatch blocks.
    WrongState {
        /// Why it was refused: `conductor.dispatch.DispatchStateConflict`.
        error: DispatchStateConflict,
    },
}

/// Report a dispatch done — the input of `conductor.dispatch.ReportDone`.
///
/// Everything it can result in is [`ReportDoneOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportDone {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `evidence` — `String`.
    pub evidence: String,
}

/// Everything `conductor.dispatch.ReportDone` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportDoneOutcome {
    /// `done` — otherwise.
    ///
    /// The dispatch's acceptance holds on origin/main, with the evidence lines.
    Done {
        /// The `conductor.dispatch.DispatchDone` this outcome publishes.
        dispatch_done: DispatchDone,
    },
    /// `no-such-dispatch` — for an identity no record carries.
    NoSuchDispatch {
        /// Why it was refused: `conductor.dispatch.DispatchNotFound`.
        error: DispatchNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a started dispatch is done.
    WrongState {
        /// Why it was refused: `conductor.dispatch.DispatchStateConflict`.
        error: DispatchStateConflict,
    },
}

/// Report a dispatch failed — the input of `conductor.dispatch.ReportFailed`.
///
/// Everything it can result in is [`ReportFailedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportFailed {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.dispatch.ReportFailed` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportFailedOutcome {
    /// `failed` — otherwise.
    ///
    /// The dispatch cannot reach its acceptance.
    Failed {
        /// The `conductor.dispatch.DispatchFailed` this outcome publishes.
        dispatch_failed: DispatchFailed,
    },
    /// `no-such-dispatch` — for an identity no record carries.
    NoSuchDispatch {
        /// Why it was refused: `conductor.dispatch.DispatchNotFound`.
        error: DispatchNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a started or blocked dispatch fails.
    WrongState {
        /// Why it was refused: `conductor.dispatch.DispatchStateConflict`.
        error: DispatchStateConflict,
    },
}

/// Report a dispatch started — the input of `conductor.dispatch.ReportStarted`.
///
/// Everything it can result in is [`ReportStartedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportStarted {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// Everything `conductor.dispatch.ReportStarted` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportStartedOutcome {
    /// `started` — otherwise.
    ///
    /// The controller is working the dispatch.
    Started {
        /// The `conductor.dispatch.DispatchStarted` this outcome publishes.
        dispatch_started: DispatchStarted,
    },
    /// `no-such-dispatch` — for an identity no record carries.
    NoSuchDispatch {
        /// Why it was refused: `conductor.dispatch.DispatchNotFound`.
        error: DispatchNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a sent dispatch starts.
    WrongState {
        /// Why it was refused: `conductor.dispatch.DispatchStateConflict`.
        error: DispatchStateConflict,
    },
}

/// Report a dispatch unblocked — the input of `conductor.dispatch.ReportUnblocked`.
///
/// Everything it can result in is [`ReportUnblockedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportUnblocked {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// Everything `conductor.dispatch.ReportUnblocked` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReportUnblockedOutcome {
    /// `unblocked` — otherwise.
    ///
    /// The controller works the dispatch again.
    Unblocked {
        /// The `conductor.dispatch.DispatchUnblocked` this outcome publishes.
        dispatch_unblocked: DispatchUnblocked,
    },
    /// `no-such-dispatch` — for an identity no record carries.
    NoSuchDispatch {
        /// Why it was refused: `conductor.dispatch.DispatchNotFound`.
        error: DispatchNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a blocked dispatch is unblocked.
    WrongState {
        /// Why it was refused: `conductor.dispatch.DispatchStateConflict`.
        error: DispatchStateConflict,
    },
}

/// Request a resource — the input of `conductor.dispatch.RequestResource`.
///
/// Everything it can result in is [`RequestResourceOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestResource {
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `resource` — `conductor.dispatch.ResourceKind`.
    pub resource: ResourceKind,
    /// `amount` — `Integer`.
    pub amount: i64,
    /// `requested_at` — `Timestamp`.
    pub requested_at: crate::primitives::Timestamp,
}

/// Everything `conductor.dispatch.RequestResource` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestResourceOutcome {
    /// `invalid-amount` — when `amount <= 0`.
    ///
    /// The amount is not positive, and nothing was requested.
    InvalidAmount {
        /// Why it was refused: `conductor.dispatch.InvalidAmount`.
        error: InvalidAmount,
    },
    /// `requested` — otherwise.
    ///
    /// The controller waits for the grant before spending it.
    Requested {
        /// The `conductor.dispatch.ResourceRequested` this outcome publishes.
        resource_requested: ResourceRequested,
    },
}

/// Resume a controller — the input of `conductor.dispatch.ResumeController`.
///
/// Everything it can result in is [`ResumeControllerOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResumeController {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// Everything `conductor.dispatch.ResumeController` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResumeControllerOutcome {
    /// `resumed` — otherwise.
    ///
    /// The controller runs again.
    Resumed {
        /// The `conductor.dispatch.ControllerResumed` this outcome publishes.
        controller_resumed: ControllerResumed,
    },
    /// `no-such-controller` — for an identity no record carries.
    NoSuchController {
        /// Why it was refused: `conductor.dispatch.ControllerNotFound`.
        error: ControllerNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only a paused controller is resumed.
    WrongState {
        /// Why it was refused: `conductor.dispatch.ControllerStateConflict`.
        error: ControllerStateConflict,
    },
}

/// Revise a charter — the input of `conductor.dispatch.ReviseCharter`.
///
/// Everything it can result in is [`ReviseCharterOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviseCharter {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// Everything `conductor.dispatch.ReviseCharter` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReviseCharterOutcome {
    /// `revised` — when the existing subject is in Running.
    ///
    /// The controller received the next revision of its charter.
    Revised {
        /// The `conductor.dispatch.CharterRevised` this outcome publishes.
        charter_revised: CharterRevised,
    },
    /// `paused-revised` — when the existing subject is in Paused.
    ///
    /// A paused controller reads the new revision when it resumes.
    PausedRevised {
        /// The `conductor.dispatch.CharterRevised` this outcome publishes.
        charter_revised: CharterRevised,
    },
    /// `no-such-controller` — for an identity no record carries.
    NoSuchController {
        /// Why it was refused: `conductor.dispatch.ControllerNotFound`.
        error: ControllerNotFound,
    },
    /// `stopped` — otherwise.
    ///
    /// A stopped controller takes no charter.
    Stopped {
        /// Why it was refused: `conductor.dispatch.ControllerStateConflict`.
        error: ControllerStateConflict,
    },
}

/// Route a need to a dispatch — the input of `conductor.dispatch.RouteNeed`.
///
/// Everything it can result in is [`RouteNeedOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteNeed {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// Everything `conductor.dispatch.RouteNeed` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteNeedOutcome {
    /// `not-a-need` — when the existing subject's stored fields satisfy `(kind != Need and state == Received)`.
    ///
    /// Only a `[NEED]` message is routed to a dispatch; the message stays where it was.
    NotANeed {
        /// Why it was refused: `conductor.dispatch.NotANeed`.
        error: NotANeed,
    },
    /// `routed` — otherwise.
    ///
    /// The need is handled, and the message records the dispatch it opened.
    Routed {
        /// The `conductor.dispatch.NeedRouted` this outcome publishes.
        need_routed: NeedRouted,
    },
    /// `no-such-message` — for an identity no record carries.
    NoSuchMessage {
        /// Why it was refused: `conductor.dispatch.MessageNotFound`.
        error: MessageNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The message was already handled or rejected.
    WrongState {
        /// Why it was refused: `conductor.dispatch.MessageStateConflict`.
        error: MessageStateConflict,
    },
}

/// Send a dispatch — the input of `conductor.dispatch.SendDispatch`.
///
/// Everything it can result in is [`SendDispatchOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SendDispatch {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: crate::direction::GoalId,
    /// `brief` — `String`.
    pub brief: String,
    /// `sent_at` — `Timestamp`.
    pub sent_at: crate::primitives::Timestamp,
    /// `priority` — `Integer`.
    pub priority: i64,
    /// `waits_on` — `Optional<conductor.dispatch.DispatchId>`.
    pub waits_on: Option<DispatchId>,
}

/// Everything `conductor.dispatch.SendDispatch` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendDispatchOutcome {
    /// `negative-priority` — when `priority < 0`.
    ///
    /// A priority is never negative; nothing was sent.
    NegativePriority {
        /// Why it was refused: `conductor.dispatch.InvalidPriority`.
        error: InvalidPriority,
    },
    /// `waits-on-itself` — when `(defined(waits_on) and waits_on == {fact: dispatch_id})`.
    ///
    /// A dispatch that waits on itself could never start; nothing was sent.
    WaitsOnItself {
        /// Why it was refused: `conductor.dispatch.WaitsOnItself`.
        error: WaitsOnItself,
    },
    /// `no-such-goal` — when no `conductor.direction.Goal` carries the identity `input.goal_id` names.
    ///
    /// Every dispatch serves a goal (design § 4); nothing was sent.
    NoSuchGoal {
        /// Why it was refused: `conductor.dispatch.GoalNotFound`.
        error: GoalNotFound,
    },
    /// `goal-closed` — when the `conductor.direction.Goal` that `input.goal_id` names satisfies `state in [Met, Dropped]`.
    ///
    /// The goal is met or dropped; nothing was sent.
    GoalClosed {
        /// Why it was refused: `conductor.dispatch.GoalClosed`.
        error: GoalClosed,
    },
    /// `sent` — otherwise.
    ///
    /// The controller has the brief.
    Sent {
        /// The `conductor.dispatch.DispatchSent` this outcome publishes.
        dispatch_sent: DispatchSent,
    },
    /// `already-sent` — for an identity a record already carries.
    ///
    /// A dispatch already has this id; the held dispatch is unchanged.
    AlreadySent {
        /// Why it was refused: `conductor.dispatch.DispatchExists`.
        error: DispatchExists,
    },
}

/// Start a controller — the input of `conductor.dispatch.StartController`.
///
/// Everything it can result in is [`StartControllerOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartController {
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: crate::observation::Harness,
}

/// Everything `conductor.dispatch.StartController` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartControllerOutcome {
    /// `already-running` — when, of the `conductor.dispatch.Controller` rows satisfying `(repository == input.repository and state in [Running, Paused])`, at least 1 rows are selected.
    ///
    /// One controller per repository (design § 3); nothing started.
    AlreadyRunning {
        /// Why it was refused: `conductor.dispatch.ControllerAlreadyRunning`.
        error: ControllerAlreadyRunning,
    },
    /// `started` — otherwise.
    ///
    /// The controller runs in its repository's checkout under its charter.
    Started {
        /// The `conductor.dispatch.ControllerStarted` this outcome publishes.
        controller_started: ControllerStarted,
    },
}

/// Stop a controller — the input of `conductor.dispatch.StopController`.
///
/// Everything it can result in is [`StopControllerOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StopController {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.dispatch.StopController` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopControllerOutcome {
    /// `stopped` — otherwise.
    ///
    /// The controller session has ended; the repository may get a new one.
    Stopped {
        /// The `conductor.dispatch.ControllerStopped` this outcome publishes.
        controller_stopped: ControllerStopped,
    },
    /// `no-such-controller` — for an identity no record carries.
    NoSuchController {
        /// Why it was refused: `conductor.dispatch.ControllerNotFound`.
        error: ControllerNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The controller is already stopped.
    WrongState {
        /// Why it was refused: `conductor.dispatch.ControllerStateConflict`.
        error: ControllerStateConflict,
    },
}

/// CharterRevised — the event `conductor.dispatch.CharterRevised`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CharterRevised {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// ControllerPaused — the event `conductor.dispatch.ControllerPaused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerPaused {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// ControllerResumed — the event `conductor.dispatch.ControllerResumed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerResumed {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// ControllerStarted — the event `conductor.dispatch.ControllerStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerStarted {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: crate::observation::Harness,
}

/// ControllerStopped — the event `conductor.dispatch.ControllerStopped`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerStopped {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
    /// `reason` — `String`.
    pub reason: String,
}

/// DispatchBlocked — the event `conductor.dispatch.DispatchBlocked`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchBlocked {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `reason` — `String`.
    pub reason: String,
}

/// DispatchCancelled — the event `conductor.dispatch.DispatchCancelled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchCancelled {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `reason` — `String`.
    pub reason: String,
}

/// DispatchDone — the event `conductor.dispatch.DispatchDone`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchDone {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `evidence` — `String`.
    pub evidence: String,
}

/// DispatchFailed — the event `conductor.dispatch.DispatchFailed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchFailed {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `reason` — `String`.
    pub reason: String,
}

/// DispatchSent — the event `conductor.dispatch.DispatchSent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchSent {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: crate::direction::GoalId,
}

/// DispatchStarted — the event `conductor.dispatch.DispatchStarted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchStarted {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// DispatchUnblocked — the event `conductor.dispatch.DispatchUnblocked`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchUnblocked {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// GuardDecisionRecorded — the event `conductor.dispatch.GuardDecisionRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDecisionRecorded {
    /// `guard_decision_id` — `conductor.dispatch.GuardDecisionId`.
    pub guard_decision_id: GuardDecisionId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `tool` — `String`.
    pub tool: String,
    /// `verdict` — `conductor.dispatch.Verdict`.
    pub verdict: Verdict,
}

/// MessageHandled — the event `conductor.dispatch.MessageHandled`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageHandled {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
}

/// MessageReceived — the event `conductor.dispatch.MessageReceived`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageReceived {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
    /// `sender` — `String`.
    pub sender: String,
    /// `kind` — `conductor.dispatch.MessageKind`.
    pub kind: MessageKind,
}

/// MessageRejected — the event `conductor.dispatch.MessageRejected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRejected {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
    /// `reason` — `String`.
    pub reason: String,
}

/// NeedRouted — the event `conductor.dispatch.NeedRouted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedRouted {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// ResourceGranted — the event `conductor.dispatch.ResourceGranted`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceGranted {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
}

/// ResourceRefused — the event `conductor.dispatch.ResourceRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRefused {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
    /// `reason` — `String`.
    pub reason: String,
}

/// ResourceReleased — the event `conductor.dispatch.ResourceReleased`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceReleased {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
}

/// ResourceRequested — the event `conductor.dispatch.ResourceRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequested {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `resource` — `conductor.dispatch.ResourceKind`.
    pub resource: ResourceKind,
}

/// The declared error `conductor.dispatch.ControllerAlreadyRunning`.
///
/// The repository already has a running or paused controller, so no second one starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerAlreadyRunning {
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
}

/// The declared error `conductor.dispatch.ControllerNotFound`.
///
/// No controller has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerNotFound {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// The declared error `conductor.dispatch.ControllerStateConflict`.
///
/// The controller is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerStateConflict {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
}

/// The declared error `conductor.dispatch.DiskBelowFloor`.
///
/// Free disk is under the 30G floor, so no build slot was granted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiskBelowFloor {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
    /// `disk_free_bytes` — `Integer`.
    pub disk_free_bytes: i64,
}

/// The declared error `conductor.dispatch.DispatchExists`.
///
/// A dispatch already has this id, so nothing was sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchExists {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// The declared error `conductor.dispatch.DispatchNotFound`.
///
/// No dispatch has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchNotFound {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// The declared error `conductor.dispatch.DispatchStateConflict`.
///
/// The dispatch is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchStateConflict {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// The declared error `conductor.dispatch.GoalClosed`.
///
/// The goal is met or dropped, so nothing was dispatched against it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalClosed {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: crate::direction::GoalId,
}

/// The declared error `conductor.dispatch.GoalNotFound`.
///
/// No goal has this id, so nothing was dispatched.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GoalNotFound {
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: crate::direction::GoalId,
}

/// The declared error `conductor.dispatch.InvalidAmount`.
///
/// A resource amount is a positive number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidAmount {
    /// `amount` — `Integer`.
    pub amount: i64,
}

/// The declared error `conductor.dispatch.InvalidPriority`.
///
/// A dispatch priority is never negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidPriority {
    /// `priority` — `Integer`.
    pub priority: i64,
}

/// The declared error `conductor.dispatch.MessageNotFound`.
///
/// No message has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageNotFound {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
}

/// The declared error `conductor.dispatch.MessageStateConflict`.
///
/// The message was already handled or rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageStateConflict {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
}

/// The declared error `conductor.dispatch.NeedNotRouted`.
///
/// The message is a `[NEED]`; route it to the dispatch it opened with `RouteNeed` instead.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedNotRouted {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
}

/// The declared error `conductor.dispatch.NotANeed`.
///
/// The message is not a `[NEED]`, so it opens no dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotANeed {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
}

/// The declared error `conductor.dispatch.ResourceRequestNotFound`.
///
/// No resource request has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequestNotFound {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
}

/// The declared error `conductor.dispatch.ResourceRequestStateConflict`.
///
/// The resource request was already granted or refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequestStateConflict {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
}

/// The declared error `conductor.dispatch.WaitsOnItself`.
///
/// The dispatch names itself as the dispatch it waits on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaitsOnItself {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
}

/// Controllers — one row of the view `conductor.dispatch.Controllers`.
///
/// Projects `conductor.dispatch.Controller` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controllers {
    /// `controller_id` — `conductor.dispatch.ControllerId`.
    pub controller_id: ControllerId,
    /// `state` — `conductor.dispatch.Controller.State`.
    pub state: ControllerState,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `harness` — `conductor.observation.Harness`.
    pub harness: crate::observation::Harness,
    /// `session_name` — `conductor.observation.RepositoryName`.
    pub session_name: crate::observation::RepositoryName,
    /// `charter_revision` — `Integer`.
    pub charter_revision: i64,
}

/// Dispatches — one row of the view `conductor.dispatch.Dispatches`.
///
/// Projects `conductor.dispatch.Dispatch` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatches {
    /// `dispatch_id` — `conductor.dispatch.DispatchId`.
    pub dispatch_id: DispatchId,
    /// `state` — `conductor.dispatch.Dispatch.State`.
    pub state: DispatchState,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `goal_id` — `conductor.direction.GoalId`.
    pub goal_id: crate::direction::GoalId,
    /// `brief` — `String`.
    pub brief: String,
    /// `sent_at` — `Timestamp`.
    pub sent_at: crate::primitives::Timestamp,
    /// `priority` — `Integer`.
    pub priority: i64,
    /// `waits_on` — `Optional<conductor.dispatch.DispatchId>`.
    pub waits_on: Option<DispatchId>,
}

/// Guard decisions — one row of the view `conductor.dispatch.GuardDecisions`.
///
/// Projects `conductor.dispatch.GuardDecision` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GuardDecisions {
    /// `guard_decision_id` — `conductor.dispatch.GuardDecisionId`.
    pub guard_decision_id: GuardDecisionId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `session_ref` — `String`.
    pub session_ref: String,
    /// `tool` — `String`.
    pub tool: String,
    /// `target` — `String`.
    pub target: String,
    /// `verdict` — `conductor.dispatch.Verdict`.
    pub verdict: Verdict,
    /// `reason` — `String`.
    pub reason: String,
    /// `decided_at` — `Timestamp`.
    pub decided_at: crate::primitives::Timestamp,
}

/// Messages — one row of the view `conductor.dispatch.Messages`.
///
/// Projects `conductor.dispatch.Message` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Messages {
    /// `message_id` — `conductor.dispatch.MessageId`.
    pub message_id: MessageId,
    /// `state` — `conductor.dispatch.Message.State`.
    pub state: MessageState,
    /// `sender` — `String`.
    pub sender: String,
    /// `recipient` — `String`.
    pub recipient: String,
    /// `kind` — `conductor.dispatch.MessageKind`.
    pub kind: MessageKind,
    /// `first_line` — `String`.
    pub first_line: String,
    /// `transport` — `conductor.dispatch.Transport`.
    pub transport: Transport,
    /// `received_at` — `Timestamp`.
    pub received_at: crate::primitives::Timestamp,
    /// `body_path` — `Optional<String>`.
    pub body_path: Option<String>,
    /// `dispatch_id` — `Optional<conductor.dispatch.DispatchId>`.
    pub dispatch_id: Option<DispatchId>,
}

/// Resource requests — one row of the view `conductor.dispatch.ResourceRequests`.
///
/// Projects `conductor.dispatch.ResourceRequest` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceRequests {
    /// `resource_request_id` — `conductor.dispatch.ResourceRequestId`.
    pub resource_request_id: ResourceRequestId,
    /// `state` — `conductor.dispatch.ResourceRequest.State`.
    pub state: ResourceRequestState,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `resource` — `conductor.dispatch.ResourceKind`.
    pub resource: ResourceKind,
    /// `amount` — `Integer`.
    pub amount: i64,
    /// `requested_at` — `Timestamp`.
    pub requested_at: crate::primitives::Timestamp,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.dispatch.CancelDispatch` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait CancelDispatchBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.CancelDispatch`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn cancel_dispatch(&mut self, input: super::CancelDispatch) -> Result<super::CancelDispatchOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.GrantResource` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait GrantResourceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.GrantResource`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn grant_resource(&mut self, input: super::GrantResource) -> Result<super::GrantResourceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.HandleMessage` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait HandleMessageBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.HandleMessage`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn handle_message(&mut self, input: super::HandleMessage) -> Result<super::HandleMessageOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.PauseController` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PauseControllerBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.PauseController`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn pause_controller(&mut self, input: super::PauseController) -> Result<super::PauseControllerOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReceiveMessage` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReceiveMessageBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReceiveMessage`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn receive_message(&mut self, input: super::ReceiveMessage) -> Result<super::ReceiveMessageOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.RecordGuardDecision` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordGuardDecisionBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.RecordGuardDecision`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_guard_decision(&mut self, input: super::RecordGuardDecision) -> Result<super::RecordGuardDecisionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.RefuseResource` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RefuseResourceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.RefuseResource`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn refuse_resource(&mut self, input: super::RefuseResource) -> Result<super::RefuseResourceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.RejectMessage` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RejectMessageBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.RejectMessage`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn reject_message(&mut self, input: super::RejectMessage) -> Result<super::RejectMessageOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReleaseResource` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReleaseResourceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReleaseResource`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn release_resource(&mut self, input: super::ReleaseResource) -> Result<super::ReleaseResourceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReportBlocked` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReportBlockedBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReportBlocked`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn report_blocked(&mut self, input: super::ReportBlocked) -> Result<super::ReportBlockedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReportDone` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReportDoneBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReportDone`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn report_done(&mut self, input: super::ReportDone) -> Result<super::ReportDoneOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReportFailed` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReportFailedBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReportFailed`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn report_failed(&mut self, input: super::ReportFailed) -> Result<super::ReportFailedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReportStarted` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReportStartedBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReportStarted`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn report_started(&mut self, input: super::ReportStarted) -> Result<super::ReportStartedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReportUnblocked` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReportUnblockedBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReportUnblocked`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn report_unblocked(&mut self, input: super::ReportUnblocked) -> Result<super::ReportUnblockedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.RequestResource` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RequestResourceBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.RequestResource`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn request_resource(&mut self, input: super::RequestResource) -> Result<super::RequestResourceOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ResumeController` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ResumeControllerBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ResumeController`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn resume_controller(&mut self, input: super::ResumeController) -> Result<super::ResumeControllerOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.ReviseCharter` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReviseCharterBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.ReviseCharter`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn revise_charter(&mut self, input: super::ReviseCharter) -> Result<super::ReviseCharterOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.RouteNeed` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RouteNeedBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.RouteNeed`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn route_need(&mut self, input: super::RouteNeed) -> Result<super::RouteNeedOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.SendDispatch` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait SendDispatchBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.SendDispatch`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn send_dispatch(&mut self, input: super::SendDispatch) -> Result<super::SendDispatchOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.StartController` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a guard over the rows a selector selects (`when_related: {entity, where, exists | count | forall}`), in `already-running`.
    ///
    /// Contract: given `conductor.dispatch.StartController` input, decide and enact exactly one outcome. Declared outcomes (declaration order, not selection precedence): `already-running` when, of the `conductor.dispatch.Controller` rows satisfying `(repository == input.repository and state in [Running, Paused])`, at least 1 rows are selected, error `conductor.dispatch.ControllerAlreadyRunning`; `started` otherwise, creates `conductor.dispatch.Controller`, emits `conductor.dispatch.ControllerStarted`.
    pub trait StartControllerBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.StartController`.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn start_controller(&mut self, input: super::StartController) -> Result<super::StartControllerOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.dispatch.StopController` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait StopControllerBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.dispatch.StopController`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn stop_controller(&mut self, input: super::StopController) -> Result<super::StopControllerOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.dispatch.Controllers` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ControllersQuery {
        /// Serves `conductor.dispatch.Controllers` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn controllers(&self) -> Result<Vec<super::Controllers>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.dispatch.Dispatches` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait DispatchesQuery {
        /// Serves `conductor.dispatch.Dispatches` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn dispatches(&self) -> Result<Vec<super::Dispatches>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.dispatch.GuardDecisions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait GuardDecisionsQuery {
        /// Serves `conductor.dispatch.GuardDecisions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn guard_decisions(&self) -> Result<Vec<super::GuardDecisions>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.dispatch.Messages` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait MessagesQuery {
        /// Serves `conductor.dispatch.Messages` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn messages(&self) -> Result<Vec<super::Messages>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.dispatch.ResourceRequests` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ResourceRequestsQuery {
        /// Serves `conductor.dispatch.ResourceRequests` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn resource_requests(&self) -> Result<Vec<super::ResourceRequests>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl StartControllerBehavior for Unimplemented {
        fn start_controller(&mut self, _input: super::StartController) -> Result<super::StartControllerOutcome, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "command behaviour", source: "conductor.dispatch.StartController" })
        }
    }
}
