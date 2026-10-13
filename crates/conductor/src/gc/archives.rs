//! The build output `conductor gc` would strip from the instance's worktree archives, measured
//! archive by archive within one bound (`story:gc-archive-measure`).
//!
//! An archive is a directory `<root>/<repository>/<directory>` holding a `manifest.json` that
//! `worktree` wrote, and it is the instance's when the manifest's `path` lies under the instance's
//! managed trees or its `repository_root` under the checkouts root ([`list`]). Each archive is
//! measured on its own by `worktree prune-archives --strip-build-output --json --repo <checkout>
//! --id <repository>/<directory>`, its own dry run, which writes nothing below the archive root,
//! and each result is handed on as soon as it is known ([`measure`]).
//!
//! A result is cached by the archive's id and modification time, the latest of its directory's
//! and its entries', in `gc/archive-sizes.json` under the instance's state directory ([`CACHE`]):
//! an archive whose modification time is the cached one is answered from the cache and not
//! walked. When [`bound`] is reached the walk stops, and what was measured is the answer, with
//! [`Strip::partial`] naming how many archives of how many.

use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What `conductor gc` calls the strip of build output from worktree archives.
pub(super) const WHAT: &str = "build output in worktree archives";

/// The cache of archive sizes, under the instance's state directory.
pub(super) const CACHE: &str = "gc/archive-sizes.json";

/// The variable that bounds, in whole seconds, how long one run walks archives; [`BOUND`] without
/// it.
pub(super) const BOUND_VARIABLE: &str = "CONDUCTOR_GC_ARCHIVE_BOUND";

/// How long one run walks archives by default.
const BOUND: Duration = Duration::from_secs(30 * 60);

/// How often the cache is written while archives are walked, so a run that is stopped keeps what
/// it measured.
const SAVE_EVERY: Duration = Duration::from_secs(2);

/// The cache's format.
const VERSION: u32 = 1;

/// The bound on walking archives: [`BOUND_VARIABLE`] when it is set, else [`BOUND`].
///
/// # Errors
///
/// The variable is set and is not a whole number of seconds.
pub(super) fn bound() -> Result<Duration> {
    let Some(value) = std::env::var_os(BOUND_VARIABLE) else {
        return Ok(BOUND);
    };
    match value
        .to_str()
        .and_then(|text| text.trim().parse::<u64>().ok())
    {
        Some(seconds) => Ok(Duration::from_secs(seconds)),
        None => bail!("{BOUND_VARIABLE} {value:?} is not a whole number of seconds"),
    }
}

/// One of the instance's archives.
pub(super) struct Archive {
    /// `<repository>/<directory>`, as `worktree prune-archives --id` names it.
    id: String,
    path: PathBuf,
    /// The latest modification time of the directory and its entries, in nanoseconds since the
    /// epoch.
    modified: u64,
}

/// One archive's result.
pub(super) struct Measured {
    pub(super) id: String,
    pub(super) path: PathBuf,
    /// `worktree`'s verdict: `strippable`, `nothing-to-strip`, …, or `skipped` when its answer
    /// did not assess the archive.
    pub(super) verdict: String,
    /// The bytes a strip would remove.
    pub(super) strip_bytes: u64,
    /// Whether the cache answered, and the archive was not walked.
    pub(super) cached: bool,
}

/// What one run measured.
pub(super) struct Strip {
    /// The archives measured, cached ones first, then those walked, in id order.
    pub(super) measured: Vec<Measured>,
    /// How many archives the instance has.
    pub(super) total: usize,
    /// Why some were not measured: the bound, or the first failure.
    unmeasured: Option<String>,
    /// Why the cache could not be written, if it could not.
    pub(super) unsaved: Option<String>,
}

impl Strip {
    /// `partial: <n> of <m> archives` and why, when some archive was not measured.
    pub(super) fn partial(&self) -> Option<String> {
        (self.measured.len() < self.total).then(|| {
            let mut text = format!(
                "partial: {} of {} archives",
                self.measured.len(),
                self.total
            );
            if let Some(why) = &self.unmeasured {
                text.push_str("; ");
                text.push_str(why);
            }
            text
        })
    }
}

/// The instance's archives under `root`, in id order: those whose manifest's `path` lies under
/// `trees` or whose `repository_root` lies under `checkouts`.
pub(super) fn list(root: &Path, trees: &Path, checkouts: &Path) -> Vec<Archive> {
    let mut archives = Vec::new();
    for repository in directories(root) {
        for path in directories(&repository) {
            if !ours(&path, trees, checkouts) {
                continue;
            }
            let name = |path: &Path| {
                path.file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            };
            let (Some(repository), Some(directory)) = (name(&repository), name(&path)) else {
                continue;
            };
            archives.push(Archive {
                id: format!("{repository}/{directory}"),
                modified: modified(&path),
                path,
            });
        }
    }
    archives
}

