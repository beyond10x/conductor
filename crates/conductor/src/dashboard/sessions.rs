//! What the Sessions page shows (`story:sessions-page-coherent`): the instance's sessions with
//! what each works on, what awaits the operator, the usage limits in force and the recent
//! decisions, each read from the source the page it summarizes reads.
//!
//! - The rows are the instance's sessions from its records, not from where a session works: its
//!   conductor (the instance's `conductor.session_name`), its maintainer when a session of the
//!   maintainer role's session name runs, and each controller the store holds `Running` or
//!   `Paused`, under the session name it was started with; a controller the store holds in any
//!   other state is a row, and counted, while the session list shows its session live
//!   (`story:dashboard-live-controllers-and-report-text`). A row of the session list joins the
//!   record of its name, a live one before one that exited; a record no row joins is listed as
//!   `not running`. Any other session is not the instance's and is not listed.
//! - A controller's dispatch is the newest open one sent to it (its last event neither `done`,
//!   `cancelled` nor `failed`), else its newest one, matched by its session name or its record's
//!   repository, as `dispatch send` names a controller by either.
//! - "Awaiting the operator" is the Decisions page's open list ([`super::decisions`]), less an item
//!   the decision log answered ([`super::decisions::answered`]); each row carries the one action it
//!   asks and the link to it on the Decisions page.
//! - A usage-limit line is shown while the reset it names is ahead; a context line only for a
//!   session the rows hold.
//! - The recent decisions are the decision log's decisions only: a line with a choice, newest
//!   first, each decision once.

use std::collections::HashSet;
use std::path::Path;

use crate::config::BuiltInRole;
use conductor_model::behaviour::{ControllerStorage, SnapshotStorage};
use conductor_model::dispatch::ControllerState;
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime};

use super::{ago, excerpt, exited, optional, rfc3339};
use crate::config;
use crate::store::Store;

/// Decision rows the page carries.
const RECENT: usize = 10;

/// The dispatch events after which a dispatch is no longer open.
const CLOSED: [&str; 3] = ["done", "cancelled", "failed"];

/// What the page reads from the store: the controllers it runs and the open operator items.
#[derive(Debug, Clone, Default)]
pub(super) struct Roster {
    /// The controllers the store holds, one per session name, by session name.
    pub controllers: Vec<Controller>,
    /// The open class-O and class-H items, as `/v1/decisions` lists them before the log is read.
    pub open: Vec<Value>,
    /// The newest complete snapshot's id and start, as `conductor status` reads it.
    pub snapshot: Option<(String, String)>,
}

/// One controller of the store the page lists.
#[derive(Debug, Clone)]
pub(super) struct Controller {
    pub repository: String,
    pub session_name: String,
    /// Whether the store holds it `Running` or `Paused`. One it holds in any other state is
    /// listed only while the session list shows its session live.
    pub held: bool,
}

/// The [`Roster`] of `store`: every controller it holds, once per session name, a `Running` or
/// `Paused` record before any other of that name.
pub(super) fn roster(store: &Store) -> Roster {
    let mut controllers: Vec<Controller> = ControllerStorage::list(store)
        .into_iter()
        .map(|held| Controller {
            held: matches!(
                held.state,
                ControllerState::Running | ControllerState::Paused
            ),
            repository: held.data.repository.0,
            session_name: held.data.session_name.0,
        })
        .collect();
    controllers.sort_by(|left, right| {
        (&left.session_name, !left.held).cmp(&(&right.session_name, !right.held))
    });
    controllers.dedup_by(|left, right| left.session_name == right.session_name);
    let snapshots = SnapshotStorage::list(store);
    Roster {
        controllers,
        open: super::decisions::open_items(store),
        snapshot: crate::status::newest_complete(&snapshots, None).map(|held| {
            (
                held.data.snapshot_id.0.0.clone(),
                held.data.started_at.0.clone(),
            )
        }),
    }
}

/// The newest complete snapshot of `roster` at `now`, `{id, started_at, age_seconds, age,
/// cadence_seconds, stale}`: `stale` when it is older than the instance's `cadence.cycle`, the time
/// between two of conductor's cycles, each of which takes one; null when the store holds none.
pub(super) fn snapshot(roster: &Roster, now: OffsetDateTime) -> Value {
    let Some((id, started_at)) = &roster.snapshot else {
        return Value::Null;
    };
    let cadence = config::seconds(&config::active().instance.cadence.cycle)
        .and_then(|seconds| i64::try_from(seconds).ok());
    let age = OffsetDateTime::parse(started_at, &Rfc3339)
        .ok()
        .map(|started| (now - started).whole_seconds());
    json!({
        "id": id,
        "started_at": started_at,
        "age_seconds": age,
        "age": age.map(|age| ago(time::Duration::seconds(age))),
        "cadence_seconds": cadence,
        "stale": matches!((age, cadence), (Some(age), Some(cadence)) if age > cadence),
    })
}

