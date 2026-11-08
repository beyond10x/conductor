// generated from conductor v1
// contract digest 0608eb83c8c48e1f8808cd29d3621f78091b34cac43c2b2e9143a335707a4202
// do not edit: regenerate with `ess synthesize --layout crate`

//! The `conductor-instance` component of `conductor` v1, on the wire.
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
pub const OPENAPI: &str = include_str!("conductor-instance.openapi.json");

/// The prose the same model produced, byte for byte as the documentation projection wrote it.
pub const DOCS: &str = include_str!("conductor-instance.docs.md");

/// Every route this surface answers, in path order.
///
/// The same set the `OpenAPI` document declares, plus the two documents about the surface
/// itself, which no specification construct names and nothing can therefore derive. A path
/// absent from this table is answered with `404`, including one the document declares and this
/// table forgot — which is the failure a table computed twice would hide.
pub const ROUTES: &[(&str, &str)] = &[
    ("POST", "/agent/commands/attach"),
    ("POST", "/agent/commands/declare-kind"),
    ("POST", "/agent/commands/deliver"),
    ("POST", "/agent/commands/record-report"),
    ("POST", "/agent/commands/restart"),
    ("POST", "/agent/commands/start"),
    ("POST", "/agent/commands/stop"),
    ("GET", "/agent/views/agent-kinds"),
    ("GET", "/agent/views/agents"),
    ("GET", "/docs"),
    ("GET", "/openapi.json"),
];

