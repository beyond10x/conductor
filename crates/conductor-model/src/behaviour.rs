// generated from conductor v1
// contract digest 58983098659d5fd5fbd918be2db0c0ce1c382725137cfe87bf81bc48ad81f500
// do not edit: regenerate with `ess synthesize --layout crate`

//! What the specification fully determines, generated: the behaviour of every command the plan
//! lists as generated, written against ports the implementor supplies.
//!
//! Storage is a port: one trait per entity, get, put and delete of a snapshot by identity. ess
//! preserves the trait; generated network entries supply an ephemeral store. `Context` carries the caller's attributes,
//! every identity and value the model says the implementation assigns, and the answer to each
//! `external:` branch. [`Generated`] implements every generated `…Behavior` trait over those ports
//! and forwards every behaviour and query the plan still owes to them, so it is a complete bundle
//! for every component port. To replace one generated behaviour, write a bundle of your own that
//! implements that trait and delegates the rest to a `Generated`.
//!
//! An `Err` from a generated behaviour is the typed refusal naming the command: the model declares
//! no outcome for the request (a guard is undecidable over it, or no declared branch answers it),
//! or — as `entity invariant` — the declared outcome would leave an entity breaking an invariant.

use crate::obligation::UnmetObligation;

/// Where `conductor.agent.Agent` is stored — a port the implementor provides.
///
/// Keyed by the identity `name`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait AgentStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::agent::AgentName) -> Option<crate::agent::AgentSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::agent::AgentSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::agent::AgentName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::agent::AgentSnapshot>;
}

/// Where `conductor.agent.AgentKind` is stored — a port the implementor provides.
///
/// Keyed by the identity `name`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait AgentKindStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::agent::AgentKindName) -> Option<crate::agent::AgentKindSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::agent::AgentKindSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::agent::AgentKindName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::agent::AgentKindSnapshot>;
}

/// Where `conductor.config.ConfigRevision` is stored — a port the implementor provides.
///
/// Keyed by the identity `revision_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ConfigRevisionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::config::RevisionId) -> Option<crate::config::ConfigRevisionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::config::ConfigRevisionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::config::RevisionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::config::ConfigRevisionSnapshot>;
}

/// Where `conductor.decision.Decision` is stored — a port the implementor provides.
///
/// Keyed by the identity `decision_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait DecisionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::decision::DecisionId) -> Option<crate::decision::DecisionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::decision::DecisionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::decision::DecisionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::decision::DecisionSnapshot>;
}

/// Where `conductor.decision.DecisionRequest` is stored — a port the implementor provides.
///
/// Keyed by the identity `request_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait DecisionRequestStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::decision::RequestId) -> Option<crate::decision::DecisionRequestSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::decision::DecisionRequestSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::decision::RequestId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::decision::DecisionRequestSnapshot>;
}

/// Where `conductor.direction.Goal` is stored — a port the implementor provides.
///
/// Keyed by the identity `goal_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait GoalStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::direction::GoalId) -> Option<crate::direction::GoalSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::direction::GoalSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::direction::GoalId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::direction::GoalSnapshot>;
}

/// Where `conductor.direction.GoalServing` is stored — a port the implementor provides.
///
/// Keyed by the identity `serving_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait GoalServingStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::direction::ServingId) -> Option<crate::direction::GoalServingSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::direction::GoalServingSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::direction::ServingId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::direction::GoalServingSnapshot>;
}

/// Where `conductor.direction.RepositoryMark` is stored — a port the implementor provides.
///
/// Keyed by the identity `repository`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait RepositoryMarkStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::direction::MarkedRepository) -> Option<crate::direction::RepositoryMarkSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::direction::RepositoryMarkSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::direction::MarkedRepository);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::direction::RepositoryMarkSnapshot>;
}

/// Where `conductor.dispatch.Controller` is stored — a port the implementor provides.
///
/// Keyed by the identity `controller_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ControllerStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::dispatch::ControllerId) -> Option<crate::dispatch::ControllerSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::dispatch::ControllerSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::dispatch::ControllerId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::dispatch::ControllerSnapshot>;
}

/// Where `conductor.dispatch.Dispatch` is stored — a port the implementor provides.
///
/// Keyed by the identity `dispatch_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait DispatchStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::dispatch::DispatchId) -> Option<crate::dispatch::DispatchSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::dispatch::DispatchSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::dispatch::DispatchId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::dispatch::DispatchSnapshot>;
}

/// Where `conductor.dispatch.GuardDecision` is stored — a port the implementor provides.
///
/// Keyed by the identity `guard_decision_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait GuardDecisionStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::dispatch::GuardDecisionId) -> Option<crate::dispatch::GuardDecisionSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::dispatch::GuardDecisionSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::dispatch::GuardDecisionId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::dispatch::GuardDecisionSnapshot>;
}

/// Where `conductor.dispatch.Message` is stored — a port the implementor provides.
///
/// Keyed by the identity `message_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait MessageStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::dispatch::MessageId) -> Option<crate::dispatch::MessageSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::dispatch::MessageSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::dispatch::MessageId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::dispatch::MessageSnapshot>;
}

/// Where `conductor.dispatch.ResourceRequest` is stored — a port the implementor provides.
///
/// Keyed by the identity `resource_request_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ResourceRequestStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::dispatch::ResourceRequestId) -> Option<crate::dispatch::ResourceRequestSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::dispatch::ResourceRequestSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::dispatch::ResourceRequestId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::dispatch::ResourceRequestSnapshot>;
}

/// Where `conductor.host.HostGrant` is stored — a port the implementor provides.
///
/// Keyed by the identity `grant_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait HostGrantStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::host::GrantId) -> Option<crate::host::HostGrantSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::host::HostGrantSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::host::GrantId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::host::HostGrantSnapshot>;
}

/// Where `conductor.host.ReclaimRun` is stored — a port the implementor provides.
///
/// Keyed by the identity `reclaim_run_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ReclaimRunStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::host::ReclaimRunId) -> Option<crate::host::ReclaimRunSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::host::ReclaimRunSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::host::ReclaimRunId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::host::ReclaimRunSnapshot>;
}

/// Where `conductor.improvement.FrictionFinding` is stored — a port the implementor provides.
///
/// Keyed by the identity `finding_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait FrictionFindingStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::improvement::FindingId) -> Option<crate::improvement::FrictionFindingSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::improvement::FrictionFindingSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::improvement::FindingId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::improvement::FrictionFindingSnapshot>;
}

/// Where `conductor.improvement.Metric` is stored — a port the implementor provides.
///
/// Keyed by the identity `metric_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait MetricStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::improvement::MetricId) -> Option<crate::improvement::MetricSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::improvement::MetricSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::improvement::MetricId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::improvement::MetricSnapshot>;
}

/// Where `conductor.improvement.MetricReading` is stored — a port the implementor provides.
///
/// Keyed by the identity `reading_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait MetricReadingStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::improvement::MetricReadingId) -> Option<crate::improvement::MetricReadingSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::improvement::MetricReadingSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::improvement::MetricReadingId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::improvement::MetricReadingSnapshot>;
}

/// Where `conductor.improvement.SettingsProposal` is stored — a port the implementor provides.
///
/// Keyed by the identity `proposal_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SettingsProposalStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::improvement::ProposalId) -> Option<crate::improvement::SettingsProposalSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::improvement::SettingsProposalSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::improvement::ProposalId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::improvement::SettingsProposalSnapshot>;
}

/// Where `conductor.instance.Instance` is stored — a port the implementor provides.
///
/// Keyed by the identity `name`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait InstanceStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::instance::InstanceSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::instance::InstanceSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::config::InstanceName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::instance::InstanceSnapshot>;
}

/// Where `conductor.instance.Reconciliation` is stored — a port the implementor provides.
///
/// Keyed by the identity `reconciliation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ReconciliationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::instance::ReconciliationId) -> Option<crate::instance::ReconciliationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::instance::ReconciliationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::instance::ReconciliationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::instance::ReconciliationSnapshot>;
}

/// Where `conductor.observation.BlockerObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait BlockerObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::BlockerObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::BlockerObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::BlockerObservationSnapshot>;
}

/// Where `conductor.observation.Board` is stored — a port the implementor provides.
///
/// Keyed by the identity `board_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait BoardStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::BoardId) -> Option<crate::observation::BoardSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::BoardSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::BoardId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::BoardSnapshot>;
}

/// Where `conductor.observation.CollectorRunObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait CollectorRunObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::CollectorRunObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::CollectorRunObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::CollectorRunObservationSnapshot>;
}

/// Where `conductor.observation.DailySeries` is stored — a port the implementor provides.
///
/// Keyed by the identity `daily_series_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait DailySeriesStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::DailySeriesId) -> Option<crate::observation::DailySeriesSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::DailySeriesSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::DailySeriesId);
}

/// Where `conductor.observation.MemberFailureObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait MemberFailureObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::MemberFailureObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::MemberFailureObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::MemberFailureObservationSnapshot>;
}

/// Where `conductor.observation.MergedPullRequestObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait MergedPullRequestObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::MergedPullRequestObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::MergedPullRequestObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::MergedPullRequestObservationSnapshot>;
}

/// Where `conductor.observation.PullRequestObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait PullRequestObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::PullRequestObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::PullRequestObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::PullRequestObservationSnapshot>;
}

/// Where `conductor.observation.ReleaseObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ReleaseObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::ReleaseObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::ReleaseObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::ReleaseObservationSnapshot>;
}

/// Where `conductor.observation.RepositoryObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait RepositoryObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::RepositoryObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::RepositoryObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::RepositoryObservationSnapshot>;
}

/// Where `conductor.observation.SecretSyncObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SecretSyncObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SecretSyncObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::SecretSyncObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::SecretSyncObservationSnapshot>;
}

/// Where `conductor.observation.SessionObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SessionObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SessionObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::SessionObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::SessionObservationSnapshot>;
}

/// Where `conductor.observation.Snapshot` is stored — a port the implementor provides.
///
/// Keyed by the identity `snapshot_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SnapshotStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::SnapshotId) -> Option<crate::observation::SnapshotSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::SnapshotSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::SnapshotId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::SnapshotSnapshot>;
}

/// Where `conductor.observation.SpecificationObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SpecificationObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::SpecificationObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::SpecificationObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::SpecificationObservationSnapshot>;
}

/// Where `conductor.observation.UsageObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait UsageObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::UsageObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::UsageObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::UsageObservationSnapshot>;
}

/// Where `conductor.observation.WorkflowRunObservation` is stored — a port the implementor provides.
///
/// Keyed by the identity `observation_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait WorkflowRunObservationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::observation::ObservationId) -> Option<crate::observation::WorkflowRunObservationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::observation::WorkflowRunObservationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::observation::ObservationId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::observation::WorkflowRunObservationSnapshot>;
}

/// Where `conductor.operator.Operator` is stored — a port the implementor provides.
///
/// Keyed by the identity `key`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait OperatorStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::operator::OperatorKey) -> Option<crate::operator::OperatorSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::operator::OperatorSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::operator::OperatorKey);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::operator::OperatorSnapshot>;
}

/// Where `conductor.placement.Box` is stored — a port the implementor provides.
///
/// Keyed by the identity `box_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait BoxStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::placement::BoxId) -> Option<crate::placement::BoxSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::placement::BoxSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::placement::BoxId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::placement::BoxSnapshot>;
}

/// Where `conductor.placement.BoxRefusal` is stored — a port the implementor provides.
///
/// Keyed by the identity `box_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait BoxRefusalStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::placement::BoxId) -> Option<crate::placement::BoxRefusalSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::placement::BoxRefusalSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::placement::BoxId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::placement::BoxRefusalSnapshot>;
}

/// Where `conductor.placement.Placement` is stored — a port the implementor provides.
///
/// Keyed by the identity `name`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait PlacementStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::placement::PlacementName) -> Option<crate::placement::PlacementSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::placement::PlacementSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::placement::PlacementName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::placement::PlacementSnapshot>;
}

/// Where `conductor.secret.Secret` is stored — a port the implementor provides.
///
/// Keyed by the identity `name`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait SecretStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::secret::SecretName) -> Option<crate::secret::SecretSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::secret::SecretSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::secret::SecretName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::secret::SecretSnapshot>;
}

/// Where `conductor.session.Delivery` is stored — a port the implementor provides.
///
/// Keyed by the identity `delivery_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait DeliveryStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::session::DeliveryId) -> Option<crate::session::DeliverySnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::session::DeliverySnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::session::DeliveryId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::session::DeliverySnapshot>;
}

/// Where `conductor.session.Lease` is stored — a port the implementor provides.
///
/// Keyed by the identity `instance`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait LeaseStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::session::LeaseSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::session::LeaseSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::config::InstanceName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::session::LeaseSnapshot>;
}

/// Where `conductor.session.RunTick` is stored — a port the implementor provides.
///
/// Keyed by the identity `tick_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait RunTickStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::session::TickId) -> Option<crate::session::RunTickSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::session::RunTickSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::session::TickId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::session::RunTickSnapshot>;
}

/// Where `conductor.store.Build` is stored — a port the implementor provides.
///
/// Keyed by the identity `version`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait BuildStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::store::BinaryVersion) -> Option<crate::store::BuildSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::store::BuildSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::store::BinaryVersion);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::store::BuildSnapshot>;
}

/// Where `conductor.store.Migration` is stored — a port the implementor provides.
///
/// Keyed by the identity `instance`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait MigrationStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::config::InstanceName) -> Option<crate::store::MigrationSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::store::MigrationSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::config::InstanceName);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::store::MigrationSnapshot>;
}

/// Where `conductor.store.Store` is stored — a port the implementor provides.
///
/// Keyed by the identity `url`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait StoreStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::store::StoreUrl) -> Option<crate::store::StoreSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::store::StoreSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::store::StoreUrl);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::store::StoreSnapshot>;
}

/// Where `conductor.token.PoolToken` is stored — a port the implementor provides.
///
/// Keyed by the identity `token_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait PoolTokenStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::token::TokenId) -> Option<crate::token::PoolTokenSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::token::PoolTokenSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::token::TokenId);
}

/// Where `conductor.tool.ToolCall` is stored — a port the implementor provides.
///
/// Keyed by the identity `tool_call_id`. Generated network entries supply an ephemeral implementation; durable storage remains a port.
pub trait ToolCallStorage {
    /// The instance with this identity, or `None` where none is stored.
    fn get(&self, identity: &crate::tool::ToolCallId) -> Option<crate::tool::ToolCallSnapshot>;

    /// Stores this instance under its identity, replacing what was held.
    fn put(&mut self, snapshot: crate::tool::ToolCallSnapshot);

    /// Removes the instance with this identity.
    fn delete(&mut self, identity: &crate::tool::ToolCallId);

    /// Every stored instance, in the order the store keeps them: the order a generated query
    /// answers an unordered view in.
    fn list(&self) -> Vec<crate::tool::ToolCallSnapshot>;
}