/// The rows of the page and what they hold.
pub(super) struct Shown<'a> {
    /// One row per session of the instance: the conductor, the maintainer, then the controllers.
    pub rows: Vec<Value>,
    /// The session-list rows the page rows joined.
    pub listed: Vec<&'a Value>,
    /// The controller rows a live session holds.
    pub controllers: usize,
}

/// The names the instance's conductor and maintainer sessions may carry: each role's session name
/// and, for this release, its old-name alias.
fn own_names() -> (Vec<String>, Vec<String>) {
    let active = config::active();
    let instance = &active.instance;
    let conductor = instance
        .conductor
        .session_name
        .as_ref()
        .map(|name| config::session_names_of(BuiltInRole::Director, &name.0))
        .unwrap_or_default();
    let maintainer = config::host_maintainer(instance)
        .map(|role| {
            let name = role
                .session_name
                .unwrap_or_else(|| config::session_name(instance, &role.role));
            config::session_names_of(BuiltInRole::Maintainer, &name.0)
        })
        .unwrap_or_default();
    (conductor, maintainer)
}

/// The row of the session list named one of `names`: a live one before one that exited.
fn pick<'a>(listed: &'a [Value], names: &[String]) -> Option<&'a Value> {
    let named = |row: &&Value| {
        row["name"]
            .as_str()
            .is_some_and(|name| names.iter().any(|known| known == name))
    };
    listed
        .iter()
        .filter(named)
        .find(|row| !exited(row))
        .or_else(|| listed.iter().find(named))
}

/// The instance's sessions from `roster` and its own role names, joined with the session list
/// `listed`, each controller with its dispatch from `dispatches`, at `now`.
pub(super) fn shown<'a>(
    listed: &'a [Value],
    roster: &Roster,
    dispatches: &[Value],
    now: OffsetDateTime,
) -> Shown<'a> {
    let (conductor, maintainer) = own_names();
    let mut shown = Shown {
        rows: Vec::new(),
        listed: Vec::new(),
        controllers: 0,
    };
    if let Some(name) = conductor.first() {
        let row = pick(listed, &conductor);
        shown
            .rows
            .push(session("conductor", name, None, row, None, now));
        shown.listed.extend(row);
    }
    if let (Some(name), Some(row)) = (maintainer.first(), pick(listed, &maintainer)) {
        shown
            .rows
            .push(session("maintainer", name, None, Some(row), None, now));
        shown.listed.push(row);
    }
    for controller in &roster.controllers {
        let names = [controller.session_name.clone()];
        let row = pick(listed, &names);
        let live = row.is_some_and(|row| !exited(row));
        // The session list decides, whatever the store recorded: the loop records a controller
        // stopped when it cannot reach its session, and the session may work on.
        if !controller.held && !live {
            continue;
        }
        let dispatch = dispatch(&names, &controller.repository, dispatches, now);
        if live {
            shown.controllers += 1;
        }
        shown.rows.push(session(
            "controller",
            &controller.session_name,
            Some(&controller.repository),
            row,
            dispatch,
            now,
        ));
        shown.listed.extend(row);
    }
    shown
}

/// One row: the session's role, its record's name and repository, and from the session-list row
/// `row` its kind, status, state (`exited` for one that exited), working directory and start; a
/// record no row joins reads `not running`.
fn session(
    role: &str,
    name: &str,
    repository: Option<&str>,
    row: Option<&Value>,
    dispatch: Option<Value>,
    now: OffsetDateTime,
) -> Value {
    let Some(row) = row else {
        return json!({
            "name": name, "role": role, "repository": repository, "kind": null,
            "exited": true, "status": null, "state": "not running", "cwd": null,
            "started_at": null, "started_ago": null, "dispatch": dispatch,
        });
    };
    let started = row["startedAt"].as_i64().and_then(|millis| {
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000).ok()
    });
    let gone = exited(row);
    json!({
        "name": row["name"].as_str().unwrap_or(name),
        "role": role,
        "repository": repository,
        "kind": row["kind"].clone(),
        "exited": gone,
        "status": if gone { Value::Null } else { row["status"].clone() },
        "state": if gone { json!("exited") } else { row["state"].clone() },
        "cwd": row["cwd"].clone(),
        "started_at": started.map(rfc3339),
        "started_ago": started.map(|at| ago(now - at)),
        "dispatch": dispatch,
    })
}

