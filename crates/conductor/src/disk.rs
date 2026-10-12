//! The free and total bytes of the file system holding a path (`story:portable-free-space`), and
//! the bytes a tree takes (`story:portable-tree-sizes`).
//!
//! Every reader of free disk — `snapshot start-snapshot`, the dashboard and `watch run` — reads it
//! here, through `statvfs`, rather than through a `df` whose options differ between platforms.
//! Every reader of a tree's size — the dashboard's disk waste — walks it here ([`tree_size`]),
//! rather than through a `du` whose options differ between platforms.

use std::collections::HashSet;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use anyhow::{Context as _, Result};
use nix::sys::statvfs::statvfs;

/// The space of one file system, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Space {
    /// What an unprivileged writer may still use: `f_bavail * f_frsize`.
    pub available: u64,
    /// The file system's size: `f_blocks * f_frsize`.
    pub total: u64,
}

/// The [`Space`] of the file system holding `path`.
///
/// # Errors
///
/// `statvfs` fails on `path`: it does not exist, or a component of it cannot be searched.
pub fn space(path: &Path) -> Result<Space> {
    let stat =
        statvfs(path).with_context(|| format!("read the free space of {}", path.display()))?;
    // The field types are `u64` on Linux and narrower on macOS.
    #[allow(clippy::useless_conversion)]
    let (available, blocks, unit) = (
        u64::from(stat.blocks_available()),
        u64::from(stat.blocks()),
        u64::from(stat.fragment_size()),
    );
    Ok(Space {
        available: available.saturating_mul(unit),
        total: blocks.saturating_mul(unit),
    })
}

/// What [`tree_size`] counted under its paths.
#[derive(Debug, Default)]
pub struct TreeSize {
    /// The bytes the counted inodes take on disk: `st_blocks * 512` of each, what `du` counts by
    /// default. A sparse or compressed file counts the blocks it holds, not its length.
    pub allocated: u64,
    /// The entries that could not be read, in the order met. A path that cannot be `lstat`ed
    /// adds nothing; a directory that cannot be listed adds its own blocks and none under it.
    pub unreadable: Vec<Unreadable>,
}

/// An entry [`tree_size`] could not read, and why.
#[derive(Debug)]
pub struct Unreadable {
    /// The entry, or the directory whose listing broke off.
    pub path: PathBuf,
    pub error: io::Error,
}

/// [`tree_size`] reached its deadline before its walk ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimedOut;

/// The allocated bytes under `paths` together, walked in Rust with `lstat` alone, std only.
///
/// Each path and everything under it is counted: directories, files and symbolic links, each by
/// its own blocks. No link is followed, and the walk goes on into other file systems mounted
/// below a path, as `du` without `-x` does. Each inode is counted once: a file of several names,
/// a directory reached through two of `paths`, or a path named twice adds its blocks the first
/// time only. What cannot be read is named in [`TreeSize::unreadable`] and the walk goes on.
///
/// # Errors
///
/// [`TimedOut`] when `deadline` passes before the walk ends; with no deadline the walk always
/// ends with a [`TreeSize`].
pub fn tree_size(paths: &[PathBuf], deadline: Option<Instant>) -> Result<TreeSize, TimedOut> {
    // A file of one name is reached only through its directory, which is counted once, or as one
    // of `paths`; only directories, files of several names and the named paths need remembering.
    let named: HashSet<(u64, u64)> = paths
        .iter()
        .filter_map(|path| fs::symlink_metadata(path).ok())
        .map(|metadata| (metadata.dev(), metadata.ino()))
        .collect();
    let mut seen: HashSet<(u64, u64)> = HashSet::new();
    let mut size = TreeSize::default();
    let mut pending: Vec<PathBuf> = paths.iter().rev().cloned().collect();
    while let Some(path) = pending.pop() {
        if deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            return Err(TimedOut);
        }
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) => {
                size.unreadable.push(Unreadable { path, error });
                continue;
            }
        };
        let inode = (metadata.dev(), metadata.ino());
        let directory = metadata.is_dir();
        if (directory || metadata.nlink() > 1 || named.contains(&inode)) && !seen.insert(inode) {
            continue;
        }
        size.allocated = size
            .allocated
            .saturating_add(metadata.blocks().saturating_mul(512));
        if !directory {
            continue;
        }
        match fs::read_dir(&path) {
            Ok(entries) => {
                for entry in entries {
                    match entry {
                        Ok(entry) => pending.push(entry.path()),
                        Err(error) => {
                            size.unreadable.push(Unreadable { path, error });
                            break;
                        }
                    }
                }
            }
            Err(error) => size.unreadable.push(Unreadable { path, error }),
        }
    }
    Ok(size)
}