/// What this process says about itself as it starts, before it answers anything.
///
/// Three lines of JSON on standard output, in this order, every member of them derived from the
/// specification — except `runtime`, which is appended by the emitted code below and holds what
/// is true of *this process*: the language it was synthesised into, and the address it bound.
/// Everything outside `runtime` is the same in every language this plan is emitted into, and
/// `cargo xtask synth --check` starts both and compares them.
pub const STARTUP: &[&str] = &[
    "{\"log\":\"ess/1\",\"event\":\"system.starting\",\"system\":\"conductor\",\"version\":\"v1\",\"model_digest\":\"1bfe189965973bb766214957260f7935cfea2caa2547b0d97577a81fd5de6d11\",\"contract_digest\":\"0608eb83c8c48e1f8808cd29d3621f78091b34cac43c2b2e9143a335707a4202\",\"components\":[\"conductor-broker\",\"conductor-cli\",\"conductor-instance\",\"conductord\"],\"capabilities\":{\"generated\":959,\"obligations\":12,\"refused\":0}",
    "{\"log\":\"ess/1\",\"event\":\"surface.serving\",\"component\":\"conductor-instance\",\"reached_by\":\"network\",\"transport\":\"http/1.1\",\"routes\":11,\"paths\":[{\"method\":\"POST\",\"path\":\"/agent/commands/attach\",\"serves\":\"command\",\"name\":\"conductor.agent.AttachAgent\"},{\"method\":\"POST\",\"path\":\"/agent/commands/declare-kind\",\"serves\":\"command\",\"name\":\"conductor.agent.DeclareAgentKind\"},{\"method\":\"POST\",\"path\":\"/agent/commands/deliver\",\"serves\":\"command\",\"name\":\"conductor.agent.DeliverToAgent\"},{\"method\":\"POST\",\"path\":\"/agent/commands/record-report\",\"serves\":\"command\",\"name\":\"conductor.agent.RecordAgentReport\"},{\"method\":\"POST\",\"path\":\"/agent/commands/restart\",\"serves\":\"command\",\"name\":\"conductor.agent.RestartAgent\"},{\"method\":\"POST\",\"path\":\"/agent/commands/start\",\"serves\":\"command\",\"name\":\"conductor.agent.StartAgent\"},{\"method\":\"POST\",\"path\":\"/agent/commands/stop\",\"serves\":\"command\",\"name\":\"conductor.agent.StopAgent\"},{\"method\":\"GET\",\"path\":\"/agent/views/agent-kinds\",\"serves\":\"view\",\"name\":\"conductor.agent.AgentKinds\"},{\"method\":\"GET\",\"path\":\"/agent/views/agents\",\"serves\":\"view\",\"name\":\"conductor.agent.Agents\"},{\"method\":\"GET\",\"path\":\"/docs\",\"serves\":\"documentation\",\"name\":\"docs\"},{\"method\":\"GET\",\"path\":\"/openapi.json\",\"serves\":\"contract\",\"name\":\"openapi\"}]",
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

/// Serves `conductor-instance` at `address`, and does not return while it can answer.
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
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match request.path.as_str() {
        "/agent/commands/attach" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.agent.AttachAgent") {
                return not_granted(actor);
            }
            serve_conductor_agent_attach_agent(system, &request.body, taken)
        }
        "/agent/commands/declare-kind" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.agent.DeclareAgentKind") {
                return not_granted(actor);
            }
            serve_conductor_agent_declare_agent_kind(system, &request.body, taken)
        }
        "/agent/commands/deliver" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.agent.DeliverToAgent") {
                return not_granted(actor);
            }
            serve_conductor_agent_deliver_to_agent(system, &request.body, taken)
        }
        "/agent/commands/record-report" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.agent.RecordAgentReport") {
                return not_granted(actor);
            }
            serve_conductor_agent_record_agent_report(system, &request.body, taken)
        }
        "/agent/commands/restart" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.agent.RestartAgent") {
                return not_granted(actor);
            }
            serve_conductor_agent_restart_agent(system, &request.body, taken)
        }
        "/agent/commands/start" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.agent.StartAgent") {
                return not_granted(actor);
            }
            serve_conductor_agent_start_agent(system, &request.body, taken)
        }
        "/agent/commands/stop" => {
            if request.method != "POST" {
                return http::method_not_allowed("POST");
            }
            if let Err(actor) = admit(caller, "conductor.agent.StopAgent") {
                return not_granted(actor);
            }
            serve_conductor_agent_stop_agent(system, &request.body, taken)
        }
        "/agent/views/agent-kinds" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "conductor.agent.AgentKinds") {
                return not_granted(actor);
            }
            http::answer(run_conductor_agent_agent_kinds(system))
        }
        "/agent/views/agents" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            if let Err(actor) = admit_read(caller, "conductor.agent.Agents") {
                return not_granted(actor);
            }
            http::answer(run_conductor_agent_agents(system))
        }
        "/docs" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::Response::new(200, http::MARKDOWN, DOCS)
        }
        "/openapi.json" => {
            if request.method != "GET" {
                return http::method_not_allowed("GET");
            }
            http::Response::new(200, http::JSON, OPENAPI)
        }
        other => http::Response::refusal(
            404,
            &format!("`{other}` is not a path this surface declares; `GET /openapi.json` publishes every one that is"),
        ),
    }
}

