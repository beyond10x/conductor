// generated from conductor v1
// contract digest e3b967aa3e3d0f313417baefaaf92190635d03007992d259a68d9a4352490e1a
// do not edit: regenerate with `ess synthesize --layout crate`

//! The `conductord` component of `conductor` v1, on the wire.
//!
//! The specification says this component's callers are not deployed with it, so its surface
//! exists on a wire. Which wire is derived rather than chosen: the one contract this model
//! projects for a command surface is the `OpenAPI` document, and an `OpenAPI` document is an
//! HTTP contract. The document is beside this file, served verbatim at `/openapi.json`.

use crate::server::{entry, http, json, wire};

/// The contract this surface answers, byte for byte as `generated/` commits it.
///
/// Embedded rather than rebuilt at run time: a server that regenerated its own contract could
/// publish one the repository never reviewed.
pub const OPENAPI: &str = include_str!("conductord.openapi.json");

/// The prose the same model produced, byte for byte as the documentation projection wrote it.
pub const DOCS: &str = include_str!("conductord.docs.md");

/// Every route this surface answers, in path order.
///
/// The same set the `OpenAPI` document declares, plus the two documents about the surface
/// itself, which no specification construct names and nothing can therefore derive. A path
/// absent from this table is answered with `404`, including one the document declares and this
/// table forgot — which is the failure a table computed twice would hide.
pub const ROUTES: &[(&str, &str)] = &[
    ("POST", "/config/commands/put-config"),
    ("GET", "/config/views/config-revisions"),
    ("GET", "/config/views/effective-settings"),
    ("GET", "/docs"),
    ("POST", "/instance/commands/observe"),
    ("POST", "/instance/commands/purge"),
    ("POST", "/instance/commands/reconcile"),
    ("POST", "/instance/commands/restart"),
    ("POST", "/instance/commands/start"),
    ("POST", "/instance/commands/stop"),
    ("POST", "/instance/commands/upgrade"),
    ("GET", "/instance/views/instances"),
    ("GET", "/instance/views/reconciliations"),
    ("GET", "/openapi.json"),
    ("POST", "/operator/commands/join-instance"),
    ("POST", "/operator/commands/leave-instance"),
    ("GET", "/operator/views/operators"),
    ("POST", "/placement/commands/close-box"),
    ("POST", "/placement/commands/open-box"),
    ("GET", "/placement/views/box-refusals"),
    ("GET", "/placement/views/boxes"),
    ("GET", "/placement/views/placements"),
    ("POST", "/secret/commands/put-secret"),
    ("POST", "/secret/commands/resolve-secrets"),
    ("GET", "/secret/views/secrets"),
    ("POST", "/store/commands/migrate-store"),
    ("GET", "/store/views/migrations"),
    ("GET", "/store/views/store-status"),
    ("POST", "/token/commands/contribute-token"),
    ("POST", "/token/commands/lease-token"),
    ("POST", "/token/commands/mark-token-limited"),
    ("POST", "/token/commands/release-token"),
    ("POST", "/token/commands/withdraw-token"),
    ("GET", "/token/views/tokens"),
];

