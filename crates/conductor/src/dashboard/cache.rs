//! What makes every endpoint answer warm in under 200 ms (`story:dashboard-fast`).
//!
//! Opening the store replays every file of `state/tree/`, and the first read of an observation
//! every file of `state/observations/`, every kept snapshot's ([`crate::store`]); each list of an
//! observation then folds every kept snapshot's feed. A request that opened the store afresh, as
//! each one did, paid all of that, and the overview's eight requests paid it eight times at once.
//! So:
//!
//! - [`Stores`] keeps one handle per state directory for the server's life, and every
//!   computation reads it on one thread of its own, one after another: the handle reopens a tree
//!   only when another writer committed to it since ([`crate::store`]'s `Mark`), two computations
//!   never replay the same files side by side, and every replay is built on the same thread, so
//!   the allocator reuses what the last one freed (`story:dashboard-memory-live`). The handle is a reader ([`store::open_reader`]): it never waits on
//!   the store's writer lock, which a writer holds for as long as it commits.
//! - [`Memo`] keeps each answer that reads the store with the [`fingerprint`] of the store it was
//!   computed from. A request whose fingerprint is the one kept is answered from memory. One whose
//!   fingerprint moved, because something was written, is answered from memory too, at once and
//!   marked stale (`X-Conductor-Age`), while one computation of the answer runs on a thread of its
//!   own: never two of one answer at once, and for an answer kept, at most one started per
//!   [`DEBOUNCE`], since the store is written almost constantly. Only a request for an answer never
//!   computed waits, up to [`COLD_WAIT`], and is answered `503` with `Retry-After` after that.
//! - [`Body`] is an answer as it is sent, with its `ETag`; a request whose `If-None-Match` names
//!   it is answered `304 Not Modified` without a body ([`send`]).
//! - The session list (`claude agents --json`), the one command a request ran, is read on a thread
//!   of its own every [`SESSIONS_EVERY`] ([`SessionList`]); a request takes the last list, with
//!   when it was read.
//!
//! The server's warmer ([`super::serve_over`]) computes every answer once at start, and looks at
//! every kept answer every [`WARM_EVERY`] and starts a computation of those whose fingerprint
//! moved, as the debounce lets it, so the request after a write mostly finds the new answer
//! already computed.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::hash::{Hash, Hasher};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc};
use std::thread::ThreadId;
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

/// How long after a recompute of an answer started the next one may start: a store written
/// almost constantly (the guard's hooks, messages, a snapshot's collectors) does not cause
/// recomputes back to back.
pub const DEBOUNCE: Duration = Duration::from_secs(15);

/// How long a request for an answer that was never computed waits for it before it is answered
/// `503` with `Retry-After`.
pub const COLD_WAIT: Duration = Duration::from_secs(5);

/// The `Retry-After` of a `503` answered before an answer was first computed, in seconds.
pub(super) const RETRY_AFTER: u64 = 5;

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

// ---------------------------------------------------------------------------------------------
// The tuning and the probe
// ---------------------------------------------------------------------------------------------

/// How the server keeps its answers: [`Tuning::default`] is what `dashboard serve` runs with; a
/// test shortens the waits and sets a [`Probe`].
#[derive(Debug, Clone)]
pub struct Tuning {
    /// [`DEBOUNCE`].
    pub debounce: Duration,
    /// [`COLD_WAIT`].
    pub cold_wait: Duration,
    /// What a test watches the computations with; none in `dashboard serve`.
    pub probe: Option<Arc<Probe>>,
}

impl Default for Tuning {
    fn default() -> Self {
        Self {
            debounce: DEBOUNCE,
            cold_wait: COLD_WAIT,
            probe: None,
        }
    }
}

/// A test's view of the computations: a delay each one waits before it computes, standing in for
/// a slow store, and when each computation of each key started and how many ran at once.
#[derive(Debug, Default)]
pub struct Probe(Mutex<Probed>);

#[derive(Debug, Default)]
struct Probed {
    delay: Duration,
    started: HashMap<String, Vec<Instant>>,
    running: HashMap<String, usize>,
    peak: HashMap<String, usize>,
    /// The store reads, and the threads they ran on.
    reads: usize,
    threads: HashSet<ThreadId>,
}

