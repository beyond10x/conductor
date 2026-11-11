//! The instance API (`story:instance-api`, design architecture-next § 5.1): what was the
//! per-instance dashboard server, its routes under `/v1/` on the instance runtime's Unix socket
//! (`<state>/api.sock`, [`crate::instance`]), which conductord proxies at
//! `/v1/instances/{name}/<route>` and whose app it serves ([`crate::daemon::api`]). The routes are
//! read-only but for the operator's answers on the Decisions page (`story:decisions-batch-answer`),
//! the settings and the Plan page's moves. What it shows is presentation, not a record, and nothing
//! it reads enters the store.
//!
//! It serves no app and checks no `Host` or `Origin`: the socket is the runtime's own, mode 0600,
//! and the daemon admits the browser (its host, its origin, its token or an Identity bearer) before
//! it forwards. `/v1/data.json` is the data the app reads every 20 s. Any other path is 404 and any
//! other method on a route 405.
//!
//! The one write ([`answers`], `story:decisions-batch-answer`): `POST /v1/answers` records a batch
//! of the operator's answers to open escalated requests, each as his decision recorded by
//! `dashboard`, and answers it row by row; any other method on it is 405. A batch is taken only
//! with the token the daemon minted for the instance at its start
//! ([`crate::instance::TOKEN_VARIABLE`]), in `X-Conductor-Token`, as JSON of at most 64 KiB. The
//! daemon names the operator it admitted in `X-Conductor-Operator`, and the answer is recorded as
//! that operator's (`story:instance-api`); without the header, as the Unix user's.
//!
//! The pages' writes take the same terms through one path (`post_write`), each recording one
//! decision by the operator: `PUT /v1/settings` writes the Configuration page's staged changes
//! (`story:dashboard-config-page`, [`configure`]), and `POST /v1/plan` the Plan page's move,
//! start, priority or comment of a story, through the `aep` CLI of the checkout's planning store
//! (`story:dashboard-plan-board`, [`plan`]). `/v1/plan` and `/v1/plan/story` answer the board
//! and one story.
//!
//! The overview's endpoints (`story:dashboard-overview`, [`overview`]): `/v1/board`,
//! `/v1/repositories`, `/v1/pull-requests`, `/v1/workflow-runs`, `/v1/releases` and
//! `/v1/reviews` (what waits for review, `story:review-limit`) answer
//! the newest complete snapshot of the store under the instance's state directory, opened only
//! when it is there ([`serve_unix`] names another); `/v1/events?since=<rfc3339>` the events since
//! then, 00:00 UTC of the day without `since`; `/v1/unanswered` the count of unanswered class-O
//! and class-H items and the oldest one's age (`story:decisions-notify`).
//!
//! The Decisions page's data ([`decisions`], `story:decisions-page`): `/v1/decisions` answers the
//! open class-O and class-H items with what the operator chooses from, oldest first, and the
//! records' decision log, newest first, computed once per write to the store or the log.
//!
//! The charts' series and the KPIs ([`series`]): `/v1/series` lists the series and, with
//! `?series=<name>&by=<dimension>&days=<n>`, answers one by day and group; `/v1/kpis` answers
//! each KPI against its target. Both read the daily rollups of the same store
//! (`story:dashboard-history`), the day ending now in UTC.
//!
//! Every endpoint answers warm in under 200 ms (`story:dashboard-fast`, [`cache`]): an answer that
//! reads the store is computed once per write to the store, through one store handle the server
//! keeps, and served from memory after that; every JSON answer carries an `ETag`, and a request
//! whose `If-None-Match` names it is answered `304 Not Modified` without a body. After a write the
//! kept answer is served at once, with its age in `X-Conductor-Age`, while it is computed again in
//! the background, at most once per endpoint per [`cache::DEBOUNCE`]; a request before an
//! endpoint's first answer is answered `503` with `Retry-After` after [`cache::COLD_WAIT`].
//!
//! Each request reads its [`Sources`] afresh, but the session list; none but `POST /v1/answers`
//! writes:
//! - the session list a command prints (`claude agents --json`), read on a thread of its own every
//!   [`cache::SESSIONS_EVERY`], each request taking the last one with when it was read
//!   (`reading.sessions_read_at`), joined with the instance's sessions from its records
//!   ([`sessions`], `story:sessions-page-coherent`): its conductor, its maintainer, and each
//!   controller the store runs, with its dispatch; a session without a `pid` has exited and reads
//!   `exited`, a record no session holds `not running`; the controllers and the open operator
//!   items are read from the store once per write to it;
//! - `dispatches/*.jsonl` and `decisions/*.jsonl` under the records root;
//! - the watch's usage-limit and context files (`limit.new`, `limit.seen`, `limit.reported`,
//!   `context-reported`);
//! - free space on `/`, through `statvfs` ([`crate::disk::space`]).
//!
//! When a config file names the instance this process runs ([`crate::config::active`]), the
//! records root is its `records` unless `--root` names another, and the watch's directory is the
//! `watch/` of its `state`, where `conductor watch run` keeps it. Without a file every source is
//! today's ([`Sources::new`]).
//!
//! The exceptions are the disk waste across the tracked repositories ([`waste`]) and the token
//! spend per repository since 00:00Z ([`usage`], under `usage`): each is measured on a thread of
//! its own, at most every [`Sources::measure_every`], and each request shows the last complete
//! measurement with its `measured_at`.
//!
//! A source that cannot be read is named in `errors`; the others still show. No path under the
//! home directory is printed in full: every string `/v1/data.json` carries writes it as `~`.

mod agents;
mod answers;
mod cache;
pub mod configure;
pub mod decisions;
mod health;
pub mod overview;
pub mod plan;
pub mod repo;
pub mod series;
mod sessions;
mod usage;
pub(crate) mod waste;

pub use answers::{
    ANSWERS_PATH, MAX_BODY, OPERATOR_HEADER, TOKEN_HEADER, mint, page_with, same as same_token,
    same_origin,
};
pub use cache::{Probe, Tuning};
pub use health::{HealthSources, failure_class};

use std::ffi::OsString;
use std::fs;
use std::hash::{Hash as _, Hasher as _};
use std::io::{self, ErrorKind, Read, Write as _};
use std::net::Shutdown;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use conductor_model::config::Instance;
use conductor_model::observation::{DashboardMemory, FailureClass};
use include_dir::{Dir, File, include_dir};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::config;

/// The dashboard app as `task ui` builds it: `index.html` and its assets, embedded at build time;
/// conductord serves it ([`crate::daemon::api`]).
static APP: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/dashboard/dist");

/// The policy every answer carries: the app's script, style and data come from its server only.
pub const CONTENT_SECURITY_POLICY: &str =
    "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'";

/// Characters of a brief, a reason or a question `/v1/data.json` carries.
const EXCERPT: usize = 120;

/// The longest request head read; a longer one is answered 400.
const MAX_HEAD: usize = 16 * 1024;

/// How long one connection may take to send its request or to take the answer.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

/// What to do about a member a collector could not read, by the class of its error's first line
/// ([`failure_class`]): the one table the health view's fixes come from
/// (`story:dashboard-health`), each class with the name `/v1/health` gives it.
pub const FAILURE_FIXES: [(FailureClass, &str, &str); 5] = [
    (
        FailureClass::NeedsLogin,
        "needs login",
        "The member's remote asked for a login or refused the credential: load the SSH key \
         (`ssh-add`) or cache HTTPS credentials for its host, check with `git fetch` in its \
         checkout, then take the snapshot again.",
    ),
    (
        FailureClass::UnreachableHost,
        "unreachable host",
        "The member's host did not resolve or answer: check its remote's host name \
         (`git remote -v` in its checkout), the network and any VPN the host needs, then take the \
         snapshot again.",
    ),
    (
        FailureClass::TimedOut,
        "timed out",
        "A command passed its bound and was stopped: run it by hand in the member's checkout to \
         see where it waits (a prompt, a slow remote), fix that, then take the snapshot again.",
    ),
    (
        FailureClass::OldStore,
        "old store",
        "The member's planning store is `aep.project/1`, which this `aep` refuses: in its \
         checkout, review `aep plan store migrate git --dry-run`, then run it without \
         `--dry-run`.",
    ),
    (
        FailureClass::Other,
        "other",
        "Read the error line, then run the command it names by hand in the member's checkout; \
         `conductor doctor` checks the programs the collectors start.",
    ),
];

