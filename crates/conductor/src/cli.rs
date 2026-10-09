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
//! `guard record-guard-decision` alone also takes `--from-pre-tool-use`.
//!
//! No flag is required on its own, because every input field may arrive through
//! `--input-json -` instead: the handler decides what is missing.
//!
//! Beside the `cli:` block, the tree presents each command the binding declares over a `local`
//! callable, with exactly the option flags the binding maps its input fields to: `dashboard
//! serve` (`story:live-dashboard`), `decision show`, `watch run` (`story:watch-command`),
//! `repository shipped` (`story:release-digest`), `repository activity`
//! (`story:repository-marks`), `resource usage` (`story:token-spend`) and `store migrate`
//! (`story:store-on-eventlog-tree`).
//!
//! `config validate` and `config show` (`story:config-file`) are such commands too, and the only
//! ones that read the global `--config`, named by `globals.config` of the binding, which is taken
//! before or after any subcommand as `--state-dir` is.
//!
//! Three such commands are one word, with no group: `trust` (`story:portable-trust`) and `init`
//! (`story:first-run-seed`), which read the config file themselves as `config show` does, and
//! `doctor` (`story:install-prerequisites`), which reads none.
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
    controller, dashboard, decide, dispatch, goal, guard, message, repository, resource, shipped,
    snapshot, status,
};

/// The records conductor and the repository controllers keep.
#[derive(Debug, Parser)]
#[command(name = "conductor", disable_help_subcommand = true)]
pub struct Cli {
    /// The state directory; conductor's records are kept in its tree/. Default: the instance's
    /// state when a config file names it, else state/ under the working directory
    #[arg(long, global = true, value_name = "DIR")]
    pub state_dir: Option<PathBuf>,

    /// The config file, named by `globals.config` of the binding. Default: $CONDUCTOR_CONFIG,
    /// else ~/.b10x/conductor/conductor.yaml. Every command reads it for its instance
    #[arg(long, global = true, value_name = "FILE")]
    pub config: Option<PathBuf>,

