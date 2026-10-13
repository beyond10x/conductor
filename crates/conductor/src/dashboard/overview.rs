//! What the overview reads (`story:dashboard-overview`): the newest complete snapshot's board,
//! repositories, open pull and merge requests, `main` workflow runs and releases
//! (`/api/board`, `/api/repositories`, `/api/pull-requests`, `/api/workflow-runs`,
//! `/api/releases`), and the day's events (`/api/events?since=<rfc3339>`).
//!
//! Each snapshot endpoint answers `{snapshot: {id, started_at}, rows: [...]}` from the complete
//! snapshot that started last, so a newer one still collecting, or one that failed, is never read;
//! with no store, or no complete snapshot in it, `{snapshot: null, rows: []}`. `/api/board`'s rows
//! are [`crate::status::repository_figures`], the one place `publish-board`'s deviations are
//! worked out too: the app does not recompute them.
//!
//! `/api/events` answers `{rows: [...]}`, one row per event at or after `since`, newest first, each
//! `{at, repository, kind, text, link}` (`repository` and `link` null where there is none), of
//! five kinds:
//!
//! | kind | from | `at` | `repository` | `text` | `link` |
//! |---|---|---|---|---|---|
//! | `dispatch` | a line with a `brief` in `dispatches/*.jsonl` | its `at` (`sent_at` in an older line) | the repository whose controller session is its `to` | its goal | — |
//! | `report` | an event line (`event`, `at`) in the same files | its `at` | its dispatch's | the event | — |
//! | `decision` | a line in `decisions/*.jsonl` | its `decided_at`; `raised_at` for an open request | its `repository` | its class and choice, or class and question | — |
//! | `release` | a row of the snapshot's `releases` | its publication | its repository | its tag | its `url` |
//! | `merge` | a row of the snapshot's `merged-pull-requests` | its merge | its repository | `#<number> <title>` | its `url` |
//!
//! Everything here is read-only: the store is opened by the caller only when it exists.

use std::cmp::Reverse;
use std::collections::HashMap;

use conductor_model::behaviour::{
    MergedPullRequestObservationStorage, PullRequestObservationStorage, ReleaseObservationStorage,
    RepositoryObservationStorage, SnapshotStorage, WorkflowRunObservationStorage,
};
use conductor_model::config::Instance;
use conductor_model::observation::SnapshotSnapshot;
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, Time, UtcOffset};

use crate::snapshot::{mergeability_name, run_conclusion_name, visibility_name};
use crate::store::Store;

/// The endpoints that answer rows of the newest complete snapshot, by path.
pub const SNAPSHOT_PATHS: [&str; 5] = [
    "/api/board",
    "/api/repositories",
    "/api/pull-requests",
    "/api/workflow-runs",
    "/api/releases",
];

/// The path of the day's events.
pub const EVENTS_PATH: &str = "/api/events";

/// The kinds of event `/api/events` answers, in the order the table above lists them.
pub const KINDS: [&str; 5] = ["dispatch", "report", "decision", "release", "merge"];

// ---------------------------------------------------------------------------------------------
// The newest complete snapshot
// ---------------------------------------------------------------------------------------------

/// The complete snapshot of `store` that started last, the one `publish-board` reads without
/// `--snapshot-id`.
fn newest(store: &Store) -> Option<SnapshotSnapshot> {
    let snapshots = SnapshotStorage::list(store);
    crate::status::newest_complete(&snapshots, None).cloned()
}

/// `{snapshot: {id, started_at}, rows}` of the newest complete snapshot of `store`, its rows those
/// `rows` answers for it; `{snapshot: null, rows: []}` without a store or a complete snapshot.
fn answer(
    store: Option<&Store>,
    rows: impl FnOnce(&Store, &SnapshotSnapshot) -> Vec<Value>,
) -> Value {
    let Some((store, held)) = store.and_then(|store| Some((store, newest(store)?))) else {
        return json!({"snapshot": null, "rows": []});
    };
    let rows = rows(store, &held);
    json!({
        "snapshot": {"id": held.data.snapshot_id.0.0, "started_at": held.data.started_at.0},
        "rows": rows,
    })
}

/// The answer of the snapshot endpoint `path` ([`SNAPSHOT_PATHS`]) over `store`, or none for
/// another path.
#[must_use]
pub fn snapshot_endpoint(path: &str, store: Option<&Store>) -> Option<Value> {
    Some(match path {
        "/api/board" => board(store),
        "/api/repositories" => repositories(store),
        "/api/pull-requests" => pull_requests(store),
        "/api/workflow-runs" => workflow_runs(store),
        "/api/releases" => releases(store),
        _ => return None,
    })
}

/// `/api/board`: one row per repository that is not archived, in name order, with its deviations
/// and the five counts the board compares.
#[must_use]
pub fn board(store: Option<&Store>) -> Value {
    answer(store, |store, held| {
        crate::status::repository_figures(store, held)
            .into_iter()
            .map(|figures| {
                json!({
                    "repository": figures.repository,
                    "deviations": figures.deviations,
                    "open_prs": figures.open_prs,
                    "failing_prs": figures.failing_prs,
                    "non_green_workflows": figures.non_green_workflows,
                    "open_blockers": figures.open_blockers,
                    "specifications_present": figures.specifications_present,
                })
            })
            .collect()
    })
}

