//! The repositories' shared targets of `conductor build run` (`story:build-shared-target-dir`):
//! `<cache>/target/<key>`, each with its lock `<cache>/target/.locks/<key>.lock`
//! ([`crate::build::targets_root`]), which `build run` holds for the whole of a command that
//! builds into it. `conductor gc` takes the lock without waiting: a target whose lock it takes has
//! no build running, and it is removed under that lock, so no build starts into it meanwhile; a
//! target whose lock a run holds is kept. An entry with no lock file is not `build run`'s and is
//! left alone.
//!
//! And every other cargo target under the instance's trees and cache roots
//! (`story:gc-idle-targets`): a directory of cargo's own shape (a `CACHEDIR.TAG` that names
//! cargo, and `.rustc_info.json` or a profile directory with cargo's `.cargo-lock`; a directory
//! another tool tagged is none) under the managed trees, the shared targets' directory or the
//! instance's scratch, never one inside a primary checkout (a directory holding a `.git`
//! directory, or anything under the checkouts root) and never one a lock above answers for.
//! [`assess`] lists each with its last write and why it is not idle; one is idle when it holds no
//! [`HOLD`] file, it is this user's, no cargo holds its lock, no process on the host has its
//! working directory, its program or an open file in it, names it in `CARGO_TARGET_DIR` or names
//! a path in it on its command line (each compared as written and with its links resolved), and
//! nothing in its top three levels was written for [`WINDOW`] ([`WINDOW_LOW`] while less than
//! [`LOW`] is free on `/`). A process this user may not read (another user's) is not seen.
//! [`clean`] makes every check again under cargo's lock, and under it moves the target aside in
//! its own directory, to `.conductor-gc-<pid>-<count>`, before it removes it: a build that starts
//! meanwhile makes a target of its own and loses nothing. `conductor gc` and the host reclaim
//! clean idle targets oldest first until free disk is at the floor. A directory of that name
//! whose process has ended is what a gc that died left: it is gc's own, listed and removed
//! without an idle check.

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io::{self, Read as _};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::build::{TARGET_LOCKS, orphaned_group, targets_root};
use crate::config;
use crate::dashboard::Sources;
use crate::disk;

/// What gc calls one repository's shared target.
pub(super) const WHAT: &str = "shared build target";

/// One repository's shared target.
pub(super) struct Target {
    /// `<root>/<key>`.
    pub(super) dir: PathBuf,
    /// `<root>/.locks/<key>.lock`.
    lock: PathBuf,
}

/// The shared targets under `root`, in name order: each directory but [`TARGET_LOCKS`] whose lock
/// file exists.
pub(super) fn list(root: &Path) -> Vec<Target> {
    let mut found: Vec<Target> = fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_name() != TARGET_LOCKS)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .filter_map(|entry| {
            let mut lock = entry.file_name();
            lock.push(".lock");
            let lock = root.join(TARGET_LOCKS).join(lock);
            lock.is_file().then(|| Target {
                dir: entry.path(),
                lock,
            })
        })
        .collect();
    found.sort_by(|a, b| a.dir.cmp(&b.dir));
    found
}

/// The paths under `root` gc answers for itself, which it keeps out of the panel's shared holders:
/// the locks' directory and each target.
pub(super) fn handled(root: &Path, targets: &[Target]) -> Vec<PathBuf> {
    std::iter::once(root.join(TARGET_LOCKS))
        .chain(targets.iter().map(|target| target.dir.clone()))
        .collect()
}

