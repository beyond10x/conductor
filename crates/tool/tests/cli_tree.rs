//! The `tool` command tree holds the verbs of its `ess-cli/2` binding, `crates/tool/cli.yaml`,
//! and none of `conductor`'s operator commands (`story:tool-broker-read`, acceptance 7; design
//! § 6.1: "no admin command is compiled in"); and the crate reaches no store.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use clap::CommandFactory;
use tool::cli::Cli;

/// `conductor`'s operator commands, each a word that must not appear anywhere in `tool`'s tree.
const OPERATOR: [&str; 8] = [
    "init",
    "setup",
    "config",
    "trust",
    "upgrade",
    "doctor",
    "dashboard",
    "gc",
];

/// Every subcommand name of `command`, at any depth.
fn words(command: &clap::Command, out: &mut BTreeSet<String>) {
    for sub in command.get_subcommands() {
        out.insert(sub.get_name().to_owned());
        for alias in sub.get_all_aliases() {
            out.insert(alias.to_owned());
        }
        words(sub, out);
    }
}

#[test]
fn the_tree_holds_no_operator_command() {
    let mut found = BTreeSet::new();
    words(&Cli::command(), &mut found);

    let operator: Vec<&str> = OPERATOR
        .into_iter()
        .filter(|word| found.contains(*word))
        .collect();
    assert!(operator.is_empty(), "tool holds {operator:?}");
}

#[test]
fn the_tree_is_the_bindings_commands() {
    let binding: serde_json::Value = serde_yaml_lite(
        &fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("cli.yaml"))
            .expect("read crates/tool/cli.yaml"),
    );
    let bound: BTreeSet<String> = binding["commands"]
        .as_array()
        .expect("the binding lists commands")
        .iter()
        .map(|command| command["path"][0].as_str().expect("a path").to_owned())
        .collect();
    let tree: BTreeSet<String> = Cli::command()
        .get_subcommands()
        .map(|sub| sub.get_name().to_owned())
        .collect();

    assert_eq!(tree, bound);
    assert_eq!(Cli::command().get_name(), binding["binary"]);
}

#[test]
fn the_crate_reaches_no_store_and_no_conductor_command() {
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .expect("read crates/tool/Cargo.toml");
    for forbidden in ["conductor-cli", "eventlog", "conductor-model"] {
        assert!(
            !manifest.contains(forbidden),
            "crates/tool/Cargo.toml names {forbidden}"
        );
    }
}

/// The two keys this test reads from the binding, `binary` and each command's `path`, without a
/// YAML dependency: the binding writes them as `binary: <word>` and `- path: [<word>]`.
fn serde_yaml_lite(text: &str) -> serde_json::Value {
    let mut binary = String::new();
    let mut commands = Vec::new();
    for line in text.lines() {
        if let Some(word) = line.strip_prefix("binary: ") {
            binary = word.trim().to_owned();
        }
        if let Some(path) = line.trim().strip_prefix("- path: [") {
            let word = path.trim_end_matches(']').trim();
            commands.push(serde_json::json!({"path": [word]}));
        }
    }
    serde_json::json!({"binary": binary, "commands": commands})
}
