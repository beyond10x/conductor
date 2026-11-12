//! The `conductor` command tree: exactly the `cli:` block of `spec/components.yaml`.
//!
//! Written in clap derive form (a workspace rule) rather than generated, and held to the
//! specification by `tests/cli_tree.rs`, which compares it with `ess specify compile` on every
//! `cargo test`:
//! - each group is a `cli:` group, in the block's order, with its summary as help;
//! - each subcommand is a placed command or view, named by its `naming.wire`, with its
//!   `naming.display` as help, in the order the compiled block gives: commands, then views, each
//!   by qualified name;
//! - each command takes one flag per input field, accepting what the field's type holds: an
//!   integer, `true` or `false`, a member of an enum, an RFC 3339 timestamp, a UUID, or a value
//!   starting with its type's `prefix:`.
//!
//! Three presentation options apply everywhere, and nothing else does: `--input-json -` on every
//! command, `--format` on every view, and the global `--state-dir`, named by `globals.state` of
//! the `ess-cli/1` binding `crates/conductor/cli.yaml`, before or after any subcommand.
//! Beside them the global `--daemon <url>` (`story:daemon-remote`), which the binding declares
//! no slot for: the daemon a command with a route on its global API talks to
//! ([`crate::remote`]); every other command refuses it.
//! `guard record-guard-decision` alone also takes `--from-pre-tool-use`, and `session claim` alone
//! `--from-session-start`: each reads the payload of the Claude Code hook it is wired as. The
//! guard also takes `--pid`, the calling session's process, which its payload does not name
//! (`story:lease-by-process`).
//!
//! The root also takes `--version` (and clap's `-V`), which prints `conductor <version>` from the
//! crate version and exits 0 (`story:version-flag`). Like `--help`, it is clap's own, not a word
//! of the `cli:` block: `ess-cli/1` declares no version global, so the binding names none.
//!
//! No flag is required on its own, because every input field may arrive through
//! `--input-json -` instead: the handler decides what is missing.
//!
//! Beside the `cli:` block, the tree presents each command the binding declares over a `local`
//! callable, with exactly the option flags the binding maps its input fields to:
//! `decision show`, `watch run` (`story:watch-command`),
//! `repository shipped` (`story:release-digest`), `repository activity`
//! (`story:repository-marks`), `repository refresh-hooks` (`story:gates-hooks-fresh`), `resource
//! usage` (`story:token-spend`), `store migrate` (`story:store-on-eventlog-tree`), `records check`
//! (`story:records-from-commands`), `rules show` (`story:rules-show`) and `role show`
//! (`story:roles-in-the-binary`).
//! `host reclaim` (`story:disk-reclaim`) joins the `host` group, and `session start` and `session
//! stop` (`story:sandbox-spec`) the `session` group; `build run` (`story:build-run`)
//! alone also takes the words after `--`, as written: the command it runs.
//!
//! `config migrate` (`story:instance-names-unique`) joins the `config` group, and `message send`
//! and `session deliver` (`story:mailbox-send-and-deliver`) the `message` and `session` groups.
//!
//! `config validate` and `config show` (`story:config-file`) are such commands too, and the only
//! ones that read the global `--config`, named by `globals.config` of the binding, which is taken
//! before or after any subcommand as `--state-dir` is.
//!
//! Nine such commands are one word, with no group: `trust` (`story:portable-trust`) and `init`
//! (`story:first-run-seed`), which read the config file themselves as `config show` does,
//! `doctor` (`story:install-prerequisites`) and `setup` (`story:setup-command`), which read it
//! for what they report and stop on no problem of it, `gc` (`story:conductor-gc`), `spawn`
//! (`story:spawn`), `run` (`story:run-daemon`), the instance's loop, `status`
//! (`story:status-command`), a view over the store, and `upgrade` (`story:conductor-upgrade`),
//! which reads no config file.
//!
//! [`LEAVES`] declares of every leaf whether it writes, and so whether a controller's session may
//! run it ([`Access`]); the guard reads that declaration, not a list of its own
//! (`story:guard-store-migrate`).
//!
//! [`Cli::run`] wires every subcommand to its handler, in the module of the story that fills it.
//! Until that story lands, the handler answers [`NotImplemented`](crate::NotImplemented).

use std::fmt::Write as _;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::Result;
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::{
    build, controller, decide, dispatch, goal, guard, host, message, metric, repository, resource,
    session, shipped, snapshot, status,
};

/// The records conductor and the repository controllers keep.
#[derive(Debug, Parser)]
#[command(name = "conductor", version, disable_help_subcommand = true)]
pub struct Cli {
    /// The state directory; conductor's records are kept in its tree/. Default: the instance's
    /// state when a config file names it, else state/ under the working directory
    #[arg(long, global = true, value_name = "DIR")]
    pub state_dir: Option<PathBuf>,

    /// The config file, named by `globals.config` of the binding. Default: $CONDUCTOR_CONFIG,
    /// else ~/.b10x/conductor/conductor.yaml. Every command reads it for its instance
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// The daemon a command talks to, an https:// address; default: $CONDUCTOR_DAEMON for the
    /// commands with a route on its global API (login, join, leave, config put, instance upgrade),
    /// else this box. Any other command refuses it
    // story:daemon-remote: the `ess-cli/1` binding declares no such global (decision R4).
    #[arg(long, global = true, value_name = "URL")]
    pub daemon: Option<String>,

    /// The group the command belongs to.
    #[command(subcommand)]
    pub group: Group,

    /// The words that name the leaf run, after `conductor`, as [`leaf_of`] reads them from the
    /// parsed arguments; empty when the caller did not give them. No flag: the binary fills it.
    #[arg(skip)]
    pub leaf: Vec<String>,

    /// The leaf's `--actor`, as [`actor_of`] reads it from the parsed arguments: who issues the
    /// command (`conductor.operator.Actor`). `None` when the leaf takes no such flag, the flag
    /// was not given or the caller did not read it; [`crate::actor::chosen`] then decides.
    #[arg(skip)]
    pub actor: Option<String>,
}

/// The `--actor` of the leaf `matches` parsed, when the leaf takes one and it was given.
#[must_use]
pub fn actor_of(matches: &clap::ArgMatches) -> Option<String> {
    let mut at = matches;
    while let Some((_, sub)) = at.subcommand() {
        at = sub;
    }
    at.try_get_one::<String>("actor").ok().flatten().cloned()
}

/// The words that name the leaf `matches` parsed, after `conductor`: `["decision",
/// "raise-decision-request"]`, `["doctor"]`.
#[must_use]
pub fn leaf_of(matches: &clap::ArgMatches) -> Vec<String> {
    let mut words = Vec::new();
    let mut at = matches;
    while let Some((name, sub)) = at.subcommand() {
        words.push(name.to_owned());
        at = sub;
    }
    words
}

/// The [`Access`] [`LEAVES`] declares of the leaf `words` names, if any.
#[must_use]
pub fn access_of(words: &[String]) -> Option<Access> {
    access_of_words(&words.iter().map(String::as_str).collect::<Vec<_>>())
}

/// Whether the leaf `words` names writes the instance's records, or reads them into what it
/// writes (`story:instance-isolation`): every writing leaf of `decision`, `dispatch`, `message`,
/// `resource` and `controller`, whose events are lines of the decision and dispatch logs; `spawn`;
/// `session deliver`, whose delivery is a line of the dispatch log;
/// `snapshot publish-board`; `snapshot start-snapshot` and `complete-snapshot`, whose rollups read
/// the dispatch log; and `build run`, whose shortfall raises an operator item. Such a leaf writes
/// its records to the instance's `records`, so its store is the instance's `state` too
/// ([`crate::state::refuse_other`]).
#[must_use]
pub fn writes_records(words: &[String]) -> bool {
    let words: Vec<&str> = words.iter().map(String::as_str).collect();
    let writes = matches!(
        access_of_words(&words),
        Some(Access::Writes | Access::RunWrites | Access::MailboxWrites)
    );
    writes
        && matches!(
            words.as_slice(),
            [
                "decision" | "dispatch" | "message" | "resource" | "controller",
                _
            ] | ["spawn"]
                | ["session", "deliver"]
                | ["run"]
                | ["build", "run"]
                | [
                    "snapshot",
                    "publish-board" | "start-snapshot" | "complete-snapshot"
                ]
        )
}

/// [`access_of`] over borrowed words.
fn access_of_words(words: &[&str]) -> Option<Access> {
    LEAVES
        .iter()
        .find(|(leaf, _)| *leaf == words)
        .map(|(_, access)| *access)
}

impl Cli {
    /// Runs the parsed command through its handler. A handler that opens the store is given the
    /// `--state-dir` flag, and finds the directory through [`state::dir`](crate::state::dir).
    ///
    /// # Errors
    ///
    /// Whatever the handler answers: [`NotImplemented`](crate::NotImplemented) until the story
    /// that fills it lands.
    pub fn run(self) -> Result<ExitCode> {
        self.run_with(&crate::confine::Host)
    }

