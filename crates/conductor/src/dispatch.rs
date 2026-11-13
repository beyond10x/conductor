//! `conductor dispatch`: briefs to controllers and their reports (design § 4, § 6).
//!
//! A command reads its input from its flags, opens the store under the state directory
//! (`--state-dir`, else the instance's `state` when a config file names it, else `state/`;
//! [`crate::state::open`]), runs the generated behaviour, and checks that the store kept what the
//! behaviour reported ([`Store::check`]) before it answers. An
//! accepted command exits 0; `send-dispatch` prints the dispatch's id on one line. A refused
//! command exits 1 with one line on standard error: the command, the error the specification
//! declares, and what it means. A flag left out is named on standard error (exit 1) before any
//! store is opened. `--input-json` is not read yet, and answers
//! [`NotImplemented`](crate::NotImplemented).
//!
//! `send-dispatch` without `--dispatch-id` allocates `DSP-YYYYMMDD-NN`: the UTC date of
//! `--sent-at`, and one more than the highest number the store holds for that date, at least two
//! digits. A `--sent-at` whose UTC year is outside 1 to 9999 is refused. An id given by hand is
//! in that form ([`allocator_form`](crate::decide::allocator_form)), or it is refused naming the
//! form; one a dispatch already has is refused (`DispatchExists`), naming it, and nothing is
//! written. The command refuses a `--repository` that neither the newest complete snapshot nor
//! the workspace file names, naming those they name. When another writer takes an allocated id
//! first, the command allocates again, while the date's highest number moves and at most
//! [`ATTEMPTS`] times in all.
//!
//! Once the store kept it, `send-dispatch`, each `report-*` and `cancel-dispatch` append one line
//! to the records' dispatch log ([`crate::records`]): `sent`, `started`, `blocked`, `unblocked`,
//! `done`, `failed`, `cancelled`, with the dispatch's id and its repository as `repo`.
//!
//! The view opens only an existing store ([`crate::state::open_reader`]) and creates nothing. It
//! lists every row it can read, in the order the dispatches were first stored, and answers through
//! [`crate::store::show`].

use std::collections::BTreeSet;
use std::fs::{File, OpenOptions};
use std::io::Write as _;
use std::path::Path;
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{
    DispatchStorage, Generated, RepositoryObservationStorage, SnapshotStorage,
};
use conductor_model::direction::GoalId;
use conductor_model::dispatch::obligations::{
    CancelDispatchBehavior, DispatchesQuery, RecordDispatchLinkBehavior, ReportBlockedBehavior,
    ReportDoneBehavior, ReportFailedBehavior, ReportStartedBehavior, ReportUnblockedBehavior,
    SendDispatchBehavior,
};
use conductor_model::dispatch::{
    CancelDispatch, CancelDispatchOutcome, DispatchId, DispatchLogEvent, DispatchLogLine,
    DispatchState, RecordDispatchLink, RecordDispatchLinkOutcome, ReportBlocked,
    ReportBlockedOutcome, ReportDone, ReportDoneOutcome, ReportFailed, ReportFailedOutcome,
    ReportStarted, ReportStartedOutcome, ReportUnblocked, ReportUnblockedOutcome, SendDispatch,
    SendDispatchOutcome,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::RepositoryName;
use conductor_model::primitives::Timestamp;
use serde_json::Value;

use crate::cli::{
    CancelDispatchArgs, JsonInput, RecordDispatchLinkArgs, ReportBlockedArgs, ReportDoneArgs,
    ReportFailedArgs, ReportStartedArgs, ReportUnblockedArgs, SendDispatchArgs, ViewArgs,
};
use crate::decide::{ATTEMPTS, highest, in_allocator_form, next_id, utc_day};
use crate::not_implemented;
use crate::records;
use crate::store::{Store, StoreError};

pub mod behaviour;

pub use behaviour::{
    ISSUE_MOST, LINK_MOST, LINKS_MOST, REFERENCE_MOST, START_REF_MOST, refused_member, valid_issue,
    valid_link, valid_reference, valid_start_ref,
};

/// The prefix `conductor.dispatch.DispatchId` declares (`spec/domains/dispatch.yaml`).
const DISPATCH_PREFIX: &str = "DSP-";

/// `conductor.dispatch.Dispatch`, as the store names the entity in a [`StoreError`].
const DISPATCH: &str = "conductor.dispatch.Dispatch";

/// `conductor dispatch send-dispatch`: the command `conductor.dispatch.SendDispatch`. Prints the
/// dispatch's id: the one given, or the one allocated.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)),
/// `--dispatch-id` is not in the allocator's form or names a dispatch that exists, `--sent-at`
/// falls outside the UTC years 1 to 9999, `--repository` is one neither the newest complete
/// snapshot nor the workspace names, the store does not open or keep the dispatch,
/// another writer took every allocated id, `--start-ref` or `--issue` is not one a dispatch
/// carries (`InvalidWorkRequest`, [`refused_member`]), or the specification refuses it:
/// `InvalidPriority`, `WaitsOnItself`, `GoalNotFound`, `GoalClosed`.
pub fn send_dispatch(state: Option<&Path>, args: SendDispatchArgs) -> Result<ExitCode> {
    let id = send_to(state, args, true)?;
    print_line("dispatch send-dispatch", &id)
}

/// The open dispatches of one repository: those `Sent`, `Started` or `Blocked`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Open {
    /// None is open.
    None,
    /// The one open, or among several the one started or blocked.
    One(String),
    /// Several, and not one alone started or blocked: their ids.
    Several(Vec<String>),
}

