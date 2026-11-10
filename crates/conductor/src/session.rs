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
//! settings (the `claude-code` kind's `conductor-settings.json`): it reads the payload Claude Code
//! sends a SessionStart hook on standard input and takes the session's id from its `session_id`,
//! as the guard reads a PreToolUse payload (`crate::guard`). The payload names no process, so the
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
//! session through its tmux launcher and records the delivery; a session started with
//! `claude --bg`, which has no launcher, takes it through `claude attach` in a pane made for the
//! one delivery (`story:deliver-reaches-bg-session`, [`type_through_attach`]). `session
//! deliveries` is the view
//! `conductor.session.Deliveries` ([`deliveries`]). The [`Deliverer`] trait puts a text in front
//! of a running session; its real form, [`ShellDeliverer`], runs `conductor session deliver
//! --name <name> --text -`, which is how `conductor run` delivers (`story:run-daemon`).
//! `session record-run-tick` and `session run-ticks` are the run daemon's, in [`crate::run`]. The
//! placed `session deliver-message` leaf, a delivery recorded with nothing typed, is not written
//! yet.

use std::fs::{self, File};
use std::io::{self, Read as _, Write as _};
use std::os::fd::AsFd as _;
use std::os::unix::fs::OpenOptionsExt as _;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    DeliveryStorage, ExternalCommand, Generated, LeaseStorage, TryContext, unmet_context,
};
use conductor_model::config::{Harness, Instance, InstanceName, Role, SessionName};
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
use nix::fcntl::OFlag;
use nix::poll::{PollFd, PollFlags, PollTimeout, poll};
use nix::sys::signal::{Signal, kill};
use nix::unistd::Pid;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::agent::claude_code;
use crate::cli::{ClaimLeaseArgs, DeliverArgs, SessionRoleArgs, ViewArgs};
use crate::config::{self, BuiltInRole, ConfinementProvider, Environment};
use crate::confine::places::{self, Placement};
use crate::confine::{self, Launch, Providers, Refusal};
use crate::daemon::host_roles;
use crate::not_implemented;
use crate::records;
use crate::store::Store;
use crate::systemd;

/// The actor of the session `name` of the active instance: an agent in the role whose session
/// name it is, else a controller (`crate::actor::agent`).
#[must_use]
pub fn actor_of(name: &str) -> conductor_model::operator::Actor {
    crate::actor::agent(&config::active().instance, name)
}

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
            actor: crate::actor::chosen(),
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
    const COLUMNS: [&str; 8] = [
        "instance",
        "state",
        "session_id",
        "pid",
        "taken_at",
        "taken_over_session_id",
        "taken_over_pid",
        "actor",
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
                Value::from(crate::actor::written(&row.actor)),
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
/// A session with no socket, or one no tmux server answers on, is a background session's
/// (`claude --bg` starts no launcher, `story:deliver-reaches-bg-session`): when the session list
/// names a live session of that name, the text is typed through `claude attach <id>` in a pane
/// made for this one delivery ([`type_through_attach`]) and recorded the same way.
///
/// A session neither route reaches is refused (`SessionNotReachable`, the specification's
/// `unreachable` branch), exit 1 naming the session and both routes on one line: nothing was
/// kept or recorded, and the command does not try again.
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
    let file = (input.text != "-").then(|| Path::new(&input.text));
    deliver_text(state, &input.name, &text, file)
}