/// What this process says about itself as it starts, before it answers anything.
///
/// Three lines of JSON on standard output, in this order, every member of them derived from the
/// specification — except `runtime`, which is appended by the emitted code below and holds what
/// is true of *this process*: the language it was synthesised into, and the address it bound.
/// Everything outside `runtime` is the same in every language this plan is emitted into, and
/// `cargo xtask synth --check` starts both and compares them.
pub const STARTUP: &[&str] = &[
    "{\"log\":\"ess/1\",\"event\":\"system.starting\",\"system\":\"conductor\",\"version\":\"v1\",\"model_digest\":\"1bf286dbd4a2b0e7a7a8448be1f91fb1c6e3328968f4356b5e113bc4f4b9a370\",\"contract_digest\":\"e3b967aa3e3d0f313417baefaaf92190635d03007992d259a68d9a4352490e1a\",\"components\":[\"conductor-broker\",\"conductor-cli\",\"conductor-instance\",\"conductord\"],\"capabilities\":{\"generated\":919,\"obligations\":12,\"refused\":0}",
    "{\"log\":\"ess/1\",\"event\":\"surface.serving\",\"component\":\"conductord\",\"reached_by\":\"network\",\"transport\":\"http/1.1\",\"routes\":34,\"paths\":[{\"method\":\"POST\",\"path\":\"/config/commands/put-config\",\"serves\":\"command\",\"name\":\"conductor.config.PutConfig\"},{\"method\":\"GET\",\"path\":\"/config/views/config-revisions\",\"serves\":\"view\",\"name\":\"conductor.config.ConfigRevisions\"},{\"method\":\"GET\",\"path\":\"/config/views/effective-settings\",\"serves\":\"view\",\"name\":\"conductor.config.EffectiveSettings\"},{\"method\":\"GET\",\"path\":\"/docs\",\"serves\":\"documentation\",\"name\":\"docs\"},{\"method\":\"POST\",\"path\":\"/instance/commands/observe\",\"serves\":\"command\",\"name\":\"conductor.instance.ObserveInstance\"},{\"method\":\"POST\",\"path\":\"/instance/commands/purge\",\"serves\":\"command\",\"name\":\"conductor.instance.PurgeInstance\"},{\"method\":\"POST\",\"path\":\"/instance/commands/reconcile\",\"serves\":\"command\",\"name\":\"conductor.instance.Reconcile\"},{\"method\":\"POST\",\"path\":\"/instance/commands/restart\",\"serves\":\"command\",\"name\":\"conductor.instance.RestartInstance\"},{\"method\":\"POST\",\"path\":\"/instance/commands/start\",\"serves\":\"command\",\"name\":\"conductor.instance.StartInstance\"},{\"method\":\"POST\",\"path\":\"/instance/commands/stop\",\"serves\":\"command\",\"name\":\"conductor.instance.StopInstance\"},{\"method\":\"POST\",\"path\":\"/instance/commands/upgrade\",\"serves\":\"command\",\"name\":\"conductor.instance.UpgradeInstance\"},{\"method\":\"GET\",\"path\":\"/instance/views/instances\",\"serves\":\"view\",\"name\":\"conductor.instance.Instances\"},{\"method\":\"GET\",\"path\":\"/instance/views/reconciliations\",\"serves\":\"view\",\"name\":\"conductor.instance.Reconciliations\"},{\"method\":\"GET\",\"path\":\"/openapi.json\",\"serves\":\"contract\",\"name\":\"openapi\"},{\"method\":\"POST\",\"path\":\"/operator/commands/join-instance\",\"serves\":\"command\",\"name\":\"conductor.operator.JoinInstance\"},{\"method\":\"POST\",\"path\":\"/operator/commands/leave-instance\",\"serves\":\"command\",\"name\":\"conductor.operator.LeaveInstance\"},{\"method\":\"GET\",\"path\":\"/operator/views/operators\",\"serves\":\"view\",\"name\":\"conductor.operator.Operators\"},{\"method\":\"POST\",\"path\":\"/placement/commands/close-box\",\"serves\":\"command\",\"name\":\"conductor.placement.CloseBox\"},{\"method\":\"POST\",\"path\":\"/placement/commands/open-box\",\"serves\":\"command\",\"name\":\"conductor.placement.OpenBox\"},{\"method\":\"GET\",\"path\":\"/placement/views/box-refusals\",\"serves\":\"view\",\"name\":\"conductor.placement.BoxRefusals\"},{\"method\":\"GET\",\"path\":\"/placement/views/boxes\",\"serves\":\"view\",\"name\":\"conductor.placement.Boxes\"},{\"method\":\"GET\",\"path\":\"/placement/views/placements\",\"serves\":\"view\",\"name\":\"conductor.placement.Placements\"},{\"method\":\"POST\",\"path\":\"/secret/commands/put-secret\",\"serves\":\"command\",\"name\":\"conductor.secret.PutSecret\"},{\"method\":\"POST\",\"path\":\"/secret/commands/resolve-secrets\",\"serves\":\"command\",\"name\":\"conductor.secret.ResolveSecrets\"},{\"method\":\"GET\",\"path\":\"/secret/views/secrets\",\"serves\":\"view\",\"name\":\"conductor.secret.Secrets\"},{\"method\":\"POST\",\"path\":\"/store/commands/migrate-store\",\"serves\":\"command\",\"name\":\"conductor.store.MigrateStore\"},{\"method\":\"GET\",\"path\":\"/store/views/migrations\",\"serves\":\"view\",\"name\":\"conductor.store.Migrations\"},{\"method\":\"GET\",\"path\":\"/store/views/store-status\",\"serves\":\"view\",\"name\":\"conductor.store.StoreStatus\"},{\"method\":\"POST\",\"path\":\"/token/commands/contribute-token\",\"serves\":\"command\",\"name\":\"conductor.token.ContributeToken\"},{\"method\":\"POST\",\"path\":\"/token/commands/lease-token\",\"serves\":\"command\",\"name\":\"conductor.token.LeaseToken\"},{\"method\":\"POST\",\"path\":\"/token/commands/mark-token-limited\",\"serves\":\"command\",\"name\":\"conductor.token.MarkTokenLimited\"},{\"method\":\"POST\",\"path\":\"/token/commands/release-token\",\"serves\":\"command\",\"name\":\"conductor.token.ReleaseToken\"},{\"method\":\"POST\",\"path\":\"/token/commands/withdraw-token\",\"serves\":\"command\",\"name\":\"conductor.token.WithdrawToken\"},{\"method\":\"GET\",\"path\":\"/token/views/tokens\",\"serves\":\"view\",\"name\":\"conductor.token.Tokens\"}]",
    "{\"log\":\"ess/1\",\"event\":\"system.ready\",\"system\":\"conductor\",\"surfaces\":2",
];

/// Writes the startup record, with this process's own facts closing each line.
fn announce(address: &std::net::SocketAddr) {
    for facts in STARTUP {
        let mut line = String::from(*facts);
        line.push_str(",\"runtime\":{\"address\":");
        json::push_text(&mut line, &address.to_string());
        line.push_str(",\"language\":\"rust\",\"port\":");
        json::push_integer(&mut line, i64::from(address.port()));
        line.push_str("}}");
        println!("{line}");
    }
}

/// Serves `conductord` at `address`, and does not return while it can answer.
///
/// `address` may name port `0`, which binds an ephemeral port; the startup record says which one
/// was taken, because a caller that cannot learn the port cannot make a request.
///
/// It chooses no realization. Every command reaches the port, and a port over unimplemented
/// obligations answers the typed refusal this surface reports as `501` — the honest empty
/// state rather than a server that pretends.
///
/// `authenticate` is the realization's: it says who each request was sent by, or `None`, and
/// [`dispatch`] checks that caller's grant before the command runs. Nothing here reads an
/// actor from the request itself.
///
/// One connection at a time: each is dropped after [`http::READ_TIMEOUT`] without a byte, or
/// [`http::WRITE_TIMEOUT`] of a stalled write, and whatever fails on one connection — a caller
/// that hung up before reading its answer, a reset, a failed accept — ends that connection only.
///
/// # Errors
///
/// What binding the address refuses: the address is taken, or the port is privileged.
pub fn serve<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, address: &str, authenticate: impl Fn(&http::Request) -> Option<crate::actor::Caller>) -> std::io::Result<()>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{ serve_with_static(system, address, authenticate, None) }

/// Serves the declared surface, with files only for paths outside its route table.
///
/// # Errors
/// Returns a listener or static-root error before announcing readiness.
pub fn serve_with_static<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, address: &str, authenticate: impl Fn(&http::Request) -> Option<crate::actor::Caller>, static_root: Option<&std::path::Path>) -> std::io::Result<()>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let static_root = static_root.map(std::fs::canonicalize).transpose()?;
    if static_root.as_ref().is_some_and(|root| !root.is_dir()) { return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "static root is not a directory")); }
    let listener = std::net::TcpListener::bind(address)?;
    announce(&listener.local_addr()?);
    for connection in listener.incoming() {
        // What fails on one connection ends that connection, never this loop: a caller that
        // gave up before it was accepted, or before it read its answer, is that caller's affair.
        let Ok(connection) = connection else {
            // An accept the listener itself failed (no descriptor left) would fail again at
            // once: pause before the next.
            std::thread::sleep(std::time::Duration::from_millis(10));
            continue;
        };
        let _ = connection.set_read_timeout(Some(http::READ_TIMEOUT));
        let _ = connection.set_write_timeout(Some(http::WRITE_TIMEOUT));
        let mut reader = std::io::BufReader::new(connection);
        let (answer, refused) = match http::read(&mut reader) {
            Ok(request) => {
                if !ROUTES.iter().any(|(_, path)| *path == request.path) {
                    if let Some(root) = &static_root {
                        let _ = crate::server::static_assets::answer(reader.get_mut(), root, &request);
                        continue;
                    }
                }
                (dispatch(system, authenticate(&request).as_ref(), &request), false)
            },
            Err(refusal) => (refusal, true),
        };
        let mut stream = reader.into_inner();
        if http::write(&mut stream, &answer).is_ok() && refused {
            http::linger(&mut stream);
        }
    }
    Ok(())
}

