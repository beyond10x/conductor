//! What the repository page reads (`story:dashboard-repo-view`): one repository's plan from the
//! newest complete snapshot (`/api/blockers`, `/api/specifications`), what each repository waits
//! on (`/api/waiting-on`), its sessions (`/api/sessions?repository=`) and its share of today's
//! events (`/api/events?since=&repository=`, [`events_for_repository`]).
//!
//! Every answer is read-only: the store is one opened only when it exists, and nothing here
//! writes. A repository's **checkout** is `<checkouts root>/<repository>`, its **managed trees**
//! are under `<trees>/<repository>/`. A repository's **controller session** is a session of the
//! session list whose working directory is its checkout, or the session the store's controller
//! record of the repository names.
//!
//! What a repository waits on is one rule per cause, each over data the server holds:
//!
//! | cause | rule |
//! |---|---|
//! | `operator` | a decision request of the repository, open or escalated, escalated as class O or H |
//! | `ci` | an open pull or merge request of the newest complete snapshot with a failing check, while no session of the repository (checkout or managed tree) is `working` |
//! | `build-slot` | a resource request of the repository in state `Requested` |
//! | `another-repository` | a `[NEED]` its controller session sent that conductor routed to a dispatch of another repository, and that dispatch is not `Done`, `Failed` or `Cancelled` |
//! | `usage-limit` | the watch's `limit.new` names its controller session |
//! | `silent` | the watch's `silent.reported` holds the id of its controller session |
//! | `stalled` | the watch's `stalled.reported` holds the id of a session of the repository, in its checkout or a managed tree (`story:watch-stalled-sessions`) |
//! | `build-queued` | the watch's `build-queued.reported` names a grant of the repository, or of a session of it by its name (`story:watch-stalled-sessions`) |
//!
//! The snapshot records a pull request's failing checks, not its pending ones, so a pull request
//! whose checks still run is not a `ci` cause.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use conductor_model::behaviour::{
    BlockerObservationStorage, ControllerStorage, DecisionRequestStorage, DispatchStorage,
    MessageStorage, PullRequestObservationStorage, RepositoryObservationStorage,
    ResourceRequestStorage, SnapshotStorage, SpecificationObservationStorage,
};
use conductor_model::decision::{DecisionClass, DecisionRequestState};
use conductor_model::dispatch::{
    ControllerState, DispatchState, MessageKind, ResourceKind, ResourceRequestState,
};
use conductor_model::observation::{SnapshotSnapshot, SnapshotState};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{Sources, rfc3339, tilde_all};
use crate::snapshot::{specification_presence_name, validation_result_name};
use crate::store::Store;

/// The causes, as `/api/waiting-on` names them, in the order a row lists them.
pub const CAUSES: [&str; 8] = [
    "operator",
    "ci",
    "build-slot",
    "another-repository",
    "usage-limit",
    "silent",
    "stalled",
    "build-queued",
];

/// The paths this module answers.
pub const PATHS: [&str; 4] = [
    "/api/waiting-on",
    "/api/sessions",
    "/api/blockers",
    "/api/specifications",
];

/// The answer of `/api/sessions` to the request target `target`, from the sources `sources`, the
/// last session list `listed` and the token measurement `usage` (the `usage` object of
/// `/data.json`), at `now`; none for another path. `/api/blockers` and `/api/specifications`
/// read the store alone and `/api/waiting-on` reads it too, so the server keeps their store's
/// part ([`super::cache`]) and answers them itself. Without `?repository=` it answers no row.
/// What could not be read is named in `errors`, beside the rows.
pub(super) fn endpoint(
    path: &str,
    target: &str,
    sources: &Sources,
    listed: &super::cache::Listed,
    usage: &Value,
    now: OffsetDateTime,
) -> Option<Value> {
    if path != "/api/sessions" {
        return None;
    }
    let Some(repository) = query(target, "repository") else {
        return Some(json!({"rows": [], "errors": ["no ?repository= named"]}));
    };
    let mut errors = listed.errors.clone();
    let dispatches = super::jsonl(&sources.root.join("dispatches"), &mut errors);
    let mut answer = sessions(sources, &repository, &listed.rows, &dispatches, usage, now);
    answer["errors"] = json!(errors);
    Some(answer)
}