/// Runs one command or view of `conductor-instance` by its qualified name, with no transport.
///
/// The same decoding, refusals and rendering the HTTP routes use — each route and this function call
/// one `run_*` function — so a conformance runner or an in-process caller drives the system
/// without a socket and without a dispatch table of its own. `Ok` is the declared outcome, as the
/// route's body renders it; a view ignores `input`, as its `GET` route ignores a body.
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
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
        "conductor.agent.AgentKinds" => match admit_read(caller, "conductor.agent.AgentKinds") {
            Ok(()) => run_conductor_agent_agent_kinds(system),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.Agents" => match admit_read(caller, "conductor.agent.Agents") {
            Ok(()) => run_conductor_agent_agents(system),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.AttachAgent" => match admit(caller, "conductor.agent.AttachAgent") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_agent_attach_agent(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.DeclareAgentKind" => match admit(caller, "conductor.agent.DeclareAgentKind") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_agent_declare_agent_kind(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.DeliverToAgent" => match admit(caller, "conductor.agent.DeliverToAgent") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_agent_deliver_to_agent(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.RecordAgentReport" => match admit(caller, "conductor.agent.RecordAgentReport") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_agent_record_agent_report(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.RestartAgent" => match admit(caller, "conductor.agent.RestartAgent") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_agent_restart_agent(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.StartAgent" => match admit(caller, "conductor.agent.StartAgent") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_agent_start_agent(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        "conductor.agent.StopAgent" => match admit(caller, "conductor.agent.StopAgent") {
            Ok(()) => checked(&input).and_then(|()| run_conductor_agent_stop_agent(system, &input, &mut taken)),
            Err(actor) => Err(entry::Refused::NotGranted(actor.map(str::to_owned))),
        },
        other => return (Err(entry::Refused::Unknown(other.to_owned())), taken),
    };
    let body = match answered {
        Ok((_, body)) => body,
        Err(refused) => return (Err(refused), taken),
    };
    let read = entry::read(&body).map_err(|error| {
        if matches!(name, "conductor.agent.AttachAgent" | "conductor.agent.DeclareAgentKind" | "conductor.agent.DeliverToAgent" | "conductor.agent.RecordAgentReport" | "conductor.agent.RestartAgent" | "conductor.agent.StartAgent" | "conductor.agent.StopAgent") {
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

/// `POST` `conductor.agent.AttachAgent`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_agent_attach_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    http::answer(run_conductor_agent_attach_agent(system, &value, taken))
}

/// `conductor.agent.AttachAgent` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_agent_attach_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_agent_attach_agent(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductor_instance.attach_agent(input) {
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
    Ok(answer_conductor_agent_attach_agent(&outcome))
}

/// One declared outcome of `conductor.agent.AttachAgent`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_agent_attach_agent(outcome: &crate::agent::AttachAgentOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::agent::AttachAgentOutcome::NotInteractive { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-interactive");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.CapabilityMissing");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_capability_missing(error, &mut body);
            409
        }
        crate::agent::AttachAgentOutcome::NotRunning { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-running");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotRunning");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_running(error, &mut body);
            409
        }
        crate::agent::AttachAgentOutcome::Unreachable { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unreachable");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotReachable");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_reachable(error, &mut body);
            502
        }
        crate::agent::AttachAgentOutcome::Attached => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "attached");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
        crate::agent::AttachAgentOutcome::NoSuchAgent { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-such-agent");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotFound");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_found(error, &mut body);
            404
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.agent.DeclareAgentKind`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_agent_declare_agent_kind<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    http::answer(run_conductor_agent_declare_agent_kind(system, &value, taken))
}

/// `conductor.agent.DeclareAgentKind` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_agent_declare_agent_kind<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_agent_declare_agent_kind(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductor_instance.declare_agent_kind(input) {
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
    Ok(answer_conductor_agent_declare_agent_kind(&outcome))
}

/// One declared outcome of `conductor.agent.DeclareAgentKind`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_agent_declare_agent_kind(outcome: &crate::agent::DeclareAgentKindOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::agent::DeclareAgentKindOutcome::Declared { agent_kind_declared, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "declared");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.agent.AgentKindDeclared");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_agent_agent_kind_declared(agent_kind_declared, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::agent::DeclareAgentKindOutcome::Redeclared { agent_kind_declared, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "redeclared");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.agent.AgentKindDeclared");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_agent_agent_kind_declared(agent_kind_declared, &mut body);
            body.push('}');
            body.push(']');
            202
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.agent.DeliverToAgent`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_agent_deliver_to_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    http::answer(run_conductor_agent_deliver_to_agent(system, &value, taken))
}

/// `conductor.agent.DeliverToAgent` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_agent_deliver_to_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_agent_deliver_to_agent(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductor_instance.deliver_to_agent(input) {
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
    Ok(answer_conductor_agent_deliver_to_agent(&outcome))
}

