//! `conductor dashboard serve`: a live, read-only page of what every session works on
//! (`story:live-dashboard`).
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `dashboard-serve` and its input `conductor.observation.DashboardServe`. It is
//! not a placed command of the `cli:` block: what it shows is presentation, not a record, and
//! nothing it reads enters the store.
//!
//! It listens on `127.0.0.1` only and answers `GET` of the app and of its data
//! (`story:dashboard-spa`): `/` is the `index.html` of the Vue app built into
//! `crates/conductor/dashboard/dist/` (`task ui`), each file under that directory is served at its
//! own path with its content type, and `/data.json` is the data the app reads every 20 s. The
//! build is embedded in the binary ([`APP`]), so serving it needs neither Node nor a file next to
//! the binary. Any other path is 404, any other method on those 405, and a request naming another
//! `Host` 403, so a page served from elsewhere cannot read the data through DNS rebinding. Every
//! answer carries [`CONTENT_SECURITY_POLICY`]: the app loads its script, its style and its data
//! from this server and nothing else.
//!
//! The overview's endpoints (`story:dashboard-overview`, [`overview`]): `/api/board`,
//! `/api/repositories`, `/api/pull-requests`, `/api/workflow-runs` and `/api/releases` answer
//! the newest complete snapshot of the store under the instance's state directory, opened only
//! when it is there ([`serve_over`] names another); `/api/events?since=<rfc3339>` the events since
//! then, 00:00 UTC of the day without `since`.
//!
//! Each request reads its [`Sources`] afresh and writes nothing:
//! - the session list a command prints (`claude agents --json`): one row per session whose working
//!   directory is under the instance's checkouts root, with its current dispatch; a session
//!   without a `pid` has exited and is listed, as `exited`, below the live ones;
//! - `dispatches/*.jsonl` and `decisions/*.jsonl` under the records root;
//! - the CI watch's state file, one `repo|workflow|run id|conclusion|time` per line;
//! - the watch's usage-limit and context files (`limit.new`, `limit.seen`, `context-reported`);
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
//! home directory is printed in full: every string `/data.json` carries writes it as `~`.

mod health;
pub mod overview;
pub mod repo;
mod usage;
pub(crate) mod waste;

pub use health::{HealthSources, failure_class};

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::io::{self, ErrorKind, Read as _, Write as _};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use conductor_model::config::Instance;
use conductor_model::observation::FailureClass;
use include_dir::{Dir, File, include_dir};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::DashboardServeArgs;
use crate::config;

/// The port `dashboard serve` listens on without `--port`.
pub const DEFAULT_PORT: u16 = 7313;

/// The dashboard app as `task ui` builds it: `index.html` and its assets, embedded at build time.
static APP: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/dashboard/dist");

/// The policy every answer carries: the app's script, style and data come from this server only.
pub const CONTENT_SECURITY_POLICY: &str =
    "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'";

/// Characters of a brief, a reason or a question `/data.json` carries.
const EXCERPT: usize = 120;

/// Decision lines `/data.json` carries.
const RECENT_DECISIONS: usize = 10;

/// The longest request head read; a longer one is answered 400.
const MAX_HEAD: usize = 16 * 1024;

/// How long one connection may take to send its request or to take the answer.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

/// What to do about a member a collector could not read, by the class of its error's first line
/// ([`failure_class`]): the one table the health view's fixes come from
/// (`story:dashboard-health`), each class with the name `/api/health` gives it.
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
    /// The directory whose entries the disk measurement lists as `/var/tmp`.
    pub var_tmp: PathBuf,
    /// How long one disk measurement is shown before a request starts the next.
    pub measure_every: Duration,
    /// How long one command of the disk measurement may run before it is stopped.
    pub measure_timeout: Duration,
    /// Tokens of a session's context above which the watch reports it, which the context line
    /// names: the instance's `thresholds.context_handover`.
    pub context_handover: u64,
    /// What the health view reads besides the disk-waste measurement (`story:dashboard-health`).
    pub health: HealthSources,
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
        Self::with(home, root, watch, checkouts, targets, context_handover)
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
        sources.health = HealthSources::of(sources.home.clone(), instance);
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
            return Ok(Self::of(home, root, &active.instance));
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
            var_tmp: PathBuf::from("/var/tmp"),
            measure_every: Duration::from_secs(600),
            measure_timeout: Duration::from_secs(180),
            // The built-in instance's; [`Sources::of`] gives an instance's own.
            health: HealthSources::new(home.clone(), &root),
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

