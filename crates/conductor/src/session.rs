//! `conductor session`: the instance's one conductor session and the lease it holds
//! (`story:single-conductor-lease`).
//!
//! `session claim` is the command `conductor.session.ClaimLease`. It records the lease in the
//! store under the state directory (`--state-dir`, else the instance's state; [`crate::state::dir`])
//! and answers exactly as the specification orders its outcomes:
//!
//! - `taken`: no lease is recorded for the instance; the claiming session holds it now.
//! - `renewed`: the lease names the claiming process itself, as it does when a session is resumed,
//!   cleared or compacted in the same process; it keeps the lease under the session id it gives
//!   now, and the takeover the lease records stays.
//! - `held`: the lease names another process, and that process lives ([`lives`]). The claim is
//!   refused with `LeaseHeld`, which names the holder's session id and pid and tells the claiming
//!   session it is a second conductor and must stop; the lease is unchanged.
//! - `taken-over`: the lease names another process, and no process has that pid any more. The
//!   claiming session takes the lease, and the lease records the session id and pid it was taken
//!   from.
//!
//! A claim the store kept (`taken`, `renewed`, `taken-over`) also writes the guard's copy of the
//! lease, [`LEASE_FILE`] under the state directory, at once ([`LeaseFile`]); a refusal leaves it
//! as it was. The guard reads that file, never the store, on every conductor write
//! (`crate::guard`, `story:lease-guard`).
//!
//! A success prints one line on standard output; a refusal exits 1 with one line on standard
//! error that names its error. Two claims at once that both find the lease as it was are not both
//! kept: the store refuses the write decided on a stale read, and that claim exits 1.
//!
//! With `--from-session-start` the command is the SessionStart hook of the conductor role's
//! settings (`.claude/conductor-settings.json`): it reads the payload Claude Code sends a
//! SessionStart hook on standard input and takes the session's id from its `session_id`, as the
//! guard reads a PreToolUse payload (`crate::guard`). The payload names no process, so the
//! settings pass it as `--pid "$PPID"`: Claude Code runs a hook command under `sh -c` from the
//! session's own process, so the shell's parent is that process, the one `claude agents --json`
//! lists as the session's `pid`, which lives exactly as long as the session does. This binary's
//! own parent is not a substitute: it is that shell, unless the shell replaced itself with its
//! last command, which depends on the shell; `$PPID` does not. Claude Code shows a SessionStart
//! hook's standard error to the operator on exit 2, which the settings turn every failure into.
//!
//! `session leases` is the view `conductor.session.Leases`. Input through `--input-json -` is not
//! written yet.
//!
//! `session start` and `session stop` (`story:session-envelope-seam`) start and stop the session
//! of the role `conductor` or `conductor-dev` through the confinement provider ([`start`],
//! [`stop`], [`crate::confine`]): with the role's harness arguments and first prompt, in the
//! records directory for conductor and the working directory for conductor-dev. The maintainer
//! (`conductor-dev`) is the host's role (`story:maintainer-host-config`): read from the host file
//! `host.yaml` in the host's own directory, else, for this release, from the instance's `roles`
//! ([`crate::config::host_maintainer`]).
//!
//! `session deliver` (`story:mailbox-send-and-deliver`, [`deliver`]) types a text into a running
//! session through its tmux launcher and records the delivery; `session deliveries` is the view
//! `conductor.session.Deliveries` ([`deliveries`]). The placed `session deliver-message` leaf, a
//! delivery recorded with nothing typed, and the run daemon's `record-run-tick` and `run-ticks`
//! (`story:run-daemon`) are not written yet.

use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    DeliveryStorage, ExternalCommand, Generated, LeaseStorage, TryContext, unmet_context,
};
use conductor_model::config::{
    ConfinementProvider, Harness, Instance, InstanceName, Role, RoleName, SessionName,
};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::{DispatchLogEvent, GuardDecisionId, MessageId, ResourceRequestId};
use conductor_model::host::{GrantId, ReclaimRunId};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, SnapshotId};
use conductor_model::primitives::{Timestamp, Uuid};
use conductor_model::session::obligations::{
    ClaimLeaseBehavior, DeliverMessageBehavior, DeliveriesQuery, LeasesQuery,
};
use conductor_model::session::{
    ClaimLease, ClaimLeaseOutcome, DeliverMessage, DeliverMessageOutcome, DeliverToSession,
    DeliveryId, DeliverySnapshot, DeliveryState, LeaseData, LeaseHeld, LeaseSnapshot, LeaseState,
    LeaseTaken, LeaseTakenOver, SessionId, SessionRequest, SessionStartRefusal,
    SessionStartRefused, SessionStopRefusal, SessionStopRefused, StartSession, StartedSession,
    StopSession, TickId,
};
use conductor_model::tool::ToolCallId;
use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{ClaimLeaseArgs, DeliverArgs, SessionRoleArgs, ViewArgs};
use crate::config::{self, Environment};
use crate::confine::places::{self, Placement};
use crate::confine::{self, Launch, Providers, Refusal, systemd};
use crate::not_implemented;
use crate::records;
use crate::store::Store;