/// What the page reads, the one seam a test points at fixtures. [`Sources::new`] gives the real
/// ones.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The program and arguments that print the session list as one JSON array.
    pub agents: Vec<OsString>,
    /// The records root, holding `dispatches/` and `decisions/`.
    pub root: PathBuf,
    /// The CI watch's state file.
    pub ci_state: PathBuf,
    /// The home directory, which the page writes `~`.
    pub home: PathBuf,
    /// The checkouts root: the page shows the sessions under it, and counts as a controller each
    /// one in a checkout directly under it.
    pub checkouts: PathBuf,
    /// The managed trees, `<trees>/<repo>/<tree>`: a transcript of a session there counts for
    /// `<repo>` in the token-spend table.
    pub trees: PathBuf,
    /// The directory whose entries the disk measurement lists as shared build targets.
    pub targets: PathBuf,
    /// A path on the file system whose free space the page shows.
    pub disk: PathBuf,
    /// The watch's directory, holding `limit.new`, `limit.seen` and `context-reported`.
    pub watch: PathBuf,
    /// The program and arguments that print `worktree sweep --dry-run --json`.
    pub sweep: Vec<OsString>,
    /// The program and arguments that print `worktree gc --dry-run --json`.
    pub gc: Vec<OsString>,
    /// The program and arguments that print `worktree inspect --json --workspace`: the registered
    /// managed trees, each with its live leases (`story:gc-finished-trees`).
    pub inspect: Vec<OsString>,
    /// The directory whose entries the disk measurement lists as `/var/tmp`.
    pub var_tmp: PathBuf,
    /// The instance's own scratch, `<cache>/scratch`: a controller's scratch is
    /// `<cache>/scratch/<repo>`, and reclaim deletes stale entries here and in no other
    /// instance's.
    pub scratch: PathBuf,
    /// How long one disk measurement is shown before a request starts the next.
    pub measure_every: Duration,
    /// How long one command of the disk measurement may run before it is stopped.
    pub measure_timeout: Duration,
    /// Tokens of a session's context above which the watch reports it, which the context line
    /// names: the instance's `thresholds.context_handover`.
    pub context_handover: u64,
    /// What the health view reads besides the disk-waste measurement (`story:dashboard-health`).
    pub health: HealthSources,
    /// The config file the Configuration page shows and writes (`story:dashboard-config-page`);
    /// none when the instance runs on the built-in defaults.
    pub config_file: Option<PathBuf>,
    /// The instance of that file the page shows; its first when none.
    pub instance_name: Option<String>,
}

impl Sources {
    /// The sources without a config file: `claude agents --json`, the repository `root`, the
    /// watch's directory ([`watch_dir`]), the built-in instance's checkouts root and context
    /// hand-over threshold ([`config::built_in`]) and `/`; the `worktree` dry-runs, with `--repo`
    /// the first git checkout under the checkouts root, else `root`, and `/var/tmp`, measured
    /// every 10 minutes.
    #[must_use]
    pub fn new(home: PathBuf, root: PathBuf) -> Self {
        let watch = watch_dir(&home, &root);
        let built_in = config::built_in(&home, &root);
        let checkouts = [
            PathBuf::from(built_in.checkouts.root),
            PathBuf::from(built_in.checkouts.trees),
        ];
        let targets = waste::built_in_targets(&home);
        let context_handover = tokens(built_in.thresholds.context_handover.0);
        let mut sources = Self::with(home, root, watch, checkouts, targets, context_handover);
        sources.scratch = Path::new(&built_in.cache).join(SCRATCH);
        sources
    }

    /// The sources of `instance`, which a config file names: `root` when `--root` names one, else
    /// the instance's `records`; the `watch/` of its `state`; its checkouts root; its
    /// `thresholds.context_handover`. The rest is as [`Sources::new`] gives it.
    #[must_use]
    pub fn of(home: PathBuf, root: Option<PathBuf>, instance: &Instance) -> Self {
        let root = root.unwrap_or_else(|| PathBuf::from(&instance.records));
        let watch = Path::new(&instance.state).join(WATCH);
        let checkouts = [
            PathBuf::from(&instance.checkouts.root),
            PathBuf::from(&instance.checkouts.trees),
        ];
        let targets = Path::new(&instance.cache).join("target");
        let context_handover = tokens(instance.thresholds.context_handover.0);
        let mut sources = Self::with(home, root, watch, checkouts, targets, context_handover);
        sources.scratch = Path::new(&instance.cache).join(SCRATCH);
        sources.health = HealthSources::of(sources.home.clone(), instance);
        sources.instance_name = Some(instance.name.0.clone());
        sources
    }

    /// The sources of the instance this process runs ([`config::active`]) under the home
    /// directory `HOME` names: [`Sources::of`] when a config file names it, else [`Sources::new`]
    /// over `root`, or the nearest directory from the working directory up that holds both
    /// `dispatches/` and `decisions/`. `dashboard serve` and `conductor gc` read these.
    ///
    /// # Errors
    ///
    /// When `HOME` is not an absolute path, or the working directory cannot be read.
    pub fn active(root: Option<PathBuf>) -> Result<Self> {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .filter(|home| home.is_absolute())
            .context("HOME is not an absolute path; the page writes it as ~")?;
        let active = config::active();
        if active.from_file {
            let mut sources = Self::of(home, root, &active.instance);
            sources.health.config = config::explicit().map(Path::to_path_buf);
            sources.health.host = config::host_dir().ok();
            sources.config_file = config::config_file().map(Path::to_path_buf);
            return Ok(sources);
        }
        let root = match root {
            Some(root) => root,
            None => find_root(&std::env::current_dir().context("read the working directory")?),
        };
        Ok(Self::new(home, root))
    }

    fn with(
        home: PathBuf,
        root: PathBuf,
        watch: PathBuf,
        checkouts: [PathBuf; 2],
        targets: PathBuf,
        context_handover: u64,
    ) -> Self {
        let [checkouts, trees] = checkouts;
        // `worktree` resolves its profile from `--repo` and refuses a directory no profile
        // contains, as the records directory is: the dry-runs name the checkout
        // `prune-archives` is given (`story:dashboard-worktree-repo`), or `root` when the
        // checkouts root holds no git checkout.
        let repo = waste::profile_checkout(&checkouts).unwrap_or_else(|| root.clone());
        let worktree = |verb: &str| {
            let mut command: Vec<OsString> = ["worktree", verb, "--dry-run", "--json", "--repo"]
                .map(OsString::from)
                .to_vec();
            command.push(repo.clone().into_os_string());
            command
        };
        Self {
            agents: ["claude", "agents", "--json"].map(OsString::from).to_vec(),
            ci_state: watch.join("ci.state"),
            watch,
            disk: PathBuf::from("/"),
            sweep: worktree("sweep"),
            gc: worktree("gc"),
            inspect: {
                let mut command: Vec<OsString> = [
                    "worktree",
                    "inspect",
                    "--json",
                    "--workspace",
                    "--max-entries",
                    "1",
                    "--repo",
                ]
                .map(OsString::from)
                .to_vec();
                command.push(repo.clone().into_os_string());
                command
            },
            var_tmp: PathBuf::from("/var/tmp"),
            scratch: PathBuf::new(),
            measure_every: Duration::from_secs(600),
            measure_timeout: Duration::from_secs(180),
            // The built-in instance's; [`Sources::of`] gives an instance's own.
            health: HealthSources::new(home.clone(), &root),
            config_file: None,
            instance_name: None,
            root,
            home,
            checkouts,
            trees,
            targets,
            context_handover,
        }
    }
}

/// A configured count of tokens as the page carries it; a negative one, which the config refuses,
/// as 0.
fn tokens(count: i64) -> u64 {
    u64::try_from(count).unwrap_or(0)
}

/// The directory under an instance's cache that holds its scratch.
pub const SCRATCH: &str = "scratch";