// ---------------------------------------------------------------------------------------------
// The newest complete snapshot
// ---------------------------------------------------------------------------------------------

/// The complete snapshot of `store` that started last.
fn newest_complete(store: &Store) -> Option<SnapshotSnapshot> {
    SnapshotStorage::list(store)
        .into_iter()
        .filter(|held| held.state == SnapshotState::Complete)
        .max_by(|left, right| order(&left.data.started_at.0, &right.data.started_at.0))
}

/// Two start times in time order; as text where either is not RFC 3339.
fn order(left: &str, right: &str) -> std::cmp::Ordering {
    match (
        OffsetDateTime::parse(left, &Rfc3339),
        OffsetDateTime::parse(right, &Rfc3339),
    ) {
        (Ok(left), Ok(right)) => left.cmp(&right),
        _ => left.cmp(right),
    }
}

/// `{id, started_at}` of a snapshot, as every snapshot endpoint names the one it read.
fn header(held: &SnapshotSnapshot) -> Value {
    json!({"id": held.data.snapshot_id.0.0, "started_at": held.data.started_at.0})
}

/// `/api/blockers`: `{snapshot, rows}`, the open blockers the newest complete snapshot of `store`
/// observed, each with the fields of the `blockers` view; `snapshot: null` and no rows without a
/// store or a complete snapshot.
#[must_use]
pub fn blockers(store: Option<&Store>) -> Value {
    let Some((store, held)) = store.and_then(|store| Some((store, newest_complete(store)?))) else {
        return json!({"snapshot": null, "rows": []});
    };
    let id = &held.data.snapshot_id;
    let rows: Vec<Value> = BlockerObservationStorage::list(store)
        .into_iter()
        .map(|observed| observed.data)
        .filter(|data| data.snapshot_id == *id)
        .map(|data| {
            json!({
                "observation_id": data.observation_id.0.0,
                "snapshot_id": data.snapshot_id.0.0,
                "repository": data.repository.0,
                "reference": data.reference,
                "blocker_kind": data.blocker_kind,
                "title": data.title,
            })
        })
        .collect();
    json!({"snapshot": header(&held), "rows": rows})
}

/// `/api/specifications`: `{snapshot, rows}`, each repository's specification as the newest
/// complete snapshot of `store` observed it, with the fields of the `specifications` view.
#[must_use]
pub fn specifications(store: Option<&Store>) -> Value {
    let Some((store, held)) = store.and_then(|store| Some((store, newest_complete(store)?))) else {
        return json!({"snapshot": null, "rows": []});
    };
    let id = &held.data.snapshot_id;
    let rows: Vec<Value> = SpecificationObservationStorage::list(store)
        .into_iter()
        .map(|observed| observed.data)
        .filter(|data| data.snapshot_id == *id)
        .map(|data| {
            json!({
                "observation_id": data.observation_id.0.0,
                "snapshot_id": data.snapshot_id.0.0,
                "repository": data.repository.0,
                "presence": specification_presence_name(data.presence),
                "path": data.path,
                "format": data.format,
                "required_ess": data.required_ess,
                "validation": validation_result_name(data.validation),
                "validation_refusals": data.validation_refusals,
                "scenarios": data.scenarios,
                "synthesis_refusals": data.synthesis_refusals,
                "conformance_status": data.conformance_status,
            })
        })
        .collect();
    json!({"snapshot": header(&held), "rows": rows})
}

// ---------------------------------------------------------------------------------------------
// What a repository waits on
// ---------------------------------------------------------------------------------------------

/// A decision request, as the `operator` cause reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub request_id: String,
    pub repository: String,
    /// Open or escalated: not answered and not withdrawn.
    pub open: bool,
    /// The class it was escalated as, `C`, `O` or `H`; none while it is not escalated.
    pub class: Option<String>,
    pub question: String,
}

/// An open pull or merge request of the newest complete snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PullRequest {
    pub repository: String,
    pub number: i64,
    pub failing_checks: i64,
}

/// A resource request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceWait {
    pub resource_request_id: String,
    pub repository: String,
    /// The resource, as the specification spells it: `BuildSlot`, `Disk`, `Usage`.
    pub resource: String,
    /// In state `Requested`: neither granted nor refused.
    pub requested: bool,
}

