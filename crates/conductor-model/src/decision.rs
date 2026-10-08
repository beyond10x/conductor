// generated from conductor v1
// model digest 91b1e04b9d3f40c554bf0e78f07a6160c885b7e7f6864fec9da0ee72675cfe04
// contract digest af130e1a335a2c4eb9139d6a63b92dd42a89d0b92bb1c80ec5e4354e782de74a
// do not edit: regenerate with `ess synthesize --layout crate`

//! Decision — `conductor.decision`.
//!
//! Decisions moved one layer up. A repository controller raises a decision request; conductor answers it with a recorded decision when its class is C, and escalates it to the operator when its class is O (he decides: architecture, risk, spending) or H (he acts: a secret, a login, a permission; listed in the daily brief's numbered to-do, never asked one at a time). A decision carries its class, its options, its choice and its reason, and stays in force until it is reversed. Design: docs/design/conductor.md §§ 5–6.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Decider — `conductor.decision.Decider`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decider {
    /// `Conductor`.
    Conductor,
    /// `Operator`.
    Operator,
}

/// The states of `conductor.decision.Decision`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `Decision<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionState {
    /// `InForce`.
    InForce,
    /// `Reversed`.
    Reversed,
}

/// DecisionClass — `conductor.decision.DecisionClass`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionClass {
    /// `C`.
    C,
    /// `O`.
    O,
    /// `H`.
    H,
}

/// DecisionId — `conductor.decision.DecisionId`: a distinct wrapper around `String`.
///
/// Every value starts with `DEC-`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionId(pub String);

/// The states of `conductor.decision.DecisionRequest`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `DecisionRequest<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionRequestState {
    /// `Answered`.
    Answered,
    /// `Escalated`.
    Escalated,
    /// `Open`.
    Open,
    /// `Withdrawn`.
    Withdrawn,
}

/// RequestId — `conductor.decision.RequestId`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestId(pub String);

/// ShowDecision — `conductor.decision.ShowDecision`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowDecision {
    /// `decision_id` — `String`.
    pub decision_id: String,
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
}

/// What Decision — `conductor.decision.Decision` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`Decision<S>`], and at a boundary by [`DecisionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionData {
    /// The identity: `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `decision_class` — `conductor.decision.DecisionClass`.
    pub decision_class: DecisionClass,
    /// `decided_by` — `conductor.decision.Decider`.
    pub decided_by: Decider,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `choice` — `String`.
    pub choice: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `evidence` — `String`.
    pub evidence: String,
    /// `decided_at` — `Timestamp`.
    pub decided_at: crate::primitives::Timestamp,
}

/// The states of `conductor.decision.Decision`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](decision_state::Marker), so [`Decision<S>`](Decision) can only ever rest in a real state.
pub mod decision_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::InForce {}
        impl Sealed for super::Reversed {}
    }

    /// A declared state of `Decision`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DecisionState;
    }

    /// `InForce`. Where a new instance starts.
    pub struct InForce;

    impl Marker for InForce {
        const STATE: super::DecisionState = super::DecisionState::InForce;
    }

    /// `Reversed`. Terminal: an instance may rest here forever.
    pub struct Reversed;

    impl Marker for Reversed {
        const STATE: super::DecisionState = super::DecisionState::Reversed;
    }
}

/// Decision — `conductor.decision.Decision` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `InForce`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DecisionSnapshot`]
/// and [`DecisionSnapshot::refine`].
pub struct Decision<S: decision_state::Marker> {
    data: DecisionData,
    state: core::marker::PhantomData<S>,
}

impl<S: decision_state::Marker> Decision<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DecisionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DecisionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DecisionData {
        self.data
    }
}