/// `conductor dashboard serve`: listens on `127.0.0.1:<--port>` (default [`DEFAULT_PORT`]; 0
/// picks a free port), prints `listening on http://127.0.0.1:<port>` as its first line, and
/// serves until it is stopped.
///
/// # Errors
///
/// When `HOME` is not an absolute path, the working directory cannot be read, or the port cannot
/// be bound.
pub fn dashboard_serve(args: &DashboardServeArgs) -> Result<ExitCode> {
    let sources = Sources::active(args.root.clone())?;
    let port = args.port.unwrap_or(DEFAULT_PORT);
    let listener = TcpListener::bind(("127.0.0.1", port))
        .with_context(|| format!("listen on 127.0.0.1:{port}"))?;
    let bound = listener.local_addr()?.port();
    let mut out = io::stdout().lock();
    writeln!(out, "listening on http://127.0.0.1:{bound}")?;
    out.flush()?;
    drop(out);
    serve(listener, sources)?;
    Ok(ExitCode::SUCCESS)
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
/// endpoints read, and the last disk and token measurements.
struct Server {
    sources: Sources,
    state: Option<PathBuf>,
    waste: waste::Cache,
    usage: usage::Cache,
    /// Doctor's last rows, which the health view serves (`story:dashboard-health`).
    doctor: health::Doctor,
}

/// Answers every connection `listener` accepts, each on a thread of its own, from `sources`. The
/// first disk and token measurements start at once. The snapshot endpoints read the store under
/// the state directory [`crate::state::dir`] decides without a flag: the instance's `state`.
///
/// # Errors
///
/// When the listener has no local address. Otherwise it does not return: a failed connection is
/// reported on standard error and the next one is taken.
pub fn serve(listener: TcpListener, sources: Sources) -> Result<()> {
    serve_over(listener, sources, crate::state::dir(None).ok())
}

/// [`serve`], the snapshot endpoints reading the store under `state`, or none when it is `None`:
/// the seam a test points at a fixture store (`story:dashboard-overview`).
///
/// # Errors
///
/// As [`serve`].
pub fn serve_over(listener: TcpListener, sources: Sources, state: Option<PathBuf>) -> Result<()> {
    let port = listener.local_addr()?.port();
    let server = Arc::new(Server {
        sources,
        state,
        waste: waste::Cache::default(),
        usage: usage::Cache::default(),
        doctor: health::Doctor::default(),
    });
    server.waste.refresh(&server.sources);
    server.usage.refresh(&server.sources);
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
                eprintln!("conductor dashboard: accept: {error}");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        let server = Arc::clone(&server);
        std::thread::spawn(move || {
            if let Err(error) = connection(stream, port, &server) {
                eprintln!("conductor dashboard: {error}");
            }
        });
    }
    Ok(())
}

/// Reads one request head, answers it and closes the connection.
fn connection(mut stream: TcpStream, port: u16, server: &Server) -> io::Result<()> {
    stream.set_read_timeout(Some(IO_TIMEOUT))?;
    stream.set_write_timeout(Some(IO_TIMEOUT))?;
    let head = read_head(&mut stream)?;
    stream.write_all(&answer(head.as_deref(), port, server))?;
    stream.flush()?;
    // Take what the client still sends before closing: closing with unread bytes resets the
    // connection, and the client may lose the answer.
    stream.shutdown(Shutdown::Write)?;
    let _ = io::copy(&mut (&stream).take(MAX_HEAD as u64), &mut io::sink());
    Ok(())
}

/// The request head up to its blank line, or `None` when the client stops or exceeds
/// [`MAX_HEAD`] first.
fn read_head(stream: &mut TcpStream) -> io::Result<Option<String>> {
    let mut head = Vec::new();
    let mut chunk = [0_u8; 2048];
    loop {
        if let Some(end) = head.windows(4).position(|window| window == b"\r\n\r\n") {
            head.truncate(end);
            return Ok(Some(String::from_utf8_lossy(&head).into_owned()));
        }
        if head.len() > MAX_HEAD {
            return Ok(None);
        }
        let read = stream.read(&mut chunk)?;
        if read == 0 {
            return Ok(None);
        }
        head.extend_from_slice(&chunk[..read]);
    }
}

