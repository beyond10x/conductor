//! `conductor host reclaim` (`story:disk-reclaim`): frees what needs no decision until free disk
//! on `/` is back over a floor, records the run, and prints each step with the bytes it freed.
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `gc-reclaim` and its input `conductor.host.ReclaimDisk`; its floor is the
//! instance's `thresholds.disk_admit` plus [`MARGIN`] ([`floor_bytes`]). The watch runs the same
//! [`reclaim`] once per fall under `disk_admit` (`crate::watch`), and `conductor build run` runs
//! it when a reservation does not fit.
//!
//! [`reclaim`] reads free disk on `/` ([`free_bytes`]) and, while it is under the bytes asked
//! for, applies in order the reclaimable rows of `conductor gc` that need no decision, one tree
//! or one archive at a time, reading free disk again after each step and stopping as soon as it
//! is enough:
//! 1. the build output of each idle managed tree, through `worktree discard-cache`, which deletes
//!    only the ignored entries it recognises; never a tree itself. A tree is idle when the
//!    disk panel's classification keeps no live session in it ([`waste::repositories`]), no
//!    process on the host works in it (`/proc/<pid>/cwd`: a running gate, a `conductor build
//!    run` whose process group is alive, or anything else), and nothing under its `target/` was
//!    written in the last [`RECENT`] (a gate between two compiler runs). The live sessions are
//!    read again just before each tree, as `conductor gc` does;
//! 2. the build output in the instance's worktree archives that `worktree prune-archives
//!    --strip-build-output` lists strippable, through its `--apply` on each archive's id;
//! 3. stale scratch and caches (`story:reclaim-scratch-class-c`: freeing them is class C): each
//!    entry directly under a root [`scratch_roots`] names (the agents' scratch, sccache and the Go
//!    build cache under the home, as the disk panel lists them, and `/var/tmp`), deleted only when
//!    [`kept_scratch`] finds no reason to keep it, read just before: the entry is ours; it neither
//!    holds nor lies in one of the instance's own places, compared both as written and with every
//!    link resolved (`story:reclaim-alias-protected`), and an entry that is a link is removed as a
//!    link, never followed; a root that is itself a link is skipped and named on standard error;
//!    nothing in it was
//!    modified in the last [`STALE`]; it is no `target/` and holds none; it holds no git worktree
//!    (a tree a session may hold a lease on: only `worktree` removes trees); no live session works
//!    in it; no process on the host has its working directory, root, program or an open file in
//!    it (`/proc/<pid>/{cwd,root,exe,fd/*}`, read at most [`HOLDERS_FRESH`] before); and no git
//!    repository in it has uncommitted or untracked work, a stash or a commit on no remote. Each
//!    deletion is one step of kind `cache` with its bytes. When the live sessions cannot be read,
//!    the step deletes nothing;
//! 4. the observations of the snapshots past the instance's `retention.snapshots`
//!    ([`Store::retain`]).
//!
//! When the floor is not reached it lists what is left, by path and size: the trees it kept and
//! why the panel or gc keeps every other row, the shared holders included. Either way it records
//! one `conductor.host.ReclaimRun` in the store under the state directory, through the generated
//! `conductor.host.Reclaim`.
//!
//! The free disk is read through [`free_bytes`]: `statvfs` on `/`, or, when
//! [`READINGS_VARIABLE`] names a file, the readings in it, one a line, each taken once and the
//! last standing; the watch reads it the same way.