    /// The group the command belongs to.
    #[command(subcommand)]
    pub group: Group,
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
        // The instance every handler reads (`crate::config::active`). The two config leaves read
        // the file themselves, to report each problem; the guard hook falls back to the built-in
        // instance rather than deny every tool call over a file that does not load.
        match &self.group {
            Group::Config(_) | Group::Trust(_) | Group::Init(_) | Group::Doctor => {}
            Group::Guard(_) => {
                let _ = crate::config::set_active(self.config.as_deref());
            }
            _ => crate::config::set_active(self.config.as_deref())?,
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
                SnapshotCommand::PublishBoard(args) => status::publish_board(args),
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
                SnapshotCommand::Boards(view) => status::boards(view),
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
                DecisionCommand::AnswerRequest(args) => decide::answer_request(state, args),
                DecisionCommand::AnswerEscalatedRequest(args) => {
                    decide::answer_escalated_request(state, args)
                }
                DecisionCommand::EscalateRequest(args) => decide::escalate_request(state, args),
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
            },
            Group::Controller(command) => match command {
                ControllerCommand::StartController(args) => {
                    controller::start_controller(state, args)
                }
                ControllerCommand::PauseController(args) => {
                    controller::pause_controller(state, args)
                }
                ControllerCommand::ResumeController(args) => {
                    controller::resume_controller(state, args)
                }
                ControllerCommand::StopController(args) => controller::stop_controller(state, args),
                ControllerCommand::ReviseCharter(args) => controller::revise_charter(state, args),
                ControllerCommand::Controllers(view) => controller::controllers(state, view),
            },
            Group::Dispatch(command) => match command {
                DispatchCommand::SendDispatch(args) => dispatch::send_dispatch(state, args),
                DispatchCommand::ReportStarted(args) => dispatch::report_started(state, args),
                DispatchCommand::ReportBlocked(args) => dispatch::report_blocked(state, args),
                DispatchCommand::ReportUnblocked(args) => dispatch::report_unblocked(state, args),
                DispatchCommand::ReportDone(args) => dispatch::report_done(state, args),
                DispatchCommand::ReportFailed(args) => dispatch::report_failed(state, args),
                DispatchCommand::CancelDispatch(args) => dispatch::cancel_dispatch(state, args),
                DispatchCommand::Dispatches(view) => dispatch::dispatches(state, view),
            },
            Group::Message(command) => match command {
                MessageCommand::ReceiveMessage(args) => message::receive_message(state, args),
                MessageCommand::HandleMessage(args) => message::handle_message(state, args),
                MessageCommand::RejectMessage(args) => message::reject_message(state, args),
                MessageCommand::RouteNeed(args) => message::route_need(state, args),
                MessageCommand::Messages(view) => message::messages(state, view),
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
                    guard::record_guard_decision(state, args)
                }
                GuardCommand::GuardDecisions(view) => guard::guard_decisions(state, view),
            },
            Group::Config(command) => match command {
                ConfigCommand::Show(args) => crate::config::show(self.config.as_deref(), &args),
                ConfigCommand::Validate(args) => {
                    crate::config::validate(self.config.as_deref(), &args)
                }
            },
            Group::Dashboard(command) => match command {
                DashboardCommand::Serve(args) => dashboard::dashboard_serve(&args),
            },
            Group::Doctor => crate::doctor::doctor(),
            Group::Init(args) => crate::init::init(self.config.as_deref(), &args),
            Group::Trust(args) => crate::trust::trust(self.config.as_deref(), &args),
            Group::Store(command) => match command {
                StoreCommand::Migrate(args) => crate::store::migrate::store_migrate(state, &args),
            },
            Group::Watch(command) => match command {
                WatchCommand::Run(args) => crate::watch::watch_run(state, &args),
            },
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

    // No help, as for `dashboard`: the binding declares the group only through `config validate`
    // and `config show` (`story:config-file`).
    #[command(subcommand)]
    Config(ConfigCommand),

    // No help: the binding declares this group only through its command, and no group summary.
    #[command(subcommand)]
    Dashboard(DashboardCommand),

    /// Report each program conductor starts as present, with its version, or missing, and each one older than its b10x.toml pin
    #[command(name = "doctor")]
    Doctor,

    /// Write and commit the instance's records directory skeleton; refuse to overwrite any file
    #[command(name = "init")]
    Init(InitArgs),

    // No help, as for `dashboard`: the binding declares the group only through `store migrate`.
    #[command(subcommand)]
    Store(StoreCommand),

    /// Mark every git checkout under the instance's checkouts root, and its records directory, as trusted Claude Code workspaces in ~/.claude.json
    #[command(name = "trust")]
    Trust(TrustArgs),

    // No help, as for `dashboard`: the binding declares the group only through `watch run`.
    #[command(subcommand)]
    Watch(WatchCommand),
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

    /// Record a session
    #[command(name = "record-session")]
    RecordSession(RecordSessionArgs),

    /// Record a specification status
    #[command(name = "record-specification")]
    RecordSpecification(RecordSpecificationArgs),

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

    /// Sessions
    #[command(name = "sessions")]
    Sessions(ViewArgs),

    /// Snapshots
    #[command(name = "snapshots")]
    Snapshots(ViewArgs),

    /// Specifications
    #[command(name = "specifications")]
    Specifications(ViewArgs),

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

    /// What shipped since an instant, from the newest complete snapshot's GitHub observations
    #[command(name = "shipped")]
    Shipped(RepositoryShippedArgs),
}

/// `conductor decision`: decision requests and the decisions that answer them.
#[derive(Debug, Subcommand)]
pub enum DecisionCommand {
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

    /// Show one decision
    #[command(name = "show")]
    Show(ShowDecisionArgs),
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

    /// Controllers
    #[command(name = "controllers")]
    Controllers(ViewArgs),
}

/// `conductor dispatch`: briefs to controllers and their reports.
#[derive(Debug, Subcommand)]
pub enum DispatchCommand {
    /// Cancel a dispatch
    #[command(name = "cancel-dispatch")]
    CancelDispatch(CancelDispatchArgs),

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
    RecordGuardDecision(RecordGuardDecisionArgs),