    /// [`Cli::run`], with the sessions `spawn`, `session start`, `session stop` and the controller
    /// commands `pause-controller`, `resume-controller` and `stop-controller` start, stop and resume
    /// going through `providers`, and `doctor` reporting them.
    ///
    /// # Errors
    ///
    /// What [`Cli::run`] answers.
    pub fn run_with(self, providers: &dyn crate::confine::Providers) -> Result<ExitCode> {
        // `--daemon` is taken only by a command with a route on the global API
        // (`story:daemon-remote`).
        crate::remote::refuse_unrouted(&self.leaf, self.daemon.as_deref())?;
        let remote = crate::remote::chosen(self.daemon.as_deref());
        // The instance every handler reads (`crate::config::active`). The two config leaves read
        // the file themselves, to report each problem; the guard hook answers a file that does
        // not load by denying every call and naming it (`story:instance-isolation`).
        let mut unloaded = None;
        // `status` with no `--state-dir` reads the federated view when the daemon answers, and
        // only then needs no instance (`story:federated-views`); a file that selects none is
        // reported when the local read runs.
        let mut deferred = None;
        match &self.group {
            Group::Config(_)
            | Group::Trust(_)
            | Group::Init(_)
            | Group::Doctor
            | Group::Setup(_)
            | Group::Upgrade(_)
            | Group::Daemon(_)
            | Group::Instance(_)
            | Group::Join(_)
            | Group::Leave(_)
            | Group::Login
            | Group::Secret(_)
            | Group::Token(_)
            | Group::Tui => {}
            Group::Guard(_) => {
                unloaded = crate::config::set_active(self.config.as_deref())
                    .err()
                    .map(|error| format!("{error:#}"));
            }
            // Attach and its listing read every instance of the file, from any directory, so
            // they select none.
            Group::Session(SessionCommand::Attach(_) | SessionCommand::List(_)) => {}
            // `agent` selects the instance `--instance` names (`story:agent-kinds`); `agent
            // attach`, like `session attach`, also runs where no instance is selected, and reads
            // the kind of `claude-code` then.
            Group::Agent(command) => {
                let selected =
                    crate::config::set_active_instance(self.config.as_deref(), command.instance());
                if !matches!(command, AgentCommand::Attach(_)) {
                    selected?;
                }
            }
            Group::Status(_) if self.state_dir.is_none() => {
                deferred = crate::config::set_active(self.config.as_deref()).err();
            }
            _ => crate::config::set_active(self.config.as_deref())?,
        }
        // Who issues every command this process records (`conductor.operator.Actor`): the leaf's
        // `--actor`, else the run daemon itself, else the calling session, else the operator.
        crate::actor::choose(self.actor.as_deref(), matches!(self.group, Group::Run(_)))?;
        if let Some(dir) = &self.state_dir {
            crate::state::refuse_other(&self.leaf, dir)?;
        }
        let state = self.state_dir.as_deref();
        match self.group {
            Group::Snapshot(command) => match command {
                SnapshotCommand::StartSnapshot(args) => snapshot::start_snapshot(state, args),
                SnapshotCommand::RecordRepository(args) => snapshot::record_repository(state, args),
                SnapshotCommand::RecordPullRequest(args) => {
                    snapshot::record_pull_request(state, args)
                }
                SnapshotCommand::RecordRelease(args) => snapshot::record_release(state, args),
                SnapshotCommand::RecordMergedPullRequest(args) => {
                    snapshot::record_merged_pull_request(state, args)
                }
                SnapshotCommand::RecordWorkflowRun(args) => {
                    snapshot::record_workflow_run(state, args)
                }
                SnapshotCommand::RecordSession(args) => snapshot::record_session(state, args),
                SnapshotCommand::RecordBlocker(args) => snapshot::record_blocker(state, args),
                SnapshotCommand::RecordSpecification(args) => {
                    snapshot::record_specification(state, args)
                }
                SnapshotCommand::RecordUsage(args) => snapshot::record_usage(state, args),
                SnapshotCommand::RecordCollectorRun(args) => {
                    snapshot::record_collector_run(state, args)
                }
                SnapshotCommand::RecordMemberFailure(args) => {
                    snapshot::record_member_failure(state, args)
                }
                SnapshotCommand::RecordSecretSync(args) => {
                    snapshot::record_secret_sync(state, args)
                }
                SnapshotCommand::PublishBoard(args) => status::publish_board(state, args),
                SnapshotCommand::CompleteSnapshot(args) => snapshot::complete_snapshot(state, args),
                SnapshotCommand::FailSnapshot(args) => snapshot::fail_snapshot(state, args),
                SnapshotCommand::Snapshots(view) => snapshot::snapshots(state, view),
                SnapshotCommand::Repositories(view) => snapshot::repositories(state, view),
                SnapshotCommand::PullRequests(view) => snapshot::pull_requests(state, view),
                SnapshotCommand::Releases(view) => snapshot::releases(state, view),
                SnapshotCommand::MergedPullRequests(view) => {
                    snapshot::merged_pull_requests(state, view)
                }
                SnapshotCommand::WorkflowRuns(view) => snapshot::workflow_runs(state, view),
                SnapshotCommand::Sessions(view) => snapshot::sessions(state, view),
                SnapshotCommand::Blockers(view) => snapshot::blockers(state, view),
                SnapshotCommand::Specifications(view) => snapshot::specifications(state, view),
                SnapshotCommand::MemberFailures(view) => snapshot::member_failures(state, view),
                SnapshotCommand::CollectorRuns(view) => snapshot::collector_runs(state, view),
                SnapshotCommand::SecretSyncs(view) => snapshot::secret_syncs(state, view),
                SnapshotCommand::UsageReadings(view) => snapshot::usage_readings(state, view),
                SnapshotCommand::Boards(view) => status::boards(state, view),
            },
            Group::Goal(command) => match command {
                GoalCommand::ProposeGoal(args) => goal::propose_goal(state, args),
                GoalCommand::ConfirmGoal(args) => goal::confirm_goal(state, args),
                GoalCommand::MarkGoalMet(args) => goal::mark_goal_met(state, args),
                GoalCommand::DropGoal(args) => goal::drop_goal(state, args),
                GoalCommand::AddServing(args) => goal::add_serving(state, args),
                GoalCommand::RemoveServing(args) => goal::remove_serving(state, args),
                GoalCommand::Goals(view) => goal::goals(state, view),
                GoalCommand::GoalServings(view) => goal::goal_servings(state, view),
            },
            Group::Repository(command) => match command {
                RepositoryCommand::MarkRepository(args) => repository::mark_repository(state, args),
                RepositoryCommand::ActivateRepository(args) => {
                    repository::activate_repository(state, args)
                }
                RepositoryCommand::DeactivateRepository(args) => {
                    repository::deactivate_repository(state, args)
                }
                RepositoryCommand::RepositoryMarks(view) => {
                    repository::repository_marks(state, view)
                }
                RepositoryCommand::Activity(args) => repository::repository_activity(state, &args),
                RepositoryCommand::Shipped(args) => shipped::shipped(state, &args),
                RepositoryCommand::RefreshHooks(args) => {
                    repository::repository_refresh_hooks(self.config.as_deref(), &args)
                }
            },
            Group::Decision(command) => match command {
                DecisionCommand::RaiseDecisionRequest(args) => {
                    decide::raise_decision_request(state, args)
                }
                DecisionCommand::RecordConductorDecision(args) => {
                    decide::record_conductor_decision(state, args)
                }
                DecisionCommand::RecordOperatorDecision(args) => {
                    decide::record_operator_decision(state, args)
                }
                DecisionCommand::AnswerEscalatedAsOperator(args) => {
                    decide::answer_escalated_as_operator(state, args)
                }
                DecisionCommand::AnswerRequest(args) => decide::answer_request(state, args),
                DecisionCommand::AnswerEscalatedRequest(args) => {
                    decide::answer_escalated_request(state, args)
                }
                DecisionCommand::EscalateRequest(args) => decide::escalate_request(state, args),
                DecisionCommand::AmendEscalatedRequest(args) => {
                    decide::amend_escalated_request(state, args)
                }
                DecisionCommand::WithdrawRequest(args) => decide::withdraw_request(state, args),
                DecisionCommand::ReverseConductorDecision(args) => {
                    decide::reverse_conductor_decision(state, args)
                }
                DecisionCommand::ReverseOperatorDecision(args) => {
                    decide::reverse_operator_decision(state, args)
                }
                DecisionCommand::Requests(view) => decide::requests(state, view),
                DecisionCommand::HandsTodo(view) => decide::hands_todo(state, view),
                DecisionCommand::Decisions(view) => decide::decisions(state, view),
                DecisionCommand::Show(args) => decide::show(state, args),
                DecisionCommand::Handle(args) => match args.request {
                    Some(request) => decide::handle(state, &request, args.decider.as_deref()),
                    None => anyhow::bail!("decision handle: --request is missing"),
                },
            },
            Group::Controller(command) => match command {
                ControllerCommand::StartController(args) => {
                    controller::start_controller(state, args)
                }
                ControllerCommand::PauseController(args) => {
                    controller::pause_controller(state, args, providers)
                }
                ControllerCommand::ResumeController(args) => {
                    controller::resume_controller(self.config.as_deref(), state, args, providers)
                }
                ControllerCommand::StopController(args) => {
                    controller::stop_controller(state, args, providers)
                }
                ControllerCommand::ReviseCharter(args) => controller::revise_charter(state, args),
                ControllerCommand::AllControllers(view) => controller::all_controllers(state, view),
                ControllerCommand::Controllers(args) => controller::controllers(state, &args),
            },
            Group::Dispatch(command) => match command {
                DispatchCommand::SendDispatch(args) => dispatch::send_dispatch(state, args),
                DispatchCommand::ReportStarted(args) => dispatch::report_started(state, args),
                DispatchCommand::ReportBlocked(args) => dispatch::report_blocked(state, args),
                DispatchCommand::ReportUnblocked(args) => dispatch::report_unblocked(state, args),
                DispatchCommand::ReportDone(args) => dispatch::report_done(state, args),
                DispatchCommand::ReportFailed(args) => dispatch::report_failed(state, args),
                DispatchCommand::CancelDispatch(args) => dispatch::cancel_dispatch(state, args),
                DispatchCommand::RecordDispatchLink(args) => dispatch::record_link(state, args),
                DispatchCommand::Dispatches(view) => dispatch::dispatches(state, view),
            },
            Group::Message(command) => match command {
                MessageCommand::ReceiveMessage(args) => message::receive_message(state, args),
                MessageCommand::HandleMessage(args) => message::handle_message(state, args),
                MessageCommand::RejectMessage(args) => message::reject_message(state, args),
                MessageCommand::RouteNeed(args) => message::route_need(state, args),
                MessageCommand::Messages(view) => message::messages(state, view),
                MessageCommand::Send(args) => message::send(state, &args),
            },
            Group::Resource(command) => match command {
                ResourceCommand::RequestResource(args) => resource::request_resource(state, args),
                ResourceCommand::GrantResource(args) => resource::grant_resource(state, args),
                ResourceCommand::RefuseResource(args) => resource::refuse_resource(state, args),
                ResourceCommand::ReleaseResource(args) => resource::release_resource(state, args),
                ResourceCommand::ResourceRequests(view) => resource::resource_requests(state, view),
                ResourceCommand::Usage(args) => crate::usage::resource_usage(&args),
            },
            Group::Guard(command) => match command {
                GuardCommand::RecordGuardDecision(args) => {
                    guard::record_guard_decision(state, *args, unloaded.as_deref())
                }
                GuardCommand::GuardDecisions(view) => guard::guard_decisions(state, view),
            },
            Group::Session(command) => match command {
                SessionCommand::Claim(args) => session::claim(state, args),
                // `session deliver` types the text and records its own delivery
                // (story:mailbox-send-and-deliver); the placed deliver-message leaf, a record
                // with nothing typed, is not written.
                SessionCommand::DeliverMessage(_) => {
                    crate::not_implemented("session deliver-message")
                }
                SessionCommand::RecordRunTick(args) => crate::run::record_run_tick(state, &args),
                SessionCommand::Deliveries(view) => session::deliveries(state, view),
                SessionCommand::Leases(view) => session::leases(state, view),
                SessionCommand::RunTicks(view) => crate::run::run_ticks(state, view),
                SessionCommand::Deliver(args) => session::deliver(state, &args),
                // Aliases of `agent start` and `agent stop` for this release
                // (`story:agent-kinds`): through the role's kind, `claude-code` by default.
                SessionCommand::Start(args) => crate::agent::cli::start(
                    &crate::agent::Context {
                        config: self.config.as_deref(),
                        state,
                        providers,
                    },
                    &AgentSlotArgs {
                        instance: None,
                        role: args.role,
                    },
                    "session start",
                ),
                SessionCommand::Stop(args) => crate::agent::cli::stop(
                    &crate::agent::Context {
                        config: self.config.as_deref(),
                        state,
                        providers,
                    },
                    &AgentSlotArgs {
                        instance: None,
                        role: args.role,
                    },
                    "session stop",
                ),
                SessionCommand::Attach(args) => {
                    crate::attach::attach(self.config.as_deref(), state, &args)
                }
                SessionCommand::List(args) => {
                    crate::attach::list(self.config.as_deref(), state, &args)
                }
            },
            Group::Host(command) => match command {
                // `host reclaim` records its own runs (`crate::gc::reclaim`); the placed
                // record-reclaim and reclaim-runs leaves are not written yet.
                HostCommand::RecordReclaim(_) => crate::not_implemented("host record-reclaim"),
                HostCommand::RecordPeak(args) => host::record_peak(state, args),
                HostCommand::ReleaseGrant(args) => host::release_grant(state, args),
                HostCommand::RequestDisk(args) => host::request_disk(state, args),
                HostCommand::ReserveDisk(args) => host::reserve_disk(state, args),
                HostCommand::StartRun(args) => host::start_run_command(state, args),
                HostCommand::TakeHostGrant(args) => host::take_host_grant(state, args),
                HostCommand::WithdrawDiskRequest(args) => host::withdraw_disk_request(state, args),
                HostCommand::Grants(view) => host::grants(state, view),
                HostCommand::ReclaimRuns(_) => crate::not_implemented("host reclaim-runs"),
                HostCommand::Reclaim(args) => {
                    crate::gc::reclaim::run_reclaim(state, args.format.as_deref())
                }
            },
            // The metrics and their readings (story:kpi-records).
            Group::Metric(command) => match command {
                MetricCommand::DefineMetric(args) => metric::define_metric(state, args),
                MetricCommand::RecordMetricReading(args) => {
                    metric::record_metric_reading(state, args)
                }
                MetricCommand::SetMetricTarget(args) => metric::set_metric_target(state, args),
                MetricCommand::SupersedeMetricReading(args) => {
                    metric::supersede_metric_reading(state, args)
                }
                MetricCommand::JudgeMetricReading(args) => {
                    metric::judge_metric_reading(state, args)
                }
                MetricCommand::KpiReadings(view) => metric::kpi_readings(state, view),
                MetricCommand::Kpis(view) => metric::kpis(state, view),
                MetricCommand::MetricReadings(view) => metric::metric_readings(state, view),
                MetricCommand::Metrics(view) => metric::metrics(state, view),
                MetricCommand::Compute(args) => metric::compute(state, &args),
            },
            // The friction findings' leaves are not written yet (story:kpi-spec placed them).
            Group::Finding(command) => match command {
                FindingCommand::FileFinding(_) => crate::not_implemented("finding file-finding"),
                FindingCommand::RaiseFinding(_) => crate::not_implemented("finding raise-finding"),
                FindingCommand::VerifyFinding(_) => {
                    crate::not_implemented("finding verify-finding")
                }
                FindingCommand::Findings(_) => crate::not_implemented("finding findings"),
            },
            // The tuner's proposals (story:tuner).
            Group::Tuner(command) => match command {
                TunerCommand::SettingsProposals(view) => {
                    crate::tune::settings_proposals(state, view)
                }
            },
            Group::Broker(command) => match command {
                BrokerCommand::Serve => crate::broker::broker_serve(state),
            },
            Group::Build(command) => match command {
                BuildCommand::Run(args) => build::build_run(state, &args),
            },
            Group::Config(command) => match command {
                ConfigCommand::Migrate(args) => {
                    crate::config::migrate(self.config.as_deref(), &args)
                }
                ConfigCommand::Put(args) => match &remote {
                    Some(daemon) => crate::remote::config_put(daemon, &args),
                    None => crate::config::put(self.config.as_deref(), &args),
                },
                ConfigCommand::Show(args) => crate::config::show(self.config.as_deref(), &args),
                ConfigCommand::Validate(args) => {
                    crate::config::validate(self.config.as_deref(), &args)
                }
                ConfigCommand::Workspace(args) => {
                    crate::config::workspace(self.config.as_deref(), &args)
                }
            },
            Group::Doctor => crate::doctor::doctor_with(self.config.as_deref()),
            Group::Gc(args) => crate::gc::gc(&args),
            Group::Init(args) => crate::init::init(self.config.as_deref(), &args),
            Group::Need(command) => match command {
                NeedCommand::New(args) => crate::need::need_new(&args),
            },
            Group::Records(command) => match command {
                RecordsCommand::Check(args) => crate::records::records_check(state, &args),
            },
            Group::Role(command) => match command {
                RoleCommand::Show(args) => crate::role::role_show(&args),
            },
            Group::Rules(command) => match command {
                RulesCommand::Show(args) => crate::rules::rules_show(&args),
            },
            Group::Run(args) => crate::run::run(self.config.as_deref(), state, &args, providers),
            Group::Setup(args) => crate::setup::setup(self.config.as_deref(), &args),
            Group::Spawn(args) => {
                crate::spawn::spawn(self.config.as_deref(), state, &args, providers)
            }
            Group::Status(args) => {
                status::status_or_federated(self.config.as_deref(), state, &args, deferred)
            }
            Group::Trust(args) => crate::trust::trust(self.config.as_deref(), &args),
            Group::Upgrade(args) => crate::upgrade::upgrade(&args),
            Group::Store(command) => match command {
                StoreCommand::Migrate(args) => crate::store::migrate::store_migrate(state, &args),
            },
            Group::Watch(command) => match command {
                WatchCommand::Run(args) => crate::watch::watch_run(state, &args),
            },
            // The agent verbs, each through its role's kind (`story:agent-kinds`).
            Group::Agent(command) => {
                let context = crate::agent::Context {
                    config: self.config.as_deref(),
                    state,
                    providers,
                };
                match command {
                    AgentCommand::Attach(args) => crate::agent::cli::attach(&context, &args),
                    AgentCommand::Deliver(args) => crate::agent::cli::deliver(state, &args),
                    AgentCommand::Start(args) => {
                        crate::agent::cli::start(&context, &args, "agent start")
                    }
                    AgentCommand::Stop(args) => {
                        crate::agent::cli::stop(&context, &args, "agent stop")
                    }
                }
            }
            Group::Daemon(command) => match command {
                DaemonCommand::Serve(args) => {
                    crate::daemon::daemon_serve(self.config.as_deref(), state, &args)
                }
                DaemonCommand::StartRole(args) => {
                    crate::daemon::start_role(self.config.as_deref(), state, &args)
                }
            },
            Group::Instance(command) => match command {
                InstanceCommand::Serve => {
                    crate::instance::instance_serve(self.config.as_deref(), state, providers)
                }
                InstanceCommand::Relay => crate::instance::instance_relay(self.config.as_deref()),
                InstanceCommand::Upgrade(args) => match &remote {
                    Some(daemon) => crate::remote::instance_upgrade(daemon, &args),
                    None => crate::daemon::upgrade::instance_upgrade(self.config.as_deref(), &args),
                },
            },
            Group::Join(args) => crate::token::join(&args, remote.as_deref()),
            Group::Leave(args) => crate::token::leave(&args, remote.as_deref()),
            Group::Login => crate::login::login(remote.as_deref()),
            Group::Secret(command) => match command {
                SecretCommand::Put(args) => {
                    crate::secret::secret_put(self.config.as_deref(), &args)
                }
            },
            Group::Token(command) => match command {
                TokenCommand::Reveal(args) => {
                    crate::token::token_reveal(self.config.as_deref(), &args)
                }
            },
            Group::Tui => crate::tui::tui(self.config.as_deref(), self.daemon.as_deref()),
        }
    }
}

/// The groups of the `cli:` block, in its order.
#[derive(Debug, Subcommand)]
pub enum Group {
    /// One sense run and what it observed
    #[command(subcommand)]
    Snapshot(SnapshotCommand),

    /// The goals dispatches serve
    #[command(subcommand)]
    Goal(GoalCommand),

    /// Which repositories have their issues and pull requests processed
    #[command(subcommand)]
    Repository(RepositoryCommand),

    /// Decision requests and the decisions that answer them
    #[command(subcommand)]
    Decision(DecisionCommand),

    /// One controller session per repository
    #[command(subcommand)]
    Controller(ControllerCommand),

    /// Briefs to controllers and their reports
    #[command(subcommand)]
    Dispatch(DispatchCommand),

    /// Every message between conductor and a controller
    #[command(subcommand)]
    Message(MessageCommand),

    /// Build slots, disk and usage asked for before they are spent
    #[command(subcommand)]
    Resource(ResourceCommand),

    /// The controller hook and the record of its verdicts
    #[command(subcommand)]
    Guard(GuardCommand),

    /// The instance's conductor lease, what the run daemon delivers to its sessions, and the daemon's ticks
    #[command(subcommand)]
    Session(SessionCommand),

    /// The host ledger of slots and disk reservations every instance shares, and the reclaim runs that free disk
    #[command(subcommand)]
    Host(HostCommand),

    /// The measured quantities of the self-improvement loop, their daily readings and the targets that make some of them KPIs
    #[command(subcommand)]
    Metric(MetricCommand),

    /// Friction the loop detected, the draft story each is filed as, and the week-later check of the metric it should move
    #[command(subcommand)]
    Finding(FindingCommand),

    /// The settings changes the tuner proposed after each daily tick, and whether each was applied, kept, reverted or refused
    #[command(subcommand)]
    Tuner(TunerCommand),

    // No help, as for `dashboard`: the binding declares the group only through `agent start`,
    // `deliver`, `attach` and `stop` (`story:spec-v1`).
    #[command(subcommand)]
    Agent(AgentCommand),

    // No help, as for `dashboard`: the binding declares the group only through `broker serve`
    // (`story:tool-broker-read`).
    #[command(subcommand)]
    Broker(BrokerCommand),

    // No help, as for `dashboard`: the binding declares the group only through `build run`
    // (`story:build-run`).
    #[command(subcommand)]
    Build(BuildCommand),

    // No help, as for `dashboard`: the binding declares the group only through `config validate`
    // and `config show` (`story:config-file`).
    #[command(subcommand)]
    Config(ConfigCommand),

    // No help, as for `dashboard`: the binding declares the group only through `daemon serve`
    // (`story:spec-v1`).
    #[command(subcommand)]
    Daemon(DaemonCommand),

    /// Report the machine, each program conductor starts as present, with its version, or missing, each one older than its pin in the nearest b10x.toml, or else in the pins conductor was built with, and the account each forge CLI is logged in as on each host of the config file's sources
    #[command(name = "doctor")]
    Doctor,

    /// Remove what the dashboard's disk panel marks reclaimable, and no tree a live session works in; print the bytes freed
    #[command(name = "gc")]
    Gc(GcArgs),

    /// Write and commit the instance's records directory skeleton; refuse to overwrite any file
    #[command(name = "init")]
    Init(InitArgs),

    // No help, as for `dashboard`: the binding declares the group only through `instance serve`
    // (`story:spec-v1`).
    #[command(subcommand)]
    Instance(InstanceCommand),

    /// Join an instance of the daemon --daemon names as one of its operators (JoinInstance, with the grants daemon.operators gives you) and contribute your Claude token, read from standard input, to the instance's pool (ContributeToken): the value goes into the daemon's secrets source, the pool records its owner, seat (--seat, your operator name by default) and state, never the value
    #[command(name = "join")]
    Join(JoinArgs),

    /// Leave an instance of the daemon --daemon names (LeaveInstance) and withdraw every token you contributed to its pool (WithdrawToken); each agent on one is resumed on another token by the instance's loop
    #[command(name = "leave")]
    Leave(LeaveArgs),

    /// Log in to the daemon --daemon or CONDUCTOR_DAEMON names through its Identity service, in a browser, and keep the Identity session in this client's keychain; each command against that daemon then presents a bearer minted from it
    #[command(name = "login")]
    Login,

    // No help, as for `dashboard`: the binding declares the group only through `need new`
    // (`story:need-id-unique`).
    #[command(subcommand)]
    Need(NeedCommand),

    // No help, as for `dashboard`: the binding declares the group only through `records check`
    // (`story:records-from-commands`).
    #[command(subcommand)]
    Records(RecordsCommand),

    // No help, as for `dashboard`: the binding declares the group only through `role show`
    // (`story:roles-in-the-binary`).
    #[command(subcommand)]
    Role(RoleCommand),

    // No help, as for `dashboard`: the binding declares the group only through `rules show`
    // (`story:rules-show`).
    #[command(subcommand)]
    Rules(RulesCommand),

    /// Run the instance's loop with no model call, recording each tick - every cadence.watch the watch pass, its handlers and the mailbox; every cadence.cycle the snapshot, the board, records check and dispatch v1; at cadence.daily metric compute, REPORT.md and a commit of the records through the bot route. --once runs one watch tick and one cycle tick and exits; --once --commit-records commits the records alone
    #[command(name = "run")]
    Run(RunArgs),

    // No help, as for `dashboard`: the binding declares the group only through `secret put`
    // (`story:spec-v1`).
    #[command(subcommand)]
    Secret(SecretCommand),

    /// Check a repository's prerequisites, doctor's and its planning store, instance, records and trust; --apply writes the instance config, runs init and trust, activates the worktree workspace, and prints what only a person may run
    #[command(name = "setup")]
    Setup(SetupArgs),

    /// Start a repository's controller session in its checkout, with the controller role's profile, model and settings and its charter, and record it running; refuse while a session of that name runs
    #[command(name = "spawn")]
    Spawn(SpawnArgs),

