// generated from conductor v1
// contract digest 2fc2642808077fb28855408e63bbd2024df0cf9fb9146a58853be9bd124f1c4b
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
    /// `conductor.agent.AgentKindDeclared`.
    AgentKindDeclared(crate::agent::AgentKindDeclared),
    /// `conductor.agent.AgentReported`.
    AgentReported(crate::agent::AgentReported),
    /// `conductor.agent.AgentStarted`.
    AgentStarted(crate::agent::AgentStarted),
    /// `conductor.agent.AgentStopped`.
    AgentStopped(crate::agent::AgentStopped),
    /// `conductor.config.ConfigChanged`.
    ConfigChanged(crate::config::ConfigChanged),
    /// `conductor.config.InstanceAdded`.
    InstanceAdded(crate::config::InstanceAdded),
    /// `conductor.config.InstanceRemoved`.
    InstanceRemoved(crate::config::InstanceRemoved),
    /// `conductor.config.InstanceReplaced`.
    InstanceReplaced(crate::config::InstanceReplaced),
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
    /// `conductor.dispatch.DispatchLinked`.
    DispatchLinked(crate::dispatch::DispatchLinked),
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
    /// `conductor.improvement.MetricReadingJudged`.
    MetricReadingJudged(crate::improvement::MetricReadingJudged),
    /// `conductor.improvement.MetricReadingRecorded`.
    MetricReadingRecorded(crate::improvement::MetricReadingRecorded),
    /// `conductor.improvement.MetricReadingSuperseded`.
    MetricReadingSuperseded(crate::improvement::MetricReadingSuperseded),
    /// `conductor.improvement.MetricTargetSet`.
    MetricTargetSet(crate::improvement::MetricTargetSet),
    /// `conductor.improvement.ProposalApplied`.
    ProposalApplied(crate::improvement::ProposalApplied),
    /// `conductor.improvement.ProposalKept`.
    ProposalKept(crate::improvement::ProposalKept),
    /// `conductor.improvement.ProposalRefused`.
    ProposalRefused(crate::improvement::ProposalRefused),
    /// `conductor.improvement.ProposalReverted`.
    ProposalReverted(crate::improvement::ProposalReverted),
    /// `conductor.improvement.SettingProposed`.
    SettingProposed(crate::improvement::SettingProposed),
    /// `conductor.instance.InstanceAnswered`.
    InstanceAnswered(crate::instance::InstanceAnswered),
    /// `conductor.instance.InstancePurged`.
    InstancePurged(crate::instance::InstancePurged),
    /// `conductor.instance.InstanceReconciled`.
    InstanceReconciled(crate::instance::InstanceReconciled),
    /// `conductor.instance.InstanceRestarted`.
    InstanceRestarted(crate::instance::InstanceRestarted),
    /// `conductor.instance.InstanceRolledBack`.
    InstanceRolledBack(crate::instance::InstanceRolledBack),
    /// `conductor.instance.InstanceStarted`.
    InstanceStarted(crate::instance::InstanceStarted),
    /// `conductor.instance.InstanceStopped`.
    InstanceStopped(crate::instance::InstanceStopped),
    /// `conductor.instance.InstanceUnreachable`.
    InstanceUnreachable(crate::instance::InstanceUnreachable),
    /// `conductor.instance.InstanceUpgrading`.
    InstanceUpgrading(crate::instance::InstanceUpgrading),
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
    /// `conductor.operator.OperatorJoined`.
    OperatorJoined(crate::operator::OperatorJoined),
    /// `conductor.operator.OperatorLeft`.
    OperatorLeft(crate::operator::OperatorLeft),
    /// `conductor.placement.BoxClosed`.
    BoxClosed(crate::placement::BoxClosed),
    /// `conductor.placement.BoxOpened`.
    BoxOpened(crate::placement::BoxOpened),
    /// `conductor.placement.BoxRefused`.
    BoxRefused(crate::placement::BoxRefused),
    /// `conductor.secret.SecretPut`.
    SecretPut(crate::secret::SecretPut),
    /// `conductor.secret.SecretsResolved`.
    SecretsResolved(crate::secret::SecretsResolved),
    /// `conductor.session.LeaseTaken`.
    LeaseTaken(crate::session::LeaseTaken),
    /// `conductor.session.LeaseTakenOver`.
    LeaseTakenOver(crate::session::LeaseTakenOver),
    /// `conductor.session.MessageDelivered`.
    MessageDelivered(crate::session::MessageDelivered),
    /// `conductor.session.RunTickRecorded`.
    RunTickRecorded(crate::session::RunTickRecorded),
    /// `conductor.store.StoreMigrated`.
    StoreMigrated(crate::store::StoreMigrated),
    /// `conductor.token.TokenContributed`.
    TokenContributed(crate::token::TokenContributed),
    /// `conductor.token.TokenLeased`.
    TokenLeased(crate::token::TokenLeased),
    /// `conductor.token.TokenLimited`.
    TokenLimited(crate::token::TokenLimited),
    /// `conductor.token.TokenReleased`.
    TokenReleased(crate::token::TokenReleased),
    /// `conductor.token.TokenWithdrawn`.
    TokenWithdrawn(crate::token::TokenWithdrawn),
    /// `conductor.tool.ToolCallRecorded`.
    ToolCallRecorded(crate::tool::ToolCallRecorded),
}

