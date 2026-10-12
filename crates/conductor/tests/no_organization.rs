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
//! - `github.com/beyond10x/eventlog`, where the store's crates are published;
//! - `b10x.toml`, the file the ecosystem installer writes its CLI pins to, which
//!   `conductor doctor` reads (`story:install-prerequisites`).
//!
//! `tests/fixtures/store/`, the eventlog-file store an earlier build wrote and the views that build
//! answered over it, is read like every other file: its repositories belong to `example-org`
//! (`tests/store_tree.rs` says how it was rewritten to that).
//!
//! The public text names no instance either (`story:public-text-check`). Every tracked file under
//! `docs/`, `website/docs/` (the generated `reference/` and `status.md` left out), `README.md`,
//! `AGENTS.md` and `.agents/` is read, and the check fails, naming file, line and rule, on:
//! - an id of an instance's record: `DEC-`, `DSP-`, `N-` or `H-` and an eight-digit date, as a word
//!   of its own; a documented example is allowed inside a fenced code block of `website/docs/`;
//! - an absolute path into a home directory: a path token, at the start of a line or after
//!   whitespace, a quote, a backtick or `(`, that starts with `/home/<name>/`;
//! - a hand-over: a file named `handoff-*` under `docs/`.
//!
//! A finding names where it is, not what it says, so the failure does not repeat the leak.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const ALLOWED: [&str; 9] = [
    ".b10x/conductor",
    ".b10x/./conductor",
    "~/.b10x",
    "\".b10x\"",
    ".cache/b10x/conductor",
    "b10x-gates",
    "b10x_gates_policy",
    "github.com/beyond10x/eventlog",
    "b10x.toml",
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

/// The public text: the tracked files under these pathspecs.
const PUBLIC: [&str; 5] = ["docs", "website/docs", "README.md", "AGENTS.md", ".agents"];

/// The pages `conductor-docs generate` writes, left out of the public text.
const GENERATED: [&str; 2] = ["website/docs/reference/", "website/docs/status.md"];

/// The prefixes of the ids of an instance's records: decision, dispatch, need and hand-over.
const ID_PREFIXES: [&str; 4] = ["DEC", "DSP", "N", "H"];

/// The start of a path into a home directory.
const HOME: &str = "/home/";

/// The characters after which a path token starts, besides whitespace and the start of a line.
const PATH_STARTS: [char; 4] = ['`', '"', '\'', '('];

/// The rules of the public text, as a finding names them.
const INSTANCE_ID: &str = "an instance id (DEC-, DSP-, N- or H- and an eight-digit date)";
const HOME_PATH: &str = "an absolute path into a home directory; write ~/";
const HANDOFF: &str = "a hand-over file (handoff-*) under docs/";

