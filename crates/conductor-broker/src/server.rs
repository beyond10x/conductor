//! The broker's sockets: one listener per registered session, and the control socket
//! [`crate::register`] asks to open a session's socket while the broker serves.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::registry::{self, control_path, socket_path, well_formed};
use crate::services::Services;
use crate::wire::{Answer, Request};
use crate::{CallOutcome, Verb, outcome_word, verb_named, verb_word, verbs};

/// One call as the broker records it (`conductor.tool.RecordToolCall`): the session whose socket
/// it came in on, the broker's instance, the verb, the paths it named and no other argument text,
/// how it ended, the grant rule a refusal broke, how long it took, the bytes it answered, and
/// when it was called (RFC 3339, UTC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallRecord {
    pub session: String,
    pub instance: String,
    pub verb: Verb,
    pub paths: Vec<String>,
    pub outcome: CallOutcome,
    pub rule: Option<String>,
    pub duration_ms: i64,
    pub output_bytes: i64,
    pub called_at: String,
}

/// Where the broker records each call: the instance's store in `conductor broker serve`.
pub trait Recorder: Send + Sync {
    /// Records one call. A call whose record fails is still answered; the recorder reports its
    /// own failure.
    fn record(&self, call: &CallRecord);
}

/// A recorder that keeps every call in memory, in the order recorded.
#[derive(Debug, Default)]
pub struct MemoryRecorder {
    calls: Mutex<Vec<CallRecord>>,
}