/// Answers one request.
///
/// A path this table does not hold is a `404` naming where the whole table is published; a
/// path it holds under a different method is a `405` naming the one it answers. Neither is a
/// status the contract declares, and neither should be: both are facts about a transport rather
/// than about any command.
///
/// Public so a caller can hand it a request it built itself: [`serve`] is this function behind a
/// socket, and nothing else.
///
/// `caller` is who the realization authenticated the request as, or `None`. Every command
/// checks its grant before it runs, and answers the standard refusal when the caller is none
/// or is an actor the specification does not grant the command.
/// A view some actor's grant names checks the reader's grant the same way before it is read.
pub fn dispatch<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, caller: Option<&crate::actor::Caller>, request: &http::Request) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    dispatch_with_events(system, caller, request).0
}

/// Answers one request as [`dispatch`] does, and hands back every event published while it
/// ran, in log order: what the command published, what the bindings that reacted to it
/// published in turn, and what a held-back delivery retried during the call published. An
/// event already on the system's log when the call started is not the call's and is never
/// in the list. The list is empty where no command ran — a refusal before the port, a view,
/// a document or a path no route matches — and holds what was published where delivering
/// it failed after the command took effect. An event a component's port published through a
/// direct call is in that component's outbox until a pump collects it, so one no pump has
/// collected yet is listed as this call's: call `pump` after calling a port directly.
pub fn dispatch_with_events<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, caller: Option<&crate::actor::Caller>, request: &http::Request) -> (http::Response, Vec<crate::system::SystemEvent>)
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let mut taken = Vec::new();
    let answer = route_request(system, caller, request, &mut taken);
    (answer, taken)
}

/// The route match behind [`dispatch_with_events`]: each command pushes what it took off the
/// log onto `taken`.
fn route_request<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, caller: Option<&crate::actor::Caller>, request: &http::Request, taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match request.path.as_str() {
        "/config/commands/put-config" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.config.PutConfig") {
                return not_granted(actor);
            }
            serve_conductor_config_put_config(system, &request.body, taken)
        }
        "/config/views/config-revisions" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_config_config_revisions(system))
        }
        "/config/views/effective-settings" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            serve_conductor_config_effective_settings(system, &request.query)
        }
        "/docs" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::Response::new(200, http::MARKDOWN, DOCS)
        }
        "/instance/commands/observe" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.instance.ObserveInstance") {
                return not_granted(actor);
            }
            serve_conductor_instance_observe_instance(system, &request.body, taken)
        }
        "/instance/commands/purge" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.instance.PurgeInstance") {
                return not_granted(actor);
            }
            serve_conductor_instance_purge_instance(system, &request.body, taken)
        }
        "/instance/commands/reconcile" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.instance.Reconcile") {
                return not_granted(actor);
            }
            serve_conductor_instance_reconcile(system, &request.body, taken)
        }
        "/instance/commands/restart" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.instance.RestartInstance") {
                return not_granted(actor);
            }
            serve_conductor_instance_restart_instance(system, &request.body, taken)
        }
        "/instance/commands/start" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.instance.StartInstance") {
                return not_granted(actor);
            }
            serve_conductor_instance_start_instance(system, &request.body, taken)
        }
        "/instance/commands/stop" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.instance.StopInstance") {
                return not_granted(actor);
            }
            serve_conductor_instance_stop_instance(system, &request.body, taken)
        }
        "/instance/commands/upgrade" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.instance.UpgradeInstance") {
                return not_granted(actor);
            }
            serve_conductor_instance_upgrade_instance(system, &request.body, taken)
        }
        "/instance/views/instances" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_instance_instances(system))
        }
        "/instance/views/reconciliations" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_instance_reconciliations(system))
        }
        "/openapi.json" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::Response::new(200, http::JSON, OPENAPI)
        }
        "/operator/commands/join-instance" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.operator.JoinInstance") {
                return not_granted(actor);
            }
            serve_conductor_operator_join_instance(system, &request.body, taken)
        }
        "/operator/commands/leave-instance" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.operator.LeaveInstance") {
                return not_granted(actor);
            }
            serve_conductor_operator_leave_instance(system, &request.body, taken)
        }
        "/operator/views/operators" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "conductor.operator.Operators") {
                return not_granted(actor);
            }
            http::answer(run_conductor_operator_operators(system))
        }
        "/placement/commands/close-box" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.placement.CloseBox") {
                return not_granted(actor);
            }
            serve_conductor_placement_close_box(system, &request.body, taken)
        }
        "/placement/commands/open-box" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.placement.OpenBox") {
                return not_granted(actor);
            }
            serve_conductor_placement_open_box(system, &request.body, taken)
        }
        "/placement/views/box-refusals" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_placement_box_refusals(system))
        }
        "/placement/views/boxes" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_placement_boxes(system))
        }
        "/placement/views/placements" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_placement_placements(system))
        }
        "/secret/commands/put-secret" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.secret.PutSecret") {
                return not_granted(actor);
            }
            serve_conductor_secret_put_secret(system, &request.body, taken)
        }
        "/secret/commands/resolve-secrets" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.secret.ResolveSecrets") {
                return not_granted(actor);
            }
            serve_conductor_secret_resolve_secrets(system, &request.body, taken)
        }
        "/secret/views/secrets" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_secret_secrets(system))
        }
        "/store/commands/migrate-store" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.store.MigrateStore") {
                return not_granted(actor);
            }
            serve_conductor_store_migrate_store(system, &request.body, taken)
        }
        "/store/views/migrations" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_store_migrations(system))
        }
        "/store/views/store-status" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_store_store_status(system))
        }
        "/token/commands/contribute-token" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.token.ContributeToken") {
                return not_granted(actor);
            }
            serve_conductor_token_contribute_token(system, &request.body, taken)
        }
        "/token/commands/lease-token" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.token.LeaseToken") {
                return not_granted(actor);
            }
            serve_conductor_token_lease_token(system, &request.body, taken)
        }
        "/token/commands/mark-token-limited" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.token.MarkTokenLimited") {
                return not_granted(actor);
            }
            serve_conductor_token_mark_token_limited(system, &request.body, taken)
        }
        "/token/commands/release-token" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.token.ReleaseToken") {
                return not_granted(actor);
            }
            serve_conductor_token_release_token(system, &request.body, taken)
        }
        "/token/commands/withdraw-token" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.token.WithdrawToken") {
                return not_granted(actor);
            }
            serve_conductor_token_withdraw_token(system, &request.body, taken)
        }
        "/token/views/tokens" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::answer(run_conductor_token_tokens(system))
        }
        other => http::Response::refusal(
            404,
            &format!("`{other}` is not a path this surface declares; `GET /openapi.json` publishes every one that is"),
        ),
    }
}