impl SystemEvent {
    /// The qualified name the specification declares this event under.
    pub fn name(&self) -> &'static str {
        match self {
            Self::AgentKindDeclared(_) => "conductor.agent.AgentKindDeclared",
            Self::AgentReported(_) => "conductor.agent.AgentReported",
            Self::AgentStarted(_) => "conductor.agent.AgentStarted",
            Self::AgentStopped(_) => "conductor.agent.AgentStopped",
            Self::ConfigChanged(_) => "conductor.config.ConfigChanged",
            Self::InstanceAdded(_) => "conductor.config.InstanceAdded",
            Self::InstanceRemoved(_) => "conductor.config.InstanceRemoved",
            Self::InstanceReplaced(_) => "conductor.config.InstanceReplaced",
            Self::DecisionRecorded(_) => "conductor.decision.DecisionRecorded",
            Self::DecisionRequested(_) => "conductor.decision.DecisionRequested",
            Self::DecisionReversedEvent(_) => "conductor.decision.DecisionReversedEvent",
            Self::RequestAmended(_) => "conductor.decision.RequestAmended",
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
            Self::DispatchLinked(_) => "conductor.dispatch.DispatchLinked",
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
            Self::DiskReclaimed(_) => "conductor.host.DiskReclaimed",
            Self::DiskRequestWithdrawn(_) => "conductor.host.DiskRequestWithdrawn",
            Self::DiskRequested(_) => "conductor.host.DiskRequested",
            Self::DiskReservationRefused(_) => "conductor.host.DiskReservationRefused",
            Self::DiskReserved(_) => "conductor.host.DiskReserved",
            Self::HostGrantReleased(_) => "conductor.host.HostGrantReleased",
            Self::HostGrantTaken(_) => "conductor.host.HostGrantTaken",
            Self::PeakRecorded(_) => "conductor.host.PeakRecorded",
            Self::ReclaimFloorNotReached(_) => "conductor.host.ReclaimFloorNotReached",
            Self::RunStarted(_) => "conductor.host.RunStarted",
            Self::FindingClosed(_) => "conductor.improvement.FindingClosed",
            Self::FindingFiled(_) => "conductor.improvement.FindingFiled",
            Self::FindingRaised(_) => "conductor.improvement.FindingRaised",
            Self::FindingVerified(_) => "conductor.improvement.FindingVerified",
            Self::MetricDefined(_) => "conductor.improvement.MetricDefined",
            Self::MetricReadingJudged(_) => "conductor.improvement.MetricReadingJudged",
            Self::MetricReadingRecorded(_) => "conductor.improvement.MetricReadingRecorded",
            Self::MetricReadingSuperseded(_) => "conductor.improvement.MetricReadingSuperseded",
            Self::MetricTargetSet(_) => "conductor.improvement.MetricTargetSet",
            Self::ProposalApplied(_) => "conductor.improvement.ProposalApplied",
            Self::ProposalKept(_) => "conductor.improvement.ProposalKept",
            Self::ProposalRefused(_) => "conductor.improvement.ProposalRefused",
            Self::ProposalReverted(_) => "conductor.improvement.ProposalReverted",
            Self::SettingProposed(_) => "conductor.improvement.SettingProposed",
            Self::InstanceAnswered(_) => "conductor.instance.InstanceAnswered",
            Self::InstancePurged(_) => "conductor.instance.InstancePurged",
            Self::InstanceReconciled(_) => "conductor.instance.InstanceReconciled",
            Self::InstanceRestarted(_) => "conductor.instance.InstanceRestarted",
            Self::InstanceRolledBack(_) => "conductor.instance.InstanceRolledBack",
            Self::InstanceStarted(_) => "conductor.instance.InstanceStarted",
            Self::InstanceStopped(_) => "conductor.instance.InstanceStopped",
            Self::InstanceUnreachable(_) => "conductor.instance.InstanceUnreachable",
            Self::InstanceUpgrading(_) => "conductor.instance.InstanceUpgrading",
            Self::BlockerRecorded(_) => "conductor.observation.BlockerRecorded",
            Self::BoardPublished(_) => "conductor.observation.BoardPublished",
            Self::CollectorRunRecorded(_) => "conductor.observation.CollectorRunRecorded",
            Self::MemberFailureRecorded(_) => "conductor.observation.MemberFailureRecorded",
            Self::MergedPullRequestRecorded(_) => "conductor.observation.MergedPullRequestRecorded",
            Self::PullRequestRecorded(_) => "conductor.observation.PullRequestRecorded",
            Self::ReleaseRecorded(_) => "conductor.observation.ReleaseRecorded",
            Self::RepositoryRecorded(_) => "conductor.observation.RepositoryRecorded",
            Self::SecretSyncRecorded(_) => "conductor.observation.SecretSyncRecorded",
            Self::SessionRecorded(_) => "conductor.observation.SessionRecorded",
            Self::SnapshotCompleted(_) => "conductor.observation.SnapshotCompleted",
            Self::SnapshotFailed(_) => "conductor.observation.SnapshotFailed",
            Self::SnapshotStarted(_) => "conductor.observation.SnapshotStarted",
            Self::SpecificationRecorded(_) => "conductor.observation.SpecificationRecorded",
            Self::UsageRecorded(_) => "conductor.observation.UsageRecorded",
            Self::WorkflowRunRecorded(_) => "conductor.observation.WorkflowRunRecorded",
            Self::OperatorJoined(_) => "conductor.operator.OperatorJoined",
            Self::OperatorLeft(_) => "conductor.operator.OperatorLeft",
            Self::BoxClosed(_) => "conductor.placement.BoxClosed",
            Self::BoxOpened(_) => "conductor.placement.BoxOpened",
            Self::BoxRefused(_) => "conductor.placement.BoxRefused",
            Self::SecretPut(_) => "conductor.secret.SecretPut",
            Self::SecretsResolved(_) => "conductor.secret.SecretsResolved",
            Self::LeaseTaken(_) => "conductor.session.LeaseTaken",
            Self::LeaseTakenOver(_) => "conductor.session.LeaseTakenOver",
            Self::MessageDelivered(_) => "conductor.session.MessageDelivered",
            Self::RunTickRecorded(_) => "conductor.session.RunTickRecorded",
            Self::StoreMigrated(_) => "conductor.store.StoreMigrated",
            Self::TokenContributed(_) => "conductor.token.TokenContributed",
            Self::TokenLeased(_) => "conductor.token.TokenLeased",
            Self::TokenLimited(_) => "conductor.token.TokenLimited",
            Self::TokenReleased(_) => "conductor.token.TokenReleased",
            Self::TokenWithdrawn(_) => "conductor.token.TokenWithdrawn",
            Self::ToolCallRecorded(_) => "conductor.tool.ToolCallRecorded",
        }
    }
}