/// The open dispatch of `repository`: the one sent, started or blocked, or among several the one
/// started or blocked. A report names it with `-`.
///
/// # Errors
///
/// The store does not open or a dispatch does not read.
pub fn open_of(state: Option<&Path>, repository: &str) -> Result<Open> {
    let store = crate::state::open(state)?;
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
    kept("the open dispatches", &store)?;
    let working: Vec<&String> = open
        .iter()
        .filter(|(_, state)| *state != DispatchState::Sent)
        .map(|(id, _)| id)
        .collect();
    Ok(match (open.as_slice(), working.as_slice()) {
        ([], _) => Open::None,
        ([(one, _)], _) => Open::One(one.clone()),
        (_, [one]) => Open::One((*one).clone()),
        _ => Open::Several(open.into_iter().map(|(id, _)| id).collect()),
    })
}

/// `dispatch send-dispatch` without its line on standard output: the dispatch's id, for a caller
/// that sends one itself, as `conductor run` does for a controller it starts.
///
/// # Errors
///
/// As [`send_dispatch`], but for the repository: it is sent to whether or not anything names it.
pub fn send(state: Option<&Path>, args: SendDispatchArgs) -> Result<String> {
    send_to(state, args, false)
}

/// What a send answered ([`send_as`]): the dispatch it created, the one that holds the reference,
/// or the specification's refusal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sent {
    /// A dispatch was created: its id.
    Created(String),
    /// A dispatch already holds the reference (`ReferenceHeld`): its id; nothing was created.
    Held(String),
    /// The specification refused it: the error's name in `conductor.dispatch`, its one line, and
    /// for `RepositoryBusy` the open dispatches of the repository.
    Refused {
        error: &'static str,
        detail: String,
        open: Vec<String>,
    },
}

/// [`send`]; with `known_only`, a repository nothing names is refused first ([`named`]).
fn send_to(state: Option<&Path>, args: SendDispatchArgs, known_only: bool) -> Result<String> {
    match send_as(state, args, known_only, crate::actor::chosen())? {
        Sent::Created(id) | Sent::Held(id) => Ok(id),
        Sent::Refused { error, detail, .. } => refused("dispatch send-dispatch", error, &detail),
    }
}

/// One unit of work a work source hands the instance (`conductor.dispatch.WorkRequest`), sent as
/// `dispatch send-dispatch --reference` sends it, as `actor`: no goal, the priority the command
/// defaults to, sent now. The repository is not checked here: the work call refuses one the
/// instance does not name before it sends ([`named_repositories`]).
///
/// # Errors
///
/// As [`send`].
pub fn send_work(
    state: Option<&Path>,
    work: &conductor_model::dispatch::WorkRequest,
    actor: conductor_model::operator::Actor,
) -> Result<Sent> {
    let args = SendDispatchArgs {
        dispatch_id: None,
        repository: Some(work.repository.0.clone()),
        goal_id: None,
        brief: Some(work.brief.clone()),
        sent_at: Some(records::now()?.0),
        priority: None,
        waits_on: None,
        reference: Some(work.reference.clone()),
        start_ref: work.start_ref.clone(),
        issue: work.issue.clone(),
        actor: None,
        input_json: None,
    };
    send_as(state, args, false, actor)
}

/// The dispatch that holds `reference`, if any: its id.
///
/// # Errors
///
/// The store does not open or a dispatch does not read.
pub fn held_by(state: Option<&Path>, reference: &str) -> Result<Option<String>> {
    let store = crate::state::open(state)?;
    let held = DispatchStorage::list(&store)
        .into_iter()
        .find(|held| held.data.reference.as_deref() == Some(reference))
        .map(|held| held.data.dispatch_id.0);
    kept("the dispatch of a reference", &store)?;
    Ok(held)
}

