//! `conductor dashboard serve`: a live, read-only page of what every session works on
//! (`story:live-dashboard`).
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `dashboard-serve` and its input `conductor.observation.DashboardServe`. It is
//! not a placed command of the `cli:` block: what it shows is presentation, not a record, and
//! nothing it reads enters the store.
//!
//! It listens on `127.0.0.1` only and answers two requests: `GET /`, an HTML page that reloads
//! itself every 20 s, and `GET /data.json`, the same data as JSON. Any other path is 404, any
//! other method on those two 405, and a request naming another `Host` 403, so a page served from
//! elsewhere cannot read the data through DNS rebinding.
//!
//! Each request reads its [`Sources`] afresh and writes nothing:
//! - the session list a command prints (`claude agents --json`): one row per session whose working
//!   directory is under the instance's checkouts root, with its current dispatch; a session
//!   without a `pid` has exited and is listed, as `exited`, below the live ones;
//! - `dispatches/*.jsonl` and `decisions/*.jsonl` under the records root;
//! - the CI watch's state file, one `repo|workflow|run id|conclusion|time` per line;
//! - the watch's usage-limit and context files (`limit.new`, `limit.seen`, `context-reported`);
//! - free space on `/`, as `df` reads it.
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
//! A source that cannot be read is named on the page; the others still show. No path under the
//! home directory is printed in full: every string the page carries writes it as `~`.

mod usage;
mod waste;

use std::collections::HashSet;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, ErrorKind, Read as _, Write as _};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context as _, Result};
use conductor_model::config::Instance;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::DashboardServeArgs;
use crate::config;

/// The port `dashboard serve` listens on without `--port`.
pub const DEFAULT_PORT: u16 = 7313;

/// Seconds between two reloads of the page.
const RELOAD_SECONDS: u32 = 20;

/// Characters of a brief, a reason or a question the page carries.
const EXCERPT: usize = 120;

/// Decision lines the page carries.
const RECENT_DECISIONS: usize = 10;

/// The longest request head read; a longer one is answered 400.
const MAX_HEAD: usize = 16 * 1024;

/// How long one connection may take to send its request or to take the answer.
const IO_TIMEOUT: Duration = Duration::from_secs(5);

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
}

impl Sources {
    /// The sources without a config file: `claude agents --json`, the repository `root`, the
    /// watch's directory ([`watch_dir`]), the built-in instance's checkouts root
    /// ([`config::built_in`]) and `/`; the `worktree` dry-runs of `root`'s workspace and
    /// `/var/tmp`, measured every 10 minutes.
    #[must_use]
    pub fn new(home: PathBuf, root: PathBuf) -> Self {
        let watch = watch_dir(&home, &root);
        let built_in = config::built_in(&home, &root).checkouts;
        let checkouts = [PathBuf::from(built_in.root), PathBuf::from(built_in.trees)];
        let targets = waste::built_in_targets(&home);
        Self::with(home, root, watch, checkouts, targets)
    }

    /// The sources of `instance`, which a config file names: `root` when `--root` names one, else
    /// the instance's `records`; the `watch/` of its `state`; its checkouts root. The rest is as
    /// [`Sources::new`] gives it.
    #[must_use]
    pub fn of(home: PathBuf, root: Option<PathBuf>, instance: &Instance) -> Self {
        let root = root.unwrap_or_else(|| PathBuf::from(&instance.records));
        let watch = Path::new(&instance.state).join(WATCH);
        let checkouts = [
            PathBuf::from(&instance.checkouts.root),
            PathBuf::from(&instance.checkouts.trees),
        ];
        let targets = Path::new(&instance.cache).join("target");
        Self::with(home, root, watch, checkouts, targets)
    }