/// One declared outcome of `conductor.agent.DeliverToAgent`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_agent_deliver_to_agent(outcome: &crate::agent::DeliverToAgentOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::agent::DeliverToAgentOutcome::NotInteractive { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-interactive");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.CapabilityMissing");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_capability_missing(error, &mut body);
            409
        }
        crate::agent::DeliverToAgentOutcome::NotRunning { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-running");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotRunning");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_running(error, &mut body);
            409
        }
        crate::agent::DeliverToAgentOutcome::Unreachable { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unreachable");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotReachable");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_reachable(error, &mut body);
            502
        }
        crate::agent::DeliverToAgentOutcome::Delivered => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "delivered");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            202
        }
        crate::agent::DeliverToAgentOutcome::NoSuchAgent { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-such-agent");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotFound");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_found(error, &mut body);
            404
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.agent.RecordAgentReport`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_agent_record_agent_report<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    http::answer(run_conductor_agent_record_agent_report(system, &value, taken))
}

/// `conductor.agent.RecordAgentReport` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_agent_record_agent_report<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_agent_record_agent_report(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductor_instance.record_agent_report(input) {
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
    Ok(answer_conductor_agent_record_agent_report(&outcome))
}

/// One declared outcome of `conductor.agent.RecordAgentReport`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_agent_record_agent_report(outcome: &crate::agent::RecordAgentReportOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::agent::RecordAgentReportOutcome::Reported { agent_reported, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "reported");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.agent.AgentReported");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_agent_agent_reported(agent_reported, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::agent::RecordAgentReportOutcome::NoSuchAgent { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-such-agent");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotFound");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_found(error, &mut body);
            404
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.agent.RestartAgent`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_agent_restart_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    http::answer(run_conductor_agent_restart_agent(system, &value, taken))
}

/// `conductor.agent.RestartAgent` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_agent_restart_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_agent_restart_agent(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductor_instance.restart_agent(input) {
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
    Ok(answer_conductor_agent_restart_agent(&outcome))
}