use std::env;
use std::fs;
use std::io::{self, Write as _};
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{Duration, Instant, SystemTime};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{Generated, ReclaimRunStorage, TryContext, unmet_context};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::{GuardDecisionId, MessageId, ResourceRequestId};
use conductor_model::host::obligations::ReclaimBehavior as _;
use conductor_model::host::{
    GrantId, LeftOver, Reclaim as RecordReclaim, ReclaimDisk, ReclaimKind, ReclaimOutcome,
    ReclaimRunId, ReclaimRunSnapshot, ReclaimStep,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{
    BoardId, GarbageOutcome, GarbageRow, ObservationId, SnapshotId,
};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use super::{allocated, archive_strip, archives, remove};
use crate::config;
use crate::dashboard::Sources;
use crate::dashboard::waste::{self, Reclaim, Row};
use crate::store::{self, Store};

const COMMAND: &str = "host reclaim";

/// What a reclaim aims at above `thresholds.disk_admit`, so the next build does not put free
/// disk straight back under it: 5 GiB.
pub const MARGIN: u64 = 5 << 30;

/// The variable naming a file of free-disk readings, one a line, read instead of `statvfs`
/// ([`free_bytes`]).
pub const READINGS_VARIABLE: &str = "CONDUCTOR_DISK_READINGS";

/// The variable naming a directory read instead of `/var/tmp`, as the disk panel's `/var/tmp`
/// and as a root of stale entries ([`scratch_roots`]); a case points it at a fixture.
pub const VAR_TMP_VARIABLE: &str = "CONDUCTOR_RECLAIM_VAR_TMP";

/// How long nothing in a scratch directory or a cache entry may have been modified before reclaim
/// deletes it: a day. Neither the disk panel nor `thresholds` holds a staleness age yet; a
/// `thresholds` key for it is a change to the specification first.
pub const STALE: Duration = Duration::from_secs(24 * 60 * 60);

/// The scratch and caches under the home directory, as the disk panel lists them among the
/// shared holders: sccache, the Go build cache and the agents' scratch. The rustup toolchains,
/// which the panel lists beside them, are neither: builds run them.
const SCRATCH_UNDER_HOME: [&str; 3] = [".cache/sccache", ".cache/go-build", ".cache/claude-tmp"];

/// How old the reading of what the host's processes hold may be when an entry is checked: the
/// links of every process are read again once it is older.
const HOLDERS_FRESH: Duration = Duration::from_secs(5);

/// The roots whose entries are scratch or a cache: [`SCRATCH_UNDER_HOME`] under the home
/// directory `home`, and `var_tmp`. Reclaim deletes their stale entries nothing holds (step 3),
/// and a shortfall left only by them needs no person (`crate::host::needs_person`).
#[must_use]
pub fn scratch_roots(home: Option<&Path>, var_tmp: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = home
        .map(|home| {
            SCRATCH_UNDER_HOME
                .iter()
                .map(|path| home.join(path))
                .collect()
        })
        .unwrap_or_default();
    roots.push(var_tmp.to_path_buf());
    roots
}

/// How recent a write under a tree's `target/` keeps its build output: a gate between two
/// compiler runs writes nothing for a while, and no process may work in the tree meanwhile.
pub const RECENT: Duration = Duration::from_secs(10 * 60);

/// How many entries under a tree's `target/` are looked at for a recent write: the directories a
/// compiler adds files to, and the first entries of each, not the whole tree.
const RECENT_ENTRIES: usize = 4096;

/// How deep under `target/` a recent write is looked for: `target/<profile>/deps/<file>`.
const RECENT_DEPTH: usize = 3;

/// The longest the archives are measured within one reclaim run; an archive measured before is
/// answered from the cache `conductor gc` keeps.
const ARCHIVE_BOUND: Duration = Duration::from_secs(5 * 60);

/// The directory under the state directory the observations live in.
const OBSERVATIONS: &str = "observations";

/// What one reclaim run did: the bytes it freed, each step in order (the path it freed under,
/// its kind and the bytes it freed), what it left by path and size when it did not reach the
/// bytes asked for, and whether it reached them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReclaimReport {
    pub reclaimed_bytes: u64,
    pub steps: Vec<(String /*path*/, ReclaimKind, u64 /*bytes*/)>,
    pub left: Vec<(String, u64)>,
    pub reached: bool,
}

/// The floor `conductor host reclaim` and the watch reclaim to, in bytes free on `/`: the
/// instance's `disk_admit` GiB plus [`MARGIN`].
#[must_use]
pub fn floor_bytes(disk_admit_gib: u64) -> u64 {
    disk_admit_gib
        .saturating_mul(1 << 30)
        .saturating_add(MARGIN)
}

/// The bytes free on the file system holding `path`: the `available` of [`crate::disk::space`];
/// or, when [`READINGS_VARIABLE`] names a file, the first line of that file, which is taken off
/// unless it is the last.
///
/// # Errors
///
/// `statvfs` fails, or the readings file cannot be read or rewritten or its first line is no
/// count of bytes.
pub fn free_bytes(path: &Path) -> Result<u64> {
    match env::var_os(READINGS_VARIABLE) {
        Some(file) if !file.is_empty() => reading(Path::new(&file)),
        _ => crate::disk::space(path).map(|space| space.available),
    }
}

