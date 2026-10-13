//! The disk waste across the tracked repositories (`story:disk-waste`), measured on a thread of
//! its own and shown with its age.
//!
//! A tracked repository is a checkout directly under the instance's checkouts root. Per
//! repository the measurement reads the recognised build cache of its idle managed trees
//! (`worktree sweep --dry-run`), its trees eligible for removal (`worktree gc --dry-run`, then a
//! walk of each), its archives under `~/.local/state/worktree/archives/` and its checkout's
//! `target/`.
//! Shared, it reads each entry of the shared build targets (`target` under the instance's cache
//! when a config file names it, else `~/.cache/<namespace>-target`: [`built_in_targets`]), sccache, the
//! rustup toolchains, the Go build cache `~/.cache/go-build`, the agents' scratch
//! `~/.cache/claude-tmp` and the entries of `/var/tmp` ([`shared_rows`]: the holders `conductor gc`
//! names and never reclaims). It deletes nothing.
//!
//! Each row says whether `conductor gc` removes it (`story:conductor-gc`), and how, or why it is
//! kept; [`repositories`] is the one classification both read. Only three kinds of the panel's row
//! are ever reclaimable, each removed by the tool that owns it, never by a deletion of conductor's
//! own (`conductor gc` adds a fourth, the build output in worktree archives,
//! [`Reclaim::StripArchives`], which the panel does not measure):
//! - the build cache of idle managed trees: `worktree discard-cache` on each tree, which deletes
//!   only the cache it recognises and refuses a tree with a live lease or a process in it;
//! - the trees eligible for removal: `worktree gc --apply --id` on each, which revalidates each
//!   tree's recovery proof before it removes it;
//! - a checkout's `target/` that is cargo's (it holds `CACHEDIR.TAG` and is no link, beside the
//!   checkout's `Cargo.toml`): `cargo clean --target-dir`.
//!
//! A tree or checkout a live session works in, its working directory there or below
//! ([`works_in`]), is kept, and so is a checkout's `target/` while any process on the host has
//! its working directory in the checkout ([`process_in`], `/proc/<pid>/cwd`), and a tree outside
//! the instance's managed trees; when the live
//! sessions cannot be read ([`live`]), every row is kept. Archives (`worktree prune-archives`
//! reviews each), the shared build targets, sccache, the toolchains, the agents' scratch and
//! `/var/tmp` are never reclaimable: something outside conductor's view may use each.
//!
//! Every size is the allocated bytes of a walk in Rust, [`disk::tree_size`], not `du`, whose
//! options differ between platforms (`story:portable-tree-sizes`).
//!
//! Every command and every walk runs under [`Sources::measure_timeout`]; one that runs longer is
//! stopped and its row says so. A request never waits for a measurement: it shows the last complete one and
//! starts the next when that one is older than [`Sources::measure_every`].

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Read};
use std::os::unix::fs::MetadataExt as _;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError, mpsc};
use std::time::{Duration, Instant, SystemTime};

use serde::Serialize;
use serde_json::{Value, json};
use time::OffsetDateTime;

use super::{Sources, ago, rfc3339};
use crate::collect;
use crate::disk::{self, TreeSize};

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

/// One row of the measurement: what was measured, where, and how large it is, and whether
/// `conductor gc` removes it.
#[derive(Debug, Default, Serialize)]
pub(crate) struct Row {
    /// The tracked repository, or none for a shared row.
    pub(crate) repository: Option<String>,
    pub(crate) what: &'static str,
    pub(crate) path: Option<String>,
    /// The trees or entries the row sums.
    pub(crate) count: Option<usize>,
    pub(crate) bytes: Option<u64>,
    /// The owner and age of a `/var/tmp` entry.
    owner: Option<String>,
    age: Option<String>,
    /// `measured`, `incomplete: …`, `timed out after N s` or `failed: …`.
    pub(crate) state: String,
    /// Whether `conductor gc` removes what the row measures: exactly when it has a [`Reclaim`].
    reclaimable: bool,
    /// The command `conductor gc` removes it with, or `kept: <why>`.
    pub(crate) gc: String,
    /// How `conductor gc` removes it; none for a row it keeps.
    #[serde(skip)]
    pub(crate) reclaim: Option<Reclaim>,
}