    /// Print what asks for attention, one line each, from the store alone (the newest complete snapshot's id and age, its deviations, blocked dispatches, waits, pending requests and usage); --full prints its sessions and every open dispatch besides; start no snapshot and create nothing
    #[command(name = "status")]
    Status(StatusArgs),

    // No help, as for `dashboard`: the binding declares the group only through `store migrate`.
    #[command(subcommand)]
    Store(StoreCommand),

    // No help, as for `dashboard`: the binding declares the group only through `token reveal`
    // (`story:token-pool`).
    #[command(subcommand)]
    Token(TokenCommand),

    /// Mark every git checkout under the instance's checkouts root, and its records directory, as trusted Claude Code workspaces in ~/.claude.json
    #[command(name = "trust")]
    Trust(TrustArgs),

    /// Open the terminal UI over the daemon's global API - the instances (state, open decisions, KPI flags), the sessions of the selected one (role, state, context, current work) and its settings, read-only; Enter attaches to the selected session, w watches it, l follows its terminal log, ctrl-] then d returns; the global --daemon names the daemon (default CONDUCTOR_DAEMON, else 127.0.0.1:daemon.port)
    #[command(name = "tui")]
    Tui,

    /// Check for a newer release of the repository conductor was built from; without --check, download its archive for this target and SHA256SUMS, verify the sum, run the new binary's --version and replace this binary, keeping conductor.previous; --release installs a named release
    #[command(name = "upgrade")]
    Upgrade(UpgradeArgs),

    // No help, as for `dashboard`: the binding declares the group only through `watch run`.
    #[command(subcommand)]
    Watch(WatchCommand),
}

/// What a leaf of the tree does with what it touches, declared per leaf in [`LEAVES`] and never
/// read from its arguments: whether it writes, and so whether a controller's session may run it.
/// The guard denies a controller every leaf it may not run (`guard::rules`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Writes nothing; every session runs it.
    Reads,
    /// Writes nothing, but only conductor's session runs it: it is not among the reads a
    /// controller is given (`repository shipped`, `resource usage`).
    ConductorReads,
    /// Writes: the store under the state directory, conductor's records, the instance's files or
    /// `~/.claude.json`, removes what the disk panel marks reclaimable (`gc`), or starts a
    /// controller's session (`spawn`). Only conductor's session runs it.
    Writes,
    /// Writes the guard's record of its own verdict: every session runs it, as the guard hook
    /// (`guard record-guard-decision`).
    HookWrites,
    /// Writes the host ledger for the session's own run: every session runs it, a controller's
    /// builds included (`build run`, `story:build-run`).
    RunWrites,
    /// Writes the session's own message: its file in the mailbox under the state directory, its
    /// record in the store and its line in the records' dispatch log. Every session runs it, a
    /// controller's reports included (`message send`, `story:mailbox-send-and-deliver`).
    MailboxWrites,
}

impl Access {
    /// Whether the leaf writes.
    #[must_use]
    pub const fn writes(self) -> bool {
        matches!(
            self,
            Self::Writes | Self::HookWrites | Self::RunWrites | Self::MailboxWrites
        )
    }

    /// Whether a controller's session may run the leaf.
    #[must_use]
    pub const fn controller_runs(self) -> bool {
        matches!(
            self,
            Self::Reads | Self::HookWrites | Self::RunWrites | Self::MailboxWrites
        )
    }
}

/// Every leaf of the tree, by the words that name it after `conductor`, with its [`Access`]: a
/// leaf of a group is two words, a leaf with no group (`doctor`, `init`, `trust`) one.
/// `tests/guard_rules.rs` walks the tree and fails on a leaf missing here or named here and not in
/// the tree, so a leaf added to the tree is declared before it builds green.
pub const LEAVES: &[(&[&str], Access)] = &[
    (&["snapshot", "complete-snapshot"], Access::Writes),
    (&["snapshot", "fail-snapshot"], Access::Writes),
    (&["snapshot", "publish-board"], Access::Writes),
    (&["snapshot", "record-blocker"], Access::Writes),
    (&["snapshot", "record-collector-run"], Access::Writes),
    (&["snapshot", "record-member-failure"], Access::Writes),
    (&["snapshot", "record-merged-pull-request"], Access::Writes),
    (&["snapshot", "record-pull-request"], Access::Writes),
    (&["snapshot", "record-release"], Access::Writes),
    (&["snapshot", "record-repository"], Access::Writes),
    (&["snapshot", "record-secret-sync"], Access::Writes),
    (&["snapshot", "record-session"], Access::Writes),
    (&["snapshot", "record-specification"], Access::Writes),
    (&["snapshot", "record-usage"], Access::Writes),
    (&["snapshot", "record-workflow-run"], Access::Writes),
    (&["snapshot", "start-snapshot"], Access::Writes),
    (&["snapshot", "blockers"], Access::Reads),
    (&["snapshot", "boards"], Access::Reads),
    (&["snapshot", "collector-runs"], Access::Reads),
    (&["snapshot", "member-failures"], Access::Reads),
    (&["snapshot", "merged-pull-requests"], Access::Reads),
    (&["snapshot", "pull-requests"], Access::Reads),
    (&["snapshot", "releases"], Access::Reads),
    (&["snapshot", "repositories"], Access::Reads),
    (&["snapshot", "secret-syncs"], Access::Reads),
    (&["snapshot", "sessions"], Access::Reads),
    (&["snapshot", "snapshots"], Access::Reads),
    (&["snapshot", "specifications"], Access::Reads),
    (&["snapshot", "usage-readings"], Access::Reads),
    (&["snapshot", "workflow-runs"], Access::Reads),
    (&["goal", "add-serving"], Access::Writes),
    (&["goal", "confirm-goal"], Access::Writes),
    (&["goal", "drop-goal"], Access::Writes),
    (&["goal", "mark-goal-met"], Access::Writes),
    (&["goal", "propose-goal"], Access::Writes),
    (&["goal", "remove-serving"], Access::Writes),
    (&["goal", "servings"], Access::Reads),
    (&["goal", "goals"], Access::Reads),
    (&["repository", "activate-repository"], Access::Writes),
    (&["repository", "deactivate-repository"], Access::Writes),
    (&["repository", "mark-repository"], Access::Writes),
    (&["repository", "repository-marks"], Access::Reads),
    (&["repository", "activity"], Access::Reads),
    (&["repository", "shipped"], Access::ConductorReads),
    // Installs the Gates hooks of the instance's checkouts with --apply (story:gates-hooks-fresh).
    (&["repository", "refresh-hooks"], Access::Writes),
    (&["decision", "answer-escalated-request"], Access::Writes),
    (
        &["decision", "answer-escalated-as-operator"],
        Access::Writes,
    ),
    (&["decision", "answer-request"], Access::Writes),
    (&["decision", "escalate-request"], Access::Writes),
    (&["decision", "amend-escalated-request"], Access::Writes),
    (&["decision", "raise-decision-request"], Access::Writes),
    (&["decision", "record-conductor-decision"], Access::Writes),
    (&["decision", "record-operator-decision"], Access::Writes),
    (&["decision", "reverse-conductor-decision"], Access::Writes),
    (&["decision", "reverse-operator-decision"], Access::Writes),
    (&["decision", "withdraw-request"], Access::Writes),
    (&["decision", "decisions"], Access::Reads),
    (&["decision", "hands-todo"], Access::Reads),
    (&["decision", "requests"], Access::Reads),
    (&["decision", "show"], Access::Reads),
    // Raises, records or escalates the request, as the run daemon does (story:decide-model-adapter).
    (&["decision", "handle"], Access::Writes),
    (&["controller", "pause-controller"], Access::Writes),
    (&["controller", "resume-controller"], Access::Writes),
    (&["controller", "revise-charter"], Access::Writes),
    (&["controller", "start-controller"], Access::Writes),
    (&["controller", "stop-controller"], Access::Writes),
    (&["controller", "all-controllers"], Access::Reads),
    (&["controller", "controllers"], Access::Reads),
    (&["dispatch", "cancel-dispatch"], Access::Writes),
    (&["dispatch", "record-link"], Access::Writes),
    (&["dispatch", "report-blocked"], Access::Writes),
    (&["dispatch", "report-done"], Access::Writes),
    (&["dispatch", "report-failed"], Access::Writes),
    (&["dispatch", "report-started"], Access::Writes),
    (&["dispatch", "report-unblocked"], Access::Writes),
    (&["dispatch", "send-dispatch"], Access::Writes),
    (&["dispatch", "dispatches"], Access::Reads),
    (&["message", "handle-message"], Access::Writes),
    (&["message", "receive-message"], Access::Writes),
    (&["message", "reject-message"], Access::Writes),
    (&["message", "route-need"], Access::Writes),
    (&["message", "messages"], Access::Reads),
    // A controller reports through the mailbox (story:mailbox-send-and-deliver).
    (&["message", "send"], Access::MailboxWrites),
    (&["resource", "grant-resource"], Access::Writes),
    (&["resource", "refuse-resource"], Access::Writes),
    (&["resource", "release-resource"], Access::Writes),
    (&["resource", "request-resource"], Access::Writes),
    (&["resource", "resource-requests"], Access::Reads),
    (&["resource", "usage"], Access::ConductorReads),
    (&["guard", "record-guard-decision"], Access::HookWrites),
    (&["guard", "guard-decisions"], Access::Reads),
    // Conductor's own SessionStart hook runs it; a controller that claimed the lease would be
    // taken for the instance's conductor.
    (&["session", "claim"], Access::Writes),
    // Types into another session and records the delivery; the run daemon delivers, a
    // controller reports with `message send` (story:mailbox-send-and-deliver,
    // docs/design/conductor-run.md § 3).
    (&["session", "deliver"], Access::Writes),
    (&["session", "deliver-message"], Access::Writes),
    // The run daemon records its ticks (story:run-daemon).
    (&["session", "record-run-tick"], Access::Writes),
    (&["session", "deliveries"], Access::Reads),
    (&["session", "leases"], Access::Reads),
    (&["session", "run-ticks"], Access::Reads),
    // Starts or stops the session of the role director or maintainer (conductor or conductor-dev, their old names, for this release) (story:sandbox-spec).
    (&["session", "start"], Access::Writes),
    (&["session", "stop"], Access::Writes),
    // Types into a live session's terminal; no session attaches to another
    // (story:session-attach-command, story:session-attach-refuses-sessions).
    (&["session", "attach"], Access::Writes),
    (&["session", "list"], Access::Reads),
    (&["host", "record-reclaim"], Access::Writes),
    (&["host", "record-peak"], Access::Writes),
    (&["host", "release-grant"], Access::Writes),
    (&["host", "request-disk"], Access::Writes),
    (&["host", "reserve-disk"], Access::Writes),
    (&["host", "start-run"], Access::Writes),
    (&["host", "take-host-grant"], Access::Writes),
    (&["host", "withdraw-disk-request"], Access::Writes),
    (&["host", "grants"], Access::Reads),
    (&["host", "reclaim-runs"], Access::Reads),
    // Removes what needs no decision until free disk is back over the floor (story:disk-reclaim).
    (&["host", "reclaim"], Access::Writes),
    (&["metric", "define-metric"], Access::Writes),
    (&["metric", "judge-metric-reading"], Access::Writes),
    (&["metric", "record-metric-reading"], Access::Writes),
    (&["metric", "set-metric-target"], Access::Writes),
    (&["metric", "supersede-metric-reading"], Access::Writes),
    (&["metric", "kpi-readings"], Access::Reads),
    (&["metric", "kpis"], Access::Reads),
    (&["metric", "metric-readings"], Access::Reads),
    (&["metric", "metrics"], Access::Reads),
    // Reads the records' logs and the snapshot, and records the day's readings
    // (story:kpi-records).
    (&["metric", "compute"], Access::Writes),
    (&["finding", "file-finding"], Access::Writes),
    (&["finding", "raise-finding"], Access::Writes),
    (&["finding", "verify-finding"], Access::Writes),
    (&["finding", "findings"], Access::Reads),
    // Serves the instance's broker and records every call in the store (story:tool-broker-read).
    (&["broker", "serve"], Access::Writes),
    // Reserves disk in the host ledger and runs the session's own build under the disk watchdog
    // (story:build-run); the guard sends every build through it.
    (&["build", "run"], Access::RunWrites),
    // Writes a session_prefix into the config file (story:instance-names-unique).
    (&["config", "migrate"], Access::Writes),
    // Writes the config file, commits it and records the revision (story:config-globals).
    (&["config", "put"], Access::Writes),
    (&["config", "show"], Access::Reads),
    // Every session checks its config; it writes nothing (story:daemon-reconcile).
    (&["config", "validate"], Access::Reads),
    (&["config", "workspace"], Access::Writes),
    (&["doctor"], Access::Reads),
    // Removes the build caches, trees and targets the disk panel marks reclaimable.
    (&["gc"], Access::Writes),
    (&["init"], Access::Writes),
    // Keeps the id it prints in the state directory (story:need-id-unique).
    (&["need", "new"], Access::Writes),
    // Reads conductor's records logs and the store's decisions (story:records-from-commands).
    (&["records", "check"], Access::ConductorReads),
    // Prints a profile the binary carries; reads nothing else (story:roles-in-the-binary).
    (&["role", "show"], Access::Reads),
    // Reads the instance's rules.md and nothing else; a controller cites its items in a worker's
    // brief (story:rules-show).
    (&["rules", "show"], Access::Reads),
    // The instance's loop: records ticks, writes the records' logs and charters, starts and
    // stops controllers, and with --install or --uninstall starts or stops its user unit
    // (story:run-daemon).
    (&["run"], Access::Writes),
    // With --apply, writes the config file, the records and ~/.claude.json (story:setup-command).
    (&["setup"], Access::Writes),
    // Starts a controller's session and records the controller in the store.
    (&["spawn"], Access::Writes),
    // A view over the store: starts no snapshot and creates nothing (story:status-command).
    (&["status"], Access::Reads),
    // Copies the store into the tree layout under the state directory.
    (&["store", "migrate"], Access::Writes),
    (&["trust"], Access::Writes),
    // Replaces the running binary's file with a release's (story:conductor-upgrade).
    (&["upgrade"], Access::Writes),
    (&["watch", "run"], Access::Writes),
    // The v1 commands, placed before their stories land; each exits 2 (`story:spec-v1`). Every
    // one that would change an instance, a box, an agent, an operator or a secret is a write
    // only conductor's session runs; the two that only show (an attach, the terminal UI) are
    // conductor's reads.
    (&["agent", "attach"], Access::ConductorReads),
    (&["agent", "deliver"], Access::Writes),
    (&["agent", "start"], Access::Writes),
    (&["agent", "stop"], Access::Writes),
    (&["daemon", "serve"], Access::Writes),
    (&["daemon", "start-role"], Access::Writes),
    (&["instance", "serve"], Access::Writes),
    (&["instance", "relay"], Access::Writes),
    // Asks the daemon to upgrade an instance's runtime (story:instance-upgrade).
    (&["instance", "upgrade"], Access::Writes),
    (&["join"], Access::Writes),
    (&["leave"], Access::Writes),
    (&["login"], Access::Writes),
    (&["secret", "put"], Access::Writes),
    // Prints a leased token's value for the harness's apiKeyHelper; no session runs it through a tool
    // call but conductor's (`story:token-pool`).
    (&["token", "reveal"], Access::Writes),
    (&["tui"], Access::ConductorReads),
    // A view of the tuner's proposals over the store (story:tuner).
    (&["tuner", "settings-proposals"], Access::Reads),
];

// `conductor agent`: the agents of an instance, each through its role's kind (design
// architecture-next § 7, `story:agent-kinds`, `crate::agent::cli`).
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    /// Attach this terminal to the agent of a role slot through its kind: for claude-code, conductor session attach --role; --watch attaches read-only; refuse a kind this binary does not implement and a kind that is not interactive (CapabilityMissing)
    #[command(name = "attach")]
    Attach(AgentAttachArgs),
    /// Deliver a text to the running agent of a role slot through its kind: for claude-code, conductor session deliver to the role's session name; --text is a file, or - for standard input; refuse a kind this binary does not implement and a kind that is not interactive (CapabilityMissing)
    #[command(name = "deliver")]
    Deliver(AgentDeliverArgs),
    /// Start the agent of a role slot through its kind (roles[].kind, claude-code by default): for claude-code, conductor session start, which stays its alias for this release; refuse a kind this binary does not implement (UnknownAgentKind), and what session start refuses
    #[command(name = "start")]
    Start(AgentSlotArgs),
    /// Stop the running agent of a role slot through its kind: for claude-code, conductor session stop, which stays its alias for this release; refuse a kind this binary does not implement, and what session stop refuses
    #[command(name = "stop")]
    Stop(AgentSlotArgs),
}

impl AgentCommand {
    /// The instance `--instance` names, if any.
    #[must_use]
    pub fn instance(&self) -> Option<&str> {
        match self {
            Self::Attach(args) => args.instance.as_deref(),
            Self::Deliver(args) => args.instance.as_deref(),
            Self::Start(args) | Self::Stop(args) => args.instance.as_deref(),
        }
    }
}

/// The input of the binding's `agent-start` and `agent-stop` callables,
/// `conductor.agent.AgentSlotSelection`.
#[derive(Debug, Clone, Default, Args)]
pub struct AgentSlotArgs {
    /// The instance of the config file. Default: the active one
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// The role whose slot starts or stops, as for `session start`
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
}