impl Probe {
    /// Every computation started from now on first waits `delay`.
    pub fn set_delay(&self, delay: Duration) {
        lock(&self.0).delay = delay;
    }

    /// How many computations of `key` started.
    #[must_use]
    pub fn starts(&self, key: &str) -> usize {
        self.started(key).len()
    }

    /// When each computation of `key` started, in order.
    #[must_use]
    pub fn started(&self, key: &str) -> Vec<Instant> {
        lock(&self.0).started.get(key).cloned().unwrap_or_default()
    }

    /// The most computations of `key` that ran at once.
    #[must_use]
    pub fn peak(&self, key: &str) -> usize {
        lock(&self.0).peak.get(key).copied().unwrap_or_default()
    }

    /// How many computations read the store (`story:dashboard-memory-live`).
    #[must_use]
    pub fn store_reads(&self) -> usize {
        lock(&self.0).reads
    }

    /// On how many threads the computations read the store.
    #[must_use]
    pub fn store_threads(&self) -> usize {
        lock(&self.0).threads.len()
    }

    /// A computation reads the store on this thread.
    fn read(&self) {
        let mut probed = lock(&self.0);
        probed.reads += 1;
        probed.threads.insert(std::thread::current().id());
    }

    /// A computation of `key` starts; its delay.
    fn start(&self, key: &str) -> Duration {
        let mut probed = lock(&self.0);
        probed
            .started
            .entry(key.to_owned())
            .or_default()
            .push(Instant::now());
        let running = probed.running.entry(key.to_owned()).or_default();
        *running += 1;
        let running = *running;
        let peak = probed.peak.entry(key.to_owned()).or_default();
        *peak = (*peak).max(running);
        probed.delay
    }

    /// A computation of `key` ends.
    fn end(&self, key: &str) {
        if let Some(running) = lock(&self.0).running.get_mut(key) {
            *running = running.saturating_sub(1);
        }
    }
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

/// One store handle per state directory, kept for the server's life and read on one thread of
/// its own, the store reader, one computation after another; with the probe of a test.
///
/// Every store computation runs on that one thread (`story:dashboard-memory-live`). The
/// allocator keeps what a thread freed in that thread's arena for its later allocations: when
/// each recompute ran on a thread of its own, each replay was built in whichever arena that
/// thread took, and every arena a reopen ran in kept about a replay's size after the replay was
/// dropped, so the process grew by a replay per arena. On one thread the next replay reuses what
/// the last one freed.
pub(super) struct Stores {
    shared: Arc<Shared>,
    /// Sends a computation to the store reader; none when its thread could not start, and each
    /// computation then runs on the thread that asks for it.
    reader: Option<Mutex<mpsc::Sender<Job>>>,
}

/// What the store reader and the server share: the handles, what they hold, and the probe.
struct Shared {
    handles: Mutex<HashMap<PathBuf, Store>>,
    held: Mutex<StoresHeld>,
    probe: Option<Arc<Probe>>,
}

/// One computation for the store reader.
type Job = Box<dyn FnOnce() + Send>;

/// What the store handles hold in memory, as the last computation left them
/// (`story:dashboard-memory-bound`): read without the lock a computation holds.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct StoresHeld {
    /// The handles kept: one per state directory.
    pub(super) handles: usize,
    /// The trees they hold replayed.
    pub(super) replays: usize,
    /// The most replays of one tree alive at once in any handle since the server started.
    pub(super) peak: usize,
    /// The bytes of the files those replays were built from.
    pub(super) bytes: u64,
}

/// What a computation is given: the store, none where there is none to read, or why it does not
/// open.
pub(super) type Held<'a> = Result<Option<&'a Store>, &'a anyhow::Error>;

impl Stores {
    /// The handles, none opened yet, watched by `probe`, and the store reader's thread, which ends
    /// when these handles are dropped.
    pub(super) fn new(probe: Option<Arc<Probe>>) -> Self {
        let shared = Arc::new(Shared {
            handles: Mutex::default(),
            held: Mutex::default(),
            probe,
        });
        let (send, jobs) = mpsc::channel::<Job>();
        let reader = std::thread::Builder::new()
            .name("dashboard-store".to_owned())
            .spawn(move || {
                for job in jobs {
                    job();
                }
            });
        let reader = match reader {
            Ok(_) => Some(Mutex::new(send)),
            Err(error) => {
                eprintln!("conductor dashboard: start the store reader: {error}");
                None
            }
        };
        Self { shared, reader }
    }