impl From<crate::ports::conductor_broker::PublishedEvent> for SystemEvent {
    fn from(event: crate::ports::conductor_broker::PublishedEvent) -> Self {
        match event {
            crate::ports::conductor_broker::PublishedEvent::ToolCallRecorded(event) => Self::ToolCallRecorded(event),
        }
    }
}

impl From<crate::ports::conductor_cli::PublishedEvent> for SystemEvent {
    fn from(event: crate::ports::conductor_cli::PublishedEvent) -> Self {
        match event {
            crate::ports::conductor_cli::PublishedEvent::DecisionRecorded(event) => Self::DecisionRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::DecisionRequested(event) => Self::DecisionRequested(event),
            crate::ports::conductor_cli::PublishedEvent::DecisionReversedEvent(event) => Self::DecisionReversedEvent(event),
            crate::ports::conductor_cli::PublishedEvent::RequestAmended(event) => Self::RequestAmended(event),
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
            crate::ports::conductor_cli::PublishedEvent::DispatchLinked(event) => Self::DispatchLinked(event),
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
            crate::ports::conductor_cli::PublishedEvent::DiskReclaimed(event) => Self::DiskReclaimed(event),
            crate::ports::conductor_cli::PublishedEvent::DiskRequestWithdrawn(event) => Self::DiskRequestWithdrawn(event),
            crate::ports::conductor_cli::PublishedEvent::DiskRequested(event) => Self::DiskRequested(event),
            crate::ports::conductor_cli::PublishedEvent::DiskReservationRefused(event) => Self::DiskReservationRefused(event),
            crate::ports::conductor_cli::PublishedEvent::DiskReserved(event) => Self::DiskReserved(event),
            crate::ports::conductor_cli::PublishedEvent::HostGrantReleased(event) => Self::HostGrantReleased(event),
            crate::ports::conductor_cli::PublishedEvent::HostGrantTaken(event) => Self::HostGrantTaken(event),
            crate::ports::conductor_cli::PublishedEvent::PeakRecorded(event) => Self::PeakRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::ReclaimFloorNotReached(event) => Self::ReclaimFloorNotReached(event),
            crate::ports::conductor_cli::PublishedEvent::RunStarted(event) => Self::RunStarted(event),
            crate::ports::conductor_cli::PublishedEvent::FindingClosed(event) => Self::FindingClosed(event),
            crate::ports::conductor_cli::PublishedEvent::FindingFiled(event) => Self::FindingFiled(event),
            crate::ports::conductor_cli::PublishedEvent::FindingRaised(event) => Self::FindingRaised(event),
            crate::ports::conductor_cli::PublishedEvent::FindingVerified(event) => Self::FindingVerified(event),
            crate::ports::conductor_cli::PublishedEvent::MetricDefined(event) => Self::MetricDefined(event),
            crate::ports::conductor_cli::PublishedEvent::MetricReadingJudged(event) => Self::MetricReadingJudged(event),
            crate::ports::conductor_cli::PublishedEvent::MetricReadingRecorded(event) => Self::MetricReadingRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::MetricReadingSuperseded(event) => Self::MetricReadingSuperseded(event),
            crate::ports::conductor_cli::PublishedEvent::MetricTargetSet(event) => Self::MetricTargetSet(event),
            crate::ports::conductor_cli::PublishedEvent::ProposalApplied(event) => Self::ProposalApplied(event),
            crate::ports::conductor_cli::PublishedEvent::ProposalKept(event) => Self::ProposalKept(event),
            crate::ports::conductor_cli::PublishedEvent::ProposalRefused(event) => Self::ProposalRefused(event),
            crate::ports::conductor_cli::PublishedEvent::ProposalReverted(event) => Self::ProposalReverted(event),
            crate::ports::conductor_cli::PublishedEvent::SettingProposed(event) => Self::SettingProposed(event),
            crate::ports::conductor_cli::PublishedEvent::BlockerRecorded(event) => Self::BlockerRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::BoardPublished(event) => Self::BoardPublished(event),
            crate::ports::conductor_cli::PublishedEvent::CollectorRunRecorded(event) => Self::CollectorRunRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::MemberFailureRecorded(event) => Self::MemberFailureRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::MergedPullRequestRecorded(event) => Self::MergedPullRequestRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::PullRequestRecorded(event) => Self::PullRequestRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::ReleaseRecorded(event) => Self::ReleaseRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::RepositoryRecorded(event) => Self::RepositoryRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::SecretSyncRecorded(event) => Self::SecretSyncRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::SessionRecorded(event) => Self::SessionRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::SnapshotCompleted(event) => Self::SnapshotCompleted(event),
            crate::ports::conductor_cli::PublishedEvent::SnapshotFailed(event) => Self::SnapshotFailed(event),
            crate::ports::conductor_cli::PublishedEvent::SnapshotStarted(event) => Self::SnapshotStarted(event),
            crate::ports::conductor_cli::PublishedEvent::SpecificationRecorded(event) => Self::SpecificationRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::UsageRecorded(event) => Self::UsageRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::WorkflowRunRecorded(event) => Self::WorkflowRunRecorded(event),
            crate::ports::conductor_cli::PublishedEvent::LeaseTaken(event) => Self::LeaseTaken(event),
            crate::ports::conductor_cli::PublishedEvent::LeaseTakenOver(event) => Self::LeaseTakenOver(event),
            crate::ports::conductor_cli::PublishedEvent::MessageDelivered(event) => Self::MessageDelivered(event),
            crate::ports::conductor_cli::PublishedEvent::RunTickRecorded(event) => Self::RunTickRecorded(event),
        }
    }
}