/// The dispatch of the controller named one of `names` for `repository`: the newest open one
/// sent to it, else its newest one, with `open` saying which; with that dispatch's last event,
/// the latest by `at` of its event lines (a report of minutes ago beats a state line written after
/// it with an older `at`).
pub(super) fn dispatch(
    names: &[String],
    repository: &str,
    dispatches: &[Value],
    now: OffsetDateTime,
) -> Option<Value> {
    let sent_to = |line: &Value| {
        line["brief"].is_string()
            && line["to"]
                .as_str()
                .is_some_and(|to| to == repository || names.iter().any(|name| name == to))
    };
    let sent: Vec<&Value> = dispatches.iter().filter(|line| sent_to(line)).collect();
    let ids: HashSet<&str> = sent.iter().filter_map(|line| line["id"].as_str()).collect();
    let events: Vec<(usize, &Value)> = dispatches
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            line["event"].is_string()
                && !line["brief"].is_string()
                && line["id"].as_str().is_some_and(|id| ids.contains(id))
        })
        .collect();
    let closed = |id: &str| {
        events
            .iter()
            .rev()
            .find(|(_, event)| event["id"].as_str() == Some(id))
            .and_then(|(_, event)| event["event"].as_str())
            .is_some_and(|event| CLOSED.contains(&event))
    };
    let open = sent
        .iter()
        .rev()
        .find(|line| line["id"].as_str().is_some_and(|id| !closed(id)));
    let (line, open) = match open {
        Some(line) => (*line, true),
        None => (*sent.last()?, false),
    };
    let id = line["id"].as_str()?;
    // The shown dispatch's own last event, so a row never pairs one dispatch with another's.
    let event = events
        .iter()
        .filter(|(_, event)| event["id"].as_str() == Some(id))
        .filter_map(|(read, event)| {
            let at = OffsetDateTime::parse(event["at"].as_str()?, &Rfc3339).ok()?;
            Some(((at, *read), *event))
        })
        .max_by_key(|(key, _)| *key)
        .map(|(_, event)| event)
        .or_else(|| {
            events
                .iter()
                .rev()
                .find(|(_, event)| event["id"].as_str() == Some(id))
                .map(|(_, event)| *event)
        });
    let at = event
        .and_then(|event| event["at"].as_str())
        .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok());
    Some(json!({
        "id": id,
        "open": open,
        "goal": line["goal"].clone(),
        "brief": excerpt(line["brief"].as_str().unwrap_or_default()),
        "event": event.map(|event| event["event"].clone()),
        "event_at": at.map(rfc3339),
        "event_ago": at.map(|at| ago(now - at)),
    }))
}

// ---------------------------------------------------------------------------------------------
// Awaiting the operator
// ---------------------------------------------------------------------------------------------

/// The open items of `open` the decision log `log` did not answer, each with the one action it
/// asks and its link on the Decisions page, in the Decisions page's order.
pub(super) fn awaiting(open: &[Value], log: &[Value]) -> Vec<Value> {
    open.iter()
        .filter(|row| {
            row["request_id"]
                .as_str()
                .is_some_and(|id| !super::decisions::answered(log, id))
        })
        .map(|row| {
            let id = row["request_id"].as_str().unwrap_or_default();
            json!({
                "id": id,
                "class": row["class"].clone(),
                "repository": row["repository"].clone(),
                "question": row["question"].as_str().map(excerpt),
                "action": action(row),
                "link": format!("#/decisions/{}", encode(id)),
            })
        })
        .collect()
}