impl Target {
    /// Takes the target's lock without waiting; why it is kept when a run holds it, the command of
    /// a killed run still builds into it ([`orphaned_group`]), or the lock cannot be taken.
    pub(super) fn hold(&self) -> Result<File, String> {
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.lock)
            .map_err(|error| {
                format!("its lock {} cannot be opened: {error}", self.lock.display())
            })?;
        match file.try_lock() {
            Ok(()) => match orphaned_group(&mut file) {
                None => Ok(file),
                Some(group) => Err(format!(
                    "the command of a stopped build, process group {group}, still runs in it"
                )),
            },
            Err(TryLockError::WouldBlock) => {
                Err("a build of the repository holds its lock: a build is running".to_owned())
            }
            Err(TryLockError::Error(error)) => Err(format!(
                "its lock {} cannot be taken: {error}",
                self.lock.display()
            )),
        }
    }

    /// Removes the target, under its lock `_held`, which keeps a build from starting into it.
    pub(super) fn remove(&self, _held: &File) -> Result<(), String> {
        fs::remove_dir_all(&self.dir)
            .map_err(|error| format!("remove {}: {error}", self.dir.display()))
    }
}

// The idle cargo targets (`story:gc-idle-targets`).

/// What gc calls a cargo target it may clean.
pub(super) const IDLE: &str = "idle cargo target";

/// What gc calls a cargo target it keeps, among the holders.
pub(super) const KEPT: &str = "cargo target";

/// What gc calls a target an earlier gc moved aside and did not finish removing.
pub(super) const LEFT: &str = "cargo target gc left half-removed";

/// How the name gc gives a target it moves aside begins: `.conductor-gc-<pid>-<count>`.
const ASIDE: &str = ".conductor-gc-";

/// The longest path Linux takes, `PATH_MAX`: a longer word a process names is no path.
const PATH_MAX: usize = 4096;

/// The targets this process has moved aside, for the `<count>` of the next one's name.
static MOVED: AtomicU64 = AtomicU64::new(0);

/// The file that keeps the target holding it from ever being cleaned.
pub const HOLD: &str = ".keeper-hold";

/// How long nothing may have written a target's top [`LEVELS`] levels before it is idle.
pub const WINDOW: Duration = Duration::from_secs(180 * 60);

/// The same while less than [`LOW`] is free on `/`.
pub const WINDOW_LOW: Duration = Duration::from_secs(90 * 60);

/// The bytes free on `/` under which the window is [`WINDOW_LOW`]: 50 GiB.
pub const LOW: u64 = 50 << 30;

/// How deep under a target a write is looked for: `<target>/<profile>/deps/<file>`.
const LEVELS: usize = 3;

/// How deep under a root a target is looked for.
const DEPTH: usize = 8;

/// The file cargo writes into a target directory.
const TAG: &str = "CACHEDIR.TAG";

/// What cargo's tag says of itself, after the signature every tool's tag carries.
const TAG_OF_CARGO: &str = "created by cargo";

/// The file cargo writes directly into a target directory beside its tag.
const RUSTC_INFO: &str = ".rustc_info.json";

/// The lock file cargo keeps in each profile directory of a target; a cargo that builds into the
/// target holds a shared `flock` on it for the whole build.
const CARGO_LOCK: &str = ".cargo-lock";

/// The window a target must have been quiet for with `free` bytes free on `/`.
pub(super) fn window(free: u64) -> Duration {
    if free < LOW { WINDOW_LOW } else { WINDOW }
}

/// One cargo target under the instance's trees or cache roots.
#[derive(Debug)]
pub(super) struct Candidate {
    /// The root it was found under.
    root: PathBuf,
    pub(super) path: PathBuf,
    /// The bytes allocated under it; none when the assessment did not measure.
    pub(super) bytes: Option<u64>,
    /// The newest modification time in its top [`LEVELS`] levels; none when it could not be read.
    pub(super) written: Option<SystemTime>,
    /// Why it is not idle; none when it is.
    pub(super) held: Option<String>,
    /// It is no target but what a gc that ended left moved aside ([`mover`]): gc's own.
    pub(super) left: bool,
}

impl Candidate {
    /// `last write <RFC 3339, to the second>`, or that it is not known.
    pub(super) fn last_write(&self) -> String {
        self.written
            .and_then(|written| OffsetDateTime::from(written).replace_nanosecond(0).ok())
            .and_then(|written| written.format(&Rfc3339).ok())
            .map_or_else(
                || "last write unknown".to_owned(),
                |written| format!("last write {written}"),
            )
    }
}