    /// What the handles hold in memory, as the last computation left them.
    pub(super) fn held(&self) -> StoresHeld {
        *lock(&self.shared.held)
    }

    /// `compute`, the computation of `key`, with the store under `state` as
    /// [`Shared::with_held`] gives it, run on the store reader while this thread waits for it: a
    /// store handle reads on one thread, always the same. A test's probe delays it first, on
    /// this thread. A computation that panics panics here, and the store reader goes on.
    pub(super) fn compute<R: Send + 'static>(
        &self,
        key: &str,
        state: Option<PathBuf>,
        compute: impl FnOnce(Held<'_>) -> R + Send + 'static,
    ) -> R {
        let probe = self.shared.probe.as_ref();
        let delay = probe.map(|probe| probe.start(key));
        if let Some(delay) = delay.filter(|delay| !delay.is_zero()) {
            std::thread::sleep(delay);
        }
        let shared = Arc::clone(&self.shared);
        let (send, answer) = mpsc::channel();
        let job: Job = Box::new(move || {
            let computed = catch_unwind(AssertUnwindSafe(|| shared.compute(state, compute)));
            let _ = send.send(computed);
        });
        let unsent = match &self.reader {
            Some(reader) => lock(reader).send(job).err().map(|unsent| unsent.0),
            None => Some(job),
        };
        if let Some(job) = unsent {
            job();
        }
        let computed = answer.recv();
        if let Some(probe) = probe {
            probe.end(key);
        }
        match computed {
            Ok(Ok(answer)) => answer,
            Ok(Err(panic)) => resume_unwind(panic),
            Err(_) => panic!("the store reader stopped before it answered {key}"),
        }
    }
}