impl MemoryRecorder {
    /// Every call recorded so far.
    #[must_use]
    pub fn calls(&self) -> Vec<CallRecord> {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Recorder for MemoryRecorder {
    fn record(&self, call: &CallRecord) {
        self.calls
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(call.clone());
    }
}

/// What a session's earlier reads were: per range, the hash of what was shown and when.
pub(crate) type Reads = HashMap<(String, String), (u64, String)>;

pub(crate) struct Inner {
    pub dir: PathBuf,
    pub instance: String,
    recorder: Arc<dyn Recorder>,
    pub services: Services,
    stop: AtomicBool,
    listeners: Mutex<BTreeMap<String, JoinHandle<()>>>,
    pub reads: Mutex<Reads>,
}

/// The broker of one instance, serving every registered session on its own socket until
/// [`Broker::shutdown`].
pub struct Broker {
    inner: Arc<Inner>,
    control: Option<JoinHandle<()>>,
}

impl Broker {
    /// Starts the broker of `instance` over the directory `dir` (`<state>/broker`): binds the
    /// control socket and a socket for each registered session, and records each call with
    /// `recorder`. A socket file a broker left behind is replaced.
    ///
    /// # Errors
    ///
    /// When the directory or a socket cannot be made.
    pub fn start(dir: &Path, instance: &str, recorder: Arc<dyn Recorder>) -> io::Result<Self> {
        Self::start_with(dir, instance, recorder, Services::none())
    }

    /// [`Broker::start`], with the ledger, the bot route and the store the build, commit and
    /// team verbs reach through `services`.
    ///
    /// # Errors
    ///
    /// When the directory or a socket cannot be made.
    pub fn start_with(
        dir: &Path,
        instance: &str,
        recorder: Arc<dyn Recorder>,
        services: Services,
    ) -> io::Result<Self> {
        fs::create_dir_all(registry::sessions_dir(dir))?;
        let sockets = dir.join("sockets");
        fs::create_dir_all(&sockets)?;
        fs::set_permissions(&sockets, fs::Permissions::from_mode(0o700))?;
        if let Ok(entries) = fs::read_dir(&sockets) {
            for entry in entries.filter_map(Result::ok) {
                let _ = fs::remove_file(entry.path());
            }
        }
        let control_socket = control_path(dir);
        let _ = fs::remove_file(&control_socket);
        let control = UnixListener::bind(&control_socket)?;
        fs::set_permissions(&control_socket, fs::Permissions::from_mode(0o600))?;
        let inner = Arc::new(Inner {
            dir: dir.to_owned(),
            instance: instance.to_owned(),
            recorder,
            services,
            stop: AtomicBool::new(false),
            listeners: Mutex::new(BTreeMap::new()),
            reads: Mutex::new(HashMap::new()),
        });
        for session in registry::sessions(dir) {
            open(&inner, &session)?;
        }
        let serving = Arc::clone(&inner);
        let control = thread::spawn(move || serve_control(&serving, &control));
        Ok(Self {
            inner,
            control: Some(control),
        })
    }

    /// Stops serving: closes every socket, waits for the calls in progress, and removes the
    /// socket files.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let inner = &self.inner;
        if inner.stop.swap(true, Ordering::SeqCst) {
            return;
        }
        let _ = UnixStream::connect(control_path(&inner.dir));
        if let Some(control) = self.control.take() {
            let _ = control.join();
        }
        let listeners = std::mem::take(
            &mut *inner
                .listeners
                .lock()
                .unwrap_or_else(PoisonError::into_inner),
        );
        for (session, listener) in listeners {
            let socket = socket_path(&inner.dir, &session);
            let _ = UnixStream::connect(&socket);
            let _ = listener.join();
            let _ = fs::remove_file(socket);
        }
        let _ = fs::remove_file(control_path(&inner.dir));
    }
}

impl Drop for Broker {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Binds `session`'s socket, unless it is bound already, and serves it on its own thread.
fn open(inner: &Arc<Inner>, session: &str) -> io::Result<PathBuf> {
    let socket = socket_path(&inner.dir, session);
    let mut listeners = inner
        .listeners
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if listeners.contains_key(session) {
        return Ok(socket);
    }
    let _ = fs::remove_file(&socket);
    let listener = UnixListener::bind(&socket)?;
    fs::set_permissions(&socket, fs::Permissions::from_mode(0o600))?;
    let serving = Arc::clone(inner);
    let name = session.to_owned();
    listeners.insert(
        session.to_owned(),
        thread::spawn(move || serve_session(&serving, &name, &listener)),
    );
    Ok(socket)
}

fn serve_control(inner: &Arc<Inner>, listener: &UnixListener) {
    for stream in listener.incoming() {
        if inner.stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(mut stream) = stream else { continue };
        let mut line = String::new();
        if BufReader::new(&stream).read_line(&mut line).is_err() {
            continue;
        }
        let request: Value = serde_json::from_str(&line).unwrap_or(Value::Null);
        let reply = match request["open"].as_str() {
            Some(session) if well_formed(session) => match open(inner, session) {
                Ok(socket) => json!({"socket": socket.to_string_lossy()}),
                Err(error) => json!({"error": error.to_string()}),
            },
            _ => json!({"error": "no session to open"}),
        };
        let _ = writeln!(stream, "{reply}");
    }
}

fn serve_session(inner: &Arc<Inner>, session: &str, listener: &UnixListener) {
    for stream in listener.incoming() {
        if inner.stop.load(Ordering::SeqCst) {
            return;
        }
        let Ok(stream) = stream else { continue };
        let inner = Arc::clone(inner);
        let session = session.to_owned();
        thread::spawn(move || answer(&inner, &session, stream));
    }
}

/// Reads one request from `stream`, answers it as `session`, and closes.
fn answer(inner: &Inner, session: &str, mut stream: UnixStream) {
    let mut line = String::new();
    if BufReader::new(&stream).read_line(&mut line).is_err() {
        return;
    }
    let answer = match Request::from_line(&line) {
        Some(request) => handle(inner, session, &request),
        None => Answer::refused("NotGranted", "tools: the request is no call"),
    };
    let _ = writeln!(stream, "{}", answer.to_line());
}

/// The answer to `request` on `session`'s socket: the session's registration, its instance and
/// its grant are checked in that order, then the verb runs; every call of a verb is recorded.
/// `grant`, which names no verb, answers the granted verbs' words and is not recorded.
pub(crate) fn handle(inner: &Inner, session: &str, request: &Request) -> Answer {
    let started = Instant::now();
    let now = OffsetDateTime::now_utc();
    let checked = check(inner, session);
    if request.verb == "grant" {
        return match checked {
            Ok(registration) => Answer::done(
                registration
                    .grant
                    .tools
                    .iter()
                    .map(|verb| verb_word(*verb))
                    .collect::<Vec<_>>()
                    .join(" ")
                    + "\n",
            ),
            Err((outcome, rule)) => Answer::refused(outcome_word(outcome), &rule),
        };
    }
    let Some(verb) = verb_named(&request.verb) else {
        return Answer::refused("NotGranted", &format!("tools: {}", request.verb));
    };
    let paths = verbs::paths(verb, &request.args);
    let (answer, outcome, rule) = match checked.and_then(|registration| {
        registration
            .grant
            .check_verb(verb, now)
            .map_err(|refusal| (refusal.outcome, refusal.rule))
            .map(|()| registration)
    }) {
        Ok(registration) => {
            let context = verbs::Context {
                dir: &inner.dir,
                session,
                instance: &inner.instance,
                registration: &registration,
                reads: &inner.reads,
                services: &inner.services,
                now,
            };
            match verbs::run(&context, verb, &request.args) {
                Ok(answer) => {
                    let outcome = if answer.exit == 0 {
                        CallOutcome::Done
                    } else {
                        CallOutcome::Failed
                    };
                    (answer, outcome, None)
                }
                Err(refusal) => (
                    Answer::refused(outcome_word(refusal.outcome), &refusal.rule),
                    refusal.outcome,
                    Some(refusal.rule),
                ),
            }
        }
        Err((outcome, rule)) => (
            Answer::refused(outcome_word(outcome), &rule),
            outcome,
            Some(rule),
        ),
    };
    inner.recorder.record(&CallRecord {
        session: session.to_owned(),
        instance: inner.instance.clone(),
        verb,
        paths,
        outcome,
        rule,
        duration_ms: i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX),
        output_bytes: i64::try_from(answer.text.len()).unwrap_or(i64::MAX),
        called_at: rfc3339(now),
    });
    answer
}

/// The session's registration, when it is registered, belongs to this broker's instance, and
/// can be read; else the refusal and what was found.
fn check(inner: &Inner, session: &str) -> Result<registry::Registration, (CallOutcome, String)> {
    let registration = match registry::read(&inner.dir, session) {
        None => {
            return Err((
                CallOutcome::UnknownSession,
                format!(
                    "session {session} is not registered with this broker; it stopped or never started"
                ),
            ));
        }
        Some(Err(why)) => {
            return Err((
                CallOutcome::UnknownSession,
                format!("the registration of session {session} cannot be read: {why}"),
            ));
        }
        Some(Ok(registration)) => registration,
    };
    if registration.instance != inner.instance || registration.session != session {
        return Err((
            CallOutcome::ForeignInstance,
            format!(
                "session {} belongs to instance {}; this broker serves {}",
                registration.session, registration.instance, inner.instance
            ),
        ));
    }
    Ok(registration)
}

/// `at` as RFC 3339, UTC, to the second.
pub(crate) fn rfc3339(at: OffsetDateTime) -> String {
    at.replace_nanosecond(0)
        .unwrap_or(at)
        .format(&Rfc3339)
        .unwrap_or_default()
}