/// The roots cargo targets are looked for under: the instance's managed trees, its shared targets
/// and its scratch, the last two both as the panel's sources and as the instance's cache name them.
fn roots(sources: &Sources) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for root in [
        sources.trees.clone(),
        sources.targets.clone(),
        sources.scratch.clone(),
        targets_root(&config::active().instance),
    ] {
        if !roots.contains(&root) {
            roots.push(root);
        }
    }
    roots
}

fn is_dir(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_dir())
}

fn is_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
}

/// Whether the file [`TAG`] of `dir` is cargo's: a file, no link, one of whose first lines says
/// [`TAG_OF_CARGO`]. Other tools write the same signature into caches of their own (a virtual
/// environment, `.pytest_cache`, `.mypy_cache`, `.ruff_cache`), under their own name or none.
fn tagged_by_cargo(dir: &Path) -> bool {
    let tag = dir.join(TAG);
    let mut text = Vec::new();
    is_file(&tag)
        && File::open(&tag)
            .and_then(|file| file.take(4096).read_to_end(&mut text))
            .is_ok()
        && String::from_utf8_lossy(&text)
            .lines()
            .any(|line| line.contains(TAG_OF_CARGO))
}

/// The lock files cargo keeps in the profile directories of `target`, `<target>/<profile>/` and
/// `<target>/<triple>/<profile>/`, in path order; no link is followed.
fn cargo_locks(target: &Path) -> Vec<PathBuf> {
    let directories = |dir: &Path| -> Vec<PathBuf> {
        fs::read_dir(dir)
            .into_iter()
            .flatten()
            .flatten()
            .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
            .map(|entry| entry.path())
            .collect()
    };
    let mut locks: Vec<PathBuf> = directories(target)
        .into_iter()
        .flat_map(|first| {
            let below = directories(&first);
            std::iter::once(first).chain(below)
        })
        .map(|dir| dir.join(CARGO_LOCK))
        .filter(|lock| is_file(lock))
        .collect();
    locks.sort();
    locks
}

/// Whether `dir` is a cargo target, by what cargo itself puts there: a directory, no link, whose
/// [`TAG`] is cargo's ([`tagged_by_cargo`]) and which holds [`RUSTC_INFO`] or a profile directory
/// with cargo's lock file ([`cargo_locks`]).
fn is_cargo(dir: &Path) -> bool {
    is_dir(dir)
        && tagged_by_cargo(dir)
        && (is_file(&dir.join(RUSTC_INFO)) || !cargo_locks(dir).is_empty())
}

/// The process that moved `dir` aside, by its name `.conductor-gc-<pid>-<count>`; none when
/// the name is not one gc gives.
fn mover(dir: &Path) -> Option<u32> {
    let name = dir.file_name()?.to_str()?;
    let (pid, count) = name.strip_prefix(ASIDE)?.split_once('-')?;
    let digits = |text: &str| !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
    if digits(pid) && digits(count) {
        pid.parse().ok()
    } else {
        None
    }
}

/// Whether `dir` is what a gc that ended left moved aside: a directory, no link, of gc's name
/// ([`mover`]) whose process is no more. One whose process runs is being removed by it.
fn left(dir: &Path) -> bool {
    is_dir(dir)
        && mover(dir).is_some_and(|pid| {
            pid != std::process::id() && !Path::new("/proc").join(pid.to_string()).exists()
        })
}

/// A name in the directory of `target` that nothing has, to move it aside to.
fn aside(target: &Path) -> Result<PathBuf, String> {
    let parent = target
        .parent()
        .ok_or_else(|| format!("{} has no directory to move it aside in", target.display()))?;
    loop {
        let count = MOVED.fetch_add(1, Ordering::Relaxed);
        let aside = parent.join(format!("{ASIDE}{}-{count}", std::process::id()));
        match fs::symlink_metadata(&aside) {
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(aside),
            Err(error) => return Err(format!("cannot read {}: {error}", aside.display())),
        }
    }
}