/// The first reading of `file`, taken off unless it is the last.
fn reading(file: &Path) -> Result<u64> {
    let text = fs::read_to_string(file)
        .with_context(|| format!("read the readings {}", file.display()))?;
    let mut lines = text.lines();
    let first = lines.next().unwrap_or_default().trim().to_owned();
    let rest: String = lines.map(|line| format!("{line}\n")).collect();
    if !rest.is_empty() {
        let next = file.with_extension("next");
        fs::write(&next, rest).with_context(|| format!("write {}", next.display()))?;
        fs::rename(&next, file).with_context(|| format!("replace {}", file.display()))?;
    }
    first
        .parse()
        .with_context(|| format!("{}: {first:?} is no count of bytes", file.display()))
}

/// Frees what needs no decision, in the module's order, until `need_bytes` are free on `/`, and
/// records the run in the store under the state directory `state`.
///
/// # Errors
///
/// Free disk cannot be read, `HOME` is not an absolute path, the working directory cannot be
/// read, `CONDUCTOR_GC_ARCHIVE_BOUND` is no whole number of seconds, the store does not open or
/// does not keep the run.
pub fn reclaim(state: &Path, need_bytes: u64) -> Result<ReclaimReport> {
    run(state, need_bytes).map(|run| run.report)
}

/// `conductor host reclaim`: reclaims to the instance's floor ([`floor_bytes`]) and prints each
/// step with the bytes it freed, as text or, with `--format json`, one JSON object; exits 1,
/// listing what is left by path and size, when the floor is not reached.
///
/// # Errors
///
/// `--format` is neither text nor json, the state directory cannot be decided, or [`reclaim`]
/// fails.
pub fn run_reclaim(state: Option<&Path>, format: Option<&str>) -> Result<ExitCode> {
    let input = ReclaimDisk {
        format: format.map(str::to_owned),
    };
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is neither text nor json"),
    };
    let state = crate::state::dir(state).map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
    let admit = config::active().instance.thresholds.disk_admit.0;
    let need = floor_bytes(u64::try_from(admit).unwrap_or(0));
    let run = run(&state, need).map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
    print(&run, need, json)?;
    if run.report.reached {
        return Ok(ExitCode::SUCCESS);
    }
    writeln!(
        io::stderr().lock(),
        "conductor: {COMMAND}: floor not reached: {} bytes short",
        run.shortfall
    )?;
    Ok(ExitCode::FAILURE)
}

/// One reclaim run: its report, the last free-disk reading and the bytes still lacking.
struct Run {
    report: ReclaimReport,
    free: u64,
    shortfall: u64,
}

/// The steps of a run so far, against the bytes asked for.
struct Steps {
    need: u64,
    free: u64,
    steps: Vec<(String, ReclaimKind, u64)>,
}

impl Steps {
    fn reached(&self) -> bool {
        self.free >= self.need
    }

    /// Records the step that freed `bytes` under `what`, and reads free disk again.
    fn took(&mut self, what: &Path, kind: ReclaimKind, bytes: u64) -> Result<()> {
        self.steps.push((what.display().to_string(), kind, bytes));
        self.free = free_bytes(Path::new("/")).context("read the free disk on /")?;
        Ok(())
    }
}

fn run(state: &Path, need: u64) -> Result<Run> {
    let mut steps = Steps {
        need,
        free: free_bytes(Path::new("/")).context("read the free disk on /")?,
        steps: Vec::new(),
    };
    let mut left = Vec::new();
    if !steps.reached() {
        let mut sources = Sources::active(None)?;
        if let Some(var_tmp) = env::var_os(VAR_TMP_VARIABLE).filter(|dir| !dir.is_empty()) {
            sources.var_tmp = PathBuf::from(var_tmp);
        }
        let kept = trees(&sources, &mut steps)?;
        let archives = strip_archives(&sources, state, &mut steps)?;
        scratch(&sources, state, &mut steps)?;
        if !steps.reached() {
            snapshots(state, &mut steps)?;
        }
        if !steps.reached() {
            left = leftover(&sources, kept, archives);
        }
    }
    let reclaimed_bytes = steps.steps.iter().map(|(_, _, bytes)| bytes).sum();
    let shortfall = need.saturating_sub(steps.free);
    let report = ReclaimReport {
        reclaimed_bytes,
        steps: steps.steps,
        left,
        reached: shortfall == 0,
    };
    record(state, &report, shortfall)?;
    Ok(Run {
        free: steps.free,
        report,
        shortfall,
    })
}

