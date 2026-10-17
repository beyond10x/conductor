//! The public text of a public repository names no instance and no private repository.
//!
//! - `.engineering/` and `docs/`, every file, hold no id of an instance's record: `DEC-`, `N-`,
//!   `H-`, `DSP-` or `DR-`, an eight-digit date starting `20`, and a number of two or more digits,
//!   as a word of its own. The placeholder forms (a `<date>` and `<nn>` in place of the digits) are
//!   not ids and the check does not match them, so it allows nothing by name.
//! - The whole repository, but `target/`, `.git/`, `node_modules/` and `.check/`, holds no name of
//!   the private repository an earlier history lived in.
//!
//! A finding names file and line, not the text, so the failure does not repeat the leak. The
//! scans take a root, so a scratch tree under the target directory proves they can go red.

mod scratch;

use std::fs;
use std::path::{Path, PathBuf};

/// The directories of the public text that must carry no instance id.
const PLANNING_AND_DOCS: [&str; 2] = [".engineering", "docs"];

/// The prefixes of the ids of an instance's records.
const ID_PREFIXES: [&str; 5] = ["DEC", "N", "H", "DSP", "DR"];

/// Directories the whole-repository scan does not enter, by name.
const SKIPPED_DIRS: [&str; 4] = ["target", ".git", "node_modules", ".check"];

/// The private repository's name, written in two halves so this file does not hold it.
const PRIVATE_REPOSITORY: &str = concat!("conductor-", "archive");

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

/// Every file under `dir`, directories named in `skipped` and symbolic links left out.
fn files(dir: &Path, skipped: &[&str], out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries {
        let entry = entry.expect("a directory entry");
        let path = entry.path();
        let kind = entry.file_type().expect("a file type");
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            let name = entry.file_name();
            if !skipped.iter().any(|skip| name == *skip) {
                files(&path, skipped, out);
            }
        } else {
            out.push(path);
        }
    }
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `line` holds an id of an instance's record as a word of its own:
/// `\b(DEC|N|H|DSP|DR)-20[0-9]{6}-[0-9]{2,}\b`.
fn holds_instance_id(line: &str) -> bool {
    line.match_indices("-20").any(|(at, _)| {
        let rest = &line[at + 1..];
        let bytes = rest.as_bytes();
        if bytes.len() < 9 || !bytes[..8].iter().all(u8::is_ascii_digit) || bytes[8] != b'-' {
            return false;
        }
        let tail = &rest[9..];
        let digits = tail.bytes().take_while(u8::is_ascii_digit).count();
        if digits < 2 || tail[digits..].starts_with(word) {
            return false;
        }
        let before = &line[..at];
        ID_PREFIXES
            .into_iter()
            .filter_map(|prefix| before.strip_suffix(prefix))
            .any(|head| !head.ends_with(word))
    })
}

fn shown(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

/// `file:line` of every instance id under the planning and documentation directories of `root`.
fn instance_id_findings(root: &Path) -> (usize, Vec<String>) {
    let mut paths = Vec::new();
    for dir in PLANNING_AND_DOCS {
        files(&root.join(dir), &[], &mut paths);
    }
    paths.sort();
    let mut found = Vec::new();
    for path in &paths {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for (at, line) in text.lines().enumerate() {
            if holds_instance_id(line) {
                found.push(format!("{}:{}", shown(root, path), at + 1));
            }
        }
    }
    (paths.len(), found)
}

/// `file:line` of every line under `root` that names the private repository.
fn private_repository_findings(root: &Path) -> (usize, Vec<String>) {
    let mut paths = Vec::new();
    files(root, &SKIPPED_DIRS, &mut paths);
    paths.sort();
    let mut found = Vec::new();
    for path in &paths {
        let Ok(text) = fs::read_to_string(path) else {
            continue;
        };
        for (at, line) in text.lines().enumerate() {
            if line.contains(PRIVATE_REPOSITORY) {
                found.push(format!("{}:{}", shown(root, path), at + 1));
            }
        }
    }
    (paths.len(), found)
}

#[test]
fn planning_and_docs_carry_no_instance_id() {
    let (read, found) = instance_id_findings(&workspace());
    assert!(read > 20, "found only {read} files");
    assert!(
        found.is_empty(),
        "planning and docs name an instance id:\n{}",
        found.join("\n")
    );
}

#[test]
fn committed_text_names_no_private_repository() {
    let (read, found) = private_repository_findings(&workspace());
    assert!(read > 100, "found only {read} files");
    assert!(
        found.is_empty(),
        "a file names a private repository:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_scans_go_red_on_a_planted_tree_and_pass_the_placeholders() {
    let case = scratch::Scratch::new("public_text");
    let scratch = case.path().to_path_buf();
    for dir in [".engineering/planning", "docs", "target", "node_modules"] {
        fs::create_dir_all(scratch.join(dir)).expect("create a scratch directory");
    }
    let id = format!("{}-{}-{}", "DEC", "20261012", "01");
    let private = format!("see {PRIVATE_REPOSITORY}\n");
    for (path, text) in [
        (
            ".engineering/planning/a.md",
            format!("ok\nanswered in {id}\n"),
        ),
        ("docs/b.md", "DEC-<date>-<nn>, N-<date>-<nn>\n".to_owned()),
        ("docs/c.md", private.clone()),
        ("target/d.md", private.clone()),
        ("node_modules/e.md", private),
    ] {
        fs::write(scratch.join(path), text).expect("write a scratch file");
    }

    let (read, found) = instance_id_findings(&scratch);
    assert_eq!(read, 3);
    assert_eq!(found, [".engineering/planning/a.md:2"]);

    let (_, found) = private_repository_findings(&scratch);
    assert_eq!(found, ["docs/c.md:1"]);

    for prefix in ID_PREFIXES {
        let line = format!("see {prefix}-20261012-01.");
        assert!(holds_instance_id(&line), "{line}");
    }
    for line in [
        "XDEC-20261012-01",
        "DEC-2026101-01",
        "DEC-20261012-1",
        "DEC-20261012-01x",
        "DEC-20261012",
        "DEC-19991012-01",
        "DEC-<date>-<nn>",
    ] {
        assert!(!holds_instance_id(line), "{line}");
    }
    fs::remove_dir_all(&scratch).expect("remove the scratch tree");
}
