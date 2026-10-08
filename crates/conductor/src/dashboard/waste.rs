//! The disk waste across the tracked repositories (`story:disk-waste`), measured on a thread of
//! its own and shown with its age.
//!
//! A tracked repository is a checkout directly under the instance's checkouts root. Per
//! repository the measurement reads the recognised build cache of its idle managed trees
//! (`worktree sweep --dry-run`), its trees eligible for removal (`worktree gc --dry-run`, then
//! `du`), its archives under `~/.local/state/worktree/archives/` and its checkout's `target/`.
//! Shared, it reads each entry of the shared build targets (`target` under the instance's cache
//! when a config file names it, else `~/.cache/<namespace>-target`: [`built_in_targets`]), sccache, the
//! rustup toolchains, the agents' scratch `~/.cache/claude-tmp` and the entries of `/var/tmp`. It
//! deletes nothing.
//!
//! Every command runs under [`Sources::measure_timeout`]; one that runs longer is stopped and its
//! row says so. A request never waits for a measurement: it shows the last complete one and
//! starts the next when that one is older than [`Sources::measure_every`].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::os::unix::fs::MetadataExt as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::{Value, json};
use time::OffsetDateTime;

use super::{Sources, ago, rfc3339};
use crate::collect;

/// A `/var/tmp` entry at least this large is a row of its own; the smaller ones share one row.
const VAR_TMP_ROW: u64 = 64 << 20;

/// How often a running command is looked at.
const POLL: Duration = Duration::from_millis(20);

const BUILD_CACHE: &str = "build cache in idle managed trees";
const ELIGIBLE: &str = "trees eligible for removal";

/// The last complete measurement, and whether the next one runs; shared by every request.
#[derive(Debug, Clone, Default)]
pub(super) struct Cache(Arc<Mutex<State>>);

#[derive(Debug, Default)]
struct State {
    last: Option<Measurement>,
    running: bool,
}

#[derive(Debug)]
struct Measurement {
    at: OffsetDateTime,
    taken: Instant,
    rows: Vec<Row>,
}

/// One row of the measurement: what was measured, where, and how large it is.
#[derive(Debug, Default, Serialize)]
struct Row {
    /// The tracked repository, or none for a shared row.
    repository: Option<String>,
    what: &'static str,
    path: Option<String>,
    /// The trees or entries the row sums.
    count: Option<usize>,
    bytes: Option<u64>,
    /// The owner and age of a `/var/tmp` entry.
    owner: Option<String>,
    age: Option<String>,
    /// `measured`, `incomplete: …`, `timed out after N s` or `failed: …`.
    state: String,
}

impl Cache {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts a measurement of `sources` on a thread of its own, unless one runs or the last is
    /// younger than [`Sources::measure_every`]. Returns at once.
    pub(super) fn refresh(&self, sources: &Sources) {
        let mut state = self.lock();
        let fresh = state
            .last
            .as_ref()
            .is_some_and(|last| last.taken.elapsed() < sources.measure_every);
        if state.running || fresh {
            return;
        }
        state.running = true;
        drop(state);
        let cache = self.clone();
        let sources = sources.clone();
        std::thread::spawn(move || {
            let rows = catch_unwind(AssertUnwindSafe(|| measure(&sources)));
            let mut state = cache.lock();
            state.running = false;
            if let Ok(rows) = rows {
                state.last = Some(Measurement {
                    at: OffsetDateTime::now_utc(),
                    taken: Instant::now(),
                    rows,
                });
            }
        });
    }

    /// The fields the `disk` object carries for the last complete measurement, at `now`.
    pub(super) fn view(&self, now: OffsetDateTime) -> Value {
        let state = self.lock();
        let last = state.last.as_ref();
        json!({
            "measured_at": last.map(|last| rfc3339(last.at)),
            "measured_ago": last.map(|last| ago(now - last.at)),
            "measuring": state.running,
            "total_bytes": last.map(|last| last.rows.iter().filter_map(|row| row.bytes).sum::<u64>()),
            "rows": last.map_or_else(|| json!([]), |last| json!(last.rows)),
        })
    }
}