/// Step 1: the build output of each idle managed tree, until enough is free. Answers the trees
/// kept, each with the bytes under its `target/`.
fn trees(sources: &Sources, steps: &mut Steps) -> Result<Vec<(String, u64)>> {
    let measured = waste::repositories(sources);
    let mut trees: Vec<PathBuf> = measured
        .rows
        .iter()
        .filter_map(|row| match &row.reclaim {
            Some(Reclaim::DiscardCache(trees)) => Some(trees.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    trees.sort();
    trees.dedup();
    let mut kept = Vec::new();
    for tree in trees {
        if steps.reached() {
            break;
        }
        let held = || allocated(&[tree.join("target")]);
        if let Some(why) = busy(&tree) {
            kept.push((format!("{} ({why})", tree.display()), held()));
            continue;
        }
        let reclaim = Reclaim::DiscardCache(vec![tree.clone()]);
        let row = remove(
            sources,
            listed("build output of an idle tree", &reclaim),
            &reclaim,
        );
        match row.outcome {
            GarbageOutcome::Removed => {
                let freed = row.freed.and_then(|bytes| u64::try_from(bytes).ok());
                steps.took(&tree, ReclaimKind::IdleTreeBuildOutput, freed.unwrap_or(0))?;
            }
            _ => {
                let why = row.detail.unwrap_or_else(|| "refused".to_owned());
                kept.push((format!("{} ({why})", tree.display()), held()));
            }
        }
    }
    Ok(kept)
}

/// Step 2: the build output `worktree` lists strippable in the instance's archives, one archive
/// at a time, until enough is free. Answers the archives left, each with its strippable bytes.
fn strip_archives(
    sources: &Sources,
    state: &Path,
    steps: &mut Steps,
) -> Result<Vec<(String, u64)>> {
    let mut left = Vec::new();
    if steps.reached() {
        return Ok(left);
    }
    let bound = archives::bound()?.min(ARCHIVE_BOUND);
    let strip = archive_strip(sources, &state.join(archives::CACHE), bound, true)?;
    let Ok((_, Reclaim::StripArchives { checkout, .. })) = &strip.listed else {
        return Ok(left);
    };
    for archive in strip
        .measured
        .iter()
        .filter(|archive| archive.verdict == "strippable")
    {
        if steps.reached() {
            break;
        }
        let reclaim = Reclaim::StripArchives {
            checkout: checkout.clone(),
            archives: vec![archive.path.clone()],
        };
        let row = remove(sources, listed(archives::WHAT, &reclaim), &reclaim);
        match row.outcome {
            GarbageOutcome::Removed => {
                let freed = row.freed.and_then(|bytes| u64::try_from(bytes).ok());
                steps.took(
                    &archive.path,
                    ReclaimKind::ArchiveBuildOutput,
                    freed.unwrap_or(0),
                )?;
            }
            _ => {
                let why = row.detail.unwrap_or_else(|| "refused".to_owned());
                left.push((
                    format!("{} ({why})", archive.path.display()),
                    archive.strip_bytes,
                ));
            }
        }
    }
    Ok(left)
}

/// Step 3: each stale entry of the scratch and cache roots ([`scratch_roots`]) that
/// [`kept_scratch`] finds no reason to keep, until enough is free.
fn scratch(sources: &Sources, state: &Path, steps: &mut Steps) -> Result<()> {
    if steps.reached() {
        return Ok(());
    }
    // A session whose working directory is not known may work in any of them.
    let Ok(live) = waste::live(sources) else {
        return Ok(());
    };
    let own = [
        state,
        sources.root.as_path(),
        sources.checkouts.as_path(),
        sources.trees.as_path(),
        sources.targets.as_path(),
        sources.watch.as_path(),
    ];
    // Each place as written and as resolved: a link anywhere on the way to an entry or to a place
    // must not hide that the two are one (`story:reclaim-alias-protected`).
    let protected: Vec<PathBuf> = own
        .iter()
        .flat_map(|place| [place.to_path_buf(), resolved(place)])
        .collect();
    let homes = [sources.home.clone(), resolved(&sources.home)];
    let mut holders = Holders::default();
    for root in scratch_roots(Some(&sources.home), &sources.var_tmp) {
        if fs::symlink_metadata(&root).is_ok_and(|meta| meta.file_type().is_symlink()) {
            let _ = writeln!(
                io::stderr().lock(),
                "conductor: {COMMAND}: skipped the scratch root {}: it is a symbolic link, and \
                 what it names is not reclaim's",
                root.display()
            );
            continue;
        }
        let Ok(listed) = fs::read_dir(&root) else {
            continue;
        };
        let mut entries: Vec<PathBuf> = listed.flatten().map(|entry| entry.path()).collect();
        entries.sort();
        for entry in entries {
            if steps.reached() {
                return Ok(());
            }
            let checks = Checks {
                own: &protected,
                home: &homes,
                live: &live,
                deadline: Instant::now() + sources.measure_timeout,
            };
            if kept_scratch(&entry, &checks, &mut holders).is_some() {
                continue;
            }
            let before = allocated(std::slice::from_ref(&entry));
            let removed = if fs::symlink_metadata(&entry).is_ok_and(|meta| meta.is_dir()) {
                fs::remove_dir_all(&entry)
            } else {
                fs::remove_file(&entry)
            };
            let after = if fs::symlink_metadata(&entry).is_ok() {
                allocated(std::slice::from_ref(&entry))
            } else {
                0
            };
            let freed = before.saturating_sub(after);
            if removed.is_ok() || freed > 0 {
                steps.took(&entry, ReclaimKind::Cache, freed)?;
            }
        }
    }
    Ok(())
}

/// What [`kept_scratch`] checks an entry against: the instance's own places, which no entry may
/// hold or lie in; the home directory, which no entry may hold, each as written and as resolved
/// ([`resolved`]); the working directories of the live sessions; and when the walk of the entry
/// must have ended.
struct Checks<'a> {
    own: &'a [PathBuf],
    home: &'a [PathBuf],
    live: &'a [PathBuf],
    deadline: Instant,
}

/// `path` with every link on its way resolved, or `path` as written when it cannot be resolved
/// (it is not there).
fn resolved(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Why the scratch or cache entry `entry` must stay, read just before it would be deleted; none
/// when it may go. Anything that cannot be read is a reason to keep. An entry that is a link is
/// checked as what it names, and removed as a link, never followed.
fn kept_scratch(entry: &Path, checks: &Checks<'_>, holders: &mut Holders) -> Option<String> {
    let metadata = match fs::symlink_metadata(entry) {
        Ok(metadata) => metadata,
        Err(error) => return Some(format!("cannot read it: {error}")),
    };
    // The owner of `/proc/self` is the effective user of this process.
    match fs::metadata("/proc/self") {
        Ok(me) if me.uid() == metadata.uid() => {}
        Ok(_) => return Some("another user's".to_owned()),
        Err(error) => return Some(format!("cannot tell whose it is: {error}")),
    }
    // What the entry is once every link on its way, its own included, is resolved.
    let real = match fs::canonicalize(entry) {
        Ok(real) => real,
        Err(error) => return Some(format!("cannot resolve it: {error}")),
    };
    let overlaps = |place: &Path| {
        place.starts_with(entry)
            || entry.starts_with(place)
            || place.starts_with(&real)
            || real.starts_with(place)
    };
    if checks
        .home
        .iter()
        .any(|home| home.starts_with(entry) || home.starts_with(&real))
        || checks.own.iter().any(|place| overlaps(place))
    {
        return Some("one of the instance's own places".to_owned());
    }
    if entry.file_name().is_some_and(|name| name == "target") {
        return Some("a target/ directory".to_owned());
    }
    let walked = match walk(entry, checks.deadline) {
        Ok(walked) => walked,
        Err(why) => return Some(why),
    };
    if walked.target {
        return Some("it holds a target/ directory".to_owned());
    }
    if walked.worktree {
        return Some("it holds a git worktree, which a session may hold".to_owned());
    }
    let Some(stale_since) = SystemTime::now().checked_sub(STALE) else {
        return Some("the clock is before the staleness age".to_owned());
    };
    if walked.newest.is_none_or(|newest| newest > stale_since) {
        return Some(format!(
            "modified in the last {} hours",
            STALE.as_secs() / 3600
        ));
    }
    if waste::works_in(checks.live, entry) {
        return Some("a live session works in it".to_owned());
    }
    match holders.holding(entry) {
        Err(why) => return Some(why),
        Ok(Some(pid)) => return Some(format!("process {pid} holds it")),
        Ok(None) => {}
    }
    walked
        .repositories
        .iter()
        .find_map(|repository| uncommitted(repository))
}

/// What a walk of a scratch or cache entry found.
#[derive(Debug, Default)]
struct Walked {
    /// The newest modification time of anything in it, itself included.
    newest: Option<SystemTime>,
    /// Whether a directory named `target` lies below it.
    target: bool,
    /// Whether a `.git` file lies in it: a linked git worktree.
    worktree: bool,
    /// The directories holding a `.git` directory: git repositories.
    repositories: Vec<PathBuf>,
}

/// Walks `entry` without following a link, by `deadline`; or why it could not be walked.
fn walk(entry: &Path, deadline: Instant) -> Result<Walked, String> {
    let mut walked = Walked::default();
    let mut pending = vec![(entry.to_path_buf(), 0usize)];
    while let Some((path, depth)) = pending.pop() {
        if Instant::now() > deadline {
            return Err("it could not be walked in time".to_owned());
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        };
        if let Ok(modified) = metadata.modified() {
            walked.newest = walked.newest.max(Some(modified));
        }
        let name = path.file_name().unwrap_or_default();
        if name == ".git" {
            if metadata.is_dir() {
                walked
                    .repositories
                    .push(path.parent().unwrap_or(entry).to_path_buf());
            } else {
                walked.worktree = true;
            }
            continue;
        }
        if !metadata.is_dir() {
            continue;
        }
        if depth > 0 && name == "target" {
            walked.target = true;
        }
        let listed = fs::read_dir(&path)
            .map_err(|error| format!("cannot list {}: {error}", path.display()))?;
        for child in listed {
            let child =
                child.map_err(|error| format!("cannot list {}: {error}", path.display()))?;
            pending.push((child.path(), depth + 1));
        }
    }
    Ok(walked)
}

/// What the host's processes hold: each process's working directory, root, program and open
/// files, read from the `/proc/<pid>` links, and when they were read.
#[derive(Debug, Default)]
struct Holders {
    read_at: Option<Instant>,
    links: Vec<(u32, PathBuf)>,
}

impl Holders {
    /// The first process that holds `path` or anything under it, from links read at most
    /// [`HOLDERS_FRESH`] before; a process whose links cannot be read (another user's, or one that
    /// ended) is skipped. When `/proc` cannot be listed, why: no process can then be told to be
    /// clear of `path`.
    fn holding(&mut self, path: &Path) -> Result<Option<u32>, String> {
        if self
            .read_at
            .is_none_or(|read_at| read_at.elapsed() > HOLDERS_FRESH)
        {
            self.links = holder_links()?;
            self.read_at = Some(Instant::now());
        }
        let paths: Vec<PathBuf> = [Some(path.to_path_buf()), fs::canonicalize(path).ok()]
            .into_iter()
            .flatten()
            .collect();
        Ok(self
            .links
            .iter()
            .find(|(_, link)| paths.iter().any(|path| link.starts_with(path)))
            .map(|(pid, _)| *pid))
    }
}

/// Every `/proc/<pid>/{cwd,root,exe,fd/*}` link that can be read, with its process.
fn holder_links() -> Result<Vec<(u32, PathBuf)>, String> {
    let entries = fs::read_dir("/proc").map_err(|error| format!("cannot list /proc: {error}"))?;
    let mut links = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let process = entry.path();
        let named = ["cwd", "root", "exe"]
            .iter()
            .filter_map(|link| fs::read_link(process.join(link)).ok());
        let open = fs::read_dir(process.join("fd"))
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|fd| fs::read_link(fd.path()).ok());
        // The root of a process outside a container is `/`, which holds everything: only a root
        // elsewhere is a hold.
        links.extend(
            named
                .chain(open)
                .filter(|link| link != Path::new("/"))
                .map(|link| (pid, link)),
        );
    }
    Ok(links)
}

/// Why the git repository at `repository` holds work: uncommitted changes or untracked files, a
/// stash, or a commit on no remote; or git could not tell. None when it holds none.
fn uncommitted(repository: &Path) -> Option<String> {
    let git = |args: &[&str]| {
        Command::new("git")
            .arg("-C")
            .arg(repository)
            .args(args)
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
    };
    let shown = repository.display();
    for (args, what) in [
        (
            &["status", "--porcelain", "--ignore-submodules=none"][..],
            "uncommitted git work",
        ),
        (
            &["rev-list", "-n", "1", "--all", "--not", "--remotes"][..],
            "a commit or stash on no remote",
        ),
    ] {
        match git(args) {
            Ok(output) if output.status.success() => {
                if !output.stdout.is_empty() {
                    return Some(format!("{what} in {shown}"));
                }
            }
            _ => return Some(format!("git could not tell whether {shown} holds work")),
        }
    }
    None
}

/// Step 4: the observations of the snapshots past the instance's retention.
fn snapshots(state: &Path, steps: &mut Steps) -> Result<()> {
    let keep = usize::try_from(config::active().instance.retention.snapshots).unwrap_or(1);
    let observations = state.join(OBSERVATIONS);
    let before = allocated(std::slice::from_ref(&observations));
    let erased = {
        let store = store::open(state)?;
        store
            .retain(keep)
            .context("drop the observations retention does not keep")?
    };
    if !erased.is_empty() {
        let after = allocated(std::slice::from_ref(&observations));
        steps.took(
            &observations,
            ReclaimKind::Snapshot,
            before.saturating_sub(after),
        )?;
    }
    Ok(())
}

/// What is left that only a person may remove, largest first: the trees and archives the run
/// kept, every row of the disk panel `conductor gc` keeps or removes only on a decision (a tree
/// eligible for removal, a checkout's `target/`), and the shared holders.
fn leftover(
    sources: &Sources,
    kept: Vec<(String, u64)>,
    archives: Vec<(String, u64)>,
) -> Vec<(String, u64)> {
    let mut left = kept;
    left.extend(archives);
    let rows = waste::repositories(sources).rows;
    let shared = waste::shared_rows(sources);
    for row in rows.iter().chain(&shared) {
        if matches!(row.reclaim, Some(Reclaim::DiscardCache(_))) {
            continue;
        }
        let Some(bytes) = row.bytes.filter(|bytes| *bytes > 0) else {
            continue;
        };
        left.push((place(row), bytes));
    }
    left.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    left
}

/// Where the row `row` lies: its path, its reclaim's paths, or its repository and what it is.
fn place(row: &Row) -> String {
    if let Some(path) = &row.path {
        return path.clone();
    }
    if let Some(reclaim) = &row.reclaim {
        let paths: Vec<String> = reclaim
            .paths()
            .iter()
            .map(|path| path.display().to_string())
            .collect();
        return paths.join(" ");
    }
    format!("{}: {}", row.repository.as_deref().unwrap_or("-"), row.what)
}

/// The row `conductor gc`'s removal takes for `reclaim`, before anything is removed.
fn listed(what: &str, reclaim: &Reclaim) -> GarbageRow {
    GarbageRow {
        repository: None,
        what: what.to_owned(),
        paths: reclaim
            .paths()
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        count: Some(1),
        bytes: None,
        freed: None,
        outcome: GarbageOutcome::Reclaimable,
        detail: None,
    }
}

/// Why the build output of `tree` must stay although no live session works in it: a process on
/// the host works in it, or its `target/` was written in the last [`RECENT`]. None when it may go.
fn busy(tree: &Path) -> Option<String> {
    match process_in(tree) {
        Err(error) => return Some(error),
        Ok(Some(pid)) => return Some(format!("process {pid} works in it")),
        Ok(None) => {}
    }
    let target = tree.join("target");
    if written_within(&target, RECENT) {
        return Some(format!(
            "its target/ was written in the last {} minutes: a gate may be running",
            RECENT.as_secs() / 60
        ));
    }
    None
}

/// The first process on the host whose working directory is `path` or lies under it, read from
/// the `/proc/<pid>/cwd` links; an entry that cannot be read is skipped. When `/proc` cannot be
/// listed, why: no process can then be told to be clear of `path`.
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
        if waste::works_in(std::slice::from_ref(&cwd), path) {
            return Ok(Some(pid));
        }
    }
    Ok(None)
}