/// The one line of what an open item asks of the operator: `answer: <letter> <label> · …` for a
/// class-O item (its own words when it recorded no options), `do: <command>` for a class-H item,
/// the command being the first one its words quote in backticks, else the option recommended,
/// else its question.
fn action(row: &Value) -> String {
    let options: Vec<(&str, &str)> = row["options"]
        .as_array()
        .map(|options| {
            options
                .iter()
                .filter_map(|option| Some((option["letter"].as_str()?, option["label"].as_str()?)))
                .collect()
        })
        .unwrap_or_default();
    let text = |key: &str| row[key].as_str().unwrap_or_default();
    if row["class"] != "H" {
        if options.is_empty() {
            return excerpt(&format!("answer: {}", text("options_text")));
        }
        let listed: Vec<String> = options
            .iter()
            .map(|(letter, label)| format!("{letter} {label}"))
            .collect();
        return excerpt(&format!("answer: {}", listed.join(" · ")));
    }
    let quoted = ["question", "reason", "options_text", "recommendation_text"]
        .into_iter()
        .map(text)
        .chain(options.iter().map(|(_, label)| *label))
        .find_map(command);
    if let Some(command) = quoted {
        return excerpt(&format!("do: {command}"));
    }
    let recommended = row["recommended"]
        .as_array()
        .and_then(|letters| letters.first())
        .and_then(Value::as_str);
    let label = options
        .iter()
        .find(|(letter, _)| Some(*letter) == recommended)
        .map(|(_, label)| *label);
    excerpt(&format!("do: {}", label.unwrap_or(text("question"))))
}

/// The first span `text` quotes in backticks, when it is not empty.
fn command(text: &str) -> Option<&str> {
    let (_, rest) = text.split_once('`')?;
    let (quoted, _) = rest.split_once('`')?;
    let quoted = quoted.trim();
    (!quoted.is_empty()).then_some(quoted)
}