impl From<crate::ports::conductor_instance::PublishedEvent> for SystemEvent {
    fn from(event: crate::ports::conductor_instance::PublishedEvent) -> Self {
        match event {
            crate::ports::conductor_instance::PublishedEvent::AgentKindDeclared(event) => Self::AgentKindDeclared(event),
            crate::ports::conductor_instance::PublishedEvent::AgentReported(event) => Self::AgentReported(event),
            crate::ports::conductor_instance::PublishedEvent::AgentStarted(event) => Self::AgentStarted(event),
            crate::ports::conductor_instance::PublishedEvent::AgentStopped(event) => Self::AgentStopped(event),
        }
    }
}

impl From<crate::ports::conductord::PublishedEvent> for SystemEvent {
    fn from(event: crate::ports::conductord::PublishedEvent) -> Self {
        match event {
            crate::ports::conductord::PublishedEvent::ConfigChanged(event) => Self::ConfigChanged(event),
            crate::ports::conductord::PublishedEvent::InstanceAdded(event) => Self::InstanceAdded(event),
            crate::ports::conductord::PublishedEvent::InstanceRemoved(event) => Self::InstanceRemoved(event),
            crate::ports::conductord::PublishedEvent::InstanceReplaced(event) => Self::InstanceReplaced(event),
            crate::ports::conductord::PublishedEvent::InstanceAnswered(event) => Self::InstanceAnswered(event),
            crate::ports::conductord::PublishedEvent::InstancePurged(event) => Self::InstancePurged(event),
            crate::ports::conductord::PublishedEvent::InstanceReconciled(event) => Self::InstanceReconciled(event),
            crate::ports::conductord::PublishedEvent::InstanceRestarted(event) => Self::InstanceRestarted(event),
            crate::ports::conductord::PublishedEvent::InstanceRolledBack(event) => Self::InstanceRolledBack(event),
            crate::ports::conductord::PublishedEvent::InstanceStarted(event) => Self::InstanceStarted(event),
            crate::ports::conductord::PublishedEvent::InstanceStopped(event) => Self::InstanceStopped(event),
            crate::ports::conductord::PublishedEvent::InstanceUnreachable(event) => Self::InstanceUnreachable(event),
            crate::ports::conductord::PublishedEvent::InstanceUpgrading(event) => Self::InstanceUpgrading(event),
            crate::ports::conductord::PublishedEvent::OperatorJoined(event) => Self::OperatorJoined(event),
            crate::ports::conductord::PublishedEvent::OperatorLeft(event) => Self::OperatorLeft(event),
            crate::ports::conductord::PublishedEvent::BoxClosed(event) => Self::BoxClosed(event),
            crate::ports::conductord::PublishedEvent::BoxOpened(event) => Self::BoxOpened(event),
            crate::ports::conductord::PublishedEvent::BoxRefused(event) => Self::BoxRefused(event),
            crate::ports::conductord::PublishedEvent::SecretPut(event) => Self::SecretPut(event),
            crate::ports::conductord::PublishedEvent::SecretsResolved(event) => Self::SecretsResolved(event),
            crate::ports::conductord::PublishedEvent::StoreMigrated(event) => Self::StoreMigrated(event),
            crate::ports::conductord::PublishedEvent::TokenContributed(event) => Self::TokenContributed(event),
            crate::ports::conductord::PublishedEvent::TokenLeased(event) => Self::TokenLeased(event),
            crate::ports::conductord::PublishedEvent::TokenLimited(event) => Self::TokenLimited(event),
            crate::ports::conductord::PublishedEvent::TokenReleased(event) => Self::TokenReleased(event),
            crate::ports::conductord::PublishedEvent::TokenWithdrawn(event) => Self::TokenWithdrawn(event),
        }
    }
}

