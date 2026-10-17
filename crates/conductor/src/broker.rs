//! `conductor broker serve` (`story:tool-broker-read`, design docs/design/roles-and-surfaces.md
//! § 6): runs the instance's broker ([`conductor_broker::Broker`]) over `<state>/broker` until it
//! is stopped, recording every call in the instance's store through the generated
//! `conductor.tool.RecordToolCall`; and the registration of a session's grant that `conductor
//! spawn` and `conductor session start` make before they start a session ([`register_session`]),
//! at its place among the instance's sessions ([`placement`]).
//!
//! The broker's build, commit and team verbs (`story:tool-build-commit-report`) reach the
//! instance through [`services`]: the host ledger through `build run`'s own path, with
//! `--reserve auto` and the measured peak it reserves; the bot route with the Gates policy that
//! [`POLICY_VARIABLE`] names in `broker serve`'s own environment; and the store, where `send` and
//! `report` record a message as `message receive-message` does and a lead's report moves its open
//! dispatch as `dispatch report-*` does.

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration as Wait;

use anyhow::{Context as _, Result, anyhow};
use conductor_broker::{
    BotRoute, Broker, BuildEnd, BuildJob, CallRecord, Ledger, Level, Messages, Placement, Recorder,
    Registration, Report, ReportStatus, Reported, Services, SessionGrant, TeamMessage, broker_dir,
};
use conductor_model::behaviour::{
    DispatchStorage, Generated, ToolCallStorage, TryContext, unmet_context,
};
use conductor_model::config::{Instance, InstanceName, Role, SessionName};
use conductor_model::dispatch::{
    DispatchState, MessageKind, ReceiveMessage, ReceiveMessageOutcome, Transport,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::tool::obligations::RecordToolCallBehavior;
use conductor_model::tool::{RecordToolCall, RecordToolCallOutcome, ToolCallId, ToolCallSnapshot};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::build;
use crate::cli::{
    ReportBlockedArgs, ReportDoneArgs, ReportFailedArgs, ReportStartedArgs, ReportUnblockedArgs,
};
use crate::store::Store;

/// The signals that stop `broker serve`.
const STOPPING: [i32; 3] = [
    nix::sys::signal::Signal::SIGTERM as i32,
    nix::sys::signal::Signal::SIGINT as i32,
    nix::sys::signal::Signal::SIGHUP as i32,
];

/// `conductor broker serve`: serves the active instance's broker over its state directory
/// until SIGTERM, SIGINT or SIGHUP, then closes every socket and exits 0.
///
/// # Errors
///
/// The state directory cannot be found, or the broker cannot bind its sockets.
pub fn broker_serve(flag: Option<&Path>) -> Result<ExitCode> {
    let state = crate::state::dir(flag)?;
    let active = &crate::config::active().instance;
    let instance = active.name.0.clone();
    let stop = Arc::new(AtomicUsize::new(0));
    for signal in STOPPING {
        signal_hook::flag::register_usize(signal, Arc::clone(&stop), 1)
            .context("broker serve: catch the stopping signals")?;
    }
    let dir = broker_dir(&state);
    let recorder: Arc<dyn Recorder> = Arc::new(StoreRecorder::new(&state));
    let services = services(&state, &crate::host::dir(flag)?, active);
    let broker = Broker::start_with(&dir, &instance, recorder, services)
        .with_context(|| format!("broker serve: serve {}", dir.display()))?;
    let mut out = io::stdout().lock();
    writeln!(out, "broker of {instance} serving {}", dir.display())?;
    out.flush()?;
    drop(out);
    while stop.load(Ordering::SeqCst) == 0 {
        thread::park_timeout(Wait::from_millis(250));
    }
    broker.shutdown();
    Ok(ExitCode::SUCCESS)
}

/// The variable that names the instance's Gates policy to `broker serve`, and to the bot route it
/// runs for `commit`; the sessions never see it.
pub const POLICY_VARIABLE: &str = "B10X_GATES_POLICY";

/// What the broker's build, commit and team verbs reach (`story:tool-build-commit-report`): the
/// host ledger under `host`, through `build run`'s own path; the bot route `b10x-gates` with the
/// policy [`POLICY_VARIABLE`] names in this process's environment, if it names one; and the store
/// under `state`.
#[must_use]
pub fn services(state: &Path, host: &Path, instance: &Instance) -> Services {
    let mut sources = build::Sources::of(instance);
    // The broker is no command of a session: a build it runs is bound to no harness session of
    // the broker's own environment.
    sources.env = |name| {
        if name == build::SESSION_VARIABLE {
            None
        } else {
            std::env::var(name).ok()
        }
    };
    Services {
        ledger: Arc::new(HostLedger {
            state: state.to_owned(),
            host: host.to_owned(),
            settings: build::Settings::of(instance),
            sources,
        }),
        bot: BotRoute {
            program: PathBuf::from(crate::gates::PROGRAM),
            policy: std::env::var_os(POLICY_VARIABLE)
                .filter(|policy| !policy.is_empty())
                .map(PathBuf::from),
        },
        messages: Arc::new(StoreMessages::new(state)),
    }
}

/// The host ledger as `conductor build run --reserve auto` uses it: [`build::run`], the command's
/// output into the job's log.
struct HostLedger {
    state: PathBuf,
    host: PathBuf,
    settings: build::Settings,
    sources: build::Sources,
}

impl Ledger for HostLedger {
    fn run(&self, job: &BuildJob) -> BuildEnd {
        let request = build::Request {
            reserve: None,
            repository: None,
            session_name: Some(job.session.clone()),
            command: job.command.clone(),
            measure: Vec::new(),
            cwd: job.cwd.clone(),
            output: Some(job.log.clone()),
        };
        match build::run(
            &self.state,
            &self.host,
            &self.settings,
            &self.sources,
            &request,
        ) {
            Ok(build::Outcome::Ran { code, .. }) => BuildEnd::Ran {
                code: i32::from(code),
            },
            Ok(build::Outcome::Refused {
                bytes, shortfall, ..
            }) => BuildEnd::Refused {
                reason: format!(
                    "{}G reserved for it, the disk is {}G short",
                    bytes.div_ceil(1 << 30),
                    shortfall.div_ceil(1 << 30)
                ),
            },
            Ok(build::Outcome::Stopped { signal, .. }) => BuildEnd::Refused {
                reason: format!("{signal} stopped it before it ran"),
            },
            Ok(build::Outcome::Withdrawn { session, .. }) => BuildEnd::Refused {
                reason: format!("withdrawn: session {session} stopped"),
            },
            Err(error) => BuildEnd::Refused {
                reason: format!("{error:#}"),
            },
        }
    }
}

/// The store `send` and `report` record in: each a `conductor.dispatch.Message` received as
/// `conductor message receive-message` records one, and a lead's report moves its team's open
/// dispatch as `conductor dispatch report-*` does.
pub struct StoreMessages {
    state: PathBuf,
    lock: Mutex<()>,
}

impl StoreMessages {
    /// The store under `state`.
    #[must_use]
    pub fn new(state: &Path) -> Self {
        Self {
            state: state.to_owned(),
            lock: Mutex::new(()),
        }
    }

    fn record(&self, command: &str, input: ReceiveMessage, repo: &str) -> Result<String, String> {
        let outcome = crate::message::record(command, Some(&self.state), input, repo)
            .map_err(|error| format!("{error:#}"))?;
        let (ReceiveMessageOutcome::Received { message_received }
        | ReceiveMessageOutcome::Unparsed { message_received }) = outcome;
        Ok(message_received.message_id.0.0)
    }

    /// The open dispatch of `repository` a lead's report names: the one sent, started or blocked,
    /// or among several, the one started or blocked; none, with why, when there is no single one.
    fn open_dispatch(&self, repository: &str) -> Result<Result<String, String>, String> {
        let store = crate::store::open(&self.state).map_err(|error| format!("{error:#}"))?;
        let open: Vec<(String, DispatchState)> = DispatchStorage::list(&store)
            .into_iter()
            .filter(|held| held.data.repository.0 == repository)
            .filter(|held| {
                matches!(
                    held.state,
                    DispatchState::Sent | DispatchState::Started | DispatchState::Blocked
                )
            })
            .map(|held| (held.data.dispatch_id.0, held.state))
            .collect();
        drop(store);
        let working: Vec<&String> = open
            .iter()
            .filter(|(_, state)| *state != DispatchState::Sent)
            .map(|(id, _)| id)
            .collect();
        Ok(if open.is_empty() {
            Err(format!("no open dispatch of {repository}"))
        } else if let [(one, _)] = open.as_slice() {
            Ok(one.clone())
        } else if let [one] = working.as_slice() {
            Ok((*one).clone())
        } else {
            Err(format!(
                "{} open dispatches of {repository}, none moved",
                open.len()
            ))
        })
    }

    /// Moves `dispatch` as `conductor dispatch report-<status>` does, with `line` as its reason
    /// or evidence; `progress` moves nothing.
    fn move_dispatch(&self, dispatch: &str, status: ReportStatus, line: &str) -> Result<bool> {
        let state = Some(self.state.as_path());
        let id = Some(dispatch.to_owned());
        let text = Some(line.to_owned());
        match status {
            ReportStatus::Progress => return Ok(false),
            ReportStatus::Started => crate::dispatch::report_started(
                state,
                ReportStartedArgs {
                    dispatch_id: id,
                    input_json: None,
                },
            ),
            ReportStatus::Blocked => crate::dispatch::report_blocked(
                state,
                ReportBlockedArgs {
                    dispatch_id: id,
                    reason: text,
                    input_json: None,
                },
            ),
            ReportStatus::Unblocked => crate::dispatch::report_unblocked(
                state,
                ReportUnblockedArgs {
                    dispatch_id: id,
                    input_json: None,
                },
            ),
            ReportStatus::Done => crate::dispatch::report_done(
                state,
                ReportDoneArgs {
                    dispatch_id: id,
                    evidence: text,
                    input_json: None,
                },
            ),
            ReportStatus::Failed => crate::dispatch::report_failed(
                state,
                ReportFailedArgs {
                    dispatch_id: id,
                    reason: text,
                    input_json: None,
                },
            ),
        }?;
        Ok(true)
    }
}

impl Messages for StoreMessages {
    fn send(&self, message: &TeamMessage) -> Result<String, String> {
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let first = message.text.lines().next().unwrap_or_default();
        self.record(
            "broker send",
            ReceiveMessage {
                sender: message.sender.clone(),
                recipient: message.recipient.clone(),
                kind: MessageKind::Team,
                first_line: format!("[TEAM {}] {first}", message.team),
                transport: Transport::SendMessage,
                received_at: Timestamp(message.at.clone()),
                body_path: None,
                parsed: true,
            },
            &message.team,
        )
    }

    fn report(&self, report: &Report) -> Result<Reported, String> {
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let status = conductor_broker::status_word(report.status);
        let repo = report.team.clone().unwrap_or_else(|| report.sender.clone());
        let mut dispatch = None;
        let mut moved = None;
        let mut note = None;
        if report.lead {
            match self.open_dispatch(&repo)? {
                Ok(open) => {
                    if self
                        .move_dispatch(&open, report.status, &report.line)
                        .map_err(|error| format!("dispatch {open} did not move: {error:#}"))?
                    {
                        moved = Some(open.clone());
                    } else {
                        note = Some(format!("{status} moves no dispatch"));
                    }
                    dispatch = Some(open);
                }
                Err(why) => note = Some(why),
            }
        }
        let first_line = match &dispatch {
            Some(open) => format!("[REPORT {repo} {open}] {status} {}", report.line),
            None => format!("[REPORT {repo}] {status} {}", report.line),
        };
        let message = self.record(
            "broker report",
            ReceiveMessage {
                sender: report.sender.clone(),
                recipient: report.recipient.clone(),
                kind: MessageKind::Report,
                first_line,
                transport: Transport::SendMessage,
                received_at: Timestamp(report.at.clone()),
                body_path: None,
                parsed: true,
            },
            &repo,
        )?;
        Ok(Reported {
            message,
            dispatch: moved,
            note,
        })
    }
}

/// Records each call in the store under a state directory, one at a time.
pub struct StoreRecorder {
    state: PathBuf,
    lock: Mutex<()>,
}

impl StoreRecorder {
    /// A recorder into the store under `state`.
    #[must_use]
    pub fn new(state: &Path) -> Self {
        Self {
            state: state.to_owned(),
            lock: Mutex::new(()),
        }
    }

    fn try_record(&self, call: &CallRecord) -> Result<()> {
        let _held = self.lock.lock().unwrap_or_else(PoisonError::into_inner);
        let mut generated = Generated::new(Ports {
            store: crate::store::open(&self.state)?,
        });
        let RecordToolCallOutcome::Recorded { .. } = generated
            .record_tool_call(RecordToolCall {
                instance: InstanceName(call.instance.clone()),
                session: SessionName(call.session.clone()),
                verb: call.verb,
                paths: call.paths.clone(),
                outcome: call.outcome,
                rule: call.rule.clone(),
                duration_ms: call.duration_ms,
                output_bytes: call.output_bytes,
                called_at: Timestamp(call.called_at.clone()),
            })
            .map_err(|unmet| anyhow!("record-tool-call: {unmet}"))?;
        generated.ports.store.check().context("keep the call")?;
        Ok(())
    }
}

impl Recorder for StoreRecorder {
    fn record(&self, call: &CallRecord) {
        if let Err(error) = self.try_record(call) {
            eprintln!(
                "broker: the call of {} by {} was answered and not recorded: {error:#}",
                conductor_broker::verb_word(call.verb),
                call.session
            );
        }
    }
}

/// The ports `RecordToolCall` runs over: the store, and the identity it assigns.
struct Ports {
    store: Store,
}

impl ToolCallStorage for Ports {
    fn get(&self, identity: &ToolCallId) -> Option<ToolCallSnapshot> {
        ToolCallStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: ToolCallSnapshot) {
        ToolCallStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &ToolCallId) {
        ToolCallStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<ToolCallSnapshot> {
        ToolCallStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_tool_tool_call_id(&mut self) -> Result<ToolCallId, UnmetObligation> {
        Ok(ToolCallId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<conductor_model::direction::ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<conductor_model::dispatch::GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(
        &mut self,
    ) -> Result<conductor_model::dispatch::MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<conductor_model::dispatch::ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_observation_board_id(
        &mut self,
    ) -> Result<conductor_model::observation::BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<conductor_model::observation::ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<conductor_model::observation::SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }

    fn try_generate_conductor_host_grant_id(
        &mut self,
    ) -> Result<conductor_model::host::GrantId, UnmetObligation> {
        Err(unmet_context("conductor.host.GrantId"))
    }

    fn try_generate_conductor_host_reclaim_run_id(
        &mut self,
    ) -> Result<conductor_model::host::ReclaimRunId, UnmetObligation> {
        Err(unmet_context("conductor.host.ReclaimRunId"))
    }
}

/// The grant a session of `role` starts with at `now`: the role's `grant`, its time to live
/// counted from `now`.
#[must_use]
pub fn session_grant(role: &Role, now: OffsetDateTime) -> SessionGrant {
    let grant = &role.grant;
    let expires_at = grant
        .ttl
        .as_ref()
        .and_then(crate::config::seconds)
        .and_then(|seconds| i64::try_from(seconds).ok())
        .map(|seconds| rfc3339(now + time::Duration::seconds(seconds)));
    SessionGrant {
        tools: grant.tools.clone(),
        repositories: grant.repositories.clone(),
        paths: grant.paths.clone(),
        max_output_bytes: grant.max_output_bytes,
        expires_at,
    }
}

fn rfc3339(at: OffsetDateTime) -> String {
    at.replace_nanosecond(0)
        .unwrap_or(at)
        .format(&Rfc3339)
        .unwrap_or_default()
}

/// The session name of the instance's director, its `conductor` role: the role's `session_name`,
/// else [`crate::config::session_name`] of the role.
#[must_use]
pub fn director_session(instance: &Instance) -> SessionName {
    instance
        .roles
        .iter()
        .find(|role| role.role == DIRECTOR_ROLE)
        .and_then(|role| role.session_name.clone())
        .unwrap_or_else(|| crate::config::session_name(instance, DIRECTOR_ROLE))
}

/// The config role whose session is an instance's director (design § 2).
const DIRECTOR_ROLE: &str = "conductor";

/// The place of a session of the config role `role` (design § 2, `story:tool-build-commit-report`):
/// the `conductor` role's session is the instance's director, with no team and nothing above it
/// while no bridge serves; a controller's, started for `repository`, is that repository's lead,
/// its team named after the repository, reporting to the director. Any other role's session has
/// its role and no level.
#[must_use]
pub fn placement(instance: &Instance, role: &str, repository: Option<&str>) -> Placement {
    let mut place = Placement {
        role: Some(role.to_owned()),
        ..Placement::default()
    };
    if role == DIRECTOR_ROLE {
        place.level = Some(Level::Director);
    } else if role == crate::config::CONTROLLER
        && let Some(repository) = repository
    {
        place.level = Some(Level::Lead);
        place.team = Some(repository.to_owned());
        place.above = Some(director_session(instance).0);
    }
    place
}

/// Registers the session `session` of `role` with the instance's broker under `state`, reading
/// in `workdir`, at `place`, and answers its socket, which the session's environment names as
/// [`conductor_broker::SOCKET_VARIABLE`]. A broker that serves opens the socket now; one that
/// starts later opens it then.
///
/// # Errors
///
/// The registration cannot be written.
pub fn register_session(
    state: &Path,
    instance: &InstanceName,
    role: &Role,
    session: &SessionName,
    workdir: &Path,
    place: Placement,
) -> Result<PathBuf> {
    let now = OffsetDateTime::now_utc();
    conductor_broker::register(
        &broker_dir(state),
        &Registration {
            session: session.0.clone(),
            instance: instance.0.clone(),
            workdir: workdir.to_owned(),
            grant: session_grant(role, now),
            started_at: rfc3339(now),
            place,
        },
    )
    .with_context(|| format!("register session {} with the broker", session.0))
}

/// Removes the session's registration: its next call is refused `UnknownSession`.
///
/// # Errors
///
/// The registration is there and cannot be removed.
pub fn unregister_session(state: &Path, session: &SessionName) -> Result<()> {
    conductor_broker::unregister(&broker_dir(state), &session.0)
        .with_context(|| format!("unregister session {} from the broker", session.0))
}