/// The watch's directory under a state directory, as `conductor watch run` keeps it.
const WATCH: &str = "watch";

/// The watch's directory the page reads without a config file: `<root>/state/watch/` when it
/// holds `ci.state`, which `conductor watch run` started in `root` writes, else the stopgap's
/// `~/.cache/conductor-watch/`.
fn watch_dir(home: &Path, root: &Path) -> PathBuf {
    let watch = root.join("state").join(WATCH);
    if watch.join("ci.state").is_file() {
        watch
    } else {
        home.join(".cache/conductor-watch")
    }
}

/// The nearest of `start` and its ancestors that holds both `dispatches/` and `decisions/`, or
/// `start` when none does.
fn find_root(start: &Path) -> PathBuf {
    start
        .ancestors()
        .find(|dir| dir.join("dispatches").is_dir() && dir.join("decisions").is_dir())
        .unwrap_or(start)
        .to_path_buf()
}

/// What every connection shares: the sources, the state directory whose store the snapshot
/// endpoints read, the last disk and token measurements, and what [`cache`] keeps.
struct Server {
    sources: Sources,
    state: Option<PathBuf>,
    waste: waste::Cache,
    usage: usage::Cache,
    /// Doctor's last rows, which the health view serves (`story:dashboard-health`).
    doctor: health::Doctor,
    /// The store handles every computation over a store takes.
    stores: cache::Stores,
    /// The answers of the endpoints that read the store ([`STORED_PATHS`]), by request target.
    answers: cache::Memo<cache::Body>,
    /// What `/v1/waiting-on` reads from the store.
    facts: cache::Memo<Result<repo::Facts, String>>,
    /// The unanswered operator items `/v1/unanswered` reads from the store
    /// (`story:decisions-notify`); their age is taken at each request.
    unanswered: cache::Memo<Result<Vec<crate::status::OperatorItem>, String>>,
    /// The health view's newest snapshot with its rows.
    snapshot_health: cache::Memo<health::Newest>,
    /// What `/v1/data.json` reads from the store: the controllers and the open operator items
    /// (`story:sessions-page-coherent`).
    roster: cache::Memo<Roster>,
    /// The last session list.
    sessions: cache::SessionList,
    /// The token the page is served with and a batch of answers is taken with
    /// (`story:decisions-batch-answer`).
    token: answers::Token,
    /// Held while a batch of answers is recorded: one batch at a time.
    writes: Mutex<()>,
    /// The debounce and the cold wait of the kept answers.
    tuning: cache::Tuning,
    /// The agents behind `agents/{name}/*` (`story:agent-kinds`).
    agents: Mutex<crate::agent::Runtime>,
    /// The providers an agent's kind starts and stops it through.
    providers: crate::confine::Host,
}

/// A computation of a kept answer: of the key given, over the store given.
type Compute<T> = fn(&Server, &str, cache::Held<'_>) -> T;

/// One kind of kept answer: where it is kept, the state directory whose store it reads, and how it
/// is computed.
struct Kind<T> {
    memo: fn(&Server) -> &cache::Memo<T>,
    state: fn(&Server) -> Option<PathBuf>,
    compute: Compute<T>,
}

impl<T> Clone for Kind<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Kind<T> {}

/// The answers of [`STORED_PATHS`], by request target.
const ANSWERS: Kind<cache::Body> = Kind {
    memo: |server| &server.answers,
    state: Server::store_path,
    compute: |server, target, held| match held {
        Ok(store) => {
            let path = target.split_once('?').map_or(target, |(path, _)| path);
            let (status, value) = server.compute(path, target, store);
            cache::Body::json(status, &value)
        }
        Err(error) => store_failure(error),
    },
};

/// What `/v1/waiting-on` reads from the store, without the session list and the watch's files,
/// which each request reads; why when the store does not open.
const FACTS: Kind<Result<repo::Facts, String>> = Kind {
    memo: |server| &server.facts,
    state: Server::store_path,
    compute: |server, _, held| match held {
        Ok(store) => Ok(repo::facts(store, &server.sources, &[])),
        Err(error) => Err(format!("the store does not open: {error:#}\n")),
    },
};

/// The unanswered operator items of a store, or why it does not open.
type Unanswered = Result<Vec<crate::status::OperatorItem>, String>;

/// The unanswered operator items of the store, which `/v1/unanswered` answers with the oldest
/// one's age now (`story:decisions-notify`); why when the store does not open.
const UNANSWERED: Kind<Unanswered> = Kind {
    memo: |server| &server.unanswered,
    state: Server::store_path,
    compute: |_, _, held| match held {
        Ok(store) => Ok(store.map(crate::status::unanswered).unwrap_or_default()),
        Err(error) => Err(format!("the store does not open: {error:#}\n")),
    },
};

/// What `/v1/data.json` reads from a store, or why it does not open.
type Roster = Result<sessions::Roster, String>;

/// The controllers and the open operator items of the store, which `/v1/data.json` joins with the
/// session list and the records at each request (`story:sessions-page-coherent`); why when the
/// store does not open.
const ROSTER: Kind<Roster> = Kind {
    memo: |server| &server.roster,
    state: Server::store_path,
    compute: |_, _, held| match held {
        Ok(store) => Ok(store.map(sessions::roster).unwrap_or_default()),
        Err(error) => Err(format!("the store does not open: {error:#}")),
    },
};

/// The health view's newest snapshot with its rows, of the store under its own state directory.
const HEALTH: Kind<health::Newest> = Kind {
    memo: |server| &server.snapshot_health,
    state: |server| Some(server.sources.health.state.clone()),
    compute: |_, _, held| health::newest(held),
};

/// The size of a kept answer that is not sent as it is kept: that of its debug text, about what
/// its strings and rows hold.
macro_rules! weighed_by_debug {
    ($($kept:ty),*) => {
        $(impl cache::Weigh for $kept {
            fn weight(&self) -> usize {
                format!("{self:?}").len()
            }
        })*
    };
}

weighed_by_debug!(
    Result<repo::Facts, String>,
    Unanswered,
    health::Newest,
    Roster
);

/// Ends a background computation that did not finish (it panicked): its key computes again.
struct Unfinished<'a, T: Clone + cache::Weigh> {
    memo: &'a cache::Memo<T>,
    key: &'a str,
    claim: Option<cache::Claim>,
}

impl<T: Clone + cache::Weigh> Drop for Unfinished<'_, T> {
    fn drop(&mut self) {
        if let Some(claim) = self.claim.take() {
            self.memo.finish(self.key, claim, None);
        }
    }
}

/// The endpoints whose answer is the store's alone, `/v1/events` and `/v1/decisions` the
/// records' too: each is computed once per write and served from memory.
const STORED_PATHS: [&str; 12] = [
    "/v1/board",
    "/v1/repositories",
    "/v1/pull-requests",
    "/v1/workflow-runs",
    "/v1/releases",
    "/v1/reviews",
    overview::EVENTS_PATH,
    decisions::DECISIONS_PATH,
    series::SERIES_PATH,
    series::KPIS_PATH,
    BLOCKERS_PATH,
    SPECIFICATIONS_PATH,
];

/// Two of [`repo::PATHS`] that read the store alone.
const BLOCKERS_PATH: &str = "/v1/blockers";
const SPECIFICATIONS_PATH: &str = "/v1/specifications";

/// The fingerprints of what the kept answers read, taken once per request or warming.
struct Fingerprints {
    /// The store's and the day's (UTC).
    store: u64,
    /// [`Fingerprints::store`] with the records' dispatch and decision files.
    events: u64,
    /// [`Fingerprints::store`] with the records' decision files (`story:decisions-page`).
    decisions: u64,
}

impl Server {
    fn new(sources: Sources, state: Option<PathBuf>, tuning: cache::Tuning, token: String) -> Self {
        Self {
            sources,
            state,
            waste: waste::Cache::default(),
            usage: usage::Cache::default(),
            doctor: health::Doctor::default(),
            stores: cache::Stores::new(tuning.probe.clone()),
            answers: cache::Memo::default(),
            facts: cache::Memo::default(),
            unanswered: cache::Memo::default(),
            snapshot_health: cache::Memo::default(),
            roster: cache::Memo::default(),
            sessions: cache::SessionList::default(),
            token: answers::Token::given(token),
            writes: Mutex::new(()),
            tuning,
            agents: Mutex::new(crate::agent::Runtime::new(Box::new(agents::print))),
            providers: crate::confine::Host,
        }
    }

