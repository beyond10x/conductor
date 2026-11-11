//! conductord's probes and metrics (`story:daemon-probes-and-metrics`): three paths both
//! listeners answer to `GET` without a bearer, ahead of the dashboard app, for a cluster that
//! restarts and routes the daemon by probes and a Prometheus that scrapes it. The loopback
//! listener's `Host` check stays ahead of them.
//!
//! - [`LIVENESS`], `/healthz`: `200 {"state":"serving"}` while the process serves, whatever the
//!   store does.
//! - [`READINESS`], `/readyz`: the one row of the view `conductor.store.Readiness`,
//!   `[{"reachable", "reason"}]`. `200` once the config is read (no listener exists before), the
//!   store answered one read no longer than [`STORE_BOUND`] ago ([`watch`] asks it every tick),
//!   the reconciler published once, and its last record did not fail without one written since;
//!   `503` with the one-line reason otherwise ([`readiness`]).
//! - [`METRICS`], `/metrics`: Prometheus text format ([`render`]). `conductor_instances` by
//!   `state`, as the reconciler last saw them; `conductor_sessions` by `instance` and `state` and
//!   `conductor_snapshot_age_seconds` by `instance`, read from each instance's `/v1/data.json`
//!   at the scrape, all of them within [`BOUND`] (`daemon.timeout` when that is shorter);
//!   `conductor_instances_unanswered`, how many instances that scrape holds no reading of; and
//!   `conductor_build_queue_length`, the disk requests still waiting in the host's build ledger
//!   ([`crate::store::queued`]). A value the daemon could not read is absent, never `0`: nothing
//!   of the instances before the reconciler published, no session or snapshot line of an
//!   instance that did not answer, no queue length of a ledger that does not read. One answer is
//!   kept for [`KEEP`], so scrapes without a bearer ask the instances no more often than that.
//!
//! **What a body holds.** No bearer guards these paths, so a body holds states, counts, ages and
//! a fixed one-line reason only: no config value, token, path, store URL, repository or session
//! name. An instance is therefore labelled by its position in the config file (`1` is the first),
//! not by its name, and a session's state is cut down to a label's letters ([`label`]).

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError, mpsc};
use std::thread;
use std::time::{Duration, Instant};

use conductor_model::instance::InstanceState;
use conductor_model::store::Readiness;
use serde_json::{Value, json};

use super::api::Api;
use super::federate::{self, Member};

/// The liveness probe.
pub const LIVENESS: &str = "/healthz";

/// The readiness probe.
pub const READINESS: &str = "/readyz";

/// The metrics.
pub const METRICS: &str = "/metrics";

/// How old the store's last answer may be for the daemon to be ready: a read that hangs is no
/// answer once this has passed.
pub const STORE_BOUND: Duration = Duration::from_secs(10);

/// How long every instance has, all asked at once, to answer one scrape.
pub const BOUND: Duration = Duration::from_secs(3);

/// How long one answer of [`METRICS`] is kept.
pub const KEEP: Duration = Duration::from_secs(5);

/// The content type of the Prometheus text format.
pub const TEXT: &str = "text/plain; version=0.0.4; charset=utf-8";

/// What a scrape asks an instance: its dashboard's data, which holds its sessions and its newest
/// complete snapshot.
const DATA: &str = "/v1/data.json";

/// Every state of an instance, so a state no instance is in reads `0`.
const STATES: [InstanceState; 6] = [
    InstanceState::Purged,
    InstanceState::Running,
    InstanceState::Starting,
    InstanceState::Stopped,
    InstanceState::Unreachable,
    InstanceState::Upgrading,
];

/// Why the daemon is not ready while its reconciler has published nothing.
pub const NOT_PUBLISHED: &str = "the reconciler has not published yet";

/// Why the daemon is not ready while its reconciler's last record failed and none was written
/// since.
pub const UNRECORDED: &str = "the reconciler's last record failed";

