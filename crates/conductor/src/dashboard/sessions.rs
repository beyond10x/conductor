//! What the Sessions page shows (`story:sessions-page-coherent`): the instance's sessions with
//! what each works on, what awaits the operator, the usage limits in force and the recent
//! decisions, each read from the source the page it summarizes reads.
//!
//! - The rows are the instance's sessions from its records, not from where a session works: its
//!   conductor (the instance's `conductor.session_name`), its maintainer when a session of the
//!   maintainer role's session name runs, and each controller the store holds `Running` or
//!   `Paused`, under the session name it was started with. A row of the session list joins the
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

use conductor_model::behaviour::{ControllerStorage, SnapshotStorage};
use conductor_model::config::RoleName;
use conductor_model::dispatch::ControllerState;
use serde_json::{Value, json};
use time::format_description::well_known::Rfc3339;
use time::{Date, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

use super::{ago, excerpt, exited, optional, rfc3339};
use crate::config;
use crate::store::Store;

/// Decision rows the page carries.
const RECENT: usize = 10;

/// Where the host keeps its time zones, which a usage-limit line names its reset in.
const ZONEINFO: &str = "/usr/share/zoneinfo";

/// The dispatch events after which a dispatch is no longer open.
const CLOSED: [&str; 3] = ["done", "cancelled", "failed"];

/// What the page reads from the store: the controllers it runs and the open operator items.
#[derive(Debug, Clone, Default)]
pub(super) struct Roster {
    /// The controllers `Running` or `Paused`, by session name.
    pub controllers: Vec<Controller>,
    /// The open class-O and class-H items, as `/api/decisions` lists them before the log is read.
    pub open: Vec<Value>,
    /// The newest complete snapshot's id and start, as `conductor status` reads it.
    pub snapshot: Option<(String, String)>,
}

/// One controller of the store the page lists.
#[derive(Debug, Clone)]
pub(super) struct Controller {
    pub repository: String,
    pub session_name: String,
}

/// The [`Roster`] of `store`.
pub(super) fn roster(store: &Store) -> Roster {
    let mut controllers: Vec<Controller> = ControllerStorage::list(store)
        .into_iter()
        .filter(|held| {
            matches!(
                held.state,
                ControllerState::Running | ControllerState::Paused
            )
        })
        .map(|held| Controller {
            repository: held.data.repository.0,
            session_name: held.data.session_name.0,
        })
        .collect();
    controllers.sort_by(|left, right| left.session_name.cmp(&right.session_name));
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

/// The names the instance's conductor and maintainer sessions may carry, under `home`'s host
/// file: each role's session name and, for this release, its old-name alias.
fn own_names(home: &Path) -> (Vec<String>, Vec<String>) {
    let active = config::active();
    let instance = &active.instance;
    let conductor = instance
        .conductor
        .session_name
        .as_ref()
        .map(|name| config::session_names_of(RoleName::Director, &name.0))
        .unwrap_or_default();
    let maintainer = config::host_maintainer(home, instance)
        .ok()
        .flatten()
        .map(|role| {
            let name = role
                .session_name
                .unwrap_or_else(|| config::session_name(instance, &role.role));
            config::session_names_of(RoleName::Maintainer, &name.0)
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
    home: &Path,
    dispatches: &[Value],
    now: OffsetDateTime,
) -> Shown<'a> {
    let (conductor, maintainer) = own_names(home);
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
        let dispatch = dispatch(&names, &controller.repository, dispatches, now);
        if row.is_some_and(|row| !exited(row)) {
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
/// sent to it, else its newest one, with `open` saying which; with the controller's last event,
/// the latest by `at` of the event lines of every dispatch sent to it.
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
    let event = events
        .iter()
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
        .map(|limit| (reset(&limit.text, limit.seen), limit))
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

/// The reset a limit text names, `resets 7:50pm (Europe/Berlin)`, `resets 11am (UTC)` or
/// `resets 19:50`, as the first instant of that clock time in that zone (UTC when it names none)
/// at or after `seen`; none when the text names no reset this reads.
fn reset(text: &str, seen: OffsetDateTime) -> Option<OffsetDateTime> {
    let (_, rest) = text.split_once("resets ")?;
    let (clock, rest) = rest.split_once(' ').unwrap_or((rest, ""));
    let clock = clock.trim_end_matches([',', '.', ';']).to_ascii_lowercase();
    let (digits, pm) = match (clock.strip_suffix("am"), clock.strip_suffix("pm")) {
        (Some(digits), _) => (digits, Some(false)),
        (_, Some(digits)) => (digits, Some(true)),
        _ => (clock.as_str(), None),
    };
    let (hour, minute) = match digits.split_once(':') {
        Some((hour, minute)) => (hour.parse::<u8>().ok()?, minute.parse::<u8>().ok()?),
        None => (digits.parse::<u8>().ok()?, 0),
    };
    let hour = match pm {
        Some(_) if hour == 0 || hour > 12 => return None,
        Some(pm) => hour % 12 + if pm { 12 } else { 0 },
        None => hour,
    };
    let clock = Time::from_hms(hour, minute, 0).ok()?;
    let zone = rest
        .trim()
        .strip_prefix('(')
        .and_then(|zone| zone.split_once(')'))
        .map(|(zone, _)| zone.trim());
    let offset = |at: OffsetDateTime| zone.map_or(Some(UtcOffset::UTC), |zone| offset(zone, at));
    let local = seen.to_offset(offset(seen)?);
    let at = |day: Date| -> Option<OffsetDateTime> {
        let wall = PrimitiveDateTime::new(day, clock);
        Some(wall.assume_offset(offset(wall.assume_utc())?))
    };
    let today = at(local.date())?;
    if today >= seen {
        return Some(today);
    }
    at(local.date().next_day()?)
}

/// The offset of the time zone `zone` at `at`: UTC for `UTC` and its names, else from the host's
/// zone file; none for a name that is no zone file's.
fn offset(zone: &str, at: OffsetDateTime) -> Option<UtcOffset> {
    if matches!(zone, "UTC" | "Etc/UTC" | "GMT" | "Etc/GMT" | "Z") {
        return Some(UtcOffset::UTC);
    }
    let plain = !zone.is_empty()
        && zone
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '-' | '+'))
        && zone
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('-'));
    if !plain {
        return None;
    }
    let bytes = std::fs::read(Path::new(ZONEINFO).join(zone)).ok()?;
    tzif_offset(&bytes, at.unix_timestamp())
}

/// The offset a TZif file `bytes` gives at the Unix time `at`: that of the last transition at or
/// before it, from the 64-bit block of a version 2 file and the 32-bit block of a version 1 one;
/// the first local time type before every transition.
fn tzif_offset(bytes: &[u8], at: i64) -> Option<UtcOffset> {
    fn counts(block: &[u8]) -> Option<[usize; 6]> {
        if block.get(..4)? != b"TZif" {
            return None;
        }
        let mut counts = [0; 6];
        for (index, count) in counts.iter_mut().enumerate() {
            let at = 20 + 4 * index;
            let raw: [u8; 4] = block.get(at..at + 4)?.try_into().ok()?;
            *count = usize::try_from(u32::from_be_bytes(raw)).ok()?;
        }
        Some(counts)
    }
    let [ut, std, leap, times, types, chars] = counts(bytes)?;
    let (block, width) = if *bytes.get(4)? >= b'2' {
        let skip = 44 + times * 5 + types * 6 + chars + leap * 8 + std + ut;
        (bytes.get(skip..)?, 8)
    } else {
        (bytes, 4)
    };
    let [_, _, _, times, types, _] = counts(block)?;
    let data = block.get(44..)?;
    let transition = |index: usize| -> Option<i64> {
        let raw = data.get(index * width..(index + 1) * width)?;
        Some(if width == 8 {
            i64::from_be_bytes(raw.try_into().ok()?)
        } else {
            i64::from(i32::from_be_bytes(raw.try_into().ok()?))
        })
    };
    let indices = data.get(times * width..times * width + times)?;
    let infos = data.get(times * width + times..times * width + times + types * 6)?;
    let mut kind = 0;
    for index in 0..times {
        if transition(index)? > at {
            break;
        }
        kind = usize::from(*indices.get(index)?);
    }
    let info: [u8; 4] = infos.get(kind * 6..kind * 6 + 4)?.try_into().ok()?;
    UtcOffset::from_whole_seconds(i32::from_be_bytes(info)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A version 1 TZif file of two types, +01:00 then +02:00 from the Unix time 1000.
    fn tzif() -> Vec<u8> {
        let mut bytes = b"TZif".to_vec();
        bytes.push(0);
        bytes.extend([0; 15]);
        for count in [0_u32, 0, 0, 1, 2, 4] {
            bytes.extend(count.to_be_bytes());
        }
        bytes.extend(1000_i32.to_be_bytes());
        bytes.push(1);
        bytes.extend(3600_i32.to_be_bytes());
        bytes.extend([0, 0]);
        bytes.extend(7200_i32.to_be_bytes());
        bytes.extend([1, 0]);
        bytes.extend(b"AB\0\0");
        bytes
    }

    #[test]
    fn a_tzif_file_gives_the_offset_of_the_last_transition_before_the_instant() {
        let hour = |hours: i8| UtcOffset::from_hms(hours, 0, 0).ok();
        assert_eq!(tzif_offset(&tzif(), 999), hour(1));
        assert_eq!(tzif_offset(&tzif(), 1000), hour(2));
        assert_eq!(tzif_offset(&tzif(), 5000), hour(2));
        assert_eq!(tzif_offset(b"nope", 0), None);
    }

    #[test]
    fn a_reset_is_the_next_instant_of_its_clock_time_after_it_was_seen() {
        let seen = OffsetDateTime::parse("2026-10-17T14:07:00Z", &Rfc3339).expect("an instant");
        let at = |text: &str| reset(text, seen).map(rfc3339);
        assert_eq!(
            at("You've hit your limit · resets 7:50pm (UTC)").as_deref(),
            Some("2026-10-17T19:50:00Z")
        );
        assert_eq!(
            at("limit · resets 11am (UTC)").as_deref(),
            Some("2026-10-18T11:00:00Z"),
            "a clock time already past that day is the next day's"
        );
        assert_eq!(
            at("limit · resets 12:30am").as_deref(),
            Some("2026-10-18T00:30:00Z")
        );
        assert_eq!(
            at("limit · resets 16:00").as_deref(),
            Some("2026-10-17T16:00:00Z")
        );
        assert_eq!(at("Usage limit reached"), None);
        assert_eq!(at("limit · resets 13pm (UTC)"), None);
        assert_eq!(at("limit · resets 7pm (../../etc/passwd)"), None);
    }

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