impl Row {
    /// The row, which `conductor gc` removes through `reclaim` when it was measured, and keeps
    /// otherwise: a row without bytes is not known to be what the panel shows.
    fn reclaimed_by(self, reclaim: Reclaim) -> Self {
        if self.bytes.is_none() {
            return self.kept("not measured");
        }
        Self {
            reclaimable: true,
            gc: reclaim.shown(),
            reclaim: Some(reclaim),
            ..self
        }
    }

    /// The row, which `conductor gc` keeps, because of `why`.
    fn kept(self, why: &str) -> Self {
        Self {
            reclaimable: false,
            gc: format!("kept: {why}"),
            reclaim: None,
            ..self
        }
    }
}

/// How `conductor gc` removes a reclaimable row: through the tool that owns what it measures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Reclaim {
    /// `worktree discard-cache --json <tree>` on each tree: the build cache `worktree` recognises,
    /// in trees it finds idle.
    DiscardCache(Vec<PathBuf>),
    /// `worktree gc --repo <root> --apply --json --id <tree>…`: the trees of the repository at
    /// `root`, each revalidated by `worktree` before it is removed.
    RemoveTrees { root: PathBuf, trees: Vec<PathBuf> },
    /// `cargo clean --manifest-path <checkout>/Cargo.toml --target-dir <target>`.
    CargoClean { checkout: PathBuf, target: PathBuf },
    /// `worktree prune-archives --strip-build-output --apply --json --repo <checkout> --id
    /// <repository>/<archive>…`: the build output in the archives `worktree` listed strippable,
    /// each re-assessed by `worktree` before it rewrites the archive's patch. `checkout` is the
    /// git checkout whose `worktree` profile the listing read ([`profile_checkout`]). No archive
    /// is deleted.
    StripArchives {
        checkout: PathBuf,
        archives: Vec<PathBuf>,
    },
}

