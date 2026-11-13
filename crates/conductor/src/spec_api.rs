//! The HTTP servers `ess` generates from the specification and its binding
//! (`conductor_model::server`, `crates/conductor/http.yaml`), over conductor's own store.
//!
//! Nothing serves them yet: conductord answers with its hand-written API ([`crate::daemon::api`])
//! and each instance runtime with its own ([`crate::dashboard`]). This module is the seam a test
//! drives both through (`tests/api_parity.rs`) to measure how far the generated surface matches
//! the hand-written one.
//!
//! The generated behaviours ([`Generated`]) run over [`StorePorts`]:
//! - every storage port the store implements ([`STORE_BACKED`]) is the store itself, the one
//!   [`crate::store::open_url`] opens for the daemon's tenant;
//! - every other storage port ([`MEMORY_BACKED`]) is the generated [`MemoryPorts`]' ephemeral
//!   store: the store keeps those records in another shape or not at all (the instances the
//!   reconciler publishes, the observations of a snapshot), so the generated behaviour that writes
//!   one forgets it when the process ends;
//! - the context (assigned identities, generated strings, `external:` answers) is
//!   [`MemoryPorts`]': a UUID for an identity, and a refusal naming what is owed for a string, a
//!   host grant's or reclaim run's id, a refusal reason and every `external:` branch;
//! - the obligations the plan owes ([`OWED`]) are [`MemoryPorts`]' refusals, but
//!   `conductor.token.Tokens`, `conductor.dispatch.SendDispatch` and
//!   `conductor.dispatch.RecordDispatchLink`, answered from the store, and
//!   `conductor.instance.InstanceByName`, answered from the ephemeral store's instances.
//!
//! A request is admitted as the loopback listener admits a write ([`caller`]): with the daemon's
//! token in `X-Conductor-Token`, and then as the operator actor of the route's domain.

use std::cell::RefCell;
use std::io::BufReader;
use std::net::TcpListener;
use std::rc::Rc;

use conductor_model::actor::{Actor, Caller};
use conductor_model::behaviour::{self as port, Generated, TryContext};
use conductor_model::obligation::UnmetObligation;
use conductor_model::ports::conductor_broker::ConductorBroker;
use conductor_model::ports::conductor_cli::ConductorCli;
use conductor_model::ports::conductor_instance::ConductorInstance;
use conductor_model::ports::conductord::Conductord;
use conductor_model::server::memory::MemoryPorts;
use conductor_model::server::{conductor_instance, conductord, http};
use conductor_model::system::System;

use crate::store::Store;

/// The storage ports the store implements: the generated behaviours read and write these records
/// in the store.
pub const STORE_BACKED: [&str; 30] = [
    "conductor.config.ConfigRevision",
    "conductor.decision.Decision",
    "conductor.decision.DecisionRequest",
    "conductor.direction.Goal",
    "conductor.direction.GoalServing",
    "conductor.direction.RepositoryMark",
    "conductor.dispatch.Controller",
    "conductor.dispatch.Dispatch",
    "conductor.dispatch.GuardDecision",
    "conductor.dispatch.Message",
    "conductor.dispatch.ResourceRequest",
    "conductor.host.HostGrant",
    "conductor.host.ReclaimRun",
    "conductor.improvement.FrictionFinding",
    "conductor.improvement.Metric",
    "conductor.improvement.MetricReading",
    "conductor.improvement.SettingsProposal",
    "conductor.instance.Reconciliation",
    "conductor.observation.Board",
    "conductor.observation.DailySeries",
    "conductor.operator.Operator",
    "conductor.placement.Box",
    "conductor.placement.BoxRefusal",
    "conductor.secret.Secret",
    "conductor.session.Delivery",
    "conductor.session.Lease",
    "conductor.session.RunTick",
    "conductor.store.Migration",
    "conductor.token.PoolToken",
    "conductor.tool.ToolCall",
];