/// A `[NEED]` message and the dispatch conductor routed it to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Need {
    /// The session that sent it.
    pub sender: String,
    pub first_line: String,
    pub dispatch_id: Option<String>,
    /// The repository of that dispatch.
    pub dispatch_repository: Option<String>,
    /// That dispatch is not `Done`, `Failed` or `Cancelled`.
    pub dispatch_open: bool,
}

/// Everything `/api/waiting-on` reads: what the store holds, the session list and the watch's
/// files. [`facts`] reads them; a test builds them.
#[derive(Debug, Clone, Default)]
pub struct Facts {
    /// The checkouts root: a repository's checkout is `<checkouts>/<repository>`.
    pub checkouts: PathBuf,
    /// The managed trees: a repository's are under `<trees>/<repository>/`.
    pub trees: PathBuf,
    /// The repositories to answer for, besides any a cause names.
    pub repositories: Vec<String>,
    /// The session list, as `claude agents --json` prints it.
    pub sessions: Vec<Value>,
    /// The session name of each repository's controller record that runs, by repository.
    pub controllers: BTreeMap<String, String>,
    pub requests: Vec<Request>,
    pub pull_requests: Vec<PullRequest>,
    pub resource_requests: Vec<ResourceWait>,
    pub needs: Vec<Need>,
    /// The watch's `limit.new`, `<text>\t<session name>` per line, when it is there.
    pub limits: Option<String>,
    /// The watch's `silent.reported`, `<session id> <newest write>` per line, when it is there.
    pub silent: Option<String>,
    /// The watch's `stalled.reported`, `<session id> <newest progress>` per line, when it is there.
    pub stalled: Option<String>,
    /// The watch's `build-queued.reported`, `<grant id>\t<repository, else session>` per line,
    /// when it is there.
    pub queued: Option<String>,
}

/// The facts `/api/waiting-on` reads from `store` (when there is one), the session list `agents`
/// and the watch's directory of `sources`.
#[must_use]
pub fn facts(store: Option<&Store>, sources: &Sources, agents: &[Value]) -> Facts {
    let read = |name: &str| std::fs::read_to_string(sources.watch.join(name)).ok();
    let mut facts = Facts {
        checkouts: sources.checkouts.clone(),
        trees: sources.trees.clone(),
        sessions: agents.to_vec(),
        limits: read("limit.new"),
        silent: read("silent.reported"),
        stalled: read("stalled.reported"),
        queued: read("build-queued.reported"),
        ..Facts::default()
    };
    let Some(store) = store else {
        return facts;
    };
    let snapshot = newest_complete(store);
    if let Some(held) = &snapshot {
        let id = &held.data.snapshot_id;
        facts.repositories = RepositoryObservationStorage::list(store)
            .into_iter()
            .map(|observed| observed.data)
            .filter(|data| data.snapshot_id == *id && !data.archived)
            .map(|data| data.repository.0)
            .collect();
        facts.pull_requests = PullRequestObservationStorage::list(store)
            .into_iter()
            .map(|observed| observed.data)
            .filter(|data| data.snapshot_id == *id)
            .map(|data| PullRequest {
                repository: data.repository.0,
                number: data.number,
                failing_checks: data.failing_checks,
            })
            .collect();
    }
    facts.controllers = ControllerStorage::list(store)
        .into_iter()
        .filter(|held| held.state == ControllerState::Running)
        .map(|held| (held.data.repository.0, held.data.session_name.0))
        .collect();
    facts.requests = DecisionRequestStorage::list(store)
        .into_iter()
        .map(|held| Request {
            request_id: held.data.request_id.0,
            repository: held.data.repository.0,
            open: matches!(
                held.state,
                DecisionRequestState::Open | DecisionRequestState::Escalated
            ),
            class: held
                .data
                .escalation_class
                .map(|class| class_name(class).to_owned()),
            question: held.data.question,
        })
        .collect();
    facts.resource_requests = ResourceRequestStorage::list(store)
        .into_iter()
        .map(|held| ResourceWait {
            resource_request_id: held.data.resource_request_id.0.0,
            repository: held.data.repository.0,
            resource: resource_name(held.data.resource).to_owned(),
            requested: held.state == ResourceRequestState::Requested,
        })
        .collect();
    let dispatches: BTreeMap<String, (String, bool)> = DispatchStorage::list(store)
        .into_iter()
        .map(|held| {
            let open = !matches!(
                held.state,
                DispatchState::Done | DispatchState::Failed | DispatchState::Cancelled
            );
            (held.data.dispatch_id.0, (held.data.repository.0, open))
        })
        .collect();
    facts.needs = MessageStorage::list(store)
        .into_iter()
        .filter(|held| held.data.kind == MessageKind::Need)
        .map(|held| {
            let dispatch_id = held.data.dispatch_id.map(|id| id.0);
            let routed = dispatch_id.as_ref().and_then(|id| dispatches.get(id));
            Need {
                sender: held.data.sender,
                first_line: held.data.first_line,
                dispatch_repository: routed.map(|(repository, _)| repository.clone()),
                dispatch_open: routed.is_some_and(|(_, open)| *open),
                dispatch_id,
            }
        })
        .collect();
    facts
}

