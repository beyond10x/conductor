//! The product names no organization: the code, the specification and the tasks hold no
//! organization's name, so a second organization runs conductor from a config file alone.
//!
//! The words `beyond10x` and `b10x` are looked for, case-insensitively, in every file under
//! `crates/conductor/src`, `crates/conductor/tests`, `crates/conductor-model/src` and `spec`, and
//! in `Taskfile.yml` and `crates/conductor/Cargo.toml`. These uses are allowed, each by its exact
//! text:
//! - `.b10x/conductor`, `~/.b10x` and `".b10x"`, the directory of the config file and the default
//!   instance directories, and `.cache/b10x/conductor`, the default cache of an instance;
//! - `b10x-gates`, the name of the Gates scanner the guard and `task check` call, and
//!   `B10X_GATES_POLICY`, the variable that names its policy;
//! - `github.com/beyond10x/eventlog`, where the store's crates are published.
//!
//! `tests/fixtures/store/`, the eventlog-file store an earlier build wrote and the views that build
//! answered over it, is read like every other file: its repositories belong to `example-org`
//! (`tests/store_tree.rs` says how it was rewritten to that).

use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED: [&str; 8] = [
    ".b10x/conductor",
    ".b10x/./conductor",
    "~/.b10x",
    "\".b10x\"",
    ".cache/b10x/conductor",
    "b10x-gates",
    "b10x_gates_policy",
    "github.com/beyond10x/eventlog",
];

/// The files read for the check: everything under `dir`, but this file.
const SKIPPED: [&str; 1] = ["tests/no_organization.rs"];

const WORDS: [&str; 2] = ["beyond10x", "b10x"];

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

/// The lines of `text` that name an organization once the allowed uses are taken out.
fn offending(text: &str) -> Vec<(usize, String)> {
    text.lines()
        .enumerate()
        .filter_map(|(at, line)| {
            let mut lower = line.to_lowercase();
            for allowed in ALLOWED {
                lower = lower.replace(allowed, "");
            }
            WORDS
                .iter()
                .any(|word| lower.contains(word))
                .then(|| (at + 1, line.trim().to_owned()))
        })
        .collect()
}

#[test]
fn no_file_of_the_product_names_an_organization() {
    let root = workspace();
    let mut paths = Vec::new();
    for dir in [
        "crates/conductor/src",
        "crates/conductor/tests",
        "crates/conductor-model/src",
        "spec",
    ] {
        files(&root.join(dir), &mut paths);
    }
    paths.retain(|path| {
        let shown = path.to_string_lossy();
        !SKIPPED.iter().any(|skipped| shown.contains(skipped))
    });
    paths.push(root.join("Taskfile.yml"));
    paths.push(root.join("crates/conductor/Cargo.toml"));
    paths.sort();
    assert!(paths.len() > 20, "found only {} files", paths.len());

    let mut found = Vec::new();
    for path in &paths {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for (line, content) in offending(&text) {
            let shown = path.strip_prefix(&root).unwrap_or(path).display();
            found.push(format!("{shown}:{line}: {content}"));
        }
    }
    assert!(
        found.is_empty(),
        "an organization is named:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_check_finds_a_name_and_passes_the_allowed_uses() {
    assert_eq!(offending("owner: Beyond10x").len(), 1);
    assert_eq!(offending("trees/b10x/ess").len(), 1);
    assert!(offending("~/.b10x/conductor/conductor.yaml").is_empty());
    assert!(offending("~/.cache/b10x/conductor/work").is_empty());
    assert!(offending("b10x-gates scan-text").is_empty());
    assert_eq!(offending("b10x-gates and ~/b10x").len(), 1);
}
