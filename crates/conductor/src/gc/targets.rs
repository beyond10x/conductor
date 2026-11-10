//! The repositories' shared targets of `conductor build run` (`story:build-shared-target-dir`):
//! `<cache>/target/<key>`, each with its lock `<cache>/target/.locks/<key>.lock`
//! ([`crate::build::targets_root`]), which `build run` holds for the whole of a command that
//! builds into it. `conductor gc` takes the lock without waiting: a target whose lock it takes has
//! no build running, and it is removed under that lock, so no build starts into it meanwhile; a
//! target whose lock a run holds is kept. An entry with no lock file is not `build run`'s and is
//! left alone.
//!
//! And every other cargo target under the instance's trees and cache roots
//! (`story:gc-idle-targets`): a directory holding `CACHEDIR.TAG` under the managed trees, the
//! shared targets' directory or the instance's scratch, never one inside a primary checkout (a
//! directory holding a `.git` directory, or anything under the checkouts root) and never one a
//! lock above answers for. [`assess`] lists each with its last write and why it is not idle; one
//! is idle when it holds no [`HOLD`] file, no process on the host has its working directory, its
//! program or an open file in it, names it in `CARGO_TARGET_DIR` or names a path in it on its
//! command line, and nothing in its top three levels was written for [`WINDOW`] ([`WINDOW_LOW`]
//! while less than [`LOW`] is free on `/`). [`clean`] makes every check again and removes an idle
//! one; `conductor gc` and the host reclaim clean them oldest first until free disk is at the
//! floor.

use std::ffi::OsStr;
use std::fs::{self, File, OpenOptions, TryLockError};
use std::io;
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Component, Path, PathBuf};
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

/// Whether `dir` is a cargo target: a directory, no link, holding the file [`TAG`].
fn is_cargo(dir: &Path) -> bool {
    is_dir(dir) && fs::symlink_metadata(dir.join(TAG)).is_ok_and(|meta| meta.is_file())
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

/// The cargo targets under `root`, at most [`DEPTH`] below it, found without following a link:
/// never `root` itself, nothing inside a primary checkout or under the checkouts root
/// `checkouts`, nothing inside another target, and nothing below a `.git` or `node_modules`.
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
    /// `CARGO_TARGET_DIR` of `/proc/<pid>/environ`, from its working directory when relative.
    target_dir: Option<PathBuf>,
    /// The words of `/proc/<pid>/cmdline`.
    words: Vec<String>,
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

/// Every process on the host; one whose links cannot be read (another user's, or one that ended)
/// holds nothing that is known. When `/proc` cannot be listed, why: no process can then be told
/// to be clear of a target.
fn processes() -> Result<Vec<Process>, String> {
    let entries = fs::read_dir("/proc").map_err(|error| format!("cannot list /proc: {error}"))?;
    let mut processes = Vec::new();
    for entry in entries.flatten() {
        let Some(pid) = entry
            .file_name()
            .to_str()
            .and_then(|name| name.parse::<u32>().ok())
        else {
            continue;
        };
        let dir = entry.path();
        let cwd = fs::read_link(dir.join("cwd")).ok();
        let from_cwd = |path: PathBuf| match &cwd {
            _ if path.is_absolute() => Some(normal(&path)),
            Some(cwd) => Some(normal(&cwd.join(path))),
            None => None,
        };
        let target_dir = fs::read(dir.join("environ")).ok().and_then(|environ| {
            environ
                .split(|byte| *byte == 0)
                .find_map(|variable| variable.strip_prefix(b"CARGO_TARGET_DIR="))
                .filter(|value| !value.is_empty())
                .and_then(|value| from_cwd(PathBuf::from(OsStr::from_bytes(value))))
        });
        let words = fs::read(dir.join("cmdline"))
            .map(|cmdline| {
                cmdline
                    .split(|byte| *byte == 0)
                    .filter(|word| !word.is_empty())
                    .map(|word| String::from_utf8_lossy(word).into_owned())
                    .collect()
            })
            .unwrap_or_default();
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
            cwd,
        });
    }
    Ok(processes)
}

/// Whether `word` names `target` or a path in it: it holds the target's path where a path ends or
/// goes on below it, or it is a relative path, alone or after a `=`, that leads into the target
/// from the working directory `cwd`.
fn names(word: &str, cwd: Option<&Path>, target: &Path) -> bool {
    let text = target.to_string_lossy();
    let written = word.match_indices(text.as_ref()).any(|(at, _)| {
        word[at + text.len()..]
            .chars()
            .next()
            .is_none_or(|next| !(next.is_alphanumeric() || matches!(next, '-' | '_' | '.')))
    });
    written
        || cwd.is_some_and(|cwd| {
            [Some(word), word.split_once('=').map(|(_, value)| value)]
                .into_iter()
                .flatten()
                .filter(|part| !part.is_empty() && !part.starts_with(['/', '-']))
                .any(|part| normal(&cwd.join(part)).starts_with(target))
        })
}