/// The input of the binding's `agent-deliver` callable, `conductor.agent.AgentTextDelivery`.
#[derive(Debug, Clone, Default, Args)]
pub struct AgentDeliverArgs {
    /// The instance of the config file. Default: the active one
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// The role whose agent the text is delivered to
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
    /// The file holding the text, or - for standard input
    #[arg(long, value_name = "FILE")]
    pub text: Option<String>,
}

/// The input of the binding's `agent-attach` callable, `conductor.agent.AgentTerminalAttach`.
#[derive(Debug, Clone, Default, Args)]
pub struct AgentAttachArgs {
    /// The instance of the config file. Default: every instance, as `session attach` looks
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// The role whose agent this terminal attaches to
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
    /// Attach read-only: nothing typed reaches the agent. `--watch` alone is `--watch true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub watch: Option<bool>,
}

// `conductor daemon`: conductord, the daemon of the control host (design architecture-next § 4,
// `story:daemon-reconcile`).
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum DaemonCommand {
    /// Run conductord, the daemon of the control host, reconciling the instance runtimes from the config file - start what is absent, stop what is gone, restart what changed placement or binary - on a config change, a lost runtime and every daemon.cadence.reconcile, recording one Reconciliation per action, and serving GET 127.0.0.1:<daemon.port>/v1/instances. --install writes, enables and starts the user unit conductord.service, or the system unit when daemon.loopback is false (a box with no local operator, its config file in a git checkout, its secrets on the Secrets service); --uninstall stops and removes it. --daemon-config names a file holding only version and daemon, such as a mounted ConfigMap - at every start the daemon writes the config file from it, keeping the file's globals and instances
    #[command(name = "serve")]
    Serve(DaemonServeArgs),

    /// Ask the daemon of the config file to start a host role now (POST /v1/roles/{role}/start) - supervisor or teacher, one globals.roles lists; the daemon starts it through conductor session start --role <role> at the host placement, its session in conductor_host.slice, and keeps it running; a role the file does not list is refused naming globals.roles
    #[command(name = "start-role")]
    StartRole(DaemonStartRoleArgs),
}

/// The input of the binding's `daemon-start-role` callable, `conductor.instance.StartHostRole`
/// (`story:host-roles-by-daemon`).
#[derive(Debug, Clone, Default, Args)]
pub struct DaemonStartRoleArgs {
    /// The host role to start: supervisor or teacher; required
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
}

/// The input of the binding's `daemon-serve` callable, `conductor.instance.ServeDaemon`
/// (`story:daemon-reconcile`).
#[derive(Debug, Clone, Default, Args)]
pub struct DaemonServeArgs {
    /// Write the user unit conductord.service running this command, then enable and start it.
    /// `--install` alone is `--install true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub install: Option<bool>,
    /// Disable and stop the user unit conductord.service and remove its file. `--uninstall`
    /// alone is `--uninstall true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub uninstall: Option<bool>,
    /// A file holding only `version` and `daemon`, such as a mounted ConfigMap: at every start
    /// the daemon writes the config file from it, keeping the file's globals and instances
    #[arg(long, value_name = "FILE")]
    pub daemon_config: Option<String>,
}

// `conductor instance`: one instance's runtime (design architecture-next § 2, § 5.1,
// `story:instance-api`).
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum InstanceCommand {
    /// Bridge one instance API exchange from standard input to the instance's api.sock (CONDUCTOR_API_SOCKET, else <state>/api.sock) and its answer to standard output; conductord runs it as a substrate pipe session in a box on a remote substrate daemon
    #[command(name = "relay")]
    Relay,
    /// Serve the instance API, the routes of the dashboard under /v1/, on the instance's <state>/api.sock until stopped, and run the instance's loop beside it under the lock conductor run takes (<state>/run/lock), so a conductor run started by hand is refused; while a conductor run holds the lock the runtime yields and takes the loop once it is let go; CONDUCTOR_RUNTIME_LOOP=off serves the API alone; a write takes the token the daemon minted for the instance, in CONDUCTOR_API_TOKEN
    #[command(name = "serve")]
    Serve,
    /// Ask the daemon to upgrade an instance to a released binary (POST /v1/instances/{name}/upgrade) - it fetches the release, installs its binary, runs its conductor doctor, restarts the runtime on it and rolls back to the previous binary unless it answers within daemon.upgrade_timeout; --all goes through every instance of the config file one at a time and stops at the first rollback
    #[command(name = "upgrade")]
    Upgrade(InstanceUpgradeArgs),
}

/// The input of the binding's `instance-upgrade` callable, `conductor.instance.UpgradeInstances`
/// (`story:instance-upgrade`).
#[derive(Debug, Clone, Default, Args)]
pub struct InstanceUpgradeArgs {
    /// The instance to upgrade
    #[arg(long, value_name = "INSTANCE")]
    pub instance: Option<String>,
    /// The release to install, v<x.y.z>; required
    #[arg(long, value_name = "TAG")]
    pub tag: Option<String>,
    /// Upgrade every instance of the config file, one at a time, stopping at the first rollback.
    /// `--all` alone is `--all true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub all: Option<bool>,
}

/// The input of the binding's `join` callable, `conductor.token.JoinDaemon`
/// (`story:token-pool`). The token is read from standard input, never from a flag.
#[derive(Debug, Clone, Default, Args)]
pub struct JoinArgs {
    /// The instance joined
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// The Claude login the token draws on; absent, your operator name
    #[arg(long, value_name = "SEAT")]
    pub seat: Option<String>,
}

/// The input of the binding's `leave` callable, `conductor.token.LeaveDaemon`.
#[derive(Debug, Clone, Default, Args)]
pub struct LeaveArgs {
    /// The instance left
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
}

/// The input of the binding's `token-reveal` callable, `conductor.token.RevealToken`.
#[derive(Debug, Clone, Default, Args)]
pub struct TokenRevealArgs {
    /// The leased token's id
    #[arg(long, value_name = "ID")]
    pub token: Option<String>,
    /// The instance whose model_token to print instead, for an agent at a placement with no box
    #[arg(long, value_name = "INSTANCE")]
    pub model_token: Option<String>,
}

// `conductor token`: the instance's token pool (`story:token-pool`).
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum TokenCommand {
    /// Print the value of a leased token of the instance's pool from the daemon's secrets source, or with --model-token the model_token of an instance whose placement opens no box, for the claude-code kind's harness, whose settings name this command as their apiKeyHelper; refuse a token that is not leased; nothing else prints a value
    #[command(name = "reveal")]
    Reveal(TokenRevealArgs),
}

// `conductor secret`: the secrets the config names (design architecture-next § 9,
// `story:secrets-source`).
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum SecretCommand {
    /// Put a secret's value, read from standard input, into the daemon's secrets source
    /// (daemon.secrets) under --name through the secrets crates, and record its name and source in
    /// the daemon's store; a name already on record is refused (SecretExists) unless --replace
    #[command(name = "put")]
    Put(SecretPutArgs),
}

/// The input of the binding's `secret-put` callable, `conductor.secret.WriteSecret`
/// (`story:secrets-source`). The value is read from standard input, never from a flag.
#[derive(Debug, Clone, Default, Args)]
pub struct SecretPutArgs {
    /// The secret's name, as the config writes it after `secret:`; required
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// Write the value again over a name already on record. `--replace` alone is
    /// `--replace true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub replace: Option<bool>,
}

/// `conductor snapshot`: one sense run and what it observed.
#[derive(Debug, Subcommand)]
pub enum SnapshotCommand {
    /// Complete a snapshot
    #[command(name = "complete-snapshot")]
    CompleteSnapshot(CompleteSnapshotArgs),

    /// Fail a snapshot
    #[command(name = "fail-snapshot")]
    FailSnapshot(FailSnapshotArgs),

    /// Publish the board
    #[command(name = "publish-board")]
    PublishBoard(PublishBoardArgs),

    /// Record a blocker
    #[command(name = "record-blocker")]
    RecordBlocker(RecordBlockerArgs),

    /// Record a collector's run
    #[command(name = "record-collector-run")]
    RecordCollectorRun(RecordCollectorRunArgs),

    /// Record a member a collector could not read
    #[command(name = "record-member-failure")]
    RecordMemberFailure(RecordMemberFailureArgs),

    /// Record a merged pull request
    #[command(name = "record-merged-pull-request")]
    RecordMergedPullRequest(RecordMergedPullRequestArgs),

    /// Record a pull request
    #[command(name = "record-pull-request")]
    RecordPullRequest(RecordPullRequestArgs),

    /// Record a release
    #[command(name = "record-release")]
    RecordRelease(RecordReleaseArgs),

    /// Record a repository
    #[command(name = "record-repository")]
    RecordRepository(RecordRepositoryArgs),

    /// Record a secret sync
    #[command(name = "record-secret-sync")]
    RecordSecretSync(RecordSecretSyncArgs),

    /// Record a session
    #[command(name = "record-session")]
    RecordSession(RecordSessionArgs),

    /// Record a specification status
    #[command(name = "record-specification")]
    RecordSpecification(RecordSpecificationArgs),

    /// Record one window of an account's usage
    #[command(name = "record-usage")]
    RecordUsage(RecordUsageArgs),

    /// Record a workflow run
    #[command(name = "record-workflow-run")]
    RecordWorkflowRun(RecordWorkflowRunArgs),

    /// Start a snapshot
    #[command(name = "start-snapshot")]
    StartSnapshot(StartSnapshotArgs),

    /// Blockers
    #[command(name = "blockers")]
    Blockers(ViewArgs),

    /// Boards
    #[command(name = "boards")]
    Boards(ViewArgs),

    /// Collector runs
    #[command(name = "collector-runs")]
    CollectorRuns(ViewArgs),

    /// Members a collector could not read
    #[command(name = "member-failures")]
    MemberFailures(ViewArgs),

    /// Merged pull requests
    #[command(name = "merged-pull-requests")]
    MergedPullRequests(ViewArgs),

    /// Pull requests
    #[command(name = "pull-requests")]
    PullRequests(ViewArgs),

    /// Releases
    #[command(name = "releases")]
    Releases(ViewArgs),

    /// Repositories
    #[command(name = "repositories")]
    Repositories(ViewArgs),

    /// Secret syncs
    #[command(name = "secret-syncs")]
    SecretSyncs(ViewArgs),

    /// Sessions
    #[command(name = "sessions")]
    Sessions(ViewArgs),

    /// Snapshots
    #[command(name = "snapshots")]
    Snapshots(ViewArgs),

    /// Specifications
    #[command(name = "specifications")]
    Specifications(ViewArgs),

    /// Usage readings
    #[command(name = "usage-readings")]
    UsageReadings(ViewArgs),

    /// Workflow runs
    #[command(name = "workflow-runs")]
    WorkflowRuns(ViewArgs),
}

/// `conductor goal`: the goals dispatches serve.
#[derive(Debug, Subcommand)]
pub enum GoalCommand {
    /// Add a serving repository to a goal
    #[command(name = "add-serving")]
    AddServing(AddServingArgs),

    /// Confirm a goal
    #[command(name = "confirm-goal")]
    ConfirmGoal(ConfirmGoalArgs),

    /// Drop a goal
    #[command(name = "drop-goal")]
    DropGoal(DropGoalArgs),

    /// Mark a goal met
    #[command(name = "mark-goal-met")]
    MarkGoalMet(MarkGoalMetArgs),

    /// Propose a goal
    #[command(name = "propose-goal")]
    ProposeGoal(ProposeGoalArgs),

    /// Remove a serving repository from a goal
    #[command(name = "remove-serving")]
    RemoveServing(RemoveServingArgs),

    /// Serving repositories
    #[command(name = "servings")]
    GoalServings(ViewArgs),

    /// Goals
    #[command(name = "goals")]
    Goals(ViewArgs),
}

/// `conductor repository`: which repositories have their issues and pull requests processed.
#[derive(Debug, Subcommand)]
pub enum RepositoryCommand {
    /// Mark a repository active
    #[command(name = "activate-repository")]
    ActivateRepository(ActivateRepositoryArgs),

    /// Mark a repository inactive
    #[command(name = "deactivate-repository")]
    DeactivateRepository(DeactivateRepositoryArgs),

    /// Mark a repository for the first time
    #[command(name = "mark-repository")]
    MarkRepository(MarkRepositoryArgs),

    /// Repository marks
    #[command(name = "repository-marks")]
    RepositoryMarks(ViewArgs),

    /// Every repository's effective activity, decided by its mark or computed from the newest complete snapshot
    #[command(name = "activity")]
    Activity(RepositoryActivityArgs),

    /// List each checkout of the instance whose Gates hooks lag the b10x-gates on PATH or are absent; --apply runs b10x-gates install in each with the instance's gates_policy and prints the hook's version before and after
    #[command(name = "refresh-hooks")]
    RefreshHooks(RefreshHooksArgs),

    /// What shipped since an instant, from the newest complete snapshot's GitHub observations
    #[command(name = "shipped")]
    Shipped(RepositoryShippedArgs),
}

/// `conductor decision`: decision requests and the decisions that answer them.
#[derive(Debug, Subcommand)]
pub enum DecisionCommand {
    /// Amend an escalated decision request
    #[command(name = "amend-escalated-request")]
    AmendEscalatedRequest(AmendEscalatedRequestArgs),

    /// Answer an escalated decision request
    #[command(name = "answer-escalated-request")]
    AnswerEscalatedRequest(AnswerEscalatedRequestArgs),

    /// Answer a decision request
    #[command(name = "answer-request")]
    AnswerRequest(AnswerRequestArgs),

    /// Escalate a decision request
    #[command(name = "escalate-request")]
    EscalateRequest(EscalateRequestArgs),

    /// Raise a decision request
    #[command(name = "raise-decision-request")]
    RaiseDecisionRequest(RaiseDecisionRequestArgs),

    /// Record a decision taken by conductor
    #[command(name = "record-conductor-decision")]
    RecordConductorDecision(RecordConductorDecisionArgs),

    /// Record a decision taken by the operator
    #[command(name = "record-operator-decision")]
    RecordOperatorDecision(RecordOperatorDecisionArgs),

    /// Answer an escalated request as the operator
    #[command(name = "answer-escalated-as-operator")]
    AnswerEscalatedAsOperator(RecordOperatorDecisionForRequestArgs),

    /// Reverse a decision taken by conductor
    #[command(name = "reverse-conductor-decision")]
    ReverseConductorDecision(ReverseConductorDecisionArgs),

    /// Reverse a decision as the operator
    #[command(name = "reverse-operator-decision")]
    ReverseOperatorDecision(ReverseOperatorDecisionArgs),

    /// Withdraw a decision request
    #[command(name = "withdraw-request")]
    WithdrawRequest(WithdrawRequestArgs),

    /// Decisions
    #[command(name = "decisions")]
    Decisions(ViewArgs),

    /// Class-H to-do
    #[command(name = "hands-todo")]
    HandsTodo(ViewArgs),

    /// Decision requests
    #[command(name = "requests")]
    Requests(ViewArgs),

    /// Classify a decision request in code and raise it; answer class C through the Decider adapter (claude, or fake, which answers the recommendation) and record it, printing the [DECISION] line; escalate class O and H with the request's options, its recommendation as default and a 48 h deadline, printing the [ESCALATION] line
    #[command(name = "handle")]
    Handle(HandleRequestArgs),

    /// Show one decision
    #[command(name = "show")]
    Show(ShowDecisionArgs),
}

/// The input of the binding's `decision-handle` callable, `conductor.decision.HandleRequest`
/// (`story:decide-model-adapter`).
#[derive(Debug, Clone, Args)]
pub struct HandleRequestArgs {
    /// The file holding one `[DECISION-REQUEST]` message, or `-` for standard input
    #[arg(long, value_name = "FILE")]
    pub request: Option<String>,
    /// The Decider adapter that answers class C: claude (the default) or fake
    #[arg(long)]
    pub decider: Option<String>,
}

/// The input of the binding's `decision-show` callable, `conductor.decision.ShowDecision`.
#[derive(Debug, Clone, Args)]
pub struct ShowDecisionArgs {
    /// The decision's id, `DEC-…`
    #[arg(long)]
    pub decision_id: Option<String>,
    /// `text`, one `field: value` line each, or `json`, one object. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

/// `conductor controller`: one controller session per repository.
#[derive(Debug, Subcommand)]
pub enum ControllerCommand {
    /// Pause a controller
    #[command(name = "pause-controller")]
    PauseController(PauseControllerArgs),

    /// Resume a controller
    #[command(name = "resume-controller")]
    ResumeController(ResumeControllerArgs),

    /// Revise a charter
    #[command(name = "revise-charter")]
    ReviseCharter(ReviseCharterArgs),

    /// Start a controller
    #[command(name = "start-controller")]
    StartController(StartControllerArgs),

    /// Stop a controller
    #[command(name = "stop-controller")]
    StopController(StopControllerArgs),

    /// All controllers
    #[command(name = "all-controllers")]
    AllControllers(ViewArgs),

    /// Print one line per Running or Paused controller, its repository, state, session name and id; --all prints the table of every controller, --format renders every one as text, json, jsonl or markdown; create nothing
    #[command(name = "controllers")]
    Controllers(ControllersArgs),
}

/// `conductor dispatch`: briefs to controllers and their reports.
#[derive(Debug, Subcommand)]
pub enum DispatchCommand {
    /// Cancel a dispatch
    #[command(name = "cancel-dispatch")]
    CancelDispatch(CancelDispatchArgs),

    /// Record a link on a dispatch
    #[command(name = "record-link")]
    RecordDispatchLink(RecordDispatchLinkArgs),

