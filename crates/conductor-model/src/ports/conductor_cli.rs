// generated from conductor v1
// model digest ed7cd6fe1bbec690931b9bdbf0c7447bd55967c3864d496b3d2f9574f226752c
// contract digest 1beeecf0e711bec0a0d49e03d7b7f74ad75ab12f69f688acf36495a420332642
// do not edit: regenerate with `ess synthesize --layout crate`

//! conductor-cli — the `conductor-cli` component of `conductor` v1.
//!
//! The conductor command line. Conductor and the repository controllers call it, and it records every command in its local store under state/.
//!
//! The component's outer surface exactly as the specification declares it: accepted commands as
//! handlers, declared views as queries, published events as a typed outbox. The behaviour behind
//! every handler is an implementation obligation — see the `PLAN.md` beside this workspace — and
//! until one is satisfied, its stub answers with a typed refusal naming what is owed.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event this component declares it publishes, on its way to the system's transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PublishedEvent {
    /// `conductor.decision.DecisionRecorded`.
    DecisionRecorded(crate::decision::DecisionRecorded),
    /// `conductor.decision.DecisionRequested`.
    DecisionRequested(crate::decision::DecisionRequested),
    /// `conductor.decision.DecisionReversedEvent`.
    DecisionReversedEvent(crate::decision::DecisionReversedEvent),
    /// `conductor.decision.RequestAmended`.
    RequestAmended(crate::decision::RequestAmended),
    /// `conductor.decision.RequestAnswered`.
    RequestAnswered(crate::decision::RequestAnswered),
    /// `conductor.decision.RequestEscalated`.
    RequestEscalated(crate::decision::RequestEscalated),
    /// `conductor.decision.RequestWithdrawn`.
    RequestWithdrawn(crate::decision::RequestWithdrawn),
    /// `conductor.direction.GoalConfirmed`.
    GoalConfirmed(crate::direction::GoalConfirmed),
    /// `conductor.direction.GoalDropped`.
    GoalDropped(crate::direction::GoalDropped),
    /// `conductor.direction.GoalMet`.
    GoalMet(crate::direction::GoalMet),
    /// `conductor.direction.GoalProposed`.
    GoalProposed(crate::direction::GoalProposed),
    /// `conductor.direction.RepositoryActivated`.
    RepositoryActivated(crate::direction::RepositoryActivated),
    /// `conductor.direction.RepositoryDeactivated`.
    RepositoryDeactivated(crate::direction::RepositoryDeactivated),
    /// `conductor.direction.RepositoryMarked`.
    RepositoryMarked(crate::direction::RepositoryMarked),
    /// `conductor.direction.ServingAdded`.
    ServingAdded(crate::direction::ServingAdded),
    /// `conductor.direction.ServingRemoved`.
    ServingRemoved(crate::direction::ServingRemoved),
    /// `conductor.dispatch.CharterRevised`.
    CharterRevised(crate::dispatch::CharterRevised),
    /// `conductor.dispatch.ControllerPaused`.
    ControllerPaused(crate::dispatch::ControllerPaused),
    /// `conductor.dispatch.ControllerResumed`.
    ControllerResumed(crate::dispatch::ControllerResumed),
    /// `conductor.dispatch.ControllerStarted`.
    ControllerStarted(crate::dispatch::ControllerStarted),
    /// `conductor.dispatch.ControllerStopped`.
    ControllerStopped(crate::dispatch::ControllerStopped),
    /// `conductor.dispatch.DispatchBlocked`.
    DispatchBlocked(crate::dispatch::DispatchBlocked),
    /// `conductor.dispatch.DispatchCancelled`.
    DispatchCancelled(crate::dispatch::DispatchCancelled),
    /// `conductor.dispatch.DispatchDone`.
    DispatchDone(crate::dispatch::DispatchDone),
    /// `conductor.dispatch.DispatchFailed`.
    DispatchFailed(crate::dispatch::DispatchFailed),
    /// `conductor.dispatch.DispatchSent`.
    DispatchSent(crate::dispatch::DispatchSent),
    /// `conductor.dispatch.DispatchStarted`.
    DispatchStarted(crate::dispatch::DispatchStarted),
    /// `conductor.dispatch.DispatchUnblocked`.
    DispatchUnblocked(crate::dispatch::DispatchUnblocked),
    /// `conductor.dispatch.GuardDecisionRecorded`.
    GuardDecisionRecorded(crate::dispatch::GuardDecisionRecorded),
    /// `conductor.dispatch.MessageHandled`.
    MessageHandled(crate::dispatch::MessageHandled),
    /// `conductor.dispatch.MessageReceived`.
    MessageReceived(crate::dispatch::MessageReceived),
    /// `conductor.dispatch.MessageRejected`.
    MessageRejected(crate::dispatch::MessageRejected),
    /// `conductor.dispatch.NeedRouted`.
    NeedRouted(crate::dispatch::NeedRouted),
    /// `conductor.dispatch.ResourceGranted`.
    ResourceGranted(crate::dispatch::ResourceGranted),
    /// `conductor.dispatch.ResourceRefused`.
    ResourceRefused(crate::dispatch::ResourceRefused),
    /// `conductor.dispatch.ResourceReleased`.
    ResourceReleased(crate::dispatch::ResourceReleased),
    /// `conductor.dispatch.ResourceRequested`.
    ResourceRequested(crate::dispatch::ResourceRequested),
    /// `conductor.host.DiskReclaimed`.
    DiskReclaimed(crate::host::DiskReclaimed),
    /// `conductor.host.DiskRequestWithdrawn`.
    DiskRequestWithdrawn(crate::host::DiskRequestWithdrawn),
    /// `conductor.host.DiskRequested`.
    DiskRequested(crate::host::DiskRequested),
    /// `conductor.host.DiskReservationRefused`.
    DiskReservationRefused(crate::host::DiskReservationRefused),
    /// `conductor.host.DiskReserved`.
    DiskReserved(crate::host::DiskReserved),
    /// `conductor.host.HostGrantReleased`.
    HostGrantReleased(crate::host::HostGrantReleased),
    /// `conductor.host.HostGrantTaken`.
    HostGrantTaken(crate::host::HostGrantTaken),
    /// `conductor.host.PeakRecorded`.
    PeakRecorded(crate::host::PeakRecorded),
    /// `conductor.host.ReclaimFloorNotReached`.
    ReclaimFloorNotReached(crate::host::ReclaimFloorNotReached),
    /// `conductor.host.RunStarted`.
    RunStarted(crate::host::RunStarted),
    /// `conductor.improvement.FindingClosed`.
    FindingClosed(crate::improvement::FindingClosed),
    /// `conductor.improvement.FindingFiled`.
    FindingFiled(crate::improvement::FindingFiled),
    /// `conductor.improvement.FindingRaised`.
    FindingRaised(crate::improvement::FindingRaised),
    /// `conductor.improvement.FindingVerified`.
    FindingVerified(crate::improvement::FindingVerified),
    /// `conductor.improvement.MetricDefined`.
    MetricDefined(crate::improvement::MetricDefined),
    /// `conductor.improvement.MetricReadingRecorded`.
    MetricReadingRecorded(crate::improvement::MetricReadingRecorded),
    /// `conductor.improvement.MetricReadingSuperseded`.
    MetricReadingSuperseded(crate::improvement::MetricReadingSuperseded),
    /// `conductor.improvement.MetricTargetSet`.
    MetricTargetSet(crate::improvement::MetricTargetSet),
    /// `conductor.observation.BlockerRecorded`.
    BlockerRecorded(crate::observation::BlockerRecorded),
    /// `conductor.observation.BoardPublished`.
    BoardPublished(crate::observation::BoardPublished),
    /// `conductor.observation.CollectorRunRecorded`.
    CollectorRunRecorded(crate::observation::CollectorRunRecorded),
    /// `conductor.observation.MemberFailureRecorded`.
    MemberFailureRecorded(crate::observation::MemberFailureRecorded),
    /// `conductor.observation.MergedPullRequestRecorded`.
    MergedPullRequestRecorded(crate::observation::MergedPullRequestRecorded),
    /// `conductor.observation.PullRequestRecorded`.
    PullRequestRecorded(crate::observation::PullRequestRecorded),
    /// `conductor.observation.ReleaseRecorded`.
    ReleaseRecorded(crate::observation::ReleaseRecorded),
    /// `conductor.observation.RepositoryRecorded`.
    RepositoryRecorded(crate::observation::RepositoryRecorded),
    /// `conductor.observation.SecretSyncRecorded`.
    SecretSyncRecorded(crate::observation::SecretSyncRecorded),
    /// `conductor.observation.SessionRecorded`.
    SessionRecorded(crate::observation::SessionRecorded),
    /// `conductor.observation.SnapshotCompleted`.
    SnapshotCompleted(crate::observation::SnapshotCompleted),
    /// `conductor.observation.SnapshotFailed`.
    SnapshotFailed(crate::observation::SnapshotFailed),
    /// `conductor.observation.SnapshotStarted`.
    SnapshotStarted(crate::observation::SnapshotStarted),
    /// `conductor.observation.SpecificationRecorded`.
    SpecificationRecorded(crate::observation::SpecificationRecorded),
    /// `conductor.observation.UsageRecorded`.
    UsageRecorded(crate::observation::UsageRecorded),
    /// `conductor.observation.WorkflowRunRecorded`.
    WorkflowRunRecorded(crate::observation::WorkflowRunRecorded),
    /// `conductor.session.LeaseTaken`.
    LeaseTaken(crate::session::LeaseTaken),
    /// `conductor.session.LeaseTakenOver`.
    LeaseTakenOver(crate::session::LeaseTakenOver),
}