    /// Guard decisions
    #[command(name = "guard-decisions")]
    GuardDecisions(ViewArgs),
}

// `conductor dashboard`: the commands the binding declares over the `local` callable
// `dashboard-serve`. A plain comment, since clap would make a doc comment the group's help, and
// the binding declares none.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum DashboardCommand {
    /// Serve a live, read-only page of what every session works on
    #[command(name = "serve")]
    Serve(DashboardServeArgs),
}

/// The input of the binding's `dashboard-serve` callable, `conductor.observation.DashboardServe`.
#[derive(Debug, Clone, Args)]
pub struct DashboardServeArgs {
    /// The port on 127.0.0.1; 0 picks a free one. Default: 7313
    #[arg(long)]
    pub port: Option<u16>,
    /// The conductor repository, whose dispatches/ and decisions/ are read. Default: the
    /// instance's records when a config file names it, else the nearest directory holding both,
    /// from the working directory up
    #[arg(long, value_name = "DIR")]
    pub root: Option<PathBuf>,
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

// `conductor store`: the command the binding declares over the `local` callable `store-migrate`
// (`story:store-on-eventlog-tree`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum StoreCommand {
    /// Copy the eventlog-file store under the state directory into a tree store, keep the old one beside it, and print the counts moved
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
// `config-validate` and `config-show` (`story:config-file`). A plain comment, as for `dashboard`.
#[allow(missing_docs)]
#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// Print one instance's effective config, after defaults, as a config file
    #[command(name = "show")]
    Show(ShowConfigArgs),

    /// Check the config file; exit 1 naming each problem by its YAML path
    #[command(name = "validate")]
    Validate(ValidateConfigArgs),
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

/// The input of the binding's `config-show` callable, `conductor.config.ShowConfig`. The format
/// is text the handler reads, so a value it refuses is named there.
#[derive(Debug, Clone, Args)]
pub struct ShowConfigArgs {
    /// How the config is rendered: text, one `path: value` line each; json; or yaml, a config
    /// file. Default: text
    #[arg(long)]
    pub format: Option<String>,
    /// The instance shown. Default: $CONDUCTOR_INSTANCE, else the config's only instance
    #[arg(long, value_name = "NAME")]
    pub instance: Option<String>,
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
    /// compact JSON, and a line break inside a value is written as a space (`text`) or `<br>`
    /// (`markdown`), so each row stays one line. `markdown` also escapes `\` and `|`, so each
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

/// `text` with every line break written as `line_break`.
fn one_line(text: &str, line_break: &str) -> String {
    text.replace("\r\n", "\n").replace(['\r', '\n'], line_break)
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

/// The input of `conductor.observation.StartSnapshot`.
#[derive(Debug, Clone, Args)]
pub struct StartSnapshotArgs {
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub started_at: Option<String>,
    /// integer
    #[arg(long, allow_negative_numbers = true)]
    pub disk_free_bytes: Option<i64>,
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
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub opened_at: Option<String>,
    /// timestamp, RFC 3339
    #[arg(long, value_parser = timestamp)]
    pub updated_at: Option<String>,
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
    #[arg(long, value_parser = ["Present", "OptedOut", "Missing"])]
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
    #[arg(long, value_parser = ["Conductor", "Operator"])]
    pub marked_by: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
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
    #[arg(long, value_parser = ["Conductor", "Operator"])]
    pub marked_by: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
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
    #[arg(long, value_parser = ["Conductor", "Operator"])]
    pub marked_by: Option<String>,
    /// string
    #[arg(long)]
    pub reason: Option<String>,
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
    /// GoalId
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
    #[arg(long, value_parser = ["Dispatch", "Report", "DecisionRequest", "Decision", "Need", "Resource", "Escalation", "Unknown"])]
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
    /// Read the input fields from standard input, as one JSON object
    #[arg(long, value_enum)]
    pub input_json: Option<JsonInput>,
    /// Read a Claude Code PreToolUse payload from standard input and compute the verdict
    #[arg(long, conflicts_with = "input_json")]
    pub from_pre_tool_use: bool,
}