/// What the probes keep between requests.
#[derive(Debug, Default)]
pub struct Probes {
    /// Whether the store answered the last read asked of it, and when; none before the first.
    store: Mutex<Option<(bool, Instant)>>,
    /// Whether the reconciler's last record failed and none was written since.
    unrecorded: AtomicBool,
    /// The last answer of [`METRICS`] and when it was read.
    scraped: Mutex<Option<(Instant, String)>>,
}

impl Probes {
    /// Notes that the store answered a read at `at`, or did not.
    pub fn store(&self, answers: bool, at: Instant) {
        *self.store.lock().unwrap_or_else(PoisonError::into_inner) = Some((answers, at));
    }

    /// Whether the store answered the last read asked of it, and when; none before the first.
    #[must_use]
    pub fn seen(&self) -> Option<(bool, Instant)> {
        *self.store.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Notes whether the reconciler's last record failed and none was written since.
    pub fn unrecorded(&self, failed: bool) {
        self.unrecorded.store(failed, Ordering::Release);
    }
}

/// Why the daemon of `api` is not ready at `now`, as [`READINESS`] answers it; none when it is:
/// its store does not answer ([`not_ready`]), its reconciler has published nothing
/// ([`NOT_PUBLISHED`]), or the reconciler's last record failed and none was written since
/// ([`UNRECORDED`]). Each reason is a fixed sentence, never the store's error.
#[must_use]
pub fn readiness(api: &Api, now: Instant) -> Option<&'static str> {
    not_ready(api.probes().seen(), now)
        .or_else(|| (!api.is_reconciled()).then_some(NOT_PUBLISHED))
        .or_else(|| {
            api.probes()
                .unrecorded
                .load(Ordering::Acquire)
                .then_some(UNRECORDED)
        })
}

/// Why the daemon is not ready at `now`, given what the store last answered (`seen`); none when
/// it is. Each reason is a fixed sentence.
#[must_use]
pub fn not_ready(seen: Option<(bool, Instant)>, now: Instant) -> Option<&'static str> {
    match seen {
        None => Some("the store has not answered yet"),
        Some((false, _)) => Some("the store does not answer"),
        Some((true, at)) if now.saturating_duration_since(at) > STORE_BOUND => {
            Some("the store has not answered within 10 s")
        }
        Some((true, _)) => None,
    }
}

/// Asks the store at `url` one read every `every` for as long as the process runs, and publishes
/// to `api` whether it answered: what [`READINESS`] answers from, and what `daemon serve` waits
/// for before it opens its store. The handle is this thread's own; one that fails is dropped and
/// the store opened again at the next turn, which creates a SQLite database that is not there as
/// the daemon's own opening does. A change is one line: on standard error when the store stops
/// answering, with why, on standard output when it answers again.
pub fn watch(url: &str, api: &Api, every: Duration) -> ! {
    let mut held = None;
    let mut said = None;
    loop {
        let mut why = None;
        if held.is_none() {
            match crate::store::open_url(url, super::TENANT) {
                Ok(store) => held = Some(store),
                Err(error) => why = Some(format!("{error:#}")),
            }
        }
        let answers = held.as_ref().is_some_and(crate::store::Store::answers);
        if !answers {
            held = None;
        }
        api.publish_store(answers);
        if said != Some(answers) {
            if !answers {
                eprintln!(
                    "conductor: daemon serve: daemon.store: {}; {READINESS} answers 503 and \
                     nothing is reconciled until it answers",
                    why.as_deref()
                        .unwrap_or("it did not answer a read")
                        .replace(['\n', '\r'], " ")
                );
            } else if said.is_some() {
                println!("conductord: the store answers; {READINESS} answers 200");
            }
            said = Some(answers);
        }
        thread::sleep(every);
    }
}

/// One answer of the three paths.
pub(super) struct Answer {
    pub(super) status: u16,
    pub(super) reason: &'static str,
    pub(super) content_type: &'static str,
    pub(super) body: String,
    /// Further header lines, each ending in CRLF.
    pub(super) headers: &'static str,
}