    /// What this process holds in memory (`story:dashboard-memory-bound`): its resident set, its
    /// store handles and the answers it keeps.
    fn memory(&self) -> DashboardMemory {
        let stores = self.stores.held();
        let memos = [
            self.answers.held(),
            self.facts.held(),
            self.unanswered.held(),
            self.snapshot_health.held(),
            self.roster.held(),
        ];
        let int = |count: u64| i64::try_from(count).unwrap_or(i64::MAX);
        let size = |count: usize| int(count as u64);
        DashboardMemory {
            rss_bytes: health::rss_bytes().map(int),
            swap_bytes: health::swap_bytes().map(int),
            store_handles: size(stores.handles),
            store_replays: size(stores.replays),
            store_replays_peak: size(stores.peak),
            store_bytes: int(stores.bytes),
            memo_entries: size(memos.iter().map(|memo| memo.entries).sum()),
            memo_entries_peak_endpoint: size(
                memos
                    .iter()
                    .map(|memo| memo.peak_endpoint)
                    .max()
                    .unwrap_or(0),
            ),
            memo_bytes: size(memos.iter().map(|memo| memo.bytes).sum()),
        }
    }

    /// The fingerprints of the store and the records now.
    fn fingerprints(&self) -> Fingerprints {
        let mut hasher = cache::hasher();
        if let Some(state) = &self.state {
            cache::fingerprint(state, &mut hasher);
        }
        OffsetDateTime::now_utc().date().hash(&mut hasher);
        let store = hasher.finish();
        let log = self.sources.root.join(crate::records::DECISIONS);
        let mut decisions = hasher.clone();
        cache::files(&log, &mut decisions);
        cache::files(&self.sources.root.join("dispatches"), &mut hasher);
        cache::files(&log, &mut hasher);
        Fingerprints {
            store,
            events: hasher.finish(),
            decisions: decisions.finish(),
        }
    }

    /// The state directory whose store the snapshot endpoints read, when it holds one.
    fn store_path(&self) -> Option<PathBuf> {
        self.state
            .clone()
            .filter(|state| crate::store::present(state))
    }

    /// The answer of `kind` for `key` at `fingerprint`: kept and fresh; kept and stale, served at
    /// once while one computation of it runs in the background, which starts at most once per
    /// [`cache::Tuning::debounce`]; or never computed, when a computation of it starts and a
    /// request (`wait`) waits for it up to [`cache::Tuning::cold_wait`]. `asked` says a request
    /// asks, not the warmer.
    fn serve<T: Clone + cache::Weigh + Send + 'static>(
        self: &Arc<Self>,
        kind: Kind<T>,
        key: &str,
        fingerprint: u64,
        asked: bool,
        wait: bool,
    ) -> cache::Served<T> {
        let memo = (kind.memo)(self);
        let deadline = Instant::now() + self.tuning.cold_wait;
        loop {
            let served = memo.look(key, fingerprint, asked);
            if !matches!(served, cache::Served::Fresh(..))
                && let Some(claim) = memo.claim(key, self.tuning.debounce)
            {
                self.recompute(kind, key, fingerprint, claim);
            }
            if !matches!(served, cache::Served::Cold) || !wait || Instant::now() >= deadline {
                return served;
            }
            memo.wait(key, deadline);
        }
    }

    /// Computes the answer of `kind` for `key` on a thread of its own, keeping it at
    /// `fingerprint` under `claim`.
    fn recompute<T: Clone + cache::Weigh + Send + 'static>(
        self: &Arc<Self>,
        kind: Kind<T>,
        key: &str,
        fingerprint: u64,
        claim: cache::Claim,
    ) {
        let server = Arc::clone(self);
        let owned = key.to_owned();
        let spawned = std::thread::Builder::new()
            .name("dashboard-compute".to_owned())
            .spawn(move || {
                let key = owned.as_str();
                let memo = (kind.memo)(&server);
                let mut unfinished = Unfinished {
                    memo,
                    key,
                    claim: Some(claim),
                };
                let state = (kind.state)(&server);
                let (reader, target) = (Arc::clone(&server), key.to_owned());
                let value = server.stores.compute(key, state, move |held| {
                    (kind.compute)(&reader, &target, held)
                });
                unfinished.claim = None;
                memo.finish(key, claim, Some((fingerprint, value)));
            });
        if let Err(error) = spawned {
            eprintln!("conductor dashboard: start a computation of {key}: {error}");
            (kind.memo)(self).finish(key, claim, None);
        }
    }

    /// The answer to `target`, whose path is one of [`STORED_PATHS`], at `fingerprints`, as
    /// [`Server::serve`] finds it. A store that does not open is answered `500`.
    fn stored(
        self: &Arc<Self>,
        target: &str,
        fingerprints: &Fingerprints,
        asked: bool,
        wait: bool,
    ) -> cache::Served<cache::Body> {
        let path = target.split_once('?').map_or(target, |(path, _)| path);
        let fingerprint = match path {
            overview::EVENTS_PATH => fingerprints.events,
            decisions::DECISIONS_PATH => fingerprints.decisions,
            _ => fingerprints.store,
        };
        self.serve(ANSWERS, target, fingerprint, asked, wait)
    }

    /// The status and answer of the endpoint `path` at `target` over `store`.
    fn compute(
        &self,
        path: &str,
        target: &str,
        store: Option<&crate::store::Store>,
    ) -> (u16, Value) {
        let now = OffsetDateTime::now_utc();
        let home = self.sources.home.display().to_string();
        let home = home.trim_end_matches('/');
        match path {
            overview::EVENTS_PATH => {
                let since = overview::since(target)
                    .ok()
                    .flatten()
                    .unwrap_or_else(|| overview::start_of_day(now));
                let mut errors = Vec::new();
                let root = &self.sources.root;
                let dispatches = jsonl(&root.join("dispatches"), &mut errors);
                let decisions = jsonl(&root.join("decisions"), &mut errors);
                let events = overview::events(
                    store,
                    &dispatches,
                    &decisions,
                    &config::active().instance,
                    since,
                );
                // `&repository=<name>` keeps that repository's rows (`story:dashboard-repo-view`).
                let events = match repo::query(target, "repository") {
                    Some(name) => {
                        let rows = events["rows"].as_array().cloned().unwrap_or_default();
                        json!({"rows": repo::events_for_repository(rows, &name)})
                    }
                    None => events,
                };
                (200, tilde_all(events, home))
            }
            // The open class-O and class-H items and the decision log (`story:decisions-page`).
            decisions::DECISIONS_PATH => {
                let mut errors = Vec::new();
                let log = jsonl(
                    &self.sources.root.join(crate::records::DECISIONS),
                    &mut errors,
                );
                (200, tilde_all(decisions::answer(store, &log, errors), home))
            }
            series::SERIES_PATH => series::answer(target, store, now.date()),
            series::KPIS_PATH => (200, series::kpis(store, now.date())),
            BLOCKERS_PATH => (200, tilde_all(repo::blockers(store), home)),
            SPECIFICATIONS_PATH => (200, tilde_all(repo::specifications(store), home)),
            _ => (
                200,
                overview::snapshot_endpoint(path, store).unwrap_or(Value::Null),
            ),
        }
    }

    /// What `/v1/waiting-on` reads from the store, at `fingerprints`, without the session list
    /// and the watch's files, which each request reads; why when the store does not open; with
    /// when it was computed when it is stale; none before it was first computed.
    fn facts(
        self: &Arc<Self>,
        fingerprints: &Fingerprints,
        asked: bool,
        wait: bool,
    ) -> Option<(Result<repo::Facts, String>, Option<Instant>)> {
        served(self.serve(FACTS, "facts", fingerprints.store, asked, wait))
    }

    /// The unanswered operator items of the store, at `fingerprints`, which `/v1/unanswered`
    /// answers with the oldest one's age now (`story:decisions-notify`); why when the store does
    /// not open; with when they were computed when they are stale; none before they were first
    /// computed.
    fn unanswered(
        self: &Arc<Self>,
        fingerprints: &Fingerprints,
        asked: bool,
        wait: bool,
    ) -> Option<(Unanswered, Option<Instant>)> {
        served(self.serve(UNANSWERED, "unanswered", fingerprints.store, asked, wait))
    }

    /// The controllers and open operator items of the store, at `fingerprints`, which
    /// `/v1/data.json` reads (`story:sessions-page-coherent`); why when the store does not open;
    /// with when they were computed when they are stale; none before they were first computed.
    fn roster(
        self: &Arc<Self>,
        fingerprints: &Fingerprints,
        asked: bool,
        wait: bool,
    ) -> Option<(Roster, Option<Instant>)> {
        served(self.serve(ROSTER, "roster", fingerprints.store, asked, wait))
    }

    /// The health view's newest snapshot with its rows, computed once per write to the store
    /// under its state directory; with when it was computed when it is stale; none before it was
    /// first computed.
    fn snapshot_health(
        self: &Arc<Self>,
        asked: bool,
        wait: bool,
    ) -> Option<(health::Newest, Option<Instant>)> {
        let mut hasher = cache::hasher();
        cache::fingerprint(&self.sources.health.state, &mut hasher);
        served(self.serve(HEALTH, "health", hasher.finish(), asked, wait))
    }

    /// Starts a computation of every answer asked lately whose store moved since, as the debounce
    /// lets it, so the request after a write mostly finds it computed; at start, of the answers
    /// the overview reads first. Waits for none.
    fn warm(self: &Arc<Self>, start: bool) {
        let fingerprints = self.fingerprints();
        let mut targets = self.answers.keys();
        if start {
            targets.extend(
                STORED_PATHS
                    .iter()
                    .filter(|path| **path != overview::EVENTS_PATH)
                    .map(|path| (*path).to_owned()),
            );
        }
        for target in targets {
            self.stored(&target, &fingerprints, false, false);
        }
        if start || !self.facts.keys().is_empty() {
            self.facts(&fingerprints, false, false);
        }
        if start || !self.unanswered.keys().is_empty() {
            self.unanswered(&fingerprints, false, false);
        }
        if start || !self.roster.keys().is_empty() {
            self.roster(&fingerprints, false, false);
        }
        if start || !self.snapshot_health.keys().is_empty() {
            self.snapshot_health(false, false);
        }
    }
}