/// The storage ports the store does not implement: the generated [`MemoryPorts`] answer them,
/// and what a behaviour writes there lasts as long as the process.
pub const MEMORY_BACKED: [&str; 19] = [
    "conductor.agent.Agent",
    "conductor.agent.AgentKind",
    "conductor.instance.Instance",
    "conductor.observation.BlockerObservation",
    "conductor.observation.CollectorRunObservation",
    "conductor.observation.MemberFailureObservation",
    "conductor.observation.MergedPullRequestObservation",
    "conductor.observation.PullRequestObservation",
    "conductor.observation.ReleaseObservation",
    "conductor.observation.RepositoryObservation",
    "conductor.observation.SecretSyncObservation",
    "conductor.observation.SessionObservation",
    "conductor.observation.Snapshot",
    "conductor.observation.SpecificationObservation",
    "conductor.observation.UsageObservation",
    "conductor.observation.WorkflowRunObservation",
    "conductor.placement.Placement",
    "conductor.store.Build",
    "conductor.store.Store",
];

/// The behaviours and queries the plan owes rather than generates, and who answers each here:
/// `store` where this module answers it from the store, `memory` where [`MemoryPorts`] refuses it
/// as unmet or, for `InstanceByName`, where this module answers it from [`MemoryPorts`]' instances.
pub const OWED: [(&str, &str); 17] = [
    ("conductor.agent.AttachAgent", "memory"),
    ("conductor.agent.DeliverToAgent", "memory"),
    ("conductor.agent.StartAgent", "memory"),
    (
        "conductor.decision.RecordOperatorDecisionForRequest",
        "memory",
    ),
    ("conductor.dispatch.RecordDispatchLink", "store"),
    ("conductor.dispatch.SendDispatch", "store"),
    ("conductor.dispatch.StartController", "memory"),
    ("conductor.improvement.SetMetricTarget", "memory"),
    ("conductor.instance.StartInstance", "memory"),
    ("conductor.secret.PutSecret", "memory"),
    ("conductor.secret.ResolveSecrets", "memory"),
    ("conductor.session.ClaimLease", "memory"),
    ("conductor.store.MigrateStore", "memory"),
    ("conductor.config.EffectiveSettings", "memory"),
    ("conductor.instance.InstanceByName", "memory"),
    ("conductor.secret.InstanceSecrets", "memory"),
    ("conductor.token.Tokens", "store"),
];

/// The ports of the generated behaviours: the store where it keeps a record, the generated
/// ephemeral store where it does not. Clones share both, so every component of the system reads
/// what another wrote.
#[derive(Clone)]
pub struct StorePorts {
    store: Rc<RefCell<Store>>,
    memory: MemoryPorts,
}

impl StorePorts {
    /// The ports over `store`, with an ephemeral store beside it that holds one record: the
    /// `conductor.store.Build` of this binary ([`crate::daemon::metrics::Build::this`]), which no
    /// command writes, as `GET /v1/version` reads it.
    #[must_use]
    pub fn new(store: Store) -> Self {
        let mut memory = MemoryPorts::default();
        let build = crate::daemon::metrics::Build::this();
        port::BuildStorage::put(
            &mut memory,
            m::store::BuildSnapshot {
                state: m::store::BuildState::Built,
                data: m::store::BuildData {
                    version: m::store::BinaryVersion(build.version),
                    config: build.config,
                },
            },
        );
        Self {
            store: Rc::new(RefCell::new(store)),
            memory,
        }
    }

    /// The first failure the store recorded since it was opened, if any: a port answers no
    /// error of its own.
    ///
    /// # Errors
    ///
    /// The store failed to read or write a record.
    pub fn check(&self) -> Result<(), crate::store::StoreError> {
        self.store.borrow().check()
    }
}

macro_rules! backed {
    ($field:ident: $($trait:ident: $id:ty => $snapshot:ty;)*) => {$(
        impl port::$trait for StorePorts {
            fn get(&self, identity: &$id) -> Option<$snapshot> {
                port::$trait::get(&*backed!(@read self $field), identity)
            }
            fn put(&mut self, snapshot: $snapshot) {
                port::$trait::put(&mut *backed!(@write self $field), snapshot);
            }
            fn delete(&mut self, identity: &$id) {
                port::$trait::delete(&mut *backed!(@write self $field), identity);
            }
            fn list(&self) -> Vec<$snapshot> {
                port::$trait::list(&*backed!(@read self $field))
            }
        }
    )*};
    (@read $self:ident store) => { $self.store.borrow() };
    (@write $self:ident store) => { $self.store.borrow_mut() };
    (@read $self:ident memory) => { &$self.memory };
    (@write $self:ident memory) => { &mut $self.memory };
}

