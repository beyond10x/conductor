// generated from conductor v1
// model digest 4cdc1583c495815bb1f15bbff67865887fce3772221709ce841063639727baec
// contract digest 5e6c57bf0e16880ffc89e58f486bbcb4b52c9695c9634aba27a490a2530b3eb7
// do not edit: regenerate with `ess synthesize --layout crate`

//! The `conductor` system, v1: its components assembled, its bindings wired, and its one transport.
//!
//! The transport is derived from the specification, not chosen: `at_least_once` is the only
//! delivery guarantee the model declares, so published events land on an append-only log and a
//! pump delivers each to every binding that reacts to it. The log is the system's observable
//! record, and so is the record of what each binding invoked. What no specification determines
//! — how an escalation event is filled, behaviour behind the ports — stays an obligation; see
//! the `PLAN.md` beside this workspace.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

/// An event on the system's log: everything any component publishes, and everything a binding
/// escalates into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SystemEvent {
    /// `conductor.decision.DecisionRecorded`.
    DecisionRecorded(crate::decision::DecisionRecorded),
    /// `conductor.decision.DecisionRequested`.
    DecisionRequested(crate::decision::DecisionRequested),
    /// `conductor.decision.DecisionReversedEvent`.
    DecisionReversedEvent(crate::decision::DecisionReversedEvent),
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
    /// `conductor.observation.BlockerRecorded`.
    BlockerRecorded(crate::observation::BlockerRecorded),
    /// `conductor.observation.BoardPublished`.
    BoardPublished(crate::observation::BoardPublished),
    /// `conductor.observation.MergedPullRequestRecorded`.
    MergedPullRequestRecorded(crate::observation::MergedPullRequestRecorded),
    /// `conductor.observation.PullRequestRecorded`.
    PullRequestRecorded(crate::observation::PullRequestRecorded),
    /// `conductor.observation.ReleaseRecorded`.
    ReleaseRecorded(crate::observation::ReleaseRecorded),
    /// `conductor.observation.RepositoryRecorded`.
    RepositoryRecorded(crate::observation::RepositoryRecorded),
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
    /// `conductor.observation.WorkflowRunRecorded`.
    WorkflowRunRecorded(crate::observation::WorkflowRunRecorded),
}

impl SystemEvent {
    /// The qualified name the specification declares this event under.
    pub fn name(&self) -> &'static str {
        match self {
            Self::DecisionRecorded(_) => "conductor.decision.DecisionRecorded",
            Self::DecisionRequested(_) => "conductor.decision.DecisionRequested",
            Self::DecisionReversedEvent(_) => "conductor.decision.DecisionReversedEvent",
            Self::RequestAnswered(_) => "conductor.decision.RequestAnswered",
            Self::RequestEscalated(_) => "conductor.decision.RequestEscalated",
            Self::RequestWithdrawn(_) => "conductor.decision.RequestWithdrawn",
            Self::GoalConfirmed(_) => "conductor.direction.GoalConfirmed",
            Self::GoalDropped(_) => "conductor.direction.GoalDropped",
            Self::GoalMet(_) => "conductor.direction.GoalMet",
            Self::GoalProposed(_) => "conductor.direction.GoalProposed",
            Self::RepositoryActivated(_) => "conductor.direction.RepositoryActivated",
            Self::RepositoryDeactivated(_) => "conductor.direction.RepositoryDeactivated",
            Self::RepositoryMarked(_) => "conductor.direction.RepositoryMarked",
            Self::ServingAdded(_) => "conductor.direction.ServingAdded",
            Self::ServingRemoved(_) => "conductor.direction.ServingRemoved",
            Self::CharterRevised(_) => "conductor.dispatch.CharterRevised",
            Self::ControllerPaused(_) => "conductor.dispatch.ControllerPaused",
            Self::ControllerResumed(_) => "conductor.dispatch.ControllerResumed",
            Self::ControllerStarted(_) => "conductor.dispatch.ControllerStarted",
            Self::ControllerStopped(_) => "conductor.dispatch.ControllerStopped",
            Self::DispatchBlocked(_) => "conductor.dispatch.DispatchBlocked",
            Self::DispatchCancelled(_) => "conductor.dispatch.DispatchCancelled",
            Self::DispatchDone(_) => "conductor.dispatch.DispatchDone",
            Self::DispatchFailed(_) => "conductor.dispatch.DispatchFailed",
            Self::DispatchSent(_) => "conductor.dispatch.DispatchSent",
            Self::DispatchStarted(_) => "conductor.dispatch.DispatchStarted",
            Self::DispatchUnblocked(_) => "conductor.dispatch.DispatchUnblocked",
            Self::GuardDecisionRecorded(_) => "conductor.dispatch.GuardDecisionRecorded",
            Self::MessageHandled(_) => "conductor.dispatch.MessageHandled",
            Self::MessageReceived(_) => "conductor.dispatch.MessageReceived",
            Self::MessageRejected(_) => "conductor.dispatch.MessageRejected",
            Self::NeedRouted(_) => "conductor.dispatch.NeedRouted",
            Self::ResourceGranted(_) => "conductor.dispatch.ResourceGranted",
            Self::ResourceRefused(_) => "conductor.dispatch.ResourceRefused",
            Self::ResourceReleased(_) => "conductor.dispatch.ResourceReleased",
            Self::ResourceRequested(_) => "conductor.dispatch.ResourceRequested",
            Self::BlockerRecorded(_) => "conductor.observation.BlockerRecorded",
            Self::BoardPublished(_) => "conductor.observation.BoardPublished",
            Self::MergedPullRequestRecorded(_) => "conductor.observation.MergedPullRequestRecorded",
            Self::PullRequestRecorded(_) => "conductor.observation.PullRequestRecorded",
            Self::ReleaseRecorded(_) => "conductor.observation.ReleaseRecorded",
            Self::RepositoryRecorded(_) => "conductor.observation.RepositoryRecorded",
            Self::SessionRecorded(_) => "conductor.observation.SessionRecorded",
            Self::SnapshotCompleted(_) => "conductor.observation.SnapshotCompleted",
            Self::SnapshotFailed(_) => "conductor.observation.SnapshotFailed",
            Self::SnapshotStarted(_) => "conductor.observation.SnapshotStarted",
            Self::SpecificationRecorded(_) => "conductor.observation.SpecificationRecorded",
            Self::WorkflowRunRecorded(_) => "conductor.observation.WorkflowRunRecorded",
        }
    }
}

