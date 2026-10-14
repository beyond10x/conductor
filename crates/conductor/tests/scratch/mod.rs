//! A case's directory under this test target's temporary directory, removed when the case ends:
//! [`Scratch`]. A git repository a case leaves under `target/tmp` makes `worktree gc` keep the tree
//! that holds it; `tests/isolated_home.rs` fails on a test file that creates one in a directory
//! that nothing removes.

use std::fs;
use std::path::{Path, PathBuf};

/// A fresh, empty directory `relative` under `target/tmp`. A case that passed leaves nothing
/// behind: dropping it removes the directory. A failed one keeps it to read.
pub struct Scratch(PathBuf);

impl Scratch {
    /// The directory `relative` under `target/tmp`, emptied of what an earlier run left there.
    pub fn new(relative: impl AsRef<Path>) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(relative);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(&dir).expect("create the case's directory");
        Self(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
}