    /// Report a dispatch blocked
    #[command(name = "report-blocked")]
    ReportBlocked(ReportBlockedArgs),

    /// Report a dispatch done
    #[command(name = "report-done")]
    ReportDone(ReportDoneArgs),

    /// Report a dispatch failed
    #[command(name = "report-failed")]
    ReportFailed(ReportFailedArgs),

    /// Report a dispatch started
    #[command(name = "report-started")]
    ReportStarted(ReportStartedArgs),

    /// Report a dispatch unblocked
    #[command(name = "report-unblocked")]
    ReportUnblocked(ReportUnblockedArgs),

    /// Send a dispatch
    #[command(name = "send-dispatch")]
    SendDispatch(SendDispatchArgs),

    /// Dispatches
    #[command(name = "dispatches")]
    Dispatches(ViewArgs),
}

/// `conductor message`: every message between conductor and a controller.
#[derive(Debug, Subcommand)]
pub enum MessageCommand {
    /// Handle a message
    #[command(name = "handle-message")]
    HandleMessage(HandleMessageArgs),

    /// Receive a message
    #[command(name = "receive-message")]
    ReceiveMessage(ReceiveMessageArgs),

    /// Reject a message
    #[command(name = "reject-message")]
    RejectMessage(RejectMessageArgs),

    /// Route a need to a dispatch
    #[command(name = "route-need")]
    RouteNeed(RouteNeedArgs),

    /// Messages
    #[command(name = "messages")]
    Messages(ViewArgs),

    /// Send a message through the mailbox: read --text, a file or - for standard input, and its first line; leave the whole text as <state>/mailbox/<to>/<ulid>.md and record it received with transport Mailbox and that body path, then print its id; print the format, exit 1 and write nothing when the first line does not parse
    #[command(name = "send")]
    Send(SendArgs),
}

/// The input of the binding's `message-send` callable, `conductor.dispatch.SendToMailbox`
/// (`story:mailbox-send-and-deliver`).
#[derive(Debug, Clone, Args)]
pub struct SendArgs {
    /// The recipient, whose mailbox directory under the state directory receives the file:
    /// conductor
    #[arg(long)]
    pub to: Option<String>,
    /// The sending session's full name
    #[arg(long)]
    pub from: Option<String>,
    /// The file holding the message, or - for standard input; its first line is one of the
    /// message forms
    #[arg(long, value_name = "FILE")]
    pub text: Option<String>,
}

/// `conductor resource`: build slots, disk and usage asked for before they are spent.
#[derive(Debug, Subcommand)]
pub enum ResourceCommand {
    /// Grant a resource
    #[command(name = "grant-resource")]
    GrantResource(GrantResourceArgs),

    /// Refuse a resource
    #[command(name = "refuse-resource")]
    RefuseResource(RefuseResourceArgs),

    /// Release a granted resource
    #[command(name = "release-resource")]
    ReleaseResource(ReleaseResourceArgs),

    /// Request a resource
    #[command(name = "request-resource")]
    RequestResource(RequestResourceArgs),

    /// Resource requests
    #[command(name = "resource-requests")]
    ResourceRequests(ViewArgs),

    /// Token spend per repository and session, read from the session transcripts
    #[command(name = "usage")]
    Usage(ResourceUsageArgs),
}

/// The input of the binding's `resource-usage` callable, `conductor.observation.ResourceUsage`.
#[derive(Debug, Clone, Args)]
pub struct ResourceUsageArgs {
    /// Count only the calls at or after this RFC 3339 instant, such as 2026-10-07T00:00:00Z.
    /// Default: every call
    #[arg(long, value_name = "INSTANT")]
    pub since: Option<String>,
    /// How the tables are rendered: text, json, jsonl or markdown. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

/// `conductor guard`: the controller hook and the record of its verdicts.
#[derive(Debug, Subcommand)]
pub enum GuardCommand {
    /// Record a guard decision
    #[command(name = "record-guard-decision")]
    RecordGuardDecision(Box<RecordGuardDecisionArgs>),

    /// Guard decisions
    #[command(name = "guard-decisions")]
    GuardDecisions(ViewArgs),
}

/// `conductor session`: the instance's conductor lease, what the run daemon delivers to its
/// sessions, and the daemon's ticks.
#[derive(Debug, Subcommand)]
pub enum SessionCommand {
    /// Claim the conductor lease
    #[command(name = "claim")]
    Claim(ClaimLeaseArgs),

    /// Deliver a message to a session
    #[command(name = "deliver-message")]
    DeliverMessage(DeliverMessageArgs),

    /// Record a tick of the run daemon
    #[command(name = "record-run-tick")]
    RecordRunTick(RecordRunTickArgs),

    /// Deliveries to sessions
    #[command(name = "deliveries")]
    Deliveries(ViewArgs),

    /// Conductor leases
    #[command(name = "leases")]
    Leases(ViewArgs),

    /// Ticks of the run daemon
    #[command(name = "run-ticks")]
    RunTicks(ViewArgs),

    /// Attach this terminal to a live session's tmux, by --name or --role, or through a picker of every instance's sessions without either; detach with ctrl-] then d, the session runs on; --watch attaches read-only, and follows the terminal log of a session started before the tmux launcher; refuse inside a session, an unknown session, and a read-write attach to a session started before the tmux launcher
    #[command(name = "attach")]
    Attach(AttachArgs),

    /// Type a text into a running session, then Enter, and record the delivery and its dispatch line: into its tmux launcher, or, for a session without one that the harness's session list names live, through the harness's attach in a tmux pane made for the one delivery; --text is a file, or - for standard input, which is kept as <state>/mailbox/delivered/<ulid>.md; refuse a session neither route reaches, naming it, without trying again
    #[command(name = "deliver")]
    Deliver(DeliverArgs),

    /// Print one line per live session of every instance (or --instance) whose state holds a launcher's files for it: its instance, its name, and attach (read-write or --watch) or watch (--watch only, started before the tmux launcher)
    #[command(name = "list")]
    List(ListSessionsArgs),

    /// Start the session of the role director or maintainer (conductor or conductor-dev, their old names, for this release) in the box the instance's confinement asks for, measure the box at its boundary and write the measurement under the state directory; refuse an unknown role, a session of that name already running, and under strict a provider not available or a box short of the request
    #[command(name = "start")]
    Start(SessionRoleArgs),

    /// Stop the live session of the role director or maintainer (conductor or conductor-dev, their old names, for this release) through the provider that started it; refuse an unknown role and a role with no live session
    #[command(name = "stop")]
    Stop(SessionRoleArgs),
}

/// `conductor host`: the host ledger of slots and disk reservations every instance shares, and
/// the reclaim runs that free disk.
#[derive(Debug, Subcommand)]
pub enum HostCommand {
    /// Record a reclaim run
    #[command(name = "record-reclaim")]
    RecordReclaim(RecordReclaimArgs),

    /// Record a run's peak
    #[command(name = "record-peak")]
    RecordPeak(RecordPeakArgs),

    /// Release a host grant
    #[command(name = "release-grant")]
    ReleaseGrant(ReleaseGrantArgs),

    /// Request disk for a command
    #[command(name = "request-disk")]
    RequestDisk(RequestDiskArgs),

    /// Answer a disk request
    #[command(name = "reserve-disk")]
    ReserveDisk(ReserveDiskArgs),

    /// Start the reserved run
    #[command(name = "start-run")]
    StartRun(StartRunArgs),

    /// Take a host grant
    #[command(name = "take-host-grant")]
    TakeHostGrant(TakeHostGrantArgs),

    /// Withdraw a waiting disk request
    #[command(name = "withdraw-disk-request")]
    WithdrawDiskRequest(WithdrawDiskRequestArgs),

    /// Host grants
    #[command(name = "grants")]
    Grants(ViewArgs),

    /// Reclaim runs
    #[command(name = "reclaim-runs")]
    ReclaimRuns(ViewArgs),

    /// Free what needs no decision until free disk is back over the floor, printing each step with the bytes it freed; exit 1 listing what is left by path and size when the floor is not reached
    #[command(name = "reclaim")]
    Reclaim(HostReclaimArgs),
}

/// `conductor metric`: the measured quantities of the self-improvement loop, their daily
/// readings and the targets that make some of them KPIs.
#[derive(Debug, Subcommand)]
pub enum MetricCommand {
    /// Define a metric
    #[command(name = "define-metric")]
    DefineMetric(DefineMetricArgs),

    /// Judge a metric's reading against its target
    #[command(name = "judge-metric-reading")]
    JudgeMetricReading(JudgeMetricReadingArgs),

    /// Record a metric's reading
    #[command(name = "record-metric-reading")]
    RecordMetricReading(RecordMetricReadingArgs),

    /// Set a metric's target
    #[command(name = "set-metric-target")]
    SetMetricTarget(SetMetricTargetArgs),

    /// Supersede a metric's reading
    #[command(name = "supersede-metric-reading")]
    SupersedeMetricReading(SupersedeMetricReadingArgs),

    /// KPI readings
    #[command(name = "kpi-readings")]
    KpiReadings(ViewArgs),

    /// KPIs
    #[command(name = "kpis")]
    Kpis(ViewArgs),

    /// Metric readings
    #[command(name = "metric-readings")]
    MetricReadings(ViewArgs),

    /// Metrics
    #[command(name = "metrics")]
    Metrics(ViewArgs),

    /// Compute one UTC day's readings of K1, K2, K4, K5, K6 and K12 from the records' decision and dispatch logs, the newest complete snapshot and the store's daily rollups, define each such metric that is absent, and record one reading per metric and scope, judged against the metric's target; print one line per reading, and name what had no value
    #[command(name = "compute")]
    Compute(MetricComputeArgs),
}

/// The input of the binding's `metric-compute` callable, `conductor.improvement.ComputeMetrics`
/// (`story:kpi-records`). The day is text the handler reads, so a value it refuses is named
/// there.
#[derive(Debug, Clone, Args)]
pub struct MetricComputeArgs {
    /// The UTC day to compute, YYYY-MM-DD
    #[arg(long)]
    pub day: Option<String>,
}

/// `conductor finding`: friction the loop detected, the draft story each is filed as, and the
/// week-later check of the metric it should move.
#[derive(Debug, Subcommand)]
pub enum FindingCommand {
    /// File a finding as a draft story
    #[command(name = "file-finding")]
    FileFinding(FileFindingArgs),

    /// Raise a friction finding
    #[command(name = "raise-finding")]
    RaiseFinding(RaiseFindingArgs),

    /// Verify a filed finding
    #[command(name = "verify-finding")]
    VerifyFinding(VerifyFindingArgs),

    /// Friction findings
    #[command(name = "findings")]
    Findings(ViewArgs),
}

/// `conductor tuner`: the settings changes the tuner proposed after each daily tick
/// (`story:tuner`).
#[derive(Debug, Subcommand)]
pub enum TunerCommand {
    /// Settings proposals
    #[command(name = "settings-proposals")]
    SettingsProposals(ViewArgs),
}

/// The input of the binding's `gc-reclaim` callable, `conductor.host.ReclaimDisk`. The format is
/// text the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct HostReclaimArgs {
    /// How the steps are printed: text or json. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

// `conductor build`: the command the binding declares over the `local` callable `build-run`
// (`story:build-run`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum BuildCommand {
    /// Reserve disk for a command's expected peak in the host ledger, reclaiming first when it does not fit; run it in its own process group under the disk watchdog, record its peak, release the reservation when it exits or is killed, and exit with its status; exit 2 naming the shortfall when reclaim cannot reach the size
    #[command(name = "run")]
    Run(BuildRunArgs),
}

/// The input of the binding's `build-run` callable, `conductor.host.BuildRun`: the size and the
/// repository as flags, and the command after `--`, taken as it is written.
#[derive(Debug, Clone, Args)]
pub struct BuildRunArgs {
    /// The command's expected peak, in GiB: `8G`, or `8`; or `auto`, which is also what no
    /// `--reserve` means: the largest peak the ledger recorded for the same repository and command
    /// in the last 14 days plus 50 %, at least 1G, and 4G when none is recorded
    #[arg(long, value_name = "SIZE")]
    pub reserve: Option<String>,
    /// The repository the command builds, recorded with the reservation
    #[arg(long)]
    pub repository: Option<String>,
    /// A directory to measure besides the run's own (`target/`, the working directory, and
    /// `CARGO_TARGET_DIR` when the command's environment sets it); repeatable
    #[arg(long, value_name = "DIR")]
    pub measure: Vec<String>,
    /// The command and its arguments, after `--`
    #[arg(last = true, value_name = "COMMAND")]
    pub command: Vec<String>,
}

// `conductor broker`: the command the binding declares over the `local` callable `broker-serve`
// (`story:tool-broker-read`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum BrokerCommand {
    /// Serve the instance's broker until stopped, on one socket per registered session under the state directory's broker/; check each call against the session's grant, run it and record it
    #[command(name = "serve")]
    Serve,
}

// `conductor watch`: the command the binding declares over the `local` callable `watch-run`
// (`story:watch-command`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum WatchCommand {
    /// Watch sessions, usage limits, conductor's context, free disk and main's CI; exit after the first pass that finds a change
    #[command(name = "run")]
    Run(WatchRunArgs),
}

/// The input of the binding's `watch-run` callable, `conductor.observation.WatchRun`.
#[derive(Debug, Clone, Args)]
pub struct WatchRunArgs {
    /// Seconds from the end of one pass over the sessions, usage limits, conductor's context and
    /// free disk to the start of the next. Default: 180
    #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(u64).range(1..))]
    pub every: Option<u64>,
    /// Seconds between two reads of main's CI; the first pass always reads it. Default: 900
    #[arg(long, value_name = "SECONDS", value_parser = clap::value_parser!(u64).range(1..))]
    pub ci_every: Option<u64>,
    /// Print each change and go on watching, rather than exit after the first pass that finds
    /// one. `--follow` alone is `--follow true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub follow: Option<bool>,
}

// `conductor records`: the command the binding declares over the `local` callable
// `records-check` (`story:records-from-commands`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum RecordsCommand {
    /// Check the records' dispatch and decision logs; exit 1 naming each decision id a dispatch line cites that no decision line or store decision holds, each line whose keys its log does not declare, and each decision line in the dispatch log
    #[command(name = "check")]
    Check(RecordsCheckArgs),
}

/// The input of the binding's `status` callable, `conductor.observation.Status`
/// (`story:status-command`). The format is text the handler reads, so a value it refuses is named
/// there.
#[derive(Debug, Clone, Args)]
pub struct StatusArgs {
    /// How the status is printed: text or json. Default: text
    #[arg(long)]
    pub format: Option<String>,
    /// Print every part under its heading, sessions and every open dispatch included, instead of
    /// the attention lines. `--full` alone is `--full true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub full: Option<bool>,
}

/// The input of the binding's `controller-controllers` callable,
/// `conductor.dispatch.ListControllers` (`story:conductor-context-budget`). The format is text
/// the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct ControllersArgs {
    /// Print the table of every controller instead of one line per Running or Paused one.
    /// `--all` alone is `--all true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub all: Option<bool>,
    /// Render every controller as text, json, jsonl or markdown, as `all-controllers` does
    #[arg(long)]
    pub format: Option<String>,
}

/// The input of the binding's `records-check` callable, `conductor.dispatch.CheckRecords`. The
/// format is text the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct RecordsCheckArgs {
    /// How the findings are printed: text or json. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

// `conductor need`: the command the binding declares over the `local` callable `need-new`
// (`story:need-id-unique`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum NeedCommand {
    /// Print the next NEED id no earlier NEED used, N-<yyyymmdd>-<nn>, and keep it in the state directory; skip an id the records' hand-overs, decisions and dispatches already cite
    #[command(name = "new")]
    New(NeedNewArgs),
}

/// The input of the binding's `need-new` callable, `conductor.direction.NewNeed`. The day is text
/// the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct NeedNewArgs {
    /// The UTC date the id is minted for, yyyy-mm-dd. Default: today
    #[arg(long)]
    pub day: Option<String>,
}

// `conductor role`: the command the binding declares over the `local` callable `role-show`
// (`story:roles-in-the-binary`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum RoleCommand {
    /// Print the profile the binary carries for a role (maintainer, supervisor, teacher, controller, or an old name of one), byte for byte, reading no config and no store; exit 1 naming a role the binary carries none for
    #[command(name = "show")]
    Show(RoleShowArgs),
}

/// The input of the binding's `role-show` callable, `conductor.agent.ShowRole`. The role is text
/// the handler reads, so a name it carries no profile for is named there.
#[derive(Debug, Clone, Args)]
pub struct RoleShowArgs {
    /// The role whose profile to print: maintainer, supervisor, teacher or controller
    #[arg(long)]
    pub role: Option<String>,
}

// `conductor rules`: the command the binding declares over the `local` callable `rules-show`
// (`story:rules-show`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum RulesCommand {
    /// Print items of the instance's rules.md verbatim, by number in the order given or every item under a heading, without opening the store; exit 1 naming a number or heading the file does not hold
    #[command(name = "show")]
    Show(RulesShowArgs),
}

/// The input of the binding's `rules-show` callable, `conductor.direction.Rules`. Each field is
/// text the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct RulesShowArgs {
    /// The item numbers to print, comma-separated (43,49,51), in the order given
    #[arg(long)]
    pub item: Option<String>,

    /// A heading of rules.md, any case: print every item under each heading of that text
    #[arg(long)]
    pub section: Option<String>,

    /// How the items are printed: text (verbatim) or json. Default: text
    #[arg(long)]
    pub format: Option<String>,

    /// One line per item instead: its number and its first sentence. `--brief` alone is
    /// `--brief true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub brief: Option<bool>,

    /// A git revision of the records directory: print only the items whose text differs from
    /// that revision's rules.md, or that it does not hold, and name the ones it held that are gone
    #[arg(long)]
    pub changed_since: Option<String>,

    /// A role (all, conductor, controller, worker, maintainer): print the rules of its file,
    /// rules/<role>.md of the records, one per line; with --changed-since, the changed ones
    #[arg(long)]
    pub role: Option<String>,
}