/// The `conductor` system: every component behind its port, and the transport between them.
///
/// The component fields are public because commands enter the system through a component's own
/// port; the log and its delivery cursor are not, because publishing happens by pumping, not by
/// writing history directly.
pub struct System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors> {
    /// The `conductor-broker` component.
    pub conductor_broker: crate::ports::conductor_broker::ConductorBroker<ConductorBrokerBehaviors>,
    /// The `conductor-cli` component.
    pub conductor_cli: crate::ports::conductor_cli::ConductorCli<ConductorCliBehaviors>,
    /// The `conductor-instance` component.
    pub conductor_instance: crate::ports::conductor_instance::ConductorInstance<ConductorInstanceBehaviors>,
    /// The `conductord` component.
    pub conductord: crate::ports::conductord::Conductord<ConductordBehaviors>,
    published: Vec<SystemEvent>,
    cursor: usize,
}

impl<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors> System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors> {
    /// Assembles the system from its components.
    pub fn new(conductor_broker: crate::ports::conductor_broker::ConductorBroker<ConductorBrokerBehaviors>, conductor_cli: crate::ports::conductor_cli::ConductorCli<ConductorCliBehaviors>, conductor_instance: crate::ports::conductor_instance::ConductorInstance<ConductorInstanceBehaviors>, conductord: crate::ports::conductord::Conductord<ConductordBehaviors>) -> Self {
        Self {
            conductor_broker,
            conductor_cli,
            conductor_instance,
            conductord,
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

impl<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors> System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordDispatchLinkBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::AddInstanceBehavior + crate::config::obligations::PutConfigBehavior + crate::config::obligations::PutInstanceBehavior + crate::config::obligations::RemoveInstanceBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstanceByNameQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::InstanceSecretsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::ReadinessQuery + crate::store::obligations::StoreStatusQuery + crate::store::obligations::VersionQuery + crate::token::obligations::TokensQuery,
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
        for event in self.conductor_broker.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.conductor_cli.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.conductor_instance.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
        for event in self.conductord.drain_outbox() {
            self.published.push(SystemEvent::from(event));
        }
    }
}