macro_rules! unlisted {
    ($($trait:ident: $id:ty => $snapshot:ty;)*) => {$(
        impl port::$trait for StorePorts {
            fn get(&self, identity: &$id) -> Option<$snapshot> {
                port::$trait::get(&*self.store.borrow(), identity)
            }
            fn put(&mut self, snapshot: $snapshot) {
                port::$trait::put(&mut *self.store.borrow_mut(), snapshot);
            }
            fn delete(&mut self, identity: &$id) {
                port::$trait::delete(&mut *self.store.borrow_mut(), identity);
            }
        }
    )*};
}

mod m {
    pub use conductor_model::{
        agent, config, decision, direction, dispatch, host, improvement, instance, observation,
        operator, placement, secret, session, store, token, tool,
    };
}

backed! { store:
    ConfigRevisionStorage: m::config::RevisionId => m::config::ConfigRevisionSnapshot;
    DecisionStorage: m::decision::DecisionId => m::decision::DecisionSnapshot;
    DecisionRequestStorage: m::decision::RequestId => m::decision::DecisionRequestSnapshot;
    GoalStorage: m::direction::GoalId => m::direction::GoalSnapshot;
    GoalServingStorage: m::direction::ServingId => m::direction::GoalServingSnapshot;
    RepositoryMarkStorage: m::direction::MarkedRepository => m::direction::RepositoryMarkSnapshot;
    ControllerStorage: m::dispatch::ControllerId => m::dispatch::ControllerSnapshot;
    DispatchStorage: m::dispatch::DispatchId => m::dispatch::DispatchSnapshot;
    GuardDecisionStorage: m::dispatch::GuardDecisionId => m::dispatch::GuardDecisionSnapshot;
    MessageStorage: m::dispatch::MessageId => m::dispatch::MessageSnapshot;
    ResourceRequestStorage: m::dispatch::ResourceRequestId => m::dispatch::ResourceRequestSnapshot;
    HostGrantStorage: m::host::GrantId => m::host::HostGrantSnapshot;
    ReclaimRunStorage: m::host::ReclaimRunId => m::host::ReclaimRunSnapshot;
    FrictionFindingStorage: m::improvement::FindingId => m::improvement::FrictionFindingSnapshot;
    MetricStorage: m::improvement::MetricId => m::improvement::MetricSnapshot;
    MetricReadingStorage: m::improvement::MetricReadingId => m::improvement::MetricReadingSnapshot;
    SettingsProposalStorage: m::improvement::ProposalId => m::improvement::SettingsProposalSnapshot;
    ReconciliationStorage: m::instance::ReconciliationId => m::instance::ReconciliationSnapshot;
    BoardStorage: m::observation::BoardId => m::observation::BoardSnapshot;
    OperatorStorage: m::operator::OperatorKey => m::operator::OperatorSnapshot;
    BoxStorage: m::placement::BoxId => m::placement::BoxSnapshot;
    BoxRefusalStorage: m::placement::BoxId => m::placement::BoxRefusalSnapshot;
    SecretStorage: m::secret::SecretName => m::secret::SecretSnapshot;
    DeliveryStorage: m::session::DeliveryId => m::session::DeliverySnapshot;
    LeaseStorage: m::config::InstanceName => m::session::LeaseSnapshot;
    RunTickStorage: m::session::TickId => m::session::RunTickSnapshot;
    MigrationStorage: m::config::InstanceName => m::store::MigrationSnapshot;
    ToolCallStorage: m::tool::ToolCallId => m::tool::ToolCallSnapshot;
}

unlisted! {
    DailySeriesStorage: m::observation::DailySeriesId => m::observation::DailySeriesSnapshot;
    PoolTokenStorage: m::token::TokenId => m::token::PoolTokenSnapshot;
}