impl Decision<decision_state::InForce> {
    /// A new instance, resting in `InForce` — the only state the lifecycle starts one in.
    pub fn new(data: DecisionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl Decision<decision_state::InForce> {
    /// `reverse` — `InForce` → `Reversed`. Taken by the `reversed` outcome of `conductor.decision.ReverseConductorDecision`, the `reversed` outcome of `conductor.decision.ReverseOperatorDecision`.
    pub fn reverse(self) -> Decision<decision_state::Reversed> {
        Decision {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.decision.Decision` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DecisionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DecisionState,
    /// What it holds.
    pub data: DecisionData,
}

/// An `Decision` in whichever declared state it was found.
pub enum AnyDecision {
    /// Resting in `InForce`.
    InForce(Decision<decision_state::InForce>),
    /// Resting in `Reversed`.
    Reversed(Decision<decision_state::Reversed>),
}

impl DecisionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DecisionState` cannot spell one.
    pub fn refine(self) -> AnyDecision {
        match self.state {
            DecisionState::InForce => AnyDecision::InForce(Decision {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DecisionState::Reversed => AnyDecision::Reversed(Decision {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDecision {
    /// The state, as the runtime value.
    pub fn state(&self) -> DecisionState {
        match self {
            Self::InForce(_) => DecisionState::InForce,
            Self::Reversed(_) => DecisionState::Reversed,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DecisionSnapshot {
        match self {
            Self::InForce(instance) => DecisionSnapshot {
                state: DecisionState::InForce,
                data: instance.into_data(),
            },
            Self::Reversed(instance) => DecisionSnapshot {
                state: DecisionState::Reversed,
                data: instance.into_data(),
            },
        }
    }
}

/// What DecisionRequest — `conductor.decision.DecisionRequest` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`DecisionRequest<S>`], and at a boundary by [`DecisionRequestSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRequestData {
    /// The identity: `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `recommendation` — `String`.
    pub recommendation: String,
    /// `raised_at` — `Timestamp`.
    pub raised_at: crate::primitives::Timestamp,
    /// `blocker` — `Optional<String>`.
    pub blocker: Option<String>,
    /// `escalation_class` — `Optional<conductor.decision.DecisionClass>`.
    pub escalation_class: Option<DecisionClass>,
    /// `escalation_reason` — `Optional<String>`.
    pub escalation_reason: Option<String>,
    /// `decision_id` — `Optional<conductor.decision.DecisionId>`.
    ///
    /// Carries `decision`: `conductor.decision.DecisionRequest` references one `conductor.decision.Decision`.
    pub decision_id: Option<DecisionId>,
}

/// The states of `conductor.decision.DecisionRequest`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](decision_request_state::Marker), so [`DecisionRequest<S>`](DecisionRequest) can only ever rest in a real state.
pub mod decision_request_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Answered {}
        impl Sealed for super::Escalated {}
        impl Sealed for super::Open {}
        impl Sealed for super::Withdrawn {}
    }

    /// A declared state of `DecisionRequest`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::DecisionRequestState;
    }

    /// `Answered`. Terminal: an instance may rest here forever.
    pub struct Answered;

    impl Marker for Answered {
        const STATE: super::DecisionRequestState = super::DecisionRequestState::Answered;
    }

    /// `Escalated`.
    pub struct Escalated;

    impl Marker for Escalated {
        const STATE: super::DecisionRequestState = super::DecisionRequestState::Escalated;
    }

    /// `Open`. Where a new instance starts.
    pub struct Open;

    impl Marker for Open {
        const STATE: super::DecisionRequestState = super::DecisionRequestState::Open;
    }

    /// `Withdrawn`. Terminal: an instance may rest here forever.
    pub struct Withdrawn;

    impl Marker for Withdrawn {
        const STATE: super::DecisionRequestState = super::DecisionRequestState::Withdrawn;
    }
}

/// DecisionRequest — `conductor.decision.DecisionRequest` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Open`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`DecisionRequestSnapshot`]
/// and [`DecisionRequestSnapshot::refine`].
pub struct DecisionRequest<S: decision_request_state::Marker> {
    data: DecisionRequestData,
    state: core::marker::PhantomData<S>,
}

impl<S: decision_request_state::Marker> DecisionRequest<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> DecisionRequestState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &DecisionRequestData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> DecisionRequestData {
        self.data
    }
}

impl DecisionRequest<decision_request_state::Open> {
    /// A new instance, resting in `Open` — the only state the lifecycle starts one in.
    pub fn new(data: DecisionRequestData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

impl DecisionRequest<decision_request_state::Escalated> {
    /// `answer_escalated` — `Escalated` → `Answered`. Taken by the `answered` outcome of `conductor.decision.AnswerEscalatedRequest`.
    pub fn answer_escalated(self) -> DecisionRequest<decision_request_state::Answered> {
        DecisionRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `withdraw` — `Escalated` → `Withdrawn`. Taken by the `withdrawn` outcome of `conductor.decision.WithdrawRequest`.
    pub fn withdraw(self) -> DecisionRequest<decision_request_state::Withdrawn> {
        DecisionRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

impl DecisionRequest<decision_request_state::Open> {
    /// `escalate` — `Open` → `Escalated`. Taken by the `escalated` outcome of `conductor.decision.EscalateRequest`.
    pub fn escalate(self) -> DecisionRequest<decision_request_state::Escalated> {
        DecisionRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `answer` — `Open` → `Answered`. Taken by the `answered` outcome of `conductor.decision.AnswerRequest`.
    pub fn answer(self) -> DecisionRequest<decision_request_state::Answered> {
        DecisionRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }

    /// `withdraw` — `Open` → `Withdrawn`. Taken by the `withdrawn` outcome of `conductor.decision.WithdrawRequest`.
    pub fn withdraw(self) -> DecisionRequest<decision_request_state::Withdrawn> {
        DecisionRequest {
            data: self.data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.decision.DecisionRequest` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`DecisionRequestSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRequestSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: DecisionRequestState,
    /// What it holds.
    pub data: DecisionRequestData,
}

/// An `DecisionRequest` in whichever declared state it was found.
pub enum AnyDecisionRequest {
    /// Resting in `Answered`.
    Answered(DecisionRequest<decision_request_state::Answered>),
    /// Resting in `Escalated`.
    Escalated(DecisionRequest<decision_request_state::Escalated>),
    /// Resting in `Open`.
    Open(DecisionRequest<decision_request_state::Open>),
    /// Resting in `Withdrawn`.
    Withdrawn(DecisionRequest<decision_request_state::Withdrawn>),
}

impl DecisionRequestSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `DecisionRequestState` cannot spell one.
    pub fn refine(self) -> AnyDecisionRequest {
        match self.state {
            DecisionRequestState::Answered => AnyDecisionRequest::Answered(DecisionRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DecisionRequestState::Escalated => AnyDecisionRequest::Escalated(DecisionRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DecisionRequestState::Open => AnyDecisionRequest::Open(DecisionRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
            DecisionRequestState::Withdrawn => AnyDecisionRequest::Withdrawn(DecisionRequest {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyDecisionRequest {
    /// The state, as the runtime value.
    pub fn state(&self) -> DecisionRequestState {
        match self {
            Self::Answered(_) => DecisionRequestState::Answered,
            Self::Escalated(_) => DecisionRequestState::Escalated,
            Self::Open(_) => DecisionRequestState::Open,
            Self::Withdrawn(_) => DecisionRequestState::Withdrawn,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> DecisionRequestSnapshot {
        match self {
            Self::Answered(instance) => DecisionRequestSnapshot {
                state: DecisionRequestState::Answered,
                data: instance.into_data(),
            },
            Self::Escalated(instance) => DecisionRequestSnapshot {
                state: DecisionRequestState::Escalated,
                data: instance.into_data(),
            },
            Self::Open(instance) => DecisionRequestSnapshot {
                state: DecisionRequestState::Open,
                data: instance.into_data(),
            },
            Self::Withdrawn(instance) => DecisionRequestSnapshot {
                state: DecisionRequestState::Withdrawn,
                data: instance.into_data(),
            },
        }
    }
}

/// Answer an escalated decision request — the input of `conductor.decision.AnswerEscalatedRequest`.
///
/// Everything it can result in is [`AnswerEscalatedRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerEscalatedRequest {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// Everything `conductor.decision.AnswerEscalatedRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerEscalatedRequestOutcome {
    /// `no-such-decision` — when no `conductor.decision.Decision` carries the identity `input.decision_id` names.
    ///
    /// The cited decision does not exist, and the request stays where it was.
    NoSuchDecision {
        /// Why it was refused: `conductor.decision.DecisionNotFound`.
        error: DecisionNotFound,
    },
    /// `reversed` — when the `conductor.decision.Decision` that `input.decision_id` names satisfies `state == Reversed`.
    ///
    /// The cited decision was reversed, and the request stays where it was.
    Reversed {
        /// Why it was refused: `conductor.decision.DecisionReversed`.
        error: DecisionReversed,
    },
    /// `answered` — otherwise.
    ///
    /// The operator answered the escalated request; the controller acts on it.
    Answered {
        /// The `conductor.decision.RequestAnswered` this outcome publishes.
        request_answered: RequestAnswered,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only an escalated request is answered here.
    WrongState {
        /// Why it was refused: `conductor.decision.RequestStateConflict`.
        error: RequestStateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// Answer a decision request — the input of `conductor.decision.AnswerRequest`.
///
/// Everything it can result in is [`AnswerRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnswerRequest {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// Everything `conductor.decision.AnswerRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerRequestOutcome {
    /// `no-such-decision` — when no `conductor.decision.Decision` carries the identity `input.decision_id` names.
    ///
    /// The cited decision does not exist, and the request stays where it was.
    NoSuchDecision {
        /// Why it was refused: `conductor.decision.DecisionNotFound`.
        error: DecisionNotFound,
    },
    /// `reversed` — when the `conductor.decision.Decision` that `input.decision_id` names satisfies `state == Reversed`.
    ///
    /// The cited decision was reversed, and the request stays where it was.
    Reversed {
        /// Why it was refused: `conductor.decision.DecisionReversed`.
        error: DecisionReversed,
    },
    /// `answered` — otherwise.
    ///
    /// The request is answered by a decision in force; the controller acts on it.
    Answered {
        /// The `conductor.decision.RequestAnswered` this outcome publishes.
        request_answered: RequestAnswered,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only an open request is answered here. An escalated one waits for the operator (`AnswerEscalatedRequest`); an answered or withdrawn one is closed.
    WrongState {
        /// Why it was refused: `conductor.decision.RequestStateConflict`.
        error: RequestStateConflict,
    },
    /// `wrong-state` — for an instance no record carries.
    ///
    /// The same declared branch and error as [`Self::WrongState`], without the error's fields: an instance
    /// that does not exist has nothing for them to describe (`docs/design/unknown-instance-seams.md`).
    WrongStateUnknownInstance,
}

/// Escalate a decision request — the input of `conductor.decision.EscalateRequest`.
///
/// Everything it can result in is [`EscalateRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EscalateRequest {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `escalation_class` — `conductor.decision.DecisionClass`.
    pub escalation_class: DecisionClass,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.decision.EscalateRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EscalateRequestOutcome {
    /// `class-c` — when `escalation_class == C`.
    ///
    /// A class-C request is decided by conductor, never escalated.
    ClassC {
        /// Why it was refused: `conductor.decision.NotEscalatable`.
        error: NotEscalatable,
    },
    /// `escalated` — otherwise.
    ///
    /// The request waits for the operator.
    Escalated {
        /// The `conductor.decision.RequestEscalated` this outcome publishes.
        request_escalated: RequestEscalated,
    },
    /// `no-such-request` — for an identity no record carries.
    NoSuchRequest {
        /// Why it was refused: `conductor.decision.RequestNotFound`.
        error: RequestNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// Only an open request is escalated.
    WrongState {
        /// Why it was refused: `conductor.decision.RequestStateConflict`.
        error: RequestStateConflict,
    },
}

/// Raise a decision request — the input of `conductor.decision.RaiseDecisionRequest`.
///
/// Everything it can result in is [`RaiseDecisionRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaiseDecisionRequest {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `recommendation` — `String`.
    pub recommendation: String,
    /// `raised_at` — `Timestamp`.
    pub raised_at: crate::primitives::Timestamp,
    /// `blocker` — `Optional<String>`.
    pub blocker: Option<String>,
}

/// Everything `conductor.decision.RaiseDecisionRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RaiseDecisionRequestOutcome {
    /// `raised` — otherwise.
    ///
    /// The request is open and queued for the next cycle.
    Raised {
        /// The `conductor.decision.DecisionRequested` this outcome publishes.
        decision_requested: DecisionRequested,
    },
    /// `already-raised` — for an identity a record already carries.
    ///
    /// A request already has this id; the held request is unchanged, answered or not.
    AlreadyRaised {
        /// Why it was refused: `conductor.decision.RequestExists`.
        error: RequestExists,
    },
}

/// Record a decision taken by conductor — the input of `conductor.decision.RecordConductorDecision`.
///
/// Everything it can result in is [`RecordConductorDecisionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordConductorDecision {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `decision_class` — `conductor.decision.DecisionClass`.
    pub decision_class: DecisionClass,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `choice` — `String`.
    pub choice: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `evidence` — `String`.
    pub evidence: String,
    /// `decided_at` — `Timestamp`.
    pub decided_at: crate::primitives::Timestamp,
}

/// Everything `conductor.decision.RecordConductorDecision` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordConductorDecisionOutcome {
    /// `reserved` — when `decision_class in [O, H]`.
    ///
    /// Conductor may not decide class O or H (design § 5), and nothing was recorded.
    Reserved {
        /// Why it was refused: `conductor.decision.ReservedForOperator`.
        error: ReservedForOperator,
    },
    /// `recorded` — otherwise.
    ///
    /// The class-C decision is in force and counts as the operator's (standing exception 4).
    Recorded {
        /// The `conductor.decision.DecisionRecorded` this outcome publishes.
        decision_recorded: DecisionRecorded,
    },
    /// `already-recorded` — for an identity a record already carries.
    ///
    /// A decision already has this id; the held decision is unchanged.
    AlreadyRecorded {
        /// Why it was refused: `conductor.decision.DecisionExists`.
        error: DecisionExists,
    },
}

/// Record a decision taken by the operator — the input of `conductor.decision.RecordOperatorDecision`.
///
/// Everything it can result in is [`RecordOperatorDecisionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordOperatorDecision {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `decision_class` — `conductor.decision.DecisionClass`.
    pub decision_class: DecisionClass,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `choice` — `String`.
    pub choice: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `evidence` — `String`.
    pub evidence: String,
    /// `decided_at` — `Timestamp`.
    pub decided_at: crate::primitives::Timestamp,
}

/// Everything `conductor.decision.RecordOperatorDecision` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecordOperatorDecisionOutcome {
    /// `recorded` — otherwise.
    ///
    /// The operator's decision of any class is in force.
    Recorded {
        /// The `conductor.decision.DecisionRecorded` this outcome publishes.
        decision_recorded: DecisionRecorded,
    },
    /// `already-recorded` — for an identity a record already carries.
    ///
    /// A decision already has this id; the held decision is unchanged.
    AlreadyRecorded {
        /// Why it was refused: `conductor.decision.DecisionExists`.
        error: DecisionExists,
    },
}

/// Reverse a decision taken by conductor — the input of `conductor.decision.ReverseConductorDecision`.
///
/// Everything it can result in is [`ReverseConductorDecisionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseConductorDecision {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.decision.ReverseConductorDecision` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverseConductorDecisionOutcome {
    /// `operator-decided` — when the existing subject's stored fields satisfy `(decided_by == Operator and state == InForce)`.
    ///
    /// The operator recorded this decision, so only he reverses it; it stays in force.
    OperatorDecided {
        /// Why it was refused: `conductor.decision.ReversalReservedForOperator`.
        error: ReversalReservedForOperator,
    },
    /// `reversed` — otherwise.
    ///
    /// Conductor's decision no longer holds; a reversed decision becomes a rule in a profile (design § 13).
    Reversed {
        /// The `conductor.decision.DecisionReversedEvent` this outcome publishes.
        decision_reversed_event: DecisionReversedEvent,
    },
    /// `no-such-decision` — for an identity no record carries.
    NoSuchDecision {
        /// Why it was refused: `conductor.decision.DecisionNotFound`.
        error: DecisionNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The decision is already reversed.
    WrongState {
        /// Why it was refused: `conductor.decision.DecisionStateConflict`.
        error: DecisionStateConflict,
    },
}

/// Reverse a decision as the operator — the input of `conductor.decision.ReverseOperatorDecision`.
///
/// Everything it can result in is [`ReverseOperatorDecisionOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReverseOperatorDecision {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.decision.ReverseOperatorDecision` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReverseOperatorDecisionOutcome {
    /// `reversed` — otherwise.
    ///
    /// The operator reversed the decision, whoever took it; a reversed decision becomes a rule in a profile (design § 13).
    Reversed {
        /// The `conductor.decision.DecisionReversedEvent` this outcome publishes.
        decision_reversed_event: DecisionReversedEvent,
    },
    /// `no-such-decision` — for an identity no record carries.
    NoSuchDecision {
        /// Why it was refused: `conductor.decision.DecisionNotFound`.
        error: DecisionNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The decision is already reversed.
    WrongState {
        /// Why it was refused: `conductor.decision.DecisionStateConflict`.
        error: DecisionStateConflict,
    },
}

/// Withdraw a decision request — the input of `conductor.decision.WithdrawRequest`.
///
/// Everything it can result in is [`WithdrawRequestOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WithdrawRequest {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `reason` — `String`.
    pub reason: String,
}

/// Everything `conductor.decision.WithdrawRequest` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WithdrawRequestOutcome {
    /// `withdrawn` — otherwise.
    ///
    /// The controller no longer needs the decision.
    Withdrawn {
        /// The `conductor.decision.RequestWithdrawn` this outcome publishes.
        request_withdrawn: RequestWithdrawn,
    },
    /// `no-such-request` — for an identity no record carries.
    NoSuchRequest {
        /// Why it was refused: `conductor.decision.RequestNotFound`.
        error: RequestNotFound,
    },
    /// `wrong-state` — from a state no declared move starts in.
    ///
    /// The request is already answered or withdrawn.
    WrongState {
        /// Why it was refused: `conductor.decision.RequestStateConflict`.
        error: RequestStateConflict,
    },
}

/// DecisionRecorded — the event `conductor.decision.DecisionRecorded`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRecorded {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `decision_class` — `conductor.decision.DecisionClass`.
    pub decision_class: DecisionClass,
    /// `decided_by` — `conductor.decision.Decider`.
    pub decided_by: Decider,
}

/// DecisionRequested — the event `conductor.decision.DecisionRequested`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionRequested {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
}

/// DecisionReversedEvent — the event `conductor.decision.DecisionReversedEvent`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionReversedEvent {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `reason` — `String`.
    pub reason: String,
}

/// RequestAnswered — the event `conductor.decision.RequestAnswered`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestAnswered {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// RequestEscalated — the event `conductor.decision.RequestEscalated`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestEscalated {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
}

/// RequestWithdrawn — the event `conductor.decision.RequestWithdrawn`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestWithdrawn {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `conductor.decision.DecisionExists`.
///
/// A decision already has this id, so nothing was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionExists {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// The declared error `conductor.decision.DecisionNotFound`.
///
/// No decision has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionNotFound {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// The declared error `conductor.decision.DecisionReversed`.
///
/// The decision was reversed, so it cannot answer a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionReversed {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// The declared error `conductor.decision.DecisionStateConflict`.
///
/// The decision is already reversed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionStateConflict {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// The declared error `conductor.decision.NotEscalatable`.
///
/// The request is class C; conductor decides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotEscalatable {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
}

/// The declared error `conductor.decision.RequestExists`.
///
/// A decision request already has this id, so nothing was raised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestExists {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
}

/// The declared error `conductor.decision.RequestNotFound`.
///
/// No decision request has this id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestNotFound {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
}

/// The declared error `conductor.decision.RequestStateConflict`.
///
/// The request is not in a state this command acts from, so nothing moved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RequestStateConflict {
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
}

/// The declared error `conductor.decision.ReservedForOperator`.
///
/// A class-O or class-H decision was submitted as decided by conductor. Those classes are the operator's (design § 5), and nothing was recorded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReservedForOperator {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// The declared error `conductor.decision.ReversalReservedForOperator`.
///
/// The decision was recorded by the operator, so only he reverses it (design § 5), and it stays in force.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReversalReservedForOperator {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
}

/// Decisions — one row of the view `conductor.decision.Decisions`.
///
/// Projects `conductor.decision.Decision` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decisions {
    /// `decision_id` — `conductor.decision.DecisionId`.
    pub decision_id: DecisionId,
    /// `state` — `conductor.decision.Decision.State`.
    pub state: DecisionState,
    /// `decision_class` — `conductor.decision.DecisionClass`.
    pub decision_class: DecisionClass,
    /// `decided_by` — `conductor.decision.Decider`.
    pub decided_by: Decider,
    /// `choice` — `String`.
    pub choice: String,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `reason` — `String`.
    pub reason: String,
    /// `evidence` — `String`.
    pub evidence: String,
    /// `decided_at` — `Timestamp`.
    pub decided_at: crate::primitives::Timestamp,
}

/// Class-H to-do — one row of the view `conductor.decision.HandsTodo`.
///
/// Projects `conductor.decision.DecisionRequest` at `read_your_writes` consistency, containing instances where `(escalation_class == H and state == Escalated)`.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandsTodo {
    /// `escalation_class` — `Optional<conductor.decision.DecisionClass>`.
    pub escalation_class: Option<DecisionClass>,
    /// `escalation_reason` — `Optional<String>`.
    pub escalation_reason: Option<String>,
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `state` — `conductor.decision.DecisionRequest.State`.
    pub state: DecisionRequestState,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `decision_id` — `Optional<conductor.decision.DecisionId>`.
    pub decision_id: Option<DecisionId>,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `recommendation` — `String`.
    pub recommendation: String,
    /// `raised_at` — `Timestamp`.
    pub raised_at: crate::primitives::Timestamp,
    /// `blocker` — `Optional<String>`.
    pub blocker: Option<String>,
}

/// Decision requests — one row of the view `conductor.decision.Requests`.
///
/// Projects `conductor.decision.DecisionRequest` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requests {
    /// `escalation_class` — `Optional<conductor.decision.DecisionClass>`.
    pub escalation_class: Option<DecisionClass>,
    /// `escalation_reason` — `Optional<String>`.
    pub escalation_reason: Option<String>,
    /// `request_id` — `conductor.decision.RequestId`.
    pub request_id: RequestId,
    /// `state` — `conductor.decision.DecisionRequest.State`.
    pub state: DecisionRequestState,
    /// `repository` — `conductor.observation.RepositoryName`.
    pub repository: crate::observation::RepositoryName,
    /// `decision_id` — `Optional<conductor.decision.DecisionId>`.
    pub decision_id: Option<DecisionId>,
    /// `question` — `String`.
    pub question: String,
    /// `options` — `String`.
    pub options: String,
    /// `recommendation` — `String`.
    pub recommendation: String,
    /// `raised_at` — `Timestamp`.
    pub raised_at: crate::primitives::Timestamp,
    /// `blocker` — `Optional<String>`.
    pub blocker: Option<String>,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
pub mod obligations {
    /// The behaviour `conductor.decision.AnswerEscalatedRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AnswerEscalatedRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.AnswerEscalatedRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn answer_escalated_request(&mut self, input: super::AnswerEscalatedRequest) -> Result<super::AnswerEscalatedRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.AnswerRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait AnswerRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.AnswerRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn answer_request(&mut self, input: super::AnswerRequest) -> Result<super::AnswerRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.EscalateRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait EscalateRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.EscalateRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn escalate_request(&mut self, input: super::EscalateRequest) -> Result<super::EscalateRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.RaiseDecisionRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RaiseDecisionRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.RaiseDecisionRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn raise_decision_request(&mut self, input: super::RaiseDecisionRequest) -> Result<super::RaiseDecisionRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.RecordConductorDecision` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordConductorDecisionBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.RecordConductorDecision`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_conductor_decision(&mut self, input: super::RecordConductorDecision) -> Result<super::RecordConductorDecisionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.RecordOperatorDecision` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait RecordOperatorDecisionBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.RecordOperatorDecision`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn record_operator_decision(&mut self, input: super::RecordOperatorDecision) -> Result<super::RecordOperatorDecisionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.ReverseConductorDecision` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReverseConductorDecisionBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.ReverseConductorDecision`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn reverse_conductor_decision(&mut self, input: super::ReverseConductorDecision) -> Result<super::ReverseConductorDecisionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.ReverseOperatorDecision` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait ReverseOperatorDecisionBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.ReverseOperatorDecision`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn reverse_operator_decision(&mut self, input: super::ReverseOperatorDecision) -> Result<super::ReverseOperatorDecisionOutcome, crate::obligation::UnmetObligation>;
    }

    /// The behaviour `conductor.decision.WithdrawRequest` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait WithdrawRequestBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.decision.WithdrawRequest`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn withdraw_request(&mut self, input: super::WithdrawRequest) -> Result<super::WithdrawRequestOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.decision.Decisions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait DecisionsQuery {
        /// Serves `conductor.decision.Decisions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn decisions(&self) -> Result<Vec<super::Decisions>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.decision.HandsTodo` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait HandsTodoQuery {
        /// Serves `conductor.decision.HandsTodo` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn hands_todo(&self) -> Result<Vec<super::HandsTodo>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.decision.Requests` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait RequestsQuery {
        /// Serves `conductor.decision.Requests` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn requests(&self) -> Result<Vec<super::Requests>, crate::obligation::UnmetObligation>;
    }

}
