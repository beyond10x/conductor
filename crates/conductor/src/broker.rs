//! `conductor broker serve` (`story:tool-broker-read`, design docs/design/roles-and-surfaces.md
//! § 6): runs the instance's broker ([`conductor_broker::Broker`]) over `<state>/broker` until it
//! is stopped, recording every call in the instance's store through the generated
//! `conductor.tool.RecordToolCall`; and the registration of a session's grant that `conductor
//! spawn` and `conductor session start` make before they start a session ([`register_session`]).

use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::Duration as Wait;

use anyhow::{Context as _, Result, anyhow};
use conductor_broker::{Broker, CallRecord, Recorder, Registration, SessionGrant, broker_dir};
use conductor_model::behaviour::{Generated, ToolCallStorage, TryContext, unmet_context};
use conductor_model::config::{InstanceName, Role, SessionName};
use conductor_model::obligation::UnmetObligation;
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::tool::obligations::RecordToolCallBehavior;
use conductor_model::tool::{RecordToolCall, RecordToolCallOutcome, ToolCallId, ToolCallSnapshot};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

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
pub fn broker_serve(state: Option<&Path>) -> Result<ExitCode> {
    let state = crate::state::dir(state)?;
    let instance = crate::config::active().instance.name.0.clone();
    let stop = Arc::new(AtomicUsize::new(0));
    for signal in STOPPING {
        signal_hook::flag::register_usize(signal, Arc::clone(&stop), 1)
            .context("broker serve: catch the stopping signals")?;
    }
    let dir = broker_dir(&state);
    let recorder: Arc<dyn Recorder> = Arc::new(StoreRecorder::new(&state));
    let broker = Broker::start(&dir, &instance, recorder)
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

/// Registers the session `session` of `role` with the instance's broker under `state`, reading
/// in `workdir`, and answers its socket, which the session's environment names as
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