/// The value of a kept answer, with when it was computed when it is stale; none before it was
/// first computed.
fn served<T>(served: cache::Served<T>) -> Option<(T, Option<Instant>)> {
    match served {
        cache::Served::Fresh(value, _) => Some((value, None)),
        cache::Served::Stale(value, computed) => Some((value, Some(computed))),
        cache::Served::Cold => None,
    }
}

/// `503 Service Unavailable` with `Retry-After`: an answer asked before it was first computed.
fn unavailable() -> Vec<u8> {
    response(
        503,
        "Service Unavailable",
        "text/plain; charset=utf-8",
        b"this answer is being computed for the first time; ask again\n",
        &format!("Retry-After: {}\r\n", cache::RETRY_AFTER),
    )
}

/// A kept answer as an HTTP answer to a request whose `If-None-Match` is `condition`: a stale one
/// with its age in `X-Conductor-Age`, `503` before it was first computed ([`cache::send`]).
fn send_served(served: cache::Served<cache::Body>, condition: Option<&str>) -> Vec<u8> {
    match served {
        cache::Served::Fresh(body, computed) => {
            cache::send(&body, condition, Some(computed), false)
        }
        cache::Served::Stale(body, computed) => cache::send(&body, condition, Some(computed), true),
        cache::Served::Cold => unavailable(),
    }
}

/// Answers every connection `listener`, the instance runtime's socket, accepts, each on a thread
/// of its own, from `sources`; the snapshot endpoints read the store under `state`, or none when
/// it is `None` (the seam a test points at a fixture store, `story:dashboard-overview`); a write
/// is taken with `token`. The first disk and token measurements start at once.
///
/// It does not return: a failed connection is reported on standard error and the next one is
/// taken.
pub fn serve_unix(listener: UnixListener, sources: Sources, state: Option<PathBuf>, token: String) {
    serve_unix_tuned(listener, sources, state, cache::Tuning::default(), token);
}

/// [`serve_unix`] with the agents of `runtime` behind the routes `agents/{name}/attach`,
/// `deliver` and `restart` ([`agents`], `story:agent-kinds`); [`serve_unix`] serves an empty
/// runtime of the built-in kinds.
pub fn serve_unix_with_agents(
    listener: UnixListener,
    sources: Sources,
    state: Option<PathBuf>,
    token: String,
    runtime: crate::agent::Runtime,
) {
    let mut server = Server::new(sources, state, cache::Tuning::default(), token);
    server.agents = Mutex::new(runtime);
    run(listener, Arc::new(server));
}

/// [`serve_unix`] with `tuning`: the seam a test shortens the waits of and watches the
/// computations through.
pub fn serve_unix_tuned(
    listener: UnixListener,
    sources: Sources,
    state: Option<PathBuf>,
    tuning: cache::Tuning,
    token: String,
) {
    run(
        listener,
        Arc::new(Server::new(sources, state, tuning, token)),
    );
}

/// Serves `server` on `listener`, as [`serve_unix`] says.
fn run(listener: UnixListener, server: Arc<Server>) {
    server.waste.refresh(&server.sources);
    server.usage.refresh(&server.sources);
    // The session list is read at start and every `SESSIONS_EVERY` after, off the request path;
    // the answers that read the store are computed at start and again after each write
    // (story:dashboard-fast).
    server
        .sessions
        .start(server.sources.agents.clone(), cache::SESSIONS_EVERY);
    let warmer = Arc::clone(&server);
    std::thread::spawn(move || {
        warmer.warm(true);
        loop {
            std::thread::sleep(cache::WARM_EVERY);
            warmer.warm(false);
        }
    });
    // Doctor is taken at start and again once its last rows are older than their interval, off
    // the request path (story:dashboard-health).
    let doctor = server.doctor.clone();
    let sources = server.sources.health.clone();
    std::thread::spawn(move || {
        loop {
            doctor.refresh(&sources);
            std::thread::sleep(health::TICK);
        }
    });
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(stream) => stream,
            Err(error) => {
                eprintln!("conductor instance serve: accept: {error}");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        let server = Arc::clone(&server);
        std::thread::spawn(move || {
            if let Err(error) = connection(stream, &server) {
                eprintln!("conductor instance serve: {error}");
            }
        });
    }
}

/// How much of what a client still sends after its answer is read before the connection closes:
/// a refused body of up to several times [`answers::MAX_BODY`].
const DRAIN: u64 = (MAX_HEAD + 4 * answers::MAX_BODY) as u64;

/// Reads one request head, answers it (reading a body only where the answer takes one) and closes
/// the connection.
fn connection(mut stream: UnixStream, server: &Arc<Server>) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let (head, early) = read_head(&mut stream)?;
    let reply = {
        let mut body = |length: usize| read_body(&mut stream, &early, length);
        answer(head.as_deref(), server, &mut body)
    };
    stream.write_all(&reply)?;
    stream.flush()?;
    // Take what the client still sends before closing: closing with unread bytes resets the
    // connection, and the client may lose the answer.
    stream.shutdown(Shutdown::Write)?;
    let _ = io::copy(&mut (&stream).take(DRAIN), &mut io::sink());
    Ok(())
}

/// The body of `length` bytes: what was read with the head, `early`, then the rest from `stream`.
fn read_body(stream: &mut impl Read, early: &[u8], length: usize) -> io::Result<Vec<u8>> {
    let mut body = early[..early.len().min(length)].to_vec();
    if body.len() < length {
        let mut rest = vec![0_u8; length - body.len()];
        stream.read_exact(&mut rest)?;
        body.extend_from_slice(&rest);
    }
    Ok(body)
}

