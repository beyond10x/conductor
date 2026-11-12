//! The global API's federated routes (`story:federated-views`, design architecture-next § 5.2):
//! `GET /v1/sessions`, `decisions`, `unanswered` and `kpis` ask every instance's `/v1/<route>` at
//! once and answer `{"rows": [...]}`, the union of their rows, each carrying `instance`
//! ([`view`]); `GET /v1/events` streams every instance's events as server-sent events, each
//! `data:` one row with the same column ([`events`]).
//!
//! An instance that does not answer within `daemon.timeout` ([`crate::config::FEDERATED_TIMEOUT`]
//! when absent), whose runtime the daemon holds no route to, or that answers anything but `200`
//! with JSON, is one row `{"instance", "state": "unreachable", "last_seen", "error"}`, and the view
//! is still `200`. `last_seen` is the later of the last time it answered this daemon and the time
//! the reconciler last saw its runtime running ([`super::api::Api::publish`]); the shared
//! database holds no record of an instance answering yet, so neither time comes from it.
//!
//! Which rows of an instance's answer are its rows, by route: `rows` of `sessions` and `events`,
//! `open` of `decisions`, every element of `kpis`, and the whole answer of `unanswered` (its count
//! and its oldest item), one row per instance.

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read as _, Write};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, bail};
use conductor_model::instance::InstanceState;
use serde_json::{Map, Value, json};

use super::api::Route;

/// The federated routes, under `/v1/`.
pub const ROUTES: [&str; 5] = ["sessions", "decisions", "unanswered", "kpis", "events"];

/// The route the operator asks the supervisor through.
pub const ASK: &str = "/v1/ask";

/// How often the events stream asks every instance for its new events.
const EVENTS_EVERY: Duration = Duration::from_secs(1);

/// An instance's state as a federated row spells it (decision F3): the one place the
/// specification's state becomes the wire's lowercase word.
#[must_use]
pub const fn wire_state(state: InstanceState) -> &'static str {
    match state {
        InstanceState::Purged => "purged",
        InstanceState::Running => "running",
        InstanceState::Starting => "starting",
        InstanceState::Stopped => "stopped",
        InstanceState::Unreachable => "unreachable",
        InstanceState::Upgrading => "upgrading",
    }
}

/// One instance a federated route asks: its name, the route to its runtime when the daemon holds
/// one, and when it was last seen.
#[derive(Debug, Clone)]
pub struct Member {
    /// The instance's name.
    pub name: String,
    /// The route to its runtime; none when the daemon started none or saw it stop.
    pub route: Option<Route>,
    /// When it was last seen, RFC 3339.
    pub last_seen: Option<String>,
}

/// The row of an instance that did not answer.
#[must_use]
pub fn unreachable(member: &Member, error: &str) -> Value {
    json!({
        "instance": member.name,
        "state": wire_state(InstanceState::Unreachable),
        "last_seen": member.last_seen,
        "error": error,
    })
}

/// Asks the instance at `route` for `target` (`/v1/<route>?<query>`), the whole exchange within
/// `timeout`; answers its JSON when it answers `200`.
///
/// # Errors
///
/// It does not connect, does not answer within `timeout`, or answers another status or no JSON.
pub fn fetch(route: &Route, target: &str, timeout: Duration) -> Result<Value> {
    // A runtime in a box on a remote substrate daemon answers through one relay session there
    // (`story:placement-substrate-remote`), bounded by the remote daemon's own timeout.
    if let Some(remote) = &route.remote {
        let bytes = remote.send(
            format!(
                "GET {target} HTTP/1.1\r\nHost: localhost\r\nAccept: application/json\r\n\
                 Connection: close\r\n\r\n"
            )
            .as_bytes(),
        )?;
        let (status, body) = super::api::split(&bytes)?;
        if status != 200 {
            bail!("it answered {status}");
        }
        return serde_json::from_slice(&body).context("its answer is no JSON");
    }
    let deadline = Instant::now() + timeout;
    let mut stream = route.connect(timeout)?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(
        &route.carrying(
            format!(
                "GET {target} HTTP/1.1\r\nHost: localhost\r\nAccept: application/json\r\n\
                 Connection: close\r\n\r\n"
            )
            .as_bytes(),
        ),
    )?;
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            bail!(
                "it did not answer within {} s (daemon.timeout)",
                timeout.as_secs_f32()
            );
        }
        stream.set_read_timeout(Some(left))?;
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(read) => bytes.extend_from_slice(&chunk[..read]),
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                bail!(
                    "it did not answer within {} s (daemon.timeout)",
                    timeout.as_secs_f32()
                );
            }
            Err(error) => return Err(error.into()),
        }
    }
    let (status, body) = super::api::split(&bytes)?;
    if status != 200 {
        bail!("it answered {status}");
    }
    serde_json::from_slice(&body).context("its answer is no JSON")
}