/// `id` as one segment of the page's hash, every byte but `A-Za-z0-9._-` percent-encoded.
fn encode(id: &str) -> String {
    id.bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-') {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Recent decisions
// ---------------------------------------------------------------------------------------------

/// The newest [`RECENT`] decisions of the log `log`: each line with an id and a choice, the newest
/// line of each id, newest first, `{id, at, class, choice, reason}`; a line whose instant does
/// not read is listed after those that do.
pub(super) fn recent(log: &[Value]) -> Vec<Value> {
    let first = |line: &Value, keys: &[&str]| -> Value {
        keys.iter()
            .map(|key| &line[*key])
            .find(|value| !value.is_null())
            .cloned()
            .unwrap_or(Value::Null)
    };
    let mut seen = HashSet::new();
    let mut rows: Vec<(Option<OffsetDateTime>, usize, Value)> = log
        .iter()
        .enumerate()
        .filter(|(_, line)| {
            line["id"].is_string()
                && line["choice"]
                    .as_str()
                    .is_some_and(|choice| !choice.trim().is_empty())
        })
        .map(|(read, line)| {
            let at = first(line, &["at", "decided_at"]);
            let instant = at
                .as_str()
                .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok());
            let row = json!({
                "id": line["id"].clone(),
                "at": at,
                "class": first(line, &["class", "escalation_class"]),
                "choice": line["choice"].clone(),
                "reason": line["reason"].as_str().map(excerpt),
            });
            (instant, read, row)
        })
        .collect();
    rows.sort_by(|(left, left_read, _), (right, right_read, _)| {
        match (left, right) {
            (Some(left), Some(right)) => right.cmp(left),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
        .then(right_read.cmp(left_read))
    });
    rows.into_iter()
        .map(|(_, _, row)| row)
        .filter(|row| seen.insert(row["id"].to_string()))
        .take(RECENT)
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Usage limits and context
// ---------------------------------------------------------------------------------------------

/// The watch's lines from its directory `dir` at `now`: `limits`, each limit in force with its
/// reset and the sessions it was found in, and `context`, each session of `listed` (the rows the
/// page shows) whose context the watch reported. Each is null while the watch has written none of
/// its files.
pub(super) fn watch(
    dir: &Path,
    listed: &[&Value],
    now: OffsetDateTime,
    errors: &mut Vec<String>,
) -> Value {
    let context = optional(&dir.join("context-reported"), errors).map(|text| {
        let mut once = HashSet::new();
        text.lines()
            .map(str::trim)
            .filter(|id| !id.is_empty() && once.insert(*id))
            .filter_map(|id| {
                let row = listed
                    .iter()
                    .find(|row| row["sessionId"].as_str() == Some(id))?;
                Some(json!({
                    "session": id.chars().take(8).collect::<String>(),
                    "name": row["name"].clone(),
                }))
            })
            .collect::<Vec<_>>()
    });
    json!({"limits": limits(dir, now, errors), "context": context})
}

/// One limit text the watch found or saw.
struct Limit {
    text: String,
    sessions: Vec<String>,
    /// Whether the watch finds it now (`limit.new`).
    current: bool,
    /// When it was seen: its report, else the start of the day it was seen, else now.
    seen: OffsetDateTime,
}

/// The limits in force at `now`, soonest reset first: those the watch finds now (`limit.new`,
/// `<text>\t<session>`) and those it saw today or yesterday (`limit.seen`, `YYYY-MM-DD <text>`),
/// each seen when the watch reported it (`limit.reported`, `<instant> <text>`). A limit is in
/// force while the reset its text names (`resets 7:50pm (Europe/Berlin)`) is ahead; one whose
/// reset does not read is in force while the watch finds it. Null without either file.
fn limits(dir: &Path, now: OffsetDateTime, errors: &mut Vec<String>) -> Option<Vec<Value>> {
    let new = optional(&dir.join("limit.new"), errors);
    let seen = optional(&dir.join("limit.seen"), errors);
    if new.is_none() && seen.is_none() {
        return None;
    }
    let reported = optional(&dir.join("limit.reported"), errors).unwrap_or_default();
    let mut found: Vec<Limit> = Vec::new();
    let mut add = |text: &str, session: Option<&str>, current: bool, seen: OffsetDateTime| {
        let at = found
            .iter()
            .position(|known| known.text == text)
            .unwrap_or_else(|| {
                found.push(Limit {
                    text: text.to_owned(),
                    sessions: Vec::new(),
                    current,
                    seen,
                });
                found.len() - 1
            });
        let limit = &mut found[at];
        limit.current |= current;
        if let Some(session) = session.map(str::trim).filter(|name| !name.is_empty())
            && !limit.sessions.iter().any(|known| known == session)
        {
            limit.sessions.push(session.to_owned());
        }
    };
    for line in new.as_deref().unwrap_or_default().lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (text, session) = line.split_once('\t').unwrap_or((line, ""));
        add(text, Some(session), true, now);
    }
    let yesterday = now.date().previous_day().unwrap_or(now.date());
    for line in seen.as_deref().unwrap_or_default().lines() {
        let Some((day, text)) = line.split_once(' ') else {
            continue;
        };
        let Some(day) = date(day).filter(|day| *day >= yesterday) else {
            continue;
        };
        add(text, None, false, day.midnight().assume_utc());
    }
    for line in reported.lines() {
        let Some((at, text)) = line.split_once(' ') else {
            continue;
        };
        let Ok(at) = OffsetDateTime::parse(at, &Rfc3339) else {
            continue;
        };
        if let Some(limit) = found.iter_mut().find(|limit| limit.text == text)
            && at <= now
        {
            limit.seen = at;
        }
    }
    let mut kept: Vec<(Option<OffsetDateTime>, Limit)> = found
        .into_iter()
        .map(|limit| (crate::limit::reset(&limit.text, limit.seen), limit))
        .filter(|(reset, limit)| reset.map_or(limit.current, |reset| reset > now))
        .collect();
    kept.sort_by_key(|(reset, _)| (reset.is_none(), *reset));
    Some(
        kept.into_iter()
            .map(|(reset, mut limit)| {
                limit.sessions.sort_unstable();
                json!({
                    "text": limit.text,
                    "resets_at": reset.map(rfc3339),
                    "sessions": limit.sessions,
                })
            })
            .collect(),
    )
}

/// `YYYY-MM-DD` as a date.
fn date(text: &str) -> Option<Date> {
    let mut parts = text.split('-').map(str::parse::<i32>);
    let (Some(Ok(year)), Some(Ok(month)), Some(Ok(day)), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return None;
    };
    let month = time::Month::try_from(u8::try_from(month).ok()?).ok()?;
    Date::from_calendar_date(year, month, u8::try_from(day).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_h_item_s_action_is_the_command_it_quotes() {
        let row = json!({"class": "H", "question": "Log in: run `task login` there",
                         "options": [], "recommended": []});
        assert_eq!(action(&row), "do: task login");
        let row = json!({"class": "H", "question": "Clear the cache?",
                         "options": [{"letter": "Y", "label": "clear it"},
                                     {"letter": "N", "label": "keep it"}],
                         "recommended": ["Y"]});
        assert_eq!(action(&row), "do: clear it");
        let row = json!({"class": "O", "options": [], "options_text": "A now; B later"});
        assert_eq!(action(&row), "answer: A now; B later");
        assert_eq!(encode("REQ 1/x"), "REQ%201%2Fx");
    }
}