/// The request head up to its blank line, or `None` when the client stops or exceeds
/// [`MAX_HEAD`] first; with the bytes read after the blank line, the start of a body.
pub(crate) fn read_head(stream: &mut impl Read) -> io::Result<(Option<String>, Vec<u8>)> {
    let mut head = Vec::new();
    let mut chunk = [0_u8; 2048];
    loop {
        if let Some(end) = head.windows(4).position(|window| window == b"\r\n\r\n") {
            let early = head.split_off(end + 4);
            head.truncate(end);
            return Ok((Some(String::from_utf8_lossy(&head).into_owned()), early));
        }
        if head.len() > MAX_HEAD {
            return Ok((None, Vec::new()));
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok((None, Vec::new()));
        }
        head.extend_from_slice(&chunk[..read]);
    }
}

/// What a path names: one of the data routes.
enum Resource {
    Data,
    /// One of [`overview::SNAPSHOT_PATHS`].
    Snapshot,
    /// [`overview::EVENTS_PATH`].
    Events,
    /// [`overview::UNANSWERED_PATH`] (`story:decisions-notify`).
    Unanswered,
    /// [`decisions::DECISIONS_PATH`] (`story:decisions-page`).
    Decisions,
    /// [`HEALTH_PATH`].
    Health,
    /// One of [`repo::PATHS`], by its path (`story:dashboard-repo-view`).
    Repo(&'static str),
    /// [`series::SERIES_PATH`] (`story:dashboard-series-api`).
    Series,
    /// [`series::KPIS_PATH`] (`story:dashboard-kpi-panels`).
    Kpis,
}

/// The health view's data (`story:dashboard-health`, `dashboard/health.rs`).
pub const HEALTH_PATH: &str = "/v1/health";

/// The file of the app at the request path `path` (conductord serves it): `index.html` for `/`,
/// else the file at that
/// path under the build, or none. A path with an empty, `.` or `..` segment names none.
pub fn app_file(path: &str) -> Option<&'static File<'static>> {
    let relative = path.strip_prefix('/')?;
    if relative.is_empty() {
        return APP.get_file("index.html");
    }
    if relative
        .split('/')
        .any(|segment| matches!(segment, "" | "." | ".."))
        || relative.contains('\\')
    {
        return None;
    }
    APP.get_file(relative)
}

/// The content type of a file of the app, by its extension.
pub fn content_type(file: &File<'_>) -> &'static str {
    match file
        .path()
        .extension()
        .and_then(|extension| extension.to_str())
    {
        Some("html") => "text/html; charset=utf-8",
        Some("js" | "mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("json") => "application/json",
        Some("svg") => "image/svg+xml",
        Some("png") => "image/png",
        Some("ico") => "image/x-icon",
        Some("woff2") => "font/woff2",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

/// The whole HTTP answer to one request head; `body` reads the request's body of a given length,
/// which only `POST /v1/answers` does.
fn answer(
    head: Option<&str>,
    server: &Arc<Server>,
    body: &mut dyn FnMut(usize) -> io::Result<Vec<u8>>,
) -> Vec<u8> {
    const PLAIN: &str = "text/plain; charset=utf-8";
    let bad = || response(400, "Bad Request", PLAIN, b"bad request\n", "");
    let Some(head) = head else {
        return bad();
    };
    let mut lines = head.split("\r\n");
    let words: Vec<&str> = lines.next().unwrap_or_default().split(' ').collect();
    let [method, target, version] = words.as_slice() else {
        return bad();
    };
    if !version.starts_with("HTTP/1.") {
        return bad();
    }
    let headers: Vec<(&str, &str)> = lines
        .filter_map(|line| line.split_once(':'))
        .map(|(name, value)| (name.trim(), value.trim()))
        .collect();
    let header = |wanted: &str| {
        headers
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
            .map(|(_, value)| *value)
    };
    // A client that holds an answer names its `ETag`; an answer it holds is `304`.
    let condition = header("if-none-match");
    let path = target.split_once('?').map_or(*target, |(path, _)| path);
    let snapshot: Option<&'static str> = overview::SNAPSHOT_PATHS
        .into_iter()
        .find(|known| *known == path);
    // The agents of the instance (`story:agent-kinds`, design § 5.1).
    if let Some(rest) = path.strip_prefix(agents::PREFIX) {
        return agents::answer(method, rest, &header, server, body);
    }
    if path == answers::ANSWERS_PATH {
        if *method != "POST" {
            return response(
                405,
                "Method Not Allowed",
                PLAIN,
                b"only POST\n",
                "Allow: POST\r\n",
            );
        }
        return post_answers(&header, server, body);
    }
    // The Configuration page (story:dashboard-config-page): the settings, read with GET and written
    // with PUT (story:instance-api).
    if path == configure::CONFIG_PATH {
        return match *method {
            "GET" => {
                let home = server.sources.home.display().to_string();
                let answer = configure::answer(&server.sources, server.state.as_deref());
                cache::send(
                    &cache::Body::json(200, &tilde_all(answer, home.trim_end_matches('/'))),
                    condition,
                    None,
                    false,
                )
            }
            "PUT" => post_write(&header, server, body, configure::write),
            _ => response(
                405,
                "Method Not Allowed",
                PLAIN,
                b"only GET or PUT\n",
                "Allow: GET, PUT\r\n",
            ),
        };
    }
    // The Plan page (story:dashboard-plan-board): its board and its one write, which takes the
    // Configuration page's path; and one story as its drawer reads it.
    if path == plan::PLAN_PATH {
        return match *method {
            "GET" => {
                let home = server.sources.home.display().to_string();
                let asked = repo::query(target, "repository");
                let answer = plan::board(&server.sources, asked.as_deref());
                cache::send(
                    &cache::Body::json(200, &tilde_all(answer, home.trim_end_matches('/'))),
                    condition,
                    None,
                    false,
                )
            }
            "POST" => post_write(&header, server, body, plan::write),
            _ => response(
                405,
                "Method Not Allowed",
                PLAIN,
                b"only GET or POST\n",
                "Allow: GET, POST\r\n",
            ),
        };
    }
    if path == plan::STORY_PATH {
        if *method != "GET" {
            return response(
                405,
                "Method Not Allowed",
                PLAIN,
                b"only GET\n",
                "Allow: GET\r\n",
            );
        }
        let repository = repo::query(target, "repository").unwrap_or_default();
        let id = repo::query(target, "id").unwrap_or_default();
        return match plan::story(&server.sources, &repository, &id) {
            Ok(story) => {
                let home = server.sources.home.display().to_string();
                cache::send(
                    &cache::Body::json(200, &tilde_all(story, home.trim_end_matches('/'))),
                    condition,
                    None,
                    false,
                )
            }
            Err((status, why)) => response(
                status,
                if status == 404 {
                    "Not Found"
                } else {
                    "Bad Request"
                },
                PLAIN,
                format!("{why}\n").as_bytes(),
                "",
            ),
        };
    }
    let resource = match (path, snapshot) {
        ("/v1/data.json", _) => Resource::Data,
        (HEALTH_PATH, _) => Resource::Health,
        (_, Some(_)) => Resource::Snapshot,
        (overview::EVENTS_PATH, _) => Resource::Events,
        (overview::UNANSWERED_PATH, _) => Resource::Unanswered,
        (decisions::DECISIONS_PATH, _) => Resource::Decisions,
        (series::SERIES_PATH, _) => Resource::Series,
        (series::KPIS_PATH, _) => Resource::Kpis,
        _ if repo::PATHS.contains(&path) => Resource::Repo(
            repo::PATHS
                .into_iter()
                .find(|known| *known == path)
                .unwrap_or_default(),
        ),
        _ => return response(404, "Not Found", PLAIN, b"not found\n", ""),
    };
    if *method != "GET" {
        return response(
            405,
            "Method Not Allowed",
            PLAIN,
            b"only GET\n",
            "Allow: GET\r\n",
        );
    }
    // An answer built from a kept one that is stale carries when that was computed.
    let send = |body: &cache::Body, stale: Option<Instant>| {
        cache::send(body, condition, stale, stale.is_some())
    };
    let now = OffsetDateTime::now_utc();
    let home = server.sources.home.display().to_string();
    let home = home.trim_end_matches('/');
    match resource {
        Resource::Data => {
            server.waste.refresh(&server.sources);
            server.usage.refresh(&server.sources);
            let (roster, stale) = match server.roster(&server.fingerprints(), true, true) {
                Some((roster, stale)) => (roster, stale),
                None => (
                    Err("the store's controllers and open items are being read".to_owned()),
                    None,
                ),
            };
            let data = collect(
                &server.sources,
                &server.sessions.last(),
                roster,
                server.waste.view(now),
                server.usage.view(now),
                now,
            );
            send(&cache::Body::json(200, &data), stale)
        }
        Resource::Health => {
            server.waste.refresh(&server.sources);
            server.doctor.refresh(&server.sources.health);
            let waste = server.waste.view(now);
            let Some((newest, stale)) = server.snapshot_health(true, true) else {
                return unavailable();
            };
            let answer = health::answer(
                &server.sources,
                &server.doctor,
                waste,
                newest,
                &server.memory(),
            );
            send(&cache::Body::json(200, &answer), stale)
        }
        Resource::Events if overview::since(target).is_err() => {
            let problem = overview::since(target).err().unwrap_or_default();
            response(
                400,
                "Bad Request",
                PLAIN,
                format!("{problem}\n").as_bytes(),
                "",
            )
        }
        Resource::Unanswered => match server.unanswered(&server.fingerprints(), true, true) {
            Some((Ok(mut items), stale)) => {
                // An item the decision log answered is not unanswered, as on the Decisions page
                // (story:sessions-page-coherent).
                let log = jsonl(
                    &server.sources.root.join(crate::records::DECISIONS),
                    &mut Vec::new(),
                );
                items.retain(|item| !decisions::answered(&log, &item.request_id));
                send(
                    &cache::Body::json(200, &overview::unanswered(&items, now)),
                    stale,
                )
            }
            Some((Err(error), stale)) => send(&cache::Body::text(500, error), stale),
            None => unavailable(),
        },
        Resource::Decisions => decisions_answer(server, condition),
        Resource::Snapshot | Resource::Events | Resource::Series | Resource::Kpis => send_served(
            server.stored(target, &server.fingerprints(), true, true),
            condition,
        ),
        Resource::Repo(path) if STORED_PATHS.contains(&path) => send_served(
            server.stored(target, &server.fingerprints(), true, true),
            condition,
        ),
        Resource::Repo(path) => {
            let listed = server.sessions.last();
            let (answer, stale) = if path == "/v1/waiting-on" {
                let (mut facts, stale) = match server.facts(&server.fingerprints(), true, true) {
                    Some((Ok(facts), stale)) => (facts, stale),
                    Some((Err(error), stale)) => {
                        return send(&cache::Body::text(500, error), stale);
                    }
                    None => return unavailable(),
                };
                // The session list and the watch's files are read now; the store's part is kept.
                let read = |name: &str| fs::read_to_string(server.sources.watch.join(name)).ok();
                facts.sessions = listed.rows;
                facts.limits = read("limit.new");
                facts.silent = read("silent.reported");
                facts.stalled = read("stalled.reported");
                facts.queued = read("build-queued.reported");
                let mut answer = repo::waiting_on(&facts);
                answer["errors"] = json!(listed.errors);
                (answer, stale)
            } else {
                server.usage.refresh(&server.sources);
                let usage = server.usage.view(now);
                let answer = repo::endpoint(path, target, &server.sources, &listed, &usage, now)
                    .unwrap_or(Value::Null);
                (answer, None)
            };
            send(&cache::Body::json(200, &tilde_all(answer, home)), stale)
        }
    }
}

/// `POST /v1/answers` (`story:decisions-batch-answer`, [`answers`]): with its headers admitted,
/// reads the body and records the batch, one batch at a time, then drops the answers kept in
/// memory, so the next read shows what was recorded.
fn post_answers<'a>(
    header: &impl Fn(&str) -> Option<&'a str>,
    server: &Server,
    body: &mut dyn FnMut(usize) -> io::Result<Vec<u8>>,
) -> Vec<u8> {
    const PLAIN: &str = "text/plain; charset=utf-8";
    let refused = |refused: answers::Refused| {
        response(
            refused.status,
            refused.reason,
            PLAIN,
            refused.why.as_bytes(),
            "",
        )
    };
    let length = match answers::admitted(header, &server.token) {
        Ok(length) => length,
        Err(why) => return refused(why),
    };
    let Ok(bytes) = body(length) else {
        return response(
            400,
            "Bad Request",
            PLAIN,
            b"the body ended before its Content-Length\n",
            "",
        );
    };
    let recorded = {
        let _one = server.writes.lock().unwrap_or_else(PoisonError::into_inner);
        let recorded = crate::actor::as_operator(header(answers::OPERATOR_HEADER), || {
            answers::record(server.state.as_deref(), &bytes)
        });
        server.answers.clear();
        server.facts.clear();
        server.unanswered.clear();
        server.roster.clear();
        recorded
    };
    match recorded {
        Ok(results) => {
            let mut text = serde_json::to_string_pretty(&results).unwrap_or_default();
            text.push('\n');
            response(200, "OK", "application/json", text.as_bytes(), "")
        }
        Err(why) => refused(why),
    }
}

/// One page's write: its sources, the state directory and the request's body; the answer, or
/// the refusal.
type PageWrite = fn(&Sources, Option<&Path>, &[u8]) -> Result<serde_json::Value, answers::Refused>;

/// The instance API's one write path for its pages' writes: `PUT /v1/settings`
/// (`story:dashboard-config-page`, [`configure`]) and `POST /v1/plan`
/// (`story:dashboard-plan-board`, [`plan`]). Admitted as a batch of answers is (the instance's
/// token, JSON of a bounded length), then done by
/// `write` one write at a time, each recording its own decision; after a write the answers kept
/// in memory are dropped, as after a batch.
fn post_write<'a>(
    header: &impl Fn(&str) -> Option<&'a str>,
    server: &Server,
    body: &mut dyn FnMut(usize) -> io::Result<Vec<u8>>,
    write: PageWrite,
) -> Vec<u8> {
    const PLAIN: &str = "text/plain; charset=utf-8";
    let refused = |refused: answers::Refused| {
        let kind = if refused.why.starts_with('{') {
            "application/json"
        } else {
            PLAIN
        };
        response(
            refused.status,
            refused.reason,
            kind,
            refused.why.as_bytes(),
            "",
        )
    };
    let length = match answers::admitted(header, &server.token) {
        Ok(length) => length,
        Err(why) => return refused(why),
    };
    let Ok(bytes) = body(length) else {
        return response(
            400,
            "Bad Request",
            PLAIN,
            b"the body ended before its Content-Length\n",
            "",
        );
    };
    let written = {
        let _one = server.writes.lock().unwrap_or_else(PoisonError::into_inner);
        let written = crate::actor::as_operator(header(answers::OPERATOR_HEADER), || {
            write(&server.sources, server.state.as_deref(), &bytes)
        });
        server.answers.clear();
        server.facts.clear();
        server.unanswered.clear();
        server.roster.clear();
        written
    };
    match written {
        Ok(answer) => {
            let mut text = serde_json::to_string_pretty(&answer).unwrap_or_default();
            text.push('\n');
            response(200, "OK", "application/json", text.as_bytes(), "")
        }
        Err(why) => refused(why),
    }
}

/// A store that is there and does not open, answered `500` with its reason on one line.
/// `/v1/decisions` (`story:decisions-page`): the answer kept while neither the store nor the
/// decision log changed ([`Server::stored`]), `304` when `condition` names its `ETag`.
fn decisions_answer(server: &Arc<Server>, condition: Option<&str>) -> Vec<u8> {
    let fingerprints = server.fingerprints();
    send_served(
        server.stored(decisions::DECISIONS_PATH, &fingerprints, true, true),
        condition,
    )
}

fn store_failure(error: &anyhow::Error) -> cache::Body {
    cache::Body::text(500, format!("the store does not open: {error:#}\n"))
}

fn response(status: u16, reason: &str, content_type: &str, body: &[u8], extra: &str) -> Vec<u8> {
    response_with(status, reason, content_type, body, "no-store", extra)
}

/// An HTTP answer whose `Cache-Control` is `cache`: `no-store` for one a client is not to keep,
/// `no-cache` for one it keeps and asks again for with its `ETag` ([`cache::send`]).
fn response_with(
    status: u16,
    reason: &str,
    content_type: &str,
    body: &[u8],
    cache: &str,
    extra: &str,
) -> Vec<u8> {
    let mut answer = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: {cache}\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Content-Security-Policy: {CONTENT_SECURITY_POLICY}\r\n\
         Connection: close\r\n\
         {extra}\r\n",
        body.len()
    )
    .into_bytes();
    answer.extend_from_slice(body);
    answer
}

/// Everything the page shows, read from `sources` at `now` with the last session list `listed`,
/// what the store holds for the page `roster` ([`sessions`]), the disk measurement `waste` and
/// the token measurement `usage`, with the home directory written `~`.
///
/// `sessions` are the instance's sessions from its records and `controllers` the count of its
/// controller rows a live session holds (`story:sessions-page-coherent`); `awaiting_operator`
/// the Decisions page's open items, less those the decision log answered, each with its action;
/// `decisions` the log's newest decisions; `watch` the usage limits in force and the context
/// lines of the instance's sessions.
fn collect(
    sources: &Sources,
    listed: &cache::Listed,
    roster: Roster,
    waste: Value,
    usage: Value,
    now: OffsetDateTime,
) -> Value {
    let mut errors = listed.errors.clone();
    let dispatches = jsonl(&sources.root.join("dispatches"), &mut errors);
    let decisions = jsonl(&sources.root.join("decisions"), &mut errors);
    let roster = roster.unwrap_or_else(|error| {
        errors.push(error);
        sessions::Roster::default()
    });
    let shown = sessions::shown(&listed.rows, &roster, &dispatches, now);
    let mut disk = disk(&sources.disk, &mut errors);
    if let (Value::Object(disk), Value::Object(waste)) = (&mut disk, waste) {
        disk.extend(waste);
    }
    let data = json!({
        "generated_at": rfc3339(now),
        "disk": disk,
        "controllers": shown.controllers,
        "snapshot": sessions::snapshot(&roster, now),
        "build_slot": "not recorded",
        "watch": sessions::watch(&sources.watch, &shown.listed, now, &mut errors),
        "sessions": shown.rows,
        "context_handover": sources.context_handover,
        "decisions": sessions::recent(&decisions),
        "awaiting_operator": sessions::awaiting(&roster.open, &decisions),
        "usage": usage,
        "reading": {
            "sessions": command_line(&sources.agents),
            "sessions_read_at": listed.at.map(rfc3339),
            "root": sources.root.display().to_string(),
            "ci_state": sources.ci_state.display().to_string(),
            "checkouts": sources.checkouts.display().to_string(),
        },
        "errors": errors,
    });
    let home = sources.home.display().to_string();
    tilde_all(data, home.trim_end_matches('/'))
}

/// Whether the session list's `row` is a session that exited: the list keeps one, without a
/// `pid` and in the state it last had.
fn exited(row: &Value) -> bool {
    row["pid"].is_null()
}

/// The rows the session-list command prints, or none, with the reason in `errors`.
fn agents(command: &[OsString], errors: &mut Vec<String>) -> Vec<Value> {
    let shown = command_line(command);
    let Some((program, arguments)) = command.split_first() else {
        errors.push("no session-list command".to_owned());
        return Vec::new();
    };
    let output = match Command::new(program)
        .args(arguments)
        .env_remove(crate::instance::SLOTS_VARIABLE)
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            errors.push(format!("`{shown}`: {error}"));
            return Vec::new();
        }
    };
    if !output.status.success() {
        errors.push(format!("`{shown}`: {}", output.status));
        return Vec::new();
    }
    match serde_json::from_slice::<Value>(&output.stdout) {
        Ok(Value::Array(rows)) => rows,
        Ok(_) => {
            errors.push(format!("`{shown}` printed no JSON array"));
            Vec::new()
        }
        Err(error) => {
            errors.push(format!("`{shown}` printed no JSON: {error}"));
            Vec::new()
        }
    }
}