/// The directories directly under `dir`, links not followed, in name order.
fn directories(dir: &Path) -> Vec<PathBuf> {
    let mut found: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect();
    found.sort();
    found
}

/// Whether the archive at `path` is the instance's, by its manifest.
fn ours(path: &Path, trees: &Path, checkouts: &Path) -> bool {
    let Ok(text) = fs::read_to_string(path.join("manifest.json")) else {
        return false;
    };
    let Ok(manifest) = serde_json::from_str::<Value>(&text) else {
        return false;
    };
    let under = |key: &str, root: &Path| {
        manifest[key]
            .as_str()
            .is_some_and(|named| Path::new(named).starts_with(root))
    };
    under("path", trees) || under("repository_root", checkouts)
}

/// The latest modification time of `dir` and its entries, in nanoseconds since the epoch; 0 when
/// none can be read.
fn modified(dir: &Path) -> u64 {
    let stamp = |path: &Path| {
        fs::symlink_metadata(path)
            .and_then(|metadata| metadata.modified())
            .ok()
            .and_then(|time| time.duration_since(SystemTime::UNIX_EPOCH).ok())
            .map_or(0, |since| {
                u64::try_from(since.as_nanos()).unwrap_or(u64::MAX)
            })
    };
    let entries: Vec<u64> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| stamp(&entry.path()))
        .collect();
    entries.into_iter().fold(stamp(dir), u64::max)
}

/// One cached result.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Size {
    modified: u64,
    verdict: String,
    strip_bytes: u64,
}

/// The cache file.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Sizes {
    version: u32,
    archives: BTreeMap<String, Size>,
}

/// The cached results in `file`; none when it is missing, unreadable or of another version.
fn load(file: &Path) -> BTreeMap<String, Size> {
    fs::read_to_string(file)
        .ok()
        .and_then(|text| serde_json::from_str::<Sizes>(&text).ok())
        .filter(|sizes| sizes.version == VERSION)
        .map(|sizes| sizes.archives)
        .unwrap_or_default()
}

/// Writes `archives` to `file`, through a file beside it renamed over it.
fn save(file: &Path, archives: &BTreeMap<String, Size>) -> Result<(), String> {
    let sizes = Sizes {
        version: VERSION,
        archives: archives.clone(),
    };
    let text = serde_json::to_string(&sizes).map_err(|error| error.to_string())?;
    let written = file.with_extension("json.new");
    let shown = |error: std::io::Error| format!("{}: {error}", file.display());
    if let Some(dir) = file.parent() {
        fs::create_dir_all(dir).map_err(shown)?;
    }
    fs::write(&written, text).map_err(shown)?;
    fs::rename(&written, file).map_err(shown)
}

/// Measures `archives` through `worktree` with `--repo checkout`, answering each from the cache
/// `cache` when its modification time is the cached one, and walking the others in order until
/// `bound` is reached. Each result is handed to `shown` as soon as it is known. The cache keeps
/// what was walked, and forgets archives that are gone.
pub(super) fn measure(
    checkout: &Path,
    archives: &[Archive],
    cache: &Path,
    bound: Duration,
    shown: &mut dyn FnMut(&Measured),
) -> Strip {
    let deadline = Instant::now() + bound;
    let mut sizes = load(cache);
    let mut dirty = false;
    let mut measured = Vec::new();
    let mut pending = Vec::new();
    for archive in archives {
        match sizes.get(&archive.id) {
            Some(size) if size.modified == archive.modified => {
                let found = Measured {
                    id: archive.id.clone(),
                    path: archive.path.clone(),
                    verdict: size.verdict.clone(),
                    strip_bytes: size.strip_bytes,
                    cached: true,
                };
                shown(&found);
                measured.push(found);
            }
            _ => pending.push(archive),
        }
    }
    let mut unmeasured = None;
    let mut unsaved = None;
    let mut saved = Instant::now();
    let out_of_time = || format!("the rest not measured within {} s", bound.as_secs());
    for archive in pending {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            unmeasured = Some(out_of_time());
            break;
        }
        match walk(checkout, archive, left) {
            Ok((verdict, strip_bytes)) => {
                sizes.insert(
                    archive.id.clone(),
                    Size {
                        modified: archive.modified,
                        verdict: verdict.clone(),
                        strip_bytes,
                    },
                );
                dirty = true;
                let found = Measured {
                    id: archive.id.clone(),
                    path: archive.path.clone(),
                    verdict,
                    strip_bytes,
                    cached: false,
                };
                shown(&found);
                measured.push(found);
                if saved.elapsed() >= SAVE_EVERY {
                    unsaved = save(cache, &sizes).err();
                    saved = Instant::now();
                    dirty = false;
                }
            }
            Err(Walk::TimedOut) => {
                unmeasured = Some(out_of_time());
                break;
            }
            Err(Walk::Failed(error)) => {
                unmeasured.get_or_insert_with(|| format!("failed: {error}"));
            }
        }
    }
    let present: HashSet<&str> = archives.iter().map(|archive| archive.id.as_str()).collect();
    let before = sizes.len();
    sizes.retain(|id, _| present.contains(id.as_str()));
    if dirty || sizes.len() != before {
        unsaved = save(cache, &sizes).err();
    }
    Strip {
        measured,
        total: archives.len(),
        unmeasured,
        unsaved,
    }
}