impl Reclaim {
    /// The paths the row measures, whose bytes its removal frees.
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        match self {
            Self::DiscardCache(trees) | Self::RemoveTrees { trees, .. } => trees.clone(),
            Self::CargoClean { target, .. } => vec![target.clone()],
            Self::StripArchives { archives, .. } => archives.clone(),
        }
    }

    /// The paths that must be gone once the removal succeeded: the trees `worktree gc` removes
    /// and the target `cargo clean` removes; none for a cache, whose tree stays, or for an
    /// archive, which a strip rewrites and keeps.
    pub(crate) fn removed(&self) -> Vec<PathBuf> {
        match self {
            Self::DiscardCache(_) | Self::StripArchives { .. } => Vec::new(),
            Self::RemoveTrees { trees, .. } => trees.clone(),
            Self::CargoClean { target, .. } => vec![target.clone()],
        }
    }

    /// The commands that remove it, in order, each with the directory it runs in, if any.
    pub(crate) fn commands(&self) -> Vec<(Vec<OsString>, Option<PathBuf>)> {
        let os = |text: &str| OsString::from(text);
        match self {
            Self::DiscardCache(trees) => trees
                .iter()
                .map(|tree| {
                    let command = vec![
                        os("worktree"),
                        os("discard-cache"),
                        os("--json"),
                        tree.clone().into_os_string(),
                    ];
                    (command, None)
                })
                .collect(),
            Self::RemoveTrees { root, trees } => {
                let mut command = vec![
                    os("worktree"),
                    os("gc"),
                    os("--repo"),
                    root.clone().into_os_string(),
                    os("--apply"),
                    os("--json"),
                ];
                for tree in trees {
                    command.push(os("--id"));
                    command.push(tree.clone().into_os_string());
                }
                vec![(command, None)]
            }
            Self::CargoClean { checkout, target } => {
                let command = vec![
                    os("cargo"),
                    os("clean"),
                    os("--manifest-path"),
                    checkout.join("Cargo.toml").into_os_string(),
                    os("--target-dir"),
                    target.clone().into_os_string(),
                ];
                vec![(command, Some(checkout.clone()))]
            }
            Self::StripArchives { checkout, archives } => {
                let mut command = vec![
                    os("worktree"),
                    os("prune-archives"),
                    os("--strip-build-output"),
                    os("--apply"),
                    os("--json"),
                    os("--repo"),
                    checkout.clone().into_os_string(),
                ];
                for archive in archives {
                    command.push(os("--id"));
                    command.push(OsString::from(archive_id(archive)));
                }
                vec![(command, None)]
            }
        }
    }

    /// The command as the panel names it.
    /// Why the row must be kept now, read again just before it is removed, against the working
    /// directories `live`: a live session works in one of its trees; or, for a checkout's
    /// `target/`, any check the listing made fails again ([`cargo_target`]). None when it may go;
    /// always for archives, which no session works in and `worktree` re-assesses itself.
    pub(crate) fn recheck(&self, live: &[PathBuf]) -> Option<String> {
        match self {
            Self::DiscardCache(trees) | Self::RemoveTrees { trees, .. } => trees
                .iter()
                .find(|tree| works_in(live, tree))
                .map(|tree| format!("a live session now works in {}", tree.display())),
            Self::CargoClean { checkout, target } => cargo_target(checkout, target, live).err(),
            Self::StripArchives { .. } => None,
        }
    }

    fn shown(&self) -> String {
        match self {
            Self::DiscardCache(_) => "worktree discard-cache",
            Self::RemoveTrees { .. } => "worktree gc --apply",
            Self::CargoClean { .. } => "cargo clean --target-dir",
            Self::StripArchives { .. } => "worktree prune-archives --strip-build-output --apply",
        }
        .to_owned()
    }
}

/// The archive at `path` as `worktree prune-archives --id` names it: `<repository>/<directory>`,
/// the directory and the one above it under the archive root; the directory alone when `path`
/// has no parent name.
fn archive_id(path: &Path) -> String {
    let name = |path: &Path| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
    };
    match (path.parent().and_then(name), name(path)) {
        (Some(repository), Some(directory)) => format!("{repository}/{directory}"),
        (None, Some(directory)) => directory,
        (_, None) => path.display().to_string(),
    }
}

/// The checkout `worktree prune-archives --repo` is given: the first git checkout directly
/// under the checkouts root `root`, in name order; none when there is none.
pub(crate) fn profile_checkout(root: &Path) -> Option<PathBuf> {
    checkouts(root)
        .into_iter()
        .next()
        .map(|name| root.join(name))
}

/// A tracked repository, and why `conductor gc` keeps the trees grouped under it: none when it
/// may remove them.
type Group = (String, Option<String>);

/// One measurement's rows, and the working directories of the live sessions it kept clear of, or
/// why they could not be read.
pub(crate) struct Waste {
    pub(crate) rows: Vec<Row>,
    pub(crate) live: Result<Vec<PathBuf>, String>,
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
            "reclaimable_bytes": last.map(|last| {
                last.rows
                    .iter()
                    .filter(|row| row.reclaimable)
                    .filter_map(|row| row.bytes)
                    .sum::<u64>()
            }),
            "rows": last.map_or_else(|| json!([]), |last| json!(last.rows)),
        })
    }
}

/// Every row of one measurement of `sources`, largest first: the rows of the tracked repositories
/// ([`repositories`]), then the shared ones, which `conductor gc` never removes.
fn measure(sources: &Sources) -> Vec<Row> {
    let mut rows = repositories(sources).rows;
    shared(sources, &mut rows);
    rows.sort_by_key(|row| std::cmp::Reverse(row.bytes));
    rows
}