    fn with(
        home: PathBuf,
        root: PathBuf,
        watch: PathBuf,
        checkouts: [PathBuf; 2],
        targets: PathBuf,
    ) -> Self {
        let [checkouts, trees] = checkouts;
        let worktree = |verb: &str| {
            let mut command: Vec<OsString> = ["worktree", verb, "--dry-run", "--json", "--repo"]
                .map(OsString::from)
                .to_vec();
            command.push(root.clone().into_os_string());
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
            root,
            home,
            checkouts,
            trees,
            targets,
        }
    }
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
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .context("HOME is not an absolute path; the page writes it as ~")?;
    let active = config::active();
    let sources = if active.from_file {
        Sources::of(home, args.root.clone(), &active.instance)
    } else {
        let root = match &args.root {
            Some(root) => root.clone(),
            None => find_root(&std::env::current_dir().context("read the working directory")?),
        };
        Sources::new(home, root)
    };
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

/// What every connection shares: the sources and the last disk and token measurements.
struct Server {
    sources: Sources,
    waste: waste::Cache,
    usage: usage::Cache,
}

/// Answers every connection `listener` accepts, each on a thread of its own, from `sources`. The
/// first disk and token measurements start at once.
///
/// # Errors
///
/// When the listener has no local address. Otherwise it does not return: a failed connection is
/// reported on standard error and the next one is taken.
pub fn serve(listener: TcpListener, sources: Sources) -> Result<()> {
    let port = listener.local_addr()?.port();
    let server = Arc::new(Server {
        sources,
        waste: waste::Cache::default(),
        usage: usage::Cache::default(),
    });
    server.waste.refresh(&server.sources);
    server.usage.refresh(&server.sources);
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

/// The two resources served.
enum Resource {
    Page,
    Data,
}

/// The whole HTTP answer to one request head.
fn answer(head: Option<&str>, port: u16, server: &Server) -> Vec<u8> {
    const PLAIN: &str = "text/plain; charset=utf-8";
    let Some(head) = head else {
        return response(400, "Bad Request", PLAIN, "bad request\n", "");
    };
    let mut lines = head.split("\r\n");
    let words: Vec<&str> = lines.next().unwrap_or_default().split(' ').collect();
    let [method, target, version] = words.as_slice() else {
        return response(400, "Bad Request", PLAIN, "bad request\n", "");
    };
    if !version.starts_with("HTTP/1.") {
        return response(400, "Bad Request", PLAIN, "bad request\n", "");
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
            "this server answers only its own host\n",
            "",
        );
    }
    let path = target.split_once('?').map_or(*target, |(path, _)| path);
    let resource = match path {
        "/" => Resource::Page,
        "/data.json" => Resource::Data,
        _ => return response(404, "Not Found", PLAIN, "not found\n", ""),
    };
    if *method != "GET" {
        return response(
            405,
            "Method Not Allowed",
            PLAIN,
            "only GET\n",
            "Allow: GET\r\n",
        );
    }
    server.waste.refresh(&server.sources);
    server.usage.refresh(&server.sources);
    let now = OffsetDateTime::now_utc();
    let data = collect(
        &server.sources,
        server.waste.view(now),
        server.usage.view(now),
        now,
    );
    match resource {
        Resource::Page => response(200, "OK", "text/html; charset=utf-8", &page(&data), ""),
        Resource::Data => {
            let mut body = serde_json::to_string_pretty(&data).unwrap_or_default();
            body.push('\n');
            response(200, "OK", "application/json", &body, "")
        }
    }
}

fn response(status: u16, reason: &str, content_type: &str, body: &str, extra: &str) -> Vec<u8> {
    format!(
        "HTTP/1.1 {status} {reason}\r\n\
         Content-Type: {content_type}\r\n\
         Content-Length: {}\r\n\
         Cache-Control: no-store\r\n\
         X-Content-Type-Options: nosniff\r\n\
         Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'\r\n\
         Connection: close\r\n\
         {extra}\r\n\
         {body}",
        body.len()
    )
    .into_bytes()
}

/// Everything the page shows, read from `sources` at `now` with the disk measurement `waste` and
/// the token measurement `usage`, with the home directory written `~`.
fn collect(sources: &Sources, waste: Value, usage: Value, now: OffsetDateTime) -> Value {
    let mut errors = Vec::new();
    let rows = agents(&sources.agents, &mut errors);
    let dispatches = jsonl(&sources.root.join("dispatches"), &mut errors);
    let decisions = jsonl(&sources.root.join("decisions"), &mut errors);
    let checkouts = sources.checkouts.as_path();
    let shown: Vec<&Value> = rows
        .iter()
        .filter(|row| {
            row["cwd"]
                .as_str()
                .is_some_and(|cwd| Path::new(cwd).starts_with(checkouts))
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
            name != "conductor"
                && !exited(row)
                && cwd.parent() == Some(checkouts)
                && cwd.file_name().is_some_and(|dir| dir == name)
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
/// with the newest event line of that dispatch.
fn dispatch(name: &str, dispatches: &[Value], now: OffsetDateTime) -> Option<Value> {
    let line = dispatches
        .iter()
        .rev()
        .find(|line| line["to"].as_str() == Some(name) && line["brief"].is_string())?;
    let id = line["id"].as_str()?;
    let event = dispatches
        .iter()
        .rev()
        .find(|event| event["id"].as_str() == Some(id) && event["event"].is_string());
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
/// reported above 300k tokens (`context-reported`, one session id per line). Each is null while
/// the watch has not written its file.
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

/// Free and total bytes of the file system holding `path`, as `df` reads them; both null when it
/// cannot.
fn disk(path: &Path, errors: &mut Vec<String>) -> Value {
    let output = Command::new("df")
        .args(["-B1", "--output=avail,size"])
        .arg(path)
        .output();
    let numbers = output
        .as_ref()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|output| {
            let text = String::from_utf8_lossy(&output.stdout);
            let line = text.lines().last()?.to_owned();
            let mut words = line.split_whitespace().map(str::parse::<u64>);
            Some((words.next()?.ok()?, words.next()?.ok()?))
        });
    match (numbers, output) {
        (Some((free, size)), _) => json!({
            "path": path.display().to_string(), "free_bytes": free, "size_bytes": size,
        }),
        (None, Ok(output)) => {
            errors.push(format!(
                "`df {}`: {}",
                path.display(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
            json!({"path": path.display().to_string(), "free_bytes": null, "size_bytes": null})
        }
        (None, Err(error)) => {
            errors.push(format!("`df {}`: {error}", path.display()));
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

/// `text` safe inside HTML text and attribute values.
fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// A value as a table cell shows it, escaped: a string as it is, null as `—`, anything else as
/// compact JSON.
fn cell(value: &Value) -> String {
    match value {
        Value::Null => "—".to_owned(),
        Value::String(text) => escape(text),
        other => escape(&other.to_string()),
    }
}

fn gib(bytes: &Value) -> String {
    bytes.as_u64().map_or_else(
        || "—".to_owned(),
        |bytes| format!("{:.1} GiB", bytes as f64 / f64::from(1_u32 << 30)),
    )
}

/// A size in GiB, MiB or KiB, whichever is the largest below it.
fn size(bytes: &Value) -> String {
    let Some(bytes) = bytes.as_u64() else {
        return "—".to_owned();
    };
    let bytes = bytes as f64;
    let [kib, mib, gib] = [10, 20, 30].map(|shift| f64::from(1_u32 << shift));
    if bytes >= gib {
        format!("{:.1} GiB", bytes / gib)
    } else if bytes >= mib {
        format!("{:.1} MiB", bytes / mib)
    } else {
        format!("{:.0} KiB", bytes / kib)
    }
}

/// One column of a table: its heading, and the escaped cell it shows for a row.
type Column<'a> = (&'a str, &'a dyn Fn(&Value) -> String);

/// A table of `rows`, one column per `(heading, cell)`.
fn table(out: &mut String, rows: &[Value], columns: &[Column<'_>]) {
    out.push_str("<table><tr>");
    for (heading, _) in columns {
        let _ = write!(out, "<th>{heading}</th>");
    }
    out.push_str("</tr>\n");
    for row in rows {
        out.push_str("<tr>");
        for (_, value) in columns {
            let _ = write!(out, "<td>{}</td>", value(row));
        }
        out.push_str("</tr>\n");
    }
    out.push_str("</table>\n");
}

/// The HTML page of `data`: plain HTML and CSS, no script, no external request.
fn page(data: &Value) -> String {
    let none = Vec::new();
    let rows = |key: &str| data[key].as_array().unwrap_or(&none).clone();
    let mut out = String::new();
    let _ = write!(
        out,
        "<!DOCTYPE html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\n\
         <meta http-equiv=\"refresh\" content=\"{RELOAD_SECONDS}\">\n\
         <title>conductor: what every session works on</title>\n<style>\n\
         body{{font:14px/1.4 system-ui,sans-serif;margin:1.5em;color:#222}}\n\
         table{{border-collapse:collapse;margin-bottom:1.5em}}\n\
         th,td{{border:1px solid #ccc;padding:.25em .5em;text-align:left;vertical-align:top}}\n\
         th{{background:#f2f2f2}} .meta{{color:#666}} .red{{color:#b00}}\n\
         </style></head><body>\n<h1>What every session works on</h1>\n\
         <p class=\"meta\">read {} · free on {}: {} of {} · controllers running: {} · \
         build slot: {}</p>\n",
        cell(&data["generated_at"]),
        cell(&data["disk"]["path"]),
        gib(&data["disk"]["free_bytes"]),
        gib(&data["disk"]["size_bytes"]),
        cell(&data["controllers"]),
        cell(&data["build_slot"]),
    );

    let sessions = rows("sessions");
    let gone = sessions.iter().filter(|row| row["exited"] == true).count();
    let _ = writeln!(
        out,
        "<h2>Sessions under {} ({} live, {gone} exited)</h2>",
        cell(&data["reading"]["checkouts"]),
        sessions.len() - gone
    );
    let state = |row: &Value| match (row["status"].as_str(), row["state"].as_str()) {
        (Some(status), Some(state)) if status != state => escape(&format!("{status} ({state})")),
        (Some(one), _) | (None, Some(one)) => escape(one),
        (None, None) => "—".to_owned(),
    };
    let event = |row: &Value| {
        let dispatch = &row["dispatch"];
        if dispatch["event"].is_null() {
            "—".to_owned()
        } else {
            format!(
                "{}, {} ago",
                cell(&dispatch["event"]),
                cell(&dispatch["event_ago"])
            )
        }
    };
    table(
        &mut out,
        &sessions,
        &[
            ("name", &|row: &Value| cell(&row["name"])),
            ("kind", &|row: &Value| cell(&row["kind"])),
            ("state", &state),
            ("started", &|row: &Value| {
                format!("{} ago", cell(&row["started_ago"]))
            }),
            ("cwd", &|row: &Value| cell(&row["cwd"])),
            ("dispatch", &|row: &Value| cell(&row["dispatch"]["id"])),
            ("goal", &|row: &Value| cell(&row["dispatch"]["goal"])),
            ("brief", &|row: &Value| cell(&row["dispatch"]["brief"])),
            ("last event", &event),
        ],
    );

    out.push_str("<h2>Usage limit and context</h2>\n");
    watch_lines(&mut out, &data["watch"]);

    out.push_str("<h2>Red CI on main</h2>\n");
    match data["red_ci"].as_array() {
        None => out.push_str("<p class=\"meta\">no watch state</p>\n"),
        Some(red) if red.is_empty() => out.push_str("<p class=\"meta\">none</p>\n"),
        Some(red) => table(
            &mut out,
            red,
            &[
                ("repository", &|row: &Value| cell(&row["repo"])),
                ("workflow", &|row: &Value| cell(&row["workflow"])),
                ("run", &|row: &Value| cell(&row["run_id"])),
                ("conclusion", &|row: &Value| {
                    format!("<span class=\"red\">{}</span>", cell(&row["conclusion"]))
                }),
                ("time", &|row: &Value| cell(&row["time"])),
            ],
        ),
    }

    let waiting = rows("awaiting_operator");
    let _ = writeln!(out, "<h2>Awaiting the operator ({})</h2>", waiting.len());
    table(
        &mut out,
        &waiting,
        &[
            ("decision", &|row: &Value| cell(&row["id"])),
            ("class", &|row: &Value| cell(&row["class"])),
            ("question", &|row: &Value| cell(&row["question"])),
        ],
    );

    out.push_str("<h2>Recent decisions</h2>\n");
    table(
        &mut out,
        &rows("decisions"),
        &[
            ("decision", &|row: &Value| cell(&row["id"])),
            ("class", &|row: &Value| cell(&row["class"])),
            ("choice", &|row: &Value| cell(&row["choice"])),
            ("reason", &|row: &Value| cell(&row["reason"])),
        ],
    );

    out.push_str("<h2>Disk waste</h2>\n");
    waste_table(&mut out, &data["disk"]);

    usage_table(&mut out, &data["usage"]);

    let errors = rows("errors");
    if !errors.is_empty() {
        out.push_str("<h2 class=\"red\">Could not read</h2>\n<ul>\n");
        for error in &errors {
            let _ = writeln!(out, "<li>{}</li>", cell(error));
        }
        out.push_str("</ul>\n");
    }
    let reading = &data["reading"];
    let _ = write!(
        out,
        "<p class=\"meta\">sessions from <code>{}</code> · records under {} · CI from {} · \
         reloads every {RELOAD_SECONDS} s</p>\n</body></html>\n",
        cell(&reading["sessions"]),
        cell(&reading["root"]),
        cell(&reading["ci_state"]),
    );
    out
}

/// The watch's lines as it prints them: each usage limit found now with its sessions, each
/// reported earlier today, and each session whose context passed 300k tokens.
fn watch_lines(out: &mut String, watch: &Value) {
    let none = Vec::new();
    let list = |key: &str| watch[key].as_array().unwrap_or(&none);
    if ["usage_limit", "usage_limit_today", "context"]
        .iter()
        .all(|key| watch[*key].is_null())
    {
        out.push_str("<p class=\"meta\">no watch state</p>\n");
        return;
    }
    let mut lines = Vec::new();
    for limit in list("usage_limit") {
        let sessions: Vec<String> = limit["sessions"]
            .as_array()
            .unwrap_or(&none)
            .iter()
            .map(cell)
            .collect();
        lines.push(format!(
            "<li class=\"red\">usage limit: {}; {} sessions: {}</li>",
            cell(&limit["text"]),
            sessions.len(),
            sessions.join(" ")
        ));
    }
    for limit in list("usage_limit_today") {
        lines.push(format!("<li>usage limit seen today: {}</li>", cell(limit)));
    }
    for context in list("context") {
        let name = context["name"]
            .as_str()
            .map_or_else(|| "a session".to_owned(), escape);
        lines.push(format!(
            "<li class=\"red\">context: {name} over 300k tokens (session {})</li>",
            cell(&context["session"])
        ));
    }
    if lines.is_empty() {
        out.push_str("<p class=\"meta\">none</p>\n");
    } else {
        let _ = writeln!(out, "<ul>\n{}\n</ul>", lines.join("\n"));
    }
}

/// The disk waste of the `disk` object: its total and age, then its rows, largest first.
fn waste_table(out: &mut String, disk: &Value) {
    let again = if disk["measuring"] == true {
        " · measuring again"
    } else {
        ""
    };
    if disk["measured_at"].is_null() {
        out.push_str(if disk["measuring"] == true {
            "<p class=\"meta\">measuring, no measurement yet</p>\n"
        } else {
            "<p class=\"meta\">no measurement</p>\n"
        });
        return;
    }
    let none = Vec::new();
    let rows = disk["rows"].as_array().unwrap_or(&none);
    let _ = writeln!(
        out,
        "<p class=\"meta\">total {} over {} rows · measured {} ({} ago){again} · \
         read-only: cleanup goes through each tree's owner</p>",
        size(&disk["total_bytes"]),
        rows.len(),
        cell(&disk["measured_at"]),
        cell(&disk["measured_ago"]),
    );
    table(
        out,
        rows,
        &[
            ("size", &|row: &Value| size(&row["bytes"])),
            ("repository", &|row: &Value| cell(&row["repository"])),
            ("what", &|row: &Value| cell(&row["what"])),
            ("path", &|row: &Value| cell(&row["path"])),
            ("trees or entries", &|row: &Value| cell(&row["count"])),
            ("owner", &|row: &Value| cell(&row["owner"])),
            ("age", &|row: &Value| cell(&row["age"])),
            ("state", &|row: &Value| cell(&row["state"])),
        ],
    );
}

/// The token spend of the `usage` object: the instant it counts from and its age, its rows
/// largest first, then what could not be read.
fn usage_table(out: &mut String, usage: &Value) {
    if usage["measured_at"].is_null() {
        out.push_str("<h2>Token spend</h2>\n");
        out.push_str(if usage["measuring"] == true {
            "<p class=\"meta\">measuring, no measurement yet</p>\n"
        } else {
            "<p class=\"meta\">no measurement</p>\n"
        });
        return;
    }
    let again = if usage["measuring"] == true {
        " · measuring again"
    } else {
        ""
    };
    let _ = writeln!(
        out,
        "<h2>Token spend since {}</h2>\n<p class=\"meta\">measured {} ({} ago){again} · from the \
         session transcripts; per session: <code>conductor resource usage</code></p>",
        cell(&usage["since"]),
        cell(&usage["measured_at"]),
        cell(&usage["measured_ago"]),
    );
    let none = Vec::new();
    table(
        out,
        usage["rows"].as_array().unwrap_or(&none),
        &[
            ("repository", &|row: &Value| cell(&row["repository"])),
            ("sessions", &|row: &Value| cell(&row["sessions"])),
            ("calls", &|row: &Value| cell(&row["calls"])),
            ("input", &|row: &Value| cell(&row["input"])),
            ("output", &|row: &Value| cell(&row["output"])),
            ("cache read", &|row: &Value| cell(&row["cache_read"])),
            ("cache write", &|row: &Value| cell(&row["cache_write"])),
            ("cache read %", &|row: &Value| {
                cell(&row["cache_read_percent"])
            }),
        ],
    );
    let errors = usage["errors"].as_array().unwrap_or(&none);
    if !errors.is_empty() {
        out.push_str("<ul>\n");
        for error in errors {
            let _ = writeln!(out, "<li class=\"red\">{}</li>", cell(error));
        }
        out.push_str("</ul>\n");
    }
}