/// The public text of the tree at `root`: its tracked files under [`PUBLIC`], relative to `root`.
fn tracked(root: &Path) -> Vec<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "-z", "--"])
        .args(PUBLIC)
        .output()
        .expect("run git ls-files");
    assert!(
        output.status.success(),
        "git ls-files: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("tracked paths in UTF-8")
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Whether the file at `path` is read: a generated page is not.
fn public(path: &str) -> bool {
    !GENERATED
        .iter()
        .any(|generated| path.starts_with(generated))
}

fn word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `line` holds an id of an instance's record, as a word of its own:
/// `\b(DEC|DSP|N|H)-20\d{6}\b`.
fn holds_instance_id(line: &str) -> bool {
    line.match_indices("-20").any(|(at, _)| {
        let date = &line[at + 1..];
        let dated = date.len() >= 8
            && date.as_bytes()[..8].iter().all(u8::is_ascii_digit)
            && !date[8..].starts_with(word);
        if !dated {
            return false;
        }
        let before = &line[..at];
        ID_PREFIXES
            .into_iter()
            .filter_map(|prefix| before.strip_suffix(prefix))
            .any(|rest| !rest.ends_with(word))
    })
}

/// Whether `line` holds a path token that starts with `/home/<name>/`.
fn holds_home_path(line: &str) -> bool {
    line.match_indices(HOME).any(|(at, _)| {
        let starts_token = line[..at]
            .chars()
            .next_back()
            .is_none_or(|c| c.is_whitespace() || PATH_STARTS.contains(&c));
        let name = line[at + HOME.len()..]
            .split_once('/')
            .map_or("", |(name, _)| name);
        starts_token && !name.is_empty() && !name.contains(char::is_whitespace)
    })
}

/// What the file at `path`, holding `text`, leaks: one finding a rule and line, naming the file,
/// the line and the rule.
fn leaks(path: &str, text: &str) -> Vec<String> {
    let mut found = Vec::new();
    let name = path.rsplit_once('/').map_or(path, |(_, name)| name);
    if path.starts_with("docs/") && name.starts_with("handoff-") {
        found.push(format!("{path}: {HANDOFF}"));
    }
    let examples = path.starts_with("website/docs/");
    let mut fenced = false;
    for (at, line) in text.lines().enumerate() {
        if holds_instance_id(line) && !(examples && fenced) {
            found.push(format!("{path}:{}: {INSTANCE_ID}", at + 1));
        }
        if holds_home_path(line) {
            found.push(format!("{path}:{}: {HOME_PATH}", at + 1));
        }
        let fence = line.trim_start();
        if fence.starts_with("```") || fence.starts_with("~~~") {
            fenced = !fenced;
        }
    }
    found
}

/// What `files`, pairs of a path relative to the root and its text, leak, the generated pages
/// left out.
fn public_leaks<P: AsRef<str>, T: AsRef<str>>(files: &[(P, T)]) -> Vec<String> {
    files
        .iter()
        .filter(|(path, _)| public(path.as_ref()))
        .flat_map(|(path, text)| leaks(path.as_ref(), text.as_ref()))
        .collect()
}

#[test]
fn no_public_text_names_an_instance() {
    let root = workspace();
    let files: Vec<(String, String)> = tracked(&root)
        .into_iter()
        .map(|path| {
            let text = fs::read_to_string(root.join(&path)).unwrap_or_default();
            (path, text)
        })
        .collect();
    assert!(files.len() > 20, "found only {} files", files.len());

    let found = public_leaks(&files);
    assert!(
        found.is_empty(),
        "the public text names an instance:\n{}",
        found.join("\n")
    );
}

#[test]
fn the_public_text_check_flags_each_leak() {
    // Written in two halves so the tests name no home directory (`AGENTS.md`, rule 4).
    let home = concat!("/ho", "me/example/");
    let from_home = format!("Run it from `{home}work`.\n");
    let planted = [
        (
            "docs/waves/wave-01.md",
            "# Wave 01\n\nAnswered in DEC-20261012-01.\n",
        ),
        ("AGENTS.md", from_home.as_str()),
        ("docs/waves/handoff-2026-10-12.md", "# Hand-over\n"),
        ("website/docs/status.md", "Answered in DEC-20261012-01.\n"),
        (
            "website/docs/guides/dashboard.md",
            "```json\n{\"id\":\"DEC-20261008-01\"}\n```\n",
        ),
        (
            "docs/analysis/sessions.md",
            "- `HOME=/workspace/home`; the run creates `/workspace/home/.claude/x.json`\n",
        ),
    ];
    assert_eq!(
        public_leaks(&planted),
        [
            format!("docs/waves/wave-01.md:3: {INSTANCE_ID}"),
            format!("AGENTS.md:1: {HOME_PATH}"),
            format!("docs/waves/handoff-2026-10-12.md: {HANDOFF}"),
        ]
    );

    for prefix in ID_PREFIXES {
        let line = format!("see {prefix}-20261012-01");
        assert!(holds_instance_id(&line), "{line}");
    }
    for line in [
        "XDEC-20261012",
        "DEC-2026101",
        "DEC-202610120",
        "DEC-19991012",
        "N-20261012x",
    ] {
        assert!(!holds_instance_id(line), "{line}");
    }
    let fenced = "Before DEC-20261008-01.\n\n```json\n{\"id\":\"DEC-20261008-01\"}\n```\n";
    assert_eq!(leaks("website/docs/guides/x.md", fenced).len(), 1);
    assert_eq!(leaks("docs/x.md", fenced).len(), 2);

    for before in ["", " ", "`", "\"", "'", "("] {
        let line = format!("{before}{home}work");
        assert!(holds_home_path(&line), "{line}");
    }
    for line in ["/workspace/home/.claude", "/home/", "/home/ x/", "~/work"] {
        assert!(!holds_home_path(line), "{line}");
    }
    assert!(!public("website/docs/reference/cli.md"));
    assert!(public("website/docs/guides/read-the-dashboard.md"));
}
