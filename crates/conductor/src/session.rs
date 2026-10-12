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

use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{Generated, LeaseStorage};
use conductor_model::config::InstanceName;
use conductor_model::obligation::UnmetObligation;
use conductor_model::primitives::Timestamp;
use conductor_model::session::obligations::{ClaimLeaseBehavior, LeasesQuery};
use conductor_model::session::{
    ClaimLease, ClaimLeaseOutcome, LeaseData, LeaseHeld, LeaseSnapshot, LeaseState, LeaseTaken,
    LeaseTakenOver, SessionId,
};
use nix::errno::Errno;
use nix::sys::signal::kill;
use nix::unistd::Pid;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{ClaimLeaseArgs, ViewArgs};
use crate::not_implemented;
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

    let generated = Generated::new(crate::state::open_existing(state)?);
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