/// The rows of the instance `instance` in its answer `answer` of `route`, each carrying
/// `instance` first.
#[must_use]
pub fn rows(route: &str, instance: &str, answer: Value) -> Vec<Value> {
    let listed = match (route, answer) {
        ("unanswered", answer) => vec![answer],
        (_, Value::Array(rows)) => rows,
        ("decisions", Value::Object(mut map)) => match map.remove("open") {
            Some(Value::Array(rows)) => rows,
            _ => Vec::new(),
        },
        (_, Value::Object(mut map)) => match map.remove("rows") {
            Some(Value::Array(rows)) => rows,
            _ => vec![Value::Object(map)],
        },
        (_, other) => vec![other],
    };
    listed
        .into_iter()
        .map(|row| {
            let mut out = Map::new();
            out.insert("instance".to_owned(), Value::from(instance));
            match row {
                Value::Object(fields) => {
                    out.extend(fields.into_iter().filter(|(key, _)| key != "instance"))
                }
                other => {
                    out.insert("value".to_owned(), other);
                }
            }
            Value::Object(out)
        })
        .collect()
}

/// Every member's answer to `target`, asked at once, in the members' order: its JSON, or why it
/// did not answer.
fn ask_all(members: &[Member], target: &str, timeout: Duration) -> Vec<Result<Value, String>> {
    std::thread::scope(|scope| {
        let asked: Vec<_> = members
            .iter()
            .map(|member| {
                scope.spawn(move || match &member.route {
                    Some(route) => {
                        fetch(route, target, timeout).map_err(|error| format!("{error:#}"))
                    }
                    None => Err("the daemon runs no runtime of it".to_owned()),
                })
            })
            .collect();
        asked
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .unwrap_or_else(|_| Err("asking it failed".to_owned()))
            })
            .collect()
    })
}

/// `GET /v1/<route>?<query>` over `members`, each within `timeout`; `answered` is told the name of
/// each member that answered.
pub fn view(
    members: &[Member],
    route: &str,
    query: Option<&str>,
    timeout: Duration,
    answered: &dyn Fn(&str),
) -> Value {
    let target = match query {
        Some(query) => format!("/v1/{route}?{query}"),
        None => format!("/v1/{route}"),
    };
    let mut all = Vec::new();
    for (member, answer) in members.iter().zip(ask_all(members, &target, timeout)) {
        match answer {
            Ok(answer) => {
                answered(&member.name);
                all.extend(rows(route, &member.name, answer));
            }
            Err(error) => all.push(unreachable(member, &error)),
        }
    }
    json!({"rows": all})
}

/// Where the events stream is with one instance: the newest `at` it sent and the rows at it, and
/// whether its unreachable row was sent since it last answered.
#[derive(Default)]
struct Cursor {
    at: Option<String>,
    sent: BTreeSet<String>,
    unreachable: bool,
}