/// Runs one command or view of `conductord` by its qualified name, with no transport.
///
/// The same decoding, refusals and rendering the HTTP routes use — each route and this function call
/// one `run_*` function — so a conformance runner or an in-process caller drives the system
/// without a socket and without a dispatch table of its own. `Ok` is the declared outcome, as the
/// route's body renders it; a view without parameters ignores `input`, as its `GET` route ignores a body, and a view with
/// parameters reads them from `input`, an object keyed by their wire names, as its route reads
/// them from the query string.
///
/// # Errors
///
/// [`entry::Refused::Unknown`] naming `name` when this surface declares no command or view
/// by it; [`entry::Refused::Input`] when `input` is not the command's declared input (the
/// route's `400`); [`entry::Refused::Unmet`] when the port reports an unmet obligation, and
/// [`entry::Refused::Undelivered`] when the command took effect and delivering what it published
/// failed (the route's `501`, with `committed` `false` and `true`).
/// [`entry::Refused::NotGranted`] when `caller` — who the realization authenticated the call
/// as — is none, or is an actor the specification does not grant the command; checked before
/// the command runs (the route's `403`).
/// A view some actor's grant names answers the same refusal, checked before it is read.
///
/// `input` is held to what the route's reader admits: nested no deeper, every number spelled
/// as JSON spells one, or [`entry::Refused::Input`]. The answer is read back at any depth; one holding a
/// `Json` value the realization built with a number JSON does not spell is not JSON, and is
/// [`entry::Refused::Undelivered`] for a command, whose effect stands, and [`entry::Refused::Unmet`]
/// for a view.
pub fn handle<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, caller: Option<&crate::actor::Caller>, name: &str, input: json::Value) -> Result<json::Value, entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    handle_with_events(system, caller, name, input).0
}

