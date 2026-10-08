//! `story:portable-free-space`: the free and total bytes of the file system holding a path are
//! read through `statvfs` in one library function, [`conductor_cli::disk::space`], and no product
//! source spawns `df`, whose GNU options (`-B1`, `--output=`) another `df` (macOS's) refuses.
//!
//! The function is compared with a direct `statvfs` on a directory of this test target's
//! temporary directory: available is `f_bavail * f_frsize`, what an unprivileged writer may use,
//! and total `f_blocks * f_frsize`. The source check reads every file under `crates/*/src` for a
//! string literal that starts with the word `df`, the way `no_organization.rs` reads for words.

use std::fs;
use std::path::{Path, PathBuf};

use conductor_cli::disk;
use nix::sys::statvfs::statvfs;

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read a directory") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            files(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// A scratch directory of this test target.
fn scratch(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("free_space")
        .join(name);
    fs::create_dir_all(&dir).expect("create the scratch directory");
    dir
}

/// Available and total bytes of the file system holding `path`, by `statvfs` itself.
fn direct(path: &Path) -> (u64, u64) {
    let stat = statvfs(path).expect("statvfs the scratch directory");
    #[allow(clippy::useless_conversion)] // the field types differ between Linux and macOS
    let (available, blocks, unit) = (
        u64::from(stat.blocks_available()),
        u64::from(stat.blocks()),
        u64::from(stat.fragment_size()),
    );
    (available * unit, blocks * unit)
}

#[test]
fn no_product_source_spawns_df() {
    let root = workspace();
    let mut paths = Vec::new();
    for entry in fs::read_dir(root.join("crates")).expect("read crates/") {
        let src = entry.expect("a crate").path().join("src");
        if src.is_dir() {
            files(&src, &mut paths);
        }
    }
    assert!(!paths.is_empty(), "no source file was read");
    let mut offending = Vec::new();
    for path in paths {
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for (at, line) in text.lines().enumerate() {
            if line.contains("\"df\"") || line.contains("\"df ") {
                offending.push(format!(
                    "{}:{}: {}",
                    path.strip_prefix(&root).unwrap_or(&path).display(),
                    at + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(offending.is_empty(), "{offending:#?}");
}

#[test]
fn space_is_what_statvfs_reads_on_a_scratch_directory() {
    let dir = scratch("space");
    // Other processes write to the same file system, so a reading is taken between two direct
    // ones that agree; the totals never move.
    for _ in 0..50 {
        let before = direct(&dir);
        let space = disk::space(&dir).expect("read the free space of a scratch directory");
        let after = direct(&dir);
        assert_eq!(space.total, before.1, "total bytes");
        if before == after && space.available == before.0 {
            assert!(space.available <= space.total, "{space:?}");
            return;
        }
    }
    panic!(
        "available never matched f_bavail * f_frsize: {:?} against {:?}",
        disk::space(&dir).expect("read the free space"),
        direct(&dir)
    );
}

#[test]
fn space_of_a_missing_path_is_an_error_naming_it() {
    let missing = scratch("missing").join("not-there");
    let error = disk::space(&missing).expect_err("a missing path has no file system");
    let text = format!("{error:#}");
    assert!(text.contains(&missing.display().to_string()), "{text}");
    assert!(!text.contains("df"), "{text}");
}
