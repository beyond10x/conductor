//! The specification holds what the repository says it holds (`AGENTS.md`, rules 1 and 8;
//! `docs/analysis/ess-audit-2026-11-07.md`).
//!
//! - Every scenario file under `spec/scenarios/` is listed in `spec/ess-inputs.yaml`. `ess
//!   specify validate` reads only the listed ones, so a file it does not list is never validated
//!   nor counted in its completeness, though the conformance step, which takes the directory,
//!   runs it: two were found so.
//! - Every error name a handler prints in a refusal (`<command>: refused: <Name>: …`,
//!   `<command>: <Name>: …`, or `refusal("<Name>", …)`) is an error the specification declares.
//!   A refused command names the specification's error (rule 8); a name it invents is a gap in
//!   the specification, as `StateDirNotInstance` was.
//!
//! Reads files only; it runs no binary and reads no instance.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn yaml(path: &Path) -> serde_yaml::Value {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("could not read {}: {error}", path.display()));
    serde_yaml::from_str(&text)
        .unwrap_or_else(|error| panic!("{} is no YAML: {error}", path.display()))
}

fn listed(inputs: &serde_yaml::Value, key: &str) -> BTreeSet<String> {
    inputs[key]
        .as_sequence()
        .unwrap_or_else(|| panic!("spec/ess-inputs.yaml has no `{key}:` list"))
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .unwrap_or_else(|| panic!("an entry of `{key}:` is not a path"))
                .to_owned()
        })
        .collect()
}

#[test]
fn every_scenario_file_is_listed_in_the_inputs() {
    let spec = root().join("spec");
    let inputs = yaml(&spec.join("ess-inputs.yaml"));
    let listed = listed(&inputs, "scenarios");
    let on_disk: BTreeSet<String> = fs::read_dir(spec.join("scenarios"))
        .expect("spec/scenarios is readable")
        .map(|entry| entry.expect("a directory entry").file_name())
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| name.ends_with(".yaml"))
        .map(|name| format!("scenarios/{name}"))
        .collect();
    let unlisted: Vec<_> = on_disk.difference(&listed).collect();
    let missing: Vec<_> = listed.difference(&on_disk).collect();
    assert!(
        unlisted.is_empty() && missing.is_empty(),
        "spec/ess-inputs.yaml `scenarios:` and spec/scenarios/ differ: on disk and never validated: \
         {unlisted:?}; listed and not on disk: {missing:?}"
    );
}

/// The short names of every error the specification declares, from each listed domain file's
/// `errors:` list.
fn declared_errors() -> BTreeSet<String> {
    let spec = root().join("spec");
    let inputs = yaml(&spec.join("ess-inputs.yaml"));
    let mut names = BTreeSet::new();
    for file in listed(&inputs, "specification") {
        let document = yaml(&spec.join(&file));
        for error in document["errors"].as_sequence().into_iter().flatten() {
            let name = error["name"]
                .as_str()
                .unwrap_or_else(|| panic!("an error of {file} has no name"));
            names.insert(name.rsplit('.').next().unwrap_or(name).to_owned());
        }
    }
    names
}

/// An error-shaped word: upper camel case with at least two humps, letters only.
fn error_shaped(word: &str) -> bool {
    let mut chars = word.chars();
    chars.next().is_some_and(|first| first.is_ascii_uppercase())
        && word.chars().all(|c| c.is_ascii_alphabetic())
        && word.chars().filter(char::is_ascii_uppercase).count() >= 2
        && word.chars().any(|c| c.is_ascii_lowercase())
}

/// The error names one line of handler source prints in a refusal.
fn refusal_names(line: &str) -> Vec<String> {
    let mut names = Vec::new();
    // `refusal("Name", …)` and `refused(…, "Name", …)` on one line.
    for marker in ["refusal(\"", "refuse(\""] {
        let mut rest = line;
        while let Some(at) = rest.find(marker) {
            rest = &rest[at + marker.len()..];
            let word: String = rest.chars().take_while(char::is_ascii_alphabetic).collect();
            if error_shaped(&word) && rest[word.len()..].starts_with('"') {
                names.push(word);
            }
        }
    }
    // `…: Name: …` inside a string literal: a word between `: ` and `: `.
    if line.contains('"') {
        let mut rest = line;
        while let Some(at) = rest.find(": ") {
            rest = &rest[at + 2..];
            let word: String = rest.chars().take_while(char::is_ascii_alphabetic).collect();
            if error_shaped(&word) && rest[word.len()..].starts_with(": ") {
                names.push(word);
            }
        }
    }
    names
}

fn rust_files(dir: &Path, into: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("a source directory is readable") {
        let path = entry.expect("a directory entry").path();
        if path.is_dir() {
            rust_files(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
}

#[test]
fn every_refusal_a_handler_prints_names_a_declared_error() {
    let declared = declared_errors();
    let source = root().join("crates/conductor/src");
    let mut files = Vec::new();
    rust_files(&source, &mut files);
    files.sort();
    let mut undeclared = Vec::new();
    for file in files {
        let text = fs::read_to_string(&file).expect("a source file is readable");
        for (number, line) in text.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for name in refusal_names(line) {
                if !declared.contains(&name) {
                    let shown = file.strip_prefix(&source).unwrap_or(&file).display();
                    undeclared.push(format!("crates/conductor/src/{shown}:{}: {name}", number + 1));
                }
            }
        }
    }
    assert!(
        undeclared.is_empty(),
        "a handler refuses under an error name spec/ does not declare (AGENTS.md rules 1 and 8): \
         declare it in the domain's `errors:`, or name the declared error:\n{}",
        undeclared.join("\n")
    );
}

#[test]
fn the_refusal_scan_reads_the_shapes_handlers_print() {
    assert_eq!(
        refusal_names(r#"bail!("{}: refused: StateDirNotInstance: --state-dir {} is not", x)"#),
        ["StateDirNotInstance"]
    );
    assert_eq!(
        refusal_names(r#"Err(reason) => return refusal("ChoiceInvalid", format!("{reason}")),"#),
        ["ChoiceInvalid"]
    );
    assert_eq!(
        refusal_names(r#""{COMMAND}: OptionsInvalid: {problem}; request {:?} is unchanged","#),
        ["OptionsInvalid"]
    );
    assert!(refusal_names(r#"bail!("{}: Error: nothing")"#).is_empty());
}