/// Runs one command or view as [`handle`] does, and hands back every event published while it
/// ran, in log order: what the command published, what the bindings that reacted to it
/// published in turn, and what a held-back delivery retried during the call published. An
/// event already on the system's log when the call started is not the call's and is never
/// in the list. The list is empty where no command ran — an unknown name, a refusal before
/// the port, a view — and holds what was published where delivering it failed after the
/// command took effect. An event a component's port published through a direct call is in
/// that component's outbox until a pump collects it, so one no pump has collected yet is
/// listed as this call's: call `pump` after calling a port directly.
pub fn handle_with_events<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, caller: Option<&crate::actor::Caller>, name: &str, input: json::Value) -> (Result<json::Value, entry::Refused>, Vec<crate::system::SystemEvent>)
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let mut taken = Vec::new();
    let checked = |input: &json::Value| {
        json::admitted(input).map_err(|expected| {
            entry::Refused::Input(format!("the input is not JSON: expected {expected}"))
        })
    };
    let answered = match name {
        "conductor.config.ConfigRevisions" => run_conductor_config_config_revisions(system),
        "conductor.config.EffectiveSettings" => checked(&input).and_then(|()| run_conductor_config_effective_settings(system, &input)),
        "conductor.config.PutConfig" => match admit(caller, "conductor.config.PutConfig") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_config_put_config(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.instance.Instances" => run_conductor_instance_instances(system),
        "conductor.instance.ObserveInstance" => match admit(caller, "conductor.instance.ObserveInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_instance_observe_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.instance.PurgeInstance" => match admit(caller, "conductor.instance.PurgeInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_instance_purge_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.instance.Reconcile" => match admit(caller, "conductor.instance.Reconcile") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_instance_reconcile(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.instance.Reconciliations" => run_conductor_instance_reconciliations(system),
        "conductor.instance.RestartInstance" => match admit(caller, "conductor.instance.RestartInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_instance_restart_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.instance.StartInstance" => match admit(caller, "conductor.instance.StartInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_instance_start_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.instance.StopInstance" => match admit(caller, "conductor.instance.StopInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_instance_stop_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.instance.UpgradeInstance" => match admit(caller, "conductor.instance.UpgradeInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_instance_upgrade_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.operator.JoinInstance" => match admit(caller, "conductor.operator.JoinInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_operator_join_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.operator.LeaveInstance" => match admit(caller, "conductor.operator.LeaveInstance") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_operator_leave_instance(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.operator.Operators" => match admit_read(caller, "conductor.operator.Operators") {
            Ok(()) => run_conductor_operator_operators(system),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.placement.BoxRefusals" => run_conductor_placement_box_refusals(system),
        "conductor.placement.Boxes" => run_conductor_placement_boxes(system),
        "conductor.placement.CloseBox" => match admit(caller, "conductor.placement.CloseBox") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_placement_close_box(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.placement.OpenBox" => match admit(caller, "conductor.placement.OpenBox") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_placement_open_box(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.placement.Placements" => run_conductor_placement_placements(system),
        "conductor.secret.PutSecret" => match admit(caller, "conductor.secret.PutSecret") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_secret_put_secret(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.secret.ResolveSecrets" => match admit(caller, "conductor.secret.ResolveSecrets") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_secret_resolve_secrets(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.secret.Secrets" => run_conductor_secret_secrets(system),
        "conductor.store.MigrateStore" => match admit(caller, "conductor.store.MigrateStore") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_store_migrate_store(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.store.Migrations" => run_conductor_store_migrations(system),
        "conductor.store.StoreStatus" => run_conductor_store_store_status(system),
        "conductor.token.ContributeToken" => match admit(caller, "conductor.token.ContributeToken") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_token_contribute_token(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.token.LeaseToken" => match admit(caller, "conductor.token.LeaseToken") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_token_lease_token(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.token.MarkTokenLimited" => match admit(caller, "conductor.token.MarkTokenLimited") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_token_mark_token_limited(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.token.ReleaseToken" => match admit(caller, "conductor.token.ReleaseToken") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_token_release_token(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.token.Tokens" => run_conductor_token_tokens(system),
        "conductor.token.WithdrawToken" => match admit(caller, "conductor.token.WithdrawToken") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_token_withdraw_token(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        other => return (Err(entry::Refused::Unknown(other.to_owned())), taken),
    };
    let body = match answered {
        Ok((_, body)) => body,
        Err(refused) => return (Err(refused), taken),
    };
    let read = entry::read(&body).map_err(|error| {
        if matches!(name, "conductor.config.PutConfig" | "conductor.instance.ObserveInstance" | "conductor.instance.PurgeInstance" | "conductor.instance.Reconcile" | "conductor.instance.RestartInstance" | "conductor.instance.StartInstance" | "conductor.instance.StopInstance" | "conductor.instance.UpgradeInstance" | "conductor.operator.JoinInstance" | "conductor.operator.LeaveInstance" | "conductor.placement.CloseBox" | "conductor.placement.OpenBox" | "conductor.secret.PutSecret" | "conductor.secret.ResolveSecrets" | "conductor.store.MigrateStore" | "conductor.token.ContributeToken" | "conductor.token.LeaseToken" | "conductor.token.MarkTokenLimited" | "conductor.token.ReleaseToken" | "conductor.token.WithdrawToken") {
            entry::Refused::Undelivered(format!(
                "delivering what the command published: its answer is not JSON this surface reads back: {error}"
            ))
        } else {
            entry::Refused::Unmet(format!(
                "unmet obligation: view_query `{name}` answered rows this surface cannot read back: {error}"
            ))
        }
    });
    (read, taken)
}

/// Nothing, where `caller` may invoke `command`; otherwise the actor the standard refusal names,
/// `None` where the request was authenticated as no actor.
///
/// Checked before the command runs, on the caller the realization authenticated the request
/// as and never on anything the request says about itself. Public, so code that drives the system
/// in process checks the grant exactly as every route does.
pub fn admit(caller: Option<&crate::actor::Caller>, command: &str) -> Result<(), Option<&'static str>> {
    match caller {
        Some(caller) if caller.may(command) => Ok(()),
        Some(caller) => Err(Some(caller.actor.name())),
        None => Err(None),
    }
}

/// The standard refusal for an actor no grant admits, as the contract declares it: `403`,
/// `{"refused": "not granted", "actor": <name or null>}`.
fn not_granted(actor: Option<&str>) -> http::Response {
    let mut body = String::from("{");
    json::member(&mut body, "refused");
    json::push_text(&mut body, "not granted");
    json::member(&mut body, "actor");
    match actor {
        Some(actor) => json::push_text(&mut body, actor),
        None => body.push_str("null"),
    }
    body.push('}');
    http::Response::new(403, http::JSON, body)
}

/// Nothing, where `caller` may read `view`, a view some actor's grant names; otherwise the actor
/// the standard refusal names, `None` where the request was authenticated as no actor.
///
/// Checked before the view is read, on the caller the realization authenticated the request as.
/// Public, so code that reads the system in process checks the grant exactly as every route does.
pub fn admit_read(caller: Option<&crate::actor::Caller>, view: &str) -> Result<(), Option<&'static str>> {
    match caller {
        Some(caller) if caller.may_read(view) => Ok(()),
        Some(caller) => Err(Some(caller.actor.name())),
        None => Err(None),
    }
}

/// `POST` `conductor.config.PutConfig`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_config_put_config<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_config_put_config(system, &value, taken))
}

/// `conductor.config.PutConfig` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_config_put_config<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_config_put_config(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.put_config(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_config_put_config(&outcome))
}

/// One declared outcome of `conductor.config.PutConfig`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_config_put_config(outcome: &crate::config::PutConfigOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::config::PutConfigOutcome::LevelScopeMismatch { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "level-scope-mismatch");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.config.LevelScopeMismatch");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_config_level_scope_mismatch(error, &mut body);
            422
        }
        crate::config::PutConfigOutcome::ActorNotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "actor-not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            502
        }
        crate::config::PutConfigOutcome::Invalid { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "invalid");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.config.ConfigInvalid");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_config_config_invalid(error, &mut body);
            502
        }
        crate::config::PutConfigOutcome::Changed { config_changed, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "changed");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.config.ConfigChanged");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_config_config_changed(config_changed, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.config.ConfigRevisions` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_config_config_revisions<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.config_revisions() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_config_config_revisions(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `conductor.config.EffectiveSettings`: reads the declared parameters from the query string by their wire names, runs the
/// port, answers its rows. A key the view does not declare is ignored.
fn serve_conductor_config_effective_settings<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, query: &str) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match http::query_object(query, &[("instance", http::Scalar::Text)], "query") {
        Ok(value) => http::answer(run_conductor_config_effective_settings(system, &value)),
        Err(refused) => http::Response::refusal(400, &refused),
    }
}

/// `GET` `conductor.config.EffectiveSettings` at `eventual` consistency: every row the owed projection holds for the declared
/// parameters, read from `value`, an object keyed by their wire names.
///
/// The one path the `GET` route and [`handle`] share. A decoding failure is located under
/// `query`, as the route reports it.
fn run_conductor_config_effective_settings<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let (param0,) = match wire::decode_params_conductor_config_effective_settings(value, "query") {
        Ok(params) => params,
        Err(error) => return Err(entry::Refused::Input(format!("{error}"))),
    };
    match system.conductord.effective_settings(param0) {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_config_effective_settings(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `POST` `conductor.instance.ObserveInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_instance_observe_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_instance_observe_instance(system, &value, taken))
}

/// `conductor.instance.ObserveInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_instance_observe_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_instance_observe_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.observe_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_instance_observe_instance(&outcome))
}

/// One declared outcome of `conductor.instance.ObserveInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_instance_observe_instance(outcome: &crate::instance::ObserveInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::instance::ObserveInstanceOutcome::Unknown { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.UnknownInstance");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_unknown_instance(error, &mut body);
            404
        }
        crate::instance::ObserveInstanceOutcome::Answering { instance_answered, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "answering");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceAnswered");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_answered(instance_answered, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::ObserveInstanceOutcome::NotAnswering { instance_unreachable, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-answering");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceUnreachable");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_unreachable(instance_unreachable, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::ObserveInstanceOutcome::NotRunning { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-running");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.InstanceStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_instance_state_conflict(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.instance.PurgeInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_instance_purge_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_instance_purge_instance(system, &value, taken))
}

/// `conductor.instance.PurgeInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_instance_purge_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_instance_purge_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.purge_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_instance_purge_instance(&outcome))
}

/// One declared outcome of `conductor.instance.PurgeInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_instance_purge_instance(outcome: &crate::instance::PurgeInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::instance::PurgeInstanceOutcome::ActorNotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "actor-not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            422
        }
        crate::instance::PurgeInstanceOutcome::Unknown { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.UnknownInstance");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_unknown_instance(error, &mut body);
            404
        }
        crate::instance::PurgeInstanceOutcome::Purged { instance_purged, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "purged");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstancePurged");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_purged(instance_purged, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::PurgeInstanceOutcome::NotStopped { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-stopped");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.InstanceStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_instance_state_conflict(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.instance.Reconcile`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_instance_reconcile<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_instance_reconcile(system, &value, taken))
}

/// `conductor.instance.Reconcile` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_instance_reconcile<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_instance_reconcile(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.reconcile(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_instance_reconcile(&outcome))
}

/// One declared outcome of `conductor.instance.Reconcile`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_instance_reconcile(outcome: &crate::instance::ReconcileOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::instance::ReconcileOutcome::Recorded { instance_reconciled, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "recorded");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceReconciled");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_reconciled(instance_reconciled, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.instance.RestartInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_instance_restart_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_instance_restart_instance(system, &value, taken))
}

/// `conductor.instance.RestartInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_instance_restart_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_instance_restart_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.restart_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_instance_restart_instance(&outcome))
}