/// The class's name as the specification spells it. Exhaustive, so a class the specification
/// adds fails to compile here.
fn class_name(class: DecisionClass) -> &'static str {
    match class {
        DecisionClass::C => "C",
        DecisionClass::O => "O",
        DecisionClass::H => "H",
    }
}

/// The resource's name as the specification spells it. Exhaustive, as [`class_name`].
fn resource_name(resource: ResourceKind) -> &'static str {
    match resource {
        ResourceKind::BuildSlot => "BuildSlot",
        ResourceKind::Disk => "Disk",
        ResourceKind::Usage => "Usage",
    }
}

/// Whether the session list's `row` works in `dir`.
fn works_in(row: &Value, dir: &Path) -> bool {
    row["cwd"].as_str().is_some_and(|cwd| Path::new(cwd) == dir)
}

/// Whether the session list's `row` works in `repository`'s checkout or one of its managed trees.
fn of_repository(row: &Value, checkouts: &Path, trees: &Path, repository: &str) -> bool {
    row["cwd"].as_str().is_some_and(|cwd| {
        let cwd = Path::new(cwd);
        cwd == checkouts.join(repository) || cwd.starts_with(trees.join(repository))
    })
}

/// `/api/waiting-on`: `{rows}`, one row per repository, by name, each `{repository, causes}`;
/// a cause is `{cause, detail}`, `another-repository` also `{on}`, the repository it waits for.
/// A repository with no cause has an empty list.
#[must_use]
pub fn waiting_on(facts: &Facts) -> Value {
    let mut names: BTreeSet<String> = facts.repositories.iter().cloned().collect();
    names.extend(facts.requests.iter().map(|r| r.repository.clone()));
    names.extend(facts.pull_requests.iter().map(|p| p.repository.clone()));
    names.extend(facts.resource_requests.iter().map(|r| r.repository.clone()));
    names.extend(facts.controllers.keys().cloned());
    let rows: Vec<Value> = names
        .iter()
        .map(|name| json!({"repository": name, "causes": causes(facts, name)}))
        .collect();
    json!({"rows": rows})
}