// `conductor store`: the command the binding declares over the `local` callable `store-migrate`
// (`story:store-on-eventlog-tree`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum StoreCommand {
    /// Read the eventlog tree under the state directory once into the store's database, record the migration there, and print the counts read; refuse a tree read before (AlreadyMigrated) or none there (SourceMissing)
    #[command(name = "migrate")]
    Migrate(StoreMigrateArgs),
}

/// The input of the binding's `store-migrate` callable, `conductor.observation.StoreMigrate`. The
/// format is text the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct StoreMigrateArgs {
    /// How the counts moved are printed: text or json. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

/// The input of the binding's `gc` callable, `conductor.observation.CollectGarbage`. The format
/// is text the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct GcArgs {
    /// List the rows the disk panel marks reclaimable, with their bytes, and remove nothing.
    /// `--dry-run` alone is `--dry-run true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub dry_run: Option<bool>,
    /// How the rows are printed: text or json. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

/// The input of the binding's `repository-shipped` callable,
/// `conductor.observation.RepositoryShipped`. Both are text the handler reads, so a value it
/// refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct RepositoryShippedArgs {
    /// The instant, RFC 3339: what was released or merged at or after it is listed
    #[arg(long)]
    pub since: Option<String>,
    /// How the rows are rendered: text, json, jsonl or markdown. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

/// The input of the binding's `repository-refresh-hooks` callable,
/// `conductor.observation.RefreshHooks` (`story:gates-hooks-fresh`). The format is text the
/// handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct RefreshHooksArgs {
    /// Run `b10x-gates install` in each checkout listed, with the instance's `gates_policy`.
    /// `--apply` alone is `--apply true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub apply: Option<bool>,
    /// How the rows are rendered: text or json. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

/// The input of the binding's `repository-activity` callable,
/// `conductor.direction.RepositoryActivity`. The format is text the handler reads, so a value it
/// refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct RepositoryActivityArgs {
    /// How the rows are rendered: text, json, jsonl or markdown. Default: text
    #[arg(long)]
    pub format: Option<String>,
}

// `conductor config`: the commands the binding declares over the `local` callables
// `config-validate` and `config-show` (`story:config-file`) and `config-workspace`
// (`story:workspace-from-config`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Convert a conductor.config/1 file to conductor.config/2 once, noting each key dropped or
    /// moved, and write a session_prefix into each instance that names none; --check prints what
    /// it would write and writes nothing
    #[command(name = "migrate")]
    Migrate(MigrateConfigArgs),

    /// Change one key of the config file at one level - global, instance or role - validated,
    /// written, committed to the config repository with the actor and the reason, and recorded
    /// in the daemon's store as a ConfigRevision; the daemon reconciles on the changed file
    #[command(name = "put")]
    Put(PutConfigArgs),

    /// Print one instance's effective config, after defaults, as a config file, and the host's
    /// roles from host.yaml under `host` (not in yaml)
    #[command(name = "show")]
    Show(ShowConfigArgs),

    /// Check the config file and the host file host.yaml; exit 1 naming each problem by its YAML
    /// path
    #[command(name = "validate")]
    Validate(ValidateConfigArgs),

    /// Write the instance's workspace file from its config; --check names each difference
    #[command(name = "workspace")]
    Workspace(WriteWorkspaceArgs),
}

/// The input of the binding's `config-workspace` callable, `conductor.config.WriteWorkspace`.
#[derive(Debug, Clone, Args)]
pub struct WriteWorkspaceArgs {
    /// The instance whose workspace file is written. Default: $CONDUCTOR_INSTANCE, else the
    /// file's default, else its only instance
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// Write nothing; exit 1 naming each member that differs from the file. `--check` alone is
    /// `--check true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub check: Option<bool>,
}

/// The input of the binding's `config-migrate` callable, `conductor.config.MigrateConfig`
/// (`story:instance-names-unique`).
#[derive(Debug, Clone, Args)]
pub struct MigrateConfigArgs {
    /// Write nothing; print the session_prefix each instance without one would get. `--check`
    /// alone is `--check true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub check: Option<bool>,
}

/// The input of the binding's `config-put` callable, `conductor.config.ChangeConfig`
/// (`story:config-globals`).
#[derive(Debug, Clone, Args)]
pub struct PutConfigArgs {
    /// The level the key is written at: global, instance or role; required
    #[arg(long, value_name = "LEVEL")]
    pub level: Option<String>,
    /// The instance an instance or role change is written under
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// The role of the instance a role change is written under
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
    /// The key under the level, its words joined by '.', such as thresholds.silent_session;
    /// required
    #[arg(long, value_name = "PATH")]
    pub path: Option<String>,
    /// The new value, as YAML reads it. Absent, the key is removed at that level
    #[arg(long, value_name = "VALUE")]
    pub value: Option<String>,
    /// Why, written into the commit and the revision; required
    #[arg(long, value_name = "TEXT")]
    pub reason: Option<String>,
}

/// The input of the binding's `config-validate` callable, `conductor.config.ValidateConfig`.
#[derive(Debug, Clone, Args)]
pub struct ValidateConfigArgs {
    /// An instance the config must name. Default: $CONDUCTOR_INSTANCE, else none is checked
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
}

/// The input of the binding's `trust` callable, `conductor.config.TrustWorkspaces`.
#[derive(Debug, Clone, Args)]
pub struct TrustArgs {
    /// The instance whose checkouts and records directory are trusted. Default:
    /// $CONDUCTOR_INSTANCE, else the file's default, else its only instance
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
}

/// The input of the binding's `init` callable, `conductor.config.InitRecords`.
#[derive(Debug, Clone, Args)]
pub struct InitArgs {
    /// The instance whose records directory is written. Default: $CONDUCTOR_INSTANCE, else the
    /// file's default, else its only instance
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
}

/// The input of the binding's `setup` callable, `conductor.config.SetupRepository`.
#[derive(Debug, Clone, Args)]
pub struct SetupArgs {
    /// The repository checked: its directory. Default: the working directory
    #[arg(long, value_name = "DIR")]
    pub repository: Option<PathBuf>,
    /// Write the instance config naming the repository, run conductor init and conductor trust,
    /// and activate its checkouts root as a worktree workspace with a profile written beside the
    /// config file, when they are missing; never log in, install or migrate. `--apply` alone is
    /// `--apply true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub apply: Option<bool>,
}

/// The input of the binding's `spawn` callable, `conductor.dispatch.SpawnController`.
#[derive(Debug, Clone, Args)]
pub struct SpawnArgs {
    /// The repository whose controller starts: its checkout under the instance's checkouts root,
    /// `<repo>` or `<group>/<repo>`, and its charter `<records>/charters/<repo>.md`
    #[arg(long, value_name = "NAME")]
    pub repository: Option<String>,
}

/// The input of the binding's `session-deliver` callable, `conductor.session.DeliverToSession`
/// (`story:mailbox-send-and-deliver`).
#[derive(Debug, Clone, Args)]
pub struct DeliverArgs {
    /// The session's full name, as its launcher socket <state>/sessions/<name>.tmux or the harness's session list names it
    #[arg(long)]
    pub name: Option<String>,
    /// The file holding the text, or - for standard input
    #[arg(long, value_name = "FILE")]
    pub text: Option<String>,
}

/// The input of `conductor.session.DeliverMessage`.
#[derive(Debug, Clone, Args)]
pub struct DeliverMessageArgs {
    /// SessionName
    #[arg(long)]
    pub session_name: Option<String>,
    /// string
    #[arg(long)]
    pub first_line: Option<String>,
    /// string, optional
    #[arg(long)]
    pub body_path: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub delivered_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of the binding's `session-start` and `session-stop` callables,
/// `conductor.session.StartSession` and `conductor.session.StopSession`.
#[derive(Debug, Clone, Args)]
pub struct SessionRoleArgs {
    /// The role whose session starts or stops: director or maintainer, or conductor or conductor-dev, their old names, for this release
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
}

/// The input of the binding's `session-attach` callable, `conductor.session.AttachSession`.
#[derive(Debug, Clone, Args)]
pub struct AttachArgs {
    /// The session's full name, `<session_prefix>-<role>` or a controller's
    #[arg(long, value_name = "NAME")]
    pub name: Option<String>,
    /// The role whose session attaches, as for `session start`
    #[arg(long, value_name = "ROLE")]
    pub role: Option<String>,
    /// Look only in this instance of the config file. Default: every instance
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// Attach read-only: nothing typed reaches the session. `--watch` alone is `--watch true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub watch: Option<bool>,
}

/// The input of the binding's `session-list` callable, `conductor.session.ListSessions`.
#[derive(Debug, Clone, Args)]
pub struct ListSessionsArgs {
    /// List only this instance of the config file. Default: every instance
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// How the rows are printed: text, one `<name> <attach|watch>` line each, or json. Default:
    /// text
    #[arg(long)]
    pub format: Option<String>,
}

/// The input of the binding's `upgrade` callable, `conductor.config.UpgradeBinary`.
#[derive(Debug, Clone, Args)]
pub struct UpgradeArgs {
    /// Only compare the installed version with the newest release; change nothing. `--check`
    /// alone is `--check true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub check: Option<bool>,
    /// The release installed, x.y.z, newer or older than the installed one. Default: the newest
    #[arg(long, value_name = "X.Y.Z")]
    pub release: Option<String>,
}

/// The input of the binding's `config-show` callable, `conductor.config.ShowConfig`. The format
/// is text the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct ShowConfigArgs {
    /// How the config is rendered: text, one `path: value` line each; json; each with the host's
    /// roles under `host`; or yaml, a config file, without them. Default: text
    #[arg(long)]
    pub format: Option<String>,
    /// The instance shown. Default: $CONDUCTOR_INSTANCE, else the config's only instance
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
    /// Print every key of the instance with its effective value and the level it came from,
    /// default, global, instance or role (`conductor.config.EffectiveSettings`). `--effective`
    /// alone is `--effective true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub effective: Option<bool>,
}

/// The source of `--input-json`: only standard input.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum JsonInput {
    /// Standard input.
    #[value(name = "-")]
    Stdin,
}

/// How a view renders its rows.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum Format {
    /// Aligned text for a terminal.
    #[default]
    Text,
    /// One JSON array.
    Json,
    /// One JSON object per line.
    Jsonl,
    /// A Markdown table.
    Markdown,
}

/// The options every view takes, and the only ones.
#[derive(Debug, Clone, Copy, Args)]
pub struct ViewArgs {
    /// How the rows are rendered
    #[arg(long, value_enum, default_value_t = Format::Text)]
    pub format: Format,
}

impl ViewArgs {
    /// Renders the rows of a view whose fields are `columns` as `--format` asks. Each row holds
    /// one value per column, in the columns' order; JSON objects keep that order.
    ///
    /// In `text` and `markdown`, a string value is written as it is and any other value as
    /// compact JSON, and a line break inside a value (U+2028 and U+2029 included) is written as a
    /// space (`text`) or `<br>` (`markdown`) and any other control character or bidirectional
    /// control (U+202A to U+202E, U+2066 to U+2069) as a space, so each row stays one line in
    /// its order and moves no terminal. `markdown` also escapes `\` and `|`, so each
    /// value is one cell that reads back as the value.
    #[must_use]
    pub fn render(&self, columns: &[&str], rows: &[Vec<Value>]) -> String {
        match self.format {
            Format::Text => text_table(columns, rows),
            Format::Json => {
                let objects: Vec<String> = rows.iter().map(|row| object(columns, row)).collect();
                format!("[{}]\n", objects.join(","))
            }
            Format::Jsonl => rows.iter().map(|row| object(columns, row) + "\n").collect(),
            Format::Markdown => markdown_table(columns, rows),
        }
    }
}

/// One row as a compact JSON object, its keys in the columns' order.
fn object(columns: &[&str], row: &[Value]) -> String {
    let fields: Vec<String> = columns
        .iter()
        .zip(row)
        .map(|(column, value)| format!("{}:{value}", Value::from(*column)))
        .collect();
    format!("{{{}}}", fields.join(","))
}

/// A value as a table cell shows it: a string as it is, anything else as compact JSON.
fn cell(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}