/// Whether `dir`, or an entry at most [`RECENT_DEPTH`] below it among the first
/// [`RECENT_ENTRIES`] looked at, was modified within `within` of now. A directory's time moves
/// when an entry is added to it, so the directories a compiler writes into show its writes.
fn written_within(dir: &Path, within: Duration) -> bool {
    let Some(since) = SystemTime::now().checked_sub(within) else {
        return false;
    };
    let mut pending = vec![(dir.to_path_buf(), 0)];
    let mut seen = 0;
    while let Some((path, depth)) = pending.pop() {
        let Ok(metadata) = fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.modified().is_ok_and(|modified| modified > since) {
            return true;
        }
        seen += 1;
        if seen >= RECENT_ENTRIES {
            break;
        }
        if metadata.is_dir()
            && depth < RECENT_DEPTH
            && let Ok(entries) = fs::read_dir(&path)
        {
            pending.extend(entries.flatten().map(|entry| (entry.path(), depth + 1)));
        }
    }
    false
}

fn signed(bytes: u64) -> i64 {
    i64::try_from(bytes).unwrap_or(i64::MAX)
}

/// Records `report` as one `conductor.host.ReclaimRun` through the generated
/// `conductor.host.Reclaim`, in the store under `state`.
fn record(state: &Path, report: &ReclaimReport, shortfall: u64) -> Result<()> {
    let ran_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format the time of the reclaim run")?;
    let input = RecordReclaim {
        instance: config::active().instance.name.clone(),
        steps: report
            .steps
            .iter()
            .map(|(what, kind, bytes)| ReclaimStep {
                what: what.clone(),
                kind: *kind,
                bytes: signed(*bytes),
            })
            .collect(),
        reclaimed_bytes: signed(report.reclaimed_bytes),
        shortfall_bytes: signed(shortfall),
        left: report
            .left
            .iter()
            .map(|(path, bytes)| LeftOver {
                path: path.clone(),
                bytes: signed(*bytes),
            })
            .collect(),
        ran_at: Timestamp(ran_at),
    };
    let mut generated = Generated::new(Ports {
        store: store::open(state)?,
    });
    let outcome = generated
        .reclaim(input)
        .map_err(|unmet| anyhow!("host record-reclaim: {unmet}"))?;
    if let ReclaimOutcome::InvalidSize { error } = outcome {
        bail!(
            "host record-reclaim: a negative count of bytes, {}",
            error.bytes
        );
    }
    generated
        .ports
        .store
        .check()
        .context("keep the reclaim run")?;
    Ok(())
}