impl Shared {
    /// `compute` with the store under `state`, under the handles' lock, on the thread it is
    /// called on; then takes down what the handles hold.
    fn compute<R>(&self, state: Option<PathBuf>, compute: impl FnOnce(Held<'_>) -> R) -> R {
        let mut stores = lock(&self.handles);
        if let Some(probe) = &self.probe {
            probe.read();
        }
        let answer = Self::with_held(&mut stores, state.as_deref(), compute);
        self.note(&stores);
        answer
    }

    /// Takes down what `stores` hold in memory now.
    fn note(&self, stores: &HashMap<PathBuf, Store>) {
        let mut held = lock(&self.held);
        let each: Vec<store::Replays> = stores.values().map(Store::held).collect();
        *held = StoresHeld {
            handles: stores.len(),
            replays: each.iter().map(|replays| replays.live).sum(),
            peak: each
                .iter()
                .map(|replays| replays.peak)
                .fold(held.peak, usize::max),
            bytes: each.iter().map(|replays| replays.bytes).sum(),
        };
    }

    /// Runs `compute` with the store under `state`, opened the first time as a reader
    /// ([`store::open_reader`], which does not wait on the store's writer lock) and kept; none
    /// without a `state`; `Err` with why when it does not open, and nothing is kept then. A
    /// handle on which a read failed is dropped after `compute`, so the next computation opens
    /// the store afresh.
    fn with_held<R>(
        stores: &mut HashMap<PathBuf, Store>,
        state: Option<&Path>,
        compute: impl FnOnce(Held<'_>) -> R,
    ) -> R {
        let Some(state) = state else {
            return compute(Ok(None));
        };
        if !stores.contains_key(state) {
            match store::open_reader(state) {
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
}

// ---------------------------------------------------------------------------------------------
// The answers kept
// ---------------------------------------------------------------------------------------------

/// How many answers one [`Memo`] keeps; past it, the one asked longest ago is dropped.
const KEPT: usize = 256;

/// How many answers one [`Memo`] keeps for one endpoint, each query string its own answer; past
/// it, the one of that endpoint asked longest ago is dropped (`story:dashboard-memory-bound`).
const PER_ENDPOINT: usize = 16;

/// How long after it was last asked an answer is still computed again by the warmer.
const WARM_FOR: Duration = Duration::from_secs(600);

/// One kept answer.
struct Kept<T> {
    fingerprint: u64,
    computed: Instant,
    asked: Instant,
    value: T,
    /// Its [`Weigh::weight`], taken when it was kept.
    weight: usize,
}

/// About how many bytes a kept answer holds.
pub(super) trait Weigh {
    /// The bytes, about.
    fn weight(&self) -> usize;
}

impl Weigh for Body {
    fn weight(&self) -> usize {
        self.bytes.len() + self.etag.len()
    }
}

/// What the answers of one [`Memo`] hold in memory (`story:dashboard-memory-bound`).
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct MemoHeld {
    /// The answers kept.
    pub(super) entries: usize,
    /// The most kept for any one endpoint: a key's path, without its query string.
    pub(super) peak_endpoint: usize,
    /// Their [`Weigh::weight`], together.
    pub(super) bytes: usize,
}

/// The endpoint of `key`: its path, without the query string.
fn endpoint(key: &str) -> &str {
    key.split_once('?').map_or(key, |(path, _)| path)
}

/// The kept answers, and the computations of them.
struct Entries<T> {
    kept: HashMap<String, Kept<T>>,
    /// The keys a computation runs for: at most one per key.
    running: HashSet<String>,
    /// When the last computation of each key started, which the debounce counts from.
    started: HashMap<String, Instant>,
    /// Moved by [`Memo::clear`]: a computation started before it keeps nothing.
    generation: u64,
}

/// An answer as a request finds it.
pub(super) enum Served<T> {
    /// Computed at the store's fingerprint now, at the instant given.
    Fresh(T, Instant),
    /// Computed at the instant given, at a fingerprint the store has moved from since: a
    /// computation of it runs, or the debounce holds one back.
    Stale(T, Instant),
    /// Never computed yet.
    Cold,
}

/// Answers by key, each with the fingerprint it was computed at, when, and when it was last
/// asked; at most one computation per key at a time, and for a key with an answer kept at most
/// one started per debounce.
pub(super) struct Memo<T>(Mutex<Entries<T>>, Condvar);

impl<T> Default for Memo<T> {
    fn default() -> Self {
        Self(
            Mutex::new(Entries {
                kept: HashMap::new(),
                running: HashSet::new(),
                started: HashMap::new(),
                generation: 0,
            }),
            Condvar::new(),
        )
    }
}

/// A computation of one key [`Memo::claim`] started: the generation its answer is kept under.
#[derive(Debug, Clone, Copy)]
pub(super) struct Claim(u64);

impl<T: Clone + Weigh> Memo<T> {
    /// What the kept answers hold in memory now.
    pub(super) fn held(&self) -> MemoHeld {
        let entries = lock(&self.0);
        let mut per: HashMap<&str, usize> = HashMap::new();
        for key in entries.kept.keys() {
            *per.entry(endpoint(key)).or_default() += 1;
        }
        MemoHeld {
            entries: entries.kept.len(),
            peak_endpoint: per.values().copied().max().unwrap_or(0),
            bytes: entries.kept.values().map(|kept| kept.weight).sum(),
        }
    }

    /// The answer kept for `key`: fresh when it was computed at `fingerprint`, else stale; cold
    /// when none is kept. `asked` says a request asks, which keeps the answer warm.
    pub(super) fn look(&self, key: &str, fingerprint: u64, asked: bool) -> Served<T> {
        let mut entries = lock(&self.0);
        let Some(entry) = entries.kept.get_mut(key) else {
            return Served::Cold;
        };
        if asked {
            entry.asked = Instant::now();
        }
        let value = entry.value.clone();
        if entry.fingerprint == fingerprint {
            Served::Fresh(value, entry.computed)
        } else {
            Served::Stale(value, entry.computed)
        }
    }

    /// Starts a computation of `key` when none runs and, for a key with an answer kept, the last
    /// one started `debounce` ago or longer; none otherwise. [`Memo::finish`] ends it.
    pub(super) fn claim(&self, key: &str, debounce: Duration) -> Option<Claim> {
        let mut entries = lock(&self.0);
        if entries.running.contains(key) {
            return None;
        }
        let held_back = entries.kept.contains_key(key)
            && entries
                .started
                .get(key)
                .is_some_and(|started| started.elapsed() < debounce);
        if held_back {
            return None;
        }
        entries.running.insert(key.to_owned());
        entries.started.insert(key.to_owned(), Instant::now());
        Some(Claim(entries.generation))
    }

    /// Ends the computation `claim` of `key`, keeping its answer computed at a fingerprint, when
    /// it has one, unless the answers were cleared since it started; wakes every request waiting
    /// for it.
    pub(super) fn finish(&self, key: &str, claim: Claim, computed: Option<(u64, T)>) {
        let mut entries = lock(&self.0);
        entries.running.remove(key);
        if let Some((fingerprint, value)) = computed
            && claim.0 == entries.generation
        {
            Self::put(&mut entries, key, fingerprint, value);
        }
        // The debounce counts only for a key with an answer kept: one without keeps none.
        if !entries.kept.contains_key(key) {
            entries.started.remove(key);
        }
        drop(entries);
        self.1.notify_all();
    }

    /// Waits until an answer for `key` is kept or no computation of it runs, up to `deadline`.
    pub(super) fn wait(&self, key: &str, deadline: Instant) {
        let entries = lock(&self.0);
        let left = deadline.saturating_duration_since(Instant::now());
        drop(
            self.1
                .wait_timeout_while(entries, left, |entries| {
                    !entries.kept.contains_key(key) && entries.running.contains(key)
                })
                .unwrap_or_else(PoisonError::into_inner),
        );
    }

    /// Keeps `value` for `key`. A new key past [`PER_ENDPOINT`] answers of its endpoint drops the
    /// one of them asked longest ago, and past [`KEPT`] answers in all the one asked longest ago
    /// of any; a dropped key's debounce goes with it.
    fn put(entries: &mut Entries<T>, key: &str, fingerprint: u64, value: T) {
        let now = Instant::now();
        let kept = &mut entries.kept;
        let last_asked = kept.get(key).map_or(now, |entry| entry.asked);
        if !kept.contains_key(key) {
            let path = endpoint(key);
            let oldest_of = |kept: &HashMap<String, Kept<T>>, same: bool| {
                kept.iter()
                    .filter(|(other, _)| !same || endpoint(other) == path)
                    .min_by_key(|(_, entry)| entry.asked)
                    .map(|(other, _)| other.clone())
            };
            let of_endpoint = kept.keys().filter(|other| endpoint(other) == path).count();
            let dropped = if of_endpoint >= PER_ENDPOINT {
                oldest_of(kept, true)
            } else if kept.len() >= KEPT {
                oldest_of(kept, false)
            } else {
                None
            };
            if let Some(dropped) = dropped {
                kept.remove(&dropped);
                if !entries.running.contains(&dropped) {
                    entries.started.remove(&dropped);
                }
            }
        }
        let kept = &mut entries.kept;
        kept.insert(
            key.to_owned(),
            Kept {
                fingerprint,
                computed: now,
                asked: last_asked,
                weight: value.weight(),
                value,
            },
        );
    }

    /// Forgets every kept answer and every debounce, so the next request computes it afresh: the
    /// server's own write to the store (`story:decisions-batch-answer`) does not wait for the
    /// fingerprint to move. A computation started before keeps nothing.
    pub(super) fn clear(&self) {
        let mut entries = lock(&self.0);
        entries.kept.clear();
        entries.started.clear();
        entries.generation += 1;
    }

    /// The keys asked within [`WARM_FOR`], which the warmer computes again when their store
    /// moved.
    pub(super) fn keys(&self) -> Vec<String> {
        lock(&self.0)
            .kept
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
/// since `computed`, its `Age` in seconds; a `stale` one, whose store moved since, its age in
/// `X-Conductor-Age` as well.
pub(super) fn send(
    body: &Body,
    condition: Option<&str>,
    computed: Option<Instant>,
    stale: bool,
) -> Vec<u8> {
    let mut extra = format!("ETag: {}\r\n", body.etag);
    if let Some(computed) = computed {
        let age = computed.elapsed().as_secs();
        extra.push_str(&format!("Age: {age}\r\n"));
        if stale {
            extra.push_str(&format!("X-Conductor-Age: {age}\r\n"));
        }
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