/// The state of the unit of work `reference` names (`conductor.dispatch.WorkState`), or none when
/// no dispatch holds it or there is no store. The dispatch gives its own fields; the records'
/// dispatch log under `records` its times, its detail and, while it is `Sent`, why no controller
/// was started for it (its last `start-refused` line, unless a `sent` line of `conductor spawn`
/// for its repository follows it), read only in the month files from the month of its `sent_at`
/// on and only from lines of its repository; and the latest `[REPORT]` message handled for it
/// ([`MessageStorage`]) its last report.
///
/// # Errors
///
/// The store does not open or read, or the log's directory does not read.
pub fn work_state(
    state: Option<&Path>,
    records: &Path,
    reference: &str,
) -> Result<Option<conductor_model::dispatch::WorkState>> {
    use conductor_model::behaviour::MessageStorage;
    use conductor_model::dispatch::{MessageKind, MessageState, WorkState};

    let dir = crate::state::dir(state)?;
    // A view creates nothing: with no store, no dispatch holds the reference.
    if !crate::store::present(&dir) {
        return Ok(None);
    }
    let store = crate::store::open_reader(&dir)?;
    let Some(held) = DispatchStorage::list(&store)
        .into_iter()
        .find(|held| held.data.reference.as_deref() == Some(reference))
    else {
        kept("the work call", &store)?;
        return Ok(None);
    };
    let id = held.data.dispatch_id.clone();
    let last_report = MessageStorage::list(&store)
        .into_iter()
        .filter(|message| {
            message.state == MessageState::Handled
                && message.data.kind == MessageKind::Report
                && message.data.dispatch_id.as_ref() == Some(&id)
        })
        .max_by(|a, b| a.data.received_at.cmp(&b.data.received_at))
        .map(|message| message.data.first_line);
    kept("the work call", &store)?;

    let (mut started_at, mut ended_at, mut reason) = (None, None, None);
    let mut start = None;
    for line in dispatch_lines(records, &held.data.sent_at)? {
        if line["repo"] != held.data.repository.0.as_str() {
            continue;
        }
        // A controller of the repository started after a refused start: one works it now.
        if line["event"] == "sent" && line["controller_id"].is_string() {
            start = None;
        }
        if line["id"] != id.0.as_str() {
            continue;
        }
        if line["event"] == "start-refused" {
            start = line["reason"].as_str().and_then(work_start_refusal);
        }
        let at = line["at"].as_str().map(|at| Timestamp(at.to_owned()));
        match line["event"].as_str() {
            Some("started") if started_at.is_none() => started_at = at,
            Some("done" | "cancelled") => ended_at = at,
            Some("failed") => {
                ended_at = at;
                reason = line["reason"].as_str().map(str::to_owned);
            }
            Some("blocked") => reason = line["reason"].as_str().map(str::to_owned),
            _ => {}
        }
    }
    let detail = matches!(held.state, DispatchState::Blocked | DispatchState::Failed)
        .then_some(reason)
        .flatten();
    Ok(Some(WorkState {
        reference: reference.to_owned(),
        dispatch_id: id,
        repository: held.data.repository,
        state: held.state,
        sent_at: held.data.sent_at,
        started_at,
        ended_at,
        last_report,
        detail,
        links: held.data.links,
        start: (held.state == DispatchState::Sent)
            .then_some(start)
            .flatten(),
    }))
}

/// The `conductor.dispatch.WorkStartRefusal` `name` spells, if any.
fn work_start_refusal(name: &str) -> Option<conductor_model::dispatch::WorkStartRefusal> {
    use conductor_model::server::{json as wire_json, wire};
    let value = wire_json::parse(&serde_json::to_string(name).ok()?).ok()?;
    wire::decode_conductor_dispatch_work_start_refusal(&value, "").ok()
}

/// The lines of the records' dispatch log under `records` in its month files from the month of
/// `since` on, in file and line order; a line that is no JSON object is passed over.
fn dispatch_lines(records: &Path, since: &Timestamp) -> Result<Vec<Value>> {
    let dir = records.join(records::DISPATCHES);
    let first = since.0.get(..7).unwrap_or_default();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok(Vec::new());
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| {
            name.strip_suffix(".jsonl")
                .is_some_and(|month| month.len() == 7 && month >= first)
        })
        .collect();
    names.sort();
    let mut lines = Vec::new();
    for name in names {
        let path = dir.join(&name);
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read the dispatch log {}", path.display()))?;
        lines.extend(
            text.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .filter(Value::is_object),
        );
    }
    Ok(lines)
}

