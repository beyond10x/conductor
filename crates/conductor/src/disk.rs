//! The free and total bytes of the file system holding a path (`story:portable-free-space`).
//!
//! Every reader of free disk — `snapshot start-snapshot`, the dashboard and `watch run` — reads it
//! here, through `statvfs`, rather than through a `df` whose options differ between platforms.

use std::path::Path;

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