/// The rows of the tracked repositories, largest first, each marked reclaimable or kept: the one
/// classification the panel and `conductor gc` read (`story:conductor-gc`).
pub(crate) fn repositories(sources: &Sources) -> Waste {
    let timeout = sources.measure_timeout;
    let home = &sources.home;
    let root = &sources.checkouts;
    let live = live(sources);
    // Why a managed tree is kept, or none when `conductor gc` may hand it to `worktree`.
    let keep = |tree: &Path| -> Option<String> {
        match &live {
            Err(error) => Some(format!("the live sessions could not be read: {error}")),
            Ok(live) if works_in(live, tree) => Some("a live session works in it".to_owned()),
            Ok(_)
                if !tree.is_absolute()
                    || tree.components().any(|part| part == Component::ParentDir) =>
            {
                Some("not a plain absolute path".to_owned())
            }
            Ok(_) if !tree.starts_with(&sources.trees) => {
                Some("outside the instance's managed trees".to_owned())
            }
            Ok(_) => None,
        }
    };
    let mut rows = Vec::new();

    // Per repository and per reason kept (none: reclaimable), the trees.
    let mut eligible: BTreeMap<Group, Vec<PathBuf>> = BTreeMap::new();
    let mut removable: HashSet<PathBuf> = HashSet::new();
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
                    let tree = PathBuf::from(tree);
                    removable.insert(tree.clone());
                    eligible
                        .entry((repository, keep(&tree)))
                        .or_default()
                        .push(tree);
                }
            }
        }
        Err(state) => rows.push(
            Row {
                what: ELIGIBLE,
                state,
                ..Row::default()
            }
            .kept("not measured"),
        ),
    }
    let mut cache: BTreeMap<Group, (Vec<PathBuf>, u64)> = BTreeMap::new();
    match json_of(&sources.sweep, timeout) {
        Ok(sweep) => {
            for item in sweep["items"].as_array().into_iter().flatten() {
                let record = &item["record"];
                let (Some(repository), Some(tree)) = (
                    repository(&record["repository_root"], root),
                    record["path"].as_str(),
                ) else {
                    continue;
                };
                let tree = PathBuf::from(tree);
                // A tree eligible for removal is counted whole, with its cache, below.
                if removable.contains(&tree) {
                    continue;
                }
                let bytes: u64 = item["cache"]["discarded"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|entry| entry["allocated_bytes"].as_u64())
                    .sum();
                if bytes > 0 {
                    let sum = cache.entry((repository, keep(&tree))).or_default();
                    sum.0.push(tree);
                    sum.1 += bytes;
                }
            }
        }
        Err(state) => rows.push(
            Row {
                what: BUILD_CACHE,
                state,
                ..Row::default()
            }
            .kept("not measured"),
        ),
    }
    for ((repository, why), (trees, bytes)) in cache {
        let row = Row {
            repository: Some(repository),
            what: BUILD_CACHE,
            count: Some(trees.len()),
            bytes: Some(bytes),
            state: "measured".to_owned(),
            ..Row::default()
        };
        rows.push(match why {
            Some(why) => row.kept(&why),
            None => row.reclaimed_by(Reclaim::DiscardCache(trees)),
        });
    }
    for ((repository, why), trees) in eligible {
        let (bytes, state) = size(&trees, timeout);
        let row = Row {
            repository: Some(repository.clone()),
            what: ELIGIBLE,
            count: Some(trees.len()),
            bytes,
            state,
            ..Row::default()
        };
        rows.push(match why {
            Some(why) => row.kept(&why),
            None => row.reclaimed_by(Reclaim::RemoveTrees {
                root: root.join(&repository),
                trees,
            }),
        });
    }

    let archives = home.join(".local/state/worktree/archives");
    for repository in checkouts(root) {
        let path = archives.join(&repository);
        if path.is_dir() {
            rows.push(
                path_row(Some(&repository), "archives", &path, timeout)
                    .kept("`worktree prune-archives` reviews each archive"),
            );
        }
        let checkout = root.join(&repository);
        let target = checkout.join("target");
        if target.is_dir() {
            let row = path_row(
                Some(&repository),
                "target/ of the checkout",
                &target,
                timeout,
            );
            let checked = match &live {
                Err(error) => Err(format!("the live sessions could not be read: {error}")),
                Ok(live) => cargo_target(&checkout, &target, live),
            };
            rows.push(match checked {
                Ok(()) => row.reclaimed_by(Reclaim::CargoClean { checkout, target }),
                Err(why) => row.kept(&why),
            });
        }
    }

    rows.sort_by_key(|row| std::cmp::Reverse(row.bytes));
    Waste { rows, live }
}