/// `/api/repositories`: one row per repository the snapshot observed, in name order, with the
/// fields of the `repositories` view.
#[must_use]
pub fn repositories(store: Option<&Store>) -> Value {
    answer(store, |store, held| {
        let mut rows: Vec<_> = RepositoryObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == held.data.snapshot_id)
            .collect();
        rows.sort_by(|left, right| left.repository.0.cmp(&right.repository.0));
        rows.into_iter()
            .map(|data| {
                json!({
                    "observation_id": data.observation_id.0.0,
                    "snapshot_id": data.snapshot_id.0.0,
                    "repository": data.repository.0,
                    "visibility": visibility_name(data.visibility),
                    "archived": data.archived,
                    "local_checkout": data.local_checkout,
                    "main_head": data.main_head.0,
                    "main_committed_at": data.main_committed_at.0,
                    "main_commits_7d": data.main_commits_7d,
                    "real_commits_7d": data.real_commits_7d,
                    "behind_main": data.behind_main,
                    "dirty_files": data.dirty_files,
                    "worktrees": data.worktrees,
                    "open_issues": data.open_issues,
                    "oldest_open_issue_at": data.oldest_open_issue_at.map(|at| at.0),
                    "latest_release": data.latest_release,
                    "latest_release_at": data.latest_release_at.map(|at| at.0),
                    "unreleased_commits": data.unreleased_commits,
                    "planning_store_version": data.planning_store_version,
                    "in_catalog": data.in_catalog,
                })
            })
            .collect()
    })
}

/// `/api/pull-requests`: one row per open pull or merge request the snapshot observed, by
/// repository and number, with its `url`.
#[must_use]
pub fn pull_requests(store: Option<&Store>) -> Value {
    answer(store, |store, held| {
        let mut rows: Vec<_> = PullRequestObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == held.data.snapshot_id)
            .collect();
        rows.sort_by(|left, right| {
            (&left.repository.0, left.number).cmp(&(&right.repository.0, right.number))
        });
        rows.into_iter()
            .map(|data| {
                json!({
                    "observation_id": data.observation_id.0.0,
                    "snapshot_id": data.snapshot_id.0.0,
                    "repository": data.repository.0,
                    "number": data.number,
                    "title": data.title,
                    "draft": data.draft,
                    "mergeability": mergeability_name(data.mergeability),
                    "failing_checks": data.failing_checks,
                    "opened_at": data.opened_at.0,
                    "updated_at": data.updated_at.0,
                    "url": data.url,
                })
            })
            .collect()
    })
}

/// `/api/workflow-runs`: the latest `main` run of each workflow the snapshot observed, by
/// repository and workflow.
#[must_use]
pub fn workflow_runs(store: Option<&Store>) -> Value {
    answer(store, |store, held| {
        let mut rows: Vec<_> = WorkflowRunObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == held.data.snapshot_id)
            .collect();
        rows.sort_by(|left, right| {
            (&left.repository.0, &left.workflow).cmp(&(&right.repository.0, &right.workflow))
        });
        rows.into_iter()
            .map(|data| {
                json!({
                    "observation_id": data.observation_id.0.0,
                    "snapshot_id": data.snapshot_id.0.0,
                    "repository": data.repository.0,
                    "workflow": data.workflow,
                    "conclusion": run_conclusion_name(data.conclusion),
                    "run_at": data.run_at.0,
                })
            })
            .collect()
    })
}

/// `/api/releases`: each release the snapshot observed, by repository and publication.
#[must_use]
pub fn releases(store: Option<&Store>) -> Value {
    answer(store, |store, held| {
        let mut rows: Vec<_> = ReleaseObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == held.data.snapshot_id)
            .collect();
        rows.sort_by(|left, right| {
            (&left.repository.0, &left.published_at.0)
                .cmp(&(&right.repository.0, &right.published_at.0))
        });
        rows.into_iter()
            .map(|data| {
                json!({
                    "observation_id": data.observation_id.0.0,
                    "snapshot_id": data.snapshot_id.0.0,
                    "repository": data.repository.0,
                    "tag": data.tag,
                    "published_at": data.published_at.0,
                    "url": data.url,
                })
            })
            .collect()
    })
}

// ---------------------------------------------------------------------------------------------
// The day's events
// ---------------------------------------------------------------------------------------------

/// 00:00 UTC of the day `now` falls on, in UTC: the `since` of the overview's "today".
#[must_use]
pub fn start_of_day(now: OffsetDateTime) -> OffsetDateTime {
    now.to_offset(UtcOffset::UTC).replace_time(Time::MIDNIGHT)
}