/// [`deliver`] of `text`, already read, to the session `name`: the text was read from `file`,
/// whose path the delivery records, or, when none is given, it is kept as
/// `<state>/mailbox/delivered/<ulid>.md`, as `--text -` is. The `claude-code` kind's `deliver`
/// (`story:agent-kinds`).
///
/// # Errors
///
/// What [`deliver`] answers, but for the flags.
pub fn deliver_text(
    state: Option<&Path>,
    name: &str,
    text: &str,
    file: Option<&Path>,
) -> Result<ExitCode> {
    const WHAT: &str = "session deliver";
    if !crate::message::plain_name(name) {
        bail!("{WHAT}: --name {name:?} names no session: one name, without `/`, not `.` or `..`");
    }
    let first_line = crate::message::first_line(text).to_owned();
    let typed = text.trim_end_matches(['\n', '\r']);
    let dir = crate::state::dir(state)?;
    let sessions = dir.join("sessions");
    let name = SessionName(name.to_owned());
    let reach = reach(&sessions, &name, typed);
    let delivered_at = records::now()?;
    let mut generated = Generated::new(Delivering {
        store: None,
        unreachable: matches!(reach, Reach::Unreached(_)),
        unread: matches!(reach, Reach::Unread(_)),
    });
    let body_path = if !matches!(reach, Reach::Typed) {
        None
    } else {
        let body_path = if let Some(file) = file {
            body_path_of(file, &dir)
        } else {
            let body_path = format!(
                "{}/{DELIVERED}/{}.md",
                crate::message::MAILBOX,
                crate::message::ulid()?
            );
            crate::message::keep(&dir.join(&body_path), text)
                .with_context(|| format!("{WHAT}: keep the text"))?;
            body_path
        };
        generated.ports.store = Some(crate::state::open(Some(&dir))?);
        Some(body_path)
    };
    let outcome = generated
        .deliver_message(DeliverMessage {
            actor: crate::actor::chosen(),
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
            reach.why().unwrap_or("it was not reached")
        ),
        DeliverMessageOutcome::Unread { error } => bail!(
            "{WHAT}: refused: DeliveryNotSeenRead: session {}: {}; nothing was recorded, and it is \
             not offered again, since the session may hold the text",
            error.session_name.0,
            reach
                .why()
                .unwrap_or("the text was typed and not seen to be read")
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

/// How long `claude attach` may take to draw in the pane made for a delivery.
const ATTACH_DRAWS: Duration = Duration::from_secs(20);

/// How long the attach client is given: between its first drawing and the paste, between Enter
/// and the first look at its terminal's input, and, when it took the text only after that look,
/// between that and the end of its pane.
const ATTACH_SETTLE: Duration = Duration::from_millis(500);

/// How long `claude attach` may take, after [`ATTACH_SETTLE`], to read the text typed into its
/// pane.
const ATTACH_READS: Duration = Duration::from_secs(10);

/// How long the session list (`claude agents --json`) may take on the delivery route: not the
/// 120 s of [`confine::BOUND`], which alone is longer than a delivery may take.
const DELIVERY_LIST: Duration = Duration::from_secs(20);

/// What a delivery may take beside the waits its route names: the launcher's try on a socket
/// left behind, about ten tmux calls, the panes other deliveries left, and the record.
const DELIVERY_MARGIN: Duration = Duration::from_secs(15);

/// The directory under `<state>/sessions` that holds the socket of each pane made for one
/// delivery: not the sessions directory itself, where every `*.tmux` is a launcher's socket to
/// the watch ([`crate::watch`]) and to `session attach` ([`crate::attach`]).
const DELIVERY_PANES: &str = "deliveries";

/// What a delivery's pane runs, given the harness's program as `$0`, the session's id and the
/// path of the pane's mark: `claude attach <id>`, then, once that client ended, the mark, and
/// then nothing that reads the terminal. The pane, and with it its terminal, therefore outlives
/// its client until the delivery ends it, so the delivery can still look at what a client that
/// ended left unread: a pane that ended with its client would take that answer with it.
const ATTACH_PANE: &str = r#""$0" attach "$1"; : > "$2"; exec sleep 86400"#;

/// Counts the delivery panes this process made, so two deliveries from one process (the agent
/// API's) name two.
static DELIVERY_PANE: AtomicU32 = AtomicU32::new(0);

/// What became of a text [`reach`] was given.
enum Reach {
    /// It was typed into the session, and, where that is observable, seen to be read.
    Typed,
    /// Nothing was typed, and why: the specification's `unreachable`.
    Unreached(String),
    /// It was typed through the harness's attach and not seen to be read, and why: the
    /// specification's `unread`. The session may hold it.
    Unread(String),
}

impl Reach {
    /// Why the text is not in front of the session, on one line; nothing once it was typed.
    fn why(&self) -> Option<&str> {
        match self {
            Self::Typed => None,
            Self::Unreached(why) | Self::Unread(why) => Some(why),
        }
    }
}

/// Why [`type_through_attach`] did not put the text in front of the session.
enum Untaken {
    /// Before the text was typed: nothing is in the pane.
    NotTyped(anyhow::Error),
    /// After [`systemd::type_into`] answered: the text was typed, then Enter, and the client was
    /// not seen to read it.
    Unread(anyhow::Error),
}

/// The signals that ask a delivery to end, watched while its pane is up
/// (`story:deliver-reaches-bg-session`): the delivery ends its pane and its `claude attach`
/// client first, then ends as the signal ends a process that does not watch it.
struct Asked {
    /// The number of the signal that arrived, or 0.
    signal: Arc<AtomicUsize>,
    hooks: Vec<signal_hook::SigId>,
}

impl Asked {
    /// Watches SIGTERM and SIGINT until it is dropped.
    fn watch() -> Self {
        let signal = Arc::new(AtomicUsize::new(0));
        let hooks = [signal_hook::consts::SIGTERM, signal_hook::consts::SIGINT]
            .into_iter()
            .filter_map(|number| {
                let value = usize::try_from(number).ok()?;
                signal_hook::flag::register_usize(number, Arc::clone(&signal), value).ok()
            })
            .collect();
        Self { signal, hooks }
    }

    /// The signal that arrived, by its number.
    fn arrived(&self) -> Option<i32> {
        i32::try_from(self.signal.load(Ordering::Relaxed))
            .ok()
            .filter(|number| *number != 0)
    }

    /// Fails once a signal arrived.
    fn check(&self) -> Result<()> {
        match self.arrived() {
            Some(number) => bail!("the delivery was asked to end (signal {number})"),
            None => Ok(()),
        }
    }

    /// Sleeps `time`, in steps short enough to notice a signal; fails once one arrived.
    fn pause(&self, time: Duration) -> Result<()> {
        let started = Instant::now();
        loop {
            self.check()?;
            let left = time.saturating_sub(started.elapsed());
            if left.is_zero() {
                return Ok(());
            }
            thread::sleep(left.min(Duration::from_millis(50)));
        }
    }
}

impl Drop for Asked {
    fn drop(&mut self) {
        for hook in self.hooks.drain(..) {
            signal_hook::low_level::unregister(hook);
        }
    }
}

/// Types `typed` into the session `name`, whose files are in `sessions`; answers whether it was
/// typed, and when not, why, on one line.
///
/// First the launcher: the tmux server on `<sessions>/<name>.tmux`. With no socket, or a socket
/// whose tmux does not take the text, the session may be a background one, which has no launcher
/// (`story:deliver-reaches-bg-session`): when the session list names a live session of that
/// name, the text goes through [`type_through_attach`], on a socket of the delivery's own. A
/// socket a tmux server still answers on is a launcher's, and is left as it is; so is a socket
/// file no server answers on, which is not the delivery's to remove.
fn reach(sessions: &Path, name: &SessionName, typed: &str) -> Reach {
    // The text is one of tmux's arguments; the refusal stays one line without it.
    let said = |error: &anyhow::Error| {
        let mut said = format!("{error:#}");
        if !typed.is_empty() {
            said = said.replace(typed, "<the text>");
        }
        said.split_whitespace().collect::<Vec<_>>().join(" ")
    };
    let socket = sessions.join(format!("{}.tmux", name.0));
    let planted = fs::symlink_metadata(&socket).is_ok();
    let launcher = if planted {
        match systemd::type_text(sessions, name, typed) {
            Ok(()) => return Reach::Typed,
            Err(error) => format!(
                "its tmux on {} did not take the text: {}",
                socket.display(),
                said(&error)
            ),
        }
    } else {
        format!("its launcher socket {} is absent", socket.display())
    };
    let list = format!("`{} agents --json`", confine::CLAUDE);
    let background = match confine::live_session_within(&name.0, DELIVERY_LIST) {
        Ok(None) => format!("{list} lists no live session of that name"),
        Err(error) => format!("the session list was not read: {}", said(&error)),
        Ok(Some(id)) if id.starts_with('-') || id.contains(char::is_whitespace) => {
            format!("{list} names its live session by no id to attach to")
        }
        Ok(Some(_))
            if planted && systemd::tmux_call(sessions, name, &["list-sessions"]).is_ok() =>
        {
            format!(
                "{list} lists it live, but a tmux server answers on that socket and is left alone"
            )
        }
        Ok(Some(id)) => match type_through_attach(sessions, name, &id, typed) {
            Ok(()) => return Reach::Typed,
            Err(Untaken::NotTyped(error)) => format!(
                "`{} attach {id}` in a pane of its own did not take it: {}",
                confine::CLAUDE,
                said(&error)
            ),
            Err(Untaken::Unread(error)) => {
                return Reach::Unread(format!(
                    "the text was typed into `{} attach {id}` in a pane of its own, and {}",
                    confine::CLAUDE,
                    said(&error)
                ));
            }
        },
    };
    Reach::Unreached(format!("{launcher}; {background}"))
}

/// Types `typed` into the live background session `id`, named `name`, through the harness's own
/// attach (`story:deliver-reaches-bg-session`): a tmux server of this one delivery, with no
/// configuration file, whose one pane runs `claude attach <id>` ([`ATTACH_PANE`]), 160 by 50;
/// once the attach client has drawn (within [`ATTACH_DRAWS`]) and [`ATTACH_SETTLE`] passed, the
/// launcher's typing ([`systemd::type_into`]); then, once the client has read what was typed
/// ([`seen_read`]), the server is ended and its socket removed, so the pane lives for the one
/// delivery and the session, which the harness keeps running without a client, has no launcher
/// left behind.
///
/// The server's socket is the delivery's own: `<sessions>/deliveries/<process id>-<count>.tmux`,
/// whose length does not grow with the session's name; the name is that of the server's one
/// tmux session. Beside it, `<process id>-<count>.ended` is the mark the pane writes once its
/// client ended. Two deliveries to one session at once therefore share nothing, none touches
/// the session's launcher socket path `<sessions>/<name>.tmux` (a file found there is not this
/// delivery's, and stays), and no reader of launcher sockets lists a delivery's pane as a
/// session. Before it makes its own, a delivery ends each pane a delivery left whose process is
/// gone ([`end_left_panes`]).
///
/// A delivery asked to end while its pane is up (SIGTERM, SIGINT: [`Asked`]) ends the pane
/// first, and then ends as that signal ends a process.
///
/// # Errors
///
/// [`Untaken::NotTyped`]: tmux does not start the pane, `claude attach` draws nothing within its
/// bound or ends before the typing, or the text is not typed. [`Untaken::Unread`]: the text was
/// typed, and the client was not seen to read it.
fn type_through_attach(
    sessions: &Path,
    name: &SessionName,
    id: &str,
    typed: &str,
) -> Result<(), Untaken> {
    let panes = sessions.join(DELIVERY_PANES);
    fs::create_dir_all(&panes)
        .with_context(|| format!("create {}", panes.display()))
        .map_err(Untaken::NotTyped)?;
    end_left_panes(&panes);
    let pane = format!(
        "{}-{}",
        std::process::id(),
        DELIVERY_PANE.fetch_add(1, Ordering::Relaxed)
    );
    let ended = panes.join(format!("{pane}.ended"));
    let asked = Asked::watch();
    let taken = systemd::tmux_on(
        &panes,
        &pane,
        &[
            "-f",
            "/dev/null",
            "new-session",
            "-d",
            "-s",
            &name.0,
            "-x",
            "160",
            "-y",
            "50",
            "--",
            "sh",
            "-c",
            ATTACH_PANE,
            confine::CLAUDE,
            id,
            &ended.display().to_string(),
        ],
    )
    .map_err(Untaken::NotTyped)
    .and_then(|_| type_once_drawn(&panes, &pane, name, typed, &ended, &asked));
    end_pane(&panes, &pane);
    let signal = asked.arrived();
    drop(asked);
    if let Some(signal) = signal {
        // The pane is gone; the process ends as it would have without the watch.
        let _ = signal_hook::low_level::emulate_default_handler(signal);
    }
    taken
}

/// Ends the tmux server of the delivery pane on `<panes>/<pane>.tmux` and removes its socket
/// and its mark.
fn end_pane(panes: &Path, pane: &str) {
    let _ = systemd::tmux_on(panes, pane, &["kill-server"]);
    let _ = fs::remove_file(panes.join(format!("{pane}.tmux")));
    let _ = fs::remove_file(panes.join(format!("{pane}.ended")));
}

/// Ends each delivery pane in `panes` whose delivery is gone: a socket
/// `<process id>-<count>.tmux` whose process does not exist was left by a delivery that was
/// killed before [`type_through_attach`] ended its pane, and its `claude attach` client would
/// run on. A pane whose process exists is another delivery's, and is left alone.
fn end_left_panes(panes: &Path) {
    let Ok(entries) = fs::read_dir(panes) else {
        return;
    };
    for entry in entries.flatten() {
        let file = entry.file_name().to_string_lossy().into_owned();
        let Some(pane) = file.strip_suffix(".tmux") else {
            continue;
        };
        let owner = pane
            .split('-')
            .next()
            .and_then(|pid| pid.parse::<i32>().ok())
            .filter(|pid| *pid > 0);
        if let Some(owner) = owner
            && kill(Pid::from_raw(owner), None) == Err(Errno::ESRCH)
        {
            end_pane(panes, pane);
        }
    }
}

/// [`type_through_attach`]'s typing, into the pane of the tmux session `name` on
/// `<panes>/<pane>.tmux`, whose client's end the file `ended` marks: waits until the pane shows
/// anything (within [`ATTACH_DRAWS`]), then [`ATTACH_SETTLE`], the text, and until the client
/// is seen to have read it ([`seen_read`]). A client that ended before the typing is given
/// nothing.
fn type_once_drawn(
    panes: &Path,
    pane: &str,
    name: &SessionName,
    typed: &str,
    ended: &Path,
    asked: &Asked,
) -> Result<(), Untaken> {
    let gone = || {
        Untaken::NotTyped(anyhow!(
            "it ended before the text was typed, which was therefore not typed"
        ))
    };
    let target = systemd::target(name);
    let started = Instant::now();
    while systemd::tmux_on(panes, pane, &["capture-pane", "-p", "-t", &target])
        .map_err(Untaken::NotTyped)?
        .trim()
        .is_empty()
    {
        if ended.exists() {
            return Err(gone());
        }
        if started.elapsed() >= ATTACH_DRAWS {
            return Err(Untaken::NotTyped(anyhow!(
                "it drew nothing within {} s",
                ATTACH_DRAWS.as_secs()
            )));
        }
        asked
            .pause(Duration::from_millis(100))
            .map_err(Untaken::NotTyped)?;
    }
    asked.pause(ATTACH_SETTLE).map_err(Untaken::NotTyped)?;
    if ended.exists() {
        return Err(gone());
    }
    systemd::type_into(panes, pane, name, typed).map_err(Untaken::NotTyped)?;
    seen_read(panes, pane, &target, ended, asked).map_err(Untaken::Unread)
}

/// What one look at a delivery pane's terminal showed ([`look`]).
enum Seen {
    /// tmux answers and names no terminal: nothing can be said.
    Unobservable,
    /// The pane's server does not answer, or its terminal cannot be opened and polled.
    Closed,
    /// The terminal holds input its client has not read.
    Unread,
    /// The terminal holds no input its client could read.
    Clear,
}

/// Looks at the terminal of the pane `target` on `<panes>/<pane>.tmux`: asks tmux for it, opens
/// it for reading (without it becoming this process's terminal), polls it once and closes it.
fn look(panes: &Path, pane: &str, target: &str) -> Seen {
    let Ok(tty) = systemd::tmux_on(
        panes,
        pane,
        &["display-message", "-p", "-t", target, "#{pane_tty}"],
    ) else {
        return Seen::Closed;
    };
    let tty = tty.trim();
    if tty.is_empty() {
        return Seen::Unobservable;
    }
    let Ok(terminal) = fs::OpenOptions::new()
        .read(true)
        .custom_flags((OFlag::O_NOCTTY | OFlag::O_NONBLOCK).bits())
        .open(tty)
    else {
        return Seen::Closed;
    };
    let mut fds = [PollFd::new(terminal.as_fd(), PollFlags::POLLIN)];
    if poll(&mut fds, PollTimeout::ZERO).is_err() {
        return Seen::Closed;
    }
    let state = fds[0].revents().unwrap_or(PollFlags::empty());
    if state.intersects(PollFlags::POLLHUP | PollFlags::POLLERR) {
        Seen::Closed
    } else if state.contains(PollFlags::POLLIN) {
        Seen::Unread
    } else {
        Seen::Clear
    }
}

/// Waits, after the text and Enter were typed into the pane `target` on `<panes>/<pane>.tmux`,
/// until a look at its terminal ([`look`]) shows no input the client has not read: a client
/// that draws before it reads its terminal has not taken the text yet, and ending its pane then
/// would lose it while the delivery is recorded. Only such a look, taken [`ATTACH_SETTLE`] or
/// more after the typing (the time tmux is given to write what was typed), answers "read"; text
/// the client took only after a look that showed it unread is given [`ATTACH_SETTLE`] more to
/// pass on.
///
/// The pane outlives its client ([`ATTACH_PANE`]), and `ended` marks the client's end, so a
/// client that ended is looked at like one that runs: one that read the text and ended left
/// nothing unread, and one that ended without reading left the text, which nobody reads any
/// more. No look, or none that late, is never taken as read.
///
/// Only where tmux names no terminal at all is the delivery as sure as the launcher's, which
/// types and does not look. A terminal on which Enter ends no line shows no input that could be
/// read, and is taken as read: what a client does not read there is not observable.
///
/// # Errors
///
/// The text was still unread after [`ATTACH_READS`] or when the client had ended, the terminal
/// closed before a look showed it read, or the delivery was asked to end first.
fn seen_read(panes: &Path, pane: &str, target: &str, ended: &Path, asked: &Asked) -> Result<()> {
    let started = Instant::now();
    let mut waited = false;
    loop {
        asked.check()?;
        // Read before the look, so the look is one taken after the client's end.
        let over = ended.exists();
        match look(panes, pane, target) {
            Seen::Unobservable => return asked.pause(ATTACH_SETTLE),
            Seen::Closed => bail!("its terminal closed before it was seen to read the text"),
            Seen::Unread if over => {
                bail!("its client ended with the text unread on its terminal")
            }
            Seen::Unread => waited = true,
            Seen::Clear => {
                if started.elapsed() >= ATTACH_SETTLE {
                    break;
                }
            }
        }
        if started.elapsed() >= ATTACH_SETTLE.saturating_add(ATTACH_READS) {
            bail!(
                "its client did not read the text within {} s",
                ATTACH_READS.as_secs()
            );
        }
        asked.pause(Duration::from_millis(100))?;
    }
    if waited {
        asked.pause(ATTACH_SETTLE)?;
    }
    Ok(())
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
    const COLUMNS: [&str; 7] = [
        "delivery_id",
        "state",
        "session_name",
        "first_line",
        "body_path",
        "delivered_at",
        "actor",
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
                Value::from(crate::actor::written(&row.actor)),
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
/// so a refusal creates none; what became of the text ([`Reach`]), which answers the
/// specification's `unreachable` and `unread` branches; and the delivery's id, assigned here.
struct Delivering {
    store: Option<Store>,
    unreachable: bool,
    unread: bool,
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
                Ok(self.unreachable)
            }
            (ExternalCommand::ConductorSessionDeliverMessage(_), "unread") => Ok(self.unread),
            (command, _) => Err(unmet_context(command.name())),
        }
    }

    // The v1 domains' generated values (`story:spec-v1`): no command this handler runs
    // generates one.

    fn try_generate_conductor_config_revision_id(
        &mut self,
    ) -> Result<conductor_model::config::RevisionId, UnmetObligation> {
        Err(conductor_model::behaviour::unmet_context(
            "conductor.config.RevisionId",
        ))
    }

    fn try_generate_conductor_instance_reconciliation_id(
        &mut self,
    ) -> Result<conductor_model::instance::ReconciliationId, UnmetObligation> {
        Err(conductor_model::behaviour::unmet_context(
            "conductor.instance.ReconciliationId",
        ))
    }

    fn try_generate_conductor_placement_box_id(
        &mut self,
    ) -> Result<conductor_model::placement::BoxId, UnmetObligation> {
        Err(conductor_model::behaviour::unmet_context(
            "conductor.placement.BoxId",
        ))
    }

    fn try_generate_conductor_placement_refusal_reason(
        &mut self,
    ) -> Result<conductor_model::placement::RefusalReason, UnmetObligation> {
        Err(conductor_model::behaviour::unmet_context(
            "conductor.placement.RefusalReason",
        ))
    }

    fn try_generate_conductor_token_token_id(
        &mut self,
    ) -> Result<conductor_model::token::TokenId, UnmetObligation> {
        Err(conductor_model::behaviour::unmet_context(
            "conductor.token.TokenId",
        ))
    }

    fn try_generate_string(&mut self) -> Result<String, UnmetObligation> {
        Err(conductor_model::behaviour::unmet_context("String"))
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
                actor: crate::actor::chosen(),
                instance: input.instance.clone(),
                session_id: input.session_id.clone(),
                pid: input.pid,
                taken_at: input.taken_at,
                taken_over_session_id: None,
                taken_over_pid: None,
            });
            return Ok(ClaimLeaseOutcome::Taken {
                lease_taken: LeaseTaken {
                    actor: crate::actor::chosen(),
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
                    actor: crate::actor::chosen(),
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
            actor: crate::actor::chosen(),
            instance: input.instance.clone(),
            session_id: input.session_id.clone(),
            pid: input.pid,
            taken_at: input.taken_at,
            taken_over_session_id: Some(held.session_id.clone()),
            taken_over_pid: Some(held.pid),
        });
        Ok(ClaimLeaseOutcome::TakenOver {
            lease_taken_over: LeaseTakenOver {
                actor: crate::actor::chosen(),
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

/// The first prompt of a director's session (`session start --role director`), started by hand
/// (`story:run-daemon`): `conductor run` runs the instance's loop, so the session schedules,
/// watches and relays nothing.
pub const DIRECTOR_PROMPT: &str = "Start of session. conductor run runs this instance's loop: \
the cycle, the watch, hand-overs, resumes and relays. Read your profile's start-of-session list, \
then answer the operator.";

/// The first prompt of conductor-dev's session (`session start --role conductor-dev`).
pub const DEV_PROMPT: &str = "Start of session. Read your profile's start-of-session list in its \
order, beginning with the newest hand-over in the conductor-dev handoff directory of the \
instance's state directory (conductor config show prints state), then begin.";

/// The roles `session start` and `session stop` act on, each by its name or its old name
/// (`story:role-name-aliases`).
const SESSION_ROLES: [BuiltInRole; 2] = [BuiltInRole::Director, BuiltInRole::Maintainer];

/// The role `role` of the active instance, when it is one `session start` starts: `director` or
/// `maintainer`, or their old names `conductor` and `conductor-dev`, on `claude`
/// (`story:role-name-aliases`). The director is the instance's, under either name; so is the
/// maintainer, the host's role, until `globals.roles` is read ([`config::host_maintainer`]).
fn session_role(instance: &Instance, role: &str) -> Result<Role, String> {
    let Some(wanted) = config::role_name(role).filter(|wanted| SESSION_ROLES.contains(wanted))
    else {
        return Err(format!(
            "{role:?} is not a role session start starts: write director or maintainer \
             (conductor or conductor-dev, their old names, for this release); a lead starts \
             with conductor spawn"
        ));
    };
    let found = if wanted == BuiltInRole::Maintainer {
        config::host_maintainer(instance)
            .ok_or_else(|| format!("the instance names no role {role} in conductor config show"))?
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

/// The role `role` `session start` and `session stop` act on: a host role, `supervisor` or
/// `teacher`, from `globals.roles` of the config file (`story:host-roles-by-daemon`), with what
/// its session starts with ([`host_roles::Found`]); one that names neither a profile nor an agent
/// starts from the profile the binary carries ([`harness_args`]); else the instance's role
/// [`session_role`] answers.
fn resolve_role(
    instance: &Instance,
    role: &str,
) -> Result<(Role, Option<host_roles::Found>), String> {
    if !host_roles::is_host_role(role) {
        return session_role(instance, role).map(|found| (found, None));
    }
    let found = host_roles::found(role)?;
    if found.role.harness != Harness::Claude {
        return Err(format!(
            "the role {role} does not run on claude (globals.roles); session start starts claude \
             only"
        ));
    }
    Ok((found.role.clone(), Some(found)))
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
    if config::is_role(&role.role, BuiltInRole::Director) {
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
/// (`--append-system-prompt-file`: the role's own, else the one the binary carries, written into
/// the state directory, [`profile_for`]) or else its agent (`--agent`, by default the role's old
/// name: `conductor` for the director, which the binary carries no profile for), its model,
/// the tools it never calls ([`denied_tools_argument`]), bypass permissions, no user settings
/// (the `none` provider adds them for `claude --bg`, [`crate::confine::background_args`]), and
/// its settings file; the director's defaults to the one the binary carries
/// ([`claude_code::settings`]).
fn harness_args(
    role: &Role,
    name: &SessionName,
    profile: Option<String>,
    settings: Option<String>,
) -> Vec<String> {
    let mut args = vec!["-n".to_owned(), name.0.clone()];
    match (profile, &role.agent) {
        (Some(profile), _) => {
            args.extend(["--append-system-prompt-file".to_owned(), profile]);
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
    args.extend(["--model".to_owned(), role.model.0.clone()]);
    if let Some(denied) = denied_tools_argument(&role.role) {
        args.extend(["--disallowedTools".to_owned(), denied]);
    }
    args.extend(
        [
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

/// The value of `--disallowedTools` a session of `role` starts with
/// (`story:conductor-context-budget`): the tools the role never calls
/// ([`claude_code::denied_tools`]) as one comma-separated word, so their definitions leave its
/// first-turn context; none for a role without such a list. A role whose config names its own
/// settings file starts without them too: that file may deny more, never less. The harness reads
/// every word up to the next flag as a tool, so the argument stands before a flag and never last
/// before the prompt.
fn denied_tools_argument(role: &str) -> Option<String> {
    let denied = claude_code::denied_tools(role);
    (!denied.is_empty()).then(|| denied.join(","))
}

/// The profile the session `name` of `role` appends to its system prompt
/// (`story:roles-in-the-binary`): the role's own `profile`, which the harness reads at start;
/// else, for a role that names no agent either, the one the binary carries, written to
/// `<state>/sessions/<name>.profile.md` ([`claude_code::write_profile`]); else none.
///
/// # Errors
///
/// The binary's profile cannot be written.
fn profile_for(role: &Role, state: &Path, name: &SessionName) -> Result<Option<String>> {
    match (&role.profile, &role.agent) {
        (Some(profile), _) => Ok(Some(profile.0.clone())),
        (None, Some(_)) => Ok(None),
        (None, None) => Ok(claude_code::write_profile(state, &name.0, &role.role)?
            .map(|file| file.display().to_string())),
    }
}

/// The settings a session of `role`, which names no settings file of its own, starts with under
/// a config file (`story:instance-isolation`): the role's default ([`claude_code::settings`]: for
/// the director the binary's `conductor-settings.json`, an empty one for every other role) pinned
/// to `instance` ([`config::pinned`]), written to `<state>/sessions/<session name>.settings.json`,
/// whose path it answers. The harness does not pass the environment it is started with on to the
/// session it forks, so only the settings select the instance the session's hooks act for.
///
/// # Errors
///
/// The file cannot be written.
fn write_pinned(role: &Role, instance: &str, state: &Path, name: &SessionName) -> Result<String> {
    let base = claude_code::settings_value(&role.role)?;
    Ok(config::write_pinned(base, instance, state, &name.0)?
        .display()
        .to_string())
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
/// names ([`confine`]), delivers its first prompt ([`DIRECTOR_PROMPT`], [`DEV_PROMPT`]),
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
    let (found, host) = match resolve_role(instance, &role) {
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
    let chosen = match confine::choose(providers, &crate::config::legacy(instance).confinement) {
        Ok(chosen) => chosen,
        Err(refusal) => return refuse_start(&refuse, refusal),
    };
    // A host role's session starts beside the config file (`story:host-roles-by-daemon`).
    let cwd = host.as_ref().map_or(&environment.cwd, |host| &host.dir);
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
    }
    let state_dir = crate::state::dir(state)?;
    let settings = match (&found.settings, active.from_file) {
        (None, true) => Some(
            write_pinned(found, &instance.name.0, &state_dir, &name).context("session start")?,
        ),
        (Some(own), true) => Some(
            config::session_settings(Path::new(&own.0), &instance.name.0, &state_dir, &name.0)
                .context("session start")?,
        ),
        (Some(own), false) => Some(own.0.clone()),
        (None, false) => claude_code::write_settings(&state_dir, &name.0, &found.role)
            .context("session start")?
            .map(|file| file.display().to_string()),
    };
    // The role's profile, else the binary's, written into the state (`story:roles-in-the-binary`).
    let profile = profile_for(found, &state_dir, &name).context("session start")?;
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
    let mut launch = Launch {
        request,
        program: confine::CLAUDE.to_owned(),
        args: harness_args(found, &name, profile, settings),
        env,
        session_prefix: instance.session_prefix.clone(),
        // A host role's slice, `conductor_host.slice`, takes `globals.limits`.
        instance_limits: host.as_ref().map_or_else(
            || crate::config::legacy(instance).confinement.limits.clone(),
            |host| host.limits.clone(),
        ),
        state: state_dir.clone(),
    };
    // story:token-pool: the agent runs on a token leased from the instance's pool, which its
    // settings name; with no token in the pool the launch is today's.
    let lease = match crate::token::lease_into(instance, found, &mut launch, false) {
        Ok(lease) => lease,
        Err(detail) => {
            if let Err(error) = crate::broker::unregister_session(&state_dir, &name) {
                eprintln!("conductor: session start: {error:#}");
            }
            return refuse(SessionStartRefusal::LaunchFailed, detail);
        }
    };
    let prompt = if host.is_some() {
        host_roles::prompt(&found.role)
    } else if config::is_role(&found.role, BuiltInRole::Director) {
        DIRECTOR_PROMPT
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
            if let Some(lease) = &lease {
                line.push_str(&format!(
                    ", token {} of {}",
                    lease.token_id.0.0, lease.owner.0
                ));
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
            if lease.is_some()
                && let Err(error) = crate::token::release_for_stop(instance, &name.0)
            {
                eprintln!("conductor: session start: {error:#}");
            }
            refuse_start(&refuse, refusal)
        }
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
    let (found, host) = match resolve_role(instance, &role) {
        Ok(found) => found,
        Err(detail) => return refuse(SessionStopRefusal::UnknownRole, detail),
    };
    let found = &found;
    let cwd = host.as_ref().map_or(&environment.cwd, |host| &host.dir);
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
    let request = request(instance, found, cwd, home);
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
    // story:token-pool: the token the session ran on is Available again.
    crate::token::release_for_stop(instance, &name.0).context("session stop")?;
    note_old_name("session stop", &role)?;
    writeln!(io::stdout().lock(), "stopped {} ({id})", name.0)
        .context("write the stopped session")?;
    Ok(ExitCode::SUCCESS)
}

/// What a [`Deliverer`] put in front of a session (`story:run-daemon`): the session, by its full
/// name, and the first line of the text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// The session's full name.
    pub session: String,
    /// The first line of the text delivered.
    pub first_line: String,
}

/// Puts a text in front of a running session (docs/design/conductor-run.md § 3): the run
/// daemon's one way to talk to a session. [`ShellDeliverer`] is the real one; a test gives its
/// own.
pub trait Deliverer {
    /// Delivers `text` to the session `name`.
    ///
    /// # Errors
    ///
    /// The text was not delivered: the session is not reachable, or the delivery did not run.
    fn deliver(&self, name: &str, text: &str) -> Result<Delivery>;
}

/// How long one delivery may take before it is stopped: the waits of its longest route, the
/// one through `claude attach` ([`reach`], [`type_through_attach`]), each at its own bound, and
/// [`DELIVERY_MARGIN`]. A delivery stopped earlier than its route allows would be stopped while
/// it types.
pub const DELIVER_BOUND: Duration = DELIVERY_LIST
    .saturating_add(ATTACH_DRAWS)
    .saturating_add(ATTACH_SETTLE)
    .saturating_add(systemd::PASTE_SETTLE)
    .saturating_add(ATTACH_SETTLE)
    .saturating_add(ATTACH_READS)
    .saturating_add(ATTACH_SETTLE)
    .saturating_add(DELIVERY_MARGIN);

/// How long a delivery past [`DELIVER_BOUND`] is given, after SIGTERM, to end its pane before it
/// is killed.
const DELIVER_GRACE: Duration = Duration::from_secs(3);

/// The deliverer that runs `conductor session deliver --name <name> --text -` with the text on
/// standard input (`story:mailbox-send-and-deliver` writes that command: it types the text into
/// the session's tmux, or through `claude attach` for a background session, and records the
/// delivery), with the global `--config` and `--state-dir`
/// the daemon was given, within [`DELIVER_BOUND`]. A session it cannot reach is answered as an
/// error naming it, never retried here.
#[derive(Debug, Clone)]
pub struct ShellDeliverer {
    /// The `conductor` binary: this process's own.
    pub program: PathBuf,
    /// The global `--config`, when the daemon was given one.
    pub config: Option<PathBuf>,
    /// The global `--state-dir`, when the daemon was given one.
    pub state: Option<PathBuf>,
}

impl ShellDeliverer {
    /// The deliverer of this process's binary, with the daemon's `config` and `state` flags.
    ///
    /// # Errors
    ///
    /// The path of this process's binary cannot be read.
    pub fn current(config: Option<&Path>, state: Option<&Path>) -> Result<Self> {
        Ok(Self {
            program: std::env::current_exe().context("read the path of the conductor binary")?,
            config: config.map(Path::to_owned),
            state: state.map(Path::to_owned),
        })
    }
}

impl Deliverer for ShellDeliverer {
    fn deliver(&self, name: &str, text: &str) -> Result<Delivery> {
        use std::process::{Command, Stdio};
        let mut command = Command::new(&self.program);
        if let Some(config) = &self.config {
            command.arg("--config").arg(config);
        }
        if let Some(state) = &self.state {
            command.arg("--state-dir").arg(state);
        }
        command
            .args(["session", "deliver", "--name", name, "--text", "-"])
            .env(crate::actor::DAEMON_VARIABLE, "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let shown = format!("conductor session deliver --name {name} --text -");
        let mut child = command
            .spawn()
            .with_context(|| format!("start `{shown}`"))?;
        if let Some(mut input) = child.stdin.take() {
            input
                .write_all(text.as_bytes())
                .with_context(|| format!("write the text to `{shown}`"))?;
        }
        let started = std::time::Instant::now();
        while child
            .try_wait()
            .with_context(|| format!("wait for `{shown}`"))?
            .is_none()
        {
            if started.elapsed() >= DELIVER_BOUND {
                // Asked first, so it ends its pane (`type_through_attach`); killed after.
                if let Ok(pid) = i32::try_from(child.id()) {
                    let _ = kill(Pid::from_raw(pid), Signal::SIGTERM);
                }
                let asked = std::time::Instant::now();
                while asked.elapsed() < DELIVER_GRACE && matches!(child.try_wait(), Ok(None)) {
                    thread::sleep(Duration::from_millis(50));
                }
                let _ = child.kill();
                let _ = child.wait();
                bail!(
                    "`{shown}` gave no answer within {} s",
                    DELIVER_BOUND.as_secs()
                );
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        let output = child
            .wait_with_output()
            .with_context(|| format!("read what `{shown}` said"))?;
        if !output.status.success() {
            bail!("`{shown}` {}", confine::exited(&output));
        }
        Ok(Delivery {
            session: name.to_owned(),
            first_line: text.lines().next().unwrap_or_default().to_owned(),
        })
    }
}

#[cfg(test)]
mod start_arguments {
    use super::denied_tools_argument;
    use crate::agent::claude_code;

    /// `story:conductor-context-budget`: the director starts without the seven tools it never
    /// calls, the list its settings deny, under either name of its role; the maintainer without
    /// five of them; every other role with every tool.
    #[test]
    fn the_director_and_the_maintainer_start_without_the_tools_they_never_call() {
        let director = "Artifact,Workflow,ScheduleWakeup,EnterWorktree,AskUserQuestion,\
                        ReportFindings,ShareOnboardingGuide";
        let maintainer =
            "Artifact,ScheduleWakeup,AskUserQuestion,ReportFindings,ShareOnboardingGuide";
        for (role, denied) in [
            ("director", Some(director)),
            ("conductor", Some(director)),
            ("maintainer", Some(maintainer)),
            ("conductor-dev", Some(maintainer)),
            ("lead", None),
            ("controller", None),
            ("supervisor", None),
            ("teacher", None),
        ] {
            assert_eq!(denied_tools_argument(role).as_deref(), denied, "{role}");
        }
        let settings: serde_json::Value =
            serde_json::from_str(claude_code::CONDUCTOR_SETTINGS).expect("the settings are JSON");
        assert_eq!(
            settings["permissions"]["deny"],
            serde_json::json!(claude_code::DIRECTOR_DENIED_TOOLS),
            "the director's settings deny the same tools"
        );
    }
}

#[cfg(test)]
mod delivery_bounds {
    use std::time::Duration;

    use super::{
        ATTACH_DRAWS, ATTACH_READS, ATTACH_SETTLE, DELIVER_BOUND, DELIVERY_LIST, DELIVERY_MARGIN,
    };
    use crate::systemd::PASTE_SETTLE;

    /// The deliverer's bound is no shorter than the attach route's own worst case: the session
    /// list, the wait for the client to draw, the settle before the paste, the paste's own
    /// settle before Enter, the settle and the wait for the client to read, and the settle
    /// after a late read; and it leaves a margin of ten seconds at least beyond them.
    #[test]
    fn the_deliverer_s_bound_covers_the_attach_route_s_waits_and_a_margin() {
        let route: Duration = [
            DELIVERY_LIST,
            ATTACH_DRAWS,
            ATTACH_SETTLE,
            PASTE_SETTLE,
            ATTACH_SETTLE,
            ATTACH_READS,
            ATTACH_SETTLE,
        ]
        .into_iter()
        .sum();
        assert_eq!(DELIVER_BOUND, route + DELIVERY_MARGIN);
        assert!(DELIVERY_MARGIN >= Duration::from_secs(10), "a margin");
        assert!(
            DELIVERY_LIST < crate::confine::BOUND,
            "the list's bound on this route is its own"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::DIRECTOR_PROMPT;

    /// `story:run-daemon`: `conductor run` runs the loop, so a director's first prompt arms no
    /// timer, starts no watch and asks for no restart; and, as before
    /// (`story:conductor-context-budget`), it has no hand-over read a second time.
    #[test]
    fn the_director_s_first_prompt_arms_nothing_the_daemon_runs() {
        for gone in [
            "CronCreate",
            "CronList",
            "Monitor",
            "watch run",
            "[RESTART",
            "read the hand-over",
        ] {
            assert!(!DIRECTOR_PROMPT.contains(gone), "{gone}: {DIRECTOR_PROMPT}");
        }
        assert!(
            DIRECTOR_PROMPT.contains("conductor run runs this instance's loop"),
            "{DIRECTOR_PROMPT}"
        );
    }
}