impl From<crate::ports::conductor_cli::PublishedEvent> for SystemEvent {
    fn from(event: crate::ports::conductor_cli::PublishedEvent) -> Self {
        match event {
            crate::ports::conductor_cli::PublishedEvent::DecisionRecorded(event) => Self::DecisionRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::DecisionRequested(event) => Self::DecisionRequested(event),
            crate::ports::conductor_cli::PublishedEvent::DecisionReversedEvent(event) => Self::DecisionReversedEvent(event),
            crate::ports::conductor_cli::PublishedEvent::RequestAnswered(event) => Self::RequestAnswered(event),
            crate::ports::conductor_cli::PublishedEvent::RequestEscalated(event) => Self::RequestEscalated(event),
            crate::ports::conductor_cli::PublishedEvent::RequestWithdrawn(event) => Self::RequestWithdrawn(event),
            crate::ports::conductor_cli::PublishedEvent::GoalConfirmed(event) => Self::GoalConfirmed(event),
            crate::ports::conductor_cli::PublishedEvent::GoalDropped(event) => Self::GoalDropped(event),
            crate::ports::conductor_cli::PublishedEvent::GoalMet(event) => Self::GoalMet(event),
            crate::ports::conductor_cli::PublishedEvent::GoalProposed(event) => Self::GoalProposed(event),
            crate::ports::conductor_cli::PublishedEvent::RepositoryActivated(event) => Self::RepositoryActivated(event),
            crate::ports::conductor_cli::PublishedEvent::RepositoryDeactivated(event) => Self::RepositoryDeactivated(event),
            crate::ports::conductor_cli::PublishedEvent::RepositoryMarked(event) => Self::RepositoryMarked(event),
            crate::ports::conductor_cli::PublishedEvent::ServingAdded(event) => Self::ServingAdded(event),
            crate::ports::conductor_cli::PublishedEvent::ServingRemoved(event) => Self::ServingRemoved(event),
            crate::ports::conductor_cli::PublishedEvent::CharterRevised(event) => Self::CharterRevised(event),
            crate::ports::conductor_cli::PublishedEvent::ControllerPaused(event) => Self::ControllerPaused(event),
            crate::ports::conductor_cli::PublishedEvent::ControllerResumed(event) => Self::ControllerResumed(event),
            crate::ports::conductor_cli::PublishedEvent::ControllerStarted(event) => Self::ControllerStarted(event),
            crate::ports::conductor_cli::PublishedEvent::ControllerStopped(event) => Self::ControllerStopped(event),
            crate::ports::conductor_cli::PublishedEvent::DispatchBlocked(event) => Self::DispatchBlocked(event),
            crate::ports::conductor_cli::PublishedEvent::DispatchCancelled(event) => Self::DispatchCancelled(event),
            crate::ports::conductor_cli::PublishedEvent::DispatchDone(event) => Self::DispatchDone(event),
            crate::ports::conductor_cli::PublishedEvent::DispatchFailed(event) => Self::DispatchFailed(event),
            crate::ports::conductor_cli::PublishedEvent::DispatchSent(event) => Self::DispatchSent(event),
            crate::ports::conductor_cli::PublishedEvent::DispatchStarted(event) => Self::DispatchStarted(event),
            crate::ports::conductor_cli::PublishedEvent::DispatchUnblocked(event) => Self::DispatchUnblocked(event),
            crate::ports::conductor_cli::PublishedEvent::GuardDecisionRecorded(event) => Self::GuardDecisionRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::MessageHandled(event) => Self::MessageHandled(event),
            crate::ports::conductor_cli::PublishedEvent::MessageReceived(event) => Self::MessageReceived(event),
            crate::ports::conductor_cli::PublishedEvent::MessageRejected(event) => Self::MessageRejected(event),
            crate::ports::conductor_cli::PublishedEvent::NeedRouted(event) => Self::NeedRouted(event),
            crate::ports::conductor_cli::PublishedEvent::ResourceGranted(event) => Self::ResourceGranted(event),
            crate::ports::conductor_cli::PublishedEvent::ResourceRefused(event) => Self::ResourceRefused(event),
            crate::ports::conductor_cli::PublishedEvent::ResourceReleased(event) => Self::ResourceReleased(event),
            crate::ports::conductor_cli::PublishedEvent::ResourceRequested(event) => Self::ResourceRequested(event),
            crate::ports::conductor_cli::PublishedEvent::BlockerRecorded(event) => Self::BlockerRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::BoardPublished(event) => Self::BoardPublished(event),
            crate::ports::conductor_cli::PublishedEvent::MergedPullRequestRecorded(event) => Self::MergedPullRequestRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::PullRequestRecorded(event) => Self::PullRequestRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::ReleaseRecorded(event) => Self::ReleaseRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::RepositoryRecorded(event) => Self::RepositoryRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::SessionRecorded(event) => Self::SessionRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::SnapshotCompleted(event) => Self::SnapshotCompleted(event),
            crate::ports::conductor_cli::PublishedEvent::SnapshotFailed(event) => Self::SnapshotFailed(event),
            crate::ports::conductor_cli::PublishedEvent::SnapshotStarted(event) => Self::SnapshotStarted(event),
            crate::ports::conductor_cli::PublishedEvent::SpecificationRecorded(event) => Self::SpecificationRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::WorkflowRunRecorded(event) => Self::WorkflowRunRecorded(event),
        }
    }
}