/// [`send_to`] as `actor`, its refusals answered as [`Sent::Refused`].
fn send_as(
    state: Option<&Path>,
    args: SendDispatchArgs,
    known_only: bool,
    actor: conductor_model::operator::Actor,
) -> Result<Sent> {
    const COMMAND: &str = "dispatch send-dispatch";
    flags_only(args.input_json, "dispatch send-dispatch --input-json")?;
    let repository = RepositoryName(flag(COMMAND, "--repository", args.repository)?);
    let goal_id = args.goal_id.map(GoalId);
    let brief = flag(COMMAND, "--brief", args.brief)?;
    let sent_at = flag(COMMAND, "--sent-at", args.sent_at)?;
    // Lower runs first; a dispatch sent without one runs first (design § 3).
    let priority = args.priority.unwrap_or(0);
    let waits_on = args.waits_on.map(DispatchId);
    let (reference, start_ref, issue) = (args.reference, args.start_ref, args.issue);
    // The members a work source names, held to what the work call takes before any store is
    // read, so the command line and the route carry the same values.
    if let Some((member, rule)) = refused_member(start_ref.as_deref(), issue.as_deref()) {
        return Ok(refusal(
            "InvalidWorkRequest",
            format!(
                "--{} is not taken: {rule}; nothing was sent",
                member.replace('_', "-")
            ),
        ));
    }
    // A flag check, so it opens no store; the behaviour refuses the same input when another caller
    // sends it.
    if goal_id.is_none() && reference.is_none() {
        return Ok(neither(&repository));
    }
    let day = utc_day(&sent_at).with_context(|| format!("{COMMAND}: --sent-at {sent_at:?}"))?;
    if let Some(given) = &args.dispatch_id {
        in_allocator_form(COMMAND, "--dispatch-id", DISPATCH_PREFIX, given)?;
    }
    let given = args.dispatch_id.map(DispatchId);
    let prefix = format!("{DISPATCH_PREFIX}{day}-");
    if known_only {
        named(COMMAND, state, &repository.0)?;
    }
    // A reference is held by one dispatch, and a repository with an open dispatch takes none with
    // a reference: the check and the send are one step under the lock, however many sends race.
    let _lock = lock(state)?;

    let mut before = None;
    for _ in 0..ATTEMPTS {
        // A fresh handle per attempt: a handle refuses every write after its first failure.
        let store = crate::state::open(state)?;
        let dispatch_id = match &given {
            Some(id) => id.clone(),
            None => {
                let highest = stored_highest(COMMAND, &store, &prefix)?;
                if before.replace(highest) == Some(highest) {
                    bail!(
                        "{COMMAND}: the id allocated above {prefix}{highest:02} was taken, yet \
                         the highest number stored for {day} did not move; nothing was sent"
                    );
                }
                DispatchId(next_id(COMMAND, &prefix, highest)?)
            }
        };
        // The behaviour reads the id before it writes (`already-sent`), so the store creates the
        // dispatch only where no stream is, and answers `StoreError::Exists` when another writer
        // created it in between.
        let mut generated = Generated::new(store);
        let outcome = generated
            .send_dispatch(SendDispatch {
                actor: actor.clone(),
                dispatch_id: dispatch_id.clone(),
                repository: repository.clone(),
                goal_id: goal_id.clone(),
                brief: brief.clone(),
                sent_at: Timestamp(sent_at.clone()),
                priority,
                waits_on: waits_on.clone(),
                reference: reference.clone(),
                start_ref: start_ref.clone(),
                issue: issue.clone(),
            })
            .map_err(|unmet| anyhow!("{COMMAND}: {unmet}"))?;
        let held = match generated.ports.check() {
            Ok(()) => matches!(outcome, SendDispatchOutcome::AlreadySent { .. }),
            Err(StoreError::Exists { entity, identity })
                if entity == DISPATCH && identity == dispatch_id.0 =>
            {
                true
            }
            Err(failure) => {
                return Err(failure)
                    .with_context(|| format!("{COMMAND}: the store did not keep it"));
            }
        };
        if held {
            match &given {
                Some(id) => return exists(COMMAND, id),
                // Another writer sent the allocated id first: allocate again.
                None => continue,
            }
        }
        return match outcome {
            SendDispatchOutcome::Sent { dispatch_sent }
            | SendDispatchOutcome::SentForReference { dispatch_sent } => {
                let mut line = records::dispatch_line(DispatchLogEvent::Sent, &repository.0)?;
                line.id = Some(dispatch_sent.dispatch_id.0.clone());
                line.to = Some(
                    crate::config::session_name(&crate::config::active().instance, &repository.0).0,
                );
                line.goal = goal_id.as_ref().map(|goal| goal.0.clone());
                line.brief = Some(brief.clone());
                line.priority = Some(priority);
                line.waits_on = waits_on.as_ref().map(|waits_on| waits_on.0.clone());
                line.reference.clone_from(&reference);
                line.start_ref.clone_from(&start_ref);
                line.issue.clone_from(&issue);
                records::append_dispatch(&line).with_context(|| {
                    format!("{COMMAND}: dispatch {}", dispatch_sent.dispatch_id.0)
                })?;
                Ok(Sent::Created(dispatch_sent.dispatch_id.0))
            }
            SendDispatchOutcome::ReferenceHeld { error } => Ok(Sent::Held(holder(
                COMMAND,
                &generated.ports,
                &error.reference,
            )?)),
            SendDispatchOutcome::AlreadySent { error } => exists(COMMAND, &error.dispatch_id),
            SendDispatchOutcome::NegativePriority { error } => Ok(refusal(
                "InvalidPriority",
                format!(
                    "the priority {} is negative, so nothing was sent",
                    error.priority
                ),
            )),
            SendDispatchOutcome::WaitsOnItself { error } => Ok(refusal(
                "WaitsOnItself",
                format!(
                    "dispatch {} names itself in --waits-on, so it could never start; nothing \
                     was sent",
                    error.dispatch_id.0
                ),
            )),
            SendDispatchOutcome::InvalidReference { error } => Ok(refusal(
                "InvalidReference",
                format!(
                    "--reference {:?} is not 1 to {REFERENCE_MOST} characters of A-Z a-z 0-9 . _ - \
                     led by a letter or a digit, so nothing was sent",
                    error.reference
                ),
            )),
            SendDispatchOutcome::GoalOrReferenceMissing { error } => Ok(neither(&error.repository)),
            SendDispatchOutcome::NoSuchGoal { error } => Ok(refusal(
                "GoalNotFound",
                format!(
                    "no goal has the id {:?}, and a dispatch that names a goal serves one; \
                     nothing was sent",
                    error.goal_id.0
                ),
            )),
            SendDispatchOutcome::GoalClosed { error } => Ok(refusal(
                "GoalClosed",
                format!(
                    "goal {:?} is met or dropped, so nothing was sent",
                    error.goal_id.0
                ),
            )),
            SendDispatchOutcome::RepositoryBusy { error } => {
                let open = open_ids(COMMAND, &generated.ports, &error.repository)?;
                Ok(Sent::Refused {
                    error: "RepositoryBusy",
                    detail: format!(
                        "{} has the open dispatch {}, and a dispatch with a reference waits \
                         until it ends; nothing was sent",
                        error.repository.0,
                        open.join(", ")
                    ),
                    open,
                })
            }
        };
    }
    bail!(
        "{COMMAND}: another writer took each of the {ATTEMPTS} ids allocated for {day}; nothing \
         was sent"
    )
}

/// The refusal `error` with its one line, `detail`.
const fn refusal(error: &'static str, detail: String) -> Sent {
    Sent::Refused {
        error,
        detail,
        open: Vec::new(),
    }
}