backed! { memory:
    AgentStorage: m::agent::AgentName => m::agent::AgentSnapshot;
    AgentKindStorage: m::agent::AgentKindName => m::agent::AgentKindSnapshot;
    InstanceStorage: m::config::InstanceName => m::instance::InstanceSnapshot;
    BlockerObservationStorage: m::observation::ObservationId => m::observation::BlockerObservationSnapshot;
    CollectorRunObservationStorage: m::observation::ObservationId => m::observation::CollectorRunObservationSnapshot;
    MemberFailureObservationStorage: m::observation::ObservationId => m::observation::MemberFailureObservationSnapshot;
    MergedPullRequestObservationStorage: m::observation::ObservationId => m::observation::MergedPullRequestObservationSnapshot;
    PullRequestObservationStorage: m::observation::ObservationId => m::observation::PullRequestObservationSnapshot;
    ReleaseObservationStorage: m::observation::ObservationId => m::observation::ReleaseObservationSnapshot;
    RepositoryObservationStorage: m::observation::ObservationId => m::observation::RepositoryObservationSnapshot;
    SecretSyncObservationStorage: m::observation::ObservationId => m::observation::SecretSyncObservationSnapshot;
    SessionObservationStorage: m::observation::ObservationId => m::observation::SessionObservationSnapshot;
    SnapshotStorage: m::observation::SnapshotId => m::observation::SnapshotSnapshot;
    SpecificationObservationStorage: m::observation::ObservationId => m::observation::SpecificationObservationSnapshot;
    UsageObservationStorage: m::observation::ObservationId => m::observation::UsageObservationSnapshot;
    WorkflowRunObservationStorage: m::observation::ObservationId => m::observation::WorkflowRunObservationSnapshot;
    PlacementStorage: m::placement::PlacementName => m::placement::PlacementSnapshot;
    BuildStorage: m::store::BinaryVersion => m::store::BuildSnapshot;
    StoreStorage: m::store::StoreUrl => m::store::StoreSnapshot;
}

macro_rules! context {
    ($($method:ident -> $type:ty;)*) => {
        impl TryContext for StorePorts {
            $(fn $method(&mut self) -> Result<$type, UnmetObligation> {
                TryContext::$method(&mut self.memory)
            })*
            fn try_external(
                &mut self,
                command: port::ExternalCommand<'_>,
                outcome: &'static str,
            ) -> Result<bool, UnmetObligation> {
                TryContext::try_external(&mut self.memory, command, outcome)
            }
        }
    };
}

context! {
    try_generate_conductor_config_revision_id -> m::config::RevisionId;
    try_generate_conductor_direction_serving_id -> m::direction::ServingId;
    try_generate_conductor_dispatch_guard_decision_id -> m::dispatch::GuardDecisionId;
    try_generate_conductor_dispatch_message_id -> m::dispatch::MessageId;
    try_generate_conductor_dispatch_resource_request_id -> m::dispatch::ResourceRequestId;
    try_generate_conductor_host_grant_id -> m::host::GrantId;
    try_generate_conductor_host_reclaim_run_id -> m::host::ReclaimRunId;
    try_generate_conductor_instance_reconciliation_id -> m::instance::ReconciliationId;
    try_generate_conductor_observation_board_id -> m::observation::BoardId;
    try_generate_conductor_observation_observation_id -> m::observation::ObservationId;
    try_generate_conductor_observation_snapshot_id -> m::observation::SnapshotId;
    try_generate_conductor_placement_box_id -> m::placement::BoxId;
    try_generate_conductor_placement_refusal_reason -> m::placement::RefusalReason;
    try_generate_conductor_session_delivery_id -> m::session::DeliveryId;
    try_generate_conductor_session_tick_id -> m::session::TickId;
    try_generate_conductor_token_token_id -> m::token::TokenId;
    try_generate_conductor_tool_tool_call_id -> m::tool::ToolCallId;
    try_generate_string -> String;
}

macro_rules! owed {
    ($($module:ident::$trait:ident::$method:ident($input:ident) -> $outcome:ident;)*) => {$(
        impl m::$module::obligations::$trait for StorePorts {
            fn $method(
                &mut self,
                input: m::$module::$input,
            ) -> Result<m::$module::$outcome, UnmetObligation> {
                m::$module::obligations::$trait::$method(&mut self.memory, input)
            }
        }
    )*};
}