/// The name of `kind` as the command prints it.
fn kind_text(kind: ReclaimKind) -> &'static str {
    match kind {
        ReclaimKind::IdleTreeBuildOutput => "idle-tree-build-output",
        ReclaimKind::ArchiveBuildOutput => "archive-build-output",
        ReclaimKind::Cache => "cache",
        ReclaimKind::Snapshot => "snapshot",
    }
}

/// Prints `run` against the floor `need`: one line a step, then, short of the floor, one line
/// for each thing left, then the total; or one JSON object.
fn print(run: &Run, need: u64, json: bool) -> Result<()> {
    let report = &run.report;
    let mut out = io::stdout().lock();
    if json {
        let steps: Vec<Value> = report
            .steps
            .iter()
            .map(|(what, kind, bytes)| json!({"what": what, "kind": kind_text(*kind), "bytes": bytes}))
            .collect();
        let left: Vec<Value> = report
            .left
            .iter()
            .map(|(path, bytes)| json!({"path": path, "bytes": bytes}))
            .collect();
        let object = json!({
            "result": if report.reached { "reclaimed" } else { "floor-not-reached" },
            "need_bytes": need,
            "free_bytes": run.free,
            "reclaimed_bytes": report.reclaimed_bytes,
            "shortfall_bytes": run.shortfall,
            "steps": steps,
            "left": left,
        });
        writeln!(out, "{object}")?;
        out.flush()?;
        return Ok(());
    }
    for (what, kind, bytes) in &report.steps {
        writeln!(
            out,
            "reclaimed  {bytes} bytes  {}  {what}",
            kind_text(*kind)
        )?;
    }
    for (path, bytes) in &report.left {
        writeln!(out, "left  {bytes} bytes  {path}")?;
    }
    if report.reached {
        writeln!(
            out,
            "reclaimed {} bytes; {} bytes free on /, the floor is {need}",
            report.reclaimed_bytes, run.free
        )?;
    } else {
        writeln!(
            out,
            "floor not reached: {} bytes short; reclaimed {} bytes, {} bytes free on /, the \
             floor is {need}",
            run.shortfall, report.reclaimed_bytes, run.free
        )?;
    }
    out.flush()?;
    Ok(())
}

/// The ports the generated `conductor.host.Reclaim` runs over: the store, and the identity it
/// assigns.
struct Ports {
    store: Store,
}

impl ReclaimRunStorage for Ports {
    fn get(&self, identity: &ReclaimRunId) -> Option<ReclaimRunSnapshot> {
        ReclaimRunStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: ReclaimRunSnapshot) {
        ReclaimRunStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &ReclaimRunId) {
        ReclaimRunStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<ReclaimRunSnapshot> {
        ReclaimRunStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.GuardDecisionId"))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_host_grant_id(&mut self) -> Result<GrantId, UnmetObligation> {
        Err(unmet_context("conductor.host.GrantId"))
    }

    fn try_generate_conductor_host_reclaim_run_id(
        &mut self,
    ) -> Result<ReclaimRunId, UnmetObligation> {
        Ok(ReclaimRunId(eventlog_core::new_event_id()))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }
}