/// One declared outcome of `conductor.instance.RestartInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_instance_restart_instance(outcome: &crate::instance::RestartInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::instance::RestartInstanceOutcome::Unknown { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.UnknownInstance");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_unknown_instance(error, &mut body);
            404
        }
        crate::instance::RestartInstanceOutcome::PlacementRefused { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "placement-refused");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.PlacementRefused");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_placement_refused(error, &mut body);
            502
        }
        crate::instance::RestartInstanceOutcome::Restarted { instance_restarted, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "restarted");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceRestarted");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_restarted(instance_restarted, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::RestartInstanceOutcome::NotRunning { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-running");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.InstanceStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_instance_state_conflict(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.instance.StartInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_instance_start_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_instance_start_instance(system, &value, taken))
}

/// `conductor.instance.StartInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_instance_start_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_instance_start_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.start_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_instance_start_instance(&outcome))
}

/// One declared outcome of `conductor.instance.StartInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_instance_start_instance(outcome: &crate::instance::StartInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::instance::StartInstanceOutcome::Created { instance_started, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "created");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceStarted");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_started(instance_started, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::StartInstanceOutcome::Started { instance_started, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "started");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceStarted");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_started(instance_started, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::StartInstanceOutcome::AlreadyStarted => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-started");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
        crate::instance::StartInstanceOutcome::NotInConfig { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-in-config");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.UnknownInstance");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_unknown_instance(error, &mut body);
            502
        }
        crate::instance::StartInstanceOutcome::PlacementRefused { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "placement-refused");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.PlacementRefused");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_placement_refused(error, &mut body);
            502
        }
        crate::instance::StartInstanceOutcome::NotStartable { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-startable");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.InstanceStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_instance_state_conflict(error, &mut body);
            422
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.instance.StopInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_instance_stop_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_instance_stop_instance(system, &value, taken))
}

/// `conductor.instance.StopInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_instance_stop_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_instance_stop_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.stop_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_instance_stop_instance(&outcome))
}

/// One declared outcome of `conductor.instance.StopInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_instance_stop_instance(outcome: &crate::instance::StopInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::instance::StopInstanceOutcome::Unknown { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.UnknownInstance");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_unknown_instance(error, &mut body);
            404
        }
        crate::instance::StopInstanceOutcome::Stopped { instance_stopped, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "stopped");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceStopped");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_stopped(instance_stopped, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::StopInstanceOutcome::AlreadyStopped => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-stopped");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
        crate::instance::StopInstanceOutcome::Purged { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "purged");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.InstanceStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_instance_state_conflict(error, &mut body);
            422
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.instance.UpgradeInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_instance_upgrade_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_instance_upgrade_instance(system, &value, taken))
}

/// `conductor.instance.UpgradeInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_instance_upgrade_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_instance_upgrade_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.upgrade_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_instance_upgrade_instance(&outcome))
}

/// One declared outcome of `conductor.instance.UpgradeInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_instance_upgrade_instance(outcome: &crate::instance::UpgradeInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::instance::UpgradeInstanceOutcome::Unknown { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.UnknownInstance");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_unknown_instance(error, &mut body);
            404
        }
        crate::instance::UpgradeInstanceOutcome::Upgrading { instance_upgrading, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "upgrading");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceUpgrading");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_upgrading(instance_upgrading, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::UpgradeInstanceOutcome::RolledBack { instance_rolled_back, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "rolled-back");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.instance.InstanceRolledBack");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_instance_instance_rolled_back(instance_rolled_back, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::instance::UpgradeInstanceOutcome::NotRunning { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-running");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.instance.InstanceStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_instance_instance_state_conflict(error, &mut body);
            422
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.instance.Instances` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_instance_instances<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.instances() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_instance_instances(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `conductor.instance.Reconciliations` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_instance_reconciliations<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.reconciliations() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_instance_reconciliations(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `POST` `conductor.operator.JoinInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_operator_join_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_operator_join_instance(system, &value, taken))
}