/// One declared outcome of `conductor.agent.RestartAgent`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_agent_restart_agent(outcome: &crate::agent::RestartAgentOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::agent::RestartAgentOutcome::LaunchFailed { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "launch-failed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentLaunchFailed");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_launch_failed(error, &mut body);
            502
        }
        crate::agent::RestartAgentOutcome::Restarted { agent_started, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "restarted");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.agent.AgentStarted");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_agent_agent_started(agent_started, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::agent::RestartAgentOutcome::NoSuchAgent { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-such-agent");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotFound");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_found(error, &mut body);
            404
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.agent.StartAgent`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_agent_start_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    http::answer(run_conductor_agent_start_agent(system, &value, taken))
}

/// `conductor.agent.StartAgent` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_agent_start_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_agent_start_agent(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductor_instance.start_agent(input) {
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
    Ok(answer_conductor_agent_start_agent(&outcome))
}

/// One declared outcome of `conductor.agent.StartAgent`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_agent_start_agent(outcome: &crate::agent::StartAgentOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::agent::StartAgentOutcome::UnknownKind { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "unknown-kind");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.UnknownAgentKind");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_unknown_agent_kind(error, &mut body);
            409
        }
        crate::agent::StartAgentOutcome::LaunchFailed { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "launch-failed");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentLaunchFailed");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_launch_failed(error, &mut body);
            502
        }
        crate::agent::StartAgentOutcome::Started { agent_started, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "started");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.agent.AgentStarted");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_agent_agent_started(agent_started, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::agent::StartAgentOutcome::Exists { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "exists");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentExists");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_exists(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `POST` `conductor.agent.StopAgent`: reads the declared input, runs the port, answers the declared outcome.
fn serve_conductor_agent_stop_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, body: &[u8], taken: &mut Vec<crate::system::SystemEvent>) -> http::Response
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
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
    http::answer(run_conductor_agent_stop_agent(system, &value, taken))
}

/// `conductor.agent.StopAgent` from its input as a JSON value: decode, run the port, render the declared outcome.
///
/// The one path the `POST` route and [`handle`] share. A decoding failure is located under
/// `body`, as the route reports it. Every event published while the call runs is pushed onto
/// `taken`, in log order, whatever it answers; what was on the log before it is taken off
/// the log but not pushed.
fn run_conductor_agent_stop_agent<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &mut crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>, value: &json::Value, taken: &mut Vec<crate::system::SystemEvent>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    // What is on the log before this call is not what this call published: it is delivered
    // and taken with the rest, and never pushed onto `taken`.
    let before = system.published().len();
    let input = match wire::decode_command_conductor_agent_stop_agent(value, "body") {
        Ok(input) => input,
        Err(error) => {
            // `400` and not `422`: this is a body the schema decides, which is the difference
            // between fixing a value and fixing a serialiser.
            return Err(entry::Refused::Input(format!("{error}")));
        }
    };
    let outcome = match system.conductor_instance.stop_agent(input) {
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
    Ok(answer_conductor_agent_stop_agent(&outcome))
}

/// One declared outcome of `conductor.agent.StopAgent`: the branch that was taken, every event it published in
/// publication order, the declared error where there is one, and that error's own payload —
/// with the status the contract declares for that branch.
fn answer_conductor_agent_stop_agent(outcome: &crate::agent::StopAgentOutcome) -> (u16, String) {
    let mut body = String::from("{");
    let status = match outcome {
        crate::agent::StopAgentOutcome::Stopped { agent_stopped, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "stopped");
            json::member(&mut body, "published");
            body.push('[');
            body.push('{');
            json::member(&mut body, "event");
            json::push_text(&mut body, "conductor.agent.AgentStopped");
            json::member(&mut body, "payload");
            wire::encode_event_conductor_agent_agent_stopped(agent_stopped, &mut body);
            body.push('}');
            body.push(']');
            202
        }
        crate::agent::StopAgentOutcome::NoSuchAgent { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "no-such-agent");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotFound");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_found(error, &mut body);
            404
        }
        crate::agent::StopAgentOutcome::NotRunning { error, .. } => {
            json::member(&mut body, "outcome");
            json::push_text(&mut body, "not-running");
            json::member(&mut body, "published");
            body.push('[');
            body.push(']');
            json::member(&mut body, "error");
            json::push_text(&mut body, "conductor.agent.AgentNotRunning");
            json::member(&mut body, "payload");
            wire::encode_error_conductor_agent_agent_not_running(error, &mut body);
            409
        }
    };
    body.push('}');
    (status, body)
}

/// `GET` `conductor.agent.AgentKinds` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_agent_agent_kinds<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductor_instance.agent_kinds() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_agent_agent_kinds(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}

/// `GET` `conductor.agent.Agents` at `read_your_writes` consistency: every row the owed projection holds.
///
/// The one path the `GET` route and [`handle`] share.
fn run_conductor_agent_agents<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>(system: &crate::system::System<ConductorBrokerBehaviors, ConductorCliBehaviors, ConductorInstanceBehaviors, ConductordBehaviors>) -> Result<(u16, String), entry::Refused>
where
    ConductorBrokerBehaviors: crate::tool::obligations::RecordToolCallBehavior + crate::tool::obligations::ToolCallsQuery,
    ConductorCliBehaviors: crate::decision::obligations::AmendEscalatedRequestBehavior + crate::decision::obligations::AnswerEscalatedRequestBehavior + crate::decision::obligations::AnswerRequestBehavior + crate::decision::obligations::EscalateRequestBehavior + crate::decision::obligations::RaiseDecisionRequestBehavior + crate::decision::obligations::RecordConductorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionBehavior + crate::decision::obligations::RecordOperatorDecisionForRequestBehavior + crate::decision::obligations::ReverseConductorDecisionBehavior + crate::decision::obligations::ReverseOperatorDecisionBehavior + crate::decision::obligations::WithdrawRequestBehavior + crate::direction::obligations::ActivateRepositoryBehavior + crate::direction::obligations::AddServingBehavior + crate::direction::obligations::ConfirmGoalBehavior + crate::direction::obligations::DeactivateRepositoryBehavior + crate::direction::obligations::DropGoalBehavior + crate::direction::obligations::MarkGoalMetBehavior + crate::direction::obligations::MarkRepositoryBehavior + crate::direction::obligations::ProposeGoalBehavior + crate::direction::obligations::RemoveServingBehavior + crate::dispatch::obligations::CancelDispatchBehavior + crate::dispatch::obligations::GrantResourceBehavior + crate::dispatch::obligations::HandleMessageBehavior + crate::dispatch::obligations::PauseControllerBehavior + crate::dispatch::obligations::ReceiveMessageBehavior + crate::dispatch::obligations::RecordGuardDecisionBehavior + crate::dispatch::obligations::RefuseResourceBehavior + crate::dispatch::obligations::RejectMessageBehavior + crate::dispatch::obligations::ReleaseResourceBehavior + crate::dispatch::obligations::ReportBlockedBehavior + crate::dispatch::obligations::ReportDoneBehavior + crate::dispatch::obligations::ReportFailedBehavior + crate::dispatch::obligations::ReportStartedBehavior + crate::dispatch::obligations::ReportUnblockedBehavior + crate::dispatch::obligations::RequestResourceBehavior + crate::dispatch::obligations::ResumeControllerBehavior + crate::dispatch::obligations::ReviseCharterBehavior + crate::dispatch::obligations::RouteNeedBehavior + crate::dispatch::obligations::SendDispatchBehavior + crate::dispatch::obligations::StartControllerBehavior + crate::dispatch::obligations::StopControllerBehavior + crate::host::obligations::ReclaimBehavior + crate::host::obligations::RecordPeakBehavior + crate::host::obligations::ReleaseGrantBehavior + crate::host::obligations::RequestDiskBehavior + crate::host::obligations::ReserveDiskBehavior + crate::host::obligations::StartRunBehavior + crate::host::obligations::TakeHostGrantBehavior + crate::host::obligations::WithdrawDiskRequestBehavior + crate::improvement::obligations::DefineMetricBehavior + crate::improvement::obligations::FileFindingBehavior + crate::improvement::obligations::JudgeMetricReadingBehavior + crate::improvement::obligations::RaiseFindingBehavior + crate::improvement::obligations::RecordMetricReadingBehavior + crate::improvement::obligations::SetMetricTargetBehavior + crate::improvement::obligations::SupersedeMetricReadingBehavior + crate::improvement::obligations::VerifyFindingBehavior + crate::observation::obligations::CompleteSnapshotBehavior + crate::observation::obligations::FailSnapshotBehavior + crate::observation::obligations::PublishBoardBehavior + crate::observation::obligations::RecordBlockerBehavior + crate::observation::obligations::RecordCollectorRunBehavior + crate::observation::obligations::RecordMemberFailureBehavior + crate::observation::obligations::RecordMergedPullRequestBehavior + crate::observation::obligations::RecordPullRequestBehavior + crate::observation::obligations::RecordReleaseBehavior + crate::observation::obligations::RecordRepositoryBehavior + crate::observation::obligations::RecordSecretSyncBehavior + crate::observation::obligations::RecordSessionBehavior + crate::observation::obligations::RecordSpecificationBehavior + crate::observation::obligations::RecordUsageBehavior + crate::observation::obligations::RecordWorkflowRunBehavior + crate::observation::obligations::StartSnapshotBehavior + crate::session::obligations::ClaimLeaseBehavior + crate::session::obligations::DeliverMessageBehavior + crate::session::obligations::RecordRunTickBehavior + crate::decision::obligations::DecisionsQuery + crate::decision::obligations::HandsTodoQuery + crate::decision::obligations::RequestsQuery + crate::direction::obligations::GoalServingsQuery + crate::direction::obligations::GoalsQuery + crate::direction::obligations::RepositoryMarksQuery + crate::dispatch::obligations::ControllersQuery + crate::dispatch::obligations::DispatchesQuery + crate::dispatch::obligations::GuardDecisionsQuery + crate::dispatch::obligations::MessagesQuery + crate::dispatch::obligations::ResourceRequestsQuery + crate::host::obligations::GrantsQuery + crate::host::obligations::ReclaimRunsQuery + crate::improvement::obligations::FindingsQuery + crate::improvement::obligations::KpiReadingsQuery + crate::improvement::obligations::KpisQuery + crate::improvement::obligations::MetricReadingsQuery + crate::improvement::obligations::MetricsQuery + crate::improvement::obligations::SettingsProposalsQuery + crate::observation::obligations::BlockersQuery + crate::observation::obligations::BoardsQuery + crate::observation::obligations::CollectorRunsQuery + crate::observation::obligations::MemberFailuresQuery + crate::observation::obligations::MergedPullRequestsQuery + crate::observation::obligations::PullRequestsQuery + crate::observation::obligations::ReleasesQuery + crate::observation::obligations::RepositoriesQuery + crate::observation::obligations::SecretSyncsQuery + crate::observation::obligations::SessionsQuery + crate::observation::obligations::SnapshotsQuery + crate::observation::obligations::SpecificationsQuery + crate::observation::obligations::UsageReadingsQuery + crate::observation::obligations::WorkflowRunsQuery + crate::session::obligations::DeliveriesQuery + crate::session::obligations::LeasesQuery + crate::session::obligations::RunTicksQuery,
    ConductorInstanceBehaviors: crate::agent::obligations::AttachAgentBehavior + crate::agent::obligations::DeclareAgentKindBehavior + crate::agent::obligations::DeliverToAgentBehavior + crate::agent::obligations::RecordAgentReportBehavior + crate::agent::obligations::RestartAgentBehavior + crate::agent::obligations::StartAgentBehavior + crate::agent::obligations::StopAgentBehavior + crate::agent::obligations::AgentKindsQuery + crate::agent::obligations::AgentsQuery,
    ConductordBehaviors: crate::config::obligations::PutConfigBehavior + crate::instance::obligations::ObserveInstanceBehavior + crate::instance::obligations::PurgeInstanceBehavior + crate::instance::obligations::ReconcileBehavior + crate::instance::obligations::RestartInstanceBehavior + crate::instance::obligations::StartInstanceBehavior + crate::instance::obligations::StopInstanceBehavior + crate::instance::obligations::UpgradeInstanceBehavior + crate::operator::obligations::JoinInstanceBehavior + crate::operator::obligations::LeaveInstanceBehavior + crate::placement::obligations::CloseBoxBehavior + crate::placement::obligations::OpenBoxBehavior + crate::secret::obligations::PutSecretBehavior + crate::secret::obligations::ResolveSecretsBehavior + crate::store::obligations::MigrateStoreBehavior + crate::token::obligations::ContributeTokenBehavior + crate::token::obligations::LeaseTokenBehavior + crate::token::obligations::MarkTokenLimitedBehavior + crate::token::obligations::ReleaseTokenBehavior + crate::token::obligations::WithdrawTokenBehavior + crate::config::obligations::ConfigRevisionsQuery + crate::config::obligations::EffectiveSettingsQuery + crate::instance::obligations::InstancesQuery + crate::instance::obligations::ReconciliationsQuery + crate::operator::obligations::OperatorsQuery + crate::placement::obligations::BoxRefusalsQuery + crate::placement::obligations::BoxesQuery + crate::placement::obligations::PlacementsQuery + crate::secret::obligations::SecretsQuery + crate::store::obligations::MigrationsQuery + crate::store::obligations::StoreStatusQuery + crate::token::obligations::TokensQuery,
{
    match system.conductor_instance.agents() {
        Ok(rows) => {
            let mut body = String::from("{");
            json::member(&mut body, "rows");
            body.push('[');
            for (position, row) in rows.iter().enumerate() {
                if position > 0 {
                    body.push(',');
                }
                wire::encode_view_conductor_agent_agents(row, &mut body);
            }
            body.push(']');
            body.push('}');
            Ok((200, body))
        }
        Err(unmet) => Err(entry::Refused::Unmet(format!("{unmet}"))),
    }
}
