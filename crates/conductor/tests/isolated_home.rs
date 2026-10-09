//! Every test that runs the built `conductor` binary, or reads the instance its own process runs,
//! is isolated from the real home: what the real `~/.b10x/conductor/conductor.yaml` holds, or
//! whether there is one, decides no case (`AGENTS.md`, rule 5; `story:tests-isolate-home`).
//!
//! - A test builds a command for the binary only through [`common::conductor`]
//!   (`tests/common/mod.rs`), which names the case's directory as `HOME` and removes
//!   `CONDUCTOR_CONFIG` and `CONDUCTOR_INSTANCE`. The scan below fails on a file under
//!   `crates/conductor/tests` that names the binary any other way.
//! - A test file that names a module of the library whose code reads the instance of the process
//!   (`config::active`) fixes that instance before anything reads it: through
//!   [`active::isolate`] (`tests/active/mod.rs`), or `config::set_active` with a config file of
//!   its own. A file that names such a module but reaches no reader is listed in
//!   [`REACHES_NO_READER`], with why.

mod active;
mod common;

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use conductor_cli::config::{self, CONFIG_VARIABLE, INSTANCE_VARIABLE};

/// The ways a test names the built binary: Cargo's variable for its path, and `assert_cmd`'s
/// lookup. Written in two halves so this file does not match itself.
const BINARY: [&str; 2] = [concat!("CARGO_BIN", "_EXE_"), concat!("cargo", "_bin")];

/// The one file that may name the binary: the shared helper.
const HELPER: &str = "common/mod.rs";

/// What fixes the instance of the process: the shared helper, or a file's own config.
const FIXES: [&str; 2] = ["active::isolate()", "config::set_active(Some("];

/// Files that name a module whose code reads the instance of the process, and reach no reader
/// of it: the guard decides through `guard::decide` and `guard::decide_in` over the places a case
/// gives it, and reads the instance only in the hook the binary runs.
const REACHES_NO_READER: [&str; 4] = [
    "adv_decision_authority.rs",
    "adv_guard.rs",
    "grouped_instance.rs",
    "guard_rules.rs",
];