/// `since` as the query `target` gives it (`?since=<rfc3339>`, percent-encoded or not), or none
/// when the query names none. `Err` when it names one that is not RFC 3339.
///
/// # Errors
///
/// The query's `since` is not an RFC 3339 instant.
pub fn since(target: &str) -> Result<Option<OffsetDateTime>, String> {
    let Some(text) = super::repo::query(target, "since") else {
        return Ok(None);
    };
    OffsetDateTime::parse(&text, &Rfc3339)
        .map(Some)
        .map_err(|_| format!("since={text} is not an RFC 3339 instant"))
}

/// The repository whose controller session is named `session` under `instance`: the name without
/// the instance's session prefix (`<prefix>-<repository>`, or the repository without a prefix);
/// none for conductor's own session, or a name without the prefix.
fn repository_of_session(instance: &Instance, session: &str) -> Option<String> {
    if session == crate::config::session_name(instance, "conductor").0 {
        return None;
    }
    match &instance.session_prefix {
        Some(prefix) => session
            .strip_prefix(&format!("{}-", prefix.0))
            .filter(|rest| !rest.is_empty())
            .map(str::to_owned),
        None => Some(session.to_owned()),
    }
}

/// The text of `value` when it is a non-empty string.
fn text(value: &Value) -> Option<&str> {
    value.as_str().filter(|text| !text.trim().is_empty())
}

/// One event row, kept with the instant it sorts by.
struct Event {
    at: OffsetDateTime,
    row: Value,
}

/// `/api/events`: the events at or after `since`, newest first, from the dispatch lines
/// `dispatches` and decision lines `decisions` (as `dashboard.rs`'s `jsonl()` reads them, in file
/// order), the controller sessions of `instance`, and the newest complete snapshot of `store`.
#[must_use]
pub fn events(
    store: Option<&Store>,
    dispatches: &[Value],
    decisions: &[Value],
    instance: &Instance,
    since: OffsetDateTime,
) -> Value {
    let mut events: Vec<Event> = Vec::new();
    let mut push = |at: Option<&str>,
                    repository: Option<String>,
                    kind: &str,
                    text: String,
                    link: Option<String>| {
        let Some(at_text) = at else {
            return;
        };
        let Ok(at) = OffsetDateTime::parse(at_text, &Rfc3339) else {
            return;
        };
        if at < since {
            return;
        }
        events.push(Event {
            at,
            row: json!({
                "at": at_text, "repository": repository, "kind": kind, "text": text, "link": link,
            }),
        });
    };

    // The repository of each dispatch, by id: that of the session it was sent to.
    let mut sent: HashMap<&str, Option<String>> = HashMap::new();
    for line in dispatches.iter().filter(|line| line["brief"].is_string()) {
        let repository = line["to"]
            .as_str()
            .and_then(|to| repository_of_session(instance, to));
        if let Some(id) = line["id"].as_str() {
            sent.insert(id, repository.clone());
        }
        let at = text(&line["at"]).or_else(|| text(&line["sent_at"]));
        let goal = text(&line["goal"])
            .or_else(|| text(&line["brief"]))
            .unwrap_or_default();
        push(at, repository, "dispatch", goal.to_owned(), None);
    }
    for line in dispatches.iter().filter(|line| line["event"].is_string()) {
        let repository = line["id"]
            .as_str()
            .and_then(|id| sent.get(id).cloned())
            .flatten();
        let event = line["event"].as_str().unwrap_or_default();
        push(
            text(&line["at"]),
            repository,
            "report",
            event.to_owned(),
            None,
        );
    }
    for line in decisions {
        let class = line["class"].as_str().unwrap_or("?");
        let repository = text(&line["repository"]).map(str::to_owned);
        match text(&line["choice"]) {
            Some(choice) => push(
                text(&line["decided_at"]),
                repository,
                "decision",
                format!("{class}: {choice}"),
                None,
            ),
            None => push(
                text(&line["raised_at"]).or_else(|| text(&line["decided_at"])),
                repository,
                "decision",
                format!("{class}: {}", line["question"].as_str().unwrap_or_default()),
                None,
            ),
        }
    }
    if let Some((store, held)) = store.and_then(|store| Some((store, newest(store)?))) {
        let id = &held.data.snapshot_id;
        for data in ReleaseObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
        {
            push(
                Some(data.published_at.0.as_str()),
                Some(data.repository.0),
                "release",
                data.tag,
                Some(data.url),
            );
        }
        for data in MergedPullRequestObservationStorage::list(store)
            .into_iter()
            .map(|held| held.data)
            .filter(|data| data.snapshot_id == *id)
        {
            push(
                Some(data.merged_at.0.as_str()),
                Some(data.repository.0),
                "merge",
                format!("#{} {}", data.number, data.title),
                Some(data.url),
            );
        }
    }
    // Newest first; of two at the same instant, the one read later first.
    let mut indexed: Vec<(usize, Event)> = events.into_iter().enumerate().collect();
    indexed.sort_by_key(|(index, event)| Reverse((event.at, *index)));
    json!({"rows": indexed.into_iter().map(|(_, event)| event.row).collect::<Vec<_>>()})
}