/// `conductor.operator.JoinInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_operator_join_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_operator_join_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.join_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_operator_join_instance(&outcome))
}

/// One declared outcome of `conductor.operator.JoinInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_operator_join_instance(outcome: &crate::operator::JoinInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::operator::JoinInstanceOutcome::ActorNotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "actor-not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            422
        }
        crate::operator::JoinInstanceOutcome::Joined { operator_joined, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "joined");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.operator.OperatorJoined");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_operator_operator_joined(operator_joined, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::operator::JoinInstanceOutcome::Rejoined { operator_joined, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "rejoined");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.operator.OperatorJoined");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_operator_operator_joined(operator_joined, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::operator::JoinInstanceOutcome::AlreadyJoined { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-joined");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.AlreadyJoined");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_already_joined(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.operator.LeaveInstance`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_operator_leave_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_operator_leave_instance(system, &value, taken))
}

/// `conductor.operator.LeaveInstance` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_operator_leave_instance<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_operator_leave_instance(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.leave_instance(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_operator_leave_instance(&outcome))
}

/// One declared outcome of `conductor.operator.LeaveInstance`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_operator_leave_instance(outcome: &crate::operator::LeaveInstanceOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::operator::LeaveInstanceOutcome::ActorNotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "actor-not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            422
        }
        crate::operator::LeaveInstanceOutcome::Left { operator_left, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "left");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.operator.OperatorLeft");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_operator_operator_left(operator_left, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::operator::LeaveInstanceOutcome::NotJoined { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-joined");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.NotJoined");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_not_joined(error, &mut body);
            409
        }
        crate::operator::LeaveInstanceOutcome::UnknownOperator { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-operator");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.NotJoined");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_not_joined(error, &mut body);
            404
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.operator.Operators` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_operator_operators<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.operators() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_operator_operators(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `POST` `conductor.placement.CloseBox`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_placement_close_box<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_placement_close_box(system, &value, taken))
}

/// `conductor.placement.CloseBox` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_placement_close_box<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_placement_close_box(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.close_box(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_placement_close_box(&outcome))
}

/// One declared outcome of `conductor.placement.CloseBox`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_placement_close_box(outcome: &crate::placement::CloseBoxOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::placement::CloseBoxOutcome::Closed { box_closed, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "closed");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.placement.BoxClosed");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_placement_box_closed(box_closed, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::placement::CloseBoxOutcome::NoSuchBox { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-such-box");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.placement.BoxNotFound");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_placement_box_not_found(error, &mut body);
            404
        }
        crate::placement::CloseBoxOutcome::WrongState { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.placement.BoxStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_placement_box_state_conflict(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.placement.OpenBox`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_placement_open_box<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_placement_open_box(system, &value, taken))
}

/// `conductor.placement.OpenBox` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_placement_open_box<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_placement_open_box(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.open_box(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_placement_open_box(&outcome))
}

/// One declared outcome of `conductor.placement.OpenBox`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_placement_open_box(outcome: &crate::placement::OpenBoxOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::placement::OpenBoxOutcome::Refused { box_refused, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "refused");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.placement.BoxRefused");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_placement_box_refused(box_refused, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::placement::OpenBoxOutcome::AdapterRefused { box_refused, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "adapter-refused");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.placement.BoxRefused");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_placement_box_refused(box_refused, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::placement::OpenBoxOutcome::Opened { box_opened, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "opened");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.placement.BoxOpened");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_placement_box_opened(box_opened, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.placement.BoxRefusals` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_placement_box_refusals<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.box_refusals() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_placement_box_refusals(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `conductor.placement.Boxes` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_placement_boxes<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.boxes() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_placement_boxes(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `conductor.placement.Placements` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_placement_placements<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.placements() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_placement_placements(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `POST` `conductor.secret.PutSecret`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_secret_put_secret<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_secret_put_secret(system, &value, taken))
}

/// `conductor.secret.PutSecret` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_secret_put_secret<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_secret_put_secret(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.put_secret(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_secret_put_secret(&outcome))
}

/// One declared outcome of `conductor.secret.PutSecret`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_secret_put_secret(outcome: &crate::secret::PutSecretOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::secret::PutSecretOutcome::NotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            422
        }
        crate::secret::PutSecretOutcome::Unreachable { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unreachable");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.secret.SourceUnreachable");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_secret_source_unreachable(error, &mut body);
            502
        }
        crate::secret::PutSecretOutcome::Stored { secret_put, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "stored");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.secret.SecretPut");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_secret_secret_put(secret_put, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::secret::PutSecretOutcome::Replaced { secret_put, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "replaced");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.secret.SecretPut");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_secret_secret_put(secret_put, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.secret.ResolveSecrets`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_secret_resolve_secrets<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_secret_resolve_secrets(system, &value, taken))
}

/// `conductor.secret.ResolveSecrets` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_secret_resolve_secrets<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_secret_resolve_secrets(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.resolve_secrets(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_secret_resolve_secrets(&outcome))
}

/// One declared outcome of `conductor.secret.ResolveSecrets`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_secret_resolve_secrets(outcome: &crate::secret::ResolveSecretsOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::secret::ResolveSecretsOutcome::Unreachable { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unreachable");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.secret.SourceUnreachable");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_secret_source_unreachable(error, &mut body);
            502
        }
        crate::secret::ResolveSecretsOutcome::Missing { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "missing");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.secret.SecretMissing");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_secret_secret_missing(error, &mut body);
            502
        }
        crate::secret::ResolveSecretsOutcome::Resolved { secrets_resolved, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "resolved");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.secret.SecretsResolved");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_secret_secrets_resolved(secrets_resolved, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.secret.Secrets` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_secret_secrets<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.secrets() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_secret_secrets(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `POST` `conductor.store.MigrateStore`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_store_migrate_store<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_store_migrate_store(system, &value, taken))
}