fn crate_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Every `.rs` file under `dir`, at any depth, sorted.
fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut dirs = vec![dir.to_owned()];
    while let Some(dir) = dirs.pop() {
        for entry in fs::read_dir(&dir).expect("read a directory") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                dirs.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

fn relative(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .expect("a file under the base")
        .display()
        .to_string()
}

/// The modules of the library whose source reads the instance of the process: the first path
/// segment under `src/` of every file that calls `config::active()`.
fn reader_modules() -> Vec<String> {
    let src = crate_dir().join("src");
    let mut modules: Vec<String> = rust_files(&src)
        .into_iter()
        .filter(|path| {
            fs::read_to_string(path)
                .expect("read a source file")
                .contains("config::active()")
        })
        .map(|path| {
            let first = relative(&path, &src)
                .split('/')
                .next()
                .expect("a path segment")
                .to_owned();
            first.trim_end_matches(".rs").to_owned()
        })
        .collect();
    modules.sort();
    modules.dedup();
    modules
}

/// The modules of `conductor_cli` that `text` names: the first segment after each
/// `conductor_cli::`, and each item's first segment in a `conductor_cli::{…}` group.
fn named_modules(text: &str) -> Vec<String> {
    let ident = |rest: &str| -> String {
        rest.trim_start()
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
            .collect()
    };
    let mut modules = Vec::new();
    for (at, _) in text.match_indices("conductor_cli::") {
        let rest = &text[at + "conductor_cli::".len()..];
        if let Some(group) = rest.strip_prefix('{') {
            let end = group.find('}').unwrap_or(group.len());
            modules.extend(group[..end].split(',').map(ident));
        } else {
            modules.push(ident(rest));
        }
    }
    modules.retain(|module| !module.is_empty());
    modules.sort();
    modules.dedup();
    modules
}

#[test]
fn every_command_for_the_binary_is_built_by_the_shared_helper() {
    let tests = crate_dir().join("tests");
    let mut problems = Vec::new();
    let mut helper_names_it = false;
    for path in rust_files(&tests) {
        let name = relative(&path, &tests);
        let text = fs::read_to_string(&path).expect("read a test file");
        if name == HELPER {
            helper_names_it = BINARY.iter().any(|needle| text.contains(needle));
            continue;
        }
        for (at, line) in text.lines().enumerate() {
            if BINARY.iter().any(|needle| line.contains(needle)) {
                problems.push(format!("tests/{name}:{}: {}", at + 1, line.trim()));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "these lines name the conductor binary outside `common::conductor` \
         (tests/{HELPER}), which sets HOME to the case's directory and removes \
         CONDUCTOR_CONFIG and CONDUCTOR_INSTANCE:\n{}",
        problems.join("\n")
    );
    assert!(
        helper_names_it,
        "tests/{HELPER} builds the binary's command"
    );
}

#[test]
fn every_test_file_that_reads_the_active_instance_fixes_it_first() {
    let readers = reader_modules();
    assert!(
        ["collect", "dashboard", "snapshot", "watch"]
            .iter()
            .all(|module| readers.iter().any(|reader| reader == module)),
        "the source scan finds the modules known to read the active instance: {readers:?}"
    );
    let tests = crate_dir().join("tests");
    let mut problems = Vec::new();
    for path in rust_files(&tests) {
        let name = relative(&path, &tests);
        let text = fs::read_to_string(&path).expect("read a test file");
        let reached: Vec<String> = named_modules(&text)
            .into_iter()
            .filter(|module| readers.contains(module))
            .collect();
        let listed = REACHES_NO_READER.contains(&name.as_str());
        if listed && reached.is_empty() {
            problems.push(format!(
                "tests/{name} is listed as reaching no reader but names none of {readers:?}"
            ));
        }
        if !listed && !reached.is_empty() && !FIXES.iter().any(|fix| text.contains(fix)) {
            problems.push(format!(
                "tests/{name} names {reached:?}, whose code reads the active instance, and never \
                 fixes it (`active::isolate()`)"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// A fresh directory for case `name` under this test target's temporary directory.
fn case_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("isolated-home")
        .join(name);
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    fs::create_dir_all(&dir).expect("create the case's directory");
    dir
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The helper's command names the case's home as `HOME` and removes both variables, and the
/// binary it runs reads the config file of that home and of no other: a home holding a config
/// file U1 refuses (two instances, neither with a `session_prefix`) is refused, naming that file,
/// and an empty home runs on the built-in instance.
#[test]
fn the_helper_runs_the_binary_on_the_case_s_home_without_either_variable() {
    let dir = case_dir("helper");
    let refused = dir.join("refused");
    fs::create_dir_all(refused.join(".b10x/conductor")).expect("create the config's directory");
    let file = refused.join(".b10x/conductor/conductor.yaml");
    fs::write(
        &file,
        "version: conductor.config/1\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: /fixture-home/alpha, trees: /fixture-home/alpha-trees}\n\
         \x20 - name: beta\n\
         \x20   sources: [{github: example-org}]\n\
         \x20   checkouts: {root: /fixture-home/beta, trees: /fixture-home/beta-trees}\n",
    )
    .expect("write the refused config");
    let empty = dir.join("empty");
    fs::create_dir_all(&empty).expect("create the empty home");

    let command = common::conductor(&refused);
    let envs: BTreeMap<&OsStr, Option<&OsStr>> = command.get_envs().collect();
    assert_eq!(
        envs,
        BTreeMap::from([
            (OsStr::new("HOME"), Some(refused.as_os_str())),
            (OsStr::new(CONFIG_VARIABLE), None),
            (OsStr::new(INSTANCE_VARIABLE), None),
        ]),
        "the command sets HOME and removes both variables, and nothing else"
    );

    let show = |home: &Path| {
        common::conductor(home)
            .args(["config", "show", "--format", "json"])
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    };
    let output = show(&refused);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains(&*file.to_string_lossy()),
        "the refusal names the home's config file: {}",
        describe(&output)
    );
    let output = show(&empty);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stdout).contains(&*empty.to_string_lossy()),
        "the built-in instance lies under the empty home: {}",
        describe(&output)
    );
}

/// After [`active::isolate`], the instance of this process is the scratch one: read from its
/// file, named after `CONDUCTOR_INSTANCE` or the built-in owner, with its state, records and
/// cache under this binary's scratch directory and nothing under the real home.
#[test]
fn isolating_fixes_the_process_s_instance_to_the_scratch_file() {
    active::isolate();
    let active = config::active();
    assert!(
        active.from_file,
        "the instance is read from the scratch file"
    );
    assert!(
        active.others.is_empty(),
        "the scratch file holds one instance"
    );
    let scratch = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("isolated-active");
    for (what, path) in [
        ("state", &active.instance.state),
        ("records", &active.instance.records),
        ("cache", &active.instance.cache),
        ("checkouts root", &active.instance.checkouts.root),
        ("checkouts trees", &active.instance.checkouts.trees),
    ] {
        assert!(
            Path::new(path).starts_with(&scratch),
            "{what} {path} lies under the scratch directory {}",
            scratch.display()
        );
    }
    assert_eq!(active.instance.session_prefix, None);
    active::isolate();
}

#[test]
fn the_module_scan_reads_paths_and_groups() {
    assert_eq!(
        named_modules(
            "use conductor_cli::snapshot::{self, Taken};\n\
             use conductor_cli::{config, store};\n\
             conductor_cli::dashboard::Sources::new(a, b);\n\
             use conductor_cli::collect::{\n    Collector,\n};"
        ),
        ["collect", "config", "dashboard", "snapshot", "store"]
    );
}
