// generated from conductor v1
// model digest 75ae36bcd64ac6ed7ea7350fa76e1813347a0d4390844f42e75e83fd78193d30
// contract digest 218b3db30bad06f16859184bd6831e8cd6288a02160b6f0fec5f8e61a9fbd7ee
// do not edit: regenerate with `ess synthesize --layout crate`

//! Every actor the specification declares, and the commands each may invoke — as data.
//!
//! A grant is checked against a caller identity, which these types do not read from anywhere:
//! whatever authenticates a request builds a [`Caller`], and a served surface checks it against
//! [`may`] before the command runs. The `PLAN.md` beside this workspace says, per actor,
//! whether a generated surface enforces the grant or the caller does.

/// An actor the specification declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Actor {
    /// `conductor.decision.Conductor`.
    DecisionConductor,
    /// `conductor.decision.Operator`.
    DecisionOperator,
    /// `conductor.decision.RepositoryController`.
    DecisionRepositoryController,
    /// `conductor.direction.Conductor`.
    DirectionConductor,
    /// `conductor.direction.Operator`.
    DirectionOperator,
    /// `conductor.dispatch.Conductor`.
    DispatchConductor,
    /// `conductor.dispatch.RepositoryController`.
    DispatchRepositoryController,
    /// `conductor.observation.Collector`.
    ObservationCollector,
    /// `conductor.session.Conductor`.
    SessionConductor,
}

impl Actor {
    /// Every declared actor, ordered by qualified name.
    pub const ALL: &'static [Actor] = &[
        Actor::DecisionConductor,
        Actor::DecisionOperator,
        Actor::DecisionRepositoryController,
        Actor::DirectionConductor,
        Actor::DirectionOperator,
        Actor::DispatchConductor,
        Actor::DispatchRepositoryController,
        Actor::ObservationCollector,
        Actor::SessionConductor,
    ];

    /// The actor's qualified name, as the specification spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Actor::DecisionConductor => "conductor.decision.Conductor",
            Actor::DecisionOperator => "conductor.decision.Operator",
            Actor::DecisionRepositoryController => "conductor.decision.RepositoryController",
            Actor::DirectionConductor => "conductor.direction.Conductor",
            Actor::DirectionOperator => "conductor.direction.Operator",
            Actor::DispatchConductor => "conductor.dispatch.Conductor",
            Actor::DispatchRepositoryController => "conductor.dispatch.RepositoryController",
            Actor::ObservationCollector => "conductor.observation.Collector",
            Actor::SessionConductor => "conductor.session.Conductor",
        }
    }
}

/// The qualified names of the commands `actor` may invoke, ordered by name; empty for an
/// actor that only observes.
pub fn may(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::DecisionConductor => &[
            "conductor.decision.AnswerRequest",
            "conductor.decision.EscalateRequest",
            "conductor.decision.RecordConductorDecision",
            "conductor.decision.ReverseConductorDecision",
        ],
        Actor::DecisionOperator => &[
            "conductor.decision.AnswerEscalatedRequest",
            "conductor.decision.RecordOperatorDecision",
            "conductor.decision.ReverseOperatorDecision",
        ],
        Actor::DecisionRepositoryController => &[
            "conductor.decision.RaiseDecisionRequest",
            "conductor.decision.WithdrawRequest",
        ],
        Actor::DirectionConductor => &[
            "conductor.direction.ActivateRepository",
            "conductor.direction.AddServing",
            "conductor.direction.DeactivateRepository",
            "conductor.direction.MarkGoalMet",
            "conductor.direction.MarkRepository",
            "conductor.direction.ProposeGoal",
            "conductor.direction.RemoveServing",
        ],
        Actor::DirectionOperator => &[
            "conductor.direction.ActivateRepository",
            "conductor.direction.ConfirmGoal",
            "conductor.direction.DeactivateRepository",
            "conductor.direction.DropGoal",
            "conductor.direction.MarkRepository",
        ],
        Actor::DispatchConductor => &[
            "conductor.dispatch.CancelDispatch",
            "conductor.dispatch.GrantResource",
            "conductor.dispatch.HandleMessage",
            "conductor.dispatch.PauseController",
            "conductor.dispatch.ReceiveMessage",
            "conductor.dispatch.RefuseResource",
            "conductor.dispatch.RejectMessage",
            "conductor.dispatch.ReportBlocked",
            "conductor.dispatch.ReportDone",
            "conductor.dispatch.ReportFailed",
            "conductor.dispatch.ReportStarted",
            "conductor.dispatch.ReportUnblocked",
            "conductor.dispatch.ResumeController",
            "conductor.dispatch.ReviseCharter",
            "conductor.dispatch.RouteNeed",
            "conductor.dispatch.SendDispatch",
            "conductor.dispatch.StartController",
            "conductor.dispatch.StopController",
        ],
        Actor::DispatchRepositoryController => &[
            "conductor.dispatch.RecordGuardDecision",
            "conductor.dispatch.ReleaseResource",
            "conductor.dispatch.ReportBlocked",
            "conductor.dispatch.ReportDone",
            "conductor.dispatch.ReportFailed",
            "conductor.dispatch.ReportStarted",
            "conductor.dispatch.ReportUnblocked",
            "conductor.dispatch.RequestResource",
        ],
        Actor::ObservationCollector => &[
            "conductor.observation.CompleteSnapshot",
            "conductor.observation.FailSnapshot",
            "conductor.observation.PublishBoard",
            "conductor.observation.RecordBlocker",
            "conductor.observation.RecordMergedPullRequest",
            "conductor.observation.RecordPullRequest",
            "conductor.observation.RecordRelease",
            "conductor.observation.RecordRepository",
            "conductor.observation.RecordSession",
            "conductor.observation.RecordSpecification",
            "conductor.observation.RecordWorkflowRun",
            "conductor.observation.StartSnapshot",
        ],
        Actor::SessionConductor => &[
            "conductor.session.ClaimLease",
        ],
    }
}

/// Who a request was authenticated as.
///
/// Built by whatever authenticates the request — a session, a token, a certificate — and
/// handed to the served surface's `dispatch` and `handle`, which check its grant
/// before the command runs. Never derived from the request itself: a client can write
/// anything into a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Caller {
    /// The declared actor.
    pub actor: Actor,
}

impl Caller {
    /// `true` when this caller may invoke `command`, named by its qualified name.
    pub fn may(&self, command: &str) -> bool {
        may(self.actor).contains(&command)
    }
}