/// The shared rows, none of which `conductor gc` removes, which it names as the holders it does
/// not reclaim (`story:gc-before-pause`).
pub(crate) fn shared_rows(sources: &Sources) -> Vec<Row> {
    let mut rows = Vec::new();
    shared(sources, &mut rows);
    rows
}

/// The shared rows, none of which `conductor gc` removes: something outside conductor's view may
/// use each.
fn shared(sources: &Sources, rows: &mut Vec<Row>) {
    let timeout = sources.measure_timeout;
    for entry in entries(&sources.targets) {
        rows.push(
            path_row(None, "shared build target", &entry, timeout)
                .kept("shared: no session is known to be clear of it"),
        );
    }
    for (what, path, why) in [
        ("sccache", ".cache/sccache", "shared compiler cache"),
        (
            "rustup toolchains",
            ".rustup/toolchains",
            "the toolchains builds run",
        ),
        ("Go build cache", ".cache/go-build", "shared Go build cache"),
        (
            "agent scratch",
            ".cache/claude-tmp",
            "the sessions' own scratch",
        ),
    ] {
        let path = sources.home.join(path);
        if path.is_dir() {
            rows.push(path_row(None, what, &path, timeout).kept(why));
        }
    }
    let start = rows.len();
    var_tmp(&sources.var_tmp, timeout, rows);
    for row in &mut rows[start..] {
        *row = std::mem::take(row).kept("not conductor's");
    }
}

/// Whether `conductor gc` may `cargo clean` the checkout `checkout`'s target `target`, or why
/// not: none of the live sessions' working directories `live` lies in the checkout, no process on
/// the host works in it ([`process_in`]), and the target is cargo's, a directory that holds
/// `CACHEDIR.TAG` and is no link, beside the checkout's `Cargo.toml`. The listing checks it, and
/// [`Reclaim::recheck`] again just before `cargo clean`.
fn cargo_target(checkout: &Path, target: &Path, live: &[PathBuf]) -> Result<(), String> {
    if works_in(live, checkout) {
        return Err("a live session works in the checkout".to_owned());
    }
    if let Some(pid) = process_in(checkout)? {
        return Err(format!("process {pid} works in the checkout"));
    }
    let is = |path: &Path, file: bool| {
        fs::symlink_metadata(path).is_ok_and(|metadata| {
            if file {
                metadata.is_file()
            } else {
                metadata.is_dir()
            }
        })
    };
    if !is(target, false) {
        return Err("not a directory: a link, or gone".to_owned());
    }
    if !is(&target.join("CACHEDIR.TAG"), true) {
        return Err("not cargo's: no CACHEDIR.TAG".to_owned());
    }
    if !is(&checkout.join("Cargo.toml"), true) {
        return Err("no Cargo.toml beside it".to_owned());
    }
    Ok(())
}