/// Every row of one measurement of `sources`, largest first.
fn measure(sources: &Sources) -> Vec<Row> {
    let timeout = sources.measure_timeout;
    let home = &sources.home;
    let root = &sources.checkouts;
    let mut rows = Vec::new();

    let mut eligible: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    match json_of(&sources.gc, timeout) {
        Ok(gc) => {
            for assessment in gc["assessments"].as_array().into_iter().flatten() {
                let record = &assessment["record"];
                let (Some(repository), Some(tree)) = (
                    repository(&record["repository_root"], root),
                    record["path"].as_str(),
                ) else {
                    continue;
                };
                if assessment["eligible"] == true {
                    eligible
                        .entry(repository)
                        .or_default()
                        .push(PathBuf::from(tree));
                }
            }
        }
        Err(state) => rows.push(Row {
            what: ELIGIBLE,
            state,
            ..Row::default()
        }),
    }
    let removable: HashSet<&PathBuf> = eligible.values().flatten().collect();
    let mut cache: BTreeMap<String, (usize, u64)> = BTreeMap::new();
    match json_of(&sources.sweep, timeout) {
        Ok(sweep) => {
            for item in sweep["items"].as_array().into_iter().flatten() {
                let record = &item["record"];
                let Some(repository) = repository(&record["repository_root"], root) else {
                    continue;
                };
                // A tree eligible for removal is counted whole, with its cache, below.
                if record["path"]
                    .as_str()
                    .is_some_and(|tree| removable.contains(&PathBuf::from(tree)))
                {
                    continue;
                }
                let bytes: u64 = item["cache"]["discarded"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry["allocated_bytes"].as_u64())
                    .sum();
                if bytes > 0 {
                    let sum = cache.entry(repository).or_default();
                    *sum = (sum.0 + 1, sum.1 + bytes);
                }
            }
        }
        Err(state) => rows.push(Row {
            what: BUILD_CACHE,
            state,
            ..Row::default()
        }),
    }
    for (repository, (count, bytes)) in cache {
        rows.push(Row {
            repository: Some(repository),
            what: BUILD_CACHE,
            count: Some(count),
            bytes: Some(bytes),
            state: "measured".to_owned(),
            ..Row::default()
        });
    }
    for (repository, trees) in &eligible {
        let (bytes, state) = du(trees, timeout);
        rows.push(Row {
            repository: Some(repository.clone()),
            what: ELIGIBLE,
            count: Some(trees.len()),
            bytes,
            state,
            ..Row::default()
        });
    }

    let archives = home.join(".local/state/worktree/archives");
    for repository in checkouts(root) {
        for (what, path) in [
            ("archives", archives.join(&repository)),
            (
                "target/ of the checkout",
                root.join(&repository).join("target"),
            ),
        ] {
            if path.is_dir() {
                rows.push(path_row(Some(&repository), what, &path, timeout));
            }
        }
    }

    for entry in entries(&sources.targets) {
        rows.push(path_row(None, "shared build target", &entry, timeout));
    }
    for (what, path) in [
        ("sccache", ".cache/sccache"),
        ("rustup toolchains", ".rustup/toolchains"),
        ("agent scratch", ".cache/claude-tmp"),
    ] {
        let path = home.join(path);
        if path.is_dir() {
            rows.push(path_row(None, what, &path, timeout));
        }
    }
    var_tmp(&sources.var_tmp, timeout, &mut rows);

    rows.sort_by_key(|row| std::cmp::Reverse(row.bytes));
    rows
}

/// The name of the tracked repository whose root is `root`: a directory directly under the
/// checkouts root `checkouts`.
fn repository(root: &Value, checkouts: &Path) -> Option<String> {
    let root = Path::new(root.as_str()?);
    (root.parent() == Some(checkouts))
        .then(|| root.file_name())
        .flatten()
        .map(|name| name.to_string_lossy().into_owned())
}

/// The names of the checkouts directly under the checkouts root `root`, in order.
fn checkouts(root: &Path) -> BTreeSet<String> {
    entries(root)
        .into_iter()
        .filter(|dir| dir.join(".git").exists())
        .filter_map(|dir| Some(dir.file_name()?.to_string_lossy().into_owned()))
        .collect()
}

/// The entries of `dir`, in order; none when it cannot be read.
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .map(|entries| {
            entries
                .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                .collect()
        })
        .unwrap_or_default();
    entries.sort();
    entries
}

fn path_row(repository: Option<&str>, what: &'static str, path: &Path, timeout: Duration) -> Row {
    let (bytes, state) = du(&[path.to_path_buf()], timeout);
    Row {
        repository: repository.map(str::to_owned),
        what,
        path: Some(path.display().to_string()),
        bytes,
        state,
        ..Row::default()
    }
}