/// `conductor session claim`: the command `conductor.session.ClaimLease`, for the instance this
/// process runs. `--instance` may name it, and no other; `--taken-at` defaults to now.
///
/// # Errors
///
/// `--input-json` is given; a flag is missing; `--pid` is not 1 or more; the SessionStart payload
/// does not read or names no `session_id`; `--instance` is not the instance this process runs; the
/// store does not open or keep the lease; or another session's live process holds the lease
/// (`LeaseHeld`).
pub fn claim(state: Option<&Path>, args: ClaimLeaseArgs) -> Result<ExitCode> {
    const WHAT: &str = "session claim";
    if args.input_json.is_some() {
        return not_implemented("session claim --input-json");
    }
    let session_id = if args.from_session_start {
        session_start_id(WHAT)?
    } else {
        required(args.session_id, WHAT, "--session-id")?
    };
    let pid = required(args.pid, WHAT, "--pid")?;
    if pid < 1 {
        bail!("{WHAT}: --pid {pid} names no process: a process id is 1 or more");
    }
    let instance = crate::config::active().instance.name.clone();
    if let Some(given) = args.instance
        && given != instance.0
    {
        bail!(
            "{WHAT}: --instance {given:?} is not the instance this process runs: {:?}",
            instance.0
        );
    }
    let taken_at = match args.taken_at {
        Some(taken_at) => taken_at,
        None => OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .context("format the time of the claim")?,
    };
    let dir = crate::state::dir(state)?;
    let claimed = LeaseFile {
        instance: instance.0.clone(),
        session_id: session_id.clone(),
        pid,
    };
    let mut generated = Generated::new(Ports {
        store: crate::state::open(Some(&dir))?,
    });
    let outcome = generated
        .claim_lease(ClaimLease {
            instance,
            session_id: SessionId(session_id),
            pid,
            taken_at: Timestamp(taken_at),
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    generated.ports.store.check().context("keep the lease")?;
    if !matches!(outcome, ClaimLeaseOutcome::Held { .. }) {
        claimed.write(&dir).with_context(|| {
            format!("{WHAT}: the store keeps the lease, but the guard's copy of it was not written")
        })?;
    }
    let line = match outcome {
        ClaimLeaseOutcome::Taken { lease_taken } => format!(
            "session {} (pid {}) holds the conductor lease of instance {}",
            lease_taken.session_id.0, lease_taken.pid, lease_taken.instance.0
        ),
        ClaimLeaseOutcome::Renewed { lease_taken } => format!(
            "session {} (pid {}) holds the conductor lease of instance {} again",
            lease_taken.session_id.0, lease_taken.pid, lease_taken.instance.0
        ),
        ClaimLeaseOutcome::TakenOver { lease_taken_over } => format!(
            "session {} (pid {}) took the conductor lease of instance {} over from session {} \
             (pid {}), whose process is gone",
            lease_taken_over.session_id.0,
            lease_taken_over.pid,
            lease_taken_over.instance.0,
            lease_taken_over.taken_over_session_id.0,
            lease_taken_over.taken_over_pid
        ),
        ClaimLeaseOutcome::Held {
            error:
                LeaseHeld {
                    instance,
                    session_id,
                    pid,
                },
        } => bail!(
            "{WHAT}: refused: LeaseHeld: session {} (pid {pid}) holds the conductor lease of \
             instance {} and its process lives: this session is a second conductor of the \
             instance and must stop",
            session_id.0,
            instance.0
        ),
    };
    writeln!(io::stdout().lock(), "{WHAT}: {line}")
        .context("write the claim to standard output")?;
    Ok(ExitCode::SUCCESS)
}

/// `conductor session leases`: the view `conductor.session.Leases`, served by the generated query
/// over the existing store under the state directory and rendered as `--format` asks. A view
/// creates no store.
///
/// Lists every lease it can read, and names each record it cannot on standard error, after them,
/// exiting 1 ([`crate::store::show`]).
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn leases(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/session.yaml`).
    const COLUMNS: [&str; 7] = [
        "instance",
        "state",
        "session_id",
        "pid",
        "taken_at",
        "taken_over_session_id",
        "taken_over_pid",
    ];

    let generated = Generated::new(crate::state::open_reader(state)?);
    let rows = generated
        .leases()
        .map_err(|unmet| anyhow!("session leases: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.instance.0),
                Value::from(state_name(row.state)),
                Value::from(row.session_id.0),
                Value::from(row.pid),
                Value::from(row.taken_at.0),
                Value::from(row.taken_over_session_id.map(|id| id.0)),
                Value::from(row.taken_over_pid),
            ]
        })
        .collect();
    crate::store::show(&generated.ports, "leases", &view.render(&COLUMNS, &rows))
}

/// The directory under the state directory's mailbox that keeps the text `session deliver` read
/// from standard input, so every delivery names the file its text is in.
const DELIVERED: &str = "delivered";

/// `conductor session deliver` (`story:mailbox-send-and-deliver`, docs/design/conductor-run.md
/// § 3): the input `conductor.session.DeliverToSession`. Types the text `--text` names (a file, or
/// `-` for standard input) into the session `--name`'s tmux server on
/// `<state>/sessions/<name>.tmux` through the launcher's [`systemd::type_text`]: the text, its
/// trailing line breaks left off, as one paste, then Enter. Then it records the delivery with the
/// generated `conductor.session.DeliverMessage` (its first line, its body path, the time) and
/// appends one `delivered` line to the records' dispatch log: `to` the session, `first_line`,
/// `body_path`, and as `repo` the repository the first line names, else the session. The body
/// path is the file's path, relative to the state directory when it lies under it; text read
/// from standard input is kept first as `<state>/mailbox/delivered/<ulid>.md`. Prints the
/// delivery's id.
///
/// A session with no socket, or whose tmux does not take the text, is refused
/// (`SessionNotReachable`, the specification's `unreachable` branch), exit 1 naming the session:
/// nothing was kept or recorded, and the command does not try again.
///
/// # Errors
///
/// A flag is missing, `--name` is no plain name, the text cannot be read, the session is not
/// reached, or the body, the store or the records line cannot be written.
pub fn deliver(state: Option<&Path>, args: &DeliverArgs) -> Result<ExitCode> {
    const WHAT: &str = "session deliver";
    let input = DeliverToSession {
        name: required(args.name.clone(), WHAT, "--name")?,
        text: required(args.text.clone(), WHAT, "--text")?,
    };
    if !crate::message::plain_name(&input.name) {
        bail!(
            "{WHAT}: --name {:?} names no session: one name, without `/`, not `.` or `..`",
            input.name
        );
    }
    let text = crate::message::read_text(WHAT, &input.text)?;
    let first_line = crate::message::first_line(&text).to_owned();
    let typed = text.trim_end_matches(['\n', '\r']);
    let dir = crate::state::dir(state)?;
    let sessions = dir.join("sessions");
    let name = SessionName(input.name);
    let socket = sessions.join(format!("{}.tmux", name.0));
    let unreached = if fs::symlink_metadata(&socket).is_err() {
        Some(format!(
            "its launcher socket {} is absent, so nothing was typed",
            socket.display()
        ))
    } else {
        systemd::type_text(&sessions, &name, typed)
            .err()
            .map(|error| {
                // The text is one of tmux's arguments; the refusal stays one line without it.
                let mut said = format!("{error:#}");
                if !typed.is_empty() {
                    said = said.replace(typed, "<the text>");
                }
                format!(
                    "its tmux on {} did not take the text: {}",
                    socket.display(),
                    said.split_whitespace().collect::<Vec<_>>().join(" ")
                )
            })
    };
    let delivered_at = records::now()?;
    let mut generated = Generated::new(Delivering {
        store: None,
        reached: unreached.is_none(),
    });
    let body_path = if unreached.is_some() {
        None
    } else {
        let body_path = if input.text == "-" {
            let body_path = format!(
                "{}/{DELIVERED}/{}.md",
                crate::message::MAILBOX,
                crate::message::ulid()?
            );
            crate::message::keep(&dir.join(&body_path), &text)
                .with_context(|| format!("{WHAT}: keep the text"))?;
            body_path
        } else {
            body_path_of(Path::new(&input.text), &dir)
        };
        generated.ports.store = Some(crate::state::open(Some(&dir))?);
        Some(body_path)
    };
    let outcome = generated
        .deliver_message(DeliverMessage {
            session_name: name.clone(),
            first_line: first_line.clone(),
            body_path: body_path.clone(),
            delivered_at: delivered_at.clone(),
        })
        .map_err(|unmet| anyhow!("{WHAT}: {unmet}"))?;
    if let Some(store) = &generated.ports.store {
        store.check().context("keep the delivery")?;
    }
    let message_delivered = match outcome {
        DeliverMessageOutcome::Delivered { message_delivered } => message_delivered,
        DeliverMessageOutcome::Unreachable { error } => bail!(
            "{WHAT}: refused: SessionNotReachable: session {}: {}; nothing was recorded, and it is \
             not tried again",
            error.session_name.0,
            unreached.as_deref().unwrap_or("it was not reached")
        ),
    };
    let repo = crate::message::named_repository(&first_line).unwrap_or_else(|| name.0.clone());
    let mut line = records::dispatch_line(DispatchLogEvent::Delivered, &repo)?;
    line.at = delivered_at;
    line.to = Some(name.0.clone());
    line.first_line = Some(first_line);
    line.body_path = body_path;
    records::append_dispatch(&line).with_context(|| {
        format!(
            "{WHAT}: delivery {} to session {}",
            message_delivered.delivery_id.0.0, name.0
        )
    })?;
    writeln!(io::stdout().lock(), "{}", message_delivered.delivery_id.0.0)
        .context("write the delivery's id to standard output")?;
    Ok(ExitCode::SUCCESS)
}

/// The body path a delivery records for the file `file`: its path relative to the state
/// directory `state` when it lies under it, else its absolute path.
fn body_path_of(file: &Path, state: &Path) -> String {
    let file = fs::canonicalize(file)
        .or_else(|_| std::path::absolute(file))
        .unwrap_or_else(|_| file.to_owned());
    let state = fs::canonicalize(state).unwrap_or_else(|_| state.to_owned());
    file.strip_prefix(&state)
        .unwrap_or(&file)
        .display()
        .to_string()
}

/// `conductor session deliveries`: the view `conductor.session.Deliveries`, served by the
/// generated query over the existing store under the state directory and rendered as `--format`
/// asks, in the order the deliveries were made. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn deliveries(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/session.yaml`).
    const COLUMNS: [&str; 6] = [
        "delivery_id",
        "state",
        "session_name",
        "first_line",
        "body_path",
        "delivered_at",
    ];

    let generated = Generated::new(crate::state::open_reader(state)?);
    let rows = generated
        .deliveries()
        .map_err(|unmet| anyhow!("session deliveries: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.delivery_id.0.0),
                Value::from(match row.state {
                    DeliveryState::Delivered => "Delivered",
                }),
                Value::from(row.session_name.0),
                Value::from(row.first_line),
                Value::from(row.body_path),
                Value::from(row.delivered_at.0),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "deliveries",
        &view.render(&COLUMNS, &rows),
    )
}

/// The ports `DeliverMessage` runs over: the store, opened only once the session was reached,
/// so a refusal creates none; whether the launcher reached the session, which answers the
/// specification's `unreachable` branch; and the delivery's id, assigned here.
struct Delivering {
    store: Option<Store>,
    reached: bool,
}

impl DeliveryStorage for Delivering {
    fn get(&self, identity: &DeliveryId) -> Option<DeliverySnapshot> {
        self.store
            .as_ref()
            .and_then(|store| DeliveryStorage::get(store, identity))
    }

    fn put(&mut self, snapshot: DeliverySnapshot) {
        if let Some(store) = &mut self.store {
            DeliveryStorage::put(store, snapshot);
        }
    }

    fn delete(&mut self, identity: &DeliveryId) {
        if let Some(store) = &mut self.store {
            DeliveryStorage::delete(store, identity);
        }
    }

    fn list(&self) -> Vec<DeliverySnapshot> {
        self.store
            .as_ref()
            .map(DeliveryStorage::list)
            .unwrap_or_default()
    }
}

impl TryContext for Delivering {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_host_grant_id(&mut self) -> Result<GrantId, UnmetObligation> {
        Err(unmet_context("conductor.host.GrantId"))
    }

    fn try_generate_conductor_host_reclaim_run_id(
        &mut self,
    ) -> Result<ReclaimRunId, UnmetObligation> {
        Err(unmet_context("conductor.host.ReclaimRunId"))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }

    fn try_generate_conductor_session_delivery_id(
        &mut self,
    ) -> Result<DeliveryId, UnmetObligation> {
        Ok(DeliveryId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_session_tick_id(&mut self) -> Result<TickId, UnmetObligation> {
        Err(unmet_context("conductor.session.TickId"))
    }

    fn try_generate_conductor_tool_tool_call_id(&mut self) -> Result<ToolCallId, UnmetObligation> {
        Err(unmet_context("conductor.tool.ToolCallId"))
    }

    fn try_external(
        &mut self,
        command: ExternalCommand<'_>,
        outcome: &'static str,
    ) -> Result<bool, UnmetObligation> {
        match (command, outcome) {
            (ExternalCommand::ConductorSessionDeliverMessage(_), "unreachable") => {
                Ok(!self.reached)
            }
            (command, _) => Err(unmet_context(command.name())),
        }
    }
}

/// The ports `ClaimLease` runs over: the store. Whether a pid names a live process is read from
/// the process table ([`lives`]).
struct Ports {
    store: Store,
}

impl LeaseStorage for Ports {
    fn get(&self, identity: &InstanceName) -> Option<LeaseSnapshot> {
        LeaseStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: LeaseSnapshot) {
        LeaseStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &InstanceName) {
        LeaseStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<LeaseSnapshot> {
        LeaseStorage::list(&self.store)
    }
}

/// `conductor.session.ClaimLease`, as its contract in `crates/conductor-model/PLAN.md` states it:
/// `taken` when no lease is recorded for the instance; `renewed` when the lease's `pid` is the
/// input's; `held`, the external refusal, when the lease's `pid` names a live process; `taken-over`
/// otherwise.
impl ClaimLeaseBehavior for Ports {
    fn claim_lease(&mut self, input: ClaimLease) -> Result<ClaimLeaseOutcome, UnmetObligation> {
        let Some(LeaseSnapshot { data: held, .. }) =
            LeaseStorage::get(&self.store, &input.instance)
        else {
            self.keep(LeaseData {
                instance: input.instance.clone(),
                session_id: input.session_id.clone(),
                pid: input.pid,
                taken_at: input.taken_at,
                taken_over_session_id: None,
                taken_over_pid: None,
            });
            return Ok(ClaimLeaseOutcome::Taken {
                lease_taken: LeaseTaken {
                    instance: input.instance,
                    session_id: input.session_id,
                    pid: input.pid,
                },
            });
        };
        if held.pid == input.pid {
            self.keep(LeaseData {
                session_id: input.session_id.clone(),
                taken_at: input.taken_at,
                ..held
            });
            return Ok(ClaimLeaseOutcome::Renewed {
                lease_taken: LeaseTaken {
                    instance: input.instance,
                    session_id: input.session_id,
                    pid: input.pid,
                },
            });
        }
        if lives(held.pid) {
            return Ok(ClaimLeaseOutcome::Held {
                error: LeaseHeld {
                    instance: input.instance,
                    session_id: held.session_id,
                    pid: held.pid,
                },
            });
        }
        self.keep(LeaseData {
            instance: input.instance.clone(),
            session_id: input.session_id.clone(),
            pid: input.pid,
            taken_at: input.taken_at,
            taken_over_session_id: Some(held.session_id.clone()),
            taken_over_pid: Some(held.pid),
        });
        Ok(ClaimLeaseOutcome::TakenOver {
            lease_taken_over: LeaseTakenOver {
                instance: input.instance,
                session_id: input.session_id,
                pid: input.pid,
                taken_over_session_id: held.session_id,
                taken_over_pid: held.pid,
            },
        })
    }
}

impl Ports {
    /// Stores `data` as the instance's lease, `Held`, its one state.
    fn keep(&mut self, data: LeaseData) {
        LeaseStorage::put(
            &mut self.store,
            LeaseSnapshot {
                state: LeaseState::Held,
                data,
            },
        );
    }
}

/// Whether `pid` names a process: `kill` with no signal, which sends nothing and only checks.
/// A process of another user answers `EPERM`, and lives; only `ESRCH` says no process has the pid.
/// A pid below 1, or beyond what the platform's pids hold, names none: `kill` would read one below
/// 1 as a process group.
pub(crate) fn lives(pid: i64) -> bool {
    match i32::try_from(pid) {
        Ok(pid) if pid >= 1 => !matches!(kill(Pid::from_raw(pid), None), Err(Errno::ESRCH)),
        _ => false,
    }
}

/// The guard's copy of the lease: `conductor.lease` under the state directory, beside the store
/// (`story:lease-guard`).
pub(crate) const LEASE_FILE: &str = "conductor.lease";

/// The most of [`LEASE_FILE`] the guard reads; the file [`LeaseFile::write`] writes is far smaller.
const LEASE_FILE_LIMIT: u64 = 4096;

/// The lease as [`LEASE_FILE`] holds it: one JSON object on one line, naming the instance, the
/// holder's session id and its pid. `session claim` writes it after the store kept a claim that
/// took, renewed or took over the lease, so the guard can hold the lease on every conductor write
/// by reading one small file instead of opening the store. The store stays the record:
/// `session leases` reads the store, not this file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeaseFile {
    pub instance: String,
    pub session_id: String,
    pub pid: i64,
}

impl LeaseFile {
    /// Writes the file under `state` at once: to a staging file of this process in the same
    /// directory, then renamed over [`LEASE_FILE`], so a reader finds the old lease or the new one
    /// and never a part of either.
    fn write(&self, state: &Path) -> Result<()> {
        let path = state.join(LEASE_FILE);
        let staged = state.join(format!(".{LEASE_FILE}.{}", std::process::id()));
        let body = json!({
            "instance": self.instance,
            "session_id": self.session_id,
            "pid": self.pid,
        });
        fs::write(&staged, format!("{body}\n"))
            .with_context(|| format!("write {}", staged.display()))?;
        fs::rename(&staged, &path).with_context(|| {
            let _ = fs::remove_file(&staged);
            format!("rename {} to {}", staged.display(), path.display())
        })
    }

    /// The lease [`LEASE_FILE`] under `state` names, or `None` when there is no such regular file
    /// (a link is not followed), it cannot be read, or it does not hold the three fields.
    pub(crate) fn read(state: &Path) -> Option<Self> {
        let path = state.join(LEASE_FILE);
        if !fs::symlink_metadata(&path).is_ok_and(|metadata| metadata.is_file()) {
            return None;
        }
        let mut text = String::new();
        File::open(&path)
            .ok()?
            .take(LEASE_FILE_LIMIT)
            .read_to_string(&mut text)
            .ok()?;
        let value: Value = serde_json::from_str(&text).ok()?;
        Some(Self {
            instance: value["instance"].as_str()?.to_owned(),
            session_id: value["session_id"].as_str()?.to_owned(),
            pid: value["pid"].as_i64()?,
        })
    }
}

/// The session's id from the SessionStart payload on standard input.
fn session_start_id(what: &str) -> Result<String> {
    let mut payload = Vec::new();
    io::stdin()
        .read_to_end(&mut payload)
        .with_context(|| format!("{what}: read the SessionStart payload on standard input"))?;
    let payload: Value = serde_json::from_slice(&payload).with_context(|| {
        format!("{what}: the SessionStart payload on standard input is not JSON")
    })?;
    match payload["session_id"].as_str() {
        Some(id) if !id.is_empty() => Ok(id.to_owned()),
        _ => bail!("{what}: the SessionStart payload on standard input names no session_id"),
    }
}

/// The value of `flag`, which `what` cannot do without.
fn required<T>(value: Option<T>, what: &str, flag: &str) -> Result<T> {
    match value {
        Some(value) => Ok(value),
        None => bail!("{what}: {flag} is required"),
    }
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than rendering under a guessed name.
fn state_name(state: LeaseState) -> &'static str {
    match state {
        LeaseState::Held => "Held",
    }
}

/// The first prompt of conductor's session (`session start --role conductor`).
pub const CONDUCTOR_PROMPT: &str = "Start of session. After the session-start reading and the \
lease check of your profile, arm what wakes you; the hand-over does not list it. Run CronList, \
then CronCreate each recurring job it does not show: one every cadence.cycle, one at \
cadence.daily (UTC) for the daily cycle, both values from conductor config show, with the prompts \
your profile gives. Start the watch once with the Monitor tool: conductor watch run --follow. \
Then act on the first task of the hand-over your start reading opened, the one committed last \
(git log -1 --name-only --format= -- docs/handoff/), without reading it again, then run one \
cycle.";

/// The first prompt of conductor-dev's session (`session start --role conductor-dev`).
pub const DEV_PROMPT: &str = "Start of session. Read your profile's start-of-session list in its \
order, beginning with the newest hand-over in the conductor-dev handoff directory of the \
instance's state directory (conductor config show prints state), then begin.";

/// The roles `session start` and `session stop` act on, each by its name or its old name
/// (`story:role-name-aliases`).
const SESSION_ROLES: [RoleName; 2] = [RoleName::Director, RoleName::Maintainer];

/// The settings file conductor starts with when its role names none, under the working
/// directory: the product checkout `task conductor:start` runs in.
const CONDUCTOR_SETTINGS: &str = ".claude/conductor-settings.json";

/// The role `role` of the active instance, when it is one `session start` starts: `director` or
/// `maintainer`, or their old names `conductor` and `conductor-dev`, on `claude`
/// (`story:role-name-aliases`). The director is the instance's, under either name; the
/// maintainer is the host's, from the host file under `home`, else, for this release, the one
/// the instance still names under either name (`story:maintainer-host-config`,
/// [`config::host_maintainer`]).
fn session_role(instance: &Instance, home: &Path, role: &str) -> Result<Role, String> {
    let Some(wanted) = config::role_name(role).filter(|wanted| SESSION_ROLES.contains(wanted))
    else {
        return Err(format!(
            "{role:?} is not a role session start starts: write director or maintainer \
             (conductor or conductor-dev, their old names, for this release); a lead starts \
             with conductor spawn"
        ));
    };
    let found = if wanted == RoleName::Maintainer {
        config::host_maintainer(home, instance)
            .map_err(|error| match error.downcast::<config::Invalid>() {
                Ok(invalid) => {
                    let problems: Vec<String> =
                        invalid.problems.iter().map(ToString::to_string).collect();
                    format!(
                        "the host file {} does not read: {}",
                        invalid.file.display(),
                        problems.join("; ")
                    )
                }
                Err(error) => format!("{error:#}"),
            })?
            .ok_or_else(|| {
                format!(
                    "the host names no role {role}: write it in {} (conductor config show, \
                     host.roles)",
                    config::host_file(home).display()
                )
            })?
    } else {
        instance
            .roles
            .iter()
            .find(|named| config::is_role(&named.role, wanted))
            .cloned()
            .ok_or_else(|| format!("the instance names no role {role} in conductor config show"))?
    };
    if found.harness != Harness::Claude {
        return Err(format!(
            "the role {role} does not run on claude (conductor config show); session start \
             starts claude only"
        ));
    }
    Ok(found)
}

/// The session name of `role`: the role's, else the one derived for it.
fn role_session_name(instance: &Instance, role: &Role) -> SessionName {
    role.session_name
        .clone()
        .unwrap_or_else(|| config::session_name(instance, &role.role))
}

/// The live session of `role`, named `name`, as the session list shows it: the one holding
/// `name`, else, for this release, one holding the name under the role's other name, such as
/// `<session_prefix>-conductor` for a role the file writes `director` (`story:role-name-aliases`).
fn live_role_session(role: &Role, name: &SessionName) -> Result<Option<(String, SessionName)>> {
    let names = config::role_name(&role.role).map_or_else(
        || vec![name.0.clone()],
        |wanted| config::session_names_of(wanted, &name.0),
    );
    for candidate in names {
        if let Some(id) = confine::live_session(&candidate)? {
            return Ok(Some((id, SessionName(candidate))));
        }
    }
    Ok(None)
}

/// Writes the note on `role` when `--role` gave a role's old name (`story:role-name-aliases`).
fn note_old_name(command: &str, role: &str) -> Result<()> {
    if let Some(note) = config::old_name_note(role) {
        writeln!(
            io::stderr().lock(),
            "conductor: {command}: note: --role {note}, read as it for this release"
        )
        .context("write the note")?;
    }
    Ok(())
}

/// Where the session of `role` starts: conductor in the instance's records directory,
/// conductor-dev in the working directory.
fn start_directory(instance: &Instance, role: &Role, cwd: &Path) -> PathBuf {
    if config::is_role(&role.role, RoleName::Director) {
        PathBuf::from(&instance.records)
    } else {
        cwd.to_owned()
    }
}

/// The request `role`'s session starts in, from `cwd`, with places resolved against `home`.
fn request(instance: &Instance, role: &Role, cwd: &Path, home: &Path) -> SessionRequest {
    let start = start_directory(instance, role, cwd);
    let root = Path::new(&instance.checkouts.root);
    let repository = start
        .strip_prefix(root)
        .ok()
        .map(|under| under.display().to_string())
        .filter(|under| !under.is_empty());
    SessionRequest {
        role: role.role.clone(),
        session_name: role_session_name(instance, role),
        cwd: start.display().to_string(),
        limits: role.limits.clone(),
        writable: places::resolve(
            &role.writable,
            &Placement {
                home: home.to_owned(),
                tmpdir: confine::tmpdir(),
                start: start.clone(),
                repository,
                trees: PathBuf::from(&instance.checkouts.trees),
                records: PathBuf::from(&instance.records),
                cache: PathBuf::from(&instance.cache),
                harness: role.harness,
                controller: false,
            },
        ),
        network: role.network,
        forge_token_env: role.forge_token_env.clone(),
    }
}

/// The arguments the harness of `role` starts with before its first prompt: its name, its profile
/// (`--append-system-prompt-file`) or else its agent (`--agent`, by default the role's old name,
/// `conductor` or `conductor-dev`, the adapters' file names for this release),
/// its model, bypass permissions, no user settings, and its settings file; conductor's defaults to
/// [`CONDUCTOR_SETTINGS`] under `cwd`.
fn harness_args(role: &Role, name: &SessionName, settings: Option<String>) -> Vec<String> {
    let mut args = vec!["-n".to_owned(), name.0.clone()];
    match (&role.profile, &role.agent) {
        (Some(profile), _) => {
            args.extend(["--append-system-prompt-file".to_owned(), profile.0.clone()]);
        }
        (None, Some(agent)) => args.extend(["--agent".to_owned(), agent.0.clone()]),
        (None, None) => args.extend([
            "--agent".to_owned(),
            config::role_name(&role.role).map_or_else(
                || role.role.clone(),
                |name| config::old_role_word(name).to_owned(),
            ),
        ]),
    }
    args.extend(
        [
            "--model",
            &role.model.0,
            "--permission-mode",
            "bypassPermissions",
            "--setting-sources",
            "project,local",
        ]
        .map(str::to_owned),
    );
    if let Some(settings) = settings {
        args.extend(["--settings".to_owned(), settings]);
    }
    args
}

/// The settings a session of `role`, which names no settings file of its own, starts with under
/// a config file (`story:instance-isolation`): the role's default ([`role_settings`]: for
/// `conductor` [`CONDUCTOR_SETTINGS`] under `cwd`, an empty one for `conductor-dev`) pinned to
/// `instance` ([`config::pinned`]), written to `<state>/sessions/<session name>.settings.json`,
/// whose path it answers. The harness does not pass the environment it is started with on to the
/// session it forks, so only the settings select the instance the session's hooks act for.
///
/// # Errors
///
/// The default is there and is no JSON, or the file cannot be written.
fn write_pinned(
    role: &Role,
    cwd: &Path,
    instance: &str,
    state: &Path,
    name: &SessionName,
) -> Result<String> {
    let base = match role_settings(role, cwd) {
        Some(default) => match fs::read_to_string(&default) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("the settings {default} are no JSON"))?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => serde_json::json!({}),
            Err(error) => {
                return Err(error).with_context(|| format!("read the settings {default}"));
            }
        },
        None => serde_json::json!({}),
    };
    Ok(config::write_pinned(base, instance, state, &name.0)?
        .display()
        .to_string())
}

/// The settings file a session of `role` starts with: its `settings`, else for `director`
/// [`CONDUCTOR_SETTINGS`] under `cwd`, else none.
fn role_settings(role: &Role, cwd: &Path) -> Option<String> {
    role.settings
        .as_ref()
        .map(|settings| settings.0.clone())
        .or_else(|| {
            config::is_role(&role.role, RoleName::Director)
                .then(|| cwd.join(CONDUCTOR_SETTINGS).display().to_string())
        })
}

/// Answers a refusal of `session start`: one line naming its `SessionStartRefusal`, and exit 1.
fn start_refused(answer: &SessionStartRefused) -> Result<ExitCode> {
    let refusal = match answer.refusal {
        SessionStartRefusal::InstanceNotPinned => "InstanceNotPinned",
        SessionStartRefusal::ProviderUnavailable => "ProviderUnavailable",
        SessionStartRefusal::MeasurementShort => "MeasurementShort",
        SessionStartRefusal::UnknownRole => "UnknownRole",
        SessionStartRefusal::SessionRunning => "SessionRunning",
        SessionStartRefusal::LaunchFailed => "LaunchFailed",
    };
    bail!(
        "session start: refused: {refusal}: role {:?}: {}",
        answer.role,
        answer.detail
    )
}

/// Answers a refusal of `session stop`: one line naming its `SessionStopRefusal`, and exit 1.
fn stop_refused(answer: &SessionStopRefused) -> Result<ExitCode> {
    let refusal = match answer.refusal {
        SessionStopRefusal::UnknownRole => "UnknownRole",
        SessionStopRefusal::NotRunning => "NotRunning",
    };
    bail!(
        "session stop: refused: {refusal}: role {:?}: {}",
        answer.role,
        answer.detail
    )
}

/// `conductor session start --role <role>`: starts the session of `conductor` or
/// `conductor-dev` under its session name through the provider the instance's `confinement`
/// names ([`confine`]), delivers its first prompt ([`CONDUCTOR_PROMPT`], [`DEV_PROMPT`]),
/// measures its box and writes the measurement under the state directory. Prints what the
/// harness printed, then one line naming the session started, its provider and its id.
///
/// In order, it refuses: a role that is not `conductor` or `conductor-dev`, not named by the
/// instance (the maintainer: by the host, or a host file that does not read), or not on `claude`
/// (`UnknownRole`); under a config file, a role whose own settings
/// file is not there, is no JSON or pins another instance (`InstanceNotPinned`,
/// `story:instance-isolation`; one that pins no instance starts with a pinned copy,
/// [`config::session_settings`], and a role with none with its default written pinned,
/// [`write_pinned`]); a live session holding the
/// session name (`SessionRunning`); under `strict` a provider not available (`ProviderUnavailable`); a start
/// or first prompt that fails (`LaunchFailed`); under `strict` a box short of the request, which
/// is stopped again (`MeasurementShort`).
///
/// # Errors
///
/// `--role` is missing, the environment or the session list cannot be read, or a refusal above.
pub fn start(
    config_flag: Option<&Path>,
    state: Option<&Path>,
    args: &SessionRoleArgs,
    providers: &dyn Providers,
) -> Result<ExitCode> {
    let input = StartSession {
        role: args.role.clone(),
    };
    let Some(role) = input.role else {
        bail!("session start: --role is required");
    };
    let refuse = |refusal: SessionStartRefusal, detail: String| {
        start_refused(&SessionStartRefused {
            role: role.clone(),
            refusal,
            detail,
        })
    };
    let active = config::active();
    let instance = &active.instance;
    let environment = Environment::from_process().context("session start")?;
    let home = environment.home().context("session start")?;
    let found = match session_role(instance, home, &role) {
        Ok(found) => found,
        Err(detail) => return refuse(SessionStartRefusal::UnknownRole, detail),
    };
    let found = &found;
    // The settings select the instance the session's hooks act for (`story:instance-isolation`):
    // a role's own settings file must not pin another.
    if active.from_file
        && let Some(settings) = &found.settings
        && let Some(problem) = config::pin_refusal(Path::new(&settings.0), &instance.name.0)
    {
        return refuse(
            SessionStartRefusal::InstanceNotPinned,
            format!("its settings {} {problem}", settings.0),
        );
    }
    let name = role_session_name(instance, found);
    if let Some((id, held)) = live_role_session(found, &name).context("session start")? {
        return refuse(
            SessionStartRefusal::SessionRunning,
            format!("the live session {id} holds the session name {}", held.0),
        );
    }
    let chosen = match confine::choose(providers, &instance.confinement) {
        Ok(chosen) => chosen,
        Err(refusal) => return refuse_start(&refuse, refusal),
    };
    let cwd = &environment.cwd;
    let mut env = Vec::new();
    if active.from_file {
        let (file, _) = config::locate(config_flag, &environment).context("session start")?;
        env.push((
            config::INSTANCE_VARIABLE.to_owned(),
            instance.name.0.clone(),
        ));
        env.push((
            config::CONFIG_VARIABLE.to_owned(),
            cwd.join(&file).display().to_string(),
        ));
        register(config_flag, &environment, &cwd.join(&file));
    }
    let state_dir = crate::state::dir(state)?;
    let settings = match (&found.settings, active.from_file) {
        (None, true) => Some(
            write_pinned(found, cwd, &instance.name.0, &state_dir, &name)
                .context("session start")?,
        ),
        (Some(own), true) => Some(
            config::session_settings(Path::new(&own.0), &instance.name.0, &state_dir, &name.0)
                .context("session start")?,
        ),
        _ => role_settings(found, cwd),
    };
    let request = request(instance, found, cwd, home);
    // The session's socket to the instance's broker (`story:tool-broker-read`): registered with
    // its role's grant before it starts, and named in its environment.
    let socket = crate::broker::register_session(
        &state_dir,
        &instance.name,
        found,
        &name,
        Path::new(&request.cwd),
        crate::broker::placement(instance, &found.role, None),
    )
    .context("session start")?;
    env.push((
        conductor_broker::SOCKET_VARIABLE.to_owned(),
        socket.display().to_string(),
    ));
    let launch = Launch {
        request,
        program: confine::CLAUDE.to_owned(),
        args: harness_args(found, &name, settings),
        env,
        session_prefix: instance.session_prefix.clone(),
        instance_limits: instance.confinement.limits.clone(),
        state: state_dir.clone(),
    };
    let prompt = if config::is_role(&found.role, RoleName::Director) {
        CONDUCTOR_PROMPT
    } else {
        DEV_PROMPT
    };
    match chosen.start(&launch, prompt, &state_dir) {
        Ok(started) => {
            confine::pass_through(&started.said)?;
            let StartedSession {
                session_name,
                provider,
                unit,
                session_id,
                ..
            } = &started.session;
            let mut line = format!("started {}: provider {provider}", session_name.0);
            if let Some(unit) = unit {
                line.push_str(&format!(", unit {unit}"));
            }
            if let Some(id) = session_id {
                line.push_str(&format!(", session {id}"));
            }
            if let Some(reason) = &started.measurement.unboxed {
                line.push_str(&format!(", unboxed: {reason}"));
            }
            // A refusal stays one line; the note on an old name goes with the start.
            note_old_name("session start", &role)?;
            writeln!(io::stdout().lock(), "{line}").context("write the started session")?;
            Ok(ExitCode::SUCCESS)
        }
        Err(refusal) => {
            // A session that did not start has no calls to make.
            if let Err(error) = crate::broker::unregister_session(&state_dir, &name) {
                eprintln!("conductor: session start: {error:#}");
            }
            refuse_start(&refuse, refusal)
        }
    }
}

/// Writes the config file `file` that a session starts from to the host registry
/// (`story:instance-names-unique`, [`crate::registry::register`]). A start never fails on it: a
/// file that does not load again, or a registry that cannot be written, is said on standard
/// error and the start goes on.
fn register(config_flag: Option<&Path>, environment: &Environment, file: &Path) {
    let written = environment.home().and_then(|home| {
        let loaded = config::load(config_flag, environment)?;
        crate::registry::register(home, &config::physical(file), &loaded.config)
    });
    if let Err(error) = written {
        eprintln!("conductor: session start: the host registry is not written: {error:#}");
    }
}

/// Answers a refusal of the confinement ([`confine::Refusal`]) as its `SessionStartRefusal`.
fn refuse_start(
    refuse: &dyn Fn(SessionStartRefusal, String) -> Result<ExitCode>,
    refusal: Refusal,
) -> Result<ExitCode> {
    match refusal {
        Refusal::ProviderUnavailable(detail) => {
            refuse(SessionStartRefusal::ProviderUnavailable, detail)
        }
        Refusal::MeasurementShort(detail) => refuse(SessionStartRefusal::MeasurementShort, detail),
        Refusal::LaunchFailed { detail, .. } => refuse(SessionStartRefusal::LaunchFailed, detail),
    }
}

/// `conductor session stop --role <role>`: stops the live session of `conductor` or
/// `conductor-dev` through the provider its measurement names (the `none` provider when there is
/// no measurement), and removes the measurement. Prints one line naming the session stopped.
///
/// It refuses a role `session start` does not start (`UnknownRole`) and a session name no live
/// session holds (`NotRunning`).
///
/// # Errors
///
/// `--role` is missing, the session list cannot be read, the provider does not stop the session,
/// or a refusal above.
pub fn stop(
    state: Option<&Path>,
    args: &SessionRoleArgs,
    providers: &dyn Providers,
) -> Result<ExitCode> {
    let input = StopSession {
        role: args.role.clone(),
    };
    let Some(role) = input.role else {
        bail!("session stop: --role is required");
    };
    let refuse = |refusal: SessionStopRefusal, detail: String| {
        stop_refused(&SessionStopRefused {
            role: role.clone(),
            refusal,
            detail,
        })
    };
    let instance = &config::active().instance;
    let environment = Environment::from_process().context("session stop")?;
    let home = environment.home().context("session stop")?;
    let found = match session_role(instance, home, &role) {
        Ok(found) => found,
        Err(detail) => return refuse(SessionStopRefusal::UnknownRole, detail),
    };
    let found = &found;
    let name = role_session_name(instance, found);
    let Some((id, name)) = live_role_session(found, &name).context("session stop")? else {
        return refuse(
            SessionStopRefusal::NotRunning,
            format!("no live session holds the session name {}", name.0),
        );
    };
    let state_dir = crate::state::dir(state)?;
    let measured = confine::read_measurement(&state_dir, &name).context("session stop")?;
    let (provider, unit) = measured.map_or_else(
        || ("none".to_owned(), None),
        |measurement| (measurement.session.provider, measurement.session.unit),
    );
    let mut stopper = match provider.as_str() {
        "systemd" => providers.provider(ConfinementProvider::Systemd),
        _ => providers.provider(ConfinementProvider::None),
    };
    let request = request(instance, found, &environment.cwd, home);
    let session = StartedSession {
        role: role.clone(),
        session_name: name.clone(),
        provider,
        unit,
        session_id: Some(id.clone()),
    };
    stopper
        .stop(&session, &request)
        .with_context(|| format!("session stop: stop {} ({id})", name.0))?;
    confine::remove_measurement(&state_dir, &name).context("session stop")?;
    crate::broker::unregister_session(&state_dir, &name).context("session stop")?;
    note_old_name("session stop", &role)?;
    writeln!(io::stdout().lock(), "stopped {} ({id})", name.0)
        .context("write the stopped session")?;
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::CONDUCTOR_PROMPT;

    /// `story:conductor-context-budget`: the profile's start reading opens the newest hand-over,
    /// so the first prompt acts on it and does not have it read a second time.
    #[test]
    fn the_first_prompt_does_not_read_the_hand_over_again() {
        assert!(
            !CONDUCTOR_PROMPT.contains("read the hand-over"),
            "{CONDUCTOR_PROMPT}"
        );
        assert!(
            CONDUCTOR_PROMPT.contains("act on the first task of the hand-over"),
            "{CONDUCTOR_PROMPT}"
        );
    }
}
