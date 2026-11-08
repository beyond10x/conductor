//! The repositories' shared targets of `conductor build run` (`story:build-shared-target-dir`):
//! `<cache>/target/<key>`, each with its lock `<cache>/target/.locks/<key>.lock`
//! ([`crate::build::targets_root`]), which `build run` holds for the whole of a command that
//! builds into it. `conductor gc` takes the lock without waiting: a target whose lock it takes has
//! no build running, and it is removed under that lock, so no build starts into it meanwhile; a
//! target whose lock a run holds is kept. An entry with no lock file is not `build run`'s and is
//! left alone.

use std::fs::{self, File, OpenOptions, TryLockError};
use std::path::{Path, PathBuf};

use crate::build::{TARGET_LOCKS, orphaned_group};

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
