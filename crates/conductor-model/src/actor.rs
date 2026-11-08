// generated from conductor v1
// contract digest 92a4370ed302f5aecf042d1cd6448ff2d89bf65c94265e16af94376031e7bd63
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
    /// `conductor.agent.Controller`.
    AgentController,
    /// `conductor.agent.InstanceRuntime`.
    AgentInstanceRuntime,
    /// `conductor.agent.Operator`.
    AgentOperator,
    /// `conductor.config.Operator`.
    ConfigOperator,
    /// `conductor.config.Tuner`.
    ConfigTuner,
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
    /// `conductor.host.BuildCaller`.
    HostBuildCaller,
    /// `conductor.host.InstanceRuntime`.
    HostInstanceRuntime,
    /// `conductor.host.Operator`.
    HostOperator,
    /// `conductor.improvement.Collector`.
    ImprovementCollector,
    /// `conductor.improvement.ConductorDev`.
    ImprovementConductorDev,
    /// `conductor.improvement.Operator`.
    ImprovementOperator,
    /// `conductor.improvement.Tuner`.
    ImprovementTuner,
    /// `conductor.instance.Conductord`.
    InstanceConductord,
    /// `conductor.instance.Operator`.
    InstanceOperator,
    /// `conductor.observation.Collector`.
    ObservationCollector,
    /// `conductor.operator.Conductord`.
    OperatorConductord,
    /// `conductor.operator.Engineer`.
    OperatorEngineer,
    /// `conductor.placement.Conductord`.
    PlacementConductord,
    /// `conductor.secret.Conductord`.
    SecretConductord,
    /// `conductor.secret.Operator`.
    SecretOperator,
    /// `conductor.session.Conductor`.
    SessionConductor,
    /// `conductor.store.Operator`.
    StoreOperator,
    /// `conductor.token.InstanceRuntime`.
    TokenInstanceRuntime,
    /// `conductor.token.Operator`.
    TokenOperator,
    /// `conductor.tool.InstanceBroker`.
    ToolInstanceBroker,
}

impl Actor {
    /// Every declared actor, ordered by qualified name.
    pub const ALL: &'static [Actor] = &[
        Actor::AgentController,
        Actor::AgentInstanceRuntime,
        Actor::AgentOperator,
        Actor::ConfigOperator,
        Actor::ConfigTuner,
        Actor::DecisionConductor,
        Actor::DecisionOperator,
        Actor::DecisionRepositoryController,
        Actor::DirectionConductor,
        Actor::DirectionOperator,
        Actor::DispatchConductor,
        Actor::DispatchRepositoryController,
        Actor::HostBuildCaller,
        Actor::HostInstanceRuntime,
        Actor::HostOperator,
        Actor::ImprovementCollector,
        Actor::ImprovementConductorDev,
        Actor::ImprovementOperator,
        Actor::ImprovementTuner,
        Actor::InstanceConductord,
        Actor::InstanceOperator,
        Actor::ObservationCollector,
        Actor::OperatorConductord,
        Actor::OperatorEngineer,
        Actor::PlacementConductord,
        Actor::SecretConductord,
        Actor::SecretOperator,
        Actor::SessionConductor,
        Actor::StoreOperator,
        Actor::TokenInstanceRuntime,
        Actor::TokenOperator,
        Actor::ToolInstanceBroker,
    ];

    /// The actor's qualified name, as the specification spells it.
    pub const fn name(self) -> &'static str {
        match self {
            Actor::AgentController => "conductor.agent.Controller",
            Actor::AgentInstanceRuntime => "conductor.agent.InstanceRuntime",
            Actor::AgentOperator => "conductor.agent.Operator",
            Actor::ConfigOperator => "conductor.config.Operator",
            Actor::ConfigTuner => "conductor.config.Tuner",
            Actor::DecisionConductor => "conductor.decision.Conductor",
            Actor::DecisionOperator => "conductor.decision.Operator",
            Actor::DecisionRepositoryController => "conductor.decision.RepositoryController",
            Actor::DirectionConductor => "conductor.direction.Conductor",
            Actor::DirectionOperator => "conductor.direction.Operator",
            Actor::DispatchConductor => "conductor.dispatch.Conductor",
            Actor::DispatchRepositoryController => "conductor.dispatch.RepositoryController",
            Actor::HostBuildCaller => "conductor.host.BuildCaller",
            Actor::HostInstanceRuntime => "conductor.host.InstanceRuntime",
            Actor::HostOperator => "conductor.host.Operator",
            Actor::ImprovementCollector => "conductor.improvement.Collector",
            Actor::ImprovementConductorDev => "conductor.improvement.ConductorDev",
            Actor::ImprovementOperator => "conductor.improvement.Operator",
            Actor::ImprovementTuner => "conductor.improvement.Tuner",
            Actor::InstanceConductord => "conductor.instance.Conductord",
            Actor::InstanceOperator => "conductor.instance.Operator",
            Actor::ObservationCollector => "conductor.observation.Collector",
            Actor::OperatorConductord => "conductor.operator.Conductord",
            Actor::OperatorEngineer => "conductor.operator.Engineer",
            Actor::PlacementConductord => "conductor.placement.Conductord",
            Actor::SecretConductord => "conductor.secret.Conductord",
            Actor::SecretOperator => "conductor.secret.Operator",
            Actor::SessionConductor => "conductor.session.Conductor",
            Actor::StoreOperator => "conductor.store.Operator",
            Actor::TokenInstanceRuntime => "conductor.token.InstanceRuntime",
            Actor::TokenOperator => "conductor.token.Operator",
            Actor::ToolInstanceBroker => "conductor.tool.InstanceBroker",
        }
    }
}