/// The causes that apply to `repository`, in [`CAUSES`] order.
#[must_use]
pub fn causes(facts: &Facts, repository: &str) -> Vec<Value> {
    let checkout = facts.checkouts.join(repository);
    let controllers: Vec<&Value> = facts
        .sessions
        .iter()
        .filter(|row| works_in(row, &checkout))
        .collect();
    let mut names: BTreeSet<&str> = controllers
        .iter()
        .filter_map(|row| row["name"].as_str())
        .collect();
    if let Some(name) = facts.controllers.get(repository) {
        names.insert(name.as_str());
    }
    let ids: BTreeSet<&str> = controllers
        .iter()
        .filter_map(|row| row["sessionId"].as_str())
        .collect();
    let mut out = Vec::new();

    for request in facts.requests.iter().filter(|request| {
        request.repository == repository
            && request.open
            && matches!(request.class.as_deref(), Some("O" | "H"))
    }) {
        out.push(json!({
            "cause": "operator",
            "detail": format!(
                "decision request {} (class {}): {}",
                request.request_id,
                request.class.as_deref().unwrap_or_default(),
                request.question
            ),
        }));
    }

    let working = facts.sessions.iter().any(|row| {
        of_repository(row, &facts.checkouts, &facts.trees, repository)
            && !row["pid"].is_null()
            && row["state"].as_str() == Some("working")
    });
    if !working {
        for pull in facts
            .pull_requests
            .iter()
            .filter(|pull| pull.repository == repository && pull.failing_checks > 0)
        {
            out.push(json!({
                "cause": "ci",
                "detail": format!(
                    "#{} has {} failing check(s) and no session of the repository is working",
                    pull.number, pull.failing_checks
                ),
            }));
        }
    }

    for wait in facts
        .resource_requests
        .iter()
        .filter(|wait| wait.repository == repository && wait.requested)
    {
        out.push(json!({
            "cause": "build-slot",
            "detail": format!(
                "resource request {} for {} is not granted yet",
                wait.resource_request_id, wait.resource
            ),
        }));
    }

    for need in facts.needs.iter().filter(|need| {
        names.contains(need.sender.as_str())
            && need.dispatch_open
            && need
                .dispatch_repository
                .as_deref()
                .is_some_and(|other| other != repository)
    }) {
        let other = need.dispatch_repository.clone().unwrap_or_default();
        out.push(json!({
            "cause": "another-repository",
            "on": other,
            "detail": format!(
                "{} (dispatch {} to {other})",
                need.first_line,
                need.dispatch_id.as_deref().unwrap_or_default()
            ),
        }));
    }

    if let Some(limits) = &facts.limits {
        for line in limits.lines() {
            let (text, session) = line.split_once('\t').unwrap_or((line, ""));
            if names.contains(session.trim()) {
                out.push(json!({
                    "cause": "usage-limit",
                    "detail": format!("{}: {}", session.trim(), text),
                }));
            }
        }
    }

    if let Some(silent) = &facts.silent {
        for line in silent.lines() {
            let id = line.split_whitespace().next().unwrap_or_default();
            if ids.contains(id) {
                let name = controllers
                    .iter()
                    .find(|row| row["sessionId"].as_str() == Some(id))
                    .and_then(|row| row["name"].as_str())
                    .unwrap_or_default();
                out.push(json!({
                    "cause": "silent",
                    "detail": format!(
                        "session {name} ({}) wrote nothing for longer than the watch allows",
                        id.chars().take(8).collect::<String>()
                    ),
                }));
            }
        }
    }

    // The sessions of the repository, in its checkout or a managed tree: a stall or a queued
    // build is any one's (story:watch-stalled-sessions).
    let of_it: Vec<&Value> = facts
        .sessions
        .iter()
        .filter(|row| of_repository(row, &facts.checkouts, &facts.trees, repository))
        .collect();
    if let Some(stalled) = &facts.stalled {
        for line in stalled.lines() {
            let id = line.split_whitespace().next().unwrap_or_default();
            if let Some(row) = of_it
                .iter()
                .find(|row| !id.is_empty() && row["sessionId"].as_str() == Some(id))
            {
                out.push(json!({
                    "cause": "stalled",
                    "detail": format!(
                        "session {} ({}) made no progress, no report or commit, for longer than \
                         the watch allows",
                        row["name"].as_str().unwrap_or_default(),
                        id.chars().take(8).collect::<String>()
                    ),
                }));
            }
        }
    }

    if let Some(queued) = &facts.queued {
        let sessions: BTreeSet<&str> = of_it
            .iter()
            .filter_map(|row| row["name"].as_str())
            .chain(names.iter().copied())
            .collect();
        for line in queued.lines() {
            let (grant, who) = line.split_once('\t').unwrap_or((line, ""));
            let who = who.trim();
            if who == repository || sessions.contains(who) {
                out.push(json!({
                    "cause": "build-queued",
                    "detail": format!(
                        "build {grant} of {who} waits for a grant longer than the watch allows"
                    ),
                }));
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------------------------
// A repository's sessions
// ---------------------------------------------------------------------------------------------

/// `/api/sessions?repository=<repository>`: `{rows}`, one per session of the session list
/// `agents` whose working directory is the repository's checkout or one of its managed trees,
/// by name: `{name, repository, state, started_at, dispatch, transcript, spend_today}`.
///
/// `state` and `started_at` are those `/data.json` gives the session; `dispatch` is its newest
/// dispatch in `dispatches` (`{id, brief}`, or null); `transcript` the path of its transcript,
/// `~/.claude/projects/<working directory encoded>/<session id>.jsonl`; `spend_today` the
/// repository's row of the token measurement `usage` (the `usage` object of `/data.json`, counted
/// from 00:00 UTC), or null when it counted none. Every path is written with `~`.
#[must_use]
pub fn sessions(
    sources: &Sources,
    repository: &str,
    agents: &[Value],
    dispatches: &[Value],
    usage: &Value,
    now: OffsetDateTime,
) -> Value {
    let spend = usage["rows"]
        .as_array()
        .and_then(|rows| {
            rows.iter()
                .find(|row| row["repository"].as_str() == Some(repository))
        })
        .map_or(Value::Null, |row| {
            json!({
                "calls": row["calls"],
                "input": row["input"],
                "output": row["output"],
                "cache_read": row["cache_read"],
                "cache_write": row["cache_write"],
            })
        });
    let mut rows: Vec<Value> = agents
        .iter()
        .filter(|row| of_repository(row, &sources.checkouts, &sources.trees, repository))
        .map(|row| {
            let name = row["name"].as_str();
            let started = row["startedAt"].as_i64().and_then(|millis| {
                OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000).ok()
            });
            let exited = super::exited(row);
            let dispatch = name
                .and_then(|name| {
                    // The repository's controller, `<session_prefix>-<repository>`, takes the
                    // dispatches sent to the repository too (story:sessions-page-coherent);
                    // another session of it only those sent to its own name.
                    let controller =
                        crate::config::session_name(&crate::config::active().instance, repository)
                            .0
                            == name;
                    let to = if controller { repository } else { name };
                    super::sessions::dispatch(&[name.to_owned()], to, dispatches, now)
                })
                .map_or(
                    Value::Null,
                    |dispatch| json!({"id": dispatch["id"], "brief": dispatch["brief"]}),
                );
            let transcript = match (row["cwd"].as_str(), row["sessionId"].as_str()) {
                (Some(cwd), Some(id)) => Value::from(
                    sources
                        .home
                        .join(".claude/projects")
                        .join(crate::usage::encode(Path::new(cwd)))
                        .join(format!("{id}.jsonl"))
                        .display()
                        .to_string(),
                ),
                _ => Value::Null,
            };
            json!({
                "name": name,
                "repository": repository,
                "state": if exited { json!("exited") } else { row["state"].clone() },
                "started_at": started.map(rfc3339),
                "dispatch": dispatch,
                "transcript": transcript,
                "spend_today": spend,
            })
        })
        .collect();
    rows.sort_by(|a, b| a["name"].as_str().cmp(&b["name"].as_str()));
    let home = sources.home.display().to_string();
    tilde_all(json!({"rows": rows}), home.trim_end_matches('/'))
}

// ---------------------------------------------------------------------------------------------
// Today's events of one repository
// ---------------------------------------------------------------------------------------------

/// The rows of `/api/events` (`story:dashboard-overview`) that are `repository`'s: those whose
/// `repository` names it. A row of no repository (`null`) is no repository's.
#[must_use]
pub fn events_for_repository(rows: Vec<Value>, repository: &str) -> Vec<Value> {
    rows.into_iter()
        .filter(|row| row["repository"].as_str() == Some(repository))
        .collect()
}

/// The value of `key` in the query of the request target `target`, percent-decoded; none when
/// the query does not name it or its value does not decode as UTF-8.
#[must_use]
pub fn query(target: &str, key: &str) -> Option<String> {
    let (_, query) = target.split_once('?')?;
    query.split('&').find_map(|pair| {
        let (name, value) = pair.split_once('=').unwrap_or((pair, ""));
        (name == key).then(|| decode(value)).flatten()
    })
}

/// `text` with each `%XX` written as its byte and each `+` as a space.
fn decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'%' => {
                let hex = text.get(at + 1..at + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                at += 3;
            }
            b'+' => {
                out.push(b' ');
                at += 1;
            }
            byte => {
                out.push(byte);
                at += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}