/// The `conductor` system: every component behind its port, and the transport between them.
///
/// The component fields are public because commands enter the system through a component's own
/// port; the log and its delivery cursor are not, because publishing happens by pumping, not by
/// writing history directly.
pub struct System<ConductorCliBehaviors> {
    /// The `conductor-cli` component.
    pub conductor_cli: crate::ports::conductor_cli::ConductorCli<ConductorCliBehaviors>,
    published: Vec<SystemEvent>,
    cursor: usize,
}

impl<ConductorCliBehaviors> System<ConductorCliBehaviors> {
    /// Assembles the system from its components.
    pub fn new(conductor_cli: crate::ports::conductor_cli::ConductorCli<ConductorCliBehaviors>) -> Self {
        Self {
            conductor_cli,
            published: Vec::new(),
            cursor: 0,
        }
    }

    /// Everything published so far, in publication order — the system's observable record.
    pub fn published(&self) -> &[SystemEvent] {
        &self.published
    }

    /// Takes every event the pump has already delivered off the log, in publication order.
    ///
    /// A long-running shell calls this after each `pump`, or the log holds every event the
    /// process ever published. A `pump` returns with every logged event delivered: each
    /// reacting binding has had its attempt, and a binding whose attempt stopped holds the event in
    /// its own held-back list, not on the log. Events published since the last `pump` stay on the
    /// log, so the next `pump` still delivers them; taking never skips a binding.
    pub fn take_published(&mut self) -> Vec<SystemEvent> {
        let delivered: Vec<SystemEvent> = self.published.drain(..self.cursor).collect();
        self.cursor = 0;
        delivered
    }
}

impl<ConductorCliBehaviors> System<ConductorCliBehaviors>
where
    ConductorCliBehaviors: crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::WorkflowRunsQuery,
{
    /// Delivers until quiescent: collects every component's outbox onto the log. No binding
    /// reacts to anything this specification publishes, so collecting is the whole delivery.
    pub fn pump(&mut self) -> Result<(), crate::obligation::UnmetObligation> {
        loop {
            self.collect();
            if self.cursor == self.published.len() {
                return Ok(());
            }
            self.cursor += 1;
        }
    }

    /// Moves every component's outbox onto the log, in component order.
    fn collect(&mut self) {
        for event in self.conductor_cli.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
    }
}