/// The qualified names of the commands `actor` may invoke, ordered by name; empty for an
/// actor that only observes.
pub fn may(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::AgentController => &[
            "conductor.agent.AttachAgent",
            "conductor.agent.DeliverToAgent",
        ],
        Actor::AgentInstanceRuntime => &[
            "conductor.agent.DeclareAgentKind",
            "conductor.agent.DeliverToAgent",
            "conductor.agent.MarkAgentRunning",
            "conductor.agent.RecordAgentReport",
            "conductor.agent.RestartAgent",
            "conductor.agent.StartAgent",
            "conductor.agent.StopAgent",
        ],
        Actor::AgentOperator => &[
            "conductor.agent.AttachAgent",
            "conductor.agent.DeliverToAgent",
            "conductor.agent.RestartAgent",
            "conductor.agent.StopAgent",
        ],
        Actor::ConfigOperator => &[
            "conductor.config.PutConfig",
        ],
        Actor::ConfigTuner => &[
            "conductor.config.PutConfig",
        ],
        Actor::DecisionConductor => &[
            "conductor.decision.AmendEscalatedRequest",
            "conductor.decision.AnswerRequest",
            "conductor.decision.EscalateRequest",
            "conductor.decision.RecordConductorDecision",
            "conductor.decision.ReverseConductorDecision",
        ],
        Actor::DecisionOperator => &[
            "conductor.decision.AnswerEscalatedRequest",
            "conductor.decision.RecordOperatorDecision",
            "conductor.decision.RecordOperatorDecisionForRequest",
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
            "conductor.dispatch.ReceiveMessage",
            "conductor.dispatch.RecordGuardDecision",
            "conductor.dispatch.ReleaseResource",
            "conductor.dispatch.ReportBlocked",
            "conductor.dispatch.ReportDone",
            "conductor.dispatch.ReportFailed",
            "conductor.dispatch.ReportStarted",
            "conductor.dispatch.ReportUnblocked",
            "conductor.dispatch.RequestResource",
        ],
        Actor::HostBuildCaller => &[
            "conductor.host.Reclaim",
            "conductor.host.RecordPeak",
            "conductor.host.ReleaseGrant",
            "conductor.host.RequestDisk",
            "conductor.host.ReserveDisk",
            "conductor.host.StartRun",
            "conductor.host.WithdrawDiskRequest",
        ],
        Actor::HostInstanceRuntime => &[
            "conductor.host.Reclaim",
            "conductor.host.ReleaseGrant",
            "conductor.host.TakeHostGrant",
        ],
        Actor::HostOperator => &[
            "conductor.host.Reclaim",
            "conductor.host.RecordPeak",
            "conductor.host.ReleaseGrant",
            "conductor.host.RequestDisk",
            "conductor.host.ReserveDisk",
            "conductor.host.StartRun",
            "conductor.host.WithdrawDiskRequest",
        ],
        Actor::ImprovementCollector => &[
            "conductor.improvement.JudgeMetricReading",
            "conductor.improvement.RaiseFinding",
            "conductor.improvement.RecordMetricReading",
            "conductor.improvement.SupersedeMetricReading",
        ],
        Actor::ImprovementConductorDev => &[
            "conductor.improvement.DefineMetric",
            "conductor.improvement.FileFinding",
            "conductor.improvement.RaiseFinding",
            "conductor.improvement.VerifyFinding",
        ],
        Actor::ImprovementOperator => &[
            "conductor.improvement.SetMetricTarget",
        ],
        Actor::ImprovementTuner => &[
            "conductor.improvement.ApplyProposal",
            "conductor.improvement.ProposeSetting",
            "conductor.improvement.RaiseFinding",
            "conductor.improvement.RefuseProposal",
            "conductor.improvement.SettleProposal",
        ],
        Actor::InstanceConductord => &[
            "conductor.instance.ObserveInstance",
            "conductor.instance.Reconcile",
            "conductor.instance.RestartInstance",
            "conductor.instance.StartInstance",
            "conductor.instance.StopInstance",
            "conductor.instance.UpgradeInstance",
        ],
        Actor::InstanceOperator => &[
            "conductor.instance.PurgeInstance",
            "conductor.instance.RestartInstance",
            "conductor.instance.StartInstance",
            "conductor.instance.StopInstance",
            "conductor.instance.UpgradeInstance",
        ],
        Actor::ObservationCollector => &[
            "conductor.observation.CompleteSnapshot",
            "conductor.observation.FailSnapshot",
            "conductor.observation.PublishBoard",
            "conductor.observation.RecordBlocker",
            "conductor.observation.RecordCollectorRun",
            "conductor.observation.RecordDailySeries",
            "conductor.observation.RecordMemberFailure",
            "conductor.observation.RecordMergedPullRequest",
            "conductor.observation.RecordPullRequest",
            "conductor.observation.RecordRelease",
            "conductor.observation.RecordRepository",
            "conductor.observation.RecordSession",
            "conductor.observation.RecordSpecification",
            "conductor.observation.RecordWorkflowRun",
            "conductor.observation.ReviseDailySeries",
            "conductor.observation.StartSnapshot",
        ],
        Actor::OperatorConductord => &[],
        Actor::OperatorEngineer => &[
            "conductor.operator.JoinInstance",
            "conductor.operator.LeaveInstance",
        ],
        Actor::PlacementConductord => &[
            "conductor.placement.CloseBox",
            "conductor.placement.OpenBox",
        ],
        Actor::SecretConductord => &[
            "conductor.secret.ResolveSecrets",
        ],
        Actor::SecretOperator => &[
            "conductor.secret.PutSecret",
        ],
        Actor::SessionConductor => &[
            "conductor.session.ClaimLease",
            "conductor.session.DeliverMessage",
            "conductor.session.RecordRunTick",
        ],
        Actor::StoreOperator => &[
            "conductor.store.MigrateStore",
        ],
        Actor::TokenInstanceRuntime => &[
            "conductor.token.LeaseToken",
            "conductor.token.MarkTokenLimited",
            "conductor.token.ReleaseToken",
        ],
        Actor::TokenOperator => &[
            "conductor.token.ContributeToken",
            "conductor.token.WithdrawToken",
        ],
        Actor::ToolInstanceBroker => &[
            "conductor.dispatch.ReceiveMessage",
            "conductor.dispatch.ReportBlocked",
            "conductor.dispatch.ReportDone",
            "conductor.dispatch.ReportFailed",
            "conductor.dispatch.ReportStarted",
            "conductor.dispatch.ReportUnblocked",
            "conductor.tool.RecordToolCall",
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

/// The qualified names of the views `actor` may read, ordered by name: the views its grant
/// names. A view no actor's grant names is open to every caller and listed for none.
pub fn may_read(actor: Actor) -> &'static [&'static str] {
    match actor {
        Actor::AgentController => &[
            "conductor.agent.AgentKinds",
            "conductor.agent.Agents",
        ],
        Actor::AgentInstanceRuntime => &[
            "conductor.agent.AgentKinds",
            "conductor.agent.Agents",
        ],
        Actor::AgentOperator => &[
            "conductor.agent.AgentKinds",
            "conductor.agent.Agents",
        ],
        Actor::ConfigOperator => &[],
        Actor::ConfigTuner => &[],
        Actor::DecisionConductor => &[],
        Actor::DecisionOperator => &[],
        Actor::DecisionRepositoryController => &[],
        Actor::DirectionConductor => &[],
        Actor::DirectionOperator => &[],
        Actor::DispatchConductor => &[],
        Actor::DispatchRepositoryController => &[],
        Actor::HostBuildCaller => &[],
        Actor::HostInstanceRuntime => &[],
        Actor::HostOperator => &[],
        Actor::ImprovementCollector => &[],
        Actor::ImprovementConductorDev => &[],
        Actor::ImprovementOperator => &[],
        Actor::ImprovementTuner => &[],
        Actor::InstanceConductord => &[],
        Actor::InstanceOperator => &[],
        Actor::ObservationCollector => &[],
        Actor::OperatorConductord => &[
            "conductor.operator.Operators",
        ],
        Actor::OperatorEngineer => &[
            "conductor.operator.Operators",
        ],
        Actor::PlacementConductord => &[],
        Actor::SecretConductord => &[],
        Actor::SecretOperator => &[],
        Actor::SessionConductor => &[],
        Actor::StoreOperator => &[],
        Actor::TokenInstanceRuntime => &[],
        Actor::TokenOperator => &[],
        Actor::ToolInstanceBroker => &[],
    }
}

impl Caller {
    /// `true` when this caller may read `view`, a view some actor's grant names, named by
    /// its qualified name.
    pub fn may_read(&self, view: &str) -> bool {
        may_read(self.actor).contains(&view)
    }
}
