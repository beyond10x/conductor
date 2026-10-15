//! What makes every endpoint answer warm in under 200 ms (`story:dashboard-fast`).
//!
//! Opening the store replays every file of `state/tree/`, and the first read of an observation
//! every file of `state/observations/`, every kept snapshot's ([`crate::store`]); each list of an
//! observation then folds every kept snapshot's feed. A request that opened the store afresh, as
//! each one did, paid all of that, and the overview's eight requests paid it eight times at once.
//! So:
//!
//! - [`Stores`] keeps one handle per state directory for the server's life, and every
//!   computation takes it under one lock: the handle reopens a tree only when another writer
//!   committed to it since ([`crate::store`]'s `Mark`), and two computations never replay the
//!   same files side by side.
//! - [`Memo`] keeps each answer that reads the store with the [`fingerprint`] of the store it was
//!   computed from. A request whose fingerprint is the one kept is answered from memory; one whose
//!   fingerprint moved, because something was written, computes the answer once and keeps it.
//! - [`Body`] is an answer as it is sent, with its `ETag`; a request whose `If-None-Match` names
//!   it is answered `304 Not Modified` without a body ([`send`]).
//! - The session list (`claude agents --json`), the one command a request ran, is read on a thread
//!   of its own every [`SESSIONS_EVERY`] ([`SessionList`]); a request takes the last list, with
//!   when it was read.
//!
//! The server's warmer ([`super::serve_over`]) looks at every kept answer every [`WARM_EVERY`]
//! and computes again those whose fingerprint moved, so the request after a write mostly finds
//! the new answer already computed.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant, SystemTime};

use serde_json::Value;
use time::OffsetDateTime;

use crate::store::{self, Store};

/// How often the session list is read again.
pub(super) const SESSIONS_EVERY: Duration = Duration::from_secs(10);

/// How long a request waits for the first session list before it answers without one.
const FIRST_SESSIONS: Duration = Duration::from_secs(30);

/// How often the warmer looks for answers whose store moved.
pub(super) const WARM_EVERY: Duration = Duration::from_secs(1);

/// A directory whose last change is younger than this may change again within the same tick of
/// the file system's clock, so its entries are counted too.
const SETTLED: Duration = Duration::from_secs(2);

/// The trees of a state directory a dashboard answer reads (`crate::store`'s `tree/`,
/// `observations/` and `series/`).
const TREES: [&str; 3] = ["tree", "observations", "series"];

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------------------------
// The fingerprint of a store
// ---------------------------------------------------------------------------------------------

/// Adds to `hasher` what changes whenever anything is written to the store under `state`, read
/// without opening it: for each of its trees, the description and the `tenants/` directory; for
/// each tenant, its `groups/` directory and each of that directory's buckets. Every write adds one
/// commit file to one bucket (eventlog-tree's layout), which moves that bucket's modification
/// time; a dropped tenant moves `tenants/`. A bucket changed within [`SETTLED`] has its entries
/// counted as well, so two commits in one tick of a coarse clock still read as two.
pub(super) fn fingerprint(state: &Path, hasher: &mut impl Hasher) {
    let now = SystemTime::now();
    for tree in TREES {
        let root = state.join(tree);
        tree.hash(hasher);
        stamp(&root.join("store.json"), now, hasher);
        let tenants = root.join("tenants");
        stamp(&tenants, now, hasher);
        for tenant in sorted(&tenants) {
            let groups = tenant.join("groups");
            tenant.file_name().hash(hasher);
            stamp(&groups, now, hasher);
            for bucket in sorted(&groups) {
                bucket.file_name().hash(hasher);
                stamp(&bucket, now, hasher);
            }
        }
    }
}

/// Adds the files `*.jsonl` of `dir`, each with its length and modification time, to `hasher`.
pub(super) fn files(dir: &Path, hasher: &mut impl Hasher) {
    let now = SystemTime::now();
    for file in sorted(dir) {
        file.file_name().hash(hasher);
        stamp(&file, now, hasher);
    }
}

/// The entries of `dir` in name order; none when it cannot be listed.
fn sorted(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| Some(entry.ok()?.path()))
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    entries
}

/// Adds `path`'s length and modification time to `hasher`, and for a directory changed within
/// [`SETTLED`] of `now` its number of entries; a mark of its own for a path that is not there.
fn stamp(path: &Path, now: SystemTime, hasher: &mut impl Hasher) {
    let Ok(metadata) = fs::metadata(path) else {
        u8::MAX.hash(hasher);
        return;
    };
    metadata.len().hash(hasher);
    let modified = metadata.modified().ok();
    modified.hash(hasher);
    let recent = modified.is_none_or(|modified| {
        now.duration_since(modified).is_ok_and(|age| age < SETTLED) || modified > now
    });
    if metadata.is_dir() && recent {
        fs::read_dir(path).map_or(0, Iterator::count).hash(hasher);
    }
}