/// `GET /v1/events?<query>` as server-sent events on `stream`: `head` first, then every
/// `EVENTS_EVERY` each member's new events, one `data:` line each with `instance`, an unreachable
/// row once each time a member stops answering, and a comment line; until a write fails, which is
/// the client going away. The first ask carries the client's `query`, each later one `since=` the
/// newest event sent.
pub fn events<S: Write>(
    stream: &mut S,
    head: &[u8],
    members: &dyn Fn() -> Vec<Member>,
    query: Option<&str>,
    timeout: Duration,
    answered: &dyn Fn(&str),
) {
    if stream
        .write_all(head)
        .and_then(|()| stream.flush())
        .is_err()
    {
        return;
    }
    let mut cursors: BTreeMap<String, Cursor> = BTreeMap::new();
    loop {
        let members = members();
        let mut by_target: Vec<String> = Vec::with_capacity(members.len());
        for member in &members {
            let cursor = cursors.entry(member.name.clone()).or_default();
            by_target.push(match (&cursor.at, query) {
                (Some(at), _) => format!("/v1/events?since={}", at.replace('+', "%2B")),
                (None, Some(query)) => format!("/v1/events?{query}"),
                (None, None) => "/v1/events".to_owned(),
            });
        }
        let answers: Vec<Result<Value, String>> = std::thread::scope(|scope| {
            let asked: Vec<_> = members
                .iter()
                .zip(&by_target)
                .map(|(member, target)| {
                    scope.spawn(move || match &member.route {
                        Some(route) => {
                            fetch(route, target, timeout).map_err(|error| format!("{error:#}"))
                        }
                        None => Err("the daemon runs no runtime of it".to_owned()),
                    })
                })
                .collect();
            asked
                .into_iter()
                .map(|handle| {
                    handle
                        .join()
                        .unwrap_or_else(|_| Err("asking it failed".to_owned()))
                })
                .collect()
        });
        let mut out = String::new();
        for (member, answer) in members.iter().zip(answers) {
            let cursor = cursors.entry(member.name.clone()).or_default();
            match answer {
                Ok(answer) => {
                    answered(&member.name);
                    cursor.unreachable = false;
                    let mut fresh = rows("events", &member.name, answer);
                    // Oldest first on the stream; the instance answers newest first.
                    fresh.reverse();
                    for row in fresh {
                        let at = row["at"].as_str().map(str::to_owned);
                        let text = row.to_string();
                        let newer = match (&at, &cursor.at) {
                            (Some(at), Some(newest)) => at > newest,
                            _ => true,
                        };
                        let same = at.is_some() && at == cursor.at;
                        if !newer && !(same && !cursor.sent.contains(&text)) {
                            continue;
                        }
                        if newer && at.is_some() {
                            cursor.at.clone_from(&at);
                            cursor.sent.clear();
                        }
                        cursor.sent.insert(text.clone());
                        out.push_str(&format!("data: {text}\n\n"));
                    }
                }
                Err(error) if !cursor.unreachable => {
                    cursor.unreachable = true;
                    out.push_str(&format!("data: {}\n\n", unreachable(member, &error)));
                }
                Err(_) => {}
            }
        }
        out.push_str(": tick\n\n");
        if stream
            .write_all(out.as_bytes())
            .and_then(|()| stream.flush())
            .is_err()
        {
            return;
        }
        std::thread::sleep(EVENTS_EVERY);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_carry_the_instance_and_are_taken_by_route() {
        let sessions = rows(
            "sessions",
            "alpha",
            json!({"rows": [{"name": "a"}], "errors": []}),
        );
        assert_eq!(sessions, vec![json!({"instance": "alpha", "name": "a"})]);
        let decisions = rows(
            "decisions",
            "alpha",
            json!({"open": [{"request_id": "r"}], "history": [{"x": 1}]}),
        );
        assert_eq!(
            decisions,
            vec![json!({"instance": "alpha", "request_id": "r"})]
        );
        let unanswered = rows("unanswered", "beta", json!({"count": 2, "oldest": null}));
        assert_eq!(
            unanswered,
            vec![json!({"instance": "beta", "count": 2, "oldest": null})]
        );
        let kpis = rows("kpis", "beta", json!([{"id": "k"}, {"id": "l"}]));
        assert_eq!(kpis.len(), 2);
        assert_eq!(wire_state(InstanceState::Unreachable), "unreachable");
    }
}