impl Answer {
    fn json(status: u16, reason: &'static str, body: &Value) -> Self {
        Self {
            status,
            reason,
            content_type: "application/json",
            body: format!("{body}\n"),
            headers: "",
        }
    }
}

/// The answer to `method` on `path` when it is one of the three paths; none for any other path.
pub(super) fn answer(method: &str, path: &str, api: &Api) -> Option<Answer> {
    if ![LIVENESS, READINESS, METRICS].contains(&path) {
        return None;
    }
    if method != "GET" {
        return Some(Answer {
            headers: "Allow: GET\r\n",
            ..Answer::json(
                405,
                "Method Not Allowed",
                &json!({"error": format!("{path} answers GET only")}),
            )
        });
    }
    Some(match path {
        LIVENESS => Answer::json(200, "OK", &json!({"state": "serving"})),
        READINESS => {
            let reason = readiness(api, Instant::now());
            let row = Readiness {
                reachable: reason.is_none(),
                reason: reason.map(str::to_owned),
            };
            let (status, phrase) = if row.reachable {
                (200, "OK")
            } else {
                (503, "Service Unavailable")
            };
            Answer::json(
                status,
                phrase,
                &json!([{"reachable": row.reachable, "reason": row.reason}]),
            )
        }
        _ => Answer {
            status: 200,
            reason: "OK",
            content_type: TEXT,
            body: scrape(api),
            headers: "",
        },
    })
}

/// The metrics of `api`: the answer kept, when it is younger than [`KEEP`], else a new reading.
/// Scrapes at the same time take turns, so they share one reading.
fn scrape(api: &Api) -> String {
    let mut kept = api
        .probes()
        .scraped
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if let Some((at, text)) = kept.as_ref()
        && at.elapsed() < KEEP
    {
        return text.clone();
    }
    let seen = api.reconciled();
    let text = render(&Scrape {
        instances: seen.as_ref().map(|(rows, _)| by_state(rows)),
        readings: seen
            .as_ref()
            .map(|(_, members)| readings(members, BOUND.min(api.timeout()))),
        queue: api.host_dir().and_then(|host| crate::store::queued(&host)),
    });
    *kept = Some((Instant::now(), text.clone()));
    text
}

/// What one scrape read.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Scrape {
    /// How many instances are in each state; none before the reconciler published.
    pub instances: Option<BTreeMap<&'static str, usize>>,
    /// What each instance answered, in the config file's order, none for one that did not; none
    /// before the reconciler published.
    pub readings: Option<Vec<Option<Reading>>>,
    /// The disk requests waiting in the host's build ledger; none when it does not read.
    pub queue: Option<usize>,
}

/// What one instance answered a scrape.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Reading {
    /// How many of its sessions are in each state.
    pub sessions: BTreeMap<String, usize>,
    /// The age of its newest complete snapshot, in seconds; none when it holds none.
    pub snapshot_age: Option<i64>,
}

/// How many of the reconciler's `rows` are in each state, every state named.
fn by_state(rows: &[Value]) -> BTreeMap<&'static str, usize> {
    STATES
        .iter()
        .map(|state| {
            let name = super::reconcile::state_name(*state);
            let count = rows
                .iter()
                .filter(|row| row["state"].as_str() == Some(name))
                .count();
            (federate::wire_state(*state), count)
        })
        .collect()
}

/// Asks every one of `members` that has a route for its sessions and its newest snapshot, all at
/// once, and answers what each answered within `bound`, in their order: none for a member
/// without a route, one that did not answer in time and one whose answer does not read. An
/// instance still silent at the bound is left to its own thread, which ends with its exchange.
#[must_use]
pub fn readings(members: &[Member], bound: Duration) -> Vec<Option<Reading>> {
    let deadline = Instant::now() + bound;
    let (sender, receiver) = mpsc::channel();
    let mut asked = 0_usize;
    for (index, member) in members.iter().enumerate() {
        let Some(route) = member.route.clone() else {
            continue;
        };
        let sender = sender.clone();
        let spawned = thread::Builder::new().spawn(move || {
            let read = federate::fetch(&route, DATA, bound)
                .ok()
                .and_then(|data| reading(&data));
            let _ = sender.send((index, read));
        });
        if spawned.is_ok() {
            asked += 1;
        }
    }
    drop(sender);
    let mut answers = vec![None; members.len()];
    while asked > 0 {
        let left = deadline.saturating_duration_since(Instant::now());
        let Ok((index, read)) = receiver.recv_timeout(left) else {
            break;
        };
        answers[index] = read;
        asked -= 1;
    }
    answers
}