/// The `GoalOrReferenceMissing` refusal of a dispatch to `repository`.
fn neither(repository: &RepositoryName) -> Sent {
    refusal(
        "GoalOrReferenceMissing",
        format!(
            "the dispatch to {} names neither --goal-id nor --reference, and a dispatch serves a \
             goal or carries a work source's reference; nothing was sent",
            repository.0
        ),
    )
}

/// The lock in the state directory `send-dispatch` holds from before it reads the dispatches
/// until its dispatch is kept: the check that no dispatch holds the reference, or keeps the
/// repository busy, and the send that records one are then one step. The store's own writer lock
/// is taken by each commit and cannot be held across one, so this lock, as the controllers'
/// (`controllers.lock`), is held instead.
const LOCK: &str = "dispatches.lock";

/// Opens the store, creating it when it is not there, then takes the dispatches' lock ([`LOCK`]),
/// held until the answer is dropped.
fn lock(state: Option<&Path>) -> Result<File> {
    let dir = crate::state::dir(state)?;
    // The store is opened first, so a state directory it refuses (one `conductor store migrate`
    // is to move) is left as it was, and the directory the lock is made in exists.
    drop(crate::store::open(&dir)?);
    let path = dir.join(LOCK);
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&path)
        .with_context(|| format!("open the dispatches' lock {}", path.display()))?;
    lock.lock()
        .with_context(|| format!("lock the dispatches {}", path.display()))?;
    Ok(lock)
}

/// The id of the dispatch that holds `reference` (`ReferenceHeld`): what `send-dispatch` answers
/// when the reference is sent again.
fn holder(command: &str, store: &Store, reference: &str) -> Result<String> {
    let held = DispatchStorage::list(store)
        .into_iter()
        .find(|held| held.data.reference.as_deref() == Some(reference))
        .map(|held| held.data.dispatch_id.0);
    kept(command, store)?;
    held.ok_or_else(|| {
        anyhow!("{command}: --reference {reference}: held by a dispatch, yet none reads back")
    })
}

/// The ids of the open dispatches of `repository`, for a `RepositoryBusy` refusal.
fn open_ids(command: &str, store: &Store, repository: &RepositoryName) -> Result<Vec<String>> {
    let ids = DispatchStorage::list(store)
        .into_iter()
        .filter(|held| {
            held.data.repository == *repository
                && matches!(
                    held.state,
                    DispatchState::Sent | DispatchState::Started | DispatchState::Blocked
                )
        })
        .map(|held| held.data.dispatch_id.0)
        .collect();
    kept(command, store)?;
    Ok(ids)
}

/// `conductor dispatch record-link`: the command `conductor.dispatch.RecordDispatchLink`, through
/// [`link`]. A link the dispatch holds already changes nothing and writes no line.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the link, or the specification refuses it: `InvalidLink`,
/// `DispatchNotFound`, `DispatchLinksFull`.
pub fn record_link(state: Option<&Path>, args: RecordDispatchLinkArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch record-link";
    flags_only(args.input_json, "dispatch record-link --input-json")?;
    let actor = crate::actor::chosen();
    let dispatch = dispatch_id(COMMAND, args.dispatch_id)?;
    let given = flag(COMMAND, "--link", args.link)?;
    match link(state, dispatch, given, actor)? {
        Linking::Linked | Linking::Held => Ok(ExitCode::SUCCESS),
        Linking::Refused { error, detail } => refused(COMMAND, error, &detail),
    }
}

/// What recording a link on a dispatch answered ([`link`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Linking {
    /// The link is the dispatch's last, and its `linked` line is written.
    Linked,
    /// The dispatch held the link already: nothing changed and no line was written.
    Held,
    /// The specification refused it: the error's name in `conductor.dispatch` and its one line.
    Refused { error: &'static str, detail: String },
}

/// Records `link` on the dispatch `dispatch` as `actor` (`conductor.dispatch.RecordDispatchLink`):
/// the code `dispatch record-link` runs, and the loop's mailbox for a `review <url>` report.
///
/// # Errors
///
/// The store does not open or keep the link, or the records' line is not written.
pub fn link(
    state: Option<&Path>,
    dispatch: DispatchId,
    link: String,
    actor: conductor_model::operator::Actor,
) -> Result<Linking> {
    const COMMAND: &str = "dispatch record-link";
    let input = RecordDispatchLink {
        actor,
        dispatch_id: dispatch,
        link,
    };
    let (outcome, store) = run(state, COMMAND, |generated| {
        generated.record_dispatch_link(input)
    })?;
    Ok(match outcome {
        RecordDispatchLinkOutcome::Linked { dispatch_linked } => {
            let _ = logged(
                COMMAND,
                &store,
                &dispatch_linked.dispatch_id,
                DispatchLogEvent::Linked,
                |line| line.link = Some(dispatch_linked.link.clone()),
            )?;
            Linking::Linked
        }
        RecordDispatchLinkOutcome::AlreadyLinked => Linking::Held,
        RecordDispatchLinkOutcome::InvalidLink { error } => Linking::Refused {
            error: "InvalidLink",
            detail: format!(
                "--link {:?} is not an http:// or https:// URL of at most {LINK_MOST} characters \
                 without a space or control character, so nothing was recorded",
                error.link
            ),
        },
        RecordDispatchLinkOutcome::NoSuchDispatch { error } => Linking::Refused {
            error: "DispatchNotFound",
            detail: format!("no dispatch has the id {}", error.dispatch_id.0),
        },
        RecordDispatchLinkOutcome::LinksFull { error } => Linking::Refused {
            error: "DispatchLinksFull",
            detail: format!(
                "dispatch {} holds {LINKS_MOST} links, the most it holds, so nothing was recorded",
                error.dispatch_id.0
            ),
        },
    })
}