/// conductor-cli — the port over the component's obligations.
///
/// `B` bundles every behaviour and query this component owes; constructing it over the domain's
/// `obligations::Unimplemented` yields a component that compiles and refuses, in the type system,
/// everything not yet implemented.
pub struct ConductorCli<B> {
    behaviors: B,
    outbox: Vec<PublishedEvent>,
}

impl<B> ConductorCli<B> {
    /// A new port over the given obligation implementations.
    pub fn new(behaviors: B) -> Self {
        Self {
            behaviors,
            outbox: Vec::new(),
        }
    }

    /// Hands over everything published since the last drain, in publication order.
    ///
    /// The system's transport calls this; anything else reading it is taking events the transport
    /// will then never deliver.
    pub fn drain_outbox(&mut self) -> Vec<PublishedEvent> {
        core::mem::take(&mut self.outbox)
    }
}

impl<B> ConductorCli<B>
where
    B: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::LeasesQuery,
{
    /// Accepts `conductor.decision.AmendEscalatedRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn amend_escalated_request(&mut self, input: crate::decision::AmendEscalatedRequest) -> Result<crate::decision::AmendEscalatedRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.amend_escalated_request(input)?;
        match &outcome {
            crate::decision::AmendEscalatedRequestOutcome::Amended { request_amended, .. } => {
                self.outbox.push(PublishedEvent::RequestAmended(request_amended.clone()));
            }
            crate::decision::AmendEscalatedRequestOutcome::NoSuchRequest { .. } => {}
            crate::decision::AmendEscalatedRequestOutcome::NotEscalated { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.AnswerEscalatedRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn answer_escalated_request(&mut self, input: crate::decision::AnswerEscalatedRequest) -> Result<crate::decision::AnswerEscalatedRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.answer_escalated_request(input)?;
        match &outcome {
            crate::decision::AnswerEscalatedRequestOutcome::NoSuchDecision { .. } => {}
            crate::decision::AnswerEscalatedRequestOutcome::Reversed { .. } => {}
            crate::decision::AnswerEscalatedRequestOutcome::Answered { request_answered, .. } => {
                self.outbox.push(PublishedEvent::RequestAnswered(request_answered.clone()));
            }
            crate::decision::AnswerEscalatedRequestOutcome::WrongState { .. } => {}
            crate::decision::AnswerEscalatedRequestOutcome::WrongStateUnknownInstance => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.AnswerRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn answer_request(&mut self, input: crate::decision::AnswerRequest) -> Result<crate::decision::AnswerRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.answer_request(input)?;
        match &outcome {
            crate::decision::AnswerRequestOutcome::NoSuchDecision { .. } => {}
            crate::decision::AnswerRequestOutcome::Reversed { .. } => {}
            crate::decision::AnswerRequestOutcome::Answered { request_answered, .. } => {
                self.outbox.push(PublishedEvent::RequestAnswered(request_answered.clone()));
            }
            crate::decision::AnswerRequestOutcome::WrongState { .. } => {}
            crate::decision::AnswerRequestOutcome::WrongStateUnknownInstance => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.EscalateRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn escalate_request(&mut self, input: crate::decision::EscalateRequest) -> Result<crate::decision::EscalateRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.escalate_request(input)?;
        match &outcome {
            crate::decision::EscalateRequestOutcome::ClassC { .. } => {}
            crate::decision::EscalateRequestOutcome::Untested { .. } => {}
            crate::decision::EscalateRequestOutcome::Incomplete { .. } => {}
            crate::decision::EscalateRequestOutcome::Escalated { request_escalated, .. } => {
                self.outbox.push(PublishedEvent::RequestEscalated(request_escalated.clone()));
            }
            crate::decision::EscalateRequestOutcome::NoSuchRequest { .. } => {}
            crate::decision::EscalateRequestOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.RaiseDecisionRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn raise_decision_request(&mut self, input: crate::decision::RaiseDecisionRequest) -> Result<crate::decision::RaiseDecisionRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.raise_decision_request(input)?;
        match &outcome {
            crate::decision::RaiseDecisionRequestOutcome::Raised { decision_requested, .. } => {
                self.outbox.push(PublishedEvent::DecisionRequested(decision_requested.clone()));
            }
            crate::decision::RaiseDecisionRequestOutcome::AlreadyRaised { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.RecordConductorDecision`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_conductor_decision(&mut self, input: crate::decision::RecordConductorDecision) -> Result<crate::decision::RecordConductorDecisionOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_conductor_decision(input)?;
        match &outcome {
            crate::decision::RecordConductorDecisionOutcome::Reserved { .. } => {}
            crate::decision::RecordConductorDecisionOutcome::Recorded { decision_recorded, .. } => {
                self.outbox.push(PublishedEvent::DecisionRecorded(decision_recorded.clone()));
            }
            crate::decision::RecordConductorDecisionOutcome::AlreadyRecorded { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.RecordOperatorDecision`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_operator_decision(&mut self, input: crate::decision::RecordOperatorDecision) -> Result<crate::decision::RecordOperatorDecisionOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_operator_decision(input)?;
        match &outcome {
            crate::decision::RecordOperatorDecisionOutcome::Recorded { decision_recorded, .. } => {
                self.outbox.push(PublishedEvent::DecisionRecorded(decision_recorded.clone()));
            }
            crate::decision::RecordOperatorDecisionOutcome::AlreadyRecorded { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.RecordOperatorDecisionForRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_operator_decision_for_request(&mut self, input: crate::decision::RecordOperatorDecisionForRequest) -> Result<crate::decision::RecordOperatorDecisionForRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_operator_decision_for_request(input)?;
        match &outcome {
            crate::decision::RecordOperatorDecisionForRequestOutcome::NoSuchRequest { .. } => {}
            crate::decision::RecordOperatorDecisionForRequestOutcome::NotEscalated { .. } => {}
            crate::decision::RecordOperatorDecisionForRequestOutcome::Recorded { decision_recorded, .. } => {
                self.outbox.push(PublishedEvent::DecisionRecorded(decision_recorded.clone()));
            }
            crate::decision::RecordOperatorDecisionForRequestOutcome::AlreadyRecorded { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.ReverseConductorDecision`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn reverse_conductor_decision(&mut self, input: crate::decision::ReverseConductorDecision) -> Result<crate::decision::ReverseConductorDecisionOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.reverse_conductor_decision(input)?;
        match &outcome {
            crate::decision::ReverseConductorDecisionOutcome::OperatorDecided { .. } => {}
            crate::decision::ReverseConductorDecisionOutcome::Reversed { decision_reversed_event, .. } => {
                self.outbox.push(PublishedEvent::DecisionReversedEvent(decision_reversed_event.clone()));
            }
            crate::decision::ReverseConductorDecisionOutcome::NoSuchDecision { .. } => {}
            crate::decision::ReverseConductorDecisionOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.ReverseOperatorDecision`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn reverse_operator_decision(&mut self, input: crate::decision::ReverseOperatorDecision) -> Result<crate::decision::ReverseOperatorDecisionOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.reverse_operator_decision(input)?;
        match &outcome {
            crate::decision::ReverseOperatorDecisionOutcome::Reversed { decision_reversed_event, .. } => {
                self.outbox.push(PublishedEvent::DecisionReversedEvent(decision_reversed_event.clone()));
            }
            crate::decision::ReverseOperatorDecisionOutcome::NoSuchDecision { .. } => {}
            crate::decision::ReverseOperatorDecisionOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.decision.WithdrawRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn withdraw_request(&mut self, input: crate::decision::WithdrawRequest) -> Result<crate::decision::WithdrawRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.withdraw_request(input)?;
        match &outcome {
            crate::decision::WithdrawRequestOutcome::Withdrawn { request_withdrawn, .. } => {
                self.outbox.push(PublishedEvent::RequestWithdrawn(request_withdrawn.clone()));
            }
            crate::decision::WithdrawRequestOutcome::NoSuchRequest { .. } => {}
            crate::decision::WithdrawRequestOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.ActivateRepository`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn activate_repository(&mut self, input: crate::direction::ActivateRepository) -> Result<crate::direction::ActivateRepositoryOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.activate_repository(input)?;
        match &outcome {
            crate::direction::ActivateRepositoryOutcome::Activated { repository_activated, .. } => {
                self.outbox.push(PublishedEvent::RepositoryActivated(repository_activated.clone()));
            }
            crate::direction::ActivateRepositoryOutcome::NoSuchMark { .. } => {}
            crate::direction::ActivateRepositoryOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.AddServing`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn add_serving(&mut self, input: crate::direction::AddServing) -> Result<crate::direction::AddServingOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.add_serving(input)?;
        match &outcome {
            crate::direction::AddServingOutcome::NoSuchGoal { .. } => {}
            crate::direction::AddServingOutcome::GoalClosed { .. } => {}
            crate::direction::AddServingOutcome::Added { serving_added, .. } => {
                self.outbox.push(PublishedEvent::ServingAdded(serving_added.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.ConfirmGoal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn confirm_goal(&mut self, input: crate::direction::ConfirmGoal) -> Result<crate::direction::ConfirmGoalOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.confirm_goal(input)?;
        match &outcome {
            crate::direction::ConfirmGoalOutcome::InvalidGoalId { .. } => {}
            crate::direction::ConfirmGoalOutcome::Confirmed { goal_confirmed, .. } => {
                self.outbox.push(PublishedEvent::GoalConfirmed(goal_confirmed.clone()));
            }
            crate::direction::ConfirmGoalOutcome::NoSuchGoal { .. } => {}
            crate::direction::ConfirmGoalOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.DeactivateRepository`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn deactivate_repository(&mut self, input: crate::direction::DeactivateRepository) -> Result<crate::direction::DeactivateRepositoryOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.deactivate_repository(input)?;
        match &outcome {
            crate::direction::DeactivateRepositoryOutcome::Deactivated { repository_deactivated, .. } => {
                self.outbox.push(PublishedEvent::RepositoryDeactivated(repository_deactivated.clone()));
            }
            crate::direction::DeactivateRepositoryOutcome::NoSuchMark { .. } => {}
            crate::direction::DeactivateRepositoryOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.DropGoal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn drop_goal(&mut self, input: crate::direction::DropGoal) -> Result<crate::direction::DropGoalOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.drop_goal(input)?;
        match &outcome {
            crate::direction::DropGoalOutcome::InvalidGoalId { .. } => {}
            crate::direction::DropGoalOutcome::Dropped { goal_dropped, .. } => {
                self.outbox.push(PublishedEvent::GoalDropped(goal_dropped.clone()));
            }
            crate::direction::DropGoalOutcome::NoSuchGoal { .. } => {}
            crate::direction::DropGoalOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.MarkGoalMet`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn mark_goal_met(&mut self, input: crate::direction::MarkGoalMet) -> Result<crate::direction::MarkGoalMetOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.mark_goal_met(input)?;
        match &outcome {
            crate::direction::MarkGoalMetOutcome::InvalidGoalId { .. } => {}
            crate::direction::MarkGoalMetOutcome::Met { goal_met, .. } => {
                self.outbox.push(PublishedEvent::GoalMet(goal_met.clone()));
            }
            crate::direction::MarkGoalMetOutcome::NoSuchGoal { .. } => {}
            crate::direction::MarkGoalMetOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.MarkRepository`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn mark_repository(&mut self, input: crate::direction::MarkRepository) -> Result<crate::direction::MarkRepositoryOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.mark_repository(input)?;
        match &outcome {
            crate::direction::MarkRepositoryOutcome::AlreadyMarked { .. } => {}
            crate::direction::MarkRepositoryOutcome::MarkedActive { repository_marked, .. } => {
                self.outbox.push(PublishedEvent::RepositoryMarked(repository_marked.clone()));
            }
            crate::direction::MarkRepositoryOutcome::MarkedInactive { repository_marked, .. } => {
                self.outbox.push(PublishedEvent::RepositoryMarked(repository_marked.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.ProposeGoal`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn propose_goal(&mut self, input: crate::direction::ProposeGoal) -> Result<crate::direction::ProposeGoalOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.propose_goal(input)?;
        match &outcome {
            crate::direction::ProposeGoalOutcome::InvalidGoalId { .. } => {}
            crate::direction::ProposeGoalOutcome::Proposed { goal_proposed, .. } => {
                self.outbox.push(PublishedEvent::GoalProposed(goal_proposed.clone()));
            }
            crate::direction::ProposeGoalOutcome::AlreadyProposed { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.direction.RemoveServing`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn remove_serving(&mut self, input: crate::direction::RemoveServing) -> Result<crate::direction::RemoveServingOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.remove_serving(input)?;
        match &outcome {
            crate::direction::RemoveServingOutcome::Removed { serving_removed, .. } => {
                self.outbox.push(PublishedEvent::ServingRemoved(serving_removed.clone()));
            }
            crate::direction::RemoveServingOutcome::NoSuchServing { .. } => {}
            crate::direction::RemoveServingOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.CancelDispatch`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn cancel_dispatch(&mut self, input: crate::dispatch::CancelDispatch) -> Result<crate::dispatch::CancelDispatchOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.cancel_dispatch(input)?;
        match &outcome {
            crate::dispatch::CancelDispatchOutcome::Cancelled { dispatch_cancelled, .. } => {
                self.outbox.push(PublishedEvent::DispatchCancelled(dispatch_cancelled.clone()));
            }
            crate::dispatch::CancelDispatchOutcome::NoSuchDispatch { .. } => {}
            crate::dispatch::CancelDispatchOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.GrantResource`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn grant_resource(&mut self, input: crate::dispatch::GrantResource) -> Result<crate::dispatch::GrantResourceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.grant_resource(input)?;
        match &outcome {
            crate::dispatch::GrantResourceOutcome::BelowDiskFloor { .. } => {}
            crate::dispatch::GrantResourceOutcome::Granted { resource_granted, .. } => {
                self.outbox.push(PublishedEvent::ResourceGranted(resource_granted.clone()));
            }
            crate::dispatch::GrantResourceOutcome::NoSuchRequest { .. } => {}
            crate::dispatch::GrantResourceOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.HandleMessage`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn handle_message(&mut self, input: crate::dispatch::HandleMessage) -> Result<crate::dispatch::HandleMessageOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.handle_message(input)?;
        match &outcome {
            crate::dispatch::HandleMessageOutcome::NeedUnrouted { .. } => {}
            crate::dispatch::HandleMessageOutcome::Handled { message_handled, .. } => {
                self.outbox.push(PublishedEvent::MessageHandled(message_handled.clone()));
            }
            crate::dispatch::HandleMessageOutcome::NoSuchMessage { .. } => {}
            crate::dispatch::HandleMessageOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.PauseController`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn pause_controller(&mut self, input: crate::dispatch::PauseController) -> Result<crate::dispatch::PauseControllerOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.pause_controller(input)?;
        match &outcome {
            crate::dispatch::PauseControllerOutcome::Paused { controller_paused, .. } => {
                self.outbox.push(PublishedEvent::ControllerPaused(controller_paused.clone()));
            }
            crate::dispatch::PauseControllerOutcome::NoSuchController { .. } => {}
            crate::dispatch::PauseControllerOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReceiveMessage`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn receive_message(&mut self, input: crate::dispatch::ReceiveMessage) -> Result<crate::dispatch::ReceiveMessageOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.receive_message(input)?;
        match &outcome {
            crate::dispatch::ReceiveMessageOutcome::Unparsed { message_received, .. } => {
                self.outbox.push(PublishedEvent::MessageReceived(message_received.clone()));
            }
            crate::dispatch::ReceiveMessageOutcome::Received { message_received, .. } => {
                self.outbox.push(PublishedEvent::MessageReceived(message_received.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.RecordGuardDecision`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_guard_decision(&mut self, input: crate::dispatch::RecordGuardDecision) -> Result<crate::dispatch::RecordGuardDecisionOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_guard_decision(input)?;
        match &outcome {
            crate::dispatch::RecordGuardDecisionOutcome::Recorded { guard_decision_recorded, .. } => {
                self.outbox.push(PublishedEvent::GuardDecisionRecorded(guard_decision_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.RefuseResource`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn refuse_resource(&mut self, input: crate::dispatch::RefuseResource) -> Result<crate::dispatch::RefuseResourceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.refuse_resource(input)?;
        match &outcome {
            crate::dispatch::RefuseResourceOutcome::Refused { resource_refused, .. } => {
                self.outbox.push(PublishedEvent::ResourceRefused(resource_refused.clone()));
            }
            crate::dispatch::RefuseResourceOutcome::NoSuchRequest { .. } => {}
            crate::dispatch::RefuseResourceOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.RejectMessage`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn reject_message(&mut self, input: crate::dispatch::RejectMessage) -> Result<crate::dispatch::RejectMessageOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.reject_message(input)?;
        match &outcome {
            crate::dispatch::RejectMessageOutcome::Rejected { message_rejected, .. } => {
                self.outbox.push(PublishedEvent::MessageRejected(message_rejected.clone()));
            }
            crate::dispatch::RejectMessageOutcome::NoSuchMessage { .. } => {}
            crate::dispatch::RejectMessageOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReleaseResource`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn release_resource(&mut self, input: crate::dispatch::ReleaseResource) -> Result<crate::dispatch::ReleaseResourceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.release_resource(input)?;
        match &outcome {
            crate::dispatch::ReleaseResourceOutcome::Released { resource_released, .. } => {
                self.outbox.push(PublishedEvent::ResourceReleased(resource_released.clone()));
            }
            crate::dispatch::ReleaseResourceOutcome::NoSuchRequest { .. } => {}
            crate::dispatch::ReleaseResourceOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReportBlocked`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn report_blocked(&mut self, input: crate::dispatch::ReportBlocked) -> Result<crate::dispatch::ReportBlockedOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.report_blocked(input)?;
        match &outcome {
            crate::dispatch::ReportBlockedOutcome::Blocked { dispatch_blocked, .. } => {
                self.outbox.push(PublishedEvent::DispatchBlocked(dispatch_blocked.clone()));
            }
            crate::dispatch::ReportBlockedOutcome::NoSuchDispatch { .. } => {}
            crate::dispatch::ReportBlockedOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReportDone`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn report_done(&mut self, input: crate::dispatch::ReportDone) -> Result<crate::dispatch::ReportDoneOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.report_done(input)?;
        match &outcome {
            crate::dispatch::ReportDoneOutcome::Done { dispatch_done, .. } => {
                self.outbox.push(PublishedEvent::DispatchDone(dispatch_done.clone()));
            }
            crate::dispatch::ReportDoneOutcome::NoSuchDispatch { .. } => {}
            crate::dispatch::ReportDoneOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReportFailed`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn report_failed(&mut self, input: crate::dispatch::ReportFailed) -> Result<crate::dispatch::ReportFailedOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.report_failed(input)?;
        match &outcome {
            crate::dispatch::ReportFailedOutcome::Failed { dispatch_failed, .. } => {
                self.outbox.push(PublishedEvent::DispatchFailed(dispatch_failed.clone()));
            }
            crate::dispatch::ReportFailedOutcome::NoSuchDispatch { .. } => {}
            crate::dispatch::ReportFailedOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReportStarted`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn report_started(&mut self, input: crate::dispatch::ReportStarted) -> Result<crate::dispatch::ReportStartedOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.report_started(input)?;
        match &outcome {
            crate::dispatch::ReportStartedOutcome::Started { dispatch_started, .. } => {
                self.outbox.push(PublishedEvent::DispatchStarted(dispatch_started.clone()));
            }
            crate::dispatch::ReportStartedOutcome::NoSuchDispatch { .. } => {}
            crate::dispatch::ReportStartedOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReportUnblocked`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn report_unblocked(&mut self, input: crate::dispatch::ReportUnblocked) -> Result<crate::dispatch::ReportUnblockedOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.report_unblocked(input)?;
        match &outcome {
            crate::dispatch::ReportUnblockedOutcome::Unblocked { dispatch_unblocked, .. } => {
                self.outbox.push(PublishedEvent::DispatchUnblocked(dispatch_unblocked.clone()));
            }
            crate::dispatch::ReportUnblockedOutcome::NoSuchDispatch { .. } => {}
            crate::dispatch::ReportUnblockedOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.RequestResource`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn request_resource(&mut self, input: crate::dispatch::RequestResource) -> Result<crate::dispatch::RequestResourceOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.request_resource(input)?;
        match &outcome {
            crate::dispatch::RequestResourceOutcome::InvalidAmount { .. } => {}
            crate::dispatch::RequestResourceOutcome::Requested { resource_requested, .. } => {
                self.outbox.push(PublishedEvent::ResourceRequested(resource_requested.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ResumeController`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn resume_controller(&mut self, input: crate::dispatch::ResumeController) -> Result<crate::dispatch::ResumeControllerOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.resume_controller(input)?;
        match &outcome {
            crate::dispatch::ResumeControllerOutcome::Resumed { controller_resumed, .. } => {
                self.outbox.push(PublishedEvent::ControllerResumed(controller_resumed.clone()));
            }
            crate::dispatch::ResumeControllerOutcome::NoSuchController { .. } => {}
            crate::dispatch::ResumeControllerOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.ReviseCharter`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn revise_charter(&mut self, input: crate::dispatch::ReviseCharter) -> Result<crate::dispatch::ReviseCharterOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.revise_charter(input)?;
        match &outcome {
            crate::dispatch::ReviseCharterOutcome::Revised { charter_revised, .. } => {
                self.outbox.push(PublishedEvent::CharterRevised(charter_revised.clone()));
            }
            crate::dispatch::ReviseCharterOutcome::PausedRevised { charter_revised, .. } => {
                self.outbox.push(PublishedEvent::CharterRevised(charter_revised.clone()));
            }
            crate::dispatch::ReviseCharterOutcome::NoSuchController { .. } => {}
            crate::dispatch::ReviseCharterOutcome::Stopped { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.RouteNeed`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn route_need(&mut self, input: crate::dispatch::RouteNeed) -> Result<crate::dispatch::RouteNeedOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.route_need(input)?;
        match &outcome {
            crate::dispatch::RouteNeedOutcome::NotANeed { .. } => {}
            crate::dispatch::RouteNeedOutcome::Routed { need_routed, .. } => {
                self.outbox.push(PublishedEvent::NeedRouted(need_routed.clone()));
            }
            crate::dispatch::RouteNeedOutcome::NoSuchMessage { .. } => {}
            crate::dispatch::RouteNeedOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.SendDispatch`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn send_dispatch(&mut self, input: crate::dispatch::SendDispatch) -> Result<crate::dispatch::SendDispatchOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.send_dispatch(input)?;
        match &outcome {
            crate::dispatch::SendDispatchOutcome::NegativePriority { .. } => {}
            crate::dispatch::SendDispatchOutcome::WaitsOnItself { .. } => {}
            crate::dispatch::SendDispatchOutcome::NoSuchGoal { .. } => {}
            crate::dispatch::SendDispatchOutcome::GoalClosed { .. } => {}
            crate::dispatch::SendDispatchOutcome::Sent { dispatch_sent, .. } => {
                self.outbox.push(PublishedEvent::DispatchSent(dispatch_sent.clone()));
            }
            crate::dispatch::SendDispatchOutcome::AlreadySent { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.StartController`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn start_controller(&mut self, input: crate::dispatch::StartController) -> Result<crate::dispatch::StartControllerOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.start_controller(input)?;
        match &outcome {
            crate::dispatch::StartControllerOutcome::AlreadyRunning { .. } => {}
            crate::dispatch::StartControllerOutcome::Started { controller_started, .. } => {
                self.outbox.push(PublishedEvent::ControllerStarted(controller_started.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.dispatch.StopController`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn stop_controller(&mut self, input: crate::dispatch::StopController) -> Result<crate::dispatch::StopControllerOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.stop_controller(input)?;
        match &outcome {
            crate::dispatch::StopControllerOutcome::Stopped { controller_stopped, .. } => {
                self.outbox.push(PublishedEvent::ControllerStopped(controller_stopped.clone()));
            }
            crate::dispatch::StopControllerOutcome::NoSuchController { .. } => {}
            crate::dispatch::StopControllerOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.Reclaim`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn reclaim(&mut self, input: crate::host::Reclaim) -> Result<crate::host::ReclaimOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.reclaim(input)?;
        match &outcome {
            crate::host::ReclaimOutcome::InvalidSize { .. } => {}
            crate::host::ReclaimOutcome::FloorNotReached { reclaim_floor_not_reached, .. } => {
                self.outbox.push(PublishedEvent::ReclaimFloorNotReached(reclaim_floor_not_reached.clone()));
            }
            crate::host::ReclaimOutcome::Reclaimed { disk_reclaimed, .. } => {
                self.outbox.push(PublishedEvent::DiskReclaimed(disk_reclaimed.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.RecordPeak`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_peak(&mut self, input: crate::host::RecordPeak) -> Result<crate::host::RecordPeakOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_peak(input)?;
        match &outcome {
            crate::host::RecordPeakOutcome::InvalidSize { .. } => {}
            crate::host::RecordPeakOutcome::Recorded { peak_recorded, .. } => {
                self.outbox.push(PublishedEvent::PeakRecorded(peak_recorded.clone()));
            }
            crate::host::RecordPeakOutcome::NoSuchGrant { .. } => {}
            crate::host::RecordPeakOutcome::NotRunning { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.ReleaseGrant`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn release_grant(&mut self, input: crate::host::ReleaseGrant) -> Result<crate::host::ReleaseGrantOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.release_grant(input)?;
        match &outcome {
            crate::host::ReleaseGrantOutcome::Released { host_grant_released, .. } => {
                self.outbox.push(PublishedEvent::HostGrantReleased(host_grant_released.clone()));
            }
            crate::host::ReleaseGrantOutcome::NoSuchGrant { .. } => {}
            crate::host::ReleaseGrantOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.RequestDisk`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn request_disk(&mut self, input: crate::host::RequestDisk) -> Result<crate::host::RequestDiskOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.request_disk(input)?;
        match &outcome {
            crate::host::RequestDiskOutcome::InvalidSize { .. } => {}
            crate::host::RequestDiskOutcome::Requested { disk_requested, .. } => {
                self.outbox.push(PublishedEvent::DiskRequested(disk_requested.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.ReserveDisk`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn reserve_disk(&mut self, input: crate::host::ReserveDisk) -> Result<crate::host::ReserveDiskOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.reserve_disk(input)?;
        match &outcome {
            crate::host::ReserveDiskOutcome::InvalidSize { .. } => {}
            crate::host::ReserveDiskOutcome::Refused { disk_reservation_refused, .. } => {
                self.outbox.push(PublishedEvent::DiskReservationRefused(disk_reservation_refused.clone()));
            }
            crate::host::ReserveDiskOutcome::GrantedAfterReclaim { disk_reserved, .. } => {
                self.outbox.push(PublishedEvent::DiskReserved(disk_reserved.clone()));
            }
            crate::host::ReserveDiskOutcome::Granted { disk_reserved, .. } => {
                self.outbox.push(PublishedEvent::DiskReserved(disk_reserved.clone()));
            }
            crate::host::ReserveDiskOutcome::NoSuchGrant { .. } => {}
            crate::host::ReserveDiskOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.StartRun`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn start_run(&mut self, input: crate::host::StartRun) -> Result<crate::host::StartRunOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.start_run(input)?;
        match &outcome {
            crate::host::StartRunOutcome::NotADiskGrant { .. } => {}
            crate::host::StartRunOutcome::Started { run_started, .. } => {
                self.outbox.push(PublishedEvent::RunStarted(run_started.clone()));
            }
            crate::host::StartRunOutcome::NoSuchGrant { .. } => {}
            crate::host::StartRunOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.TakeHostGrant`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn take_host_grant(&mut self, input: crate::host::TakeHostGrant) -> Result<crate::host::TakeHostGrantOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.take_host_grant(input)?;
        match &outcome {
            crate::host::TakeHostGrantOutcome::DiskThroughReservation { .. } => {}
            crate::host::TakeHostGrantOutcome::Granted { host_grant_taken, .. } => {
                self.outbox.push(PublishedEvent::HostGrantTaken(host_grant_taken.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.host.WithdrawDiskRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn withdraw_disk_request(&mut self, input: crate::host::WithdrawDiskRequest) -> Result<crate::host::WithdrawDiskRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.withdraw_disk_request(input)?;
        match &outcome {
            crate::host::WithdrawDiskRequestOutcome::Withdrawn { disk_request_withdrawn, .. } => {
                self.outbox.push(PublishedEvent::DiskRequestWithdrawn(disk_request_withdrawn.clone()));
            }
            crate::host::WithdrawDiskRequestOutcome::NoSuchGrant { .. } => {}
            crate::host::WithdrawDiskRequestOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.improvement.DefineMetric`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn define_metric(&mut self, input: crate::improvement::DefineMetric) -> Result<crate::improvement::DefineMetricOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.define_metric(input)?;
        match &outcome {
            crate::improvement::DefineMetricOutcome::Defined { metric_defined, .. } => {
                self.outbox.push(PublishedEvent::MetricDefined(metric_defined.clone()));
            }
            crate::improvement::DefineMetricOutcome::AlreadyDefined { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.improvement.FileFinding`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn file_finding(&mut self, input: crate::improvement::FileFinding) -> Result<crate::improvement::FileFindingOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.file_finding(input)?;
        match &outcome {
            crate::improvement::FileFindingOutcome::Filed { finding_filed, .. } => {
                self.outbox.push(PublishedEvent::FindingFiled(finding_filed.clone()));
            }
            crate::improvement::FileFindingOutcome::NoSuchFinding { .. } => {}
            crate::improvement::FileFindingOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.improvement.RaiseFinding`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn raise_finding(&mut self, input: crate::improvement::RaiseFinding) -> Result<crate::improvement::RaiseFindingOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.raise_finding(input)?;
        match &outcome {
            crate::improvement::RaiseFindingOutcome::NoSuchMetric { .. } => {}
            crate::improvement::RaiseFindingOutcome::AlreadyRaised { .. } => {}
            crate::improvement::RaiseFindingOutcome::Raised { finding_raised, .. } => {
                self.outbox.push(PublishedEvent::FindingRaised(finding_raised.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.improvement.RecordMetricReading`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_metric_reading(&mut self, input: crate::improvement::RecordMetricReading) -> Result<crate::improvement::RecordMetricReadingOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_metric_reading(input)?;
        match &outcome {
            crate::improvement::RecordMetricReadingOutcome::NoSuchMetric { .. } => {}
            crate::improvement::RecordMetricReadingOutcome::AlreadyRecorded { .. } => {}
            crate::improvement::RecordMetricReadingOutcome::Recorded { metric_reading_recorded, .. } => {
                self.outbox.push(PublishedEvent::MetricReadingRecorded(metric_reading_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.improvement.SetMetricTarget`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn set_metric_target(&mut self, input: crate::improvement::SetMetricTarget) -> Result<crate::improvement::SetMetricTargetOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.set_metric_target(input)?;
        match &outcome {
            crate::improvement::SetMetricTargetOutcome::Targeted { metric_target_set, .. } => {
                self.outbox.push(PublishedEvent::MetricTargetSet(metric_target_set.clone()));
            }
            crate::improvement::SetMetricTargetOutcome::Retargeted { metric_target_set, .. } => {
                self.outbox.push(PublishedEvent::MetricTargetSet(metric_target_set.clone()));
            }
            crate::improvement::SetMetricTargetOutcome::NoSuchMetric { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.improvement.SupersedeMetricReading`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn supersede_metric_reading(&mut self, input: crate::improvement::SupersedeMetricReading) -> Result<crate::improvement::SupersedeMetricReadingOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.supersede_metric_reading(input)?;
        match &outcome {
            crate::improvement::SupersedeMetricReadingOutcome::Superseded { metric_reading_superseded, .. } => {
                self.outbox.push(PublishedEvent::MetricReadingSuperseded(metric_reading_superseded.clone()));
            }
            crate::improvement::SupersedeMetricReadingOutcome::NoSuchReading { .. } => {}
            crate::improvement::SupersedeMetricReadingOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.improvement.VerifyFinding`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn verify_finding(&mut self, input: crate::improvement::VerifyFinding) -> Result<crate::improvement::VerifyFindingOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.verify_finding(input)?;
        match &outcome {
            crate::improvement::VerifyFindingOutcome::Verified { finding_verified, .. } => {
                self.outbox.push(PublishedEvent::FindingVerified(finding_verified.clone()));
            }
            crate::improvement::VerifyFindingOutcome::Closed { finding_closed, .. } => {
                self.outbox.push(PublishedEvent::FindingClosed(finding_closed.clone()));
            }
            crate::improvement::VerifyFindingOutcome::NoSuchFinding { .. } => {}
            crate::improvement::VerifyFindingOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.CompleteSnapshot`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn complete_snapshot(&mut self, input: crate::observation::CompleteSnapshot) -> Result<crate::observation::CompleteSnapshotOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.complete_snapshot(input)?;
        match &outcome {
            crate::observation::CompleteSnapshotOutcome::Completed { snapshot_completed, .. } => {
                self.outbox.push(PublishedEvent::SnapshotCompleted(snapshot_completed.clone()));
            }
            crate::observation::CompleteSnapshotOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::CompleteSnapshotOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.FailSnapshot`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn fail_snapshot(&mut self, input: crate::observation::FailSnapshot) -> Result<crate::observation::FailSnapshotOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.fail_snapshot(input)?;
        match &outcome {
            crate::observation::FailSnapshotOutcome::Failed { snapshot_failed, .. } => {
                self.outbox.push(PublishedEvent::SnapshotFailed(snapshot_failed.clone()));
            }
            crate::observation::FailSnapshotOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::FailSnapshotOutcome::WrongState { .. } => {}
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.PublishBoard`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn publish_board(&mut self, input: crate::observation::PublishBoard) -> Result<crate::observation::PublishBoardOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.publish_board(input)?;
        match &outcome {
            crate::observation::PublishBoardOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::PublishBoardOutcome::NotComplete { .. } => {}
            crate::observation::PublishBoardOutcome::NegativeRows { .. } => {}
            crate::observation::PublishBoardOutcome::Published { board_published, .. } => {
                self.outbox.push(PublishedEvent::BoardPublished(board_published.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordBlocker`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_blocker(&mut self, input: crate::observation::RecordBlocker) -> Result<crate::observation::RecordBlockerOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_blocker(input)?;
        match &outcome {
            crate::observation::RecordBlockerOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordBlockerOutcome::Closed { .. } => {}
            crate::observation::RecordBlockerOutcome::Recorded { blocker_recorded, .. } => {
                self.outbox.push(PublishedEvent::BlockerRecorded(blocker_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordCollectorRun`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_collector_run(&mut self, input: crate::observation::RecordCollectorRun) -> Result<crate::observation::RecordCollectorRunOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_collector_run(input)?;
        match &outcome {
            crate::observation::RecordCollectorRunOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordCollectorRunOutcome::Closed { .. } => {}
            crate::observation::RecordCollectorRunOutcome::NegativeDuration { .. } => {}
            crate::observation::RecordCollectorRunOutcome::Recorded { collector_run_recorded, .. } => {
                self.outbox.push(PublishedEvent::CollectorRunRecorded(collector_run_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordMemberFailure`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_member_failure(&mut self, input: crate::observation::RecordMemberFailure) -> Result<crate::observation::RecordMemberFailureOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_member_failure(input)?;
        match &outcome {
            crate::observation::RecordMemberFailureOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordMemberFailureOutcome::Closed { .. } => {}
            crate::observation::RecordMemberFailureOutcome::Recorded { member_failure_recorded, .. } => {
                self.outbox.push(PublishedEvent::MemberFailureRecorded(member_failure_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordMergedPullRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_merged_pull_request(&mut self, input: crate::observation::RecordMergedPullRequest) -> Result<crate::observation::RecordMergedPullRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_merged_pull_request(input)?;
        match &outcome {
            crate::observation::RecordMergedPullRequestOutcome::InvalidNumber { .. } => {}
            crate::observation::RecordMergedPullRequestOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordMergedPullRequestOutcome::Closed { .. } => {}
            crate::observation::RecordMergedPullRequestOutcome::Recorded { merged_pull_request_recorded, .. } => {
                self.outbox.push(PublishedEvent::MergedPullRequestRecorded(merged_pull_request_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordPullRequest`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_pull_request(&mut self, input: crate::observation::RecordPullRequest) -> Result<crate::observation::RecordPullRequestOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_pull_request(input)?;
        match &outcome {
            crate::observation::RecordPullRequestOutcome::InvalidNumber { .. } => {}
            crate::observation::RecordPullRequestOutcome::NegativeChecks { .. } => {}
            crate::observation::RecordPullRequestOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordPullRequestOutcome::Closed { .. } => {}
            crate::observation::RecordPullRequestOutcome::Recorded { pull_request_recorded, .. } => {
                self.outbox.push(PublishedEvent::PullRequestRecorded(pull_request_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordRelease`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_release(&mut self, input: crate::observation::RecordRelease) -> Result<crate::observation::RecordReleaseOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_release(input)?;
        match &outcome {
            crate::observation::RecordReleaseOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordReleaseOutcome::Closed { .. } => {}
            crate::observation::RecordReleaseOutcome::Recorded { release_recorded, .. } => {
                self.outbox.push(PublishedEvent::ReleaseRecorded(release_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordRepository`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_repository(&mut self, input: crate::observation::RecordRepository) -> Result<crate::observation::RecordRepositoryOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_repository(input)?;
        match &outcome {
            crate::observation::RecordRepositoryOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordRepositoryOutcome::Closed { .. } => {}
            crate::observation::RecordRepositoryOutcome::Recorded { repository_recorded, .. } => {
                self.outbox.push(PublishedEvent::RepositoryRecorded(repository_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordSecretSync`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_secret_sync(&mut self, input: crate::observation::RecordSecretSync) -> Result<crate::observation::RecordSecretSyncOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_secret_sync(input)?;
        match &outcome {
            crate::observation::RecordSecretSyncOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordSecretSyncOutcome::Closed { .. } => {}
            crate::observation::RecordSecretSyncOutcome::Recorded { secret_sync_recorded, .. } => {
                self.outbox.push(PublishedEvent::SecretSyncRecorded(secret_sync_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordSession`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_session(&mut self, input: crate::observation::RecordSession) -> Result<crate::observation::RecordSessionOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_session(input)?;
        match &outcome {
            crate::observation::RecordSessionOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordSessionOutcome::Closed { .. } => {}
            crate::observation::RecordSessionOutcome::Recorded { session_recorded, .. } => {
                self.outbox.push(PublishedEvent::SessionRecorded(session_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordSpecification`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_specification(&mut self, input: crate::observation::RecordSpecification) -> Result<crate::observation::RecordSpecificationOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_specification(input)?;
        match &outcome {
            crate::observation::RecordSpecificationOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordSpecificationOutcome::Closed { .. } => {}
            crate::observation::RecordSpecificationOutcome::NegativeRefusals { .. } => {}
            crate::observation::RecordSpecificationOutcome::Recorded { specification_recorded, .. } => {
                self.outbox.push(PublishedEvent::SpecificationRecorded(specification_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordUsage`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_usage(&mut self, input: crate::observation::RecordUsage) -> Result<crate::observation::RecordUsageOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_usage(input)?;
        match &outcome {
            crate::observation::RecordUsageOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordUsageOutcome::Closed { .. } => {}
            crate::observation::RecordUsageOutcome::NegativePercent { .. } => {}
            crate::observation::RecordUsageOutcome::Recorded { usage_recorded, .. } => {
                self.outbox.push(PublishedEvent::UsageRecorded(usage_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.RecordWorkflowRun`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn record_workflow_run(&mut self, input: crate::observation::RecordWorkflowRun) -> Result<crate::observation::RecordWorkflowRunOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.record_workflow_run(input)?;
        match &outcome {
            crate::observation::RecordWorkflowRunOutcome::NoSuchSnapshot { .. } => {}
            crate::observation::RecordWorkflowRunOutcome::Closed { .. } => {}
            crate::observation::RecordWorkflowRunOutcome::Recorded { workflow_run_recorded, .. } => {
                self.outbox.push(PublishedEvent::WorkflowRunRecorded(workflow_run_recorded.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.observation.StartSnapshot`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn start_snapshot(&mut self, input: crate::observation::StartSnapshot) -> Result<crate::observation::StartSnapshotOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.start_snapshot(input)?;
        match &outcome {
            crate::observation::StartSnapshotOutcome::NegativeDisk { .. } => {}
            crate::observation::StartSnapshotOutcome::Started { snapshot_started, .. } => {
                self.outbox.push(PublishedEvent::SnapshotStarted(snapshot_started.clone()));
            }
        }
        Ok(outcome)
    }

    /// Accepts `conductor.session.ClaimLease`: runs the behaviour obligation, then publishes the declared events
    /// the outcome carries.
    ///
    /// `Err` is the typed refusal of an unmet obligation — never a domain outcome, which always
    /// arrives as a variant of the outcome type, refusals included.
    pub fn claim_lease(&mut self, input: crate::session::ClaimLease) -> Result<crate::session::ClaimLeaseOutcome, crate::obligation::UnmetObligation> {
        let outcome = self.behaviors.claim_lease(input)?;
        match &outcome {
            crate::session::ClaimLeaseOutcome::Taken { lease_taken, .. } => {
                self.outbox.push(PublishedEvent::LeaseTaken(lease_taken.clone()));
            }
            crate::session::ClaimLeaseOutcome::Renewed { lease_taken, .. } => {
                self.outbox.push(PublishedEvent::LeaseTaken(lease_taken.clone()));
            }
            crate::session::ClaimLeaseOutcome::Held { .. } => {}
            crate::session::ClaimLeaseOutcome::TakenOver { lease_taken_over, .. } => {
                self.outbox.push(PublishedEvent::LeaseTakenOver(lease_taken_over.clone()));
            }
        }
        Ok(outcome)
    }

    /// Serves `conductor.decision.Decisions` at `read_your_writes` consistency, from the owed projection.
    pub fn decisions(&self) -> Result<Vec<crate::decision::Decisions>, crate::obligation::UnmetObligation> {
        self.behaviors.decisions()
    }

    /// Serves `conductor.decision.HandsTodo` at `read_your_writes` consistency, from the owed projection.
    pub fn hands_todo(&self) -> Result<Vec<crate::decision::HandsTodo>, crate::obligation::UnmetObligation> {
        self.behaviors.hands_todo()
    }

    /// Serves `conductor.decision.Requests` at `read_your_writes` consistency, from the owed projection.
    pub fn requests(&self) -> Result<Vec<crate::decision::Requests>, crate::obligation::UnmetObligation> {
        self.behaviors.requests()
    }

    /// Serves `conductor.direction.GoalServings` at `read_your_writes` consistency, from the owed projection.
    pub fn goal_servings(&self) -> Result<Vec<crate::direction::GoalServings>, crate::obligation::UnmetObligation> {
        self.behaviors.goal_servings()
    }

    /// Serves `conductor.direction.Goals` at `read_your_writes` consistency, from the owed projection.
    pub fn goals(&self) -> Result<Vec<crate::direction::Goals>, crate::obligation::UnmetObligation> {
        self.behaviors.goals()
    }

    /// Serves `conductor.direction.RepositoryMarks` at `read_your_writes` consistency, from the owed projection.
    pub fn repository_marks(&self) -> Result<Vec<crate::direction::RepositoryMarks>, crate::obligation::UnmetObligation> {
        self.behaviors.repository_marks()
    }

    /// Serves `conductor.dispatch.Controllers` at `read_your_writes` consistency, from the owed projection.
    pub fn controllers(&self) -> Result<Vec<crate::dispatch::Controllers>, crate::obligation::UnmetObligation> {
        self.behaviors.controllers()
    }

    /// Serves `conductor.dispatch.Dispatches` at `read_your_writes` consistency, from the owed projection.
    pub fn dispatches(&self) -> Result<Vec<crate::dispatch::Dispatches>, crate::obligation::UnmetObligation> {
        self.behaviors.dispatches()
    }

    /// Serves `conductor.dispatch.GuardDecisions` at `read_your_writes` consistency, from the owed projection.
    pub fn guard_decisions(&self) -> Result<Vec<crate::dispatch::GuardDecisions>, crate::obligation::UnmetObligation> {
        self.behaviors.guard_decisions()
    }

    /// Serves `conductor.dispatch.Messages` at `read_your_writes` consistency, from the owed projection.
    pub fn messages(&self) -> Result<Vec<crate::dispatch::Messages>, crate::obligation::UnmetObligation> {
        self.behaviors.messages()
    }

    /// Serves `conductor.dispatch.ResourceRequests` at `read_your_writes` consistency, from the owed projection.
    pub fn resource_requests(&self) -> Result<Vec<crate::dispatch::ResourceRequests>, crate::obligation::UnmetObligation> {
        self.behaviors.resource_requests()
    }

    /// Serves `conductor.host.Grants` at `read_your_writes` consistency, from the owed projection.
    pub fn grants(&self) -> Result<Vec<crate::host::Grants>, crate::obligation::UnmetObligation> {
        self.behaviors.grants()
    }

    /// Serves `conductor.host.ReclaimRuns` at `read_your_writes` consistency, from the owed projection.
    pub fn reclaim_runs(&self) -> Result<Vec<crate::host::ReclaimRuns>, crate::obligation::UnmetObligation> {
        self.behaviors.reclaim_runs()
    }

    /// Serves `conductor.improvement.Findings` at `read_your_writes` consistency, from the owed projection.
    pub fn findings(&self) -> Result<Vec<crate::improvement::Findings>, crate::obligation::UnmetObligation> {
        self.behaviors.findings()
    }

    /// Serves `conductor.improvement.KpiReadings` at `read_your_writes` consistency, from the owed projection.
    pub fn kpi_readings(&self) -> Result<Vec<crate::improvement::KpiReadings>, crate::obligation::UnmetObligation> {
        self.behaviors.kpi_readings()
    }

    /// Serves `conductor.improvement.Kpis` at `read_your_writes` consistency, from the owed projection.
    pub fn kpis(&self) -> Result<Vec<crate::improvement::Kpis>, crate::obligation::UnmetObligation> {
        self.behaviors.kpis()
    }

    /// Serves `conductor.improvement.MetricReadings` at `read_your_writes` consistency, from the owed projection.
    pub fn metric_readings(&self) -> Result<Vec<crate::improvement::MetricReadings>, crate::obligation::UnmetObligation> {
        self.behaviors.metric_readings()
    }

    /// Serves `conductor.improvement.Metrics` at `read_your_writes` consistency, from the owed projection.
    pub fn metrics(&self) -> Result<Vec<crate::improvement::Metrics>, crate::obligation::UnmetObligation> {
        self.behaviors.metrics()
    }

    /// Serves `conductor.observation.Blockers` at `read_your_writes` consistency, from the owed projection.
    pub fn blockers(&self) -> Result<Vec<crate::observation::Blockers>, crate::obligation::UnmetObligation> {
        self.behaviors.blockers()
    }

    /// Serves `conductor.observation.Boards` at `read_your_writes` consistency, from the owed projection.
    pub fn boards(&self) -> Result<Vec<crate::observation::Boards>, crate::obligation::UnmetObligation> {
        self.behaviors.boards()
    }

    /// Serves `conductor.observation.CollectorRuns` at `read_your_writes` consistency, from the owed projection.
    pub fn collector_runs(&self) -> Result<Vec<crate::observation::CollectorRuns>, crate::obligation::UnmetObligation> {
        self.behaviors.collector_runs()
    }

    /// Serves `conductor.observation.MemberFailures` at `read_your_writes` consistency, from the owed projection.
    pub fn member_failures(&self) -> Result<Vec<crate::observation::MemberFailures>, crate::obligation::UnmetObligation> {
        self.behaviors.member_failures()
    }

    /// Serves `conductor.observation.MergedPullRequests` at `read_your_writes` consistency, from the owed projection.
    pub fn merged_pull_requests(&self) -> Result<Vec<crate::observation::MergedPullRequests>, crate::obligation::UnmetObligation> {
        self.behaviors.merged_pull_requests()
    }

    /// Serves `conductor.observation.PullRequests` at `read_your_writes` consistency, from the owed projection.
    pub fn pull_requests(&self) -> Result<Vec<crate::observation::PullRequests>, crate::obligation::UnmetObligation> {
        self.behaviors.pull_requests()
    }

    /// Serves `conductor.observation.Releases` at `read_your_writes` consistency, from the owed projection.
    pub fn releases(&self) -> Result<Vec<crate::observation::Releases>, crate::obligation::UnmetObligation> {
        self.behaviors.releases()
    }

    /// Serves `conductor.observation.Repositories` at `read_your_writes` consistency, from the owed projection.
    pub fn repositories(&self) -> Result<Vec<crate::observation::Repositories>, crate::obligation::UnmetObligation> {
        self.behaviors.repositories()
    }

    /// Serves `conductor.observation.SecretSyncs` at `read_your_writes` consistency, from the owed projection.
    pub fn secret_syncs(&self) -> Result<Vec<crate::observation::SecretSyncs>, crate::obligation::UnmetObligation> {
        self.behaviors.secret_syncs()
    }

    /// Serves `conductor.observation.Sessions` at `read_your_writes` consistency, from the owed projection.
    pub fn sessions(&self) -> Result<Vec<crate::observation::Sessions>, crate::obligation::UnmetObligation> {
        self.behaviors.sessions()
    }

    /// Serves `conductor.observation.Snapshots` at `read_your_writes` consistency, from the owed projection.
    pub fn snapshots(&self) -> Result<Vec<crate::observation::Snapshots>, crate::obligation::UnmetObligation> {
        self.behaviors.snapshots()
    }

    /// Serves `conductor.observation.Specifications` at `read_your_writes` consistency, from the owed projection.
    pub fn specifications(&self) -> Result<Vec<crate::observation::Specifications>, crate::obligation::UnmetObligation> {
        self.behaviors.specifications()
    }

    /// Serves `conductor.observation.UsageReadings` at `read_your_writes` consistency, from the owed projection.
    pub fn usage_readings(&self) -> Result<Vec<crate::observation::UsageReadings>, crate::obligation::UnmetObligation> {
        self.behaviors.usage_readings()
    }

    /// Serves `conductor.observation.WorkflowRuns` at `read_your_writes` consistency, from the owed projection.
    pub fn workflow_runs(&self) -> Result<Vec<crate::observation::WorkflowRuns>, crate::obligation::UnmetObligation> {
        self.behaviors.workflow_runs()
    }

    /// Serves `conductor.session.Leases` at `read_your_writes` consistency, from the owed projection.
    pub fn leases(&self) -> Result<Vec<crate::session::Leases>, crate::obligation::UnmetObligation> {
        self.behaviors.leases()
    }
}