owed! {
    agent::AttachAgentBehavior::attach_agent(AttachAgent) -> AttachAgentOutcome;
    agent::DeliverToAgentBehavior::deliver_to_agent(DeliverToAgent) -> DeliverToAgentOutcome;
    agent::StartAgentBehavior::start_agent(StartAgent) -> StartAgentOutcome;
    decision::RecordOperatorDecisionForRequestBehavior::record_operator_decision_for_request(
        RecordOperatorDecisionForRequest) -> RecordOperatorDecisionForRequestOutcome;
    dispatch::StartControllerBehavior::start_controller(StartController) -> StartControllerOutcome;
    improvement::SetMetricTargetBehavior::set_metric_target(SetMetricTarget) -> SetMetricTargetOutcome;
    instance::StartInstanceBehavior::start_instance(StartInstance) -> StartInstanceOutcome;
    secret::PutSecretBehavior::put_secret(PutSecret) -> PutSecretOutcome;
    secret::ResolveSecretsBehavior::resolve_secrets(ResolveSecrets) -> ResolveSecretsOutcome;
    session::ClaimLeaseBehavior::claim_lease(ClaimLease) -> ClaimLeaseOutcome;
    store::MigrateStoreBehavior::migrate_store(MigrateStore) -> MigrateStoreOutcome;
}

/// `conductor.dispatch.SendDispatch`, decided over the store as `conductor dispatch send-dispatch`
/// decides it ([`crate::dispatch::behaviour`]).
impl m::dispatch::obligations::SendDispatchBehavior for StorePorts {
    fn send_dispatch(
        &mut self,
        input: m::dispatch::SendDispatch,
    ) -> Result<m::dispatch::SendDispatchOutcome, UnmetObligation> {
        crate::dispatch::behaviour::send_dispatch(&mut *self.store.borrow_mut(), input)
    }
}

/// `conductor.dispatch.RecordDispatchLink`, decided over the store as `conductor dispatch
/// record-link` decides it ([`crate::dispatch::behaviour`]).
impl m::dispatch::obligations::RecordDispatchLinkBehavior for StorePorts {
    fn record_dispatch_link(
        &mut self,
        input: m::dispatch::RecordDispatchLink,
    ) -> Result<m::dispatch::RecordDispatchLinkOutcome, UnmetObligation> {
        crate::dispatch::behaviour::record_dispatch_link(&mut *self.store.borrow_mut(), input)
    }
}

impl m::config::obligations::EffectiveSettingsQuery for StorePorts {
    fn effective_settings(
        &self,
        instance: m::config::InstanceName,
    ) -> Result<Vec<m::config::EffectiveSettings>, UnmetObligation> {
        m::config::obligations::EffectiveSettingsQuery::effective_settings(&self.memory, instance)
    }
}

/// `conductor.secret.InstanceSecrets`, `GET /v1/instances/{instance}/secrets`: refused as unmet by
/// [`MemoryPorts`]. The hand-written route derives its rows from the daemon's config file and its
/// secrets source at each read, neither of which the generated surface reads.
impl m::secret::obligations::InstanceSecretsQuery for StorePorts {
    fn instance_secrets(
        &self,
        instance: m::config::InstanceName,
    ) -> Result<Vec<m::secret::InstanceSecrets>, UnmetObligation> {
        m::secret::obligations::InstanceSecretsQuery::instance_secrets(&self.memory, instance)
    }
}

/// `conductor.instance.InstanceByName`, `GET /v1/instances/{name}`: the instance of that name the
/// generated ephemeral store holds, as `conductor.instance.Instances` reads every one; none, the
/// view's `404`, for a name it does not hold.
impl m::instance::obligations::InstanceByNameQuery for StorePorts {
    fn instance_by_name(
        &self,
        name: m::config::InstanceName,
    ) -> Result<Vec<m::instance::InstanceByName>, UnmetObligation> {
        Ok(port::InstanceStorage::get(&self.memory, &name)
            .into_iter()
            .map(|instance| {
                let data = instance.data;
                m::instance::InstanceByName {
                    name: data.name,
                    state: instance.state,
                    placement: data.placement,
                    roles: data.roles,
                    binary_version: data.binary_version,
                    last_seen: data.last_seen,
                    box_id: data.box_id,
                    sources: data.sources,
                    detail: data.detail,
                    changed_by: data.changed_by,
                    git_identity: data.git_identity,
                    model_token_kind: data.model_token_kind,
                }
            })
            .collect())
    }
}