/// `conductor dispatch report-started`: the command `conductor.dispatch.ReportStarted`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_started(state: Option<&Path>, args: ReportStartedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-started";
    flags_only(args.input_json, "dispatch report-started --input-json")?;
    let input = ReportStarted {
        actor: crate::actor::chosen(),
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
    };
    let (outcome, store) = run(state, COMMAND, |generated| generated.report_started(input))?;
    match outcome {
        ReportStartedOutcome::Started { dispatch_started } => logged(
            COMMAND,
            &store,
            &dispatch_started.dispatch_id,
            DispatchLogEvent::Started,
            |_| {},
        ),
        ReportStartedOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportStartedOutcome::WrongState { error } => {
            wrong_state(COMMAND, &error.dispatch_id, "only a sent dispatch starts")
        }
    }
}

/// `conductor dispatch report-blocked`: the command `conductor.dispatch.ReportBlocked`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_blocked(state: Option<&Path>, args: ReportBlockedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-blocked";
    flags_only(args.input_json, "dispatch report-blocked --input-json")?;
    let input = ReportBlocked {
        actor: crate::actor::chosen(),
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let (outcome, store) = run(state, COMMAND, |generated| generated.report_blocked(input))?;
    match outcome {
        ReportBlockedOutcome::Blocked { dispatch_blocked } => logged(
            COMMAND,
            &store,
            &dispatch_blocked.dispatch_id,
            DispatchLogEvent::Blocked,
            |line| line.reason = Some(dispatch_blocked.reason.clone()),
        ),
        ReportBlockedOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportBlockedOutcome::WrongState { error } => wrong_state(
            COMMAND,
            &error.dispatch_id,
            "only a started dispatch blocks",
        ),
    }
}

/// `conductor dispatch report-unblocked`: the command `conductor.dispatch.ReportUnblocked`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_unblocked(state: Option<&Path>, args: ReportUnblockedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-unblocked";
    flags_only(args.input_json, "dispatch report-unblocked --input-json")?;
    let input = ReportUnblocked {
        actor: crate::actor::chosen(),
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
    };
    let (outcome, store) = run(state, COMMAND, |generated| {
        generated.report_unblocked(input)
    })?;
    match outcome {
        ReportUnblockedOutcome::Unblocked { dispatch_unblocked } => logged(
            COMMAND,
            &store,
            &dispatch_unblocked.dispatch_id,
            DispatchLogEvent::Unblocked,
            |_| {},
        ),
        ReportUnblockedOutcome::NoSuchDispatch { error } => {
            no_dispatch(COMMAND, &error.dispatch_id)
        }
        ReportUnblockedOutcome::WrongState { error } => wrong_state(
            COMMAND,
            &error.dispatch_id,
            "only a blocked dispatch is unblocked",
        ),
    }
}

/// `conductor dispatch report-done`: the command `conductor.dispatch.ReportDone`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_done(state: Option<&Path>, args: ReportDoneArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-done";
    flags_only(args.input_json, "dispatch report-done --input-json")?;
    let input = ReportDone {
        actor: crate::actor::chosen(),
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        evidence: flag(COMMAND, "--evidence", args.evidence)?,
    };
    let evidence = input.evidence.clone();
    let (outcome, store) = run(state, COMMAND, |generated| generated.report_done(input))?;
    match outcome {
        ReportDoneOutcome::Done { dispatch_done } => logged(
            COMMAND,
            &store,
            &dispatch_done.dispatch_id,
            DispatchLogEvent::Done,
            |line| line.evidence = Some(dispatch_done.evidence.clone()),
        ),
        ReportDoneOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportDoneOutcome::WrongState { error } => retried(
            COMMAND,
            &store,
            &error.dispatch_id,
            (DispatchState::Done, DispatchLogEvent::Done),
            |line| line.evidence = Some(evidence),
            "only a sent, started or blocked dispatch is done",
        ),
    }
}

/// `conductor dispatch report-failed`: the command `conductor.dispatch.ReportFailed`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn report_failed(state: Option<&Path>, args: ReportFailedArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch report-failed";
    flags_only(args.input_json, "dispatch report-failed --input-json")?;
    let input = ReportFailed {
        actor: crate::actor::chosen(),
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let reason = input.reason.clone();
    let (outcome, store) = run(state, COMMAND, |generated| generated.report_failed(input))?;
    match outcome {
        ReportFailedOutcome::Failed { dispatch_failed } => logged(
            COMMAND,
            &store,
            &dispatch_failed.dispatch_id,
            DispatchLogEvent::Failed,
            |line| line.reason = Some(dispatch_failed.reason.clone()),
        ),
        ReportFailedOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        ReportFailedOutcome::WrongState { error } => retried(
            COMMAND,
            &store,
            &error.dispatch_id,
            (DispatchState::Failed, DispatchLogEvent::Failed),
            |line| line.reason = Some(reason),
            "only a started or blocked dispatch fails",
        ),
    }
}