/// Every JSON object line of the `*.jsonl` files in `dir`, files in name order, lines in file
/// order; a line that is no JSON object is counted in `errors`.
fn jsonl(dir: &Path, errors: &mut Vec<String>) -> Vec<Value> {
    let mut files: Vec<PathBuf> = match fs::read_dir(dir) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok().map(|entry| entry.path()))
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "jsonl")
            })
            .collect(),
        Err(error) => {
            errors.push(format!("{}: {error}", dir.display()));
            return Vec::new();
        }
    };
    files.sort();
    let mut lines = Vec::new();
    for file in files {
        let text = match fs::read_to_string(&file) {
            Ok(text) => text,
            Err(error) => {
                errors.push(format!("{}: {error}", file.display()));
                continue;
            }
        };
        let mut unreadable = 0;
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            match serde_json::from_str::<Value>(line) {
                Ok(object @ Value::Object(_)) => lines.push(object),
                _ => unreadable += 1,
            }
        }
        if unreadable > 0 {
            errors.push(format!(
                "{}: {unreadable} line(s) are not JSON objects",
                file.display()
            ));
        }
    }
    lines
}

/// The text of the file at `path`, or none when it does not exist; any other failure is named in
/// `errors`.
fn optional(path: &Path, errors: &mut Vec<String>) -> Option<String> {
    match fs::read_to_string(path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => {
            errors.push(format!("{}: {error}", path.display()));
            None
        }
    }
}