/// The entries of `dir`: one row each for those of at least [`VAR_TMP_ROW`] bytes, with owner
/// and age, and one row for the rest.
fn var_tmp(dir: &Path, timeout: Duration, rows: &mut Vec<Row>) {
    let entries = entries(dir);
    if entries.is_empty() {
        return;
    }
    let mut command: Vec<OsString> = ["du", "-s", "-B1", "--"].map(OsString::from).to_vec();
    command.extend(entries.iter().map(|entry| entry.clone().into_os_string()));
    let (stdout, stderr) = match run(&command, timeout) {
        Ran::Done { stdout, stderr, .. } => (stdout, stderr),
        Ran::TimedOut => {
            rows.push(Row {
                what: "/var/tmp entries",
                path: Some(dir.display().to_string()),
                state: timed_out(timeout),
                ..Row::default()
            });
            return;
        }
        Ran::Failed(error) => {
            rows.push(Row {
                what: "/var/tmp entries",
                path: Some(dir.display().to_string()),
                state: format!("failed: `du`: {error}"),
                ..Row::default()
            });
            return;
        }
    };
    let sizes: HashMap<&str, u64> = stdout
        .lines()
        .filter_map(|line| {
            let (bytes, path) = line.split_once('\t')?;
            Some((path, bytes.parse().ok()?))
        })
        .collect();
    // `du` names what it cannot read as `'<path>'`, one line each.
    let state = |entry: &str| {
        let unreadable = stderr.lines().any(|line| {
            line.contains(&format!("'{entry}/")) || line.contains(&format!("'{entry}'"))
        });
        if unreadable {
            "incomplete: parts are unreadable".to_owned()
        } else {
            "measured".to_owned()
        }
    };
    let owners = owners();
    let now = OffsetDateTime::now_utc();
    let mut small = Row {
        what: "smaller /var/tmp entries",
        path: Some(dir.display().to_string()),
        count: Some(0),
        bytes: Some(0),
        state: "measured".to_owned(),
        ..Row::default()
    };
    for entry in &entries {
        let shown = entry.display().to_string();
        let Some(&bytes) = sizes.get(shown.as_str()) else {
            continue;
        };
        if bytes < VAR_TMP_ROW {
            small.count = small.count.map(|count| count + 1);
            small.bytes = small.bytes.map(|sum| sum + bytes);
            if state(&shown) != "measured" {
                small.state = state(&shown);
            }
            continue;
        }
        let metadata = fs::symlink_metadata(entry).ok();
        rows.push(Row {
            what: "/var/tmp entry",
            owner: metadata.as_ref().map(|metadata| {
                owners
                    .get(&metadata.uid())
                    .cloned()
                    .unwrap_or_else(|| metadata.uid().to_string())
            }),
            age: metadata
                .and_then(|metadata| metadata.modified().ok())
                .map(|modified| ago(now - OffsetDateTime::from(modified))),
            state: state(&shown),
            path: Some(shown),
            bytes: Some(bytes),
            ..Row::default()
        });
    }
    if small.count > Some(0) {
        rows.push(small);
    }
}

/// User names by id, from `/etc/passwd`.
fn owners() -> HashMap<u32, String> {
    fs::read_to_string("/etc/passwd")
        .unwrap_or_default()
        .lines()
        .filter_map(|line| {
            let mut fields = line.split(':');
            let name = fields.next()?;
            let uid = fields.nth(1)?.parse().ok()?;
            Some((uid, name.to_owned()))
        })
        .collect()
}

/// The bytes `du` counts under `paths` together, and the row's state.
fn du(paths: &[PathBuf], timeout: Duration) -> (Option<u64>, String) {
    let mut command: Vec<OsString> = ["du", "-s", "-c", "-B1", "--"].map(OsString::from).to_vec();
    command.extend(paths.iter().map(|path| path.clone().into_os_string()));
    match run(&command, timeout) {
        Ran::Done {
            status,
            stdout,
            stderr,
        } => {
            let total = stdout
                .lines()
                .last()
                .and_then(|line| line.split_whitespace().next()?.parse().ok());
            let state = match (total, status.success()) {
                (Some(_), true) => "measured".to_owned(),
                (Some(_), false) => format!("incomplete: {}", first_line(&stderr)),
                (None, _) => format!("failed: `du`: {status}: {}", first_line(&stderr)),
            };
            (total, state)
        }
        Ran::TimedOut => (None, timed_out(timeout)),
        Ran::Failed(error) => (None, format!("failed: `du`: {error}")),
    }
}