/// `conductor dispatch cancel-dispatch`: the command `conductor.dispatch.CancelDispatch`.
///
/// # Errors
///
/// A flag is missing, `--input-json` is given ([`NotImplemented`](crate::NotImplemented)), the
/// store does not open or keep the move, or the specification refuses it: `DispatchNotFound`,
/// `DispatchStateConflict`.
pub fn cancel_dispatch(state: Option<&Path>, args: CancelDispatchArgs) -> Result<ExitCode> {
    const COMMAND: &str = "dispatch cancel-dispatch";
    flags_only(args.input_json, "dispatch cancel-dispatch --input-json")?;
    let input = CancelDispatch {
        actor: crate::actor::chosen(),
        dispatch_id: dispatch_id(COMMAND, args.dispatch_id)?,
        reason: flag(COMMAND, "--reason", args.reason)?,
    };
    let reason = input.reason.clone();
    let (outcome, store) = run(state, COMMAND, |generated| generated.cancel_dispatch(input))?;
    match outcome {
        CancelDispatchOutcome::Cancelled { dispatch_cancelled } => logged(
            COMMAND,
            &store,
            &dispatch_cancelled.dispatch_id,
            DispatchLogEvent::Cancelled,
            |line| line.reason = Some(dispatch_cancelled.reason.clone()),
        ),
        CancelDispatchOutcome::NoSuchDispatch { error } => no_dispatch(COMMAND, &error.dispatch_id),
        CancelDispatchOutcome::WrongState { error } => retried(
            COMMAND,
            &store,
            &error.dispatch_id,
            (DispatchState::Cancelled, DispatchLogEvent::Cancelled),
            |line| line.reason = Some(reason),
            "the dispatch already ended",
        ),
    }
}