/// Why an archive was not measured.
enum Walk {
    /// Its listing ran until the bound and was stopped.
    TimedOut,
    Failed(String),
}

/// `worktree`'s verdict on `archive` and the bytes a strip would remove, from its own dry run,
/// stopped after `timeout`.
fn walk(checkout: &Path, archive: &Archive, timeout: Duration) -> Result<(String, u64), Walk> {
    let mut command: Vec<OsString> = [
        "worktree",
        "prune-archives",
        "--strip-build-output",
        "--json",
        "--repo",
    ]
    .map(OsString::from)
    .to_vec();
    command.push(checkout.as_os_str().to_owned());
    command.push(OsString::from("--id"));
    command.push(OsString::from(&archive.id));
    let shown = command
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let stdout = run(&command, timeout)?;
    let report: Value = serde_json::from_str(&stdout)
        .map_err(|error| Walk::Failed(format!("`{shown}` printed no JSON: {error}")))?;
    if report["ok"] == false {
        return Err(Walk::Failed(format!("`{shown}` answered \"ok\": false")));
    }
    let items = report["archives"].as_array().map_or(&[][..], Vec::as_slice);
    let directory = archive
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned());
    let item = items
        .iter()
        .find(|item| {
            item["path"]
                .as_str()
                .is_some_and(|path| Path::new(path) == archive.path)
        })
        .or_else(|| {
            items
                .iter()
                .find(|item| item["archive_directory"].as_str() == directory.as_deref())
        });
    Ok(match item {
        Some(item) => (
            item["verdict"].as_str().unwrap_or("unknown").to_owned(),
            item["strip_bytes"].as_u64().unwrap_or(0),
        ),
        None => ("skipped".to_owned(), 0),
    })
}

/// What `command` printed on standard output when it exited 0 within `timeout`; it is stopped
/// at `timeout`.
fn run(command: &[OsString], timeout: Duration) -> Result<String, Walk> {
    let shown = command
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let Some((program, arguments)) = command.split_first() else {
        return Err(Walk::Failed("no command".to_owned()));
    };
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| Walk::Failed(format!("`{shown}`: {error}")))?;
    let stdout = drain(child.stdout.take());
    let stderr = drain(child.stderr.take());
    let deadline = Instant::now() + timeout;
    let mut poll = Duration::from_millis(1);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => {
                std::thread::sleep(poll);
                poll = (poll * 2).min(Duration::from_millis(20));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Walk::TimedOut);
            }
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(Walk::Failed(format!("`{shown}`: {error}")));
            }
        }
    };
    // A process the command left behind may hold its output open: wait for it a little only.
    let grace = Duration::from_secs(5);
    let stdout = stdout.recv_timeout(grace).unwrap_or_default();
    if !status.success() {
        let stderr = stderr.recv_timeout(grace).unwrap_or_default();
        return Err(Walk::Failed(format!(
            "`{shown}`: {status}: {}",
            stderr.lines().next().unwrap_or_default().trim()
        )));
    }
    Ok(stdout)
}

/// Reads `pipe` to its end on a thread of its own, and sends what it read.
fn drain(pipe: Option<impl Read + Send + 'static>) -> mpsc::Receiver<String> {
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut text = String::new();
        if let Some(mut pipe) = pipe {
            let _ = pipe.read_to_string(&mut text);
        }
        let _ = sender.send(text);
    });
    receiver
}