/// The JSON `command` prints, or the row state that says why there is none.
fn json_of(command: &[OsString], timeout: Duration) -> Result<Value, String> {
    let shown = super::command_line(command);
    match run(command, timeout) {
        Ran::Done {
            status,
            stdout,
            stderr,
        } => {
            if !status.success() {
                return Err(format!(
                    "failed: `{shown}`: {status}: {}",
                    first_line(&stderr)
                ));
            }
            serde_json::from_str(&stdout)
                .map_err(|error| format!("failed: `{shown}` printed no JSON: {error}"))
        }
        Ran::TimedOut => Err(timed_out(timeout)),
        Ran::Failed(error) => Err(format!("failed: `{shown}`: {error}")),
    }
}

fn timed_out(timeout: Duration) -> String {
    format!("timed out after {} s", timeout.as_secs())
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default().trim()
}

/// How one command ended.
enum Ran {
    Done {
        status: ExitStatus,
        stdout: String,
        stderr: String,
    },
    /// It ran longer than its timeout and was killed.
    TimedOut,
    /// It could not be started or waited for.
    Failed(String),
}

/// Runs `command` with no input, for at most `timeout`.
fn run(command: &[OsString], timeout: Duration) -> Ran {
    let Some((program, arguments)) = command.split_first() else {
        return Ran::Failed("no command".to_owned());
    };
    let mut child = match Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => return Ran::Failed(error.to_string()),
    };
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(POLL),
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Ran::TimedOut;
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Ran::Failed(error.to_string());
            }
        }
    };
    // A process the command left behind may hold its output open; do not wait on it for long.
    let grace = deadline
        .saturating_duration_since(Instant::now())
        .max(Duration::from_secs(1));
    let text = |bytes: Vec<u8>| String::from_utf8_lossy(&bytes).into_owned();
    Ran::Done {
        status,
        stdout: text(stdout.recv_timeout(grace).unwrap_or_default()),
        stderr: text(stderr.recv_timeout(grace).unwrap_or_default()),
    }
}

/// Everything `pipe` yields up to its end, read on a thread of its own.
fn drain<R: Read + Send + 'static>(pipe: Option<R>) -> mpsc::Receiver<Vec<u8>> {
    let (sender, receiver) = mpsc::channel();
    if let Some(mut pipe) = pipe {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let _ = pipe.read_to_end(&mut bytes);
            let _ = sender.send(bytes);
        });
    }
    receiver
}

/// The shared build targets of the built-in instance under the home directory `home`:
/// `~/.cache/<namespace>-target`, `<namespace>` being the last step of the built-in managed trees'
/// directory, as the worktree tool names both. A config file's instance has `target` under its
/// cache instead ([`Sources::of`]).
pub(super) fn built_in_targets(home: &Path) -> PathBuf {
    let checkouts = collect::built_in_checkouts(home);
    let namespace = Path::new(&checkouts.trees)
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    home.join(".cache").join(format!("{namespace}-target"))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::super::Sources;
    use super::built_in_targets;
    use crate::config;

    #[test]
    fn a_config_files_instance_places_the_checkouts_and_the_shared_targets() {
        let mut instance = config::built_in(Path::new("/h"), Path::new("/h/work"));
        instance.checkouts.root = "/srv/src".to_owned();
        instance.cache = "/srv/cache/acme".to_owned();
        let sources = Sources::of(PathBuf::from("/h"), None, &instance);
        assert_eq!(sources.checkouts, PathBuf::from("/srv/src"));
        assert_eq!(sources.targets, PathBuf::from("/srv/cache/acme/target"));
    }

    #[test]
    fn without_a_file_the_places_are_the_built_in_ones_under_the_measured_home() {
        let sources = Sources::new(PathBuf::from("/h"), PathBuf::from("/h/work"));
        let built_in = config::built_in(Path::new("/h"), Path::new("/h"));
        assert_eq!(sources.checkouts, PathBuf::from(&built_in.checkouts.root));
        assert_eq!(sources.targets, built_in_targets(Path::new("/h")));
        assert_eq!(sources.targets.parent(), Some(Path::new("/h/.cache")));
        assert!(
            sources
                .targets
                .file_name()
                .is_some_and(|name| name.to_string_lossy().ends_with("-target")),
            "{sources:?}"
        );
    }
}