/// The exact executing command input supplied to an external decision.
///
/// This supplies facts, not authority: the context must verify its request-bound proof.
#[derive(Debug, Clone, Copy)]
pub enum ExternalCommand<'a> {
    /// The executing `conductor.agent.RestartAgent` input.
    ConductorAgentRestartAgent(&'a crate::agent::RestartAgent),
    /// The executing `conductor.config.AddInstance` input.
    ConductorConfigAddInstance(&'a crate::config::AddInstance),
    /// The executing `conductor.config.PutConfig` input.
    ConductorConfigPutConfig(&'a crate::config::PutConfig),
    /// The executing `conductor.config.PutInstance` input.
    ConductorConfigPutInstance(&'a crate::config::PutInstance),
    /// The executing `conductor.config.RemoveInstance` input.
    ConductorConfigRemoveInstance(&'a crate::config::RemoveInstance),
    /// The executing `conductor.dispatch.HandleMessage` input.
    ConductorDispatchHandleMessage(&'a crate::dispatch::HandleMessage),
    /// The executing `conductor.instance.RestartInstance` input.
    ConductorInstanceRestartInstance(&'a crate::instance::RestartInstance),
    /// The executing `conductor.placement.OpenBox` input.
    ConductorPlacementOpenBox(&'a crate::placement::OpenBox),
    /// The executing `conductor.session.DeliverMessage` input.
    ConductorSessionDeliverMessage(&'a crate::session::DeliverMessage),
    /// The executing `conductor.token.LeaseToken` input.
    ConductorTokenLeaseToken(&'a crate::token::LeaseToken),
    /// The executing `conductor.token.WithdrawToken` input.
    ConductorTokenWithdrawToken(&'a crate::token::WithdrawToken),
}

impl ExternalCommand<'_> {
    /// The canonical qualified identity of this command.
    pub fn name(&self) -> &'static str {
        match self {
            Self::ConductorAgentRestartAgent(_) => "conductor.agent.RestartAgent",
            Self::ConductorConfigAddInstance(_) => "conductor.config.AddInstance",
            Self::ConductorConfigPutConfig(_) => "conductor.config.PutConfig",
            Self::ConductorConfigPutInstance(_) => "conductor.config.PutInstance",
            Self::ConductorConfigRemoveInstance(_) => "conductor.config.RemoveInstance",
            Self::ConductorDispatchHandleMessage(_) => "conductor.dispatch.HandleMessage",
            Self::ConductorInstanceRestartInstance(_) => "conductor.instance.RestartInstance",
            Self::ConductorPlacementOpenBox(_) => "conductor.placement.OpenBox",
            Self::ConductorSessionDeliverMessage(_) => "conductor.session.DeliverMessage",
            Self::ConductorTokenLeaseToken(_) => "conductor.token.LeaseToken",
            Self::ConductorTokenWithdrawToken(_) => "conductor.token.WithdrawToken",
        }
    }
}

/// What the specification leaves to the implementor's context — a port the implementor provides.
///
/// The caller's attributes, the values the model says the implementation assigns, and the answer
/// to each `external:` branch.
pub trait Context {
    /// A new `conductor.config.RevisionId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_config_revision_id(&mut self) -> crate::config::RevisionId;

    /// A new `conductor.direction.ServingId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_direction_serving_id(&mut self) -> crate::direction::ServingId;

    /// A new `conductor.dispatch.GuardDecisionId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_dispatch_guard_decision_id(&mut self) -> crate::dispatch::GuardDecisionId;

    /// A new `conductor.dispatch.MessageId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_dispatch_message_id(&mut self) -> crate::dispatch::MessageId;

    /// A new `conductor.dispatch.ResourceRequestId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_dispatch_resource_request_id(&mut self) -> crate::dispatch::ResourceRequestId;

    /// A new `conductor.host.GrantId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_host_grant_id(&mut self) -> crate::host::GrantId;

    /// A new `conductor.host.ReclaimRunId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_host_reclaim_run_id(&mut self) -> crate::host::ReclaimRunId;

    /// A new `conductor.instance.ReconciliationId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_instance_reconciliation_id(&mut self) -> crate::instance::ReconciliationId;

    /// A new `conductor.observation.BoardId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_observation_board_id(&mut self) -> crate::observation::BoardId;

    /// A new `conductor.observation.ObservationId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_observation_observation_id(&mut self) -> crate::observation::ObservationId;

    /// A new `conductor.observation.SnapshotId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_observation_snapshot_id(&mut self) -> crate::observation::SnapshotId;

    /// A new `conductor.placement.BoxId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_placement_box_id(&mut self) -> crate::placement::BoxId;

    /// A new `conductor.placement.RefusalReason`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_placement_refusal_reason(&mut self) -> crate::placement::RefusalReason;

    /// A new `conductor.session.DeliveryId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_session_delivery_id(&mut self) -> crate::session::DeliveryId;

    /// A new `conductor.session.TickId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_session_tick_id(&mut self) -> crate::session::TickId;

    /// A new `conductor.token.TokenId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_token_token_id(&mut self) -> crate::token::TokenId;

    /// A new `conductor.tool.ToolCallId`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_conductor_tool_tool_call_id(&mut self) -> crate::tool::ToolCallId;

    /// A new `String`, which the model says the implementation assigns — a created identity, a
    /// `{generated: true}` value, or an event field the model leaves undetermined.
    fn generate_string(&mut self) -> String;

    /// Whether the external branch `outcome` of `command` is taken on this invocation.
    ///
    /// Asked in declaration order, before the branch's input guard is read; the first branch
    /// answered `true` whose guard holds is taken. A test forces a branch by answering `true`
    /// for it alone; a deployment asks whatever decides it.
    fn external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> bool;
}

/// Context answers that may be unavailable, without fabricated values.
/// Existing `Context` implementations receive the blanket adapter.
pub trait TryContext {
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_config_revision_id(&mut self) -> Result<crate::config::RevisionId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_direction_serving_id(&mut self) -> Result<crate::direction::ServingId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_dispatch_guard_decision_id(&mut self) -> Result<crate::dispatch::GuardDecisionId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<crate::dispatch::MessageId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_dispatch_resource_request_id(&mut self) -> Result<crate::dispatch::ResourceRequestId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_host_grant_id(&mut self) -> Result<crate::host::GrantId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_host_reclaim_run_id(&mut self) -> Result<crate::host::ReclaimRunId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_instance_reconciliation_id(&mut self) -> Result<crate::instance::ReconciliationId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_observation_board_id(&mut self) -> Result<crate::observation::BoardId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_observation_observation_id(&mut self) -> Result<crate::observation::ObservationId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_observation_snapshot_id(&mut self) -> Result<crate::observation::SnapshotId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_placement_box_id(&mut self) -> Result<crate::placement::BoxId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_placement_refusal_reason(&mut self) -> Result<crate::placement::RefusalReason, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_session_delivery_id(&mut self) -> Result<crate::session::DeliveryId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_session_tick_id(&mut self) -> Result<crate::session::TickId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_token_token_id(&mut self) -> Result<crate::token::TokenId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_conductor_tool_tool_call_id(&mut self) -> Result<crate::tool::ToolCallId, UnmetObligation>;
/// Assigns the value, or names the unavailable answer.
fn try_generate_string(&mut self) -> Result<String, UnmetObligation>;
/// Decides the named external branch, or names the unavailable answer.
fn try_external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> Result<bool, UnmetObligation>;
}

impl<T: Context + ?Sized> TryContext for T {
fn try_generate_conductor_config_revision_id(&mut self) -> Result<crate::config::RevisionId, UnmetObligation> { Ok(Context::generate_conductor_config_revision_id(self)) }
fn try_generate_conductor_direction_serving_id(&mut self) -> Result<crate::direction::ServingId, UnmetObligation> { Ok(Context::generate_conductor_direction_serving_id(self)) }
fn try_generate_conductor_dispatch_guard_decision_id(&mut self) -> Result<crate::dispatch::GuardDecisionId, UnmetObligation> { Ok(Context::generate_conductor_dispatch_guard_decision_id(self)) }
fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<crate::dispatch::MessageId, UnmetObligation> { Ok(Context::generate_conductor_dispatch_message_id(self)) }
fn try_generate_conductor_dispatch_resource_request_id(&mut self) -> Result<crate::dispatch::ResourceRequestId, UnmetObligation> { Ok(Context::generate_conductor_dispatch_resource_request_id(self)) }
fn try_generate_conductor_host_grant_id(&mut self) -> Result<crate::host::GrantId, UnmetObligation> { Ok(Context::generate_conductor_host_grant_id(self)) }
fn try_generate_conductor_host_reclaim_run_id(&mut self) -> Result<crate::host::ReclaimRunId, UnmetObligation> { Ok(Context::generate_conductor_host_reclaim_run_id(self)) }
fn try_generate_conductor_instance_reconciliation_id(&mut self) -> Result<crate::instance::ReconciliationId, UnmetObligation> { Ok(Context::generate_conductor_instance_reconciliation_id(self)) }
fn try_generate_conductor_observation_board_id(&mut self) -> Result<crate::observation::BoardId, UnmetObligation> { Ok(Context::generate_conductor_observation_board_id(self)) }
fn try_generate_conductor_observation_observation_id(&mut self) -> Result<crate::observation::ObservationId, UnmetObligation> { Ok(Context::generate_conductor_observation_observation_id(self)) }
fn try_generate_conductor_observation_snapshot_id(&mut self) -> Result<crate::observation::SnapshotId, UnmetObligation> { Ok(Context::generate_conductor_observation_snapshot_id(self)) }
fn try_generate_conductor_placement_box_id(&mut self) -> Result<crate::placement::BoxId, UnmetObligation> { Ok(Context::generate_conductor_placement_box_id(self)) }
fn try_generate_conductor_placement_refusal_reason(&mut self) -> Result<crate::placement::RefusalReason, UnmetObligation> { Ok(Context::generate_conductor_placement_refusal_reason(self)) }
fn try_generate_conductor_session_delivery_id(&mut self) -> Result<crate::session::DeliveryId, UnmetObligation> { Ok(Context::generate_conductor_session_delivery_id(self)) }
fn try_generate_conductor_session_tick_id(&mut self) -> Result<crate::session::TickId, UnmetObligation> { Ok(Context::generate_conductor_session_tick_id(self)) }
fn try_generate_conductor_token_token_id(&mut self) -> Result<crate::token::TokenId, UnmetObligation> { Ok(Context::generate_conductor_token_token_id(self)) }
fn try_generate_conductor_tool_tool_call_id(&mut self) -> Result<crate::tool::ToolCallId, UnmetObligation> { Ok(Context::generate_conductor_tool_tool_call_id(self)) }
fn try_generate_string(&mut self) -> Result<String, UnmetObligation> { Ok(Context::generate_string(self)) }
fn try_external(&mut self, command: ExternalCommand<'_>, outcome: &'static str) -> Result<bool, UnmetObligation> { Ok(Context::external(self, command, outcome)) }
}

/// An unavailable runtime context answer, rather than a new planned capability.
pub fn unmet_context(source: &'static str) -> UnmetObligation { UnmetObligation { capability: "context answer", source } }

/// Every generated behaviour of this workspace, over the ports `P` supplies.
///
/// `P` implements the storage trait of each entity a generated behaviour reads or writes,
/// `TryContext` (or its legacy `Context` blanket adapter) where one asks it anything, and every `…Behavior` and `…Query` trait the plan still
/// owes; `Generated<P>` forwards those to it.
pub struct Generated<P> {
    /// The storage and context ports, and every behaviour or query still owed.
    pub ports: P,
}

impl<P> Generated<P> {
    /// The generated behaviours, over `ports`.
    pub fn new(ports: P) -> Self {
        Self { ports }
    }
}

impl<P: crate::agent::obligations::AttachAgentBehavior> crate::agent::obligations::AttachAgentBehavior for Generated<P> {
    fn attach_agent(&mut self, input: crate::agent::AttachAgent) -> Result<crate::agent::AttachAgentOutcome, UnmetObligation> {
        crate::agent::obligations::AttachAgentBehavior::attach_agent(&mut self.ports, input)
    }
}

/// `conductor.agent.DeclareAgentKind`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::DeclareAgentKindBehavior for Generated<P>
where
    P: AgentKindStorage,
{
    fn declare_agent_kind(&mut self, input: crate::agent::DeclareAgentKind) -> Result<crate::agent::DeclareAgentKindOutcome, UnmetObligation> {
        let _ = &input;
        // `redeclared`: the default.
        let Some(held) = AgentKindStorage::get(&self.ports, &input.name) else {
            let identity: crate::agent::AgentKindName = input.name.clone();
            let data = crate::agent::AgentKindData {
                name: identity.clone(),
                capabilities: input.capabilities.clone(),
                binary: input.binary.clone(),
                actor: input.actor.clone(),
            };
            let answer = crate::agent::DeclareAgentKindOutcome::Declared { agent_kind_declared: crate::agent::AgentKindDeclared { actor: input.actor.clone(), name: identity.clone(), capabilities: input.capabilities.clone(), binary: input.binary.clone() } };
            AgentKindStorage::put(&mut self.ports, crate::agent::AnyAgentKind::Available(crate::agent::AgentKind::new(data)).snapshot());
            return Ok(answer);
        };
        let _ = &held;
        let mut next = held;
        next.data.capabilities = input.capabilities.clone();
        next.data.binary = input.binary.clone();
        next.data.actor = input.actor.clone();
        let answer = crate::agent::DeclareAgentKindOutcome::Redeclared { agent_kind_declared: crate::agent::AgentKindDeclared { actor: input.actor.clone(), name: input.name.clone(), capabilities: input.capabilities.clone(), binary: input.binary.clone() } };
        AgentKindStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::agent::obligations::DeliverToAgentBehavior> crate::agent::obligations::DeliverToAgentBehavior for Generated<P> {
    fn deliver_to_agent(&mut self, input: crate::agent::DeliverToAgent) -> Result<crate::agent::DeliverToAgentOutcome, UnmetObligation> {
        crate::agent::obligations::DeliverToAgentBehavior::deliver_to_agent(&mut self.ports, input)
    }
}

/// `conductor.agent.MarkAgentRunning`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::MarkAgentRunningBehavior for Generated<P>
where
    P: AgentStorage,
{
    fn mark_agent_running(&mut self, input: crate::agent::MarkAgentRunning) -> Result<crate::agent::MarkAgentRunningOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if AgentStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::agent::MarkAgentRunningOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        }
        // `running`: the default.
        let Some(held) = AgentStorage::get(&self.ports, &input.name) else {
            return Ok(crate::agent::MarkAgentRunningOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::agent::AnyAgent::Starting(instance) => crate::agent::AnyAgent::Running(instance.run()),
            _ => return Ok(crate::agent::MarkAgentRunningOutcome::NotStarting { error: crate::agent::AgentNotRunning { name: input.name.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::agent::MarkAgentRunningOutcome::Running { agent_running: crate::agent::AgentRunning { actor: input.actor.clone(), name: input.name.clone(), running_at: input.running_at.clone() } };
        AgentStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.agent.RecordAgentReport`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::RecordAgentReportBehavior for Generated<P>
where
    P: AgentStorage,
{
    fn record_agent_report(&mut self, input: crate::agent::RecordAgentReport) -> Result<crate::agent::RecordAgentReportOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if AgentStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::agent::RecordAgentReportOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        }
        // `reported`: the default.
        let Some(held) = AgentStorage::get(&self.ports, &input.name) else {
            return Ok(crate::agent::RecordAgentReportOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let next = held;
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::agent::RecordAgentReportOutcome::Reported { agent_reported: crate::agent::AgentReported { actor: input.actor.clone(), name: input.name.clone(), line: input.line.clone(), exit_code: input.exit_code.clone(), events_path: input.events_path.clone(), reported_at: input.reported_at.clone() } };
        AgentStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.agent.RestartAgent`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::RestartAgentBehavior for Generated<P>
where
    P: TryContext + AgentStorage,
{
    fn restart_agent(&mut self, input: crate::agent::RestartAgent) -> Result<crate::agent::RestartAgentOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if AgentStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::agent::RestartAgentOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        }
        // `launch-failed`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorAgentRestartAgent(&input), "launch-failed")? {
            return Ok(crate::agent::RestartAgentOutcome::LaunchFailed { error: crate::agent::AgentLaunchFailed { name: input.name.clone() } });
        }
        // `restarted`: the default.
        let Some(held) = AgentStorage::get(&self.ports, &input.name) else {
            return Ok(crate::agent::RestartAgentOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::agent::AnyAgent::Running(instance) => crate::agent::AnyAgent::Starting(instance.restart()),
            crate::agent::AnyAgent::Starting(instance) => crate::agent::AnyAgent::Starting(instance.restart()),
            crate::agent::AnyAgent::Stopped(instance) => crate::agent::AnyAgent::Starting(instance.restart()),
        };
        let mut next = moved.snapshot();
        next.data.token_owner = input.token_owner.clone();
        next.data.started_at = input.started_at.clone();
        next.data.generation = before.generation + 1;
        next.data.stopped_at = None;
        next.data.exit_class = None;
        next.data.exit_code = None;
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::agent::RestartAgentOutcome::Restarted { agent_started: crate::agent::AgentStarted { actor: input.actor.clone(), name: input.name.clone(), instance: before.instance.clone(), role: before.role.clone(), kind: before.kind.clone(), r#box: before.r#box.clone(), token_owner: input.token_owner.clone(), started_at: input.started_at.clone() } };
        AgentStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::agent::obligations::StartAgentBehavior> crate::agent::obligations::StartAgentBehavior for Generated<P> {
    fn start_agent(&mut self, input: crate::agent::StartAgent) -> Result<crate::agent::StartAgentOutcome, UnmetObligation> {
        crate::agent::obligations::StartAgentBehavior::start_agent(&mut self.ports, input)
    }
}

/// `conductor.agent.StopAgent`, generated: every outcome is one the specification fully determines.
impl<P> crate::agent::obligations::StopAgentBehavior for Generated<P>
where
    P: AgentStorage,
{
    fn stop_agent(&mut self, input: crate::agent::StopAgent) -> Result<crate::agent::StopAgentOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if AgentStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::agent::StopAgentOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        }
        // `stopped`: the default.
        let Some(held) = AgentStorage::get(&self.ports, &input.name) else {
            return Ok(crate::agent::StopAgentOutcome::NoSuchAgent { error: crate::agent::AgentNotFound { name: input.name.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::agent::AnyAgent::Running(instance) => crate::agent::AnyAgent::Stopped(instance.stop()),
            crate::agent::AnyAgent::Starting(instance) => crate::agent::AnyAgent::Stopped(instance.stop()),
            _ => return Ok(crate::agent::StopAgentOutcome::NotRunning { error: crate::agent::AgentNotRunning { name: input.name.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.stopped_at = Some(input.stopped_at.clone());
        next.data.exit_class = Some(input.exit_class.clone());
        next.data.exit_code = input.exit_code.clone();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::agent::StopAgentOutcome::Stopped { agent_stopped: crate::agent::AgentStopped { actor: input.actor.clone(), name: input.name.clone(), exit_class: input.exit_class.clone(), exit_code: input.exit_code.clone(), stopped_at: input.stopped_at.clone() } };
        AgentStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.config.AddInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::config::obligations::AddInstanceBehavior for Generated<P>
where
    P: TryContext + ConfigRevisionStorage,
{
    fn add_instance(&mut self, input: crate::config::AddInstance) -> Result<crate::config::AddInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(all(&[equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("ControlPlane".to_owned())).map(|value| !value)]), "conductor.config.AddInstance")? {
            return Ok(crate::config::AddInstanceOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `exists`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorConfigAddInstance(&input), "exists")? {
            return Ok(crate::config::AddInstanceOutcome::Exists { error: crate::config::InstanceExists { name: input.name.clone() } });
        }
        // `invalid`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorConfigAddInstance(&input), "invalid")? {
            return Ok(crate::config::AddInstanceOutcome::Invalid { error: crate::config::ConfigInvalid { path: self.ports.try_generate_string()? } });
        }
        // `changed`: the default.
        let identity: crate::config::RevisionId = self.ports.try_generate_conductor_config_revision_id()?;
        let data = crate::config::ConfigRevisionData {
            revision_id: identity.clone(),
            actor: input.actor.clone(),
            level: crate::config::Level::Instance,
            instance: Some(input.name.clone()),
            role: None,
            path: ".".to_owned(),
            value: Some(input.mapping.clone()),
            reason: input.reason.clone(),
            commit: self.ports.try_generate_string()?,
            changed_at: input.changed_at.clone(),
        };
        let answer = crate::config::AddInstanceOutcome::Changed { instance_added: crate::config::InstanceAdded { revision_id: identity.clone(), actor: input.actor.clone(), name: input.name.clone(), mapping: input.mapping.clone(), reason: input.reason.clone() } };
        ConfigRevisionStorage::put(&mut self.ports, crate::config::AnyConfigRevision::Committed(crate::config::ConfigRevision::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.config.PutConfig`, generated: every outcome is one the specification fully determines.
impl<P> crate::config::obligations::PutConfigBehavior for Generated<P>
where
    P: TryContext + ConfigRevisionStorage,
{
    fn put_config(&mut self, input: crate::config::PutConfig) -> Result<crate::config::PutConfigOutcome, UnmetObligation> {
        let _ = &input;
        // `level-scope-mismatch`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[equal(Some(&input.level).map(|value| match value { crate::config::Level::Default => "Default", crate::config::Level::Global => "Global", crate::config::Level::Instance => "Instance", crate::config::Level::Role => "Role" }.to_owned()), Some("Default".to_owned())), all(&[equal(Some(&input.level).map(|value| match value { crate::config::Level::Default => "Default", crate::config::Level::Global => "Global", crate::config::Level::Instance => "Instance", crate::config::Level::Role => "Role" }.to_owned()), Some("Global".to_owned())), Some(Some(&input.instance).and_then(|value| value.as_ref()).map(|value| &value.0).is_some())]), all(&[equal(Some(&input.level).map(|value| match value { crate::config::Level::Default => "Default", crate::config::Level::Global => "Global", crate::config::Level::Instance => "Instance", crate::config::Level::Role => "Role" }.to_owned()), Some("Global".to_owned())), Some(Some(&input.role).and_then(|value| value.as_ref()).map(|value| &value.0).is_some())]), all(&[equal(Some(&input.level).map(|value| match value { crate::config::Level::Default => "Default", crate::config::Level::Global => "Global", crate::config::Level::Instance => "Instance", crate::config::Level::Role => "Role" }.to_owned()), Some("Instance".to_owned())), (Some(Some(&input.instance).and_then(|value| value.as_ref()).map(|value| &value.0).is_some())).map(|value| !value)]), all(&[equal(Some(&input.level).map(|value| match value { crate::config::Level::Default => "Default", crate::config::Level::Global => "Global", crate::config::Level::Instance => "Instance", crate::config::Level::Role => "Role" }.to_owned()), Some("Instance".to_owned())), Some(Some(&input.role).and_then(|value| value.as_ref()).map(|value| &value.0).is_some())]), all(&[equal(Some(&input.level).map(|value| match value { crate::config::Level::Default => "Default", crate::config::Level::Global => "Global", crate::config::Level::Instance => "Instance", crate::config::Level::Role => "Role" }.to_owned()), Some("Role".to_owned())), (Some(Some(&input.instance).and_then(|value| value.as_ref()).map(|value| &value.0).is_some())).map(|value| !value)]), all(&[equal(Some(&input.level).map(|value| match value { crate::config::Level::Default => "Default", crate::config::Level::Global => "Global", crate::config::Level::Instance => "Instance", crate::config::Level::Role => "Role" }.to_owned()), Some("Role".to_owned())), (Some(Some(&input.role).and_then(|value| value.as_ref()).map(|value| &value.0).is_some())).map(|value| !value)])]), "conductor.config.PutConfig")? {
            return Ok(crate::config::PutConfigOutcome::LevelScopeMismatch { error: crate::config::LevelScopeMismatch { level: input.level.clone() } });
        }
        // `actor-not-human`: an external branch, where the context takes it and its input is eligible.
        if self.ports.try_external(ExternalCommand::ConductorConfigPutConfig(&input), "actor-not-human")? && decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.config.PutConfig")? {
            return Ok(crate::config::PutConfigOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `invalid`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorConfigPutConfig(&input), "invalid")? {
            return Ok(crate::config::PutConfigOutcome::Invalid { error: crate::config::ConfigInvalid { path: input.path.clone() } });
        }
        // `changed`: the default.
        let identity: crate::config::RevisionId = self.ports.try_generate_conductor_config_revision_id()?;
        let data = crate::config::ConfigRevisionData {
            revision_id: identity.clone(),
            actor: input.actor.clone(),
            level: input.level.clone(),
            instance: input.instance.clone(),
            role: input.role.clone(),
            path: input.path.clone(),
            value: input.value.clone(),
            reason: input.reason.clone(),
            commit: self.ports.try_generate_string()?,
            changed_at: input.changed_at.clone(),
        };
        let answer = crate::config::PutConfigOutcome::Changed { config_changed: crate::config::ConfigChanged { revision_id: identity.clone(), actor: input.actor.clone(), level: input.level.clone(), instance_name: input.instance.clone(), role: input.role.clone(), path: input.path.clone(), value: input.value.clone(), reason: input.reason.clone() } };
        ConfigRevisionStorage::put(&mut self.ports, crate::config::AnyConfigRevision::Committed(crate::config::ConfigRevision::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.config.PutInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::config::obligations::PutInstanceBehavior for Generated<P>
where
    P: TryContext + ConfigRevisionStorage,
{
    fn put_instance(&mut self, input: crate::config::PutInstance) -> Result<crate::config::PutInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(all(&[equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("ControlPlane".to_owned())).map(|value| !value)]), "conductor.config.PutInstance")? {
            return Ok(crate::config::PutInstanceOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `unknown`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorConfigPutInstance(&input), "unknown")? {
            return Ok(crate::config::PutInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        }
        // `invalid`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorConfigPutInstance(&input), "invalid")? {
            return Ok(crate::config::PutInstanceOutcome::Invalid { error: crate::config::ConfigInvalid { path: self.ports.try_generate_string()? } });
        }
        // `changed`: the default.
        let identity: crate::config::RevisionId = self.ports.try_generate_conductor_config_revision_id()?;
        let data = crate::config::ConfigRevisionData {
            revision_id: identity.clone(),
            actor: input.actor.clone(),
            level: crate::config::Level::Instance,
            instance: Some(input.name.clone()),
            role: None,
            path: ".".to_owned(),
            value: Some(input.mapping.clone()),
            reason: input.reason.clone(),
            commit: self.ports.try_generate_string()?,
            changed_at: input.changed_at.clone(),
        };
        let answer = crate::config::PutInstanceOutcome::Changed { instance_replaced: crate::config::InstanceReplaced { revision_id: identity.clone(), actor: input.actor.clone(), name: input.name.clone(), mapping: input.mapping.clone(), reason: input.reason.clone() } };
        ConfigRevisionStorage::put(&mut self.ports, crate::config::AnyConfigRevision::Committed(crate::config::ConfigRevision::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.config.RemoveInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::config::obligations::RemoveInstanceBehavior for Generated<P>
where
    P: TryContext + ConfigRevisionStorage,
{
    fn remove_instance(&mut self, input: crate::config::RemoveInstance) -> Result<crate::config::RemoveInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(all(&[equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("ControlPlane".to_owned())).map(|value| !value)]), "conductor.config.RemoveInstance")? {
            return Ok(crate::config::RemoveInstanceOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `unknown`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorConfigRemoveInstance(&input), "unknown")? {
            return Ok(crate::config::RemoveInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        }
        // `invalid`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorConfigRemoveInstance(&input), "invalid")? {
            return Ok(crate::config::RemoveInstanceOutcome::Invalid { error: crate::config::ConfigInvalid { path: self.ports.try_generate_string()? } });
        }
        // `changed`: the default.
        let identity: crate::config::RevisionId = self.ports.try_generate_conductor_config_revision_id()?;
        let data = crate::config::ConfigRevisionData {
            revision_id: identity.clone(),
            actor: input.actor.clone(),
            level: crate::config::Level::Instance,
            instance: Some(input.name.clone()),
            role: None,
            path: ".".to_owned(),
            value: None,
            reason: input.reason.clone(),
            commit: self.ports.try_generate_string()?,
            changed_at: input.changed_at.clone(),
        };
        let answer = crate::config::RemoveInstanceOutcome::Changed { instance_removed: crate::config::InstanceRemoved { revision_id: identity.clone(), actor: input.actor.clone(), name: input.name.clone(), reason: input.reason.clone() } };
        ConfigRevisionStorage::put(&mut self.ports, crate::config::AnyConfigRevision::Committed(crate::config::ConfigRevision::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.decision.AmendEscalatedRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::AmendEscalatedRequestBehavior for Generated<P>
where
    P: DecisionRequestStorage,
{
    fn amend_escalated_request(&mut self, input: crate::decision::AmendEscalatedRequest) -> Result<crate::decision::AmendEscalatedRequestOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DecisionRequestStorage::get(&self.ports, &input.request_id).is_none() {
            return Ok(crate::decision::AmendEscalatedRequestOutcome::NoSuchRequest { error: crate::decision::RequestNotFound { request_id: input.request_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
            return Ok(crate::decision::AmendEscalatedRequestOutcome::NoSuchRequest { error: crate::decision::RequestNotFound { request_id: input.request_id.clone() } });
        };
        let _ = &held;
        // `amended`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::decision::DecisionRequestState::Escalated)), "conductor.decision.AmendEscalatedRequest")? {
            let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
                return Ok(crate::decision::AmendEscalatedRequestOutcome::NoSuchRequest { error: crate::decision::RequestNotFound { request_id: input.request_id.clone() } });
            };
            let _ = &held;
            let mut next = held;
            next.data.dry_run = input.dry_run.clone();
            next.data.kind = input.kind.clone();
            next.data.lettered_options = input.lettered_options.clone();
            next.data.recommended_letters = input.recommended_letters.clone();
            next.data.default_letters = input.default_letters.clone();
            next.data.deadline = input.deadline.clone();
            let answer = crate::decision::AmendEscalatedRequestOutcome::Amended { request_amended: crate::decision::RequestAmended { actor: input.actor.clone(), request_id: input.request_id.clone() } };
            DecisionRequestStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `not-escalated`: the default.
        return Ok(crate::decision::AmendEscalatedRequestOutcome::NotEscalated { error: crate::decision::RequestStateConflict { request_id: input.request_id.clone() } });
    }
}

/// `conductor.decision.AnswerEscalatedRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::AnswerEscalatedRequestBehavior for Generated<P>
where
    P: DecisionStorage + DecisionRequestStorage,
{
    fn answer_escalated_request(&mut self, input: crate::decision::AnswerEscalatedRequest) -> Result<crate::decision::AnswerEscalatedRequestOutcome, UnmetObligation> {
        let _ = &input;
        // `agent-actor`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.decision.AnswerEscalatedRequest")? {
            return Ok(crate::decision::AnswerEscalatedRequestOutcome::AgentActor { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `when_related:` reads the `conductor.decision.Decision` row `input.decision_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.decision_id);
        let related = reference.and_then(|identity| DecisionStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
                {
                let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
                    return Ok(crate::decision::AnswerEscalatedRequestOutcome::WrongStateUnknownInstance);
                };
                let _ = &held;
                if !matches!(held.state, crate::decision::DecisionRequestState::Escalated) {
                    return Ok(crate::decision::AnswerEscalatedRequestOutcome::WrongState { error: crate::decision::RequestStateConflict { request_id: input.request_id.clone() } });
                }
                }
        }
        // `no-such-decision`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::decision::AnswerEscalatedRequestOutcome::NoSuchDecision { error: crate::decision::DecisionNotFound { decision_id: input.decision_id.clone() } });
        }
        // `reversed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::decision::DecisionState::InForce => "InForce", crate::decision::DecisionState::Reversed => "Reversed" }.to_owned()), Some("Reversed".to_owned())), "conductor.decision.AnswerEscalatedRequest")? {
            return Ok(crate::decision::AnswerEscalatedRequestOutcome::Reversed { error: crate::decision::DecisionReversed { decision_id: input.decision_id.clone() } });
        }
        }
        // `answered`: the default.
        let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
            return Ok(crate::decision::AnswerEscalatedRequestOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::decision::AnyDecisionRequest::Escalated(instance) => crate::decision::AnyDecisionRequest::Answered(instance.answer_escalated()),
            _ => return Ok(crate::decision::AnswerEscalatedRequestOutcome::WrongState { error: crate::decision::RequestStateConflict { request_id: input.request_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.decision_id = Some(input.decision_id.clone());
        let answer = crate::decision::AnswerEscalatedRequestOutcome::Answered { request_answered: crate::decision::RequestAnswered { actor: input.actor.clone(), request_id: input.request_id.clone(), decision_id: input.decision_id.clone() } };
        DecisionRequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.decision.AnswerRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::AnswerRequestBehavior for Generated<P>
where
    P: DecisionStorage + DecisionRequestStorage,
{
    fn answer_request(&mut self, input: crate::decision::AnswerRequest) -> Result<crate::decision::AnswerRequestOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.decision.Decision` row `input.decision_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.decision_id);
        let related = reference.and_then(|identity| DecisionStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
                {
                let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
                    return Ok(crate::decision::AnswerRequestOutcome::WrongStateUnknownInstance);
                };
                let _ = &held;
                if !matches!(held.state, crate::decision::DecisionRequestState::Open) {
                    return Ok(crate::decision::AnswerRequestOutcome::WrongState { error: crate::decision::RequestStateConflict { request_id: input.request_id.clone() } });
                }
                }
        }
        // `no-such-decision`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::decision::AnswerRequestOutcome::NoSuchDecision { error: crate::decision::DecisionNotFound { decision_id: input.decision_id.clone() } });
        }
        // `agent-actor`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(all(&[all(&[equal(Some(&related.data.decision_class).map(|value| match value { crate::decision::DecisionClass::C => "C", crate::decision::DecisionClass::O => "O", crate::decision::DecisionClass::H => "H" }.to_owned()), Some("H".to_owned())), equal(Some(&related.state).map(|value| match value { crate::decision::DecisionState::InForce => "InForce", crate::decision::DecisionState::Reversed => "Reversed" }.to_owned()), Some("InForce".to_owned()))]), equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value)]), "conductor.decision.AnswerRequest")? {
            return Ok(crate::decision::AnswerRequestOutcome::AgentActor { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        }
        // `reversed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::decision::DecisionState::InForce => "InForce", crate::decision::DecisionState::Reversed => "Reversed" }.to_owned()), Some("Reversed".to_owned())), "conductor.decision.AnswerRequest")? {
            return Ok(crate::decision::AnswerRequestOutcome::Reversed { error: crate::decision::DecisionReversed { decision_id: input.decision_id.clone() } });
        }
        }
        // `answered`: the default.
        let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
            return Ok(crate::decision::AnswerRequestOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::decision::AnyDecisionRequest::Open(instance) => crate::decision::AnyDecisionRequest::Answered(instance.answer()),
            _ => return Ok(crate::decision::AnswerRequestOutcome::WrongState { error: crate::decision::RequestStateConflict { request_id: input.request_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.decision_id = Some(input.decision_id.clone());
        let answer = crate::decision::AnswerRequestOutcome::Answered { request_answered: crate::decision::RequestAnswered { actor: input.actor.clone(), request_id: input.request_id.clone(), decision_id: input.decision_id.clone() } };
        DecisionRequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.decision.EscalateRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::EscalateRequestBehavior for Generated<P>
where
    P: DecisionRequestStorage,
{
    fn escalate_request(&mut self, input: crate::decision::EscalateRequest) -> Result<crate::decision::EscalateRequestOutcome, UnmetObligation> {
        let _ = &input;
        // `class-c`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.escalation_class).map(|value| match value { crate::decision::DecisionClass::C => "C", crate::decision::DecisionClass::O => "O", crate::decision::DecisionClass::H => "H" }.to_owned()), Some("C".to_owned())), "conductor.decision.EscalateRequest")? {
            return Ok(crate::decision::EscalateRequestOutcome::ClassC { error: crate::decision::NotEscalatable { request_id: input.request_id.clone() } });
        }
        // `untested`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(all(&[equal(Some(&input.escalation_class).map(|value| match value { crate::decision::DecisionClass::C => "C", crate::decision::DecisionClass::O => "O", crate::decision::DecisionClass::H => "H" }.to_owned()), Some("H".to_owned())), (Some(Some(&input.dry_run).and_then(|value| value.as_ref()).is_some())).map(|value| !value), (Some(Some(&input.no_dry_run).and_then(|value| value.as_ref()).is_some())).map(|value| !value)]), "conductor.decision.EscalateRequest")? {
            return Ok(crate::decision::EscalateRequestOutcome::Untested { error: crate::decision::DryRunMissing { request_id: input.request_id.clone() } });
        }
        // `incomplete`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(all(&[equal(Some(&input.escalation_class).map(|value| match value { crate::decision::DecisionClass::C => "C", crate::decision::DecisionClass::O => "O", crate::decision::DecisionClass::H => "H" }.to_owned()), Some("O".to_owned())), any(&[(Some(Some(&input.kind).and_then(|value| value.as_ref()).is_some())).map(|value| !value), (Some(Some(&input.lettered_options).and_then(|value| value.as_ref()).is_some())).map(|value| !value), (Some(Some(&input.recommended_letters).and_then(|value| value.as_ref()).is_some())).map(|value| !value), (Some(Some(&input.default_letters).and_then(|value| value.as_ref()).is_some())).map(|value| !value), (Some(Some(&input.deadline).and_then(|value| value.as_ref()).is_some())).map(|value| !value)])]), "conductor.decision.EscalateRequest")? {
            return Ok(crate::decision::EscalateRequestOutcome::Incomplete { error: crate::decision::EscalationIncomplete { request_id: input.request_id.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DecisionRequestStorage::get(&self.ports, &input.request_id).is_none() {
            return Ok(crate::decision::EscalateRequestOutcome::NoSuchRequest { error: crate::decision::RequestNotFound { request_id: input.request_id.clone() } });
        }
        // `escalated`: the default.
        let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
            return Ok(crate::decision::EscalateRequestOutcome::NoSuchRequest { error: crate::decision::RequestNotFound { request_id: input.request_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::decision::AnyDecisionRequest::Open(instance) => crate::decision::AnyDecisionRequest::Escalated(instance.escalate()),
            _ => return Ok(crate::decision::EscalateRequestOutcome::WrongState { error: crate::decision::RequestStateConflict { request_id: input.request_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.escalation_class = Some(input.escalation_class.clone());
        next.data.escalation_reason = Some(input.reason.clone());
        next.data.dry_run = input.dry_run.clone();
        next.data.no_dry_run = input.no_dry_run.clone();
        next.data.kind = input.kind.clone();
        next.data.lettered_options = input.lettered_options.clone();
        next.data.recommended_letters = input.recommended_letters.clone();
        next.data.default_letters = input.default_letters.clone();
        next.data.deadline = input.deadline.clone();
        let answer = crate::decision::EscalateRequestOutcome::Escalated { request_escalated: crate::decision::RequestEscalated { actor: input.actor.clone(), request_id: input.request_id.clone() } };
        DecisionRequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.decision.RaiseDecisionRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::RaiseDecisionRequestBehavior for Generated<P>
where
    P: DecisionRequestStorage,
{
    fn raise_decision_request(&mut self, input: crate::decision::RaiseDecisionRequest) -> Result<crate::decision::RaiseDecisionRequestOutcome, UnmetObligation> {
        let _ = &input;
        // `already-raised`: an identity a record already carries, before any branch is taken.
        if DecisionRequestStorage::get(&self.ports, &input.request_id).is_some() {
            return Ok(crate::decision::RaiseDecisionRequestOutcome::AlreadyRaised { error: crate::decision::RequestExists { request_id: input.request_id.clone() } });
        }
        // `raised`: the default.
        let identity: crate::decision::RequestId = input.request_id.clone();
        let data = crate::decision::DecisionRequestData {
            request_id: identity.clone(),
            actor: input.actor.clone(),
            repository: input.repository.clone(),
            question: input.question.clone(),
            options: input.options.clone(),
            recommendation: input.recommendation.clone(),
            raised_at: input.raised_at.clone(),
            blocker: input.blocker.clone(),
            classification_rule: input.classification_rule.clone(),
            escalation_class: None,
            escalation_reason: None,
            dry_run: None,
            no_dry_run: None,
            kind: None,
            lettered_options: None,
            recommended_letters: None,
            default_letters: None,
            deadline: None,
            decision_id: None,
        };
        let answer = crate::decision::RaiseDecisionRequestOutcome::Raised { decision_requested: crate::decision::DecisionRequested { actor: input.actor.clone(), request_id: identity.clone(), repository: input.repository.clone() } };
        DecisionRequestStorage::put(&mut self.ports, crate::decision::AnyDecisionRequest::Open(crate::decision::DecisionRequest::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.decision.RecordConductorDecision`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::RecordConductorDecisionBehavior for Generated<P>
where
    P: DecisionStorage,
{
    fn record_conductor_decision(&mut self, input: crate::decision::RecordConductorDecision) -> Result<crate::decision::RecordConductorDecisionOutcome, UnmetObligation> {
        let _ = &input;
        // `reserved`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[equal(Some(&input.decision_class).map(|value| match value { crate::decision::DecisionClass::C => "C", crate::decision::DecisionClass::O => "O", crate::decision::DecisionClass::H => "H" }.to_owned()), Some("O".to_owned())), equal(Some(&input.decision_class).map(|value| match value { crate::decision::DecisionClass::C => "C", crate::decision::DecisionClass::O => "O", crate::decision::DecisionClass::H => "H" }.to_owned()), Some("H".to_owned()))]), "conductor.decision.RecordConductorDecision")? {
            return Ok(crate::decision::RecordConductorDecisionOutcome::Reserved { error: crate::decision::ReservedForOperator { decision_id: input.decision_id.clone() } });
        }
        // `already-recorded`: an identity a record already carries, before any branch is taken.
        if DecisionStorage::get(&self.ports, &input.decision_id).is_some() {
            return Ok(crate::decision::RecordConductorDecisionOutcome::AlreadyRecorded { error: crate::decision::DecisionExists { decision_id: input.decision_id.clone() } });
        }
        // `recorded`: the default.
        let identity: crate::decision::DecisionId = input.decision_id.clone();
        let data = crate::decision::DecisionData {
            decision_id: identity.clone(),
            decision_class: input.decision_class.clone(),
            decided_by: crate::decision::Decider::Conductor,
            actor: input.actor.clone(),
            question: input.question.clone(),
            options: input.options.clone(),
            choice: input.choice.clone(),
            reason: input.reason.clone(),
            evidence: input.evidence.clone(),
            decided_at: input.decided_at.clone(),
            recorded_by: None,
            operator_words: None,
            repository: input.repository.clone(),
            decider_name: input.decider_name.clone(),
            prompt_sha256: input.prompt_sha256.clone(),
        };
        let answer = crate::decision::RecordConductorDecisionOutcome::Recorded { decision_recorded: crate::decision::DecisionRecorded { actor: input.actor.clone(), decision_id: identity.clone(), decision_class: input.decision_class.clone(), decided_by: crate::decision::Decider::Conductor } };
        DecisionStorage::put(&mut self.ports, crate::decision::AnyDecision::InForce(crate::decision::Decision::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.decision.RecordOperatorDecision`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::RecordOperatorDecisionBehavior for Generated<P>
where
    P: DecisionStorage,
{
    fn record_operator_decision(&mut self, input: crate::decision::RecordOperatorDecision) -> Result<crate::decision::RecordOperatorDecisionOutcome, UnmetObligation> {
        let _ = &input;
        // `agent-actor`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.decision.RecordOperatorDecision")? {
            return Ok(crate::decision::RecordOperatorDecisionOutcome::AgentActor { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `already-recorded`: an identity a record already carries, before any branch is taken.
        if DecisionStorage::get(&self.ports, &input.decision_id).is_some() {
            return Ok(crate::decision::RecordOperatorDecisionOutcome::AlreadyRecorded { error: crate::decision::DecisionExists { decision_id: input.decision_id.clone() } });
        }
        // `recorded`: the default.
        let identity: crate::decision::DecisionId = input.decision_id.clone();
        let data = crate::decision::DecisionData {
            decision_id: identity.clone(),
            decision_class: input.decision_class.clone(),
            decided_by: crate::decision::Decider::Operator,
            actor: input.actor.clone(),
            question: input.question.clone(),
            options: input.options.clone(),
            choice: input.choice.clone(),
            reason: input.reason.clone(),
            evidence: input.evidence.clone(),
            decided_at: input.decided_at.clone(),
            recorded_by: Some(input.recorded_by.clone()),
            operator_words: Some(input.operator_words.clone()),
            repository: input.repository.clone(),
            decider_name: None,
            prompt_sha256: None,
        };
        let answer = crate::decision::RecordOperatorDecisionOutcome::Recorded { decision_recorded: crate::decision::DecisionRecorded { actor: input.actor.clone(), decision_id: identity.clone(), decision_class: input.decision_class.clone(), decided_by: crate::decision::Decider::Operator } };
        DecisionStorage::put(&mut self.ports, crate::decision::AnyDecision::InForce(crate::decision::Decision::new(data)).snapshot());
        return Ok(answer);
    }
}

impl<P: crate::decision::obligations::RecordOperatorDecisionForRequestBehavior> crate::decision::obligations::RecordOperatorDecisionForRequestBehavior for Generated<P> {
    fn record_operator_decision_for_request(&mut self, input: crate::decision::RecordOperatorDecisionForRequest) -> Result<crate::decision::RecordOperatorDecisionForRequestOutcome, UnmetObligation> {
        crate::decision::obligations::RecordOperatorDecisionForRequestBehavior::record_operator_decision_for_request(&mut self.ports, input)
    }
}

/// `conductor.decision.ReverseConductorDecision`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::ReverseConductorDecisionBehavior for Generated<P>
where
    P: DecisionStorage,
{
    fn reverse_conductor_decision(&mut self, input: crate::decision::ReverseConductorDecision) -> Result<crate::decision::ReverseConductorDecisionOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DecisionStorage::get(&self.ports, &input.decision_id).is_none() {
            return Ok(crate::decision::ReverseConductorDecisionOutcome::NoSuchDecision { error: crate::decision::DecisionNotFound { decision_id: input.decision_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = DecisionStorage::get(&self.ports, &input.decision_id) else {
            return Ok(crate::decision::ReverseConductorDecisionOutcome::NoSuchDecision { error: crate::decision::DecisionNotFound { decision_id: input.decision_id.clone() } });
        };
        let _ = &held;
        // `operator-decided`: selected by the addressed row.
        if decided(all(&[equal(Some(&held.data.decided_by).map(|value| match value { crate::decision::Decider::Conductor => "Conductor", crate::decision::Decider::Operator => "Operator" }.to_owned()), Some("Operator".to_owned())), equal(Some(&held.state).map(|value| match value { crate::decision::DecisionState::InForce => "InForce", crate::decision::DecisionState::Reversed => "Reversed" }.to_owned()), Some("InForce".to_owned()))]), "conductor.decision.ReverseConductorDecision")? {
            return Ok(crate::decision::ReverseConductorDecisionOutcome::OperatorDecided { error: crate::decision::ReversalReservedForOperator { decision_id: input.decision_id.clone() } });
        }
        // `reversed`: the default.
        let Some(held) = DecisionStorage::get(&self.ports, &input.decision_id) else {
            return Ok(crate::decision::ReverseConductorDecisionOutcome::NoSuchDecision { error: crate::decision::DecisionNotFound { decision_id: input.decision_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::decision::AnyDecision::InForce(instance) => crate::decision::AnyDecision::Reversed(instance.reverse()),
            _ => return Ok(crate::decision::ReverseConductorDecisionOutcome::WrongState { error: crate::decision::DecisionStateConflict { decision_id: input.decision_id.clone() } }),
        };
        let next = moved.snapshot();
        let answer = crate::decision::ReverseConductorDecisionOutcome::Reversed { decision_reversed_event: crate::decision::DecisionReversedEvent { actor: input.actor.clone(), decision_id: input.decision_id.clone(), reason: input.reason.clone() } };
        DecisionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.decision.ReverseOperatorDecision`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::ReverseOperatorDecisionBehavior for Generated<P>
where
    P: DecisionStorage,
{
    fn reverse_operator_decision(&mut self, input: crate::decision::ReverseOperatorDecision) -> Result<crate::decision::ReverseOperatorDecisionOutcome, UnmetObligation> {
        let _ = &input;
        // `agent-actor`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.decision.ReverseOperatorDecision")? {
            return Ok(crate::decision::ReverseOperatorDecisionOutcome::AgentActor { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DecisionStorage::get(&self.ports, &input.decision_id).is_none() {
            return Ok(crate::decision::ReverseOperatorDecisionOutcome::NoSuchDecision { error: crate::decision::DecisionNotFound { decision_id: input.decision_id.clone() } });
        }
        // `reversed`: the default.
        let Some(held) = DecisionStorage::get(&self.ports, &input.decision_id) else {
            return Ok(crate::decision::ReverseOperatorDecisionOutcome::NoSuchDecision { error: crate::decision::DecisionNotFound { decision_id: input.decision_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::decision::AnyDecision::InForce(instance) => crate::decision::AnyDecision::Reversed(instance.reverse()),
            _ => return Ok(crate::decision::ReverseOperatorDecisionOutcome::WrongState { error: crate::decision::DecisionStateConflict { decision_id: input.decision_id.clone() } }),
        };
        let next = moved.snapshot();
        let answer = crate::decision::ReverseOperatorDecisionOutcome::Reversed { decision_reversed_event: crate::decision::DecisionReversedEvent { actor: input.actor.clone(), decision_id: input.decision_id.clone(), reason: input.reason.clone() } };
        DecisionStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.decision.WithdrawRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::decision::obligations::WithdrawRequestBehavior for Generated<P>
where
    P: DecisionRequestStorage,
{
    fn withdraw_request(&mut self, input: crate::decision::WithdrawRequest) -> Result<crate::decision::WithdrawRequestOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DecisionRequestStorage::get(&self.ports, &input.request_id).is_none() {
            return Ok(crate::decision::WithdrawRequestOutcome::NoSuchRequest { error: crate::decision::RequestNotFound { request_id: input.request_id.clone() } });
        }
        // `withdrawn`: the default.
        let Some(held) = DecisionRequestStorage::get(&self.ports, &input.request_id) else {
            return Ok(crate::decision::WithdrawRequestOutcome::NoSuchRequest { error: crate::decision::RequestNotFound { request_id: input.request_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::decision::AnyDecisionRequest::Escalated(instance) => crate::decision::AnyDecisionRequest::Withdrawn(instance.withdraw()),
            crate::decision::AnyDecisionRequest::Open(instance) => crate::decision::AnyDecisionRequest::Withdrawn(instance.withdraw()),
            _ => return Ok(crate::decision::WithdrawRequestOutcome::WrongState { error: crate::decision::RequestStateConflict { request_id: input.request_id.clone() } }),
        };
        let next = moved.snapshot();
        let answer = crate::decision::WithdrawRequestOutcome::Withdrawn { request_withdrawn: crate::decision::RequestWithdrawn { actor: input.actor.clone(), request_id: input.request_id.clone(), reason: input.reason.clone() } };
        DecisionRequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.direction.ActivateRepository`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::ActivateRepositoryBehavior for Generated<P>
where
    P: RepositoryMarkStorage,
{
    fn activate_repository(&mut self, input: crate::direction::ActivateRepository) -> Result<crate::direction::ActivateRepositoryOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if RepositoryMarkStorage::get(&self.ports, &input.repository).is_none() {
            return Ok(crate::direction::ActivateRepositoryOutcome::NoSuchMark { error: crate::direction::RepositoryMarkNotFound { repository: input.repository.clone() } });
        }
        // `activated`: the default.
        let Some(held) = RepositoryMarkStorage::get(&self.ports, &input.repository) else {
            return Ok(crate::direction::ActivateRepositoryOutcome::NoSuchMark { error: crate::direction::RepositoryMarkNotFound { repository: input.repository.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::direction::AnyRepositoryMark::Inactive(instance) => crate::direction::AnyRepositoryMark::Active(instance.activate()),
            _ => return Ok(crate::direction::ActivateRepositoryOutcome::WrongState { error: crate::direction::RepositoryMarkConflict { repository: input.repository.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.marked_by = input.marked_by.clone();
        next.data.actor = input.actor.clone();
        next.data.session = input.session.clone();
        next.data.reason = input.reason.clone();
        let answer = crate::direction::ActivateRepositoryOutcome::Activated { repository_activated: crate::direction::RepositoryActivated { actor: input.actor.clone(), repository: input.repository.clone(), marked_by: input.marked_by.clone() } };
        RepositoryMarkStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.direction.AddServing`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::AddServingBehavior for Generated<P>
where
    P: TryContext + GoalStorage + GoalServingStorage,
{
    fn add_serving(&mut self, input: crate::direction::AddServing) -> Result<crate::direction::AddServingOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.direction.Goal` row `input.goal_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.goal_id);
        let related = reference.and_then(|identity| GoalStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-goal`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::direction::AddServingOutcome::NoSuchGoal { error: crate::direction::GoalNotFound { goal_id: input.goal_id.clone() } });
        }
        // `goal-closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(any(&[equal(Some(&related.state).map(|value| match value { crate::direction::GoalState::Confirmed => "Confirmed", crate::direction::GoalState::Draft => "Draft", crate::direction::GoalState::Dropped => "Dropped", crate::direction::GoalState::Met => "Met" }.to_owned()), Some("Met".to_owned())), equal(Some(&related.state).map(|value| match value { crate::direction::GoalState::Confirmed => "Confirmed", crate::direction::GoalState::Draft => "Draft", crate::direction::GoalState::Dropped => "Dropped", crate::direction::GoalState::Met => "Met" }.to_owned()), Some("Dropped".to_owned()))]), "conductor.direction.AddServing")? {
            return Ok(crate::direction::AddServingOutcome::GoalClosed { error: crate::direction::GoalStateConflict { goal_id: input.goal_id.clone() } });
        }
        }
        // `added`: the default.
        let identity: crate::direction::ServingId = self.ports.try_generate_conductor_direction_serving_id()?;
        let data = crate::direction::GoalServingData {
            serving_id: identity.clone(),
            actor: input.actor.clone(),
            goal_id: input.goal_id.clone(),
            repository: input.repository.clone(),
        };
        let answer = crate::direction::AddServingOutcome::Added { serving_added: crate::direction::ServingAdded { actor: input.actor.clone(), serving_id: identity.clone(), goal_id: input.goal_id.clone(), repository: input.repository.clone() } };
        GoalServingStorage::put(&mut self.ports, crate::direction::AnyGoalServing::Serving(crate::direction::GoalServing::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.direction.ConfirmGoal`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::ConfirmGoalBehavior for Generated<P>
where
    P: GoalStorage,
{
    fn confirm_goal(&mut self, input: crate::direction::ConfirmGoal) -> Result<crate::direction::ConfirmGoalOutcome, UnmetObligation> {
        let _ = &input;
        // `agent-actor`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.direction.ConfirmGoal")? {
            return Ok(crate::direction::ConfirmGoalOutcome::AgentActor { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `invalid-goal-id`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[equal(Some(&input.goal_id).map(|value| &value.0).map(|value| value.clone()), Some("".to_owned())), compare_numbers(Some(&input.goal_id).map(|value| &value.0).map(|value| value.len().to_string()), Some("170".to_owned()), core::cmp::Ordering::is_gt)]), "conductor.direction.ConfirmGoal")? {
            return Ok(crate::direction::ConfirmGoalOutcome::InvalidGoalId { error: crate::direction::InvalidGoalId { goal_id: input.goal_id.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if GoalStorage::get(&self.ports, &input.goal_id).is_none() {
            return Ok(crate::direction::ConfirmGoalOutcome::NoSuchGoal { error: crate::direction::GoalNotFound { goal_id: input.goal_id.clone() } });
        }
        // `confirmed`: the default.
        let Some(held) = GoalStorage::get(&self.ports, &input.goal_id) else {
            return Ok(crate::direction::ConfirmGoalOutcome::NoSuchGoal { error: crate::direction::GoalNotFound { goal_id: input.goal_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::direction::AnyGoal::Draft(instance) => crate::direction::AnyGoal::Confirmed(instance.confirm()),
            _ => return Ok(crate::direction::ConfirmGoalOutcome::WrongState { error: crate::direction::GoalStateConflict { goal_id: input.goal_id.clone() } }),
        };
        let next = moved.snapshot();
        let answer = crate::direction::ConfirmGoalOutcome::Confirmed { goal_confirmed: crate::direction::GoalConfirmed { actor: input.actor.clone(), goal_id: input.goal_id.clone() } };
        GoalStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.direction.DeactivateRepository`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::DeactivateRepositoryBehavior for Generated<P>
where
    P: RepositoryMarkStorage,
{
    fn deactivate_repository(&mut self, input: crate::direction::DeactivateRepository) -> Result<crate::direction::DeactivateRepositoryOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if RepositoryMarkStorage::get(&self.ports, &input.repository).is_none() {
            return Ok(crate::direction::DeactivateRepositoryOutcome::NoSuchMark { error: crate::direction::RepositoryMarkNotFound { repository: input.repository.clone() } });
        }
        // `deactivated`: the default.
        let Some(held) = RepositoryMarkStorage::get(&self.ports, &input.repository) else {
            return Ok(crate::direction::DeactivateRepositoryOutcome::NoSuchMark { error: crate::direction::RepositoryMarkNotFound { repository: input.repository.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::direction::AnyRepositoryMark::Active(instance) => crate::direction::AnyRepositoryMark::Inactive(instance.deactivate()),
            _ => return Ok(crate::direction::DeactivateRepositoryOutcome::WrongState { error: crate::direction::RepositoryMarkConflict { repository: input.repository.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.marked_by = input.marked_by.clone();
        next.data.actor = input.actor.clone();
        next.data.session = input.session.clone();
        next.data.reason = input.reason.clone();
        let answer = crate::direction::DeactivateRepositoryOutcome::Deactivated { repository_deactivated: crate::direction::RepositoryDeactivated { actor: input.actor.clone(), repository: input.repository.clone(), marked_by: input.marked_by.clone(), reason: input.reason.clone() } };
        RepositoryMarkStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.direction.DropGoal`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::DropGoalBehavior for Generated<P>
where
    P: GoalStorage,
{
    fn drop_goal(&mut self, input: crate::direction::DropGoal) -> Result<crate::direction::DropGoalOutcome, UnmetObligation> {
        let _ = &input;
        // `agent-actor`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.direction.DropGoal")? {
            return Ok(crate::direction::DropGoalOutcome::AgentActor { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `invalid-goal-id`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[equal(Some(&input.goal_id).map(|value| &value.0).map(|value| value.clone()), Some("".to_owned())), compare_numbers(Some(&input.goal_id).map(|value| &value.0).map(|value| value.len().to_string()), Some("170".to_owned()), core::cmp::Ordering::is_gt)]), "conductor.direction.DropGoal")? {
            return Ok(crate::direction::DropGoalOutcome::InvalidGoalId { error: crate::direction::InvalidGoalId { goal_id: input.goal_id.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if GoalStorage::get(&self.ports, &input.goal_id).is_none() {
            return Ok(crate::direction::DropGoalOutcome::NoSuchGoal { error: crate::direction::GoalNotFound { goal_id: input.goal_id.clone() } });
        }
        // `dropped`: the default.
        let Some(held) = GoalStorage::get(&self.ports, &input.goal_id) else {
            return Ok(crate::direction::DropGoalOutcome::NoSuchGoal { error: crate::direction::GoalNotFound { goal_id: input.goal_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::direction::AnyGoal::Confirmed(instance) => crate::direction::AnyGoal::Dropped(instance.drop()),
            crate::direction::AnyGoal::Draft(instance) => crate::direction::AnyGoal::Dropped(instance.drop()),
            _ => return Ok(crate::direction::DropGoalOutcome::WrongState { error: crate::direction::GoalStateConflict { goal_id: input.goal_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.reason = Some(input.reason.clone());
        let answer = crate::direction::DropGoalOutcome::Dropped { goal_dropped: crate::direction::GoalDropped { actor: input.actor.clone(), goal_id: input.goal_id.clone(), reason: input.reason.clone() } };
        GoalStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.direction.MarkGoalMet`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::MarkGoalMetBehavior for Generated<P>
where
    P: GoalStorage,
{
    fn mark_goal_met(&mut self, input: crate::direction::MarkGoalMet) -> Result<crate::direction::MarkGoalMetOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-goal-id`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[equal(Some(&input.goal_id).map(|value| &value.0).map(|value| value.clone()), Some("".to_owned())), compare_numbers(Some(&input.goal_id).map(|value| &value.0).map(|value| value.len().to_string()), Some("170".to_owned()), core::cmp::Ordering::is_gt)]), "conductor.direction.MarkGoalMet")? {
            return Ok(crate::direction::MarkGoalMetOutcome::InvalidGoalId { error: crate::direction::InvalidGoalId { goal_id: input.goal_id.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if GoalStorage::get(&self.ports, &input.goal_id).is_none() {
            return Ok(crate::direction::MarkGoalMetOutcome::NoSuchGoal { error: crate::direction::GoalNotFound { goal_id: input.goal_id.clone() } });
        }
        // `met`: the default.
        let Some(held) = GoalStorage::get(&self.ports, &input.goal_id) else {
            return Ok(crate::direction::MarkGoalMetOutcome::NoSuchGoal { error: crate::direction::GoalNotFound { goal_id: input.goal_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::direction::AnyGoal::Confirmed(instance) => crate::direction::AnyGoal::Met(instance.meet()),
            _ => return Ok(crate::direction::MarkGoalMetOutcome::WrongState { error: crate::direction::GoalStateConflict { goal_id: input.goal_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.evidence = Some(input.evidence.clone());
        let answer = crate::direction::MarkGoalMetOutcome::Met { goal_met: crate::direction::GoalMet { actor: input.actor.clone(), goal_id: input.goal_id.clone(), evidence: input.evidence.clone() } };
        GoalStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.direction.MarkRepository`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::MarkRepositoryBehavior for Generated<P>
where
    P: RepositoryMarkStorage,
{
    fn mark_repository(&mut self, input: crate::direction::MarkRepository) -> Result<crate::direction::MarkRepositoryOutcome, UnmetObligation> {
        let _ = &input;
        // `already-marked`: an identity a record already carries, before any branch is taken.
        if RepositoryMarkStorage::get(&self.ports, &input.repository).is_some() {
            return Ok(crate::direction::MarkRepositoryOutcome::AlreadyMarked { error: crate::direction::RepositoryAlreadyMarked { repository: input.repository.clone() } });
        }
        // `marked-active`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.activity).map(|value| match value { crate::direction::Activity::Active => "Active", crate::direction::Activity::Inactive => "Inactive" }.to_owned()), Some("Active".to_owned())), "conductor.direction.MarkRepository")? {
            let identity: crate::direction::MarkedRepository = input.repository.clone();
            let data = crate::direction::RepositoryMarkData {
                repository: identity.clone(),
                marked_by: input.marked_by.clone(),
                actor: input.actor.clone(),
                session: input.session.clone(),
                reason: input.reason.clone(),
            };
            let answer = crate::direction::MarkRepositoryOutcome::MarkedActive { repository_marked: crate::direction::RepositoryMarked { actor: input.actor.clone(), repository: identity.clone(), activity: input.activity.clone(), marked_by: input.marked_by.clone() } };
            RepositoryMarkStorage::put(&mut self.ports, crate::direction::AnyRepositoryMark::Active(crate::direction::RepositoryMark::new_active(data)).snapshot());
            return Ok(answer);
        }
        // `marked-inactive`: the default.
        let identity: crate::direction::MarkedRepository = input.repository.clone();
        let data = crate::direction::RepositoryMarkData {
            repository: identity.clone(),
            marked_by: input.marked_by.clone(),
            actor: input.actor.clone(),
            session: input.session.clone(),
            reason: input.reason.clone(),
        };
        let answer = crate::direction::MarkRepositoryOutcome::MarkedInactive { repository_marked: crate::direction::RepositoryMarked { actor: input.actor.clone(), repository: identity.clone(), activity: input.activity.clone(), marked_by: input.marked_by.clone() } };
        RepositoryMarkStorage::put(&mut self.ports, crate::direction::AnyRepositoryMark::Inactive(crate::direction::RepositoryMark::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.direction.ProposeGoal`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::ProposeGoalBehavior for Generated<P>
where
    P: GoalStorage,
{
    fn propose_goal(&mut self, input: crate::direction::ProposeGoal) -> Result<crate::direction::ProposeGoalOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-goal-id`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[equal(Some(&input.goal_id).map(|value| &value.0).map(|value| value.clone()), Some("".to_owned())), compare_numbers(Some(&input.goal_id).map(|value| &value.0).map(|value| value.len().to_string()), Some("170".to_owned()), core::cmp::Ordering::is_gt)]), "conductor.direction.ProposeGoal")? {
            return Ok(crate::direction::ProposeGoalOutcome::InvalidGoalId { error: crate::direction::InvalidGoalId { goal_id: input.goal_id.clone() } });
        }
        // `already-proposed`: an identity a record already carries, before any branch is taken.
        if GoalStorage::get(&self.ports, &input.goal_id).is_some() {
            return Ok(crate::direction::ProposeGoalOutcome::AlreadyProposed { error: crate::direction::GoalExists { goal_id: input.goal_id.clone() } });
        }
        // `proposed`: the default.
        let identity: crate::direction::GoalId = input.goal_id.clone();
        let data = crate::direction::GoalData {
            goal_id: identity.clone(),
            actor: input.actor.clone(),
            title: input.title.clone(),
            exit_evidence: input.exit_evidence.clone(),
            evidence: None,
            reason: None,
        };
        let answer = crate::direction::ProposeGoalOutcome::Proposed { goal_proposed: crate::direction::GoalProposed { actor: input.actor.clone(), goal_id: identity.clone(), title: input.title.clone() } };
        GoalStorage::put(&mut self.ports, crate::direction::AnyGoal::Draft(crate::direction::Goal::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.direction.RemoveServing`, generated: every outcome is one the specification fully determines.
impl<P> crate::direction::obligations::RemoveServingBehavior for Generated<P>
where
    P: GoalServingStorage,
{
    fn remove_serving(&mut self, input: crate::direction::RemoveServing) -> Result<crate::direction::RemoveServingOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if GoalServingStorage::get(&self.ports, &input.serving_id).is_none() {
            return Ok(crate::direction::RemoveServingOutcome::NoSuchServing { error: crate::direction::ServingNotFound { serving_id: input.serving_id.clone() } });
        }
        // `removed`: the default.
        let Some(held) = GoalServingStorage::get(&self.ports, &input.serving_id) else {
            return Ok(crate::direction::RemoveServingOutcome::NoSuchServing { error: crate::direction::ServingNotFound { serving_id: input.serving_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::direction::AnyGoalServing::Serving(instance) => crate::direction::AnyGoalServing::Removed(instance.remove()),
            _ => return Ok(crate::direction::RemoveServingOutcome::WrongState { error: crate::direction::ServingStateConflict { serving_id: input.serving_id.clone() } }),
        };
        let next = moved.snapshot();
        let answer = crate::direction::RemoveServingOutcome::Removed { serving_removed: crate::direction::ServingRemoved { actor: input.actor.clone(), serving_id: input.serving_id.clone(), reason: input.reason.clone() } };
        GoalServingStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.CancelDispatch`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::CancelDispatchBehavior for Generated<P>
where
    P: DispatchStorage,
{
    fn cancel_dispatch(&mut self, input: crate::dispatch::CancelDispatch) -> Result<crate::dispatch::CancelDispatchOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DispatchStorage::get(&self.ports, &input.dispatch_id).is_none() {
            return Ok(crate::dispatch::CancelDispatchOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        }
        // `cancelled`: the default.
        let Some(held) = DispatchStorage::get(&self.ports, &input.dispatch_id) else {
            return Ok(crate::dispatch::CancelDispatchOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyDispatch::Blocked(instance) => crate::dispatch::AnyDispatch::Cancelled(instance.cancel()),
            crate::dispatch::AnyDispatch::Sent(instance) => crate::dispatch::AnyDispatch::Cancelled(instance.cancel()),
            crate::dispatch::AnyDispatch::Started(instance) => crate::dispatch::AnyDispatch::Cancelled(instance.cancel()),
            _ => return Ok(crate::dispatch::CancelDispatchOutcome::WrongState { error: crate::dispatch::DispatchStateConflict { dispatch_id: input.dispatch_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::CancelDispatchOutcome::Cancelled { dispatch_cancelled: crate::dispatch::DispatchCancelled { actor: input.actor.clone(), dispatch_id: input.dispatch_id.clone(), reason: input.reason.clone() } };
        DispatchStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.GrantResource`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::GrantResourceBehavior for Generated<P>
where
    P: ResourceRequestStorage,
{
    fn grant_resource(&mut self, input: crate::dispatch::GrantResource) -> Result<crate::dispatch::GrantResourceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if ResourceRequestStorage::get(&self.ports, &input.resource_request_id).is_none() {
            return Ok(crate::dispatch::GrantResourceOutcome::NoSuchRequest { error: crate::dispatch::ResourceRequestNotFound { resource_request_id: input.resource_request_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = ResourceRequestStorage::get(&self.ports, &input.resource_request_id) else {
            return Ok(crate::dispatch::GrantResourceOutcome::NoSuchRequest { error: crate::dispatch::ResourceRequestNotFound { resource_request_id: input.resource_request_id.clone() } });
        };
        let _ = &held;
        // `below-disk-floor`: selected by the addressed row.
        if decided(all(&[all(&[equal(Some(&held.data.resource).map(|value| match value { crate::dispatch::ResourceKind::BuildSlot => "BuildSlot", crate::dispatch::ResourceKind::Disk => "Disk", crate::dispatch::ResourceKind::Usage => "Usage" }.to_owned()), Some("BuildSlot".to_owned())), equal(Some(&held.state).map(|value| match value { crate::dispatch::ResourceRequestState::Granted => "Granted", crate::dispatch::ResourceRequestState::Refused => "Refused", crate::dispatch::ResourceRequestState::Released => "Released", crate::dispatch::ResourceRequestState::Requested => "Requested" }.to_owned()), Some("Requested".to_owned()))]), compare_numbers(Some(&input.disk_free_bytes).map(|value| value.to_string()), Some("32212254720".to_owned()), core::cmp::Ordering::is_lt)]), "conductor.dispatch.GrantResource")? {
            return Ok(crate::dispatch::GrantResourceOutcome::BelowDiskFloor { error: crate::dispatch::DiskBelowFloor { resource_request_id: input.resource_request_id.clone(), disk_free_bytes: input.disk_free_bytes.clone() } });
        }
        // `granted`: the default.
        let Some(held) = ResourceRequestStorage::get(&self.ports, &input.resource_request_id) else {
            return Ok(crate::dispatch::GrantResourceOutcome::NoSuchRequest { error: crate::dispatch::ResourceRequestNotFound { resource_request_id: input.resource_request_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyResourceRequest::Requested(instance) => crate::dispatch::AnyResourceRequest::Granted(instance.grant()),
            _ => return Ok(crate::dispatch::GrantResourceOutcome::WrongState { error: crate::dispatch::ResourceRequestStateConflict { resource_request_id: input.resource_request_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::GrantResourceOutcome::Granted { resource_granted: crate::dispatch::ResourceGranted { actor: input.actor.clone(), resource_request_id: input.resource_request_id.clone() } };
        ResourceRequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.HandleMessage`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::HandleMessageBehavior for Generated<P>
where
    P: TryContext + MessageStorage,
{
    fn handle_message(&mut self, input: crate::dispatch::HandleMessage) -> Result<crate::dispatch::HandleMessageOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.message_id).is_none() {
            return Ok(crate::dispatch::HandleMessageOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::dispatch::HandleMessageOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        };
        let _ = &held;
        // `need-unrouted`: selected by the addressed row.
        if decided(all(&[equal(Some(&held.data.kind).map(|value| match value { crate::dispatch::MessageKind::Dispatch => "Dispatch", crate::dispatch::MessageKind::Report => "Report", crate::dispatch::MessageKind::DecisionRequest => "DecisionRequest", crate::dispatch::MessageKind::Decision => "Decision", crate::dispatch::MessageKind::Need => "Need", crate::dispatch::MessageKind::Resource => "Resource", crate::dispatch::MessageKind::Escalation => "Escalation", crate::dispatch::MessageKind::Team => "Team", crate::dispatch::MessageKind::Handover => "Handover", crate::dispatch::MessageKind::Unknown => "Unknown" }.to_owned()), Some("Need".to_owned())), equal(Some(&held.state).map(|value| match value { crate::dispatch::MessageState::Handled => "Handled", crate::dispatch::MessageState::Received => "Received", crate::dispatch::MessageState::Rejected => "Rejected" }.to_owned()), Some("Received".to_owned()))]), "conductor.dispatch.HandleMessage")? {
            return Ok(crate::dispatch::HandleMessageOutcome::NeedUnrouted { error: crate::dispatch::NeedNotRouted { message_id: input.message_id.clone() } });
        }
        // `dispatch-not-of-message`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorDispatchHandleMessage(&input), "dispatch-not-of-message")? {
            return Ok(crate::dispatch::HandleMessageOutcome::DispatchNotOfMessage { error: crate::dispatch::MessageDispatchMismatch { message_id: input.message_id.clone(), dispatch_id: input.dispatch_id.clone() } });
        }
        // `handled`: the default.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::dispatch::HandleMessageOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyMessage::Received(instance) => crate::dispatch::AnyMessage::Handled(instance.handle()),
            _ => return Ok(crate::dispatch::HandleMessageOutcome::WrongState { error: crate::dispatch::MessageStateConflict { message_id: input.message_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.dispatch_id = input.dispatch_id.clone();
        let answer = crate::dispatch::HandleMessageOutcome::Handled { message_handled: crate::dispatch::MessageHandled { actor: input.actor.clone(), message_id: input.message_id.clone() } };
        MessageStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.PauseController`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::PauseControllerBehavior for Generated<P>
where
    P: ControllerStorage,
{
    fn pause_controller(&mut self, input: crate::dispatch::PauseController) -> Result<crate::dispatch::PauseControllerOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if ControllerStorage::get(&self.ports, &input.controller_id).is_none() {
            return Ok(crate::dispatch::PauseControllerOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        }
        // `paused`: the default.
        let Some(held) = ControllerStorage::get(&self.ports, &input.controller_id) else {
            return Ok(crate::dispatch::PauseControllerOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyController::Running(instance) => crate::dispatch::AnyController::Paused(instance.pause()),
            _ => return Ok(crate::dispatch::PauseControllerOutcome::WrongState { error: crate::dispatch::ControllerStateConflict { controller_id: input.controller_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::PauseControllerOutcome::Paused { controller_paused: crate::dispatch::ControllerPaused { actor: input.actor.clone(), controller_id: input.controller_id.clone() } };
        ControllerStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReceiveMessage`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReceiveMessageBehavior for Generated<P>
where
    P: TryContext + MessageStorage,
{
    fn receive_message(&mut self, input: crate::dispatch::ReceiveMessage) -> Result<crate::dispatch::ReceiveMessageOutcome, UnmetObligation> {
        let _ = &input;
        // `mailbox-without-body`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(all(&[equal(Some(&input.transport).map(|value| match value { crate::dispatch::Transport::SendMessage => "SendMessage", crate::dispatch::Transport::Mailbox => "Mailbox" }.to_owned()), Some("Mailbox".to_owned())), (Some(Some(&input.body_path).and_then(|value| value.as_ref()).is_some())).map(|value| !value)]), "conductor.dispatch.ReceiveMessage")? {
            return Ok(crate::dispatch::ReceiveMessageOutcome::MailboxWithoutBody { error: crate::dispatch::MailboxBodyMissing { sender: input.sender.clone(), recipient: input.recipient.clone() } });
        }
        // `unparsed`: an accepting branch, in declaration order.
        if decided(any(&[equal(Some(&input.parsed).map(|value| *value), Some(false)), equal(Some(&input.kind).map(|value| match value { crate::dispatch::MessageKind::Dispatch => "Dispatch", crate::dispatch::MessageKind::Report => "Report", crate::dispatch::MessageKind::DecisionRequest => "DecisionRequest", crate::dispatch::MessageKind::Decision => "Decision", crate::dispatch::MessageKind::Need => "Need", crate::dispatch::MessageKind::Resource => "Resource", crate::dispatch::MessageKind::Escalation => "Escalation", crate::dispatch::MessageKind::Team => "Team", crate::dispatch::MessageKind::Handover => "Handover", crate::dispatch::MessageKind::Unknown => "Unknown" }.to_owned()), Some("Unknown".to_owned()))]), "conductor.dispatch.ReceiveMessage")? {
            let identity: crate::dispatch::MessageId = self.ports.try_generate_conductor_dispatch_message_id()?;
            let data = crate::dispatch::MessageData {
                message_id: identity.clone(),
                sender: input.sender.clone(),
                actor: input.actor.clone(),
                recipient: input.recipient.clone(),
                kind: input.kind.clone(),
                first_line: input.first_line.clone(),
                transport: input.transport.clone(),
                received_at: input.received_at.clone(),
                body_path: input.body_path.clone(),
                dispatch_id: None,
            };
            let answer = crate::dispatch::ReceiveMessageOutcome::Unparsed { message_received: crate::dispatch::MessageReceived { actor: input.actor.clone(), message_id: identity.clone(), sender: input.sender.clone(), kind: input.kind.clone() } };
            MessageStorage::put(&mut self.ports, crate::dispatch::AnyMessage::Rejected(crate::dispatch::Message::new_rejected(data)).snapshot());
            return Ok(answer);
        }
        // `received`: the default.
        let identity: crate::dispatch::MessageId = self.ports.try_generate_conductor_dispatch_message_id()?;
        let data = crate::dispatch::MessageData {
            message_id: identity.clone(),
            sender: input.sender.clone(),
            actor: input.actor.clone(),
            recipient: input.recipient.clone(),
            kind: input.kind.clone(),
            first_line: input.first_line.clone(),
            transport: input.transport.clone(),
            received_at: input.received_at.clone(),
            body_path: input.body_path.clone(),
            dispatch_id: None,
        };
        let answer = crate::dispatch::ReceiveMessageOutcome::Received { message_received: crate::dispatch::MessageReceived { actor: input.actor.clone(), message_id: identity.clone(), sender: input.sender.clone(), kind: input.kind.clone() } };
        MessageStorage::put(&mut self.ports, crate::dispatch::AnyMessage::Received(crate::dispatch::Message::new(data)).snapshot());
        return Ok(answer);
    }
}

impl<P: crate::dispatch::obligations::RecordDispatchLinkBehavior> crate::dispatch::obligations::RecordDispatchLinkBehavior for Generated<P> {
    fn record_dispatch_link(&mut self, input: crate::dispatch::RecordDispatchLink) -> Result<crate::dispatch::RecordDispatchLinkOutcome, UnmetObligation> {
        crate::dispatch::obligations::RecordDispatchLinkBehavior::record_dispatch_link(&mut self.ports, input)
    }
}

/// `conductor.dispatch.RecordGuardDecision`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::RecordGuardDecisionBehavior for Generated<P>
where
    P: TryContext + GuardDecisionStorage,
{
    fn record_guard_decision(&mut self, input: crate::dispatch::RecordGuardDecision) -> Result<crate::dispatch::RecordGuardDecisionOutcome, UnmetObligation> {
        let _ = &input;
        // `recorded`: the default.
        let identity: crate::dispatch::GuardDecisionId = self.ports.try_generate_conductor_dispatch_guard_decision_id()?;
        let data = crate::dispatch::GuardDecisionData {
            guard_decision_id: identity.clone(),
            actor: input.actor.clone(),
            repository: input.repository.clone(),
            session_ref: input.session_ref.clone(),
            tool: input.tool.clone(),
            target: input.target.clone(),
            verdict: input.verdict.clone(),
            reason: input.reason.clone(),
            decided_at: input.decided_at.clone(),
        };
        let answer = crate::dispatch::RecordGuardDecisionOutcome::Recorded { guard_decision_recorded: crate::dispatch::GuardDecisionRecorded { actor: input.actor.clone(), guard_decision_id: identity.clone(), repository: input.repository.clone(), tool: input.tool.clone(), verdict: input.verdict.clone() } };
        GuardDecisionStorage::put(&mut self.ports, crate::dispatch::AnyGuardDecision::Recorded(crate::dispatch::GuardDecision::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.dispatch.RefuseResource`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::RefuseResourceBehavior for Generated<P>
where
    P: ResourceRequestStorage,
{
    fn refuse_resource(&mut self, input: crate::dispatch::RefuseResource) -> Result<crate::dispatch::RefuseResourceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if ResourceRequestStorage::get(&self.ports, &input.resource_request_id).is_none() {
            return Ok(crate::dispatch::RefuseResourceOutcome::NoSuchRequest { error: crate::dispatch::ResourceRequestNotFound { resource_request_id: input.resource_request_id.clone() } });
        }
        // `refused`: the default.
        let Some(held) = ResourceRequestStorage::get(&self.ports, &input.resource_request_id) else {
            return Ok(crate::dispatch::RefuseResourceOutcome::NoSuchRequest { error: crate::dispatch::ResourceRequestNotFound { resource_request_id: input.resource_request_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyResourceRequest::Requested(instance) => crate::dispatch::AnyResourceRequest::Refused(instance.refuse()),
            _ => return Ok(crate::dispatch::RefuseResourceOutcome::WrongState { error: crate::dispatch::ResourceRequestStateConflict { resource_request_id: input.resource_request_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::RefuseResourceOutcome::Refused { resource_refused: crate::dispatch::ResourceRefused { actor: input.actor.clone(), resource_request_id: input.resource_request_id.clone(), reason: input.reason.clone() } };
        ResourceRequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.RejectMessage`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::RejectMessageBehavior for Generated<P>
where
    P: MessageStorage,
{
    fn reject_message(&mut self, input: crate::dispatch::RejectMessage) -> Result<crate::dispatch::RejectMessageOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.message_id).is_none() {
            return Ok(crate::dispatch::RejectMessageOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        }
        // `rejected`: the default.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::dispatch::RejectMessageOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyMessage::Received(instance) => crate::dispatch::AnyMessage::Rejected(instance.reject()),
            _ => return Ok(crate::dispatch::RejectMessageOutcome::WrongState { error: crate::dispatch::MessageStateConflict { message_id: input.message_id.clone() } }),
        };
        let next = moved.snapshot();
        let answer = crate::dispatch::RejectMessageOutcome::Rejected { message_rejected: crate::dispatch::MessageRejected { actor: input.actor.clone(), message_id: input.message_id.clone(), reason: input.reason.clone() } };
        MessageStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReleaseResource`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReleaseResourceBehavior for Generated<P>
where
    P: ResourceRequestStorage,
{
    fn release_resource(&mut self, input: crate::dispatch::ReleaseResource) -> Result<crate::dispatch::ReleaseResourceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if ResourceRequestStorage::get(&self.ports, &input.resource_request_id).is_none() {
            return Ok(crate::dispatch::ReleaseResourceOutcome::NoSuchRequest { error: crate::dispatch::ResourceRequestNotFound { resource_request_id: input.resource_request_id.clone() } });
        }
        // `released`: the default.
        let Some(held) = ResourceRequestStorage::get(&self.ports, &input.resource_request_id) else {
            return Ok(crate::dispatch::ReleaseResourceOutcome::NoSuchRequest { error: crate::dispatch::ResourceRequestNotFound { resource_request_id: input.resource_request_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyResourceRequest::Granted(instance) => crate::dispatch::AnyResourceRequest::Released(instance.release()),
            _ => return Ok(crate::dispatch::ReleaseResourceOutcome::WrongState { error: crate::dispatch::ResourceRequestStateConflict { resource_request_id: input.resource_request_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::ReleaseResourceOutcome::Released { resource_released: crate::dispatch::ResourceReleased { actor: input.actor.clone(), resource_request_id: input.resource_request_id.clone() } };
        ResourceRequestStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReportBlocked`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReportBlockedBehavior for Generated<P>
where
    P: DispatchStorage,
{
    fn report_blocked(&mut self, input: crate::dispatch::ReportBlocked) -> Result<crate::dispatch::ReportBlockedOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DispatchStorage::get(&self.ports, &input.dispatch_id).is_none() {
            return Ok(crate::dispatch::ReportBlockedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        }
        // `blocked`: the default.
        let Some(held) = DispatchStorage::get(&self.ports, &input.dispatch_id) else {
            return Ok(crate::dispatch::ReportBlockedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyDispatch::Started(instance) => crate::dispatch::AnyDispatch::Blocked(instance.block()),
            _ => return Ok(crate::dispatch::ReportBlockedOutcome::WrongState { error: crate::dispatch::DispatchStateConflict { dispatch_id: input.dispatch_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::ReportBlockedOutcome::Blocked { dispatch_blocked: crate::dispatch::DispatchBlocked { actor: input.actor.clone(), dispatch_id: input.dispatch_id.clone(), reason: input.reason.clone() } };
        DispatchStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReportDone`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReportDoneBehavior for Generated<P>
where
    P: DispatchStorage,
{
    fn report_done(&mut self, input: crate::dispatch::ReportDone) -> Result<crate::dispatch::ReportDoneOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DispatchStorage::get(&self.ports, &input.dispatch_id).is_none() {
            return Ok(crate::dispatch::ReportDoneOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        }
        // `done`: the default.
        let Some(held) = DispatchStorage::get(&self.ports, &input.dispatch_id) else {
            return Ok(crate::dispatch::ReportDoneOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyDispatch::Blocked(instance) => crate::dispatch::AnyDispatch::Done(instance.finish()),
            crate::dispatch::AnyDispatch::Sent(instance) => crate::dispatch::AnyDispatch::Done(instance.finish()),
            crate::dispatch::AnyDispatch::Started(instance) => crate::dispatch::AnyDispatch::Done(instance.finish()),
            _ => return Ok(crate::dispatch::ReportDoneOutcome::WrongState { error: crate::dispatch::DispatchStateConflict { dispatch_id: input.dispatch_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::ReportDoneOutcome::Done { dispatch_done: crate::dispatch::DispatchDone { actor: input.actor.clone(), dispatch_id: input.dispatch_id.clone(), evidence: input.evidence.clone() } };
        DispatchStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReportFailed`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReportFailedBehavior for Generated<P>
where
    P: DispatchStorage,
{
    fn report_failed(&mut self, input: crate::dispatch::ReportFailed) -> Result<crate::dispatch::ReportFailedOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DispatchStorage::get(&self.ports, &input.dispatch_id).is_none() {
            return Ok(crate::dispatch::ReportFailedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        }
        // `failed`: the default.
        let Some(held) = DispatchStorage::get(&self.ports, &input.dispatch_id) else {
            return Ok(crate::dispatch::ReportFailedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyDispatch::Blocked(instance) => crate::dispatch::AnyDispatch::Failed(instance.fail()),
            crate::dispatch::AnyDispatch::Started(instance) => crate::dispatch::AnyDispatch::Failed(instance.fail()),
            _ => return Ok(crate::dispatch::ReportFailedOutcome::WrongState { error: crate::dispatch::DispatchStateConflict { dispatch_id: input.dispatch_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::ReportFailedOutcome::Failed { dispatch_failed: crate::dispatch::DispatchFailed { actor: input.actor.clone(), dispatch_id: input.dispatch_id.clone(), reason: input.reason.clone() } };
        DispatchStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReportStarted`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReportStartedBehavior for Generated<P>
where
    P: DispatchStorage,
{
    fn report_started(&mut self, input: crate::dispatch::ReportStarted) -> Result<crate::dispatch::ReportStartedOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DispatchStorage::get(&self.ports, &input.dispatch_id).is_none() {
            return Ok(crate::dispatch::ReportStartedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        }
        // `started`: the default.
        let Some(held) = DispatchStorage::get(&self.ports, &input.dispatch_id) else {
            return Ok(crate::dispatch::ReportStartedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyDispatch::Sent(instance) => crate::dispatch::AnyDispatch::Started(instance.start()),
            _ => return Ok(crate::dispatch::ReportStartedOutcome::WrongState { error: crate::dispatch::DispatchStateConflict { dispatch_id: input.dispatch_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::ReportStartedOutcome::Started { dispatch_started: crate::dispatch::DispatchStarted { actor: input.actor.clone(), dispatch_id: input.dispatch_id.clone() } };
        DispatchStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReportUnblocked`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReportUnblockedBehavior for Generated<P>
where
    P: DispatchStorage,
{
    fn report_unblocked(&mut self, input: crate::dispatch::ReportUnblocked) -> Result<crate::dispatch::ReportUnblockedOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DispatchStorage::get(&self.ports, &input.dispatch_id).is_none() {
            return Ok(crate::dispatch::ReportUnblockedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        }
        // `unblocked`: the default.
        let Some(held) = DispatchStorage::get(&self.ports, &input.dispatch_id) else {
            return Ok(crate::dispatch::ReportUnblockedOutcome::NoSuchDispatch { error: crate::dispatch::DispatchNotFound { dispatch_id: input.dispatch_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyDispatch::Blocked(instance) => crate::dispatch::AnyDispatch::Started(instance.unblock()),
            _ => return Ok(crate::dispatch::ReportUnblockedOutcome::WrongState { error: crate::dispatch::DispatchStateConflict { dispatch_id: input.dispatch_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::ReportUnblockedOutcome::Unblocked { dispatch_unblocked: crate::dispatch::DispatchUnblocked { actor: input.actor.clone(), dispatch_id: input.dispatch_id.clone() } };
        DispatchStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.RequestResource`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::RequestResourceBehavior for Generated<P>
where
    P: TryContext + ResourceRequestStorage,
{
    fn request_resource(&mut self, input: crate::dispatch::RequestResource) -> Result<crate::dispatch::RequestResourceOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-amount`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.amount).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_le), "conductor.dispatch.RequestResource")? {
            return Ok(crate::dispatch::RequestResourceOutcome::InvalidAmount { error: crate::dispatch::InvalidAmount { amount: input.amount.clone() } });
        }
        // `requested`: the default.
        let identity: crate::dispatch::ResourceRequestId = self.ports.try_generate_conductor_dispatch_resource_request_id()?;
        let data = crate::dispatch::ResourceRequestData {
            resource_request_id: identity.clone(),
            actor: input.actor.clone(),
            repository: input.repository.clone(),
            resource: input.resource.clone(),
            amount: input.amount.clone(),
            requested_at: input.requested_at.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::RequestResourceOutcome::Requested { resource_requested: crate::dispatch::ResourceRequested { actor: input.actor.clone(), resource_request_id: identity.clone(), repository: input.repository.clone(), resource: input.resource.clone() } };
        ResourceRequestStorage::put(&mut self.ports, crate::dispatch::AnyResourceRequest::Requested(crate::dispatch::ResourceRequest::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.dispatch.ResumeController`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ResumeControllerBehavior for Generated<P>
where
    P: ControllerStorage,
{
    fn resume_controller(&mut self, input: crate::dispatch::ResumeController) -> Result<crate::dispatch::ResumeControllerOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if ControllerStorage::get(&self.ports, &input.controller_id).is_none() {
            return Ok(crate::dispatch::ResumeControllerOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        }
        // `resumed`: the default.
        let Some(held) = ControllerStorage::get(&self.ports, &input.controller_id) else {
            return Ok(crate::dispatch::ResumeControllerOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyController::Paused(instance) => crate::dispatch::AnyController::Running(instance.resume()),
            _ => return Ok(crate::dispatch::ResumeControllerOutcome::WrongState { error: crate::dispatch::ControllerStateConflict { controller_id: input.controller_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::ResumeControllerOutcome::Resumed { controller_resumed: crate::dispatch::ControllerResumed { actor: input.actor.clone(), controller_id: input.controller_id.clone() } };
        ControllerStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.dispatch.ReviseCharter`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::ReviseCharterBehavior for Generated<P>
where
    P: ControllerStorage,
{
    fn revise_charter(&mut self, input: crate::dispatch::ReviseCharter) -> Result<crate::dispatch::ReviseCharterOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if ControllerStorage::get(&self.ports, &input.controller_id).is_none() {
            return Ok(crate::dispatch::ReviseCharterOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = ControllerStorage::get(&self.ports, &input.controller_id) else {
            return Ok(crate::dispatch::ReviseCharterOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        };
        let _ = &held;
        // `revised`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::dispatch::ControllerState::Running)), "conductor.dispatch.ReviseCharter")? {
            let Some(held) = ControllerStorage::get(&self.ports, &input.controller_id) else {
                return Ok(crate::dispatch::ReviseCharterOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
            };
            let _ = &held;
            let before = held.data.clone();
            let mut next = held;
            next.data.charter_revision = before.charter_revision + 1;
            if let Some(broken) = next.data.broken_invariant() {
                let capability = "entity invariant";
                return Err(UnmetObligation { capability, source: broken });
            }
            let answer = crate::dispatch::ReviseCharterOutcome::Revised { charter_revised: crate::dispatch::CharterRevised { actor: input.actor.clone(), controller_id: input.controller_id.clone() } };
            ControllerStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `paused-revised`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::dispatch::ControllerState::Paused)), "conductor.dispatch.ReviseCharter")? {
            let Some(held) = ControllerStorage::get(&self.ports, &input.controller_id) else {
                return Ok(crate::dispatch::ReviseCharterOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
            };
            let _ = &held;
            let before = held.data.clone();
            let mut next = held;
            next.data.charter_revision = before.charter_revision + 1;
            if let Some(broken) = next.data.broken_invariant() {
                let capability = "entity invariant";
                return Err(UnmetObligation { capability, source: broken });
            }
            let answer = crate::dispatch::ReviseCharterOutcome::PausedRevised { charter_revised: crate::dispatch::CharterRevised { actor: input.actor.clone(), controller_id: input.controller_id.clone() } };
            ControllerStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `stopped`: the default.
        return Ok(crate::dispatch::ReviseCharterOutcome::Stopped { error: crate::dispatch::ControllerStateConflict { controller_id: input.controller_id.clone() } });
    }
}

/// `conductor.dispatch.RouteNeed`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::RouteNeedBehavior for Generated<P>
where
    P: MessageStorage,
{
    fn route_need(&mut self, input: crate::dispatch::RouteNeed) -> Result<crate::dispatch::RouteNeedOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MessageStorage::get(&self.ports, &input.message_id).is_none() {
            return Ok(crate::dispatch::RouteNeedOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::dispatch::RouteNeedOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        };
        let _ = &held;
        // `not-a-need`: selected by the addressed row.
        if decided(all(&[equal(Some(&held.data.kind).map(|value| match value { crate::dispatch::MessageKind::Dispatch => "Dispatch", crate::dispatch::MessageKind::Report => "Report", crate::dispatch::MessageKind::DecisionRequest => "DecisionRequest", crate::dispatch::MessageKind::Decision => "Decision", crate::dispatch::MessageKind::Need => "Need", crate::dispatch::MessageKind::Resource => "Resource", crate::dispatch::MessageKind::Escalation => "Escalation", crate::dispatch::MessageKind::Team => "Team", crate::dispatch::MessageKind::Handover => "Handover", crate::dispatch::MessageKind::Unknown => "Unknown" }.to_owned()), Some("Need".to_owned())).map(|value| !value), equal(Some(&held.state).map(|value| match value { crate::dispatch::MessageState::Handled => "Handled", crate::dispatch::MessageState::Received => "Received", crate::dispatch::MessageState::Rejected => "Rejected" }.to_owned()), Some("Received".to_owned()))]), "conductor.dispatch.RouteNeed")? {
            return Ok(crate::dispatch::RouteNeedOutcome::NotANeed { error: crate::dispatch::NotANeed { message_id: input.message_id.clone() } });
        }
        // `routed`: the default.
        let Some(held) = MessageStorage::get(&self.ports, &input.message_id) else {
            return Ok(crate::dispatch::RouteNeedOutcome::NoSuchMessage { error: crate::dispatch::MessageNotFound { message_id: input.message_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyMessage::Received(instance) => crate::dispatch::AnyMessage::Handled(instance.handle()),
            _ => return Ok(crate::dispatch::RouteNeedOutcome::WrongState { error: crate::dispatch::MessageStateConflict { message_id: input.message_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.dispatch_id = Some(input.dispatch_id.clone());
        let answer = crate::dispatch::RouteNeedOutcome::Routed { need_routed: crate::dispatch::NeedRouted { actor: input.actor.clone(), message_id: input.message_id.clone(), dispatch_id: input.dispatch_id.clone() } };
        MessageStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::dispatch::obligations::SendDispatchBehavior> crate::dispatch::obligations::SendDispatchBehavior for Generated<P> {
    fn send_dispatch(&mut self, input: crate::dispatch::SendDispatch) -> Result<crate::dispatch::SendDispatchOutcome, UnmetObligation> {
        crate::dispatch::obligations::SendDispatchBehavior::send_dispatch(&mut self.ports, input)
    }
}

impl<P: crate::dispatch::obligations::StartControllerBehavior> crate::dispatch::obligations::StartControllerBehavior for Generated<P> {
    fn start_controller(&mut self, input: crate::dispatch::StartController) -> Result<crate::dispatch::StartControllerOutcome, UnmetObligation> {
        crate::dispatch::obligations::StartControllerBehavior::start_controller(&mut self.ports, input)
    }
}

/// `conductor.dispatch.StopController`, generated: every outcome is one the specification fully determines.
impl<P> crate::dispatch::obligations::StopControllerBehavior for Generated<P>
where
    P: ControllerStorage,
{
    fn stop_controller(&mut self, input: crate::dispatch::StopController) -> Result<crate::dispatch::StopControllerOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if ControllerStorage::get(&self.ports, &input.controller_id).is_none() {
            return Ok(crate::dispatch::StopControllerOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        }
        // `stopped`: the default.
        let Some(held) = ControllerStorage::get(&self.ports, &input.controller_id) else {
            return Ok(crate::dispatch::StopControllerOutcome::NoSuchController { error: crate::dispatch::ControllerNotFound { controller_id: input.controller_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::dispatch::AnyController::Paused(instance) => crate::dispatch::AnyController::Stopped(instance.stop()),
            crate::dispatch::AnyController::Running(instance) => crate::dispatch::AnyController::Stopped(instance.stop()),
            _ => return Ok(crate::dispatch::StopControllerOutcome::WrongState { error: crate::dispatch::ControllerStateConflict { controller_id: input.controller_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::dispatch::StopControllerOutcome::Stopped { controller_stopped: crate::dispatch::ControllerStopped { actor: input.actor.clone(), controller_id: input.controller_id.clone(), reason: input.reason.clone() } };
        ControllerStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.host.Reclaim`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::ReclaimBehavior for Generated<P>
where
    P: TryContext + ReclaimRunStorage,
{
    fn reclaim(&mut self, input: crate::host::Reclaim) -> Result<crate::host::ReclaimOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-size`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[compare_numbers(Some(&input.reclaimed_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), compare_numbers(Some(&input.shortfall_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt)]), "conductor.host.Reclaim")? {
            return Ok(crate::host::ReclaimOutcome::InvalidSize { error: crate::host::InvalidSize { bytes: input.reclaimed_bytes.clone() } });
        }
        // `floor-not-reached`: an accepting branch, in declaration order.
        if decided(compare_numbers(Some(&input.shortfall_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_gt), "conductor.host.Reclaim")? {
            let identity: crate::host::ReclaimRunId = self.ports.try_generate_conductor_host_reclaim_run_id()?;
            let data = crate::host::ReclaimRunData {
                reclaim_run_id: identity.clone(),
                instance: input.instance.clone(),
                result: crate::host::ReclaimResult::FloorNotReached,
                steps: input.steps.clone(),
                reclaimed_bytes: input.reclaimed_bytes.clone(),
                shortfall_bytes: Some(input.shortfall_bytes.clone()),
                left: input.left.clone(),
                actor: input.actor.clone(),
                ran_at: input.ran_at.clone(),
            };
            if let Some(broken) = data.broken_invariant() {
                let capability = "entity invariant";
                return Err(UnmetObligation { capability, source: broken });
            }
            let answer = crate::host::ReclaimOutcome::FloorNotReached { reclaim_floor_not_reached: crate::host::ReclaimFloorNotReached { actor: input.actor.clone(), reclaim_run_id: identity.clone(), reclaimed_bytes: input.reclaimed_bytes.clone(), shortfall_bytes: input.shortfall_bytes.clone() } };
            ReclaimRunStorage::put(&mut self.ports, crate::host::AnyReclaimRun::Recorded(crate::host::ReclaimRun::new(data)).snapshot());
            return Ok(answer);
        }
        // `reclaimed`: the default.
        let identity: crate::host::ReclaimRunId = self.ports.try_generate_conductor_host_reclaim_run_id()?;
        let data = crate::host::ReclaimRunData {
            reclaim_run_id: identity.clone(),
            instance: input.instance.clone(),
            result: crate::host::ReclaimResult::Reclaimed,
            steps: input.steps.clone(),
            reclaimed_bytes: input.reclaimed_bytes.clone(),
            shortfall_bytes: None,
            left: input.left.clone(),
            actor: input.actor.clone(),
            ran_at: input.ran_at.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::host::ReclaimOutcome::Reclaimed { disk_reclaimed: crate::host::DiskReclaimed { actor: input.actor.clone(), reclaim_run_id: identity.clone(), reclaimed_bytes: input.reclaimed_bytes.clone() } };
        ReclaimRunStorage::put(&mut self.ports, crate::host::AnyReclaimRun::Recorded(crate::host::ReclaimRun::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.host.RecordPeak`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::RecordPeakBehavior for Generated<P>
where
    P: HostGrantStorage,
{
    fn record_peak(&mut self, input: crate::host::RecordPeak) -> Result<crate::host::RecordPeakOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-size`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.peak_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.host.RecordPeak")? {
            return Ok(crate::host::RecordPeakOutcome::InvalidSize { error: crate::host::InvalidSize { bytes: input.peak_bytes.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if HostGrantStorage::get(&self.ports, &input.grant_id).is_none() {
            return Ok(crate::host::RecordPeakOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
            return Ok(crate::host::RecordPeakOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        };
        let _ = &held;
        // `recorded`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::host::HostGrantState::Running)), "conductor.host.RecordPeak")? {
            let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
                return Ok(crate::host::RecordPeakOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
            };
            let _ = &held;
            let mut next = held;
            next.data.peak_bytes = Some(input.peak_bytes.clone());
            next.data.pauses = input.pauses.clone();
            let answer = crate::host::RecordPeakOutcome::Recorded { peak_recorded: crate::host::PeakRecorded { actor: input.actor.clone(), grant_id: input.grant_id.clone(), peak_bytes: input.peak_bytes.clone() } };
            HostGrantStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `not-running`: the default.
        return Ok(crate::host::RecordPeakOutcome::NotRunning { error: crate::host::HostGrantStateConflict { grant_id: input.grant_id.clone() } });
    }
}

/// `conductor.host.ReleaseGrant`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::ReleaseGrantBehavior for Generated<P>
where
    P: HostGrantStorage,
{
    fn release_grant(&mut self, input: crate::host::ReleaseGrant) -> Result<crate::host::ReleaseGrantOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if HostGrantStorage::get(&self.ports, &input.grant_id).is_none() {
            return Ok(crate::host::ReleaseGrantOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        }
        // `released`: the default.
        let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
            return Ok(crate::host::ReleaseGrantOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::host::AnyHostGrant::Granted(instance) => crate::host::AnyHostGrant::Released(instance.release()),
            crate::host::AnyHostGrant::Running(instance) => crate::host::AnyHostGrant::Released(instance.release()),
            _ => return Ok(crate::host::ReleaseGrantOutcome::WrongState { error: crate::host::HostGrantStateConflict { grant_id: input.grant_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.released_at = Some(input.released_at.clone());
        next.data.release_reason = input.reason.clone();
        next.data.cache_hits = input.cache_hits.clone();
        next.data.cache_misses = input.cache_misses.clone();
        next.data.lane_overrun = input.lane_overrun.clone();
        let answer = crate::host::ReleaseGrantOutcome::Released { host_grant_released: crate::host::HostGrantReleased { actor: input.actor.clone(), grant_id: input.grant_id.clone(), reason: input.reason.clone() } };
        HostGrantStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.host.RequestDisk`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::RequestDiskBehavior for Generated<P>
where
    P: TryContext + HostGrantStorage,
{
    fn request_disk(&mut self, input: crate::host::RequestDisk) -> Result<crate::host::RequestDiskOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-size`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_le), "conductor.host.RequestDisk")? {
            return Ok(crate::host::RequestDiskOutcome::InvalidSize { error: crate::host::InvalidSize { bytes: input.bytes.clone() } });
        }
        // `requested`: the default.
        let identity: crate::host::GrantId = self.ports.try_generate_conductor_host_grant_id()?;
        let data = crate::host::HostGrantData {
            grant_id: identity.clone(),
            instance: input.instance.clone(),
            repository: input.repository.clone(),
            kind: crate::host::GrantKind::Disk,
            bytes: Some(input.bytes.clone()),
            peak_bytes: None,
            pauses: None,
            command: Some(input.command.clone()),
            process_group: None,
            session_name: input.session_name.clone(),
            session_id: input.session_id.clone(),
            reclaimed_bytes: None,
            shortfall_bytes: None,
            actor: input.actor.clone(),
            requested_at: input.requested_at.clone(),
            started_at: None,
            released_at: None,
            release_reason: None,
            cache_hits: None,
            cache_misses: None,
            lane: input.lane.clone(),
            lane_overrun: None,
        };
        let answer = crate::host::RequestDiskOutcome::Requested { disk_requested: crate::host::DiskRequested { actor: input.actor.clone(), grant_id: identity.clone(), instance_name: input.instance.clone(), bytes: input.bytes.clone() } };
        HostGrantStorage::put(&mut self.ports, crate::host::AnyHostGrant::Requested(crate::host::HostGrant::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.host.ReserveDisk`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::ReserveDiskBehavior for Generated<P>
where
    P: HostGrantStorage,
{
    fn reserve_disk(&mut self, input: crate::host::ReserveDisk) -> Result<crate::host::ReserveDiskOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-size`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(any(&[compare_numbers(Some(&input.shortfall_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), compare_numbers(Some(&input.reclaimed_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt)]), "conductor.host.ReserveDisk")? {
            return Ok(crate::host::ReserveDiskOutcome::InvalidSize { error: crate::host::InvalidSize { bytes: input.shortfall_bytes.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if HostGrantStorage::get(&self.ports, &input.grant_id).is_none() {
            return Ok(crate::host::ReserveDiskOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        }
        // `refused`: an accepting branch, in declaration order.
        if decided(compare_numbers(Some(&input.shortfall_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_gt), "conductor.host.ReserveDisk")? {
            let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
                return Ok(crate::host::ReserveDiskOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
            };
            let _ = &held;
            let before = held.data.clone();
            let moved = match held.refine() {
                crate::host::AnyHostGrant::Requested(instance) => crate::host::AnyHostGrant::Refused(instance.refuse()),
                _ => return Ok(crate::host::ReserveDiskOutcome::WrongState { error: crate::host::HostGrantStateConflict { grant_id: input.grant_id.clone() } }),
            };
            let mut next = moved.snapshot();
            next.data.reclaimed_bytes = Some(input.reclaimed_bytes.clone());
            next.data.shortfall_bytes = Some(input.shortfall_bytes.clone());
            let answer = crate::host::ReserveDiskOutcome::Refused { disk_reservation_refused: crate::host::DiskReservationRefused { actor: input.actor.clone(), grant_id: input.grant_id.clone(), bytes: before.bytes.clone(), shortfall_bytes: input.shortfall_bytes.clone() } };
            HostGrantStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `granted-after-reclaim`: an accepting branch, in declaration order.
        if decided(compare_numbers(Some(&input.reclaimed_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_gt), "conductor.host.ReserveDisk")? {
            let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
                return Ok(crate::host::ReserveDiskOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
            };
            let _ = &held;
            let before = held.data.clone();
            let moved = match held.refine() {
                crate::host::AnyHostGrant::Requested(instance) => crate::host::AnyHostGrant::Granted(instance.grant()),
                _ => return Ok(crate::host::ReserveDiskOutcome::WrongState { error: crate::host::HostGrantStateConflict { grant_id: input.grant_id.clone() } }),
            };
            let mut next = moved.snapshot();
            next.data.reclaimed_bytes = Some(input.reclaimed_bytes.clone());
            next.data.lane = input.lane.clone();
            let answer = crate::host::ReserveDiskOutcome::GrantedAfterReclaim { disk_reserved: crate::host::DiskReserved { actor: input.actor.clone(), grant_id: input.grant_id.clone(), bytes: before.bytes.clone(), reclaimed_bytes: input.reclaimed_bytes.clone() } };
            HostGrantStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `granted`: the default.
        let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
            return Ok(crate::host::ReserveDiskOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        };
        let _ = &held;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::host::AnyHostGrant::Requested(instance) => crate::host::AnyHostGrant::Granted(instance.grant()),
            _ => return Ok(crate::host::ReserveDiskOutcome::WrongState { error: crate::host::HostGrantStateConflict { grant_id: input.grant_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.reclaimed_bytes = Some(0);
        next.data.lane = input.lane.clone();
        let answer = crate::host::ReserveDiskOutcome::Granted { disk_reserved: crate::host::DiskReserved { actor: input.actor.clone(), grant_id: input.grant_id.clone(), bytes: before.bytes.clone(), reclaimed_bytes: 0 } };
        HostGrantStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.host.StartRun`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::StartRunBehavior for Generated<P>
where
    P: HostGrantStorage,
{
    fn start_run(&mut self, input: crate::host::StartRun) -> Result<crate::host::StartRunOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if HostGrantStorage::get(&self.ports, &input.grant_id).is_none() {
            return Ok(crate::host::StartRunOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
            return Ok(crate::host::StartRunOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        };
        let _ = &held;
        // `not-a-disk-grant`: selected by the addressed row.
        if decided(equal(Some(&held.data.kind).map(|value| match value { crate::host::GrantKind::ControllerSlot => "ControllerSlot", crate::host::GrantKind::BuildSlot => "BuildSlot", crate::host::GrantKind::Disk => "Disk" }.to_owned()), Some("Disk".to_owned())).map(|value| !value), "conductor.host.StartRun")? {
            return Ok(crate::host::StartRunOutcome::NotADiskGrant { error: crate::host::NotADiskGrant { grant_id: input.grant_id.clone() } });
        }
        // `started`: the default.
        let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
            return Ok(crate::host::StartRunOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::host::AnyHostGrant::Granted(instance) => crate::host::AnyHostGrant::Running(instance.start()),
            _ => return Ok(crate::host::StartRunOutcome::WrongState { error: crate::host::HostGrantStateConflict { grant_id: input.grant_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.process_group = Some(input.process_group.clone());
        next.data.started_at = Some(input.started_at.clone());
        let answer = crate::host::StartRunOutcome::Started { run_started: crate::host::RunStarted { actor: input.actor.clone(), grant_id: input.grant_id.clone(), process_group: input.process_group.clone() } };
        HostGrantStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.host.TakeHostGrant`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::TakeHostGrantBehavior for Generated<P>
where
    P: TryContext + HostGrantStorage,
{
    fn take_host_grant(&mut self, input: crate::host::TakeHostGrant) -> Result<crate::host::TakeHostGrantOutcome, UnmetObligation> {
        let _ = &input;
        // `disk-through-reservation`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.kind).map(|value| match value { crate::host::GrantKind::ControllerSlot => "ControllerSlot", crate::host::GrantKind::BuildSlot => "BuildSlot", crate::host::GrantKind::Disk => "Disk" }.to_owned()), Some("Disk".to_owned())), "conductor.host.TakeHostGrant")? {
            return Ok(crate::host::TakeHostGrantOutcome::DiskThroughReservation { error: crate::host::DiskThroughReservation { kind: input.kind.clone() } });
        }
        // `granted`: the default.
        let identity: crate::host::GrantId = self.ports.try_generate_conductor_host_grant_id()?;
        let data = crate::host::HostGrantData {
            grant_id: identity.clone(),
            instance: input.instance.clone(),
            repository: Some(input.repository.clone()),
            kind: input.kind.clone(),
            bytes: input.bytes.clone(),
            peak_bytes: None,
            pauses: None,
            command: None,
            process_group: None,
            session_name: Some(input.session_name.clone()),
            session_id: None,
            reclaimed_bytes: None,
            shortfall_bytes: None,
            actor: input.actor.clone(),
            requested_at: input.requested_at.clone(),
            started_at: None,
            released_at: None,
            release_reason: None,
            cache_hits: None,
            cache_misses: None,
            lane: None,
            lane_overrun: None,
        };
        let answer = crate::host::TakeHostGrantOutcome::Granted { host_grant_taken: crate::host::HostGrantTaken { actor: input.actor.clone(), grant_id: identity.clone(), instance: input.instance.clone(), kind: input.kind.clone() } };
        HostGrantStorage::put(&mut self.ports, crate::host::AnyHostGrant::Granted(crate::host::HostGrant::new_granted(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.host.WithdrawDiskRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::host::obligations::WithdrawDiskRequestBehavior for Generated<P>
where
    P: HostGrantStorage,
{
    fn withdraw_disk_request(&mut self, input: crate::host::WithdrawDiskRequest) -> Result<crate::host::WithdrawDiskRequestOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if HostGrantStorage::get(&self.ports, &input.grant_id).is_none() {
            return Ok(crate::host::WithdrawDiskRequestOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        }
        // `withdrawn`: the default.
        let Some(held) = HostGrantStorage::get(&self.ports, &input.grant_id) else {
            return Ok(crate::host::WithdrawDiskRequestOutcome::NoSuchGrant { error: crate::host::HostGrantNotFound { grant_id: input.grant_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::host::AnyHostGrant::Requested(instance) => crate::host::AnyHostGrant::Withdrawn(instance.withdraw()),
            _ => return Ok(crate::host::WithdrawDiskRequestOutcome::WrongState { error: crate::host::HostGrantStateConflict { grant_id: input.grant_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.released_at = Some(input.withdrawn_at.clone());
        next.data.release_reason = input.reason.clone();
        let answer = crate::host::WithdrawDiskRequestOutcome::Withdrawn { disk_request_withdrawn: crate::host::DiskRequestWithdrawn { actor: input.actor.clone(), grant_id: input.grant_id.clone() } };
        HostGrantStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.improvement.ApplyProposal`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::ApplyProposalBehavior for Generated<P>
where
    P: SettingsProposalStorage,
{
    fn apply_proposal(&mut self, input: crate::improvement::ApplyProposal) -> Result<crate::improvement::ApplyProposalOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if SettingsProposalStorage::get(&self.ports, &input.proposal_id).is_none() {
            return Ok(crate::improvement::ApplyProposalOutcome::NoSuchProposal { error: crate::improvement::ProposalNotFound { proposal_id: input.proposal_id.clone() } });
        }
        // `applied`: the default.
        let Some(held) = SettingsProposalStorage::get(&self.ports, &input.proposal_id) else {
            return Ok(crate::improvement::ApplyProposalOutcome::NoSuchProposal { error: crate::improvement::ProposalNotFound { proposal_id: input.proposal_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::improvement::AnySettingsProposal::Proposed(instance) => crate::improvement::AnySettingsProposal::Applied(instance.apply()),
            _ => return Ok(crate::improvement::ApplyProposalOutcome::WrongState { error: crate::improvement::ProposalStateConflict { proposal_id: input.proposal_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.revision = Some(input.revision.clone());
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::improvement::ApplyProposalOutcome::Applied { proposal_applied: crate::improvement::ProposalApplied { actor: input.actor.clone(), proposal_id: input.proposal_id.clone(), revision: input.revision.clone() } };
        SettingsProposalStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.improvement.DefineMetric`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::DefineMetricBehavior for Generated<P>
where
    P: MetricStorage,
{
    fn define_metric(&mut self, input: crate::improvement::DefineMetric) -> Result<crate::improvement::DefineMetricOutcome, UnmetObligation> {
        let _ = &input;
        // `already-defined`: an identity a record already carries, before any branch is taken.
        if MetricStorage::get(&self.ports, &input.metric_id).is_some() {
            return Ok(crate::improvement::DefineMetricOutcome::AlreadyDefined { error: crate::improvement::MetricExists { metric_id: input.metric_id.clone() } });
        }
        // `defined`: the default.
        let identity: crate::improvement::MetricId = input.metric_id.clone();
        let data = crate::improvement::MetricData {
            metric_id: identity.clone(),
            name: input.name.clone(),
            unit: input.unit.clone(),
            instrument: input.instrument.clone(),
            definition: input.definition.clone(),
            baseline: input.baseline.clone(),
            baseline_period: input.baseline_period.clone(),
            target: None,
            direction: None,
            target_owner: None,
            target_set_at: None,
            actor: input.actor.clone(),
        };
        let answer = crate::improvement::DefineMetricOutcome::Defined { metric_defined: crate::improvement::MetricDefined { actor: input.actor.clone(), metric_id: identity.clone(), name: input.name.clone(), unit: input.unit.clone() } };
        MetricStorage::put(&mut self.ports, crate::improvement::AnyMetric::Untargeted(crate::improvement::Metric::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.improvement.FileFinding`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::FileFindingBehavior for Generated<P>
where
    P: FrictionFindingStorage,
{
    fn file_finding(&mut self, input: crate::improvement::FileFinding) -> Result<crate::improvement::FileFindingOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if FrictionFindingStorage::get(&self.ports, &input.finding_id).is_none() {
            return Ok(crate::improvement::FileFindingOutcome::NoSuchFinding { error: crate::improvement::FindingNotFound { finding_id: input.finding_id.clone() } });
        }
        // `filed`: the default.
        let Some(held) = FrictionFindingStorage::get(&self.ports, &input.finding_id) else {
            return Ok(crate::improvement::FileFindingOutcome::NoSuchFinding { error: crate::improvement::FindingNotFound { finding_id: input.finding_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::improvement::AnyFrictionFinding::Open(instance) => crate::improvement::AnyFrictionFinding::Filed(instance.file()),
            _ => return Ok(crate::improvement::FileFindingOutcome::WrongState { error: crate::improvement::FindingStateConflict { finding_id: input.finding_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.story = Some(input.story.clone());
        let answer = crate::improvement::FileFindingOutcome::Filed { finding_filed: crate::improvement::FindingFiled { actor: input.actor.clone(), finding_id: input.finding_id.clone(), story: input.story.clone() } };
        FrictionFindingStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.improvement.JudgeMetricReading`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::JudgeMetricReadingBehavior for Generated<P>
where
    P: MetricReadingStorage,
{
    fn judge_metric_reading(&mut self, input: crate::improvement::JudgeMetricReading) -> Result<crate::improvement::JudgeMetricReadingOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MetricReadingStorage::get(&self.ports, &input.reading_id).is_none() {
            return Ok(crate::improvement::JudgeMetricReadingOutcome::NoSuchReading { error: crate::improvement::MetricReadingNotFound { reading_id: input.reading_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = MetricReadingStorage::get(&self.ports, &input.reading_id) else {
            return Ok(crate::improvement::JudgeMetricReadingOutcome::NoSuchReading { error: crate::improvement::MetricReadingNotFound { reading_id: input.reading_id.clone() } });
        };
        let _ = &held;
        // `judged`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::improvement::MetricReadingState::Recorded)), "conductor.improvement.JudgeMetricReading")? {
            let Some(held) = MetricReadingStorage::get(&self.ports, &input.reading_id) else {
                return Ok(crate::improvement::JudgeMetricReadingOutcome::NoSuchReading { error: crate::improvement::MetricReadingNotFound { reading_id: input.reading_id.clone() } });
            };
            let _ = &held;
            let mut next = held;
            next.data.target = Some(input.target.clone());
            next.data.met = Some(input.met.clone());
            let answer = crate::improvement::JudgeMetricReadingOutcome::Judged { metric_reading_judged: crate::improvement::MetricReadingJudged { actor: input.actor.clone(), reading_id: input.reading_id.clone(), target: input.target.clone(), met: input.met.clone() } };
            MetricReadingStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `wrong-state`: the default.
        return Ok(crate::improvement::JudgeMetricReadingOutcome::WrongState { error: crate::improvement::MetricReadingStateConflict { reading_id: input.reading_id.clone() } });
    }
}

/// `conductor.improvement.ProposeSetting`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::ProposeSettingBehavior for Generated<P>
where
    P: MetricStorage + SettingsProposalStorage,
{
    fn propose_setting(&mut self, input: crate::improvement::ProposeSetting) -> Result<crate::improvement::ProposeSettingOutcome, UnmetObligation> {
        let _ = &input;
        // `already-proposed`: an identity a record already carries, before any branch is taken.
        if SettingsProposalStorage::get(&self.ports, &input.proposal_id).is_some() {
            return Ok(crate::improvement::ProposeSettingOutcome::AlreadyProposed { error: crate::improvement::ProposalExists { proposal_id: input.proposal_id.clone() } });
        }
        // `when_related:` reads the `conductor.improvement.Metric` row `input.metric_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.metric_id);
        let related = reference.and_then(|identity| MetricStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-metric`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::improvement::ProposeSettingOutcome::NoSuchMetric { error: crate::improvement::MetricNotFound { metric_id: input.metric_id.clone() } });
        }
        // `proposed`: the default.
        let identity: crate::improvement::ProposalId = input.proposal_id.clone();
        let data = crate::improvement::SettingsProposalData {
            proposal_id: identity.clone(),
            instance: input.instance.clone(),
            key: input.key.clone(),
            old: input.old.clone(),
            new: input.new.clone(),
            metric_id: input.metric_id.clone(),
            reading: input.reading.clone(),
            baseline: input.baseline.clone(),
            window_days: input.window_days.clone(),
            proposed_at: input.proposed_at.clone(),
            actor: input.actor.clone(),
            revision: None,
            reason: None,
            finding: None,
            verdict: None,
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::improvement::ProposeSettingOutcome::Proposed { setting_proposed: crate::improvement::SettingProposed { actor: input.actor.clone(), proposal_id: identity.clone(), instance: input.instance.clone(), key: input.key.clone(), old: input.old.clone(), new: input.new.clone(), metric_id: input.metric_id.clone() } };
        SettingsProposalStorage::put(&mut self.ports, crate::improvement::AnySettingsProposal::Proposed(crate::improvement::SettingsProposal::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.improvement.RaiseFinding`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::RaiseFindingBehavior for Generated<P>
where
    P: FrictionFindingStorage + MetricStorage,
{
    fn raise_finding(&mut self, input: crate::improvement::RaiseFinding) -> Result<crate::improvement::RaiseFindingOutcome, UnmetObligation> {
        let _ = &input;
        // `already-raised`: an identity a record already carries, before any branch is taken.
        if FrictionFindingStorage::get(&self.ports, &input.finding_id).is_some() {
            return Ok(crate::improvement::RaiseFindingOutcome::AlreadyRaised { error: crate::improvement::FindingExists { finding_id: input.finding_id.clone() } });
        }
        // `when_related:` reads the `conductor.improvement.Metric` row `input.metric_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.metric_id);
        let related = reference.and_then(|identity| MetricStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-metric`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::improvement::RaiseFindingOutcome::NoSuchMetric { error: crate::improvement::MetricNotFound { metric_id: input.metric_id.clone() } });
        }
        // `raised`: the default.
        let identity: crate::improvement::FindingId = input.finding_id.clone();
        let data = crate::improvement::FrictionFindingData {
            finding_id: identity.clone(),
            signature: input.signature.clone(),
            evidence: input.evidence.clone(),
            metric_id: input.metric_id.clone(),
            raised_at: input.raised_at.clone(),
            actor: input.actor.clone(),
            story: None,
            verdict: None,
        };
        let answer = crate::improvement::RaiseFindingOutcome::Raised { finding_raised: crate::improvement::FindingRaised { actor: input.actor.clone(), finding_id: identity.clone(), signature: input.signature.clone(), evidence: input.evidence.clone(), metric_id: input.metric_id.clone() } };
        FrictionFindingStorage::put(&mut self.ports, crate::improvement::AnyFrictionFinding::Open(crate::improvement::FrictionFinding::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.improvement.RecordMetricReading`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::RecordMetricReadingBehavior for Generated<P>
where
    P: MetricStorage + MetricReadingStorage,
{
    fn record_metric_reading(&mut self, input: crate::improvement::RecordMetricReading) -> Result<crate::improvement::RecordMetricReadingOutcome, UnmetObligation> {
        let _ = &input;
        // `already-recorded`: an identity a record already carries, before any branch is taken.
        if MetricReadingStorage::get(&self.ports, &input.reading_id).is_some() {
            return Ok(crate::improvement::RecordMetricReadingOutcome::AlreadyRecorded { error: crate::improvement::MetricReadingExists { reading_id: input.reading_id.clone() } });
        }
        // `when_related:` reads the `conductor.improvement.Metric` row `input.metric_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.metric_id);
        let related = reference.and_then(|identity| MetricStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-metric`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::improvement::RecordMetricReadingOutcome::NoSuchMetric { error: crate::improvement::MetricNotFound { metric_id: input.metric_id.clone() } });
        }
        // `recorded`: the default.
        let identity: crate::improvement::MetricReadingId = input.reading_id.clone();
        let data = crate::improvement::MetricReadingData {
            reading_id: identity.clone(),
            metric_id: input.metric_id.clone(),
            day: input.day.clone(),
            scope: input.scope.clone(),
            value: input.value.clone(),
            sources: input.sources.clone(),
            target: input.target.clone(),
            met: input.met.clone(),
            recorded_at: input.recorded_at.clone(),
            actor: input.actor.clone(),
        };
        let answer = crate::improvement::RecordMetricReadingOutcome::Recorded { metric_reading_recorded: crate::improvement::MetricReadingRecorded { actor: input.actor.clone(), reading_id: identity.clone(), metric_id: input.metric_id.clone(), day: input.day.clone(), scope: input.scope.clone(), value: input.value.clone() } };
        MetricReadingStorage::put(&mut self.ports, crate::improvement::AnyMetricReading::Recorded(crate::improvement::MetricReading::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.improvement.RefuseProposal`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::RefuseProposalBehavior for Generated<P>
where
    P: SettingsProposalStorage,
{
    fn refuse_proposal(&mut self, input: crate::improvement::RefuseProposal) -> Result<crate::improvement::RefuseProposalOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if SettingsProposalStorage::get(&self.ports, &input.proposal_id).is_none() {
            return Ok(crate::improvement::RefuseProposalOutcome::NoSuchProposal { error: crate::improvement::ProposalNotFound { proposal_id: input.proposal_id.clone() } });
        }
        // `refused`: the default.
        let Some(held) = SettingsProposalStorage::get(&self.ports, &input.proposal_id) else {
            return Ok(crate::improvement::RefuseProposalOutcome::NoSuchProposal { error: crate::improvement::ProposalNotFound { proposal_id: input.proposal_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::improvement::AnySettingsProposal::Proposed(instance) => crate::improvement::AnySettingsProposal::Refused(instance.refuse()),
            _ => return Ok(crate::improvement::RefuseProposalOutcome::WrongState { error: crate::improvement::ProposalStateConflict { proposal_id: input.proposal_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.reason = Some(input.reason.clone());
        next.data.finding = input.finding.clone();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::improvement::RefuseProposalOutcome::Refused { proposal_refused: crate::improvement::ProposalRefused { actor: input.actor.clone(), proposal_id: input.proposal_id.clone(), reason: input.reason.clone() } };
        SettingsProposalStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::improvement::obligations::SetMetricTargetBehavior> crate::improvement::obligations::SetMetricTargetBehavior for Generated<P> {
    fn set_metric_target(&mut self, input: crate::improvement::SetMetricTarget) -> Result<crate::improvement::SetMetricTargetOutcome, UnmetObligation> {
        crate::improvement::obligations::SetMetricTargetBehavior::set_metric_target(&mut self.ports, input)
    }
}

/// `conductor.improvement.SettleProposal`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::SettleProposalBehavior for Generated<P>
where
    P: SettingsProposalStorage,
{
    fn settle_proposal(&mut self, input: crate::improvement::SettleProposal) -> Result<crate::improvement::SettleProposalOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if SettingsProposalStorage::get(&self.ports, &input.proposal_id).is_none() {
            return Ok(crate::improvement::SettleProposalOutcome::NoSuchProposal { error: crate::improvement::ProposalNotFound { proposal_id: input.proposal_id.clone() } });
        }
        // `kept`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.improved).map(|value| *value), Some(true)), "conductor.improvement.SettleProposal")? {
            let Some(held) = SettingsProposalStorage::get(&self.ports, &input.proposal_id) else {
                return Ok(crate::improvement::SettleProposalOutcome::NoSuchProposal { error: crate::improvement::ProposalNotFound { proposal_id: input.proposal_id.clone() } });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::improvement::AnySettingsProposal::Applied(instance) => crate::improvement::AnySettingsProposal::Kept(instance.keep()),
                _ => return Ok(crate::improvement::SettleProposalOutcome::WrongState { error: crate::improvement::ProposalStateConflict { proposal_id: input.proposal_id.clone() } }),
            };
            let mut next = moved.snapshot();
            next.data.verdict = Some(input.verdict.clone());
            if let Some(broken) = next.data.broken_invariant() {
                let capability = "entity invariant";
                return Err(UnmetObligation { capability, source: broken });
            }
            let answer = crate::improvement::SettleProposalOutcome::Kept { proposal_kept: crate::improvement::ProposalKept { actor: input.actor.clone(), proposal_id: input.proposal_id.clone(), verdict: input.verdict.clone() } };
            SettingsProposalStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `reverted`: the default.
        let Some(held) = SettingsProposalStorage::get(&self.ports, &input.proposal_id) else {
            return Ok(crate::improvement::SettleProposalOutcome::NoSuchProposal { error: crate::improvement::ProposalNotFound { proposal_id: input.proposal_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::improvement::AnySettingsProposal::Applied(instance) => crate::improvement::AnySettingsProposal::Reverted(instance.revert()),
            _ => return Ok(crate::improvement::SettleProposalOutcome::WrongState { error: crate::improvement::ProposalStateConflict { proposal_id: input.proposal_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.revision = input.revision.clone();
        next.data.verdict = Some(input.verdict.clone());
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::improvement::SettleProposalOutcome::Reverted { proposal_reverted: crate::improvement::ProposalReverted { actor: input.actor.clone(), proposal_id: input.proposal_id.clone(), verdict: input.verdict.clone() } };
        SettingsProposalStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.improvement.SupersedeMetricReading`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::SupersedeMetricReadingBehavior for Generated<P>
where
    P: MetricReadingStorage,
{
    fn supersede_metric_reading(&mut self, input: crate::improvement::SupersedeMetricReading) -> Result<crate::improvement::SupersedeMetricReadingOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if MetricReadingStorage::get(&self.ports, &input.reading_id).is_none() {
            return Ok(crate::improvement::SupersedeMetricReadingOutcome::NoSuchReading { error: crate::improvement::MetricReadingNotFound { reading_id: input.reading_id.clone() } });
        }
        // `superseded`: the default.
        let Some(held) = MetricReadingStorage::get(&self.ports, &input.reading_id) else {
            return Ok(crate::improvement::SupersedeMetricReadingOutcome::NoSuchReading { error: crate::improvement::MetricReadingNotFound { reading_id: input.reading_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::improvement::AnyMetricReading::Recorded(instance) => crate::improvement::AnyMetricReading::Superseded(instance.supersede()),
            _ => return Ok(crate::improvement::SupersedeMetricReadingOutcome::WrongState { error: crate::improvement::MetricReadingStateConflict { reading_id: input.reading_id.clone() } }),
        };
        let next = moved.snapshot();
        let answer = crate::improvement::SupersedeMetricReadingOutcome::Superseded { metric_reading_superseded: crate::improvement::MetricReadingSuperseded { actor: input.actor.clone(), reading_id: input.reading_id.clone(), by: input.by.clone() } };
        MetricReadingStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.improvement.VerifyFinding`, generated: every outcome is one the specification fully determines.
impl<P> crate::improvement::obligations::VerifyFindingBehavior for Generated<P>
where
    P: FrictionFindingStorage,
{
    fn verify_finding(&mut self, input: crate::improvement::VerifyFinding) -> Result<crate::improvement::VerifyFindingOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if FrictionFindingStorage::get(&self.ports, &input.finding_id).is_none() {
            return Ok(crate::improvement::VerifyFindingOutcome::NoSuchFinding { error: crate::improvement::FindingNotFound { finding_id: input.finding_id.clone() } });
        }
        // `verified`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.moved).map(|value| *value), Some(true)), "conductor.improvement.VerifyFinding")? {
            let Some(held) = FrictionFindingStorage::get(&self.ports, &input.finding_id) else {
                return Ok(crate::improvement::VerifyFindingOutcome::NoSuchFinding { error: crate::improvement::FindingNotFound { finding_id: input.finding_id.clone() } });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::improvement::AnyFrictionFinding::Filed(instance) => crate::improvement::AnyFrictionFinding::Verified(instance.verify()),
                _ => return Ok(crate::improvement::VerifyFindingOutcome::WrongState { error: crate::improvement::FindingStateConflict { finding_id: input.finding_id.clone() } }),
            };
            let mut next = moved.snapshot();
            next.data.verdict = Some(input.verdict.clone());
            let answer = crate::improvement::VerifyFindingOutcome::Verified { finding_verified: crate::improvement::FindingVerified { actor: input.actor.clone(), finding_id: input.finding_id.clone(), verdict: input.verdict.clone() } };
            FrictionFindingStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `closed`: the default.
        let Some(held) = FrictionFindingStorage::get(&self.ports, &input.finding_id) else {
            return Ok(crate::improvement::VerifyFindingOutcome::NoSuchFinding { error: crate::improvement::FindingNotFound { finding_id: input.finding_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::improvement::AnyFrictionFinding::Filed(instance) => crate::improvement::AnyFrictionFinding::Closed(instance.close()),
            _ => return Ok(crate::improvement::VerifyFindingOutcome::WrongState { error: crate::improvement::FindingStateConflict { finding_id: input.finding_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.verdict = Some(input.verdict.clone());
        let answer = crate::improvement::VerifyFindingOutcome::Closed { finding_closed: crate::improvement::FindingClosed { actor: input.actor.clone(), finding_id: input.finding_id.clone(), verdict: input.verdict.clone() } };
        FrictionFindingStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.instance.ObserveInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::ObserveInstanceBehavior for Generated<P>
where
    P: InstanceStorage,
{
    fn observe_instance(&mut self, input: crate::instance::ObserveInstance) -> Result<crate::instance::ObserveInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if InstanceStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::instance::ObserveInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        }
        // `answering`: an accepting branch, in declaration order.
        if decided(equal(Some(&input.answering).map(|value| *value), Some(true)), "conductor.instance.ObserveInstance")? {
            let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
                return Ok(crate::instance::ObserveInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::instance::AnyInstance::Running(instance) => crate::instance::AnyInstance::Running(instance.come_up()),
                crate::instance::AnyInstance::Starting(instance) => crate::instance::AnyInstance::Running(instance.come_up()),
                crate::instance::AnyInstance::Unreachable(instance) => crate::instance::AnyInstance::Running(instance.come_up()),
                crate::instance::AnyInstance::Upgrading(instance) => crate::instance::AnyInstance::Running(instance.come_up()),
                _ => return Ok(crate::instance::ObserveInstanceOutcome::NotRunning { error: crate::instance::InstanceStateConflict { name: input.name.clone() } }),
            };
            let mut next = moved.snapshot();
            next.data.binary_version = input.binary_version.clone();
            next.data.last_seen = Some(input.at.clone());
            next.data.detail = None;
            next.data.changed_by = input.actor.clone();
            let answer = crate::instance::ObserveInstanceOutcome::Answering { instance_answered: crate::instance::InstanceAnswered { actor: input.actor.clone(), name: input.name.clone(), binary_version: input.binary_version.clone(), at: input.at.clone() } };
            InstanceStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `not-answering`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::ObserveInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::instance::AnyInstance::Running(instance) => crate::instance::AnyInstance::Unreachable(instance.lose()),
            crate::instance::AnyInstance::Starting(instance) => crate::instance::AnyInstance::Unreachable(instance.lose()),
            crate::instance::AnyInstance::Unreachable(instance) => crate::instance::AnyInstance::Unreachable(instance.lose()),
            crate::instance::AnyInstance::Upgrading(instance) => crate::instance::AnyInstance::Unreachable(instance.lose()),
            _ => return Ok(crate::instance::ObserveInstanceOutcome::NotRunning { error: crate::instance::InstanceStateConflict { name: input.name.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.detail = input.detail.clone();
        next.data.changed_by = input.actor.clone();
        let answer = crate::instance::ObserveInstanceOutcome::NotAnswering { instance_unreachable: crate::instance::InstanceUnreachable { actor: input.actor.clone(), name: input.name.clone(), at: input.at.clone() } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.instance.PurgeInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::PurgeInstanceBehavior for Generated<P>
where
    P: InstanceStorage,
{
    fn purge_instance(&mut self, input: crate::instance::PurgeInstance) -> Result<crate::instance::PurgeInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.instance.PurgeInstance")? {
            return Ok(crate::instance::PurgeInstanceOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if InstanceStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::instance::PurgeInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        }
        // `purged`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::PurgeInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::instance::AnyInstance::Stopped(instance) => crate::instance::AnyInstance::Purged(instance.purge()),
            _ => return Ok(crate::instance::PurgeInstanceOutcome::NotStopped { error: crate::instance::InstanceStateConflict { name: input.name.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.changed_by = input.actor.clone();
        let answer = crate::instance::PurgeInstanceOutcome::Purged { instance_purged: crate::instance::InstancePurged { actor: input.actor.clone(), name: input.name.clone(), at: input.at.clone() } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.instance.Reconcile`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::ReconcileBehavior for Generated<P>
where
    P: TryContext + ReconciliationStorage,
{
    fn reconcile(&mut self, input: crate::instance::Reconcile) -> Result<crate::instance::ReconcileOutcome, UnmetObligation> {
        let _ = &input;
        // `recorded`: the default.
        let identity: crate::instance::ReconciliationId = self.ports.try_generate_conductor_instance_reconciliation_id()?;
        let data = crate::instance::ReconciliationData {
            reconciliation_id: identity.clone(),
            instance: input.instance.clone(),
            action: input.action.clone(),
            reason: input.reason.clone(),
            result: input.result.clone(),
            detail: input.detail.clone(),
            actor: input.actor.clone(),
            at: input.at.clone(),
        };
        let answer = crate::instance::ReconcileOutcome::Recorded { instance_reconciled: crate::instance::InstanceReconciled { reconciliation_id: identity.clone(), actor: input.actor.clone(), instance: input.instance.clone(), action: input.action.clone(), reason: input.reason.clone(), result: input.result.clone(), at: input.at.clone() } };
        ReconciliationStorage::put(&mut self.ports, crate::instance::AnyReconciliation::Recorded(crate::instance::Reconciliation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.instance.RestartInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::RestartInstanceBehavior for Generated<P>
where
    P: TryContext + InstanceStorage,
{
    fn restart_instance(&mut self, input: crate::instance::RestartInstance) -> Result<crate::instance::RestartInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if InstanceStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::instance::RestartInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        }
        // `placement-refused`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorInstanceRestartInstance(&input), "placement-refused")? {
            return Ok(crate::instance::RestartInstanceOutcome::PlacementRefused { error: crate::instance::PlacementRefused { name: input.name.clone(), placement: input.placement.clone() } });
        }
        // `restarted`: the default.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::RestartInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::instance::AnyInstance::Running(instance) => crate::instance::AnyInstance::Starting(instance.restart()),
            crate::instance::AnyInstance::Unreachable(instance) => crate::instance::AnyInstance::Starting(instance.restart()),
            _ => return Ok(crate::instance::RestartInstanceOutcome::NotRunning { error: crate::instance::InstanceStateConflict { name: input.name.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.placement = input.placement.clone();
        next.data.roles = input.roles.clone();
        next.data.box_id = Some(input.box_id.clone());
        next.data.sources = input.sources.clone();
        next.data.changed_by = input.actor.clone();
        let answer = crate::instance::RestartInstanceOutcome::Restarted { instance_restarted: crate::instance::InstanceRestarted { actor: input.actor.clone(), name: input.name.clone(), placement: input.placement.clone(), box_id: input.box_id.clone(), at: input.at.clone() } };
        InstanceStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

impl<P: crate::instance::obligations::StartInstanceBehavior> crate::instance::obligations::StartInstanceBehavior for Generated<P> {
    fn start_instance(&mut self, input: crate::instance::StartInstance) -> Result<crate::instance::StartInstanceOutcome, UnmetObligation> {
        crate::instance::obligations::StartInstanceBehavior::start_instance(&mut self.ports, input)
    }
}

/// `conductor.instance.StopInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::StopInstanceBehavior for Generated<P>
where
    P: InstanceStorage,
{
    fn stop_instance(&mut self, input: crate::instance::StopInstance) -> Result<crate::instance::StopInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if InstanceStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::instance::StopInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::StopInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        };
        let _ = &held;
        // `stopped`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::instance::InstanceState::Running | crate::instance::InstanceState::Starting | crate::instance::InstanceState::Unreachable | crate::instance::InstanceState::Upgrading)), "conductor.instance.StopInstance")? {
            let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
                return Ok(crate::instance::StopInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::instance::AnyInstance::Running(instance) => crate::instance::AnyInstance::Stopped(instance.stop()),
                crate::instance::AnyInstance::Starting(instance) => crate::instance::AnyInstance::Stopped(instance.stop()),
                crate::instance::AnyInstance::Unreachable(instance) => crate::instance::AnyInstance::Stopped(instance.stop()),
                crate::instance::AnyInstance::Upgrading(instance) => crate::instance::AnyInstance::Stopped(instance.stop()),
                _ => return Err(undeclared("conductor.instance.StopInstance")),
            };
            let mut next = moved.snapshot();
            next.data.box_id = None;
            next.data.changed_by = input.actor.clone();
            let answer = crate::instance::StopInstanceOutcome::Stopped { instance_stopped: crate::instance::InstanceStopped { actor: input.actor.clone(), name: input.name.clone(), at: input.at.clone() } };
            InstanceStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `already-stopped`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::instance::InstanceState::Stopped)), "conductor.instance.StopInstance")? {
            let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
                return Ok(crate::instance::StopInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
            };
            let _ = &held;
            let answer = crate::instance::StopInstanceOutcome::AlreadyStopped;
            return Ok(answer);
        }
        // `purged`: the default.
        return Ok(crate::instance::StopInstanceOutcome::Purged { error: crate::instance::InstanceStateConflict { name: input.name.clone() } });
    }
}

/// `conductor.instance.UpgradeInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::instance::obligations::UpgradeInstanceBehavior for Generated<P>
where
    P: InstanceStorage,
{
    fn upgrade_instance(&mut self, input: crate::instance::UpgradeInstance) -> Result<crate::instance::UpgradeInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if InstanceStorage::get(&self.ports, &input.name).is_none() {
            return Ok(crate::instance::UpgradeInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
            return Ok(crate::instance::UpgradeInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
        };
        let _ = &held;
        // `upgrading`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::instance::InstanceState::Running)), "conductor.instance.UpgradeInstance")? {
            let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
                return Ok(crate::instance::UpgradeInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::instance::AnyInstance::Running(instance) => crate::instance::AnyInstance::Upgrading(instance.upgrade()),
                _ => return Err(undeclared("conductor.instance.UpgradeInstance")),
            };
            let mut next = moved.snapshot();
            next.data.changed_by = input.actor.clone();
            let answer = crate::instance::UpgradeInstanceOutcome::Upgrading { instance_upgrading: crate::instance::InstanceUpgrading { actor: input.actor.clone(), name: input.name.clone(), binary_version: input.binary_version.clone(), at: input.at.clone() } };
            InstanceStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `rolled-back`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::instance::InstanceState::Unreachable | crate::instance::InstanceState::Upgrading)), "conductor.instance.UpgradeInstance")? {
            let Some(held) = InstanceStorage::get(&self.ports, &input.name) else {
                return Ok(crate::instance::UpgradeInstanceOutcome::Unknown { error: crate::instance::UnknownInstance { name: input.name.clone() } });
            };
            let _ = &held;
            let moved = match held.refine() {
                crate::instance::AnyInstance::Unreachable(instance) => crate::instance::AnyInstance::Starting(instance.roll_back()),
                crate::instance::AnyInstance::Upgrading(instance) => crate::instance::AnyInstance::Starting(instance.roll_back()),
                _ => return Err(undeclared("conductor.instance.UpgradeInstance")),
            };
            let mut next = moved.snapshot();
            next.data.changed_by = input.actor.clone();
            let answer = crate::instance::UpgradeInstanceOutcome::RolledBack { instance_rolled_back: crate::instance::InstanceRolledBack { actor: input.actor.clone(), name: input.name.clone(), binary_version: input.binary_version.clone(), at: input.at.clone() } };
            InstanceStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // `not-running`: the default.
        return Ok(crate::instance::UpgradeInstanceOutcome::NotRunning { error: crate::instance::InstanceStateConflict { name: input.name.clone() } });
    }
}

/// `conductor.observation.CompleteSnapshot`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::CompleteSnapshotBehavior for Generated<P>
where
    P: SnapshotStorage,
{
    fn complete_snapshot(&mut self, input: crate::observation::CompleteSnapshot) -> Result<crate::observation::CompleteSnapshotOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if SnapshotStorage::get(&self.ports, &input.snapshot_id).is_none() {
            return Ok(crate::observation::CompleteSnapshotOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `completed`: the default.
        let Some(held) = SnapshotStorage::get(&self.ports, &input.snapshot_id) else {
            return Ok(crate::observation::CompleteSnapshotOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::observation::AnySnapshot::Collecting(instance) => crate::observation::AnySnapshot::Complete(instance.complete()),
            _ => return Ok(crate::observation::CompleteSnapshotOutcome::WrongState { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } }),
        };
        let next = moved.snapshot();
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::CompleteSnapshotOutcome::Completed { snapshot_completed: crate::observation::SnapshotCompleted { actor: input.actor.clone(), snapshot_id: input.snapshot_id.clone() } };
        SnapshotStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.observation.FailSnapshot`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::FailSnapshotBehavior for Generated<P>
where
    P: SnapshotStorage,
{
    fn fail_snapshot(&mut self, input: crate::observation::FailSnapshot) -> Result<crate::observation::FailSnapshotOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if SnapshotStorage::get(&self.ports, &input.snapshot_id).is_none() {
            return Ok(crate::observation::FailSnapshotOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `failed`: the default.
        let Some(held) = SnapshotStorage::get(&self.ports, &input.snapshot_id) else {
            return Ok(crate::observation::FailSnapshotOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::observation::AnySnapshot::Collecting(instance) => crate::observation::AnySnapshot::Failed(instance.fail()),
            _ => return Ok(crate::observation::FailSnapshotOutcome::WrongState { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.failure_reason = Some(input.reason.clone());
        if let Some(broken) = next.data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::FailSnapshotOutcome::Failed { snapshot_failed: crate::observation::SnapshotFailed { actor: input.actor.clone(), snapshot_id: input.snapshot_id.clone(), reason: input.reason.clone() } };
        SnapshotStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.observation.PublishBoard`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::PublishBoardBehavior for Generated<P>
where
    P: TryContext + BoardStorage + SnapshotStorage,
{
    fn publish_board(&mut self, input: crate::observation::PublishBoard) -> Result<crate::observation::PublishBoardOutcome, UnmetObligation> {
        let _ = &input;
        // `negative-rows`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.deviation_rows).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.observation.PublishBoard")? {
            return Ok(crate::observation::PublishBoardOutcome::NegativeRows { error: crate::observation::InvalidCount { count: input.deviation_rows.clone() } });
        }
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::PublishBoardOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `not-complete`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Complete".to_owned())).map(|value| !value), "conductor.observation.PublishBoard")? {
            return Ok(crate::observation::PublishBoardOutcome::NotComplete { error: crate::observation::SnapshotNotComplete { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `published`: the default.
        let identity: crate::observation::BoardId = self.ports.try_generate_conductor_observation_board_id()?;
        let data = crate::observation::BoardData {
            board_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            previous_snapshot_id: input.previous_snapshot_id.clone(),
            deviation_rows: input.deviation_rows.clone(),
            path: input.path.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::PublishBoardOutcome::Published { board_published: crate::observation::BoardPublished { actor: input.actor.clone(), board_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), deviation_rows: input.deviation_rows.clone() } };
        BoardStorage::put(&mut self.ports, crate::observation::AnyBoard::Published(crate::observation::Board::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordBlocker`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordBlockerBehavior for Generated<P>
where
    P: TryContext + BlockerObservationStorage + SnapshotStorage,
{
    fn record_blocker(&mut self, input: crate::observation::RecordBlocker) -> Result<crate::observation::RecordBlockerOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordBlockerOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordBlocker")? {
            return Ok(crate::observation::RecordBlockerOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::BlockerObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            repository: input.repository.clone(),
            reference: input.reference.clone(),
            blocker_kind: input.blocker_kind.clone(),
            title: input.title.clone(),
        };
        let answer = crate::observation::RecordBlockerOutcome::Recorded { blocker_recorded: crate::observation::BlockerRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), repository: input.repository.clone(), reference: input.reference.clone() } };
        BlockerObservationStorage::put(&mut self.ports, crate::observation::AnyBlockerObservation::Recorded(crate::observation::BlockerObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordCollectorRun`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordCollectorRunBehavior for Generated<P>
where
    P: TryContext + CollectorRunObservationStorage + SnapshotStorage,
{
    fn record_collector_run(&mut self, input: crate::observation::RecordCollectorRun) -> Result<crate::observation::RecordCollectorRunOutcome, UnmetObligation> {
        let _ = &input;
        // `negative-duration`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.duration_ms).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.observation.RecordCollectorRun")? {
            return Ok(crate::observation::RecordCollectorRunOutcome::NegativeDuration { error: crate::observation::InvalidCount { count: input.duration_ms.clone() } });
        }
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordCollectorRunOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordCollectorRun")? {
            return Ok(crate::observation::RecordCollectorRunOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::CollectorRunObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            collector: input.collector.clone(),
            started_at: input.started_at.clone(),
            duration_ms: input.duration_ms.clone(),
            outcome: input.outcome.clone(),
            read_bytes: input.read_bytes.clone(),
            written_bytes: input.written_bytes.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::RecordCollectorRunOutcome::Recorded { collector_run_recorded: crate::observation::CollectorRunRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), collector: input.collector.clone(), duration_ms: input.duration_ms.clone(), outcome: input.outcome.clone() } };
        CollectorRunObservationStorage::put(&mut self.ports, crate::observation::AnyCollectorRunObservation::Recorded(crate::observation::CollectorRunObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordDailySeries`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordDailySeriesBehavior for Generated<P>
where
    P: DailySeriesStorage,
{
    fn record_daily_series(&mut self, input: crate::observation::RecordDailySeries) -> Result<crate::observation::RecordDailySeriesOutcome, UnmetObligation> {
        let _ = &input;
        // `already-recorded`: an identity a record already carries, before any branch is taken.
        if DailySeriesStorage::get(&self.ports, &input.daily_series_id).is_some() {
            return Ok(crate::observation::RecordDailySeriesOutcome::AlreadyRecorded { error: crate::observation::DailySeriesExists { daily_series_id: input.daily_series_id.clone() } });
        }
        // `recorded`: the default.
        let identity: crate::observation::DailySeriesId = input.daily_series_id.clone();
        let data = crate::observation::DailySeriesData {
            daily_series_id: identity.clone(),
            actor: input.actor.clone(),
            series: input.series.clone(),
            day: input.day.clone(),
            by: input.by.clone(),
            key: input.key.clone(),
            value: input.value.clone(),
            items: input.items.clone(),
            snapshot_id: input.snapshot_id.clone(),
        };
        let answer = crate::observation::RecordDailySeriesOutcome::Recorded { daily_series_recorded: crate::observation::DailySeriesRecorded { actor: input.actor.clone(), daily_series_id: identity.clone(), snapshot_id: input.snapshot_id.clone() } };
        DailySeriesStorage::put(&mut self.ports, crate::observation::AnyDailySeries::Recorded(crate::observation::DailySeries::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordMemberFailure`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordMemberFailureBehavior for Generated<P>
where
    P: TryContext + MemberFailureObservationStorage + SnapshotStorage,
{
    fn record_member_failure(&mut self, input: crate::observation::RecordMemberFailure) -> Result<crate::observation::RecordMemberFailureOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordMemberFailureOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordMemberFailure")? {
            return Ok(crate::observation::RecordMemberFailureOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::MemberFailureObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            member: input.member.clone(),
            collector: input.collector.clone(),
            error_line: input.error_line.clone(),
        };
        let answer = crate::observation::RecordMemberFailureOutcome::Recorded { member_failure_recorded: crate::observation::MemberFailureRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), member: input.member.clone(), collector: input.collector.clone() } };
        MemberFailureObservationStorage::put(&mut self.ports, crate::observation::AnyMemberFailureObservation::Recorded(crate::observation::MemberFailureObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordMergedPullRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordMergedPullRequestBehavior for Generated<P>
where
    P: TryContext + MergedPullRequestObservationStorage + SnapshotStorage,
{
    fn record_merged_pull_request(&mut self, input: crate::observation::RecordMergedPullRequest) -> Result<crate::observation::RecordMergedPullRequestOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-number`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.number).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_le), "conductor.observation.RecordMergedPullRequest")? {
            return Ok(crate::observation::RecordMergedPullRequestOutcome::InvalidNumber { error: crate::observation::InvalidPullRequestNumber { number: input.number.clone() } });
        }
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordMergedPullRequestOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordMergedPullRequest")? {
            return Ok(crate::observation::RecordMergedPullRequestOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::MergedPullRequestObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            repository: input.repository.clone(),
            number: input.number.clone(),
            title: input.title.clone(),
            merged_at: input.merged_at.clone(),
            url: input.url.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::RecordMergedPullRequestOutcome::Recorded { merged_pull_request_recorded: crate::observation::MergedPullRequestRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), repository: input.repository.clone(), number: input.number.clone() } };
        MergedPullRequestObservationStorage::put(&mut self.ports, crate::observation::AnyMergedPullRequestObservation::Recorded(crate::observation::MergedPullRequestObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordPullRequest`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordPullRequestBehavior for Generated<P>
where
    P: TryContext + PullRequestObservationStorage + SnapshotStorage,
{
    fn record_pull_request(&mut self, input: crate::observation::RecordPullRequest) -> Result<crate::observation::RecordPullRequestOutcome, UnmetObligation> {
        let _ = &input;
        // `invalid-number`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.number).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_le), "conductor.observation.RecordPullRequest")? {
            return Ok(crate::observation::RecordPullRequestOutcome::InvalidNumber { error: crate::observation::InvalidPullRequestNumber { number: input.number.clone() } });
        }
        // `negative-checks`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.failing_checks).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.observation.RecordPullRequest")? {
            return Ok(crate::observation::RecordPullRequestOutcome::NegativeChecks { error: crate::observation::InvalidCount { count: input.failing_checks.clone() } });
        }
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordPullRequestOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordPullRequest")? {
            return Ok(crate::observation::RecordPullRequestOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::PullRequestObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            repository: input.repository.clone(),
            number: input.number.clone(),
            title: input.title.clone(),
            draft: input.draft.clone(),
            mergeability: input.mergeability.clone(),
            failing_checks: input.failing_checks.clone(),
            failing_check_names: input.failing_check_names.clone(),
            opened_at: input.opened_at.clone(),
            updated_at: input.updated_at.clone(),
            url: input.url.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::RecordPullRequestOutcome::Recorded { pull_request_recorded: crate::observation::PullRequestRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), repository: input.repository.clone(), number: input.number.clone() } };
        PullRequestObservationStorage::put(&mut self.ports, crate::observation::AnyPullRequestObservation::Recorded(crate::observation::PullRequestObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordRelease`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordReleaseBehavior for Generated<P>
where
    P: TryContext + ReleaseObservationStorage + SnapshotStorage,
{
    fn record_release(&mut self, input: crate::observation::RecordRelease) -> Result<crate::observation::RecordReleaseOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordReleaseOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordRelease")? {
            return Ok(crate::observation::RecordReleaseOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::ReleaseObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            repository: input.repository.clone(),
            tag: input.tag.clone(),
            published_at: input.published_at.clone(),
            url: input.url.clone(),
        };
        let answer = crate::observation::RecordReleaseOutcome::Recorded { release_recorded: crate::observation::ReleaseRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), repository: input.repository.clone(), tag: input.tag.clone() } };
        ReleaseObservationStorage::put(&mut self.ports, crate::observation::AnyReleaseObservation::Recorded(crate::observation::ReleaseObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordRepository`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordRepositoryBehavior for Generated<P>
where
    P: TryContext + RepositoryObservationStorage + SnapshotStorage,
{
    fn record_repository(&mut self, input: crate::observation::RecordRepository) -> Result<crate::observation::RecordRepositoryOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordRepositoryOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordRepository")? {
            return Ok(crate::observation::RecordRepositoryOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::RepositoryObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            repository: input.repository.clone(),
            visibility: input.visibility.clone(),
            archived: input.archived.clone(),
            local_checkout: input.local_checkout.clone(),
            main_head: input.main_head.clone(),
            main_committed_at: input.main_committed_at.clone(),
            main_commits_7d: input.main_commits_7d.clone(),
            real_commits_7d: input.real_commits_7d.clone(),
            behind_main: input.behind_main.clone(),
            dirty_files: input.dirty_files.clone(),
            worktrees: input.worktrees.clone(),
            open_issues: input.open_issues.clone(),
            oldest_open_issue_at: input.oldest_open_issue_at.clone(),
            latest_release: input.latest_release.clone(),
            latest_release_at: input.latest_release_at.clone(),
            unreleased_commits: input.unreleased_commits.clone(),
            planning_store_version: input.planning_store_version.clone(),
            in_catalog: input.in_catalog.clone(),
            gates_hooks_version: input.gates_hooks_version.clone(),
            gates_version: input.gates_version.clone(),
            wave_width: input.wave_width.clone(),
            wave_ready: input.wave_ready.clone(),
            wave_reason: input.wave_reason.clone(),
        };
        let answer = crate::observation::RecordRepositoryOutcome::Recorded { repository_recorded: crate::observation::RepositoryRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), repository: input.repository.clone() } };
        RepositoryObservationStorage::put(&mut self.ports, crate::observation::AnyRepositoryObservation::Recorded(crate::observation::RepositoryObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordSecretSync`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordSecretSyncBehavior for Generated<P>
where
    P: TryContext + SecretSyncObservationStorage + SnapshotStorage,
{
    fn record_secret_sync(&mut self, input: crate::observation::RecordSecretSync) -> Result<crate::observation::RecordSecretSyncOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordSecretSyncOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordSecretSync")? {
            return Ok(crate::observation::RecordSecretSyncOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::SecretSyncObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            sync: input.sync.clone(),
            repository: input.repository.clone(),
            branch: input.branch.clone(),
            path: input.path.clone(),
            source_committed_at: input.source_committed_at.clone(),
            secrets_updated_at: input.secrets_updated_at.clone(),
            error_line: input.error_line.clone(),
        };
        let answer = crate::observation::RecordSecretSyncOutcome::Recorded { secret_sync_recorded: crate::observation::SecretSyncRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), sync: input.sync.clone(), repository: input.repository.clone() } };
        SecretSyncObservationStorage::put(&mut self.ports, crate::observation::AnySecretSyncObservation::Recorded(crate::observation::SecretSyncObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordSession`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordSessionBehavior for Generated<P>
where
    P: TryContext + SessionObservationStorage + SnapshotStorage,
{
    fn record_session(&mut self, input: crate::observation::RecordSession) -> Result<crate::observation::RecordSessionOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordSessionOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordSession")? {
            return Ok(crate::observation::RecordSessionOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::SessionObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            harness: input.harness.clone(),
            session_ref: input.session_ref.clone(),
            name: input.name.clone(),
            cwd: input.cwd.clone(),
            repository: input.repository.clone(),
            role: input.role.clone(),
            activity: input.activity.clone(),
            envelope_provider: input.envelope_provider.clone(),
            envelope_cgroup: input.envelope_cgroup.clone(),
            envelope_memory_max: input.envelope_memory_max.clone(),
            envelope_cpu_weight: input.envelope_cpu_weight.clone(),
            envelope_tasks_max: input.envelope_tasks_max.clone(),
            envelope_writable: input.envelope_writable.clone(),
            envelope_unboxed: input.envelope_unboxed.clone(),
        };
        let answer = crate::observation::RecordSessionOutcome::Recorded { session_recorded: crate::observation::SessionRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), harness: input.harness.clone(), session_ref: input.session_ref.clone() } };
        SessionObservationStorage::put(&mut self.ports, crate::observation::AnySessionObservation::Recorded(crate::observation::SessionObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordSpecification`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordSpecificationBehavior for Generated<P>
where
    P: TryContext + SnapshotStorage + SpecificationObservationStorage,
{
    fn record_specification(&mut self, input: crate::observation::RecordSpecification) -> Result<crate::observation::RecordSpecificationOutcome, UnmetObligation> {
        let _ = &input;
        // `negative-refusals`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.validation_refusals).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.observation.RecordSpecification")? {
            return Ok(crate::observation::RecordSpecificationOutcome::NegativeRefusals { error: crate::observation::InvalidCount { count: input.validation_refusals.clone() } });
        }
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordSpecificationOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordSpecification")? {
            return Ok(crate::observation::RecordSpecificationOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::SpecificationObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            repository: input.repository.clone(),
            presence: input.presence.clone(),
            path: input.path.clone(),
            format: input.format.clone(),
            required_ess: input.required_ess.clone(),
            validation: input.validation.clone(),
            validation_refusals: input.validation_refusals.clone(),
            scenarios: input.scenarios.clone(),
            synthesis_refusals: input.synthesis_refusals.clone(),
            conformance_status: input.conformance_status.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::RecordSpecificationOutcome::Recorded { specification_recorded: crate::observation::SpecificationRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), repository: input.repository.clone(), presence: input.presence.clone(), validation: input.validation.clone() } };
        SpecificationObservationStorage::put(&mut self.ports, crate::observation::AnySpecificationObservation::Recorded(crate::observation::SpecificationObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordUsage`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordUsageBehavior for Generated<P>
where
    P: TryContext + SnapshotStorage + UsageObservationStorage,
{
    fn record_usage(&mut self, input: crate::observation::RecordUsage) -> Result<crate::observation::RecordUsageOutcome, UnmetObligation> {
        let _ = &input;
        // `negative-percent`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.used_percent).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.observation.RecordUsage")? {
            return Ok(crate::observation::RecordUsageOutcome::NegativePercent { error: crate::observation::InvalidCount { count: input.used_percent.clone() } });
        }
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordUsageOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordUsage")? {
            return Ok(crate::observation::RecordUsageOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::UsageObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            read_at: input.read_at.clone(),
            account: input.account.clone(),
            active: input.active.clone(),
            health: input.health.clone(),
            window: input.window.clone(),
            used_percent: input.used_percent.clone(),
            resets_at: input.resets_at.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::RecordUsageOutcome::Recorded { usage_recorded: crate::observation::UsageRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), account: input.account.clone(), window: input.window.clone(), used_percent: input.used_percent.clone() } };
        UsageObservationStorage::put(&mut self.ports, crate::observation::AnyUsageObservation::Recorded(crate::observation::UsageObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.RecordWorkflowRun`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::RecordWorkflowRunBehavior for Generated<P>
where
    P: TryContext + SnapshotStorage + WorkflowRunObservationStorage,
{
    fn record_workflow_run(&mut self, input: crate::observation::RecordWorkflowRun) -> Result<crate::observation::RecordWorkflowRunOutcome, UnmetObligation> {
        let _ = &input;
        // `when_related:` reads the `conductor.observation.Snapshot` row `input.snapshot_id` names, through its storage port; an absent
        // reference reads no row and selects no related branch.
        let reference = Some(&input.snapshot_id);
        let related = reference.and_then(|identity| SnapshotStorage::get(&self.ports, identity));
        let _ = &related;
        // The addressed row's existence and held state, for the branch the request selects
        // before any present related row's refusal.
        {
        }
        // `no-such-snapshot`: the reference names an identity no row carries.
        if reference.is_some() && related.is_none() {
            return Ok(crate::observation::RecordWorkflowRunOutcome::NoSuchSnapshot { error: crate::observation::SnapshotNotFound { snapshot_id: input.snapshot_id.clone() } });
        }
        // `closed`: a present related row's refusal, before every accepting branch.
        if let Some(related) = &related {
        if decided(equal(Some(&related.state).map(|value| match value { crate::observation::SnapshotState::Collecting => "Collecting", crate::observation::SnapshotState::Complete => "Complete", crate::observation::SnapshotState::Failed => "Failed" }.to_owned()), Some("Collecting".to_owned())).map(|value| !value), "conductor.observation.RecordWorkflowRun")? {
            return Ok(crate::observation::RecordWorkflowRunOutcome::Closed { error: crate::observation::SnapshotNotCollecting { snapshot_id: input.snapshot_id.clone() } });
        }
        }
        // `recorded`: the default.
        let identity: crate::observation::ObservationId = self.ports.try_generate_conductor_observation_observation_id()?;
        let data = crate::observation::WorkflowRunObservationData {
            observation_id: identity.clone(),
            actor: input.actor.clone(),
            snapshot_id: input.snapshot_id.clone(),
            repository: input.repository.clone(),
            workflow: input.workflow.clone(),
            conclusion: input.conclusion.clone(),
            run_at: input.run_at.clone(),
        };
        let answer = crate::observation::RecordWorkflowRunOutcome::Recorded { workflow_run_recorded: crate::observation::WorkflowRunRecorded { actor: input.actor.clone(), observation_id: identity.clone(), snapshot_id: input.snapshot_id.clone(), repository: input.repository.clone(), workflow: input.workflow.clone() } };
        WorkflowRunObservationStorage::put(&mut self.ports, crate::observation::AnyWorkflowRunObservation::Recorded(crate::observation::WorkflowRunObservation::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.observation.ReviseDailySeries`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::ReviseDailySeriesBehavior for Generated<P>
where
    P: DailySeriesStorage,
{
    fn revise_daily_series(&mut self, input: crate::observation::ReviseDailySeries) -> Result<crate::observation::ReviseDailySeriesOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if DailySeriesStorage::get(&self.ports, &input.daily_series_id).is_none() {
            return Ok(crate::observation::ReviseDailySeriesOutcome::NoSuchRollup { error: crate::observation::DailySeriesNotFound { daily_series_id: input.daily_series_id.clone() } });
        }
        // The addressed row, read before the branches that select by it.
        let Some(held) = DailySeriesStorage::get(&self.ports, &input.daily_series_id) else {
            return Ok(crate::observation::ReviseDailySeriesOutcome::NoSuchRollup { error: crate::observation::DailySeriesNotFound { daily_series_id: input.daily_series_id.clone() } });
        };
        let _ = &held;
        // `revised`: selected by the addressed row.
        if decided(Some(matches!(held.state, crate::observation::DailySeriesState::Recorded)), "conductor.observation.ReviseDailySeries")? {
            let Some(held) = DailySeriesStorage::get(&self.ports, &input.daily_series_id) else {
                return Ok(crate::observation::ReviseDailySeriesOutcome::NoSuchRollup { error: crate::observation::DailySeriesNotFound { daily_series_id: input.daily_series_id.clone() } });
            };
            let _ = &held;
            let mut next = held;
            next.data.value = input.value.clone();
            next.data.items = input.items.clone();
            next.data.snapshot_id = input.snapshot_id.clone();
            let answer = crate::observation::ReviseDailySeriesOutcome::Revised { daily_series_revised: crate::observation::DailySeriesRevised { actor: input.actor.clone(), daily_series_id: input.daily_series_id.clone(), snapshot_id: input.snapshot_id.clone() } };
            DailySeriesStorage::put(&mut self.ports, next);
            return Ok(answer);
        }
        // No declared branch answers this request.
        Err(undeclared("conductor.observation.ReviseDailySeries"))
    }
}

/// `conductor.observation.StartSnapshot`, generated: every outcome is one the specification fully determines.
impl<P> crate::observation::obligations::StartSnapshotBehavior for Generated<P>
where
    P: TryContext + SnapshotStorage,
{
    fn start_snapshot(&mut self, input: crate::observation::StartSnapshot) -> Result<crate::observation::StartSnapshotOutcome, UnmetObligation> {
        let _ = &input;
        // `negative-disk`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.disk_free_bytes).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.observation.StartSnapshot")? {
            return Ok(crate::observation::StartSnapshotOutcome::NegativeDisk { error: crate::observation::InvalidCount { count: input.disk_free_bytes.clone() } });
        }
        // `started`: the default.
        let identity: crate::observation::SnapshotId = self.ports.try_generate_conductor_observation_snapshot_id()?;
        let data = crate::observation::SnapshotData {
            snapshot_id: identity.clone(),
            actor: input.actor.clone(),
            started_at: input.started_at.clone(),
            disk_free_bytes: input.disk_free_bytes.clone(),
            failure_reason: None,
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::observation::StartSnapshotOutcome::Started { snapshot_started: crate::observation::SnapshotStarted { actor: input.actor.clone(), snapshot_id: identity.clone(), started_at: input.started_at.clone() } };
        SnapshotStorage::put(&mut self.ports, crate::observation::AnySnapshot::Collecting(crate::observation::Snapshot::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.operator.JoinInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::operator::obligations::JoinInstanceBehavior for Generated<P>
where
    P: OperatorStorage,
{
    fn join_instance(&mut self, input: crate::operator::JoinInstance) -> Result<crate::operator::JoinInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.operator.JoinInstance")? {
            return Ok(crate::operator::JoinInstanceOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `rejoined`: the default.
        let Some(held) = OperatorStorage::get(&self.ports, &input.key) else {
            let identity: crate::operator::OperatorKey = input.key.clone();
            let data = crate::operator::OperatorData {
                key: identity.clone(),
                grants: input.grants.clone(),
                joined_at: input.joined_at.clone(),
                joined_by: input.actor.clone(),
                left_at: None,
                left_by: None,
            };
            let answer = crate::operator::JoinInstanceOutcome::Joined { operator_joined: crate::operator::OperatorJoined { key: identity.clone(), grants: input.grants.clone(), joined_at: input.joined_at.clone(), actor: input.actor.clone() } };
            OperatorStorage::put(&mut self.ports, crate::operator::AnyOperator::Active(crate::operator::Operator::new(data)).snapshot());
            return Ok(answer);
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::operator::AnyOperator::Left(instance) => crate::operator::AnyOperator::Active(instance.rejoin()),
            _ => return Ok(crate::operator::JoinInstanceOutcome::AlreadyJoined { error: crate::operator::AlreadyJoined { key: input.key.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.grants = input.grants.clone();
        next.data.joined_at = input.joined_at.clone();
        next.data.joined_by = input.actor.clone();
        next.data.left_at = None;
        next.data.left_by = None;
        let answer = crate::operator::JoinInstanceOutcome::Rejoined { operator_joined: crate::operator::OperatorJoined { key: input.key.clone(), grants: input.grants.clone(), joined_at: input.joined_at.clone(), actor: input.actor.clone() } };
        OperatorStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.operator.LeaveInstance`, generated: every outcome is one the specification fully determines.
impl<P> crate::operator::obligations::LeaveInstanceBehavior for Generated<P>
where
    P: OperatorStorage,
{
    fn leave_instance(&mut self, input: crate::operator::LeaveInstance) -> Result<crate::operator::LeaveInstanceOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.operator.LeaveInstance")? {
            return Ok(crate::operator::LeaveInstanceOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // The addressed row's existence, at step 2 of the one precedence rule.
        if OperatorStorage::get(&self.ports, &input.key).is_none() {
            return Ok(crate::operator::LeaveInstanceOutcome::UnknownOperator { error: crate::operator::NotJoined { key: input.key.clone() } });
        }
        // `left`: the default.
        let Some(held) = OperatorStorage::get(&self.ports, &input.key) else {
            return Ok(crate::operator::LeaveInstanceOutcome::UnknownOperator { error: crate::operator::NotJoined { key: input.key.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::operator::AnyOperator::Active(instance) => crate::operator::AnyOperator::Left(instance.leave()),
            _ => return Ok(crate::operator::LeaveInstanceOutcome::NotJoined { error: crate::operator::NotJoined { key: input.key.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.left_at = Some(input.left_at.clone());
        next.data.left_by = Some(input.actor.clone());
        let answer = crate::operator::LeaveInstanceOutcome::Left { operator_left: crate::operator::OperatorLeft { key: input.key.clone(), left_at: input.left_at.clone(), actor: input.actor.clone() } };
        OperatorStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.placement.CloseBox`, generated: every outcome is one the specification fully determines.
impl<P> crate::placement::obligations::CloseBoxBehavior for Generated<P>
where
    P: BoxStorage,
{
    fn close_box(&mut self, input: crate::placement::CloseBox) -> Result<crate::placement::CloseBoxOutcome, UnmetObligation> {
        let _ = &input;
        // The addressed row's existence, at step 2 of the one precedence rule.
        if BoxStorage::get(&self.ports, &input.box_id).is_none() {
            return Ok(crate::placement::CloseBoxOutcome::NoSuchBox { error: crate::placement::BoxNotFound { box_id: input.box_id.clone() } });
        }
        // `closed`: the default.
        let Some(held) = BoxStorage::get(&self.ports, &input.box_id) else {
            return Ok(crate::placement::CloseBoxOutcome::NoSuchBox { error: crate::placement::BoxNotFound { box_id: input.box_id.clone() } });
        };
        let _ = &held;
        let moved = match held.refine() {
            crate::placement::AnyBox::Opened(instance) => crate::placement::AnyBox::Closed(instance.close()),
            _ => return Ok(crate::placement::CloseBoxOutcome::WrongState { error: crate::placement::BoxStateConflict { box_id: input.box_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.closed_at = Some(input.at.clone());
        next.data.closed_by = Some(input.actor.clone());
        let answer = crate::placement::CloseBoxOutcome::Closed { box_closed: crate::placement::BoxClosed { box_id: input.box_id.clone(), actor: input.actor.clone(), closed_at: input.at.clone() } };
        BoxStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.placement.OpenBox`, generated: every outcome is one the specification fully determines.
impl<P> crate::placement::obligations::OpenBoxBehavior for Generated<P>
where
    P: TryContext + BoxStorage + BoxRefusalStorage,
{
    fn open_box(&mut self, input: crate::placement::OpenBox) -> Result<crate::placement::OpenBoxOutcome, UnmetObligation> {
        let _ = &input;
        // `refused`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorPlacementOpenBox(&input), "refused")? {
            let identity: crate::placement::BoxId = self.ports.try_generate_conductor_placement_box_id()?;
            let data = crate::placement::BoxRefusalData {
                box_id: identity.clone(),
                placement: input.placement.clone(),
                instance: input.instance.clone(),
                actor: input.actor.clone(),
                reason: self.ports.try_generate_conductor_placement_refusal_reason()?,
                refused_at: input.at.clone(),
            };
            let answer = crate::placement::OpenBoxOutcome::Refused { box_refused: crate::placement::BoxRefused { box_id: identity.clone(), actor: input.actor.clone(), placement: input.placement.clone(), instance: input.instance.clone(), reason: self.ports.try_generate_conductor_placement_refusal_reason()?, refused_at: input.at.clone() } };
            BoxRefusalStorage::put(&mut self.ports, crate::placement::AnyBoxRefusal::Refused(crate::placement::BoxRefusal::new(data)).snapshot());
            return Ok(answer);
        }
        // `adapter-refused`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorPlacementOpenBox(&input), "adapter-refused")? {
            let identity: crate::placement::BoxId = self.ports.try_generate_conductor_placement_box_id()?;
            let data = crate::placement::BoxRefusalData {
                box_id: identity.clone(),
                placement: input.placement.clone(),
                instance: input.instance.clone(),
                actor: input.actor.clone(),
                reason: self.ports.try_generate_conductor_placement_refusal_reason()?,
                refused_at: input.at.clone(),
            };
            let answer = crate::placement::OpenBoxOutcome::AdapterRefused { box_refused: crate::placement::BoxRefused { box_id: identity.clone(), actor: input.actor.clone(), placement: input.placement.clone(), instance: input.instance.clone(), reason: self.ports.try_generate_conductor_placement_refusal_reason()?, refused_at: input.at.clone() } };
            BoxRefusalStorage::put(&mut self.ports, crate::placement::AnyBoxRefusal::Refused(crate::placement::BoxRefusal::new(data)).snapshot());
            return Ok(answer);
        }
        // `opened`: the default.
        let identity: crate::placement::BoxId = self.ports.try_generate_conductor_placement_box_id()?;
        let data = crate::placement::BoxData {
            box_id: identity.clone(),
            placement: input.placement.clone(),
            instance: input.instance.clone(),
            actor: input.actor.clone(),
            opened_at: input.at.clone(),
            closed_at: None,
            closed_by: None,
        };
        let answer = crate::placement::OpenBoxOutcome::Opened { box_opened: crate::placement::BoxOpened { box_id: identity.clone(), actor: input.actor.clone(), placement: input.placement.clone(), instance: input.instance.clone(), opened_at: input.at.clone() } };
        BoxStorage::put(&mut self.ports, crate::placement::AnyBox::Opened(crate::placement::Box::new(data)).snapshot());
        return Ok(answer);
    }
}

impl<P: crate::secret::obligations::PutSecretBehavior> crate::secret::obligations::PutSecretBehavior for Generated<P> {
    fn put_secret(&mut self, input: crate::secret::PutSecret) -> Result<crate::secret::PutSecretOutcome, UnmetObligation> {
        crate::secret::obligations::PutSecretBehavior::put_secret(&mut self.ports, input)
    }
}

impl<P: crate::secret::obligations::ResolveSecretsBehavior> crate::secret::obligations::ResolveSecretsBehavior for Generated<P> {
    fn resolve_secrets(&mut self, input: crate::secret::ResolveSecrets) -> Result<crate::secret::ResolveSecretsOutcome, UnmetObligation> {
        crate::secret::obligations::ResolveSecretsBehavior::resolve_secrets(&mut self.ports, input)
    }
}

impl<P: crate::session::obligations::ClaimLeaseBehavior> crate::session::obligations::ClaimLeaseBehavior for Generated<P> {
    fn claim_lease(&mut self, input: crate::session::ClaimLease) -> Result<crate::session::ClaimLeaseOutcome, UnmetObligation> {
        crate::session::obligations::ClaimLeaseBehavior::claim_lease(&mut self.ports, input)
    }
}

/// `conductor.session.DeliverMessage`, generated: every outcome is one the specification fully determines.
impl<P> crate::session::obligations::DeliverMessageBehavior for Generated<P>
where
    P: TryContext + DeliveryStorage,
{
    fn deliver_message(&mut self, input: crate::session::DeliverMessage) -> Result<crate::session::DeliverMessageOutcome, UnmetObligation> {
        let _ = &input;
        // `unreachable`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorSessionDeliverMessage(&input), "unreachable")? {
            return Ok(crate::session::DeliverMessageOutcome::Unreachable { error: crate::session::SessionNotReachable { session_name: input.session_name.clone() } });
        }
        // `unread`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorSessionDeliverMessage(&input), "unread")? {
            return Ok(crate::session::DeliverMessageOutcome::Unread { error: crate::session::DeliveryNotSeenRead { session_name: input.session_name.clone() } });
        }
        // `delivered`: the default.
        let identity: crate::session::DeliveryId = self.ports.try_generate_conductor_session_delivery_id()?;
        let data = crate::session::DeliveryData {
            delivery_id: identity.clone(),
            actor: input.actor.clone(),
            session_name: input.session_name.clone(),
            first_line: input.first_line.clone(),
            body_path: input.body_path.clone(),
            delivered_at: input.delivered_at.clone(),
        };
        let answer = crate::session::DeliverMessageOutcome::Delivered { message_delivered: crate::session::MessageDelivered { actor: input.actor.clone(), delivery_id: identity.clone(), session_name: input.session_name.clone(), first_line: input.first_line.clone(), body_path: input.body_path.clone(), delivered_at: input.delivered_at.clone() } };
        DeliveryStorage::put(&mut self.ports, crate::session::AnyDelivery::Delivered(crate::session::Delivery::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.session.RecordRunTick`, generated: every outcome is one the specification fully determines.
impl<P> crate::session::obligations::RecordRunTickBehavior for Generated<P>
where
    P: TryContext + RunTickStorage,
{
    fn record_run_tick(&mut self, input: crate::session::RecordRunTick) -> Result<crate::session::RecordRunTickOutcome, UnmetObligation> {
        let _ = &input;
        // `negative-count`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(compare_numbers(Some(&input.lines_handled).map(|value| value.to_string()), Some("0".to_owned()), core::cmp::Ordering::is_lt), "conductor.session.RecordRunTick")? {
            return Ok(crate::session::RecordRunTickOutcome::NegativeCount { error: crate::session::InvalidLineCount { lines_handled: input.lines_handled.clone() } });
        }
        // `recorded`: the default.
        let identity: crate::session::TickId = self.ports.try_generate_conductor_session_tick_id()?;
        let data = crate::session::RunTickData {
            tick_id: identity.clone(),
            actor: input.actor.clone(),
            kind: input.kind.clone(),
            started_at: input.started_at.clone(),
            lines_handled: input.lines_handled.clone(),
            actions: input.actions.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::session::RecordRunTickOutcome::Recorded { run_tick_recorded: crate::session::RunTickRecorded { actor: input.actor.clone(), tick_id: identity.clone(), kind: input.kind.clone(), started_at: input.started_at.clone(), lines_handled: input.lines_handled.clone() } };
        RunTickStorage::put(&mut self.ports, crate::session::AnyRunTick::Recorded(crate::session::RunTick::new(data)).snapshot());
        return Ok(answer);
    }
}

impl<P: crate::store::obligations::MigrateStoreBehavior> crate::store::obligations::MigrateStoreBehavior for Generated<P> {
    fn migrate_store(&mut self, input: crate::store::MigrateStore) -> Result<crate::store::MigrateStoreOutcome, UnmetObligation> {
        crate::store::obligations::MigrateStoreBehavior::migrate_store(&mut self.ports, input)
    }
}

/// `conductor.token.ContributeToken`, generated: every outcome is one the specification fully determines.
impl<P> crate::token::obligations::ContributeTokenBehavior for Generated<P>
where
    P: TryContext + PoolTokenStorage,
{
    fn contribute_token(&mut self, input: crate::token::ContributeToken) -> Result<crate::token::ContributeTokenOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.token.ContributeToken")? {
            return Ok(crate::token::ContributeTokenOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `contributed`: the default.
        let identity: crate::token::TokenId = self.ports.try_generate_conductor_token_token_id()?;
        let data = crate::token::PoolTokenData {
            token_id: identity.clone(),
            instance: input.instance.clone(),
            owner: input.owner.clone(),
            seat: input.seat.clone(),
            secret: input.secret.clone(),
            token_state: crate::token::TokenState::Ok,
            limited_until: None,
            agent: None,
            contributed_at: input.contributed_at.clone(),
            actor: input.actor.clone(),
        };
        let answer = crate::token::ContributeTokenOutcome::Contributed { token_contributed: crate::token::TokenContributed { actor: input.actor.clone(), token_id: identity.clone(), instance: input.instance.clone(), owner: input.owner.clone(), seat: input.seat.clone(), secret: input.secret.clone(), contributed_at: input.contributed_at.clone() } };
        PoolTokenStorage::put(&mut self.ports, crate::token::AnyPoolToken::Available(crate::token::PoolToken::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.token.LeaseToken`, generated: every outcome is one the specification fully determines.
impl<P> crate::token::obligations::LeaseTokenBehavior for Generated<P>
where
    P: TryContext + PoolTokenStorage,
{
    fn lease_token(&mut self, input: crate::token::LeaseToken) -> Result<crate::token::LeaseTokenOutcome, UnmetObligation> {
        let _ = &input;
        // `none-available`: an external refusal answered before the row is read, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorTokenLeaseToken(&input), "none-available")? {
            return Ok(crate::token::LeaseTokenOutcome::NoneAvailable { error: crate::token::NoTokenAvailable { instance: input.instance.clone() } });
        }
        // `leased`: the default.
        let Some(held) = PoolTokenStorage::get(&self.ports, &input.token_id) else {
            return Ok(crate::token::LeaseTokenOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::token::AnyPoolToken::Available(instance) => crate::token::AnyPoolToken::Leased(instance.lease()),
            crate::token::AnyPoolToken::Limited(instance) => crate::token::AnyPoolToken::Leased(instance.lease()),
            _ => return Ok(crate::token::LeaseTokenOutcome::WrongState { error: crate::token::TokenStateConflict { token_id: input.token_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.token_state = crate::token::TokenState::Ok;
        next.data.limited_until = None;
        next.data.agent = Some(input.agent.clone());
        next.data.actor = input.actor.clone();
        let answer = crate::token::LeaseTokenOutcome::Leased { token_leased: crate::token::TokenLeased { actor: input.actor.clone(), token_id: input.token_id.clone(), instance: input.instance.clone(), owner: before.owner.clone(), seat: before.seat.clone(), agent: input.agent.clone(), preferred_owner: input.preferred_owner.clone(), leased_at: input.leased_at.clone() } };
        PoolTokenStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.token.MarkTokenLimited`, generated: every outcome is one the specification fully determines.
impl<P> crate::token::obligations::MarkTokenLimitedBehavior for Generated<P>
where
    P: PoolTokenStorage,
{
    fn mark_token_limited(&mut self, input: crate::token::MarkTokenLimited) -> Result<crate::token::MarkTokenLimitedOutcome, UnmetObligation> {
        let _ = &input;
        // `limited`: the default.
        let Some(held) = PoolTokenStorage::get(&self.ports, &input.token_id) else {
            return Ok(crate::token::MarkTokenLimitedOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::token::AnyPoolToken::Leased(instance) => crate::token::AnyPoolToken::Limited(instance.limit()),
            _ => return Ok(crate::token::MarkTokenLimitedOutcome::WrongState { error: crate::token::TokenStateConflict { token_id: input.token_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.token_state = crate::token::TokenState::Limited;
        next.data.limited_until = Some(input.limited_until.clone());
        next.data.agent = None;
        next.data.actor = input.actor.clone();
        let answer = crate::token::MarkTokenLimitedOutcome::Limited { token_limited: crate::token::TokenLimited { actor: input.actor.clone(), token_id: input.token_id.clone(), owner: before.owner.clone(), agent: before.agent.clone(), limited_until: input.limited_until.clone(), limited_at: input.limited_at.clone() } };
        PoolTokenStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.token.ReleaseToken`, generated: every outcome is one the specification fully determines.
impl<P> crate::token::obligations::ReleaseTokenBehavior for Generated<P>
where
    P: PoolTokenStorage,
{
    fn release_token(&mut self, input: crate::token::ReleaseToken) -> Result<crate::token::ReleaseTokenOutcome, UnmetObligation> {
        let _ = &input;
        // `released`: the default.
        let Some(held) = PoolTokenStorage::get(&self.ports, &input.token_id) else {
            return Ok(crate::token::ReleaseTokenOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::token::AnyPoolToken::Leased(instance) => crate::token::AnyPoolToken::Available(instance.release()),
            _ => return Ok(crate::token::ReleaseTokenOutcome::WrongState { error: crate::token::TokenStateConflict { token_id: input.token_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.agent = None;
        next.data.actor = input.actor.clone();
        let answer = crate::token::ReleaseTokenOutcome::Released { token_released: crate::token::TokenReleased { actor: input.actor.clone(), token_id: input.token_id.clone(), agent: before.agent.clone(), released_at: input.released_at.clone() } };
        PoolTokenStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.token.WithdrawToken`, generated: every outcome is one the specification fully determines.
impl<P> crate::token::obligations::WithdrawTokenBehavior for Generated<P>
where
    P: TryContext + PoolTokenStorage,
{
    fn withdraw_token(&mut self, input: crate::token::WithdrawToken) -> Result<crate::token::WithdrawTokenOutcome, UnmetObligation> {
        let _ = &input;
        // `actor-not-human`: an input-guarded refusal, before the addressed subject is loaded.
        if decided(equal(Some(&input.actor).map(|value| &value.kind).map(|value| match value { crate::operator::ActorKind::ControlPlane => "ControlPlane", crate::operator::ActorKind::Human => "Human", crate::operator::ActorKind::Agent => "Agent", crate::operator::ActorKind::Daemon => "Daemon" }.to_owned()), Some("Human".to_owned())).map(|value| !value), "conductor.token.WithdrawToken")? {
            return Ok(crate::token::WithdrawTokenOutcome::ActorNotHuman { error: crate::operator::ActorNotHuman { actor: input.actor.clone() } });
        }
        // `not-owner`: an external branch, where the context takes it.
        if self.ports.try_external(ExternalCommand::ConductorTokenWithdrawToken(&input), "not-owner")? {
            return Ok(crate::token::WithdrawTokenOutcome::NotOwner { error: crate::token::NotTokenOwner { token_id: input.token_id.clone() } });
        }
        // `withdrawn`: the default.
        let Some(held) = PoolTokenStorage::get(&self.ports, &input.token_id) else {
            return Ok(crate::token::WithdrawTokenOutcome::WrongStateUnknownInstance);
        };
        let _ = &held;
        let before = held.data.clone();
        let moved = match held.refine() {
            crate::token::AnyPoolToken::Available(instance) => crate::token::AnyPoolToken::Withdrawn(instance.withdraw()),
            crate::token::AnyPoolToken::Leased(instance) => crate::token::AnyPoolToken::Withdrawn(instance.withdraw()),
            crate::token::AnyPoolToken::Limited(instance) => crate::token::AnyPoolToken::Withdrawn(instance.withdraw()),
            _ => return Ok(crate::token::WithdrawTokenOutcome::WrongState { error: crate::token::TokenStateConflict { token_id: input.token_id.clone() } }),
        };
        let mut next = moved.snapshot();
        next.data.agent = None;
        next.data.actor = input.actor.clone();
        let answer = crate::token::WithdrawTokenOutcome::Withdrawn { token_withdrawn: crate::token::TokenWithdrawn { actor: input.actor.clone(), token_id: input.token_id.clone(), owner: before.owner.clone(), withdrawn_at: input.withdrawn_at.clone() } };
        PoolTokenStorage::put(&mut self.ports, next);
        return Ok(answer);
    }
}

/// `conductor.tool.RecordToolCall`, generated: every outcome is one the specification fully determines.
impl<P> crate::tool::obligations::RecordToolCallBehavior for Generated<P>
where
    P: TryContext + ToolCallStorage,
{
    fn record_tool_call(&mut self, input: crate::tool::RecordToolCall) -> Result<crate::tool::RecordToolCallOutcome, UnmetObligation> {
        let _ = &input;
        // `recorded`: the default.
        let identity: crate::tool::ToolCallId = self.ports.try_generate_conductor_tool_tool_call_id()?;
        let data = crate::tool::ToolCallData {
            tool_call_id: identity.clone(),
            actor: input.actor.clone(),
            instance: input.instance.clone(),
            session: input.session.clone(),
            verb: input.verb.clone(),
            paths: input.paths.clone(),
            outcome: input.outcome.clone(),
            rule: input.rule.clone(),
            duration_ms: input.duration_ms.clone(),
            output_bytes: input.output_bytes.clone(),
            called_at: input.called_at.clone(),
        };
        if let Some(broken) = data.broken_invariant() {
            let capability = "entity invariant";
            return Err(UnmetObligation { capability, source: broken });
        }
        let answer = crate::tool::RecordToolCallOutcome::Recorded { tool_call_recorded: crate::tool::ToolCallRecorded { actor: input.actor.clone(), tool_call_id: identity.clone(), session: input.session.clone(), verb: input.verb.clone(), outcome: input.outcome.clone() } };
        ToolCallStorage::put(&mut self.ports, crate::tool::AnyToolCall::Recorded(crate::tool::ToolCall::new(data)).snapshot());
        return Ok(answer);
    }
}

/// `conductor.agent.AgentKinds`, generated: every row is one the specification fully determines from the stored `conductor.agent.AgentKind`s.
impl<P> crate::agent::obligations::AgentKindsQuery for Generated<P>
where
    P: AgentKindStorage,
{
    fn agent_kinds(&self) -> Result<Vec<crate::agent::AgentKinds>, UnmetObligation> {
        let admitted = AgentKindStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::agent::AgentKinds {
                name: held.data.name,
                state: held.state,
                capabilities: held.data.capabilities,
                binary: held.data.binary,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.agent.Agents`, generated: every row is one the specification fully determines from the stored `conductor.agent.Agent`s.
impl<P> crate::agent::obligations::AgentsQuery for Generated<P>
where
    P: AgentStorage,
{
    fn agents(&self) -> Result<Vec<crate::agent::Agents>, UnmetObligation> {
        let admitted = AgentStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::agent::Agents {
                name: held.data.name,
                state: held.state,
                instance: held.data.instance,
                role: held.data.role,
                kind: held.data.kind,
                capabilities: held.data.capabilities,
                r#box: held.data.r#box,
                profile: held.data.profile,
                limits: held.data.limits,
                token_owner: held.data.token_owner,
                started_by: held.data.started_by,
                started_at: held.data.started_at,
                generation: held.data.generation,
                stopped_at: held.data.stopped_at,
                exit_class: held.data.exit_class,
                exit_code: held.data.exit_code,
            })
            .collect())
    }
}

/// `conductor.config.ConfigRevisions`, generated: every row is one the specification fully determines from the stored `conductor.config.ConfigRevision`s.
impl<P> crate::config::obligations::ConfigRevisionsQuery for Generated<P>
where
    P: ConfigRevisionStorage,
{
    fn config_revisions(&self) -> Result<Vec<crate::config::ConfigRevisions>, UnmetObligation> {
        let admitted = ConfigRevisionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::config::ConfigRevisions {
                revision_id: held.data.revision_id,
                state: held.state,
                actor: held.data.actor,
                level: held.data.level,
                instance: held.data.instance,
                role: held.data.role,
                path: held.data.path,
                value: held.data.value,
                reason: held.data.reason,
                commit: held.data.commit,
                changed_at: held.data.changed_at,
            })
            .collect())
    }
}

impl<P: crate::config::obligations::EffectiveSettingsQuery> crate::config::obligations::EffectiveSettingsQuery for Generated<P> {
    fn effective_settings(&self, instance: crate::config::InstanceName) -> Result<Vec<crate::config::EffectiveSettings>, UnmetObligation> {
        crate::config::obligations::EffectiveSettingsQuery::effective_settings(&self.ports, instance)
    }
}

/// `conductor.decision.Decisions`, generated: every row is one the specification fully determines from the stored `conductor.decision.Decision`s.
impl<P> crate::decision::obligations::DecisionsQuery for Generated<P>
where
    P: DecisionStorage,
{
    fn decisions(&self) -> Result<Vec<crate::decision::Decisions>, UnmetObligation> {
        let admitted = DecisionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::decision::Decisions {
                decision_id: held.data.decision_id,
                state: held.state,
                decision_class: held.data.decision_class,
                decided_by: held.data.decided_by,
                choice: held.data.choice,
                question: held.data.question,
                options: held.data.options,
                reason: held.data.reason,
                evidence: held.data.evidence,
                decided_at: held.data.decided_at,
                recorded_by: held.data.recorded_by,
                operator_words: held.data.operator_words,
                repository: held.data.repository,
                decider_name: held.data.decider_name,
                prompt_sha256: held.data.prompt_sha256,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.decision.HandsTodo`, generated: every row is one the specification fully determines from the stored `conductor.decision.DecisionRequest`s.
impl<P> crate::decision::obligations::HandsTodoQuery for Generated<P>
where
    P: DecisionRequestStorage,
{
    fn hands_todo(&self) -> Result<Vec<crate::decision::HandsTodo>, UnmetObligation> {
        let mut admitted = DecisionRequestStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| all(&[equal(Some(&held.data.escalation_class).and_then(|value| value.as_ref()).map(|value| match value { crate::decision::DecisionClass::C => "C", crate::decision::DecisionClass::O => "O", crate::decision::DecisionClass::H => "H" }.to_owned()), Some("H".to_owned())), equal(Some(&held.state).map(|value| match value { crate::decision::DecisionRequestState::Answered => "Answered", crate::decision::DecisionRequestState::Escalated => "Escalated", crate::decision::DecisionRequestState::Open => "Open", crate::decision::DecisionRequestState::Withdrawn => "Withdrawn" }.to_owned()), Some("Escalated".to_owned()))]) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::decision::HandsTodo {
                escalation_class: held.data.escalation_class,
                escalation_reason: held.data.escalation_reason,
                request_id: held.data.request_id,
                state: held.state,
                repository: held.data.repository,
                decision_id: held.data.decision_id,
                question: held.data.question,
                options: held.data.options,
                recommendation: held.data.recommendation,
                raised_at: held.data.raised_at,
                blocker: held.data.blocker,
                dry_run: held.data.dry_run,
                no_dry_run: held.data.no_dry_run,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.decision.Requests`, generated: every row is one the specification fully determines from the stored `conductor.decision.DecisionRequest`s.
impl<P> crate::decision::obligations::RequestsQuery for Generated<P>
where
    P: DecisionRequestStorage,
{
    fn requests(&self) -> Result<Vec<crate::decision::Requests>, UnmetObligation> {
        let admitted = DecisionRequestStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::decision::Requests {
                escalation_class: held.data.escalation_class,
                escalation_reason: held.data.escalation_reason,
                request_id: held.data.request_id,
                state: held.state,
                repository: held.data.repository,
                decision_id: held.data.decision_id,
                question: held.data.question,
                options: held.data.options,
                recommendation: held.data.recommendation,
                raised_at: held.data.raised_at,
                blocker: held.data.blocker,
                classification_rule: held.data.classification_rule,
                kind: held.data.kind,
                lettered_options: held.data.lettered_options,
                recommended_letters: held.data.recommended_letters,
                default_letters: held.data.default_letters,
                deadline: held.data.deadline,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.direction.GoalServings`, generated: every row is one the specification fully determines from the stored `conductor.direction.GoalServing`s.
impl<P> crate::direction::obligations::GoalServingsQuery for Generated<P>
where
    P: GoalServingStorage,
{
    fn goal_servings(&self) -> Result<Vec<crate::direction::GoalServings>, UnmetObligation> {
        let admitted = GoalServingStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::direction::GoalServings {
                serving_id: held.data.serving_id,
                state: held.state,
                goal_id: held.data.goal_id,
                repository: held.data.repository,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.direction.Goals`, generated: every row is one the specification fully determines from the stored `conductor.direction.Goal`s.
impl<P> crate::direction::obligations::GoalsQuery for Generated<P>
where
    P: GoalStorage,
{
    fn goals(&self) -> Result<Vec<crate::direction::Goals>, UnmetObligation> {
        let admitted = GoalStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::direction::Goals {
                goal_id: held.data.goal_id,
                state: held.state,
                title: held.data.title,
                exit_evidence: held.data.exit_evidence,
                evidence: held.data.evidence,
                reason: held.data.reason,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.direction.RepositoryMarks`, generated: every row is one the specification fully determines from the stored `conductor.direction.RepositoryMark`s.
impl<P> crate::direction::obligations::RepositoryMarksQuery for Generated<P>
where
    P: RepositoryMarkStorage,
{
    fn repository_marks(&self) -> Result<Vec<crate::direction::RepositoryMarks>, UnmetObligation> {
        let admitted = RepositoryMarkStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::direction::RepositoryMarks {
                repository: held.data.repository,
                state: held.state,
                marked_by: held.data.marked_by,
                session: held.data.session,
                reason: held.data.reason,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.dispatch.Controllers`, generated: every row is one the specification fully determines from the stored `conductor.dispatch.Controller`s.
impl<P> crate::dispatch::obligations::ControllersQuery for Generated<P>
where
    P: ControllerStorage,
{
    fn controllers(&self) -> Result<Vec<crate::dispatch::Controllers>, UnmetObligation> {
        let admitted = ControllerStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::dispatch::Controllers {
                controller_id: held.data.controller_id,
                state: held.state,
                repository: held.data.repository,
                harness: held.data.harness,
                session_name: held.data.session_name,
                charter_revision: held.data.charter_revision,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.dispatch.Dispatches`, generated: every row is one the specification fully determines from the stored `conductor.dispatch.Dispatch`s.
impl<P> crate::dispatch::obligations::DispatchesQuery for Generated<P>
where
    P: DispatchStorage,
{
    fn dispatches(&self) -> Result<Vec<crate::dispatch::Dispatches>, UnmetObligation> {
        let admitted = DispatchStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::dispatch::Dispatches {
                dispatch_id: held.data.dispatch_id,
                state: held.state,
                repository: held.data.repository,
                goal_id: held.data.goal_id,
                brief: held.data.brief,
                sent_at: held.data.sent_at,
                priority: held.data.priority,
                waits_on: held.data.waits_on,
                actor: held.data.actor,
                reference: held.data.reference,
                start_ref: held.data.start_ref,
                issue: held.data.issue,
                links: held.data.links,
            })
            .collect())
    }
}

/// `conductor.dispatch.GuardDecisions`, generated: every row is one the specification fully determines from the stored `conductor.dispatch.GuardDecision`s.
impl<P> crate::dispatch::obligations::GuardDecisionsQuery for Generated<P>
where
    P: GuardDecisionStorage,
{
    fn guard_decisions(&self) -> Result<Vec<crate::dispatch::GuardDecisions>, UnmetObligation> {
        let admitted = GuardDecisionStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::dispatch::GuardDecisions {
                guard_decision_id: held.data.guard_decision_id,
                repository: held.data.repository,
                session_ref: held.data.session_ref,
                tool: held.data.tool,
                target: held.data.target,
                verdict: held.data.verdict,
                reason: held.data.reason,
                decided_at: held.data.decided_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.dispatch.Messages`, generated: every row is one the specification fully determines from the stored `conductor.dispatch.Message`s.
impl<P> crate::dispatch::obligations::MessagesQuery for Generated<P>
where
    P: MessageStorage,
{
    fn messages(&self) -> Result<Vec<crate::dispatch::Messages>, UnmetObligation> {
        let admitted = MessageStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::dispatch::Messages {
                message_id: held.data.message_id,
                state: held.state,
                sender: held.data.sender,
                recipient: held.data.recipient,
                kind: held.data.kind,
                first_line: held.data.first_line,
                transport: held.data.transport,
                received_at: held.data.received_at,
                body_path: held.data.body_path,
                dispatch_id: held.data.dispatch_id,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.dispatch.ResourceRequests`, generated: every row is one the specification fully determines from the stored `conductor.dispatch.ResourceRequest`s.
impl<P> crate::dispatch::obligations::ResourceRequestsQuery for Generated<P>
where
    P: ResourceRequestStorage,
{
    fn resource_requests(&self) -> Result<Vec<crate::dispatch::ResourceRequests>, UnmetObligation> {
        let admitted = ResourceRequestStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::dispatch::ResourceRequests {
                resource_request_id: held.data.resource_request_id,
                state: held.state,
                repository: held.data.repository,
                resource: held.data.resource,
                amount: held.data.amount,
                requested_at: held.data.requested_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.host.Grants`, generated: every row is one the specification fully determines from the stored `conductor.host.HostGrant`s.
impl<P> crate::host::obligations::GrantsQuery for Generated<P>
where
    P: HostGrantStorage,
{
    fn grants(&self) -> Result<Vec<crate::host::Grants>, UnmetObligation> {
        let admitted = HostGrantStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::host::Grants {
                grant_id: held.data.grant_id,
                state: held.state,
                instance: held.data.instance,
                repository: held.data.repository,
                kind: held.data.kind,
                bytes: held.data.bytes,
                peak_bytes: held.data.peak_bytes,
                pauses: held.data.pauses,
                command: held.data.command,
                process_group: held.data.process_group,
                session_name: held.data.session_name,
                session_id: held.data.session_id,
                reclaimed_bytes: held.data.reclaimed_bytes,
                shortfall_bytes: held.data.shortfall_bytes,
                actor: held.data.actor,
                requested_at: held.data.requested_at,
                started_at: held.data.started_at,
                released_at: held.data.released_at,
                release_reason: held.data.release_reason,
                cache_hits: held.data.cache_hits,
                cache_misses: held.data.cache_misses,
                lane: held.data.lane,
                lane_overrun: held.data.lane_overrun,
            })
            .collect())
    }
}

/// `conductor.host.ReclaimRuns`, generated: every row is one the specification fully determines from the stored `conductor.host.ReclaimRun`s.
impl<P> crate::host::obligations::ReclaimRunsQuery for Generated<P>
where
    P: ReclaimRunStorage,
{
    fn reclaim_runs(&self) -> Result<Vec<crate::host::ReclaimRuns>, UnmetObligation> {
        let admitted = ReclaimRunStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::host::ReclaimRuns {
                reclaim_run_id: held.data.reclaim_run_id,
                state: held.state,
                instance: held.data.instance,
                result: held.data.result,
                steps: held.data.steps,
                reclaimed_bytes: held.data.reclaimed_bytes,
                shortfall_bytes: held.data.shortfall_bytes,
                left: held.data.left,
                actor: held.data.actor,
                ran_at: held.data.ran_at,
            })
            .collect())
    }
}

/// `conductor.improvement.Findings`, generated: every row is one the specification fully determines from the stored `conductor.improvement.FrictionFinding`s.
impl<P> crate::improvement::obligations::FindingsQuery for Generated<P>
where
    P: FrictionFindingStorage,
{
    fn findings(&self) -> Result<Vec<crate::improvement::Findings>, UnmetObligation> {
        let admitted = FrictionFindingStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::improvement::Findings {
                finding_id: held.data.finding_id,
                state: held.state,
                signature: held.data.signature,
                evidence: held.data.evidence,
                metric_id: held.data.metric_id,
                raised_at: held.data.raised_at,
                actor: held.data.actor,
                story: held.data.story,
                verdict: held.data.verdict,
            })
            .collect())
    }
}

/// `conductor.improvement.KpiReadings`, generated: every row is one the specification fully determines from the stored `conductor.improvement.MetricReading`s.
impl<P> crate::improvement::obligations::KpiReadingsQuery for Generated<P>
where
    P: MetricReadingStorage,
{
    fn kpi_readings(&self) -> Result<Vec<crate::improvement::KpiReadings>, UnmetObligation> {
        let mut admitted = MetricReadingStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| all(&[equal(Some(&held.state).map(|value| match value { crate::improvement::MetricReadingState::Recorded => "Recorded", crate::improvement::MetricReadingState::Superseded => "Superseded" }.to_owned()), Some("Recorded".to_owned())), equal(Some(&held.data.scope).map(|value| value.clone()), Some("instance".to_owned())), Some(Some(&held.data.target).and_then(|value| value.as_ref()).is_some())]) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::improvement::KpiReadings {
                metric_id: held.data.metric_id,
                scope: held.data.scope,
                day: held.data.day,
                value: held.data.value,
                target: held.data.target,
                met: held.data.met,
                reading_id: held.data.reading_id,
            })
            .collect())
    }
}

/// `conductor.improvement.Kpis`, generated: every row is one the specification fully determines from the stored `conductor.improvement.Metric`s.
impl<P> crate::improvement::obligations::KpisQuery for Generated<P>
where
    P: MetricStorage,
{
    fn kpis(&self) -> Result<Vec<crate::improvement::Kpis>, UnmetObligation> {
        let mut admitted = MetricStorage::list(&self.ports);
        // `filter:` shows a row where it holds; false or unknown hides it.
        admitted.retain(|held| equal(Some(&held.state).map(|value| match value { crate::improvement::MetricState::Targeted => "Targeted", crate::improvement::MetricState::Untargeted => "Untargeted" }.to_owned()), Some("Targeted".to_owned())) == Some(true));
        Ok(admitted
            .into_iter()
            .map(|held| crate::improvement::Kpis {
                metric_id: held.data.metric_id,
                state: held.state,
                name: held.data.name,
                unit: held.data.unit,
                instrument: held.data.instrument,
                baseline: held.data.baseline,
                target: held.data.target,
                direction: held.data.direction,
                target_owner: held.data.target_owner,
                target_set_at: held.data.target_set_at,
            })
            .collect())
    }
}

/// `conductor.improvement.MetricReadings`, generated: every row is one the specification fully determines from the stored `conductor.improvement.MetricReading`s.
impl<P> crate::improvement::obligations::MetricReadingsQuery for Generated<P>
where
    P: MetricReadingStorage,
{
    fn metric_readings(&self) -> Result<Vec<crate::improvement::MetricReadings>, UnmetObligation> {
        let admitted = MetricReadingStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::improvement::MetricReadings {
                reading_id: held.data.reading_id,
                state: held.state,
                metric_id: held.data.metric_id,
                day: held.data.day,
                scope: held.data.scope,
                value: held.data.value,
                sources: held.data.sources,
                target: held.data.target,
                met: held.data.met,
                recorded_at: held.data.recorded_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.improvement.Metrics`, generated: every row is one the specification fully determines from the stored `conductor.improvement.Metric`s.
impl<P> crate::improvement::obligations::MetricsQuery for Generated<P>
where
    P: MetricStorage,
{
    fn metrics(&self) -> Result<Vec<crate::improvement::Metrics>, UnmetObligation> {
        let admitted = MetricStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::improvement::Metrics {
                metric_id: held.data.metric_id,
                state: held.state,
                name: held.data.name,
                unit: held.data.unit,
                instrument: held.data.instrument,
                definition: held.data.definition,
                baseline: held.data.baseline,
                baseline_period: held.data.baseline_period,
                target: held.data.target,
                direction: held.data.direction,
                target_owner: held.data.target_owner,
                target_set_at: held.data.target_set_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.improvement.SettingsProposals`, generated: every row is one the specification fully determines from the stored `conductor.improvement.SettingsProposal`s.
impl<P> crate::improvement::obligations::SettingsProposalsQuery for Generated<P>
where
    P: SettingsProposalStorage,
{
    fn settings_proposals(&self) -> Result<Vec<crate::improvement::SettingsProposals>, UnmetObligation> {
        let admitted = SettingsProposalStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::improvement::SettingsProposals {
                proposal_id: held.data.proposal_id,
                state: held.state,
                instance: held.data.instance,
                key: held.data.key,
                old: held.data.old,
                new: held.data.new,
                metric_id: held.data.metric_id,
                reading: held.data.reading,
                baseline: held.data.baseline,
                window_days: held.data.window_days,
                proposed_at: held.data.proposed_at,
                revision: held.data.revision,
                reason: held.data.reason,
                finding: held.data.finding,
                verdict: held.data.verdict,
            })
            .collect())
    }
}

impl<P: crate::instance::obligations::InstanceByNameQuery> crate::instance::obligations::InstanceByNameQuery for Generated<P> {
    fn instance_by_name(&self, name: crate::config::InstanceName) -> Result<Vec<crate::instance::InstanceByName>, UnmetObligation> {
        crate::instance::obligations::InstanceByNameQuery::instance_by_name(&self.ports, name)
    }
}

/// `conductor.instance.Instances`, generated: every row is one the specification fully determines from the stored `conductor.instance.Instance`s.
impl<P> crate::instance::obligations::InstancesQuery for Generated<P>
where
    P: InstanceStorage,
{
    fn instances(&self) -> Result<Vec<crate::instance::Instances>, UnmetObligation> {
        let admitted = InstanceStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::instance::Instances {
                name: held.data.name,
                state: held.state,
                placement: held.data.placement,
                roles: held.data.roles,
                binary_version: held.data.binary_version,
                last_seen: held.data.last_seen,
                box_id: held.data.box_id,
                sources: held.data.sources,
                detail: held.data.detail,
                changed_by: held.data.changed_by,
            })
            .collect())
    }
}

/// `conductor.instance.Reconciliations`, generated: every row is one the specification fully determines from the stored `conductor.instance.Reconciliation`s.
impl<P> crate::instance::obligations::ReconciliationsQuery for Generated<P>
where
    P: ReconciliationStorage,
{
    fn reconciliations(&self) -> Result<Vec<crate::instance::Reconciliations>, UnmetObligation> {
        let admitted = ReconciliationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::instance::Reconciliations {
                reconciliation_id: held.data.reconciliation_id,
                state: held.state,
                instance: held.data.instance,
                action: held.data.action,
                reason: held.data.reason,
                result: held.data.result,
                detail: held.data.detail,
                actor: held.data.actor,
                at: held.data.at,
            })
            .collect())
    }
}

/// `conductor.observation.Blockers`, generated: every row is one the specification fully determines from the stored `conductor.observation.BlockerObservation`s.
impl<P> crate::observation::obligations::BlockersQuery for Generated<P>
where
    P: BlockerObservationStorage,
{
    fn blockers(&self) -> Result<Vec<crate::observation::Blockers>, UnmetObligation> {
        let admitted = BlockerObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::Blockers {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                repository: held.data.repository,
                reference: held.data.reference,
                blocker_kind: held.data.blocker_kind,
                title: held.data.title,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.Boards`, generated: every row is one the specification fully determines from the stored `conductor.observation.Board`s.
impl<P> crate::observation::obligations::BoardsQuery for Generated<P>
where
    P: BoardStorage,
{
    fn boards(&self) -> Result<Vec<crate::observation::Boards>, UnmetObligation> {
        let admitted = BoardStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::Boards {
                board_id: held.data.board_id,
                snapshot_id: held.data.snapshot_id,
                previous_snapshot_id: held.data.previous_snapshot_id,
                deviation_rows: held.data.deviation_rows,
                path: held.data.path,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.CollectorRuns`, generated: every row is one the specification fully determines from the stored `conductor.observation.CollectorRunObservation`s.
impl<P> crate::observation::obligations::CollectorRunsQuery for Generated<P>
where
    P: CollectorRunObservationStorage,
{
    fn collector_runs(&self) -> Result<Vec<crate::observation::CollectorRuns>, UnmetObligation> {
        let admitted = CollectorRunObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::CollectorRuns {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                collector: held.data.collector,
                started_at: held.data.started_at,
                duration_ms: held.data.duration_ms,
                outcome: held.data.outcome,
                read_bytes: held.data.read_bytes,
                written_bytes: held.data.written_bytes,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.MemberFailures`, generated: every row is one the specification fully determines from the stored `conductor.observation.MemberFailureObservation`s.
impl<P> crate::observation::obligations::MemberFailuresQuery for Generated<P>
where
    P: MemberFailureObservationStorage,
{
    fn member_failures(&self) -> Result<Vec<crate::observation::MemberFailures>, UnmetObligation> {
        let admitted = MemberFailureObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::MemberFailures {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                member: held.data.member,
                collector: held.data.collector,
                error_line: held.data.error_line,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.MergedPullRequests`, generated: every row is one the specification fully determines from the stored `conductor.observation.MergedPullRequestObservation`s.
impl<P> crate::observation::obligations::MergedPullRequestsQuery for Generated<P>
where
    P: MergedPullRequestObservationStorage,
{
    fn merged_pull_requests(&self) -> Result<Vec<crate::observation::MergedPullRequests>, UnmetObligation> {
        let admitted = MergedPullRequestObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::MergedPullRequests {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                repository: held.data.repository,
                number: held.data.number,
                title: held.data.title,
                merged_at: held.data.merged_at,
                url: held.data.url,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.PullRequests`, generated: every row is one the specification fully determines from the stored `conductor.observation.PullRequestObservation`s.
impl<P> crate::observation::obligations::PullRequestsQuery for Generated<P>
where
    P: PullRequestObservationStorage,
{
    fn pull_requests(&self) -> Result<Vec<crate::observation::PullRequests>, UnmetObligation> {
        let admitted = PullRequestObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::PullRequests {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                repository: held.data.repository,
                number: held.data.number,
                title: held.data.title,
                draft: held.data.draft,
                mergeability: held.data.mergeability,
                failing_checks: held.data.failing_checks,
                failing_check_names: held.data.failing_check_names,
                opened_at: held.data.opened_at,
                updated_at: held.data.updated_at,
                url: held.data.url,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.Releases`, generated: every row is one the specification fully determines from the stored `conductor.observation.ReleaseObservation`s.
impl<P> crate::observation::obligations::ReleasesQuery for Generated<P>
where
    P: ReleaseObservationStorage,
{
    fn releases(&self) -> Result<Vec<crate::observation::Releases>, UnmetObligation> {
        let admitted = ReleaseObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::Releases {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                repository: held.data.repository,
                tag: held.data.tag,
                published_at: held.data.published_at,
                url: held.data.url,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.Repositories`, generated: every row is one the specification fully determines from the stored `conductor.observation.RepositoryObservation`s.
impl<P> crate::observation::obligations::RepositoriesQuery for Generated<P>
where
    P: RepositoryObservationStorage,
{
    fn repositories(&self) -> Result<Vec<crate::observation::Repositories>, UnmetObligation> {
        let admitted = RepositoryObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::Repositories {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                repository: held.data.repository,
                visibility: held.data.visibility,
                archived: held.data.archived,
                local_checkout: held.data.local_checkout,
                main_head: held.data.main_head,
                main_committed_at: held.data.main_committed_at,
                main_commits_7d: held.data.main_commits_7d,
                real_commits_7d: held.data.real_commits_7d,
                behind_main: held.data.behind_main,
                dirty_files: held.data.dirty_files,
                worktrees: held.data.worktrees,
                open_issues: held.data.open_issues,
                oldest_open_issue_at: held.data.oldest_open_issue_at,
                latest_release: held.data.latest_release,
                latest_release_at: held.data.latest_release_at,
                unreleased_commits: held.data.unreleased_commits,
                planning_store_version: held.data.planning_store_version,
                in_catalog: held.data.in_catalog,
                gates_hooks_version: held.data.gates_hooks_version,
                gates_version: held.data.gates_version,
                wave_width: held.data.wave_width,
                wave_ready: held.data.wave_ready,
                wave_reason: held.data.wave_reason,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.SecretSyncs`, generated: every row is one the specification fully determines from the stored `conductor.observation.SecretSyncObservation`s.
impl<P> crate::observation::obligations::SecretSyncsQuery for Generated<P>
where
    P: SecretSyncObservationStorage,
{
    fn secret_syncs(&self) -> Result<Vec<crate::observation::SecretSyncs>, UnmetObligation> {
        let admitted = SecretSyncObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::SecretSyncs {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                sync: held.data.sync,
                repository: held.data.repository,
                branch: held.data.branch,
                path: held.data.path,
                source_committed_at: held.data.source_committed_at,
                secrets_updated_at: held.data.secrets_updated_at,
                error_line: held.data.error_line,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.Sessions`, generated: every row is one the specification fully determines from the stored `conductor.observation.SessionObservation`s.
impl<P> crate::observation::obligations::SessionsQuery for Generated<P>
where
    P: SessionObservationStorage,
{
    fn sessions(&self) -> Result<Vec<crate::observation::Sessions>, UnmetObligation> {
        let admitted = SessionObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::Sessions {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                harness: held.data.harness,
                session_ref: held.data.session_ref,
                name: held.data.name,
                cwd: held.data.cwd,
                repository: held.data.repository,
                role: held.data.role,
                activity: held.data.activity,
                envelope_provider: held.data.envelope_provider,
                envelope_cgroup: held.data.envelope_cgroup,
                envelope_memory_max: held.data.envelope_memory_max,
                envelope_cpu_weight: held.data.envelope_cpu_weight,
                envelope_tasks_max: held.data.envelope_tasks_max,
                envelope_writable: held.data.envelope_writable,
                envelope_unboxed: held.data.envelope_unboxed,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.Snapshots`, generated: every row is one the specification fully determines from the stored `conductor.observation.Snapshot`s.
impl<P> crate::observation::obligations::SnapshotsQuery for Generated<P>
where
    P: SnapshotStorage,
{
    fn snapshots(&self) -> Result<Vec<crate::observation::Snapshots>, UnmetObligation> {
        let admitted = SnapshotStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::Snapshots {
                snapshot_id: held.data.snapshot_id,
                state: held.state,
                started_at: held.data.started_at,
                disk_free_bytes: held.data.disk_free_bytes,
                failure_reason: held.data.failure_reason,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.Specifications`, generated: every row is one the specification fully determines from the stored `conductor.observation.SpecificationObservation`s.
impl<P> crate::observation::obligations::SpecificationsQuery for Generated<P>
where
    P: SpecificationObservationStorage,
{
    fn specifications(&self) -> Result<Vec<crate::observation::Specifications>, UnmetObligation> {
        let admitted = SpecificationObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::Specifications {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                repository: held.data.repository,
                presence: held.data.presence,
                path: held.data.path,
                format: held.data.format,
                required_ess: held.data.required_ess,
                validation: held.data.validation,
                validation_refusals: held.data.validation_refusals,
                scenarios: held.data.scenarios,
                synthesis_refusals: held.data.synthesis_refusals,
                conformance_status: held.data.conformance_status,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.UsageReadings`, generated: every row is one the specification fully determines from the stored `conductor.observation.UsageObservation`s.
impl<P> crate::observation::obligations::UsageReadingsQuery for Generated<P>
where
    P: UsageObservationStorage,
{
    fn usage_readings(&self) -> Result<Vec<crate::observation::UsageReadings>, UnmetObligation> {
        let admitted = UsageObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::UsageReadings {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                read_at: held.data.read_at,
                account: held.data.account,
                active: held.data.active,
                health: held.data.health,
                window: held.data.window,
                used_percent: held.data.used_percent,
                resets_at: held.data.resets_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.observation.WorkflowRuns`, generated: every row is one the specification fully determines from the stored `conductor.observation.WorkflowRunObservation`s.
impl<P> crate::observation::obligations::WorkflowRunsQuery for Generated<P>
where
    P: WorkflowRunObservationStorage,
{
    fn workflow_runs(&self) -> Result<Vec<crate::observation::WorkflowRuns>, UnmetObligation> {
        let admitted = WorkflowRunObservationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::observation::WorkflowRuns {
                observation_id: held.data.observation_id,
                snapshot_id: held.data.snapshot_id,
                repository: held.data.repository,
                workflow: held.data.workflow,
                conclusion: held.data.conclusion,
                run_at: held.data.run_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.operator.Operators`, generated: every row is one the specification fully determines from the stored `conductor.operator.Operator`s.
impl<P> crate::operator::obligations::OperatorsQuery for Generated<P>
where
    P: OperatorStorage,
{
    fn operators(&self) -> Result<Vec<crate::operator::Operators>, UnmetObligation> {
        let admitted = OperatorStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::operator::Operators {
                key: held.data.key,
                state: held.state,
                grants: held.data.grants,
                joined_at: held.data.joined_at,
                joined_by: held.data.joined_by,
                left_at: held.data.left_at,
                left_by: held.data.left_by,
            })
            .collect())
    }
}

/// `conductor.placement.BoxRefusals`, generated: every row is one the specification fully determines from the stored `conductor.placement.BoxRefusal`s.
impl<P> crate::placement::obligations::BoxRefusalsQuery for Generated<P>
where
    P: BoxRefusalStorage,
{
    fn box_refusals(&self) -> Result<Vec<crate::placement::BoxRefusals>, UnmetObligation> {
        let admitted = BoxRefusalStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::placement::BoxRefusals {
                box_id: held.data.box_id,
                state: held.state,
                placement: held.data.placement,
                instance: held.data.instance,
                actor: held.data.actor,
                reason: held.data.reason,
                refused_at: held.data.refused_at,
            })
            .collect())
    }
}

/// `conductor.placement.Boxes`, generated: every row is one the specification fully determines from the stored `conductor.placement.Box`s.
impl<P> crate::placement::obligations::BoxesQuery for Generated<P>
where
    P: BoxStorage,
{
    fn boxes(&self) -> Result<Vec<crate::placement::Boxes>, UnmetObligation> {
        let admitted = BoxStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::placement::Boxes {
                box_id: held.data.box_id,
                state: held.state,
                placement: held.data.placement,
                instance: held.data.instance,
                actor: held.data.actor,
                opened_at: held.data.opened_at,
                closed_at: held.data.closed_at,
                closed_by: held.data.closed_by,
            })
            .collect())
    }
}

/// `conductor.placement.Placements`, generated: every row is one the specification fully determines from the stored `conductor.placement.Placement`s.
impl<P> crate::placement::obligations::PlacementsQuery for Generated<P>
where
    P: PlacementStorage,
{
    fn placements(&self) -> Result<Vec<crate::placement::Placements>, UnmetObligation> {
        let admitted = PlacementStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::placement::Placements {
                name: held.data.name,
                state: held.state,
                daemon: held.data.daemon,
                adapter: held.data.adapter,
                profile: held.data.profile,
            })
            .collect())
    }
}

impl<P: crate::secret::obligations::InstanceSecretsQuery> crate::secret::obligations::InstanceSecretsQuery for Generated<P> {
    fn instance_secrets(&self, instance: crate::config::InstanceName) -> Result<Vec<crate::secret::InstanceSecrets>, UnmetObligation> {
        crate::secret::obligations::InstanceSecretsQuery::instance_secrets(&self.ports, instance)
    }
}

/// `conductor.secret.Secrets`, generated: every row is one the specification fully determines from the stored `conductor.secret.Secret`s.
impl<P> crate::secret::obligations::SecretsQuery for Generated<P>
where
    P: SecretStorage,
{
    fn secrets(&self) -> Result<Vec<crate::secret::Secrets>, UnmetObligation> {
        let admitted = SecretStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::secret::Secrets {
                name: held.data.name,
                state: held.state,
                source: held.data.source,
                created_by: held.data.created_by,
                created_at: held.data.created_at,
            })
            .collect())
    }
}

/// `conductor.session.Deliveries`, generated: every row is one the specification fully determines from the stored `conductor.session.Delivery`s.
impl<P> crate::session::obligations::DeliveriesQuery for Generated<P>
where
    P: DeliveryStorage,
{
    fn deliveries(&self) -> Result<Vec<crate::session::Deliveries>, UnmetObligation> {
        let admitted = DeliveryStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::session::Deliveries {
                delivery_id: held.data.delivery_id,
                state: held.state,
                session_name: held.data.session_name,
                first_line: held.data.first_line,
                body_path: held.data.body_path,
                delivered_at: held.data.delivered_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.session.Leases`, generated: every row is one the specification fully determines from the stored `conductor.session.Lease`s.
impl<P> crate::session::obligations::LeasesQuery for Generated<P>
where
    P: LeaseStorage,
{
    fn leases(&self) -> Result<Vec<crate::session::Leases>, UnmetObligation> {
        let admitted = LeaseStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::session::Leases {
                instance: held.data.instance,
                state: held.state,
                session_id: held.data.session_id,
                pid: held.data.pid,
                taken_at: held.data.taken_at,
                taken_over_session_id: held.data.taken_over_session_id,
                taken_over_pid: held.data.taken_over_pid,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.session.RunTicks`, generated: every row is one the specification fully determines from the stored `conductor.session.RunTick`s.
impl<P> crate::session::obligations::RunTicksQuery for Generated<P>
where
    P: RunTickStorage,
{
    fn run_ticks(&self) -> Result<Vec<crate::session::RunTicks>, UnmetObligation> {
        let admitted = RunTickStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::session::RunTicks {
                tick_id: held.data.tick_id,
                state: held.state,
                kind: held.data.kind,
                started_at: held.data.started_at,
                lines_handled: held.data.lines_handled,
                actions: held.data.actions,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.store.Migrations`, generated: every row is one the specification fully determines from the stored `conductor.store.Migration`s.
impl<P> crate::store::obligations::MigrationsQuery for Generated<P>
where
    P: MigrationStorage,
{
    fn migrations(&self) -> Result<Vec<crate::store::Migrations>, UnmetObligation> {
        let admitted = MigrationStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::store::Migrations {
                instance: held.data.instance,
                state: held.state,
                source: held.data.source,
                store: held.data.store,
                events_copied: held.data.events_copied,
                migrated_at: held.data.migrated_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// `conductor.store.Readiness`, generated: every row is one the specification fully determines from the stored `conductor.store.Store`s.
impl<P> crate::store::obligations::ReadinessQuery for Generated<P>
where
    P: StoreStorage,
{
    fn readiness(&self) -> Result<Vec<crate::store::Readiness>, UnmetObligation> {
        let admitted = StoreStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::store::Readiness {
                reachable: held.data.reachable,
                reason: held.data.reason,
            })
            .collect())
    }
}

/// `conductor.store.StoreStatus`, generated: every row is one the specification fully determines from the stored `conductor.store.Store`s.
impl<P> crate::store::obligations::StoreStatusQuery for Generated<P>
where
    P: StoreStorage,
{
    fn store_status(&self) -> Result<Vec<crate::store::StoreStatus>, UnmetObligation> {
        let admitted = StoreStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::store::StoreStatus {
                url: held.data.url,
                state: held.state,
                backend: held.data.backend,
                reachable: held.data.reachable,
                last_event_id: held.data.last_event_id,
                instance_last_event_ids: held.data.instance_last_event_ids,
            })
            .collect())
    }
}

/// `conductor.store.Version`, generated: every row is one the specification fully determines from the stored `conductor.store.Build`s.
impl<P> crate::store::obligations::VersionQuery for Generated<P>
where
    P: BuildStorage,
{
    fn version(&self) -> Result<Vec<crate::store::Version>, UnmetObligation> {
        let admitted = BuildStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::store::Version {
                version: held.data.version,
                config: held.data.config,
            })
            .collect())
    }
}

impl<P: crate::token::obligations::TokensQuery> crate::token::obligations::TokensQuery for Generated<P> {
    fn tokens(&self, instance: crate::config::InstanceName) -> Result<Vec<crate::token::Tokens>, UnmetObligation> {
        crate::token::obligations::TokensQuery::tokens(&self.ports, instance)
    }
}

/// `conductor.tool.ToolCalls`, generated: every row is one the specification fully determines from the stored `conductor.tool.ToolCall`s.
impl<P> crate::tool::obligations::ToolCallsQuery for Generated<P>
where
    P: ToolCallStorage,
{
    fn tool_calls(&self) -> Result<Vec<crate::tool::ToolCalls>, UnmetObligation> {
        let admitted = ToolCallStorage::list(&self.ports);
        Ok(admitted
            .into_iter()
            .map(|held| crate::tool::ToolCalls {
                tool_call_id: held.data.tool_call_id,
                session: held.data.session,
                verb: held.data.verb,
                paths: held.data.paths,
                outcome: held.data.outcome,
                rule: held.data.rule,
                duration_ms: held.data.duration_ms,
                output_bytes: held.data.output_bytes,
                called_at: held.data.called_at,
                actor: held.data.actor,
            })
            .collect())
    }
}

/// The typed refusal of a request the model declares no outcome for.
fn undeclared(source: &'static str) -> UnmetObligation {
    let capability = "command behaviour";
    UnmetObligation { capability, source }
}

/// A guard's truth, where it has one: Unknown selects no branch, so the model declares no outcome.
fn decided(truth: Option<bool>, command: &'static str) -> Result<bool, UnmetObligation> {
    truth.ok_or_else(|| undeclared(command))
}

/// Three-valued conjunction: false wins, then Unknown.
fn all(truths: &[Option<bool>]) -> Option<bool> {
    if truths.contains(&Some(false)) {
        Some(false)
    } else if truths.contains(&None) {
        None
    } else {
        Some(true)
    }
}

/// Three-valued disjunction: true wins, then Unknown.
fn any(truths: &[Option<bool>]) -> Option<bool> {
    if truths.contains(&Some(true)) {
        Some(true)
    } else if truths.contains(&None) {
        None
    } else {
        Some(false)
    }
}

/// Equality of two read values; an unread one is Unknown.
fn equal<T: PartialEq>(left: Option<T>, right: Option<T>) -> Option<bool> {
    Some(left? == right?)
}

/// A decimal rendering as its sign, its whole digits and its fraction digits, without the zeros
/// that do not change its value; `None` where it is not a plain decimal.
fn number_parts(text: &str) -> Option<(bool, String, String)> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text.strip_prefix('+').unwrap_or(text)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty() && fraction.is_empty() {
        return None;
    }
    if !whole.bytes().chain(fraction.bytes()).all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let whole = whole.trim_start_matches('0').to_owned();
    let fraction = fraction.trim_end_matches('0').to_owned();
    let zero = whole.is_empty() && fraction.is_empty();
    Some((negative && !zero, whole, fraction))
}

/// Compares two decimal renderings exactly; an unread or unparsable one is Unknown.
fn compare_numbers(
    left: Option<String>,
    right: Option<String>,
    accepts: fn(core::cmp::Ordering) -> bool,
) -> Option<bool> {
    let (left, right) = (number_parts(&left?)?, number_parts(&right?)?);
    let magnitude = left
        .1
        .len()
        .cmp(&right.1.len())
        .then_with(|| left.1.cmp(&right.1))
        .then_with(|| left.2.cmp(&right.2));
    let ordering = match (left.0, right.0) {
        (false, false) => magnitude,
        (true, true) => magnitude.reverse(),
        (true, false) => core::cmp::Ordering::Less,
        (false, true) => core::cmp::Ordering::Greater,
    };
    Some(accepts(ordering))
}