/// Whether `dir` lies outside `root` or in a primary checkout under it.
fn in_checkout(root: &Path, dir: &Path) -> bool {
    !dir.starts_with(root)
        || dir
            .ancestors()
            .take_while(|dir| dir.starts_with(root))
            .any(primary)
}

/// Whether `dir` is a primary checkout: it holds a `.git` directory. A linked worktree, as a
/// managed tree is, holds a `.git` file.
fn primary(dir: &Path) -> bool {
    is_dir(&dir.join(".git"))
}

/// `path` with every link on its way resolved, or as written when it cannot be resolved.
fn resolved(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `path` with the links on its way resolved as far as it is there: its deepest ancestor that
/// resolves, and the rest as written (a path a process names need not exist yet); `path` as
/// written when nothing of it resolves.
fn resolved_far(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut at = path;
    loop {
        if let Ok(real) = fs::canonicalize(at) {
            return rest.iter().rev().fold(real, |path, name| path.join(name));
        }
        match (at.parent(), at.file_name()) {
            (Some(parent), Some(name)) => {
                rest.push(name.to_os_string());
                at = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// The cargo targets under `root`, at most [`DEPTH`] below it, and what a gc that ended left
/// moved aside ([`left`]), found without following a link: never `root` itself, nothing inside a
/// primary checkout or under the checkouts root `checkouts`, nothing inside a target or a
/// directory of gc's name, and nothing below a `.git` or `node_modules`. A directory another
/// tool tagged is no target, and the walk goes on below it.
fn find(root: &Path, checkouts: &Path) -> Vec<PathBuf> {
    let (real_root, real_checkouts) = (resolved(root), resolved(checkouts));
    let mut found = Vec::new();
    let mut pending = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = pending.pop() {
        let real = dir
            .strip_prefix(root)
            .map_or_else(|_| dir.clone(), |below| real_root.join(below));
        if !is_dir(&dir)
            || primary(&dir)
            || dir.starts_with(checkouts)
            || real.starts_with(&real_checkouts)
        {
            continue;
        }
        if depth > 0 && mover(&dir).is_some() {
            if left(&dir) {
                found.push(dir);
            }
            continue;
        }
        if depth > 0 && is_cargo(&dir) {
            found.push(dir);
            continue;
        }
        if depth >= DEPTH {
            continue;
        }
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name != ".git"
                && name != "node_modules"
                && entry.file_type().is_ok_and(|kind| kind.is_dir())
            {
                pending.push((entry.path(), depth + 1));
            }
        }
    }
    found
}

/// The newest modification time of `target` and of everything at most [`LEVELS`] below it, read
/// without following a link; or what could not be read. A directory's time moves when an entry is
/// added to it, so the directories a compiler writes into show its writes.
fn newest(target: &Path) -> Result<SystemTime, String> {
    let mut newest = None;
    let mut pending = vec![(target.to_path_buf(), 0)];
    while let Some((path, depth)) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if depth > 0 && error.kind() == io::ErrorKind::NotFound => continue,
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        };
        let modified = metadata
            .modified()
            .map_err(|error| format!("cannot read when {} was written: {error}", path.display()))?;
        newest = newest.max(Some(modified));
        if metadata.is_dir() && depth < LEVELS {
            let entries = fs::read_dir(&path)
                .map_err(|error| format!("cannot list {}: {error}", path.display()))?;
            for entry in entries {
                let entry =
                    entry.map_err(|error| format!("cannot list {}: {error}", path.display()))?;
                pending.push((entry.path(), depth + 1));
            }
        }
    }
    newest.ok_or_else(|| format!("cannot read {}", target.display()))
}

/// What one process on the host holds, as far as this user may read it.
#[derive(Debug, Default)]
struct Process {
    pid: u32,
    /// `/proc/<pid>/cwd`.
    cwd: Option<PathBuf>,
    /// `/proc/<pid>/exe`.
    exe: Option<PathBuf>,
    /// `/proc/<pid>/fd/*`.
    open: Vec<PathBuf>,
    /// `CARGO_TARGET_DIR` of `/proc/<pid>/environ`, as [`named`] reads a path.
    target_dir: Vec<PathBuf>,
    /// The words of `/proc/<pid>/cmdline`.
    words: Vec<String>,
    /// Each word, and what follows a `=` in it, read as a path ([`named`]); an option is none.
    paths: Vec<PathBuf>,
}

/// `path` without its `.` and `..` components, each `..` taking the component before it.
fn normal(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

/// The path a process whose working directory is `cwd` names with `path`, a relative one from
/// that directory: as written, and with its links resolved as far as it is there
/// ([`resolved_far`]), so a target named through a link is the target. None when it is relative
/// and the working directory is not known, and none when it is longer than [`PATH_MAX`]: that
/// is no path, and resolving it costs its length for each of its components.
fn named(path: &Path, cwd: Option<&Path>) -> Vec<PathBuf> {
    if path.as_os_str().len() > PATH_MAX {
        return Vec::new();
    }
    let path = match cwd {
        _ if path.is_absolute() => path.to_path_buf(),
        Some(cwd) => cwd.join(path),
        None => return Vec::new(),
    };
    let (written, real) = (normal(&path), normal(&resolved_far(&path)));
    if written == real {
        vec![written]
    } else {
        vec![written, real]
    }
}

/// Every process on the host but this one; one whose links cannot be read (another user's, or
/// one that ended) holds nothing that is known. When `/proc` cannot be listed, why: no process
/// can then be told to be clear of a target.
fn processes() -> Result<Vec<Process>, String> {
    let entries = fs::read_dir("/proc").map_err(|error| format!("cannot list /proc: {error}"))?;
    let mut processes = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
            .filter(|pid| *pid != std::process::id())
        else {
            continue;
        };
        let dir = entry.path();
        let cwd = fs::read_link(dir.join("cwd")).ok();
        let target_dir = fs::read(dir.join("environ"))
            .ok()
            .and_then(|environ| {
                environ
                    .split(|byte| *byte == 0)
                    .find_map(|variable| variable.strip_prefix(b"CARGO_TARGET_DIR="))
                    .filter(|value| !value.is_empty())
                    .map(|value| named(Path::new(OsStr::from_bytes(value)), cwd.as_deref()))
            })
            .unwrap_or_default();
        let words: Vec<String> = fs::read(dir.join("cmdline"))
            .map(|cmdline| {
                cmdline
                    .split(|byte| *byte == 0)
                    .filter(|word| !word.is_empty())
                    .map(|word| String::from_utf8_lossy(word).into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let paths = words
            .iter()
            .flat_map(|word| {
                [
                    Some(word.as_str()),
                    word.split_once('=').map(|(_, value)| value),
                ]
            })
            .flatten()
            .filter(|part| !part.is_empty() && !part.starts_with('-'))
            .flat_map(|part| named(Path::new(part), cwd.as_deref()))
            .collect();
        processes.push(Process {
            pid,
            exe: fs::read_link(dir.join("exe")).ok(),
            open: fs::read_dir(dir.join("fd"))
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|fd| fs::read_link(fd.path()).ok())
                .collect(),
            target_dir,
            words,
            paths,
            cwd,
        });
    }
    Ok(processes)
}

/// Whether `word` holds the path of `target` where a path ends or goes on below it: a path inside
/// a longer word, as a shell command carries one.
fn writes(word: &str, target: &Path) -> bool {
    let text = target.to_string_lossy();
    word.match_indices(text.as_ref()).any(|(at, _)| {
        word[at + text.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || matches!(next, '-' | '_' | '.')))
    })
}

/// The first of `processes` that holds `target`, and how: its working directory, its program or
/// an open file lies in it, its `CARGO_TARGET_DIR` names it, or its command line names it or a
/// path in it. The target is compared as written and resolved, and so is what a process names.
fn holding(processes: &[Process], target: &Path) -> Option<String> {
    let mut targets = vec![target.to_path_buf()];
    let real = resolved(target);
    if real != target {
        targets.push(real);
    }
    let within = |path: &Path| targets.iter().any(|target| path.starts_with(target));
    processes.iter().find_map(|process| {
        let how = if process.cwd.as_deref().is_some_and(within) {
            "has its working directory in it"
        } else if process.exe.as_deref().is_some_and(within) {
            "runs a program in it"
        } else if process.open.iter().any(|open| within(open)) {
            "has a file open in it"
        } else if process.target_dir.iter().any(|path| within(path)) {
            "has CARGO_TARGET_DIR naming it"
        } else if process.paths.iter().any(|path| within(path))
            || process
                .words
                .iter()
                .any(|word| targets.iter().any(|target| writes(word, target)))
        {
            "names it on its command line"
        } else {
            return None;
        };
        Some(format!("process {} {how}", process.pid))
    })
}

/// Why the directory `target`, found under `root`, is never cleaned as it stands, whatever runs:
/// it is no cargo target ([`is_cargo`]), it lies in a primary checkout, it holds a [`HOLD`] file,
/// or it is not this user's. None when none holds.
fn kept(root: &Path, target: &Path) -> Option<String> {
    if !is_cargo(target) {
        return Some(format!(
            "it is no cargo target: no directory with cargo's {TAG} and layout"
        ));
    }
    if in_checkout(root, target) {
        return Some("it lies in a primary checkout".to_owned());
    }
    if fs::symlink_metadata(target.join(HOLD)).is_ok() {
        return Some(format!("it holds a {HOLD} file"));
    }
    // The owner of `/proc/self` is the effective user of this process.
    match (fs::metadata("/proc/self"), fs::symlink_metadata(target)) {
        (Ok(me), Ok(it)) if me.uid() == it.uid() => None,
        (Ok(_), Ok(_)) => Some("it is another user's".to_owned()),
        _ => Some("cannot tell whose it is".to_owned()),
    }
}

/// Takes cargo's own lock on `target` without waiting: an exclusive `flock` on every lock file of
/// its profile directories ([`cargo_locks`]), which a cargo that builds into the target holds
/// shared for the whole build and a cargo that starts waits for. The locks, held until they are
/// dropped; or why one is not taken: a build is running.
fn lock(target: &Path) -> Result<Vec<File>, String> {
    cargo_locks(target)
        .into_iter()
        .map(|path| {
            let shown = path.display();
            let file = File::open(&path)
                .map_err(|error| format!("cargo's lock {shown} cannot be opened: {error}"))?;
            match file.try_lock() {
                Ok(()) => Ok(file),
                Err(TryLockError::WouldBlock) => {
                    Err(format!("cargo's lock {shown} is held: a build is running"))
                }
                Err(TryLockError::Error(error)) => {
                    Err(format!("cargo's lock {shown} cannot be taken: {error}"))
                }
            }
        })
        .collect()
}

/// Why something works in `target` or did within `window`; none when nothing does: none of
/// `processes` holds it ([`holding`]), and its last write `written`, the newest in its top
/// [`LEVELS`] levels, is older than the window. A write that could not be read is a reason.
fn busy(
    target: &Path,
    window: Duration,
    processes: &[Process],
    written: &Result<SystemTime, String>,
) -> Option<String> {
    if let Some(holder) = holding(processes, target) {
        return Some(holder);
    }
    let written = match written {
        Ok(written) => *written,
        Err(why) => return Some(why.clone()),
    };
    let now = SystemTime::now();
    let Some(quiet_since) = now.checked_sub(window) else {
        return Some("the clock is before the window".to_owned());
    };
    if written > quiet_since {
        let minutes = now
            .duration_since(written)
            .map_or(0, |ago| ago.as_secs() / 60);
        return Some(format!(
            "written {minutes} minutes ago, within the {} minutes a target must be quiet",
            window.as_secs() / 60
        ));
    }
    None
}

/// Every cargo target under the instance's trees and cache roots ([`find`]) but those under a
/// path of `skip` (what another row of gc answers for), each with its last write and why it is
/// not idle within `window` (the first of [`kept`], a lock of cargo's that is held ([`lock`],
/// released at once) and [`busy`]), and with its bytes when `measure`; the oldest last write
/// first. Before them what a gc that ended left moved aside, which nothing holds. When the
/// host's processes cannot be read, why: nothing is then known to be idle.
pub(super) fn assess(
    sources: &Sources,
    window: Duration,
    skip: &[PathBuf],
    measure: bool,
) -> Result<Vec<Candidate>, String> {
    let processes = processes()?;
    let mut candidates: Vec<Candidate> = Vec::new();
    for root in roots(sources) {
        for path in find(&root, &sources.checkouts) {
            if skip.iter().any(|skip| path.starts_with(skip))
                || candidates.iter().any(|known| known.path == path)
            {
                continue;
            }
            let written = newest(&path);
            let left = mover(&path).is_some();
            let held = if left {
                None
            } else {
                kept(&root, &path)
                    .or_else(|| lock(&path).err())
                    .or_else(|| busy(&path, window, &processes, &written))
            };
            candidates.push(Candidate {
                bytes: measure.then(|| allocated(&path)),
                root: root.clone(),
                path,
                written: written.ok(),
                held,
                left,
            });
        }
    }
    candidates.sort_by(|a, b| {
        b.left
            .cmp(&a.left)
            .then_with(|| a.written.cmp(&b.written))
            .then_with(|| a.path.cmp(&b.path))
    });
    Ok(candidates)
}

/// Removes the idle target `candidate`: [`kept`] is asked again, cargo's own lock is taken
/// ([`lock`]), and under it the host's processes are read anew and [`busy`] is asked again (a
/// cargo that waits for the lock has its lock file open, and holds the target). Still under the
/// lock the target is moved aside in its own directory ([`aside`]), and what was moved is
/// removed. The lock files go with the target, so the lock keeps no build out once they are
/// gone: a cargo that starts after the move finds no target, makes one of its own and loses
/// nothing to the removal. A target that cannot be moved stays. What a gc that ended left moved
/// aside (`candidate.left`) is removed as it is, when it still is that ([`left`]) and lies in no
/// primary checkout. The bytes freed; or why it stays, with the bytes a removal that failed
/// half-way freed: what is left of it then is this process's, for a later gc.
pub(super) fn clean(candidate: &Candidate, window: Duration) -> Result<u64, (String, u64)> {
    let path = &candidate.path;
    if candidate.left {
        if !left(path) || in_checkout(&candidate.root, path) {
            return Err(("it is no longer what a gc that ended left".to_owned(), 0));
        }
        let before = allocated(path);
        let removed = fs::remove_dir_all(path);
        let freed = before.saturating_sub(allocated(path));
        return match removed {
            Ok(()) => Ok(freed),
            Err(error) => Err((format!("remove {}: {error}", path.display()), freed)),
        };
    }
    if let Some(why) = kept(&candidate.root, path) {
        return Err((why, 0));
    }
    let locks = lock(path).map_err(|why| (why, 0))?;
    let processes = processes().map_err(|why| (why, 0))?;
    if let Some(why) = busy(path, window, &processes, &newest(path)) {
        return Err((why, 0));
    }
    let before = allocated(path);
    let aside = aside(path).map_err(|why| (why, 0))?;
    fs::rename(path, &aside).map_err(|error| {
        let why = format!("cannot move it aside to {}: {error}", aside.display());
        (why, 0)
    })?;
    let removed = fs::remove_dir_all(&aside);
    let freed = before.saturating_sub(allocated(&aside));
    drop(locks);
    match removed {
        Ok(()) => Ok(freed),
        Err(error) => Err((format!("remove {}: {error}", aside.display()), freed)),
    }
}

fn allocated(path: &Path) -> u64 {
    disk::tree_size(&[path.to_path_buf()], None).map_or(0, |size| size.allocated)
}