/// The first of `processes` that holds `target`, and how: its working directory, its program or
/// an open file lies in it, its `CARGO_TARGET_DIR` names it, or a word of its command line does.
fn holding(processes: &[Process], target: &Path) -> Option<String> {
    let mut paths = vec![target.to_path_buf()];
    let real = resolved(target);
    if real != target {
        paths.push(real);
    }
    let within = |path: &Path| paths.iter().any(|target| path.starts_with(target));
    processes.iter().find_map(|process| {
        let how = if process.cwd.as_deref().is_some_and(within) {
            "has its working directory in it"
        } else if process.exe.as_deref().is_some_and(within) {
            "runs a program in it"
        } else if process.open.iter().any(|open| within(open)) {
            "has a file open in it"
        } else if process.target_dir.as_deref().is_some_and(within) {
            "has CARGO_TARGET_DIR naming it"
        } else if process.words.iter().any(|word| {
            paths
                .iter()
                .any(|target| names(word, process.cwd.as_deref(), target))
        }) {
            "names it on its command line"
        } else {
            return None;
        };
        Some(format!("process {} {how}", process.pid))
    })
}

/// Why the cargo target `target`, found under `root`, is not idle; none when it is: it is still a
/// cargo target outside every primary checkout, it holds no [`HOLD`] file, it is this user's, none
/// of `processes` holds it ([`holding`]), and nothing in its top [`LEVELS`] levels was written
/// within `window`. Anything that cannot be read is a reason. With it, the last write.
fn held(
    root: &Path,
    target: &Path,
    window: Duration,
    processes: &[Process],
) -> (Option<SystemTime>, Option<String>) {
    let written = newest(target);
    let last = written.as_ref().ok().copied();
    let why = || -> Option<String> {
        if !is_cargo(target) {
            return Some(format!("it is no cargo target: no directory holding {TAG}"));
        }
        if !target.starts_with(root)
            || target
                .ancestors()
                .take_while(|dir| dir.starts_with(root))
                .any(primary)
        {
            return Some("it lies in a primary checkout".to_owned());
        }
        if fs::symlink_metadata(target.join(HOLD)).is_ok() {
            return Some(format!("it holds a {HOLD} file"));
        }
        // The owner of `/proc/self` is the effective user of this process.
        match (fs::metadata("/proc/self"), fs::symlink_metadata(target)) {
            (Ok(me), Ok(it)) if me.uid() == it.uid() => {}
            (Ok(_), Ok(_)) => return Some("it is another user's".to_owned()),
            _ => return Some("cannot tell whose it is".to_owned()),
        }
        if let Some(holder) = holding(processes, target) {
            return Some(holder);
        }
        let written = match &written {
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
    };
    (last, why())
}

/// Every cargo target under the instance's trees and cache roots ([`find`]) but those under a
/// path of `skip` (what another row of gc answers for), each with its last write and why it is
/// not idle within `window` ([`held`]), and with its bytes when `measure`; the oldest last write
/// first. When the host's processes cannot be read, why: nothing is then known to be idle.
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
            let (written, held) = held(&root, &path, window, &processes);
            candidates.push(Candidate {
                bytes: measure.then(|| allocated(&path)),
                root: root.clone(),
                path,
                written,
                held,
            });
        }
    }
    candidates.sort_by(|a, b| a.written.cmp(&b.written).then_with(|| a.path.cmp(&b.path)));
    Ok(candidates)
}

/// Removes the idle target `candidate`, once every check of [`held`] has passed again over the
/// host's processes read anew: the bytes freed; or why it stays, with the bytes a removal that
/// failed half-way freed.
pub(super) fn clean(candidate: &Candidate, window: Duration) -> Result<u64, (String, u64)> {
    let processes = processes().map_err(|why| (why, 0))?;
    if let (_, Some(why)) = held(&candidate.root, &candidate.path, window, &processes) {
        return Err((why, 0));
    }
    let before = allocated(&candidate.path);
    let removed = fs::remove_dir_all(&candidate.path);
    let freed = before.saturating_sub(allocated(&candidate.path));
    match removed {
        Ok(()) => Ok(freed),
        Err(error) => Err((
            format!("remove {}: {error}", candidate.path.display()),
            freed,
        )),
    }
}

fn allocated(path: &Path) -> u64 {
    disk::tree_size(&[path.to_path_buf()], None).map_or(0, |size| size.allocated)
}