/// `conductor dispatch dispatches`: the view `conductor.dispatch.Dispatches`, served by the
/// generated query over the existing store under the state directory and rendered as `--format`
/// asks, in the order the dispatches were first stored. `--format jsonl` is the export of the
/// dispatch records; two runs over one store give the same bytes. A view creates no store.
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn dispatches(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/dispatch.yaml`).
    const COLUMNS: [&str; 13] = [
        "dispatch_id",
        "state",
        "repository",
        "goal_id",
        "brief",
        "sent_at",
        "priority",
        "waits_on",
        "actor",
        "reference",
        "start_ref",
        "issue",
        "links",
    ];

    let generated = Generated::new(crate::state::open_reader(state)?);
    let rows = generated
        .dispatches()
        .map_err(|unmet| anyhow!("dispatch dispatches: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.dispatch_id.0),
                Value::from(state_name(row.state)),
                Value::from(row.repository.0),
                Value::from(row.goal_id.map(|goal| goal.0)),
                Value::from(row.brief),
                Value::from(row.sent_at.0),
                Value::from(row.priority),
                Value::from(row.waits_on.map(|waits_on| waits_on.0)),
                Value::from(crate::actor::written(&row.actor)),
                Value::from(row.reference),
                Value::from(row.start_ref),
                Value::from(row.issue),
                Value::from(row.links),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "dispatches",
        &view.render(&COLUMNS, &rows),
    )
}

/// The state's name as the specification spells it. Exhaustive, so a state the specification adds
/// fails to compile here rather than rendering under a guessed name.
fn state_name(state: DispatchState) -> &'static str {
    match state {
        DispatchState::Blocked => "Blocked",
        DispatchState::Cancelled => "Cancelled",
        DispatchState::Done => "Done",
        DispatchState::Failed => "Failed",
        DispatchState::Sent => "Sent",
        DispatchState::Started => "Started",
    }
}

/// Refuses `repository` when neither the newest complete snapshot
/// ([`crate::repository::newest_complete`]) observed it nor the workspace file of the records
/// (`conductor config workspace`; read only with a config file, as the records' logs are) has a
/// member of that name (`story:dispatch-refuses-unknown-repository`): no controller would start
/// for it. The refusal names every repository the two name. Where they name none (a store
/// without a complete snapshot and no workspace file) there is nothing to hold the name
/// against, and it is accepted.
fn named(command: &str, state: Option<&Path>, repository: &str) -> Result<()> {
    let known = named_repositories(state).with_context(|| command.to_owned())?;
    if known.is_empty() || known.contains(repository) {
        return Ok(());
    }
    bail!(
        "{command}: --repository {repository}: neither the newest complete snapshot nor the \
         workspace names it; they name {}; nothing was sent",
        known.into_iter().collect::<Vec<_>>().join(", ")
    )
}

/// The repositories the instance names: those the newest complete snapshot observed, and the
/// members of the records' workspace file (read only with a config file, as the records' logs
/// are).
///
/// # Errors
///
/// The store does not open or read, or the workspace file does not read.
pub fn named_repositories(state: Option<&Path>) -> Result<BTreeSet<String>> {
    let mut known = BTreeSet::new();
    let store = crate::state::open(state)?;
    if let Some(newest) = crate::repository::newest_complete(SnapshotStorage::list(&store)) {
        known.extend(
            RepositoryObservationStorage::list(&store)
                .into_iter()
                .filter(|observation| observation.data.snapshot_id == newest)
                .map(|observation| observation.data.repository.0),
        );
    }
    store.check().context("read the newest complete snapshot")?;
    let active = crate::config::active();
    if active.from_file {
        let file = Path::new(&active.instance.records).join(crate::config::WORKSPACE_FILE);
        let (_, members) = crate::config::written_members(&file).context("read the workspace")?;
        known.extend(members.into_keys());
    }
    Ok(known)
}

/// The highest number `store` holds under `prefix`, `DSP-<day>-` ([`highest`]). Every dispatch is
/// read: one the store holds but this build cannot read might carry the highest number, so it
/// fails the allocation rather than being passed over.
fn stored_highest(command: &str, store: &Store, prefix: &str) -> Result<u128> {
    let held = DispatchStorage::list(store);
    kept(command, store)?;
    Ok(highest(
        prefix,
        held.iter()
            .map(|dispatch| dispatch.data.dispatch_id.0.as_str()),
    ))
}

/// The refusal `DispatchExists`: an id given by hand that a dispatch already has.
fn exists<T>(command: &str, dispatch: &DispatchId) -> Result<T> {
    bail!(
        "{command}: --dispatch-id {}: a dispatch already has this id; nothing was sent",
        dispatch.0
    )
}

/// Opens the store, runs one generated behaviour on it, and answers its outcome once the store has
/// kept every read and write the behaviour made.
fn run<O>(
    state: Option<&Path>,
    command: &str,
    behaviour: impl FnOnce(&mut Generated<Store>) -> Result<O, UnmetObligation>,
) -> Result<(O, Store)> {
    let mut generated = Generated::new(crate::state::open(state)?);
    let outcome = behaviour(&mut generated).map_err(|unmet| anyhow!("{command}: {unmet}"))?;
    kept(command, &generated.ports)?;
    Ok((outcome, generated.ports))
}

/// Appends the records line of a dispatch `event` the store has kept: `id` the dispatch, `repo`
/// its repository as the store holds it, and what `fill` adds.
fn logged(
    command: &str,
    store: &Store,
    dispatch: &DispatchId,
    event: DispatchLogEvent,
    fill: impl FnOnce(&mut DispatchLogLine),
) -> Result<ExitCode> {
    let held = DispatchStorage::get(store, dispatch);
    kept(command, store)?;
    let repository = held.map(|held| held.data.repository.0).ok_or_else(|| {
        anyhow!(
            "{command}: dispatch {} moved, yet reads back as absent",
            dispatch.0
        )
    })?;
    let mut line = records::dispatch_line(event, &repository)?;
    line.id = Some(dispatch.0.clone());
    fill(&mut line);
    records::append_dispatch(&line)
        .with_context(|| format!("{command}: dispatch {}", dispatch.0))?;
    Ok(ExitCode::SUCCESS)
}

/// The answer to a command that found its dispatch already moved (`DispatchStateConflict`): when
/// the dispatch is in the `ended` state this command moves it to and the records' dispatch log
/// holds no line of that event for it, the earlier run kept the move and failed to append its
/// line, so this run appends it from its own input and succeeds. Otherwise the refusal stands.
fn retried(
    command: &str,
    store: &Store,
    dispatch: &DispatchId,
    (ended, event): (DispatchState, DispatchLogEvent),
    fill: impl FnOnce(&mut DispatchLogLine),
    rule: &str,
) -> Result<ExitCode> {
    let state = DispatchStorage::get(store, dispatch).map(|held| held.state);
    kept(command, store)?;
    if state == Some(ended) && !records::dispatch_event_logged(&dispatch.0, event)? {
        return logged(command, store, dispatch, event, fill);
    }
    wrong_state(command, dispatch, rule)
}

/// Refuses `--input-json`, which no handler reads yet, as `what` not implemented.
fn flags_only(input_json: Option<JsonInput>, what: &'static str) -> Result<()> {
    match input_json {
        Some(_) => not_implemented(what),
        None => Ok(()),
    }
}

/// The value of the flag `name`, or an error naming it.
fn flag<T>(command: &str, name: &str, value: Option<T>) -> Result<T> {
    value.ok_or_else(|| anyhow!("{command}: {name} is missing"))
}

/// `--dispatch-id`, which the command line has already checked starts with `DSP-`.
fn dispatch_id(command: &str, value: Option<String>) -> Result<DispatchId> {
    Ok(DispatchId(flag(command, "--dispatch-id", value)?))
}

/// Whether the store kept every read and write the behaviour made; an outcome is answered only
/// then.
fn kept(command: &str, store: &Store) -> Result<()> {
    store
        .check()
        .with_context(|| format!("{command}: the store did not keep it"))
}

/// The refusal `error` of `command`, as the one line the binary writes on standard error.
fn refused<T>(command: &str, error: &str, detail: &str) -> Result<T> {
    bail!("{command}: {error}: {detail}")
}

fn no_dispatch(command: &str, dispatch: &DispatchId) -> Result<ExitCode> {
    refused(
        command,
        "DispatchNotFound",
        &format!("no dispatch has the id {}", dispatch.0),
    )
}

fn wrong_state(command: &str, dispatch: &DispatchId, rule: &str) -> Result<ExitCode> {
    refused(
        command,
        "DispatchStateConflict",
        &format!("dispatch {} did not move: {rule}", dispatch.0),
    )
}

/// Writes `line` to standard output.
fn print_line(command: &str, line: &str) -> Result<ExitCode> {
    writeln!(std::io::stdout().lock(), "{line}")
        .with_context(|| format!("{command}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}