/// The hasher every fingerprint and `ETag` is taken with: the same keys in every process.
pub(super) fn hasher() -> DefaultHasher {
    DefaultHasher::new()
}

// ---------------------------------------------------------------------------------------------
// The store handles
// ---------------------------------------------------------------------------------------------

/// One store handle per state directory, kept for the server's life, behind the one lock every
/// computation over a store takes.
#[derive(Default)]
pub(super) struct Stores(Mutex<HashMap<PathBuf, Store>>);

/// What a computation is given: the store, none where there is none to read, or why it does not
/// open.
pub(super) type Held<'a> = Result<Option<&'a Store>, &'a anyhow::Error>;

impl Stores {
    /// Runs `compute` with the store under `state`, opened read-only the first time
    /// ([`store::open_existing`]) and kept; none without a `state`; `Err` with why when it does
    /// not open, and nothing is kept then. A handle on which a read failed is dropped after
    /// `compute`, so the next computation opens the store afresh.
    fn with_held<R>(
        stores: &mut HashMap<PathBuf, Store>,
        state: Option<&Path>,
        compute: impl FnOnce(Held<'_>) -> R,
    ) -> R {
        let Some(state) = state else {
            return compute(Ok(None));
        };
        if !stores.contains_key(state) {
            match store::open_existing(state) {
                Ok(opened) => {
                    stores.insert(state.to_path_buf(), opened);
                }
                Err(error) => return compute(Err(&error)),
            }
        }
        let held = &stores[state];
        let answer = compute(Ok(Some(held)));
        if held.check().is_err() {
            stores.remove(state);
        }
        answer
    }

    /// [`Memo::get`] of `key` at `fingerprint`, with when it was computed; else `compute` with the
    /// store under `state` as [`Stores::with_held`] gives it, kept in `memo` at `fingerprint`.
    /// Two requests for one answer compute it once: the second waits for the lock and then finds
    /// it kept.
    pub(super) fn memoized<T: Clone>(
        &self,
        memo: &Memo<T>,
        key: &str,
        fingerprint: u64,
        asked: bool,
        state: Option<&Path>,
        compute: impl FnOnce(Held<'_>) -> T,
    ) -> (T, Instant) {
        if let Some(kept) = memo.get(key, fingerprint, asked) {
            return kept;
        }
        let mut stores = lock(&self.0);
        if let Some(kept) = memo.get(key, fingerprint, asked) {
            return kept;
        }
        let value = Self::with_held(&mut stores, state, compute);
        memo.put(key, fingerprint, asked, value.clone());
        (value, Instant::now())
    }
}

// ---------------------------------------------------------------------------------------------
// The answers kept
// ---------------------------------------------------------------------------------------------

/// How many answers one [`Memo`] keeps; past it, the one asked longest ago is dropped.
const KEPT: usize = 256;

/// How long after it was last asked an answer is still computed again by the warmer.
const WARM_FOR: Duration = Duration::from_secs(600);

/// One kept answer.
struct Kept<T> {
    fingerprint: u64,
    computed: Instant,
    asked: Instant,
    value: T,
}

/// Answers by key, each with the fingerprint it was computed at, when, and when it was last
/// asked.
pub(super) struct Memo<T>(Mutex<HashMap<String, Kept<T>>>);

impl<T> Default for Memo<T> {
    fn default() -> Self {
        Self(Mutex::new(HashMap::new()))
    }
}

impl<T: Clone> Memo<T> {
    /// The answer kept for `key`, with when it was computed, when it was computed at
    /// `fingerprint`; `asked` says a request asks, which keeps the answer warm.
    fn get(&self, key: &str, fingerprint: u64, asked: bool) -> Option<(T, Instant)> {
        let mut kept = lock(&self.0);
        let entry = kept.get_mut(key)?;
        if asked {
            entry.asked = Instant::now();
        }
        (entry.fingerprint == fingerprint).then(|| (entry.value.clone(), entry.computed))
    }

    fn put(&self, key: &str, fingerprint: u64, asked: bool, value: T) {
        let mut kept = lock(&self.0);
        let now = Instant::now();
        let last_asked = kept
            .get(key)
            .map_or(now, |entry| if asked { now } else { entry.asked });
        if kept.len() >= KEPT && !kept.contains_key(key) {
            let oldest = kept
                .iter()
                .min_by_key(|(_, entry)| entry.asked)
                .map(|(key, _)| key.clone());
            if let Some(oldest) = oldest {
                kept.remove(&oldest);
            }
        }
        kept.insert(
            key.to_owned(),
            Kept {
                fingerprint,
                computed: now,
                asked: last_asked,
                value,
            },
        );
    }

    /// The keys asked within [`WARM_FOR`], which the warmer computes again when their store
    /// moved.
    pub(super) fn keys(&self) -> Vec<String> {
        lock(&self.0)
            .iter()
            .filter(|(_, entry)| entry.asked.elapsed() < WARM_FOR)
            .map(|(key, _)| key.clone())
            .collect()
    }
}

// ---------------------------------------------------------------------------------------------
// An answer as it is sent
// ---------------------------------------------------------------------------------------------

/// An answer as it is sent: its status, its content type, its bytes and its `ETag`.
#[derive(Debug, Clone)]
pub(super) struct Body {
    pub(super) status: u16,
    pub(super) content_type: &'static str,
    pub(super) bytes: Arc<[u8]>,
    pub(super) etag: String,
}

impl Body {
    /// `value` as pretty JSON with a final newline, answered with `status`.
    pub(super) fn json(status: u16, value: &Value) -> Self {
        let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
        text.push('\n');
        Self::of(status, "application/json", text)
    }

    /// `text` as plain text, answered with `status`.
    pub(super) fn text(status: u16, text: String) -> Self {
        Self::of(status, "text/plain; charset=utf-8", text)
    }

    fn of(status: u16, content_type: &'static str, text: String) -> Self {
        let mut hasher = hasher();
        status.hash(&mut hasher);
        text.hash(&mut hasher);
        Self {
            status,
            content_type,
            etag: format!("\"{:016x}-{:x}\"", hasher.finish(), text.len()),
            bytes: Arc::from(text.into_bytes()),
        }
    }
}

/// Whether the `If-None-Match` value `condition` names `etag`: it lists it, or is `*`.
pub(super) fn matches(condition: &str, etag: &str) -> bool {
    condition
        .split(',')
        .map(str::trim)
        .any(|named| named == "*" || named == etag || named.strip_prefix("W/") == Some(etag))
}

/// `body` as an HTTP answer to a request whose `If-None-Match` is `condition`: `304` without a
/// body when it names the body's `ETag`, else the body. Either carries the `ETag`, `Cache-Control:
/// no-cache` (a client keeps the answer and asks again with its `ETag`) and, for an answer kept
/// since `computed`, its `Age` in seconds.
pub(super) fn send(body: &Body, condition: Option<&str>, computed: Option<Instant>) -> Vec<u8> {
    let mut extra = format!("ETag: {}\r\n", body.etag);
    if let Some(computed) = computed {
        extra.push_str(&format!("Age: {}\r\n", computed.elapsed().as_secs()));
    }
    if condition.is_some_and(|condition| matches(condition, &body.etag)) {
        return super::response_with(
            304,
            "Not Modified",
            body.content_type,
            b"",
            "no-cache",
            &extra,
        );
    }
    let reason = match body.status {
        200 => "OK",
        400 => "Bad Request",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    super::response_with(
        body.status,
        reason,
        body.content_type,
        &body.bytes,
        "no-cache",
        &extra,
    )
}

// ---------------------------------------------------------------------------------------------
// The session list
// ---------------------------------------------------------------------------------------------

/// One reading of the session list: its rows, what could not be read, and when it was read.
#[derive(Debug, Clone, Default)]
pub(super) struct Listed {
    pub(super) rows: Vec<Value>,
    pub(super) errors: Vec<String>,
    pub(super) at: Option<OffsetDateTime>,
}

/// The last reading of the session list, which a thread of its own renews every
/// [`SESSIONS_EVERY`]; shared by every request.
#[derive(Debug, Clone, Default)]
pub(super) struct SessionList(Arc<(Mutex<Option<Listed>>, Condvar)>);

impl SessionList {
    /// Reads the session list `command` prints now, and again every `every`, on a thread of its
    /// own, for as long as the process runs.
    pub(super) fn start(&self, command: Vec<OsString>, every: Duration) {
        let list = self.clone();
        std::thread::spawn(move || {
            loop {
                let mut errors = Vec::new();
                let rows = super::agents(&command, &mut errors);
                let read = Listed {
                    rows,
                    errors,
                    at: Some(OffsetDateTime::now_utc()),
                };
                let (last, ready) = &*list.0;
                *lock(last) = Some(read);
                ready.notify_all();
                std::thread::sleep(every);
            }
        });
    }

    /// The last reading; the first one is waited for, up to [`FIRST_SESSIONS`], and without it
    /// the answer has no rows and says why.
    pub(super) fn last(&self) -> Listed {
        let (last, ready) = &*self.0;
        let guard = lock(last);
        let (guard, _) = ready
            .wait_timeout_while(guard, FIRST_SESSIONS, |last| last.is_none())
            .unwrap_or_else(PoisonError::into_inner);
        guard.clone().unwrap_or_else(|| Listed {
            errors: vec!["the session list has not been read yet".to_owned()],
            ..Listed::default()
        })
    }
}