/// Whether `character` is a bidirectional embedding, override or isolate (U+202A to U+202E,
/// U+2066 to U+2069), which reorders what a terminal shows of the text around it.
#[must_use]
pub fn bidirectional(character: char) -> bool {
    matches!(character, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// `text` with every line break, U+2028 and U+2029 included, written as `line_break`, and every
/// other control character (C0, DEL and C1) and every [`bidirectional`] control as a space: a
/// value moves no terminal, reorders no row and splits none.
fn one_line(text: &str, line_break: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.replace("\r\n", "\n").chars() {
        match character {
            '\r' | '\n' | '\u{2028}' | '\u{2029}' => out.push_str(line_break),
            control if control.is_control() || bidirectional(control) => out.push(' '),
            other => out.push(other),
        }
    }
    out
}

/// A header of the column names, then one line per row, each column padded to its widest value
/// and two spaces from the next.
fn text_table(columns: &[&str], rows: &[Vec<Value>]) -> String {
    let lines: Vec<Vec<String>> = std::iter::once(columns.iter().map(|&c| c.to_owned()).collect())
        .chain(rows.iter().map(|row| {
            row.iter()
                .map(|value| one_line(&cell(value), " "))
                .collect()
        }))
        .collect();
    let widths: Vec<usize> = (0..columns.len())
        .map(|column| {
            lines
                .iter()
                .filter_map(|line| line.get(column))
                .map(|text| text.chars().count())
                .max()
                .unwrap_or(0)
        })
        .collect();
    let mut out = String::new();
    for line in &lines {
        let mut text = String::new();
        for (column, (value, width)) in line.iter().zip(&widths).enumerate() {
            if column > 0 {
                text.push_str("  ");
            }
            let _ = write!(text, "{value:<width$}");
        }
        out.push_str(text.trim_end());
        out.push('\n');
    }
    out
}

/// A Markdown table: a header of the column names, its separator, and one line per row. Each `\`
/// of a value is doubled before each `|` is escaped, so no `\` of the value escapes the escape.
fn markdown_table(columns: &[&str], rows: &[Vec<Value>]) -> String {
    let markdown = |text: &str| one_line(&text.replace('\\', "\\\\").replace('|', "\\|"), "<br>");
    let line = |cells: Vec<String>| format!("| {} |\n", cells.join(" | "));
    let mut out = line(columns.iter().map(|column| markdown(column)).collect());
    out.push('|');
    out.push_str(&"---|".repeat(columns.len()));
    out.push('\n');
    for row in rows {
        out.push_str(&line(
            row.iter().map(|value| markdown(&cell(value))).collect(),
        ));
    }
    out
}

/// Accepts an RFC 3339 instant, such as `2026-10-06T17:00:00Z`, exactly as the generated
/// `Timestamp` reader reads one (`conductor_model::primitives`), and keeps it as written: the
/// generated `Timestamp` carries its wire rendering.
///
/// `time`'s RFC 3339 parser is wider than that reader in three places, each refused here: a
/// separator other than `T` or `t`, a leap second (`60`), and more than 9 fraction digits.
fn timestamp(value: &str) -> Result<String, String> {
    OffsetDateTime::parse(value, &Rfc3339)
        .map_err(|error| format!("not an RFC 3339 instant: {error}"))?;
    let bytes = value.as_bytes();
    if !matches!(bytes.get(10), Some(b'T' | b't')) {
        return Err("not an RFC 3339 instant: date and time are joined by `T`".to_owned());
    }
    if bytes.get(17..19) == Some(b"60") {
        return Err("not an RFC 3339 instant: a second is at most 59".to_owned());
    }
    if bytes.get(19) == Some(&b'.') {
        let fraction = bytes[20..]
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if fraction > 9 {
            return Err("not an RFC 3339 instant: at most 9 fraction digits".to_owned());
        }
    }
    Ok(value.to_owned())
}

/// Accepts an actor (`conductor.operator.Actor`) as one JSON object the generated decoder reads,
/// such as `{"kind": "Human", "operator": "example-operator"}`, and keeps it as written for
/// [`crate::actor::chosen`].
fn actor(value: &str) -> Result<String, String> {
    crate::actor::parse(value)
        .map(|_| value.to_owned())
        .map_err(|error| format!("not an actor: {error}"))
}

/// Accepts an exact decimal, such as `0.1`, `-10.50` or `15e-1`, exactly as the generated
/// decimal reader reads one (`conductor_model::primitives::invariant::Number`), and keeps it as
/// written: the generated `Decimal` carries its wire rendering.
fn decimal(value: &str) -> Result<String, String> {
    conductor_model::primitives::invariant::Number::parse(value)
        .map(|_| value.to_owned())
        .ok_or_else(|| "not a decimal: expected digits, a `.` and an exponent at most".to_owned())
}

/// Accepts a UUID as 8-4-4-4-12 hexadecimal digits in either case, and answers its canonical
/// lower-case rendering, which the generated `Uuid` carries.
fn uuid(value: &str) -> Result<String, String> {
    const GROUPS: [usize; 5] = [8, 4, 4, 4, 12];
    let groups: Vec<&str> = value.split('-').collect();
    let canonical = groups.len() == GROUPS.len()
        && groups.iter().zip(GROUPS).all(|(group, length)| {
            group.len() == length && group.bytes().all(|byte| byte.is_ascii_hexdigit())
        });
    if canonical {
        Ok(value.to_ascii_lowercase())
    } else {
        Err("not a UUID: expected 8-4-4-4-12 hexadecimal digits".to_owned())
    }
}

/// Accepts a value that starts with `prefix`, as its type's `prefix:` declares.
fn prefixed(
    prefix: &'static str,
) -> impl Fn(&str) -> Result<String, String> + Clone + Send + Sync + 'static {
    move |value: &str| {
        if value.starts_with(prefix) {
            Ok(value.to_owned())
        } else {
            Err(format!("must start with `{prefix}`"))
        }
    }
}

/// Accepts a JSON array, as the flag of a `List<…>` field holds its elements, one JSON object
/// each, for the handler to read.
fn json_list(value: &str) -> Result<String, String> {
    match serde_json::from_str::<Value>(value) {
        Ok(Value::Array(_)) => Ok(value.to_owned()),
        Ok(_) => Err("not a JSON array".to_owned()),
        Err(error) => Err(format!("not a JSON array: {error}")),
    }
}

/// The input of `conductor.host.Reclaim`.
#[derive(Debug, Clone, Args)]
pub struct RecordReclaimArgs {
    /// InstanceName
    #[arg(long)]
    pub instance: Option<String>,
    /// list of ReclaimStep, a JSON array
    #[arg(long, value_parser = json_list)]
    pub steps: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub reclaimed_bytes: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub shortfall_bytes: Option<i64>,
    /// list of LeftOver, a JSON array
    #[arg(long, value_parser = json_list)]
    pub left: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub ran_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.host.RecordPeak`.
#[derive(Debug, Clone, Args)]
pub struct RecordPeakArgs {
    /// GrantId
    #[arg(long)]
    pub grant_id: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub peak_bytes: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub pauses: Option<i64>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.host.ReleaseGrant`.
#[derive(Debug, Clone, Args)]
pub struct ReleaseGrantArgs {
    /// GrantId
    #[arg(long)]
    pub grant_id: Option<String>,
    /// ReleaseReason, optional
    #[arg(long, value_parser = ["Exited", "Killed", "ProcessGroupGone", "SessionStopped"])]
    pub reason: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub released_at: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub cache_hits: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub cache_misses: Option<i64>,
    /// boolean
    #[arg(long)]
    pub lane_overrun: Option<bool>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.host.RequestDisk`.
#[derive(Debug, Clone, Args)]
pub struct RequestDiskArgs {
    /// InstanceName
    #[arg(long)]
    pub instance: Option<String>,
    /// RepositoryName, optional
    #[arg(long)]
    pub repository: Option<String>,
    /// SessionName, optional
    #[arg(long)]
    pub session_name: Option<String>,
    /// SessionId, optional
    #[arg(long)]
    pub session_id: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub bytes: Option<i64>,
    /// string
    #[arg(long)]
    pub command: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub requested_at: Option<String>,
    /// BuildLane, optional
    #[arg(long, value_parser = ["Regular", "Short"])]
    pub lane: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.host.ReserveDisk`.
#[derive(Debug, Clone, Args)]
pub struct ReserveDiskArgs {
    /// GrantId
    #[arg(long)]
    pub grant_id: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub shortfall_bytes: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub reclaimed_bytes: Option<i64>,
    /// BuildLane, optional
    #[arg(long, value_parser = ["Regular", "Short"])]
    pub lane: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.host.StartRun`.
#[derive(Debug, Clone, Args)]
pub struct StartRunArgs {
    /// GrantId
    #[arg(long)]
    pub grant_id: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub process_group: Option<i64>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub started_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.host.TakeHostGrant`.
#[derive(Debug, Clone, Args)]
pub struct TakeHostGrantArgs {
    /// InstanceName
    #[arg(long)]
    pub instance: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// GrantKind
    #[arg(long, value_parser = ["ControllerSlot", "BuildSlot", "Disk"])]
    pub kind: Option<String>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub bytes: Option<i64>,
    /// SessionName
    #[arg(long)]
    pub session_name: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub requested_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.DefineMetric`.
#[derive(Debug, Clone, Args)]
pub struct DefineMetricArgs {
    /// MetricId
    #[arg(long)]
    pub metric_id: Option<String>,
    /// string
    #[arg(long)]
    pub name: Option<String>,
    /// string
    #[arg(long)]
    pub unit: Option<String>,
    /// string
    #[arg(long)]
    pub instrument: Option<String>,
    /// string
    #[arg(long)]
    pub definition: Option<String>,
    /// decimal, optional
    #[arg(long, allow_negative_numbers = true, value_parser = decimal)]
    pub baseline: Option<String>,
    /// string, optional
    #[arg(long)]
    pub baseline_period: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.RecordMetricReading`.
#[derive(Debug, Clone, Args)]
pub struct RecordMetricReadingArgs {
    /// MetricReadingId
    #[arg(long)]
    pub reading_id: Option<String>,
    /// MetricId
    #[arg(long)]
    pub metric_id: Option<String>,
    /// string
    #[arg(long)]
    pub day: Option<String>,
    /// string
    #[arg(long)]
    pub scope: Option<String>,
    /// decimal
    #[arg(long, allow_negative_numbers = true, value_parser = decimal)]
    pub value: Option<String>,
    /// list of string, a JSON array
    #[arg(long, value_parser = json_list)]
    pub sources: Option<String>,
    /// decimal, optional
    #[arg(long, allow_negative_numbers = true, value_parser = decimal)]
    pub target: Option<String>,
    /// boolean, optional
    #[arg(long)]
    pub met: Option<bool>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub recorded_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.SupersedeMetricReading`.
#[derive(Debug, Clone, Args)]
pub struct SupersedeMetricReadingArgs {
    /// MetricReadingId
    #[arg(long)]
    pub reading_id: Option<String>,
    /// MetricReadingId
    #[arg(long)]
    pub by: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.JudgeMetricReading`.
#[derive(Debug, Clone, Args)]
pub struct JudgeMetricReadingArgs {
    /// MetricReadingId
    #[arg(long)]
    pub reading_id: Option<String>,
    /// decimal
    #[arg(long, allow_negative_numbers = true, value_parser = decimal)]
    pub target: Option<String>,
    /// boolean
    #[arg(long)]
    pub met: Option<bool>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.SetMetricTarget`.
#[derive(Debug, Clone, Args)]
pub struct SetMetricTargetArgs {
    /// MetricId
    #[arg(long)]
    pub metric_id: Option<String>,
    /// decimal
    #[arg(long, allow_negative_numbers = true, value_parser = decimal)]
    pub target: Option<String>,
    /// TargetDirection
    #[arg(long, value_parser = ["AtMost", "AtLeast"])]
    pub direction: Option<String>,
    /// string
    #[arg(long)]
    pub target_owner: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub target_set_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.FileFinding`.
#[derive(Debug, Clone, Args)]
pub struct FileFindingArgs {
    /// FindingId
    #[arg(long)]
    pub finding_id: Option<String>,
    /// string
    #[arg(long)]
    pub story: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.RaiseFinding`.
#[derive(Debug, Clone, Args)]
pub struct RaiseFindingArgs {
    /// FindingId
    #[arg(long)]
    pub finding_id: Option<String>,
    /// string
    #[arg(long)]
    pub signature: Option<String>,
    /// list of string, a JSON array
    #[arg(long, value_parser = json_list)]
    pub evidence: Option<String>,
    /// MetricId
    #[arg(long)]
    pub metric_id: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub raised_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.improvement.VerifyFinding`.
#[derive(Debug, Clone, Args)]
pub struct VerifyFindingArgs {
    /// FindingId
    #[arg(long)]
    pub finding_id: Option<String>,
    /// boolean
    #[arg(long)]
    pub moved: Option<bool>,
    /// string
    #[arg(long)]
    pub verdict: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.host.WithdrawDiskRequest`.
#[derive(Debug, Clone, Args)]
pub struct WithdrawDiskRequestArgs {
    /// GrantId
    #[arg(long)]
    pub grant_id: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub withdrawn_at: Option<String>,
    /// ReleaseReason, optional
    #[arg(long, value_parser = ["Exited", "Killed", "ProcessGroupGone", "SessionStopped"])]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.StartSnapshot`.
#[derive(Debug, Clone, Args)]
pub struct StartSnapshotArgs {
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub started_at: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub disk_free_bytes: Option<i64>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub budget: Option<i64>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordRepository`.
#[derive(Debug, Clone, Args)]
pub struct RecordRepositoryArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// Visibility
    #[arg(long, value_parser = ["Public", "Private"])]
    pub visibility: Option<String>,
    /// boolean
    #[arg(long)]
    pub archived: Option<bool>,
    /// boolean
    #[arg(long)]
    pub local_checkout: Option<bool>,
    /// CommitSha
    #[arg(long)]
    pub main_head: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub main_committed_at: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub main_commits_7d: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub real_commits_7d: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub behind_main: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub dirty_files: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub worktrees: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub open_issues: Option<i64>,
    /// timestamp, RFC 3339, optional
    #[arg(long, value_parser = timestamp)]
    pub oldest_open_issue_at: Option<String>,
    /// string, optional
    #[arg(long)]
    pub latest_release: Option<String>,
    /// timestamp, RFC 3339, optional
    #[arg(long, value_parser = timestamp)]
    pub latest_release_at: Option<String>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub unreleased_commits: Option<i64>,
    /// string, optional
    #[arg(long)]
    pub planning_store_version: Option<String>,
    /// boolean, optional
    #[arg(long)]
    pub in_catalog: Option<bool>,
    /// string, optional
    #[arg(long)]
    pub gates_hooks_version: Option<String>,
    /// string, optional
    #[arg(long)]
    pub gates_version: Option<String>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub wave_width: Option<i64>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub wave_ready: Option<i64>,
    /// string, optional
    #[arg(long)]
    pub wave_reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordPullRequest`.
#[derive(Debug, Clone, Args)]
pub struct RecordPullRequestArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub number: Option<i64>,
    /// string
    #[arg(long)]
    pub title: Option<String>,
    /// boolean
    #[arg(long)]
    pub draft: Option<bool>,
    /// Mergeability
    #[arg(long, value_parser = ["Mergeable", "Conflicting", "Unknown"])]
    pub mergeability: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub failing_checks: Option<i64>,
    /// list of string, a JSON array
    #[arg(long, value_parser = json_list)]
    pub failing_check_names: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub opened_at: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub updated_at: Option<String>,
    /// string, optional
    #[arg(long)]
    pub url: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordRelease`.
#[derive(Debug, Clone, Args)]
pub struct RecordReleaseArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// string
    #[arg(long)]
    pub tag: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub published_at: Option<String>,
    /// string
    #[arg(long)]
    pub url: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordMergedPullRequest`.
#[derive(Debug, Clone, Args)]
pub struct RecordMergedPullRequestArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub number: Option<i64>,
    /// string
    #[arg(long)]
    pub title: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub merged_at: Option<String>,
    /// string
    #[arg(long)]
    pub url: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordWorkflowRun`.
#[derive(Debug, Clone, Args)]
pub struct RecordWorkflowRunArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// string
    #[arg(long)]
    pub workflow: Option<String>,
    /// RunConclusion
    #[arg(long, value_parser = ["Success", "Failure", "Cancelled", "Skipped", "Neutral", "Pending"])]
    pub conclusion: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub run_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordSecretSync`.
#[derive(Debug, Clone, Args)]
pub struct RecordSecretSyncArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// string
    #[arg(long)]
    pub sync: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// string
    #[arg(long)]
    pub branch: Option<String>,
    /// string
    #[arg(long)]
    pub path: Option<String>,
    /// timestamp, RFC 3339, optional
    #[arg(long, value_parser = timestamp)]
    pub source_committed_at: Option<String>,
    /// timestamp, RFC 3339, optional
    #[arg(long, value_parser = timestamp)]
    pub secrets_updated_at: Option<String>,
    /// string, optional
    #[arg(long)]
    pub error_line: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordSession`.
#[derive(Debug, Clone, Args)]
pub struct RecordSessionArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// Harness
    #[arg(long, value_parser = ["Claude", "Codex"])]
    pub harness: Option<String>,
    /// string
    #[arg(long)]
    pub session_ref: Option<String>,
    /// string, optional
    #[arg(long)]
    pub name: Option<String>,
    /// string
    #[arg(long)]
    pub cwd: Option<String>,
    /// RepositoryName, optional
    #[arg(long)]
    pub repository: Option<String>,
    /// string, optional
    #[arg(long)]
    pub role: Option<String>,
    /// SessionState
    #[arg(long, value_parser = ["Busy", "Idle", "Waiting", "Background", "Unknown"])]
    pub activity: Option<String>,
    /// EnvelopeProvider, optional
    #[arg(long, value_parser = ["Systemd"])]
    pub envelope_provider: Option<String>,
    /// string, optional
    #[arg(long)]
    pub envelope_cgroup: Option<String>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub envelope_memory_max: Option<i64>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub envelope_cpu_weight: Option<i64>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub envelope_tasks_max: Option<i64>,
    /// string, optional
    #[arg(long)]
    pub envelope_writable: Option<String>,
    /// string, optional
    #[arg(long)]
    pub envelope_unboxed: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordBlocker`.
#[derive(Debug, Clone, Args)]
pub struct RecordBlockerArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// string
    #[arg(long)]
    pub reference: Option<String>,
    /// string
    #[arg(long)]
    pub blocker_kind: Option<String>,
    /// string
    #[arg(long)]
    pub title: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordCollectorRun`.
#[derive(Debug, Clone, Args)]
pub struct RecordCollectorRunArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// string
    #[arg(long)]
    pub collector: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub started_at: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub duration_ms: Option<i64>,
    /// CollectorRunOutcome
    #[arg(long, value_parser = ["Completed", "Failed", "OverBudget"])]
    pub outcome: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub read_bytes: Option<i64>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub written_bytes: Option<i64>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordUsage`.
#[derive(Debug, Clone, Args)]
pub struct RecordUsageArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub read_at: Option<String>,
    /// string
    #[arg(long)]
    pub account: Option<String>,
    /// boolean
    #[arg(long)]
    pub active: Option<bool>,
    /// string
    #[arg(long)]
    pub health: Option<String>,
    /// string
    #[arg(long)]
    pub window: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub used_percent: Option<i64>,
    /// timestamp, RFC 3339, optional
    #[arg(long, value_parser = timestamp)]
    pub resets_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordMemberFailure`.
#[derive(Debug, Clone, Args)]
pub struct RecordMemberFailureArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub member: Option<String>,
    /// string
    #[arg(long)]
    pub collector: Option<String>,
    /// string
    #[arg(long)]
    pub error_line: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.RecordSpecification`.
#[derive(Debug, Clone, Args)]
pub struct RecordSpecificationArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// SpecificationPresence
    #[arg(long, value_parser = ["Present", "OptedOut", "Missing", "NotInWorkspace"])]
    pub presence: Option<String>,
    /// string, optional
    #[arg(long)]
    pub path: Option<String>,
    /// string, optional
    #[arg(long)]
    pub format: Option<String>,
    /// string, optional
    #[arg(long)]
    pub required_ess: Option<String>,
    /// ValidationResult
    #[arg(long, value_parser = ["Valid", "Refused", "NotRun"])]
    pub validation: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub validation_refusals: Option<i64>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub scenarios: Option<i64>,
    /// integer, optional
    #[arg(long, allow_negative_numbers = true)]
    pub synthesis_refusals: Option<i64>,
    /// string, optional
    #[arg(long)]
    pub conformance_status: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.PublishBoard`.
#[derive(Debug, Clone, Args)]
pub struct PublishBoardArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// SnapshotId, optional
    #[arg(long, value_parser = uuid)]
    pub previous_snapshot_id: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub deviation_rows: Option<i64>,
    /// string
    #[arg(long)]
    pub path: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.CompleteSnapshot`.
#[derive(Debug, Clone, Args)]
pub struct CompleteSnapshotArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.observation.FailSnapshot`.
#[derive(Debug, Clone, Args)]
pub struct FailSnapshotArgs {
    /// SnapshotId
    #[arg(long, value_parser = uuid)]
    pub snapshot_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.ProposeGoal`.
#[derive(Debug, Clone, Args)]
pub struct ProposeGoalArgs {
    /// GoalId
    #[arg(long)]
    pub goal_id: Option<String>,
    /// string
    #[arg(long)]
    pub title: Option<String>,
    /// string
    #[arg(long)]
    pub exit_evidence: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.ConfirmGoal`.
#[derive(Debug, Clone, Args)]
pub struct ConfirmGoalArgs {
    /// GoalId
    #[arg(long)]
    pub goal_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.MarkGoalMet`.
#[derive(Debug, Clone, Args)]
pub struct MarkGoalMetArgs {
    /// GoalId
    #[arg(long)]
    pub goal_id: Option<String>,
    /// string
    #[arg(long)]
    pub evidence: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.DropGoal`.
#[derive(Debug, Clone, Args)]
pub struct DropGoalArgs {
    /// GoalId
    #[arg(long)]
    pub goal_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.AddServing`.
#[derive(Debug, Clone, Args)]
pub struct AddServingArgs {
    /// GoalId
    #[arg(long)]
    pub goal_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.RemoveServing`.
#[derive(Debug, Clone, Args)]
pub struct RemoveServingArgs {
    /// ServingId
    #[arg(long, value_parser = uuid)]
    pub serving_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.MarkRepository`.
#[derive(Debug, Clone, Args)]
pub struct MarkRepositoryArgs {
    /// MarkedRepository
    #[arg(long)]
    pub repository: Option<String>,
    /// Activity
    #[arg(long, value_parser = ["Active", "Inactive"])]
    pub activity: Option<String>,
    /// MarkedBy
    #[arg(long, value_parser = ["Conductor", "Operator", "Controller"])]
    pub marked_by: Option<String>,
    /// string
    #[arg(long)]
    pub session: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.ActivateRepository`.
#[derive(Debug, Clone, Args)]
pub struct ActivateRepositoryArgs {
    /// MarkedRepository
    #[arg(long)]
    pub repository: Option<String>,
    /// MarkedBy
    #[arg(long, value_parser = ["Conductor", "Operator", "Controller"])]
    pub marked_by: Option<String>,
    /// string
    #[arg(long)]
    pub session: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.direction.DeactivateRepository`.
#[derive(Debug, Clone, Args)]
pub struct DeactivateRepositoryArgs {
    /// MarkedRepository
    #[arg(long)]
    pub repository: Option<String>,
    /// MarkedBy
    #[arg(long, value_parser = ["Conductor", "Operator", "Controller"])]
    pub marked_by: Option<String>,
    /// string
    #[arg(long)]
    pub session: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.RaiseDecisionRequest`.
#[derive(Debug, Clone, Args)]
pub struct RaiseDecisionRequestArgs {
    /// RequestId
    #[arg(long)]
    pub request_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// string
    #[arg(long)]
    pub question: Option<String>,
    /// string
    #[arg(long)]
    pub options: Option<String>,
    /// string
    #[arg(long)]
    pub recommendation: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub raised_at: Option<String>,
    /// string, optional
    #[arg(long)]
    pub blocker: Option<String>,
    /// ClassificationRule, optional: the rule that classified the request in code
    #[arg(long, value_parser = [
        "data-loss",
        "history-rewrite",
        "repository-lifecycle",
        "architecture",
        "credential",
        "hands",
        "none",
    ])]
    pub classification_rule: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.RecordConductorDecision`.
#[derive(Debug, Clone, Args)]
pub struct RecordConductorDecisionArgs {
    /// DecisionId, starting with `DEC-`
    #[arg(long, value_parser = prefixed("DEC-"))]
    pub decision_id: Option<String>,
    /// DecisionClass
    #[arg(long, value_parser = ["C", "O", "H"])]
    pub decision_class: Option<String>,
    /// string
    #[arg(long)]
    pub question: Option<String>,
    /// string
    #[arg(long)]
    pub options: Option<String>,
    /// string
    #[arg(long)]
    pub choice: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// string
    #[arg(long)]
    pub evidence: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub decided_at: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// string, optional: the `Decider` adapter that answered
    #[arg(long)]
    pub decider_name: Option<String>,
    /// PromptHash, optional: the SHA-256 of the adapter's prompt, 64 lower-case hexadecimal
    /// characters
    #[arg(long)]
    pub prompt_sha256: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.RecordOperatorDecision`.
#[derive(Debug, Clone, Args)]
pub struct RecordOperatorDecisionArgs {
    /// DecisionId, starting with `DEC-`
    #[arg(long, value_parser = prefixed("DEC-"))]
    pub decision_id: Option<String>,
    /// DecisionClass
    #[arg(long, value_parser = ["C", "O", "H"])]
    pub decision_class: Option<String>,
    /// string
    #[arg(long)]
    pub question: Option<String>,
    /// string
    #[arg(long)]
    pub options: Option<String>,
    /// string
    #[arg(long)]
    pub choice: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// string
    #[arg(long)]
    pub evidence: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub decided_at: Option<String>,
    /// string
    #[arg(long)]
    pub recorded_by: Option<String>,
    /// string
    #[arg(long)]
    pub operator_words: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.RecordOperatorDecisionForRequest` (`story:decisions-batch-answer`):
/// the operator's answer to an escalated request. `--request-id`, `--choice` (the letters, joined
/// by `,`; `done` for class H), `--recorded-by` and `--operator-words` are required; the rest
/// default to the request's, or now, or are allocated.
#[derive(Debug, Clone, Args)]
pub struct RecordOperatorDecisionForRequestArgs {
    /// RequestId
    #[arg(long)]
    pub request_id: Option<String>,
    /// DecisionId, starting with `DEC-`
    #[arg(long, value_parser = prefixed("DEC-"))]
    pub decision_id: Option<String>,
    /// DecisionClass
    #[arg(long, value_parser = ["C", "O", "H"])]
    pub decision_class: Option<String>,
    /// string
    #[arg(long)]
    pub question: Option<String>,
    /// string
    #[arg(long)]
    pub options: Option<String>,
    /// string
    #[arg(long)]
    pub choice: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// string
    #[arg(long)]
    pub evidence: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub decided_at: Option<String>,
    /// string
    #[arg(long)]
    pub recorded_by: Option<String>,
    /// string
    #[arg(long)]
    pub operator_words: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.AnswerRequest`.
#[derive(Debug, Clone, Args)]
pub struct AnswerRequestArgs {
    /// RequestId
    #[arg(long)]
    pub request_id: Option<String>,
    /// DecisionId, starting with `DEC-`
    #[arg(long, value_parser = prefixed("DEC-"))]
    pub decision_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.AnswerEscalatedRequest`.
#[derive(Debug, Clone, Args)]
pub struct AnswerEscalatedRequestArgs {
    /// RequestId
    #[arg(long)]
    pub request_id: Option<String>,
    /// DecisionId, starting with `DEC-`
    #[arg(long, value_parser = prefixed("DEC-"))]
    pub decision_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.EscalateRequest`.
#[derive(Debug, Clone, Args)]
pub struct EscalateRequestArgs {
    /// RequestId
    #[arg(long)]
    pub request_id: Option<String>,
    /// DecisionClass
    #[arg(long, value_parser = ["C", "O", "H"])]
    pub escalation_class: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// The file holding the dry run: what every step of the operator's script that needs no
    /// credential printed. A class-H escalation carries this or `--no-dry-run`
    #[arg(long, value_name = "FILE")]
    pub dry_run: Option<String>,
    /// Why the operator's script has no dry run. A class-H escalation carries this or `--dry-run`
    #[arg(long, value_name = "REASON")]
    pub no_dry_run: Option<String>,
    /// RequestKind: how the operator answers. Required for class O.
    #[arg(long, value_parser = ["yes-no", "one-of", "several-of"])]
    pub kind: Option<String>,
    /// list of RequestOption, a JSON array of `{letter, label, cost}`; `Y` and `N` for `yes-no`.
    /// Required for class O
    #[arg(long, value_parser = json_list)]
    pub lettered_options: Option<String>,
    /// The letter conductor recommends, or several joined by `,` for `several-of`. Required for
    /// class O
    #[arg(long, value_name = "LETTERS")]
    pub recommended_letters: Option<String>,
    /// The letter that stands when the operator has not answered by the deadline, or several
    /// joined by `,` for `several-of`. Required for class O
    #[arg(long, value_name = "LETTERS")]
    pub default_letters: Option<String>,
    /// timestamp, RFC 3339: when the default stands. Required for class O
    #[arg(long, value_parser = timestamp)]
    pub deadline: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.AmendEscalatedRequest` (`story:request-amend-escalated`). A
/// field given replaces the held one; a field left out stays.
#[derive(Debug, Clone, Args)]
pub struct AmendEscalatedRequestArgs {
    /// RequestId of the escalated request
    #[arg(long)]
    pub request_id: Option<String>,
    /// Why the request is completed
    #[arg(long)]
    pub reason: Option<String>,
    /// RequestKind: how the operator answers
    #[arg(long, value_parser = ["yes-no", "one-of", "several-of"])]
    pub kind: Option<String>,
    /// list of RequestOption, a JSON array of `{letter, label, cost}`; replaces the held options
    #[arg(long, value_parser = json_list)]
    pub lettered_options: Option<String>,
    /// The letter conductor recommends, or several joined by `,` for `several-of`
    #[arg(long, value_name = "LETTERS")]
    pub recommended_letters: Option<String>,
    /// The letter that stands when the operator has not answered by the deadline, or several
    /// joined by `,` for `several-of`
    #[arg(long, value_name = "LETTERS")]
    pub default_letters: Option<String>,
    /// timestamp, RFC 3339: when the default stands
    #[arg(long, value_parser = timestamp)]
    pub deadline: Option<String>,
    /// The dry run of a class-H item, as text (not a file, unlike `escalate-request`)
    #[arg(long, value_name = "TEXT")]
    pub dry_run: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.WithdrawRequest`.
#[derive(Debug, Clone, Args)]
pub struct WithdrawRequestArgs {
    /// RequestId
    #[arg(long)]
    pub request_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.ReverseConductorDecision`.
#[derive(Debug, Clone, Args)]
pub struct ReverseConductorDecisionArgs {
    /// DecisionId, starting with `DEC-`
    #[arg(long, value_parser = prefixed("DEC-"))]
    pub decision_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.decision.ReverseOperatorDecision`.
#[derive(Debug, Clone, Args)]
pub struct ReverseOperatorDecisionArgs {
    /// DecisionId, starting with `DEC-`
    #[arg(long, value_parser = prefixed("DEC-"))]
    pub decision_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.StartController`.
#[derive(Debug, Clone, Args)]
pub struct StartControllerArgs {
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// Harness
    #[arg(long, value_parser = ["Claude", "Codex"])]
    pub harness: Option<String>,
    /// SessionName
    #[arg(long)]
    pub session_name: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.PauseController`.
#[derive(Debug, Clone, Args)]
pub struct PauseControllerArgs {
    /// ControllerId
    #[arg(long, value_parser = uuid)]
    pub controller_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ResumeController`.
#[derive(Debug, Clone, Args)]
pub struct ResumeControllerArgs {
    /// ControllerId
    #[arg(long, value_parser = uuid)]
    pub controller_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.StopController`.
#[derive(Debug, Clone, Args)]
pub struct StopControllerArgs {
    /// ControllerId
    #[arg(long, value_parser = uuid)]
    pub controller_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReviseCharter`.
#[derive(Debug, Clone, Args)]
pub struct ReviseCharterArgs {
    /// ControllerId
    #[arg(long, value_parser = uuid)]
    pub controller_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.SendDispatch`.
#[derive(Debug, Clone, Args)]
pub struct SendDispatchArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// GoalId, optional
    #[arg(long)]
    pub goal_id: Option<String>,
    /// string
    #[arg(long)]
    pub brief: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub sent_at: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub priority: Option<i64>,
    /// DispatchId, starting with `DSP-`, optional
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub waits_on: Option<String>,
    /// string, optional
    #[arg(long)]
    pub reference: Option<String>,
    /// string, optional
    #[arg(long)]
    pub start_ref: Option<String>,
    /// string, optional
    #[arg(long)]
    pub issue: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReportStarted`.
#[derive(Debug, Clone, Args)]
pub struct ReportStartedArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReportBlocked`.
#[derive(Debug, Clone, Args)]
pub struct ReportBlockedArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReportUnblocked`.
#[derive(Debug, Clone, Args)]
pub struct ReportUnblockedArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReportDone`.
#[derive(Debug, Clone, Args)]
pub struct ReportDoneArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// string
    #[arg(long)]
    pub evidence: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReportFailed`.
#[derive(Debug, Clone, Args)]
pub struct ReportFailedArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.CancelDispatch`.
#[derive(Debug, Clone, Args)]
pub struct CancelDispatchArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.RecordDispatchLink`.
#[derive(Debug, Clone, Args)]
pub struct RecordDispatchLinkArgs {
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// string
    #[arg(long)]
    pub link: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReceiveMessage`.
#[derive(Debug, Clone, Args)]
pub struct ReceiveMessageArgs {
    /// string
    #[arg(long)]
    pub sender: Option<String>,
    /// string
    #[arg(long)]
    pub recipient: Option<String>,
    /// MessageKind
    #[arg(long, value_parser = ["Dispatch", "Report", "DecisionRequest", "Decision", "Need", "Resource", "Escalation", "Team", "Handover", "Unknown"])]
    pub kind: Option<String>,
    /// string
    #[arg(long)]
    pub first_line: Option<String>,
    /// Transport
    #[arg(long, value_parser = ["SendMessage", "Mailbox"])]
    pub transport: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub received_at: Option<String>,
    /// string, optional
    #[arg(long)]
    pub body_path: Option<String>,
    /// boolean
    #[arg(long)]
    pub parsed: Option<bool>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.HandleMessage`.
#[derive(Debug, Clone, Args)]
pub struct HandleMessageArgs {
    /// MessageId
    #[arg(long, value_parser = uuid)]
    pub message_id: Option<String>,
    /// DispatchId, starting with `DSP-`, optional
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.RejectMessage`.
#[derive(Debug, Clone, Args)]
pub struct RejectMessageArgs {
    /// MessageId
    #[arg(long, value_parser = uuid)]
    pub message_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.RouteNeed`.
#[derive(Debug, Clone, Args)]
pub struct RouteNeedArgs {
    /// MessageId
    #[arg(long, value_parser = uuid)]
    pub message_id: Option<String>,
    /// DispatchId, starting with `DSP-`
    #[arg(long, value_parser = prefixed("DSP-"))]
    pub dispatch_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.RequestResource`.
#[derive(Debug, Clone, Args)]
pub struct RequestResourceArgs {
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// ResourceKind
    #[arg(long, value_parser = ["BuildSlot", "Disk", "Usage"])]
    pub resource: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub amount: Option<i64>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub requested_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.GrantResource`.
#[derive(Debug, Clone, Args)]
pub struct GrantResourceArgs {
    /// ResourceRequestId
    #[arg(long, value_parser = uuid)]
    pub resource_request_id: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub disk_free_bytes: Option<i64>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.RefuseResource`.
#[derive(Debug, Clone, Args)]
pub struct RefuseResourceArgs {
    /// ResourceRequestId
    #[arg(long, value_parser = uuid)]
    pub resource_request_id: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.ReleaseResource`.
#[derive(Debug, Clone, Args)]
pub struct ReleaseResourceArgs {
    /// ResourceRequestId
    #[arg(long, value_parser = uuid)]
    pub resource_request_id: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of `conductor.dispatch.RecordGuardDecision`.
#[derive(Debug, Clone, Args)]
pub struct RecordGuardDecisionArgs {
    /// RepositoryName
    #[arg(long)]
    pub repository: Option<String>,
    /// string
    #[arg(long)]
    pub session_ref: Option<String>,
    /// string
    #[arg(long)]
    pub tool: Option<String>,
    /// string
    #[arg(long)]
    pub target: Option<String>,
    /// Verdict
    #[arg(long, value_parser = ["Allow", "Deny"])]
    pub verdict: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub decided_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
    /// Read a Claude Code PreToolUse payload from standard input and compute the verdict
    #[arg(long, conflicts_with = "input_json")]
    pub from_pre_tool_use: bool,
    /// The calling session's process, which the PreToolUse payload does not name (the hook
    /// shell's parent, `--pid "$PPID"`); the conductor lease denies a write from another process
    #[arg(long, allow_negative_numbers = true)]
    pub pid: Option<i64>,
}

/// The input of `conductor.session.RecordRunTick` (`story:run-daemon`).
#[derive(Debug, Clone, Args)]
pub struct RecordRunTickArgs {
    /// TickKind
    #[arg(long, value_parser = ["watch", "cycle", "daily"])]
    pub kind: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub started_at: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub lines_handled: Option<i64>,
    /// list of DispatchLogEvent, a JSON array of the events as the dispatch log writes them
    /// (`relayed`, `handover-sent`, …)
    #[arg(long, value_parser = json_list)]
    pub actions: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
}

/// The input of the binding's `run` callable, `conductor.session.RunDaemon` (`story:run-daemon`).
#[derive(Debug, Clone, Default, Args)]
pub struct RunArgs {
    /// Run one watch tick and one cycle tick, then exit 0. `--once` alone is `--once true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub once: Option<bool>,
    /// With --once: commit the records directory through the bot route and push it when it has
    /// an origin, and run no tick. `--commit-records` alone is `--commit-records true`
    #[arg(long, value_name = "BOOL", num_args = 0..=1, default_missing_value = "true")]
    pub commit_records: Option<bool>,
}

/// The input of `conductor.session.ClaimLease`.
#[derive(Debug, Clone, Args)]
pub struct ClaimLeaseArgs {
    /// InstanceName
    #[arg(long)]
    pub instance: Option<String>,
    /// SessionId
    #[arg(long)]
    pub session_id: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub pid: Option<i64>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub taken_at: Option<String>,
    /// Actor, one JSON object: who issues the command, `{"kind": "Human", "operator": "<name>"}`,
    /// `{"kind": "Agent", "agent": {"instance", "role", "name", "kind"}}` or `{"kind": "Daemon"}`;
    /// default: the calling session as an agent where `CONDUCTOR_SESSION` names one, else the
    /// operator
    #[arg(long, value_parser = actor)]
    pub actor: Option<String>,
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
    /// Read the session id from a Claude Code SessionStart payload on standard input
    #[arg(long, conflicts_with_all = ["input_json", "session_id"])]
    pub from_session_start: bool,
}