/// What a path names: a file of the app, or the data.
enum Resource {
    App(&'static File<'static>),
    Data,
    /// One of [`overview::SNAPSHOT_PATHS`], by its path.
    Snapshot(&'static str),
    /// [`overview::EVENTS_PATH`].
    Events,
    /// [`HEALTH_PATH`].
    Health,
    /// One of [`repo::PATHS`], by its path (`story:dashboard-repo-view`).
    Repo(&'static str),
}

/// The health view's data (`story:dashboard-health`, `dashboard/health.rs`).
pub const HEALTH_PATH: &str = "/api/health";

/// The file of the app at the request path `path`: `index.html` for `/`, else the file at that
/// path under the build, or none. A path with an empty, `.` or `..` segment names none.
fn app_file(path: &str) -> Option<&'static File<'static>> {
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
fn content_type(file: &File<'_>) -> &'static str {
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

/// The whole HTTP answer to one request head.
fn answer(head: Option<&str>, port: u16, server: &Server) -> Vec<u8> {
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
    let host = lines
        .filter_map(|line| line.split_once(':'))
        .find(|(name, _)| name.trim().eq_ignore_ascii_case("host"))
        .map(|(_, value)| value.trim());
    let own = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    if !host.is_some_and(|host| own.iter().any(|own| own.eq_ignore_ascii_case(host))) {
        return response(
            403,
            "Forbidden",
            PLAIN,
            b"this server answers only its own host\n",
            "",
        );
    }
    let path = target.split_once('?').map_or(*target, |(path, _)| path);
    let snapshot: Option<&'static str> = overview::SNAPSHOT_PATHS
        .into_iter()
        .find(|known| *known == path);
    let resource = match (path, snapshot) {
        ("/data.json", _) => Resource::Data,
        (HEALTH_PATH, _) => Resource::Health,
        (_, Some(known)) => Resource::Snapshot(known),
        (overview::EVENTS_PATH, _) => Resource::Events,
        _ if repo::PATHS.contains(&path) => Resource::Repo(
            repo::PATHS
                .into_iter()
                .find(|known| *known == path)
                .unwrap_or_default(),
        ),
        _ => match app_file(path) {
            Some(file) => Resource::App(file),
            None => return response(404, "Not Found", PLAIN, b"not found\n", ""),
        },
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
    match resource {
        Resource::App(file) => response(200, "OK", content_type(file), file.contents(), ""),
        Resource::Data => {
            server.waste.refresh(&server.sources);
            server.usage.refresh(&server.sources);
            let now = OffsetDateTime::now_utc();
            let data = collect(
                &server.sources,
                server.waste.view(now),
                server.usage.view(now),
                now,
            );
            let mut body = serde_json::to_string_pretty(&data).unwrap_or_default();
            body.push('\n');
            response(200, "OK", "application/json", body.as_bytes(), "")
        }
        Resource::Health => {
            server.waste.refresh(&server.sources);
            server.doctor.refresh(&server.sources.health);
            let waste = server.waste.view(OffsetDateTime::now_utc());
            json_answer(&health::answer(&server.sources, &server.doctor, waste))
        }
        Resource::Snapshot(path) => match open_store(server.state.as_deref()) {
            Ok(store) => json_answer(
                &overview::snapshot_endpoint(path, store.as_ref()).unwrap_or(Value::Null),
            ),
            Err(error) => store_failure(&error),
        },
        Resource::Events => {
            let now = OffsetDateTime::now_utc();
            let since = match overview::since(target) {
                Ok(since) => since.unwrap_or_else(|| overview::start_of_day(now)),
                Err(problem) => {
                    return response(
                        400,
                        "Bad Request",
                        PLAIN,
                        format!("{problem}\n").as_bytes(),
                        "",
                    );
                }
            };
            let store = match open_store(server.state.as_deref()) {
                Ok(store) => store,
                Err(error) => return store_failure(&error),
            };
            let mut errors = Vec::new();
            let root = &server.sources.root;
            let dispatches = jsonl(&root.join("dispatches"), &mut errors);
            let decisions = jsonl(&root.join("decisions"), &mut errors);
            let events = overview::events(
                store.as_ref(),
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
            let home = server.sources.home.display().to_string();
            json_answer(&tilde_all(events, home.trim_end_matches('/')))
        }
        Resource::Repo(path) => match open_store(server.state.as_deref()) {
            Ok(store) => {
                server.usage.refresh(&server.sources);
                let now = OffsetDateTime::now_utc();
                let usage = server.usage.view(now);
                let answer =
                    repo::endpoint(path, target, store.as_ref(), &server.sources, &usage, now)
                        .unwrap_or(Value::Null);
                let home = server.sources.home.display().to_string();
                json_answer(&tilde_all(answer, home.trim_end_matches('/')))
            }
            Err(error) => store_failure(&error),
        },
    }
}

/// `value` answered as JSON, `200 OK`.
fn json_answer(value: &Value) -> Vec<u8> {
    let mut body = serde_json::to_string_pretty(value).unwrap_or_default();
    body.push('\n');
    response(200, "OK", "application/json", body.as_bytes(), "")
}

/// The store under `state`, opened read-only as a view opens it ([`crate::store::open_existing`]),
/// or none when there is no state directory or no store in it (`tree/`, as
/// [`crate::store::open_existing`] looks for it): an endpoint then answers no snapshot.
///
/// # Errors
///
/// A store is there and does not open.
fn open_store(state: Option<&Path>) -> Result<Option<crate::store::Store>> {
    match state {
        Some(state) if state.join("tree").exists() => crate::store::open_existing(state).map(Some),
        _ => Ok(None),
    }
}

/// A store that is there and does not open, answered `500` with its reason on one line.
fn store_failure(error: &anyhow::Error) -> Vec<u8> {
    response(
        500,
        "Internal Server Error",
        "text/plain; charset=utf-8",
        format!("the store does not open: {error:#}\n").as_bytes(),
        "",
    )
}

fn response(status: u16, reason: &str, content_type: &str, body: &[u8], extra: &str) -> Vec<u8> {
    let mut answer = format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
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

/// Everything the page shows, read from `sources` at `now` with the disk measurement `waste` and
/// the token measurement `usage`, with the home directory written `~`.
fn collect(sources: &Sources, waste: Value, usage: Value, now: OffsetDateTime) -> Value {
    let mut errors = Vec::new();
    let rows = agents(&sources.agents, &mut errors);
    let dispatches = jsonl(&sources.root.join("dispatches"), &mut errors);
    let decisions = jsonl(&sources.root.join("decisions"), &mut errors);
    let checkouts = sources.checkouts.as_path();
    let active = config::active();
    let instances = active.all();
    // Another instance's session is not this page's, wherever it works: its name and directory
    // say whose it is (story:instance-session-names).
    let theirs = |row: &Value| {
        let (Some(name), Some(cwd)) = (row["name"].as_str(), row["cwd"].as_str()) else {
            return false;
        };
        crate::collect::session_instance(&instances, name, Some(Path::new(cwd)))
            .is_some_and(|owner| owner.name != active.instance.name)
    };
    let shown: Vec<&Value> = rows
        .iter()
        .filter(|row| {
            row["cwd"]
                .as_str()
                .is_some_and(|cwd| Path::new(cwd).starts_with(checkouts))
                && !theirs(row)
        })
        .collect();
    // A controller is named after its repository and runs in its checkout (design § 2);
    // conductor, in its own checkout, is not one, and one that exited does not run.
    let controllers = shown
        .iter()
        .filter(|row| {
            let (Some(name), Some(cwd)) = (row["name"].as_str(), row["cwd"].as_str()) else {
                return false;
            };
            let cwd = Path::new(cwd);
            // A controller's session is named `<session_prefix>-<repository>`, or the repository
            // without a prefix (story:instance-session-names).
            let instance = &active.instance;
            name != crate::config::session_name(instance, "conductor").0
                && !exited(row)
                && cwd.parent() == Some(checkouts)
                && cwd.file_name().is_some_and(|dir| {
                    dir.to_str()
                        .is_some_and(|dir| crate::config::session_name(instance, dir).0 == name)
                })
        })
        .count();
    let mut sessions: Vec<Value> = shown
        .iter()
        .map(|row| session(row, &dispatches, now))
        .collect();
    sessions.sort_by(|a, b| {
        let key = |row: &Value| {
            (
                row["exited"] == true,
                row["name"].is_null(),
                row["name"].as_str().map(str::to_owned),
            )
        };
        key(a).cmp(&key(b))
    });
    let mut disk = disk(&sources.disk, &mut errors);
    if let (Value::Object(disk), Value::Object(waste)) = (&mut disk, waste) {
        disk.extend(waste);
    }
    let data = json!({
        "generated_at": rfc3339(now),
        "disk": disk,
        "controllers": controllers,
        "build_slot": "not recorded",
        "sessions": sessions,
        "watch": watch(&sources.watch, &rows, now, &mut errors),
        "context_handover": sources.context_handover,
        "red_ci": red_ci(&sources.ci_state, &mut errors),
        "decisions": recent(&decisions),
        "awaiting_operator": awaiting(&decisions),
        "usage": usage,
        "reading": {
            "sessions": command_line(&sources.agents),
            "root": sources.root.display().to_string(),
            "ci_state": sources.ci_state.display().to_string(),
            "checkouts": checkouts.display().to_string(),
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

/// One shown session: its name, kind, status and state (`exited` for one that exited), working
/// directory, start, and current dispatch.
fn session(row: &Value, dispatches: &[Value], now: OffsetDateTime) -> Value {
    let name = row["name"].as_str();
    let started = row["startedAt"].as_i64().and_then(|millis| {
        OffsetDateTime::from_unix_timestamp_nanos(i128::from(millis) * 1_000_000).ok()
    });
    let exited = exited(row);
    json!({
        "name": name,
        "kind": row["kind"].clone(),
        "exited": exited,
        "status": if exited { Value::Null } else { row["status"].clone() },
        "state": if exited { json!("exited") } else { row["state"].clone() },
        "cwd": row["cwd"].clone(),
        "started_at": started.map(rfc3339),
        "started_ago": started.map(|at| ago(now - at)),
        "dispatch": name.and_then(|name| dispatch(name, dispatches, now)),
    })
}

/// The newest dispatch line to `name` (a line whose `to` is `name` and that carries a brief),
/// with the session's last event: of the event lines of every dispatch sent to `name`, the one
/// whose `at` is latest, so a report of minutes ago wins over a state written after it with an
/// older `at` (`story:dashboard-overview`). Without an event line whose `at` reads, the last
/// event line of the newest dispatch, as before.
fn dispatch(name: &str, dispatches: &[Value], now: OffsetDateTime) -> Option<Value> {
    let sent_to = |line: &Value| line["to"].as_str() == Some(name) && line["brief"].is_string();
    let line = dispatches.iter().rev().find(|line| sent_to(line))?;
    let id = line["id"].as_str()?;
    let sent: HashSet<&str> = dispatches
        .iter()
        .filter(|line| sent_to(line))
        .filter_map(|line| line["id"].as_str())
        .collect();
    let newest_report = dispatches
        .iter()
        .enumerate()
        .filter(|(_, event)| {
            event["event"].is_string()
                && event["id"]
                    .as_str()
                    .is_some_and(|dispatch| sent.contains(dispatch))
        })
        .filter_map(|(read, event)| {
            let at = OffsetDateTime::parse(event["at"].as_str()?, &Rfc3339).ok()?;
            Some(((at, read), event))
        })
        .max_by_key(|(key, _)| *key)
        .map(|(_, event)| event);
    let event = newest_report.or_else(|| {
        dispatches
            .iter()
            .rev()
            .find(|event| event["id"].as_str() == Some(id) && event["event"].is_string())
    });
    let at = event
        .and_then(|event| event["at"].as_str())
        .and_then(|at| OffsetDateTime::parse(at, &Rfc3339).ok());
    Some(json!({
        "id": id,
        "goal": line["goal"].clone(),
        "brief": excerpt(line["brief"].as_str().unwrap_or_default()),
        "event": event.map(|event| event["event"].clone()),
        "event_at": at.map(rfc3339),
        "event_ago": at.map(|at| ago(now - at)),
    }))
}

/// The newest [`RECENT_DECISIONS`] decision lines, newest first.
fn recent(decisions: &[Value]) -> Vec<Value> {
    decisions
        .iter()
        .rev()
        .take(RECENT_DECISIONS)
        .map(|line| {
            json!({
                "id": line["id"].clone(),
                "class": line["class"].clone(),
                "choice": line["choice"].clone(),
                "reason": line["reason"].as_str().map(excerpt),
            })
        })
        .collect()
}

/// The decisions whose newest line is of class O or H and carries no choice yet, newest first.
fn awaiting(decisions: &[Value]) -> Vec<Value> {
    let mut seen = HashSet::new();
    decisions
        .iter()
        .rev()
        .filter(|line| seen.insert(line["id"].to_string()))
        .filter(|line| {
            let open = match &line["choice"] {
                Value::Null => true,
                Value::String(choice) => choice.trim().is_empty(),
                _ => false,
            };
            open && matches!(line["class"].as_str(), Some("O" | "H"))
        })
        .map(|line| {
            json!({
                "id": line["id"].clone(),
                "class": line["class"].clone(),
                "question": line["question"].as_str().map(excerpt),
            })
        })
        .collect()
}

/// The rows the session-list command prints, or none, with the reason in `errors`.
fn agents(command: &[OsString], errors: &mut Vec<String>) -> Vec<Value> {
    let shown = command_line(command);
    let Some((program, arguments)) = command.split_first() else {
        errors.push("no session-list command".to_owned());
        return Vec::new();
    };
    let output = match Command::new(program).args(arguments).output() {
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

/// The red rows of the CI watch's state file (a conclusion of [`crate::watch::RED`]), or null
/// when the watch has written none.
fn red_ci(path: &Path, errors: &mut Vec<String>) -> Value {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return Value::Null,
        Err(error) => {
            errors.push(format!("{}: {error}", path.display()));
            return Value::Null;
        }
    };
    let mut rows = Vec::new();
    let mut unreadable = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        let fields: Vec<&str> = line.split('|').collect();
        let [repo, workflow, run_id, conclusion, time] = fields.as_slice() else {
            unreadable += 1;
            continue;
        };
        if crate::watch::RED.contains(conclusion) {
            rows.push(json!({
                "repo": repo, "workflow": workflow, "run_id": run_id,
                "conclusion": conclusion, "time": time,
            }));
        }
    }
    if unreadable > 0 {
        errors.push(format!(
            "{}: {unreadable} line(s) are not repo|workflow|run id|conclusion|time",
            path.display()
        ));
    }
    Value::Array(rows)
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

/// The watch's usage-limit and context lines, from its directory `dir`: the limits it finds now
/// (`limit.new`, one `text<TAB>session` per line) with their sessions, those it reported today
/// (`limit.seen`, one `YYYY-MM-DD text` per line, UTC), and the sessions in `rows` whose context it
/// reported above the context hand-over threshold (`context-reported`, one session id per line).
/// Each is null while the watch has not written its file.
fn watch(dir: &Path, rows: &[Value], now: OffsetDateTime, errors: &mut Vec<String>) -> Value {
    let limits = optional(&dir.join("limit.new"), errors).map(|text| {
        let mut limits: Vec<(&str, Vec<&str>)> = Vec::new();
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let (limit, session) = line.split_once('\t').unwrap_or((line, ""));
            let at = limits
                .iter()
                .position(|(known, _)| *known == limit)
                .unwrap_or_else(|| {
                    limits.push((limit, Vec::new()));
                    limits.len() - 1
                });
            if !session.trim().is_empty() {
                limits[at].1.push(session.trim());
            }
        }
        limits
            .into_iter()
            .map(|(text, mut sessions)| {
                sessions.sort_unstable();
                sessions.dedup();
                json!({"text": text, "sessions": sessions})
            })
            .collect::<Vec<_>>()
    });
    let today = now.date().to_string();
    let seen = optional(&dir.join("limit.seen"), errors).map(|text| {
        let mut seen: Vec<String> = Vec::new();
        for line in text.lines() {
            if let Some((date, limit)) = line.split_once(' ')
                && date == today
                && !seen.iter().any(|known| known == limit)
            {
                seen.push(limit.to_owned());
            }
        }
        seen
    });
    let context = optional(&dir.join("context-reported"), errors).map(|text| {
        let mut ids: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|id| !id.is_empty())
            .collect();
        let mut once = HashSet::new();
        ids.retain(|id| once.insert(*id));
        ids.into_iter()
            .filter_map(|id| {
                let row = rows
                    .iter()
                    .find(|row| row["sessionId"].as_str() == Some(id))?;
                Some(json!({
                    "session": id.chars().take(8).collect::<String>(),
                    "name": row["name"].clone(),
                }))
            })
            .collect::<Vec<_>>()
    });
    json!({"usage_limit": limits, "usage_limit_today": seen, "context": context})
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