/// What an instance's `data` holds for a scrape; none when it holds no list of sessions.
fn reading(data: &Value) -> Option<Reading> {
    let mut sessions = BTreeMap::new();
    for session in data.get("sessions")?.as_array()? {
        let state = session.get("state").and_then(Value::as_str).unwrap_or("");
        *sessions.entry(label(state)).or_insert(0) += 1;
    }
    Some(Reading {
        sessions,
        snapshot_age: data
            .get("snapshot")
            .and_then(|snapshot| snapshot.get("age_seconds"))
            .and_then(Value::as_i64),
    })
}

/// `state` as a label's value: its ASCII letters and digits in lower case, anything else `_`, at
/// most 40 characters; `unknown` for an empty one. A label so made carries no path and no line
/// break, whatever the instance answered.
#[must_use]
pub fn label(state: &str) -> String {
    let label: String = state
        .trim()
        .chars()
        .take(40)
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if label.is_empty() {
        "unknown".to_owned()
    } else {
        label
    }
}

/// `scrape` in the Prometheus text format. A metric the scrape holds no value of has no line at
/// all, its `HELP` and `TYPE` included.
#[must_use]
pub fn render(scrape: &Scrape) -> String {
    let mut text = String::new();
    if let Some(instances) = &scrape.instances {
        head(
            &mut text,
            "conductor_instances",
            "Instances of the config file by the state the daemon last saw them in.",
        );
        for (state, count) in instances {
            let _ = writeln!(text, "conductor_instances{{state=\"{state}\"}} {count}");
        }
    }
    if let Some(readings) = &scrape.readings {
        head(
            &mut text,
            "conductor_instances_unanswered",
            "Instances this scrape holds no sessions and no snapshot age of: without a runtime, \
             or silent within the scrape's bound.",
        );
        let unanswered = readings.iter().filter(|read| read.is_none()).count();
        let _ = writeln!(text, "conductor_instances_unanswered {unanswered}");
        let answered = || {
            readings
                .iter()
                .enumerate()
                .filter_map(|(index, read)| Some((index + 1, read.as_ref()?)))
        };
        if answered().any(|(_, read)| !read.sessions.is_empty()) {
            head(
                &mut text,
                "conductor_sessions",
                "Sessions by instance (its position in the config file) and state.",
            );
            for (instance, read) in answered() {
                for (state, count) in &read.sessions {
                    let _ = writeln!(
                        text,
                        "conductor_sessions{{instance=\"{instance}\",state=\"{state}\"}} {count}"
                    );
                }
            }
        }
        if answered().any(|(_, read)| read.snapshot_age.is_some()) {
            head(
                &mut text,
                "conductor_snapshot_age_seconds",
                "Age of the newest complete snapshot by instance (its position in the config \
                 file).",
            );
            for (instance, read) in answered() {
                if let Some(age) = read.snapshot_age {
                    let _ = writeln!(
                        text,
                        "conductor_snapshot_age_seconds{{instance=\"{instance}\"}} {age}"
                    );
                }
            }
        }
    }
    if let Some(queue) = scrape.queue {
        head(
            &mut text,
            "conductor_build_queue_length",
            "Disk requests waiting in the host's build ledger.",
        );
        let _ = writeln!(text, "conductor_build_queue_length {queue}");
    }
    text
}

/// The `HELP` and `TYPE` lines of the gauge `name`.
fn head(text: &mut String, name: &str, help: &str) {
    let _ = writeln!(text, "# HELP {name} {help}\n# TYPE {name} gauge");
}