/// The first process on the host whose working directory is `path` or lies under it, read from
/// the `/proc/<pid>/cwd` links ([`works_in`]); an entry that cannot be read (another user's, or a
/// process that has ended) is skipped. When `/proc` cannot be listed, why: no process can then be
/// told to be clear of `path`.
fn process_in(path: &Path) -> Result<Option<u32>, String> {
    let entries = fs::read_dir("/proc").map_err(|error| format!("cannot list /proc: {error}"))?;
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let Ok(cwd) = fs::read_link(entry.path().join("cwd")) else {
            continue;
        };
        if works_in(std::slice::from_ref(&cwd), path) {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

/// The working directories of the live sessions, Claude's from the session list `sources.agents`
/// prints and Codex's under `~/.codex/sessions`, as the sessions collector reads them; or why they
/// could not be read. A working directory that is no absolute path cannot be placed, so it is
/// such a reason too.
pub(crate) fn live(sources: &Sources) -> Result<Vec<PathBuf>, String> {
    let sessions = collect::sessions::Sources {
        agents: sources.agents.clone(),
        codex: sources.home.join(".codex/sessions"),
        home: sources.home.clone(),
        now: SystemTime::now(),
        bound: sources.measure_timeout,
    };
    let live = collect::sessions::live_working_directories(&sessions)
        .map_err(|error| first_line(&format!("{error:#}")).to_owned())?;
    match live.iter().find(|cwd| !cwd.is_absolute()) {
        Some(cwd) => Err(format!(
            "a live session works in {}, which is no absolute path",
            cwd.display()
        )),
        None => Ok(live),
    }
}

/// Whether one of the working directories `live` is `path` or lies under it, each read both as
/// written and with its links resolved.
pub(crate) fn works_in(live: &[PathBuf], path: &Path) -> bool {
    let paths: Vec<PathBuf> = [Some(path.to_path_buf()), fs::canonicalize(path).ok()]
        .into_iter()
        .flatten()
        .collect();
    live.iter().any(|cwd| {
        [Some(cwd.clone()), fs::canonicalize(cwd).ok()]
            .into_iter()
            .flatten()
            .any(|cwd| paths.iter().any(|path| cwd.starts_with(path)))
    })
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
    let (bytes, state) = size(&[path.to_path_buf()], timeout);
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
    let deadline = Instant::now() + timeout;
    let mut sizes = Vec::with_capacity(entries.len());
    for entry in &entries {
        match disk::tree_size(std::slice::from_ref(entry), Some(deadline)) {
            Ok(size) => sizes.push(size),
            Err(disk::TimedOut) => {
                rows.push(Row {
                    what: "/var/tmp entries",
                    path: Some(dir.display().to_string()),
                    state: timed_out(timeout),
                    ..Row::default()
                });
                return;
            }
        }
    }
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
    for (entry, size) in entries.iter().zip(&sizes) {
        // An entry removed since the listing is not a row.
        let removed = size.unreadable.first().is_some_and(|unreadable| {
            unreadable.path == *entry && unreadable.error.kind() == io::ErrorKind::NotFound
        });
        if removed {
            continue;
        }
        let (bytes, state) = (size.allocated, state(size));
        if bytes < VAR_TMP_ROW {
            small.count = small.count.map(|count| count + 1);
            small.bytes = small.bytes.map(|sum| sum + bytes);
            if state != "measured" {
                small.state = state;
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
            state,
            path: Some(entry.display().to_string()),
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

/// The bytes allocated under `paths` together ([`disk::tree_size`]), and the row's state.
fn size(paths: &[PathBuf], timeout: Duration) -> (Option<u64>, String) {
    match disk::tree_size(paths, Some(Instant::now() + timeout)) {
        Ok(size) => (Some(size.allocated), state(&size)),
        Err(disk::TimedOut) => (None, timed_out(timeout)),
    }
}

/// `measured`, or `incomplete: …` naming the first entry `size` could not read.
fn state(size: &TreeSize) -> String {
    match size.unreadable.first() {
        None => "measured".to_owned(),
        Some(unreadable) => format!(
            "incomplete: cannot read {}: {}",
            unreadable.path.display(),
            unreadable.error
        ),
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