/// `conductor.store.MigrateStore` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_store_migrate_store<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_store_migrate_store(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.migrate_store(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_store_migrate_store(&outcome))
}

/// One declared outcome of `conductor.store.MigrateStore`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_store_migrate_store(outcome: &crate::store::MigrateStoreOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::store::MigrateStoreOutcome::NotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            422
        }
        crate::store::MigrateStoreOutcome::NegativeCount { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "negative-count");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.store.InvalidEventCount");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_store_invalid_event_count(error, &mut body);
            422
        }
        crate::store::MigrateStoreOutcome::AlreadyMigrated { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "already-migrated");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.store.AlreadyMigrated");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_store_already_migrated(error, &mut body);
            409
        }
        crate::store::MigrateStoreOutcome::SourceMissing { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "source-missing");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.store.SourceMissing");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_store_source_missing(error, &mut body);
            502
        }
        crate::store::MigrateStoreOutcome::Migrated { store_migrated, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "migrated");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.store.StoreMigrated");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_store_store_migrated(store_migrated, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.store.Migrations` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_store_migrations<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.migrations() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_store_migrations(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `conductor.store.StoreStatus` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_store_store_status<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.store_status() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_store_store_status(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `POST` `conductor.token.ContributeToken`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_token_contribute_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_token_contribute_token(system, &value, taken))
}

/// `conductor.token.ContributeToken` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_token_contribute_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_token_contribute_token(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.contribute_token(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_token_contribute_token(&outcome))
}

/// One declared outcome of `conductor.token.ContributeToken`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_token_contribute_token(outcome: &crate::token::ContributeTokenOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::token::ContributeTokenOutcome::ActorNotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "actor-not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            422
        }
        crate::token::ContributeTokenOutcome::Contributed { token_contributed, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "contributed");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.token.TokenContributed");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_token_token_contributed(token_contributed, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.token.LeaseToken`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_token_lease_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_token_lease_token(system, &value, taken))
}

/// `conductor.token.LeaseToken` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_token_lease_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_token_lease_token(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.lease_token(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_token_lease_token(&outcome))
}

/// One declared outcome of `conductor.token.LeaseToken`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_token_lease_token(outcome: &crate::token::LeaseTokenOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::token::LeaseTokenOutcome::NoneAvailable { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "none-available");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.NoTokenAvailable");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_token_no_token_available(error, &mut body);
            502
        }
        crate::token::LeaseTokenOutcome::Leased { token_leased, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "leased");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.token.TokenLeased");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_token_token_leased(token_leased, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::token::LeaseTokenOutcome::WrongState { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_token_token_state_conflict(error, &mut body);
            409
        }
        crate::token::LeaseTokenOutcome::WrongStateUnknownInstance => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push_str("[]");
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.token.MarkTokenLimited`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_token_mark_token_limited<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_token_mark_token_limited(system, &value, taken))
}

/// `conductor.token.MarkTokenLimited` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_token_mark_token_limited<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_token_mark_token_limited(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.mark_token_limited(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_token_mark_token_limited(&outcome))
}

/// One declared outcome of `conductor.token.MarkTokenLimited`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_token_mark_token_limited(outcome: &crate::token::MarkTokenLimitedOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::token::MarkTokenLimitedOutcome::Limited { token_limited, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "limited");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.token.TokenLimited");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_token_token_limited(token_limited, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::token::MarkTokenLimitedOutcome::WrongState { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_token_token_state_conflict(error, &mut body);
            409
        }
        crate::token::MarkTokenLimitedOutcome::WrongStateUnknownInstance => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push_str("[]");
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.token.ReleaseToken`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_token_release_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_token_release_token(system, &value, taken))
}

/// `conductor.token.ReleaseToken` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_token_release_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_token_release_token(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.release_token(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_token_release_token(&outcome))
}

/// One declared outcome of `conductor.token.ReleaseToken`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_token_release_token(outcome: &crate::token::ReleaseTokenOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::token::ReleaseTokenOutcome::Released { token_released, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "released");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.token.TokenReleased");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_token_token_released(token_released, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::token::ReleaseTokenOutcome::WrongState { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_token_token_state_conflict(error, &mut body);
            409
        }
        crate::token::ReleaseTokenOutcome::WrongStateUnknownInstance => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push_str("[]");
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.token.WithdrawToken`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_token_withdraw_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not UTF-8: {error}"));
        }
    };
    let value = match json::parse(text) {
        Ok(value) => value,
        Err(error) => {
            return http::Response::refusal(400, &format!("the body is not JSON: {error}"));
        }
    };
    http::answer(run_conductor_token_withdraw_token(system, &value, taken))
}

/// `conductor.token.WithdrawToken` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_token_withdraw_token<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_token_withdraw_token(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductord.withdraw_token(input) {
        Ok(outcome) => outcome,
        Err(unmet) => return Err(entry::Refused::Unmet(format!("{unmet}"))),
    };
    // Deliver what this command published to every binding that reacts to it, then take it
    // off the log: a long-running server keeps nothing from one request to the next.
    let delivered = system.pump();
    taken.extend(system.take_published().into_iter().skip(before));
    if let Err(failure) = delivered {
        return Err(entry::Refused::Undelivered(format!("delivering what the command published: {failure}")));
    }
    Ok(answer_conductor_token_withdraw_token(&outcome))
}

/// One declared outcome of `conductor.token.WithdrawToken`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_token_withdraw_token(outcome: &crate::token::WithdrawTokenOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::token::WithdrawTokenOutcome::ActorNotHuman { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "actor-not-human");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.operator.ActorNotHuman");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_operator_actor_not_human(error, &mut body);
            422
        }
        crate::token::WithdrawTokenOutcome::Withdrawn { token_withdrawn, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "withdrawn");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.token.TokenWithdrawn");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_token_token_withdrawn(token_withdrawn, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::token::WithdrawTokenOutcome::WrongState { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_token_token_state_conflict(error, &mut body);
            409
        }
        crate::token::WithdrawTokenOutcome::WrongStateUnknownInstance => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "wrong-state");
            json::member(&mut body, "published");
            body.push_str("[]");
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.token.TokenStateConflict");
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.token.Tokens` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_token_tokens<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductord.tokens() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_token_tokens(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}