/// Free and total bytes of the file system holding `path` ([`crate::disk::space`]); both null
/// when they cannot be read.
fn disk(path: &Path, errors: &mut Vec<String>) -> Value {
    match crate::disk::space(path) {
        Ok(space) => json!({
            "path": path.display().to_string(),
            "free_bytes": space.available,
            "size_bytes": space.total,
        }),
        Err(error) => {
            errors.push(format!("{error:#}"));
            json!({"path": path.display().to_string(), "free_bytes": null, "size_bytes": null})
        }
    }
}

fn command_line(command: &[OsString]) -> String {
    command
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ")
}

fn rfc3339(instant: OffsetDateTime) -> String {
    instant.format(&Rfc3339).unwrap_or_default()
}

/// The first [`EXCERPT`] characters of `text`.
fn excerpt(text: &str) -> String {
    text.chars().take(EXCERPT).collect()
}

/// A duration as the page shows an age: `45s`, `12m`, `3h 5m`, `2d 4h`.
fn ago(elapsed: time::Duration) -> String {
    match elapsed.whole_seconds().max(0) {
        seconds @ 0..60 => format!("{seconds}s"),
        seconds @ 60..3_600 => format!("{}m", seconds / 60),
        seconds @ 3_600..86_400 => format!("{}h {}m", seconds / 3_600, seconds % 3_600 / 60),
        seconds => format!("{}d {}h", seconds / 86_400, seconds % 86_400 / 3_600),
    }
}

/// `value` with the home directory written `~` in every string it holds.
fn tilde_all(value: Value, home: &str) -> Value {
    match value {
        Value::String(text) => Value::String(tilde(&text, home)),
        Value::Array(items) => {
            Value::Array(items.into_iter().map(|v| tilde_all(v, home)).collect())
        }
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, v)| (key, tilde_all(v, home)))
                .collect(),
        ),
        other => other,
    }
}

/// `text` with each occurrence of `home` that is a whole path (not part of a longer name, and not
/// inside another path) written `~`.
fn tilde(text: &str, home: &str) -> String {
    let path_char = |c: char| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | '/');
    if home.is_empty() {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(home) {
        let (before, after) = (&rest[..at], &rest[at + home.len()..]);
        let whole = !before.chars().next_back().is_some_and(path_char)
            && !after
                .chars()
                .next()
                .is_some_and(|c| c != '/' && path_char(c));
        out.push_str(before);
        out.push_str(if whole { "~" } else { home });
        rest = after;
    }
    out.push_str(rest);
    out
}