/// `conductor.token.Tokens` of `instance`, from the store's pool: every token, withdrawn ones
/// included, as the view's source holds them.
impl m::token::obligations::TokensQuery for StorePorts {
    fn tokens(
        &self,
        instance: m::config::InstanceName,
    ) -> Result<Vec<m::token::Tokens>, UnmetObligation> {
        Ok(self
            .store
            .borrow()
            .pool_tokens()
            .into_iter()
            .filter(|token| token.data.instance == instance)
            .map(|token| m::token::Tokens {
                token_id: token.data.token_id,
                state: token.state,
                instance: token.data.instance,
                owner: token.data.owner,
                seat: token.data.seat,
                token_state: token.data.token_state,
                limited_until: token.data.limited_until,
                agent: token.data.agent,
                contributed_at: token.data.contributed_at,
                actor: token.data.actor,
            })
            .collect())
    }
}

/// Every component's behaviours: the generated ones over [`StorePorts`].
pub type Behaviours = Generated<StorePorts>;

/// The whole generated system over one store.
pub type SpecSystem = System<Behaviours, Behaviours, Behaviours, Behaviours>;

/// The generated system over `store`: every component on clones of one [`StorePorts`].
#[must_use]
pub fn system(store: Store) -> SpecSystem {
    let ports = StorePorts::new(store);
    System::new(
        ConductorBroker::new(Generated::new(ports.clone())),
        ConductorCli::new(Generated::new(ports.clone())),
        ConductorInstance::new(Generated::new(ports.clone())),
        Conductord::new(Generated::new(ports)),
    )
}

/// Which generated surface a listener serves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    /// conductord's global API (`conductor_model::server::conductord`).
    Conductord,
    /// One instance runtime's API (`conductor_model::server::conductor_instance`).
    Instance,
}

/// Who a request is from, as the loopback listener would admit its write: the daemon's `token` in
/// `X-Conductor-Token`, then the operator actor of the route's domain (`/v1/config`,
/// `/v1/instances` and `/v1/instances/{name}` the config's, `/v1/instances/{name}/…` the
/// instance's, `/v1/agents/…` the agent's). Without the token, or on another path, no actor: a
/// command answers the standard `403`.
#[must_use]
pub fn caller(request: &http::Request, token: &str) -> Option<Caller> {
    let given = request
        .headers
        .iter()
        .find(|(name, _)| name.eq_ignore_ascii_case(crate::dashboard::TOKEN_HEADER))
        .map(|(_, value)| value.as_str())?;
    if !crate::dashboard::same_token(token, given) {
        return None;
    }
    let path = request.path.as_str();
    let whole_instance = path == "/v1/instances"
        || path
            .strip_prefix("/v1/instances/")
            .is_some_and(|name| !name.is_empty() && !name.contains('/'));
    let actor = if path == "/v1/config" || whole_instance {
        Actor::ConfigOperator
    } else if path.starts_with("/v1/instances/") {
        Actor::InstanceOperator
    } else if path.starts_with("/v1/agents/") {
        Actor::AgentOperator
    } else {
        return None;
    };
    Some(Caller { actor })
}

/// Serves `surface` of the generated system over `store` on `listener`, one connection at a time,
/// until the listener fails; each request admitted by [`caller`] with `token`.
pub fn serve(listener: &TcpListener, surface: Surface, store: Store, token: &str) {
    let mut system = system(store);
    for connection in listener.incoming() {
        let Ok(connection) = connection else {
            continue;
        };
        let _ = connection.set_read_timeout(Some(http::READ_TIMEOUT));
        let _ = connection.set_write_timeout(Some(http::WRITE_TIMEOUT));
        let mut reader = BufReader::new(connection);
        let answer = match http::read(&mut reader) {
            Ok(request) => {
                let who = caller(&request, token);
                match surface {
                    Surface::Conductord => {
                        conductord::dispatch(&mut system, who.as_ref(), &request)
                    }
                    Surface::Instance => {
                        conductor_instance::dispatch(&mut system, who.as_ref(), &request)
                    }
                }
            }
            Err(refusal) => refusal,
        };
        let mut stream = reader.into_inner();
        let _ = http::write(&mut stream, &answer);
    }
}
