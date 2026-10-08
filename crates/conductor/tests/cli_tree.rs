//! The `conductor` derive tree is exactly the `cli:` block of `spec/components.yaml`.
//!
//! The expected tree is read from `ess specify compile --path spec --format json` on every run,
//! never from a copy kept in this file, so a placement changed in the specification fails here
//! until `src/cli.rs` follows it. `ess` is taken from `PATH`.
//!
//! The words the tree may hold, and nothing else:
//! - one group per `cli:` group, in the block's order, with the group's summary as its help;
//! - in each group, one subcommand per placed command and view, named by its `naming.wire`, with
//!   its `naming.display` as its help, in the compiled block's order (commands, then views, each
//!   by qualified name);
//! - on each command, one flag per input field (`naming.wire`, or the field name in kebab case),
//!   accepting what the field's type holds and refusing what it does not, plus `--input-json -`;
//! - on `conductor.dispatch.RecordGuardDecision`, also `--from-pre-tool-use`;
//! - on each view, `--format text|json|jsonl|markdown`;
//! - everywhere, the state-directory flag and the config-file flag the `ess-cli/1` binding
//!   `crates/conductor/cli.yaml` names in `globals.state` and `globals.config`, read from
//!   `ess specify cli --path spec --binding crates/conductor/cli.yaml --format json` on every
//!   run;
//! - besides the `cli:` block, each command the binding declares over a `local` callable
//!   (`story:live-dashboard`), at its binding path, with its `about` as help and exactly the
//!   option flags the binding maps its input fields to. Its group is the block's group of that
//!   name, or a new one after the block's groups, with no help, when the block has none.
//!
//! No other word parses: no catch-all, no inferred prefix, no `help` word, no positional. The
//! binding's `globals` must also name an output flag (`ess-cli/1` refuses a binding without
//! one); the tree does not present it, and the flag comparison below refuses it.

use std::collections::BTreeSet;
use std::io::ErrorKind as IoErrorKind;
use std::path::PathBuf;
use std::process::Command as Process;

use clap::error::ErrorKind;
use clap::{Command, CommandFactory, Parser};
use conductor_cli::cli::Cli;
use serde_json::Value;

/// The renderings every view offers, in the order its help lists them.
const FORMATS: [&str; 4] = ["text", "json", "jsonl", "markdown"];

/// The one command that also reads a Claude Code PreToolUse payload (`story:guard-hook`).
const PRE_TOOL_USE_COMMAND: &str = "conductor.dispatch.RecordGuardDecision";

/// What a flag accepts, read from its field's type.
#[derive(Debug, Clone)]
enum Accepts {
    /// `String`: any text.
    Text,
    Integer,
    Boolean,
    /// RFC 3339, the wire rendering of `Timestamp`.
    Timestamp,
    /// 8-4-4-4-12 hexadecimal digits, the wire rendering of `Uuid`.
    Uuid,
    /// A `String` newtype that declares `prefix:`.
    Prefixed(String),
    OneOf(Vec<String>),
}

#[derive(Debug, Clone)]
struct Field {
    flag: String,
    accepts: Accepts,
}

/// An option flag of a binding command: its long name, and the kind of value its input field
/// holds (`integer`, `string`, `boolean`, …) as the binding's resolved input shape gives it.
#[derive(Debug, Clone)]
struct LocalFlag {
    long: String,
    holds: String,
}

#[derive(Debug, Clone)]
enum Kind {
    Command {
        fields: Vec<Field>,
        pre_tool_use: bool,
    },
    View,
    /// A command the binding declares over a `local` callable outside the `cli:` block.
    Local {
        flags: Vec<LocalFlag>,
    },
}

#[derive(Debug, Clone)]
struct Leaf {
    qualified: String,
    wire: String,
    display: Option<String>,
    kind: Kind,
}

#[derive(Debug, Clone)]
struct Group {
    name: String,
    summary: Option<String>,
    leaves: Vec<Leaf>,
}

#[derive(Debug)]
struct Spec {
    binary: String,
    groups: Vec<Group>,
}

fn spec_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec")
}

/// The `ess-cli/1` presentation binding, outside the model directory `spec/`.
fn binding_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("cli.yaml")
}

/// Runs `ess specify <verb> --path spec <extra> --format json` and answers the JSON it prints.
fn ess_specify(verb: &str, extra: &[&std::ffi::OsStr]) -> Value {
    let spec = spec_dir();
    let output = Process::new("ess")
        .args(["specify", verb, "--path"])
        .arg(&spec)
        .args(extra)
        .args(["--format", "json"])
        .output()
        .unwrap_or_else(|error| {
            if error.kind() == IoErrorKind::NotFound {
                panic!(
                    "`ess` is not on PATH. This test compares the derive tree with the `cli:` \
                     block printed by `ess specify compile --path spec --format json` and with \
                     the binding `ess specify cli` validates; install ESS (the version spec/ess-inputs.yaml requires) \
                     and rerun."
                );
            }
            panic!("could not run `ess specify {verb}`: {error}");
        });
    assert!(
        output.status.success(),
        "`ess specify {verb} --path {} {extra:?}` exited {}:\n{}",
        spec.display(),
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("`ess specify {verb} --format json` printed no JSON: {error}")
    })
}

fn compile() -> Value {
    ess_specify("compile", &[])
}

/// What the binding presents, as `ess specify cli` validated and resolved it.
#[derive(Debug)]
struct Binding {
    binary: String,
    /// `globals.state`: the long name of the state-directory flag.
    state: String,
    /// `globals.config`: the long name of the config-file flag (`story:config-file`).
    config: String,
    commands: Vec<BindingCommand>,
}

/// One command of the binding.
#[derive(Debug)]
struct BindingCommand {
    path: Vec<String>,
    about: Option<String>,
    /// The kind of its callable's target: `local`, `service_forward` or `dynamic`.
    target: String,
    aliases: usize,
    /// Its arguments with an `option` source, as flags.
    flags: Vec<LocalFlag>,
    /// Its arguments with any other source, described; this test maps none of them.
    unmapped: Vec<String>,
}

fn read_binding() -> Binding {
    let file = binding_file();
    let plan = ess_specify("cli", &["--binding".as_ref(), file.as_os_str()]);
    let commands = list(&plan["commands"])
        .iter()
        .map(|command| {
            let path: Vec<String> = list(&command["path"])
                .iter()
                .map(|token| text(token, "a binding command's path").to_owned())
                .collect();
            let callable =
                &plan["callables"][text(&command["callable"], "a binding command's callable")];
            let fields = &callable["input"]["shape"]["fields"];
            let mut flags = Vec::new();
            let mut unmapped = Vec::new();
            for argument in list(&command["arguments"]) {
                let field = text(&argument["field"], "a binding argument's field");
                let source = &argument["source"];
                match (source["kind"].as_str(), source["long"].as_str()) {
                    (Some("option"), Some(long)) => flags.push(LocalFlag {
                        long: long.to_owned(),
                        holds: holds(&fields[field]),
                    }),
                    _ => unmapped.push(format!("field `{field}` from {source}")),
                }
            }
            BindingCommand {
                about: command["about"].as_str().map(str::to_owned),
                target: text(
                    &callable["target"]["kind"],
                    "a binding callable's target kind",
                )
                .to_owned(),
                aliases: list(&command["aliases"]).len(),
                flags,
                unmapped,
                path,
            }
        })
        .collect();
    Binding {
        binary: text(&plan["binary"], "the binding's binary").to_owned(),
        state: text(&plan["globals"]["state"], "the binding's globals.state").to_owned(),
        config: text(&plan["globals"]["config"], "the binding's globals.config").to_owned(),
        commands,
    }
}

/// The kind of value a resolved input shape holds, through `optional`.
fn holds(shape: &Value) -> String {
    match shape["kind"].as_str() {
        Some("optional") => holds(&shape["of"]),
        Some(kind) => kind.to_owned(),
        None => format!("an unresolved shape {shape}"),
    }
}

/// Whether the `cli:` block places a leaf at `path`.
fn placed(spec: &Spec, path: &[String]) -> bool {
    match path {
        [group, wire] => spec
            .groups
            .iter()
            .filter(|placed| placed.name == *group)
            .flat_map(|placed| &placed.leaves)
            .any(|leaf| leaf.wire == *wire),
        _ => false,
    }
}

/// The groups the derive tree presents: the `cli:` block's, then each two-word command the
/// binding declares over a `local` callable outside the block, in the binding's order. Such a
/// command joins the block's group of its first word, or opens a new group, with no help, after
/// the block's. `the_binding_presents_only_placed_leaves_and_local_commands` reports every other
/// binding command outside the block.
fn presented(spec: &Spec, binding: &Binding) -> Vec<Group> {
    let mut groups = spec.groups.clone();
    for command in &binding.commands {
        let [group, wire] = command.path.as_slice() else {
            continue;
        };
        if command.target != "local" || placed(spec, &command.path) {
            continue;
        }
        let leaf = Leaf {
            qualified: format!("binding command `{}`", command.path.join(" ")),
            wire: wire.clone(),
            display: command.about.clone(),
            kind: Kind::Local {
                flags: command.flags.clone(),
            },
        };
        match groups.iter_mut().find(|known| known.name == *group) {
            Some(known) => known.leaves.push(leaf),
            None => groups.push(Group {
                name: group.clone(),
                summary: None,
                leaves: vec![leaf],
            }),
        }
    }
    groups
}

fn text<'a>(value: &'a Value, what: &str) -> &'a str {
    value
        .as_str()
        .unwrap_or_else(|| panic!("the compiled specification has no string for {what}"))
}

fn list(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or_default()
}

fn accepts(model: &Value, type_ref: &Value, context: &str) -> Accepts {
    match type_ref["kind"].as_str() {
        Some("optional") => accepts(model, &type_ref["of"], context),
        Some("primitive") => match text(&type_ref["name"], context) {
            "string" => Accepts::Text,
            "integer" => Accepts::Integer,
            "boolean" => Accepts::Boolean,
            "timestamp" => Accepts::Timestamp,
            "uuid" => Accepts::Uuid,
            other => panic!(
                "{context}: primitive `{other}` has a wire grammar this test does not check; give \
                 its flag a value parser and teach this test before placing such a field"
            ),
        },
        Some("declared") => {
            let name = text(&type_ref["name"], context);
            let body = &model["types"][name]["body"];
            match body["kind"].as_str() {
                Some("newtype") => {
                    assert!(
                        list(&body["invariants"]).is_empty(),
                        "{context}: newtype `{name}` declares invariants, which no flag checks at \
                         parse time; decide where they are enforced and teach this test"
                    );
                    match body["prefix"].as_str() {
                        Some(prefix) => {
                            assert!(
                                body["of"]["name"].as_str() == Some("string"),
                                "{context}: newtype `{name}` declares `prefix:` over {}, which \
                                 this test does not map onto a flag",
                                body["of"]
                            );
                            Accepts::Prefixed(prefix.to_owned())
                        }
                        None => accepts(model, &body["of"], context),
                    }
                }
                Some("enum") => Accepts::OneOf(
                    list(&body["variants"])
                        .iter()
                        .map(|variant| text(variant, context).to_owned())
                        .collect(),
                ),
                other => panic!(
                    "{context}: type `{name}` has body kind {other:?}, which this test does not \
                     map onto a flag; teach it before placing such a field"
                ),
            }
        }
        other => panic!(
            "{context}: type reference kind {other:?} is not mapped onto a flag; teach this test \
             before placing such a field"
        ),
    }
}

fn read_spec() -> Spec {
    let model = compile();
    let components = model["components"]
        .as_object()
        .expect("the compiled specification has a `components` object");
    let mut clis = components
        .values()
        .filter(|component| !component["cli"].is_null());
    let component = clis
        .next()
        .expect("no component of the specification declares a `cli:` block");
    assert!(
        clis.next().is_none(),
        "more than one component declares a `cli:` block; this test reads exactly one"
    );
    let cli = &component["cli"];
    let mut groups = Vec::new();
    for group in list(&cli["groups"]) {
        let name = text(&group["name"], "a cli group's name").to_owned();
        let summary = group["summary"].as_str().map(str::to_owned);
        let mut leaves = Vec::new();
        for qualified in list(&group["commands"]) {
            let qualified = text(qualified, "a placed command").to_owned();
            let command = &model["commands"][&qualified];
            assert!(
                command.is_object(),
                "group `{name}` places `{qualified}`, which the specification does not declare"
            );
            let context = format!("command `{qualified}`");
            let fields = list(&command["input"])
                .iter()
                .map(|field| {
                    let field_name = text(&field["name"], &context);
                    let flag = field["naming"]["wire"]
                        .as_str()
                        .map_or_else(|| field_name.replace('_', "-"), str::to_owned);
                    let context = format!("{context}, field `{field_name}`");
                    Field {
                        flag,
                        accepts: accepts(&model, &field["type_ref"], &context),
                    }
                })
                .collect();
            leaves.push(Leaf {
                wire: text(
                    &command["naming"]["wire"],
                    &format!("{context} naming.wire"),
                )
                .to_owned(),
                display: command["naming"]["display"].as_str().map(str::to_owned),
                kind: Kind::Command {
                    fields,
                    pre_tool_use: qualified == PRE_TOOL_USE_COMMAND,
                },
                qualified,
            });
        }
        for qualified in list(&group["views"]) {
            let qualified = text(qualified, "a placed view").to_owned();
            let view = &model["views"][&qualified];
            assert!(
                view.is_object(),
                "group `{name}` places view `{qualified}`, which the specification does not declare"
            );
            leaves.push(Leaf {
                wire: text(
                    &view["naming"]["wire"],
                    &format!("view `{qualified}` naming.wire"),
                )
                .to_owned(),
                display: view["naming"]["display"].as_str().map(str::to_owned),
                kind: Kind::View,
                qualified,
            });
        }
        groups.push(Group {
            name,
            summary,
            leaves,
        });
    }
    Spec {
        binary: text(&cli["binary"], "the cli binary").to_owned(),
        groups,
    }
}

/// The derive tree as clap will run it: `build` adds what clap adds on its own, such as a `help`
/// subcommand, so an extra word clap would add is visible here too.
fn derive_tree() -> Command {
    let mut root = Cli::command();
    root.build();
    root
}

fn compare_names(
    path: &str,
    word: &str,
    expected: &[&str],
    actual: &[&str],
    problems: &mut Vec<String>,
) {
    let mut seen = BTreeSet::new();
    for name in expected {
        if !seen.insert(*name) {
            problems.push(format!(
                "`{path}`: the cli block places two {word}s named `{name}`"
            ));
        }
    }
    let actual_set: BTreeSet<&str> = actual.iter().copied().collect();
    for name in seen.difference(&actual_set) {
        problems.push(format!("`{path}`: {word} `{name}` is missing"));
    }
    for name in actual_set.difference(&seen) {
        problems.push(format!(
            "`{path}`: {word} `{name}` is not in the cli block (extra or renamed)"
        ));
    }
}

/// The names both sides hold must come in the order the compiled block gives. Missing and extra
/// names are `compare_names`'s to report, so they are left out here.
fn compare_order(
    path: &str,
    words: &str,
    expected: &[&str],
    actual: &[&str],
    problems: &mut Vec<String>,
) {
    let actual_known: Vec<&str> = actual
        .iter()
        .copied()
        .filter(|name| expected.contains(name))
        .collect();
    let expected_known: Vec<&str> = expected
        .iter()
        .copied()
        .filter(|name| actual.contains(name))
        .collect();
    if actual_known != expected_known {
        problems.push(format!(
            "`{path}`: {words} are ordered {actual_known:?}; the cli block orders them \
             {expected_known:?}"
        ));
    }
}

/// Every word on `command` besides its subcommands: long flags, plus a problem for anything
/// that is not a long flag (a positional, a short flag, an alias).
fn flag_words(path: &str, command: &Command, problems: &mut Vec<String>) -> BTreeSet<String> {
    let mut longs = BTreeSet::new();
    for arg in command.get_arguments() {
        if arg.get_id() == "help" {
            continue;
        }
        match arg.get_long() {
            Some(long) => {
                longs.insert(long.to_owned());
            }
            None => problems.push(format!(
                "`{path}`: argument `{}` is not a long flag; the cli block declares none",
                arg.get_id()
            )),
        }
        if let Some(short) = arg.get_short() {
            problems.push(format!(
                "`{path}`: short flag `-{short}` is a word the cli block does not declare"
            ));
        }
        if let Some(aliases) = arg.get_all_aliases() {
            problems.push(format!(
                "`{path}`: aliases {aliases:?} on `--{}` are words the cli block does not declare",
                arg.get_id()
            ));
        }
    }
    longs
}

fn compare_flags(
    path: &str,
    expected: &BTreeSet<String>,
    actual: &BTreeSet<String>,
    problems: &mut Vec<String>,
) {
    for flag in expected.difference(actual) {
        problems.push(format!("`{path}`: flag `--{flag}` is missing"));
    }
    for flag in actual.difference(expected) {
        problems.push(format!(
            "`{path}`: flag `--{flag}` is not an input field of the command (extra or renamed)"
        ));
    }
}

fn check_about(path: &str, command: &Command, expected: Option<&str>, problems: &mut Vec<String>) {
    let actual = command.get_about().map(ToString::to_string);
    if actual.as_deref() != expected {
        problems.push(format!(
            "`{path}`: help reads {actual:?}, the specification says {expected:?}"
        ));
    }
}

fn fail(problems: Vec<String>, what: &str) {
    assert!(
        problems.is_empty(),
        "{what} ({} problem(s)):\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

#[test]
fn derive_tree_is_the_cli_block() {
    let spec = read_spec();
    let binding = read_binding();
    let groups = presented(&spec, &binding);
    let root = derive_tree();
    let mut problems = Vec::new();
    // The words the tree takes on every level: the binding's state-directory and config-file
    // flags.
    let global = BTreeSet::from([binding.state.clone(), binding.config.clone()]);

    if root.get_name() != spec.binary {
        problems.push(format!(
            "the binary is `{}`, the cli block names `{}`",
            root.get_name(),
            spec.binary
        ));
    }
    let root_flags = flag_words(&spec.binary, &root, &mut problems);
    compare_flags(&spec.binary, &global, &root_flags, &mut problems);
    for flag in [&binding.state, &binding.config] {
        match root
            .get_arguments()
            .find(|arg| arg.get_long() == Some(flag.as_str()))
        {
            Some(arg) if arg.is_global_set() => {}
            Some(_) => problems.push(format!(
                "`--{flag}` is not global, so it is not taken after a subcommand"
            )),
            None => {}
        }
    }

    let expected_groups: Vec<&str> = groups.iter().map(|group| group.name.as_str()).collect();
    let actual_groups: Vec<&str> = root.get_subcommands().map(Command::get_name).collect();
    compare_names(
        &spec.binary,
        "group",
        &expected_groups,
        &actual_groups,
        &mut problems,
    );
    compare_order(
        &spec.binary,
        "groups",
        &expected_groups,
        &actual_groups,
        &mut problems,
    );
    for subcommand in root.get_subcommands() {
        if subcommand.get_all_aliases().next().is_some() {
            problems.push(format!(
                "group `{}` has aliases, which are words the cli block does not declare",
                subcommand.get_name()
            ));
        }
    }

    for group in &groups {
        let Some(actual) = root.find_subcommand(&group.name) else {
            continue;
        };
        let path = format!("{} {}", spec.binary, group.name);
        check_about(&path, actual, group.summary.as_deref(), &mut problems);
        let group_flags = flag_words(&path, actual, &mut problems);
        compare_flags(&path, &global, &group_flags, &mut problems);

        let expected_leaves: Vec<&str> =
            group.leaves.iter().map(|leaf| leaf.wire.as_str()).collect();
        let actual_leaves: Vec<&str> = actual.get_subcommands().map(Command::get_name).collect();
        compare_names(
            &path,
            "subcommand",
            &expected_leaves,
            &actual_leaves,
            &mut problems,
        );
        compare_order(
            &path,
            "subcommands",
            &expected_leaves,
            &actual_leaves,
            &mut problems,
        );

        for leaf in &group.leaves {
            let Some(command) = actual.find_subcommand(&leaf.wire) else {
                continue;
            };
            let path = format!("{path} {}", leaf.wire);
            check_about(&path, command, leaf.display.as_deref(), &mut problems);
            if command.get_all_aliases().next().is_some() {
                problems.push(format!(
                    "`{path}` ({}) has aliases, which are words the cli block does not declare",
                    leaf.qualified
                ));
            }
            for nested in command.get_subcommands() {
                problems.push(format!(
                    "`{path}` ({}) has subcommand `{}`; a placed command or view has none",
                    leaf.qualified,
                    nested.get_name()
                ));
            }
            let mut expected = global.clone();
            match &leaf.kind {
                Kind::Command {
                    fields,
                    pre_tool_use,
                } => {
                    if !expected.insert("input-json".to_owned()) {
                        problems.push(format!(
                            "`{path}`: `--input-json` collides with the binding's `--{}`",
                            binding.state
                        ));
                    }
                    if *pre_tool_use {
                        expected.insert("from-pre-tool-use".to_owned());
                    }
                    for field in fields {
                        if !expected.insert(field.flag.clone()) {
                            problems.push(format!(
                                "`{path}`: input field flag `--{}` collides with another word",
                                field.flag
                            ));
                        }
                    }
                }
                Kind::View => {
                    if !expected.insert("format".to_owned()) {
                        problems.push(format!(
                            "`{path}`: `--format` collides with the binding's `--{}`",
                            binding.state
                        ));
                    }
                }
                Kind::Local { flags } => {
                    for flag in flags {
                        if !expected.insert(flag.long.clone()) {
                            problems.push(format!(
                                "`{path}`: binding flag `--{}` collides with another word",
                                flag.long
                            ));
                        }
                    }
                }
            }
            let actual_flags = flag_words(&path, command, &mut problems);
            compare_flags(&path, &expected, &actual_flags, &mut problems);
        }
    }

    fail(
        problems,
        "the derive tree in src/cli.rs differs from the `cli:` block of spec/components.yaml",
    );
}

/// One command line: the leaf's words, then `extra`.
fn argv<'a>(base: &[&'a str], extra: &[&'a str]) -> Vec<&'a str> {
    base.iter().chain(extra).copied().collect()
}

/// Whether `argv` parses; on refusal, whether clap refused the value rather than the word.
fn parses(argv: &[&str]) -> Result<(), ErrorKind> {
    Cli::try_parse_from(argv)
        .map(drop)
        .map_err(|error| error.kind())
}

fn expect_accepted(argv: &[&str], problems: &mut Vec<String>) {
    if let Err(kind) = parses(argv) {
        problems.push(format!("`{}` is refused ({kind:?})", argv.join(" ")));
    }
}

fn expect_value_refused(argv: &[&str], problems: &mut Vec<String>) {
    match parses(argv) {
        Err(ErrorKind::InvalidValue | ErrorKind::ValueValidation) => {}
        Err(kind) => problems.push(format!(
            "`{}` is refused for the wrong reason ({kind:?}); expected the value to be refused",
            argv.join(" ")
        )),
        Ok(()) => problems.push(format!("`{}` is accepted", argv.join(" "))),
    }
}

fn possible_values(command: &Command, flag: &str) -> Vec<String> {
    command
        .get_arguments()
        .find(|arg| arg.get_long() == Some(flag))
        .map(|arg| {
            arg.get_possible_values()
                .iter()
                .map(|value| value.get_name().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_flag_accepts_what_its_field_holds() {
    let spec = read_spec();
    let binding = read_binding();
    let state_flag = format!("--{}", binding.state);
    let config_flag = format!("--{}", binding.config);
    let root = derive_tree();
    let mut problems = Vec::new();
    let mut checked = 0;
    for group in &presented(&spec, &binding) {
        for leaf in &group.leaves {
            let Some(command) = root
                .find_subcommand(&group.name)
                .and_then(|actual_group| actual_group.find_subcommand(&leaf.wire))
            else {
                problems.push(format!(
                    "`{} {} {}` ({}) cannot be parsed: it is not in the derive tree",
                    spec.binary, group.name, leaf.wire, leaf.qualified
                ));
                continue;
            };
            checked += 1;
            let base = [
                spec.binary.as_str(),
                group.name.as_str(),
                leaf.wire.as_str(),
            ];
            // Every input may arrive through `--input-json -`, so no flag is required on its own:
            // the handler decides what is missing.
            expect_accepted(&base, &mut problems);
            // The state directory is taken before the group and after the leaf; an empty one
            // would name the working directory itself and is refused.
            expect_accepted(
                &[
                    spec.binary.as_str(),
                    state_flag.as_str(),
                    "some/state",
                    group.name.as_str(),
                    leaf.wire.as_str(),
                ],
                &mut problems,
            );
            expect_accepted(
                &argv(&base, &[state_flag.as_str(), "some/state"]),
                &mut problems,
            );
            expect_value_refused(&argv(&base, &[state_flag.as_str(), ""]), &mut problems);
            // The config file likewise, and an empty one names no file.
            expect_accepted(
                &[
                    spec.binary.as_str(),
                    config_flag.as_str(),
                    "some/config.yaml",
                    group.name.as_str(),
                    leaf.wire.as_str(),
                ],
                &mut problems,
            );
            expect_accepted(
                &argv(&base, &[config_flag.as_str(), "some/config.yaml"]),
                &mut problems,
            );
            expect_value_refused(&argv(&base, &[config_flag.as_str(), ""]), &mut problems);
            match &leaf.kind {
                Kind::Command {
                    fields,
                    pre_tool_use,
                } => {
                    expect_accepted(&argv(&base, &["--input-json", "-"]), &mut problems);
                    if *pre_tool_use {
                        expect_accepted(&argv(&base, &["--from-pre-tool-use"]), &mut problems);
                    }
                    for field in fields {
                        let flag = format!("--{}", field.flag);
                        let owned = |values: &[&str]| -> Vec<String> {
                            values.iter().map(|value| (*value).to_owned()).collect()
                        };
                        let (accepted, refused): (Vec<String>, Vec<&str>) = match &field.accepts {
                            Accepts::Text => (owned(&["a value"]), vec![]),
                            Accepts::Integer => (owned(&["-1", "42"]), vec!["4.2", "x"]),
                            Accepts::Boolean => (owned(&["true", "false"]), vec!["maybe"]),
                            // Exactly what the generated `Timestamp` reader reads
                            // (`conductor_model::primitives`, `fn rfc3339`).
                            Accepts::Timestamp => (
                                owned(&[
                                    "2026-10-06T17:00:00Z",
                                    "2026-10-06t17:00:00z",
                                    "2026-10-06T17:00:00-00:00",
                                    "2026-10-06T19:00:00.5+02:00",
                                    "2026-10-06T17:00:00.123456789Z",
                                ]),
                                vec![
                                    "yesterday",
                                    "2026-10-06",
                                    "2026-10-06 17:00:00",
                                    "2026-10-06 17:00:00Z",
                                    "2026-10-06X17:00:00Z",
                                    "2026-10-06T17:00:00.1234567891Z",
                                    "2026-12-31T23:59:60Z",
                                ],
                            ),
                            Accepts::Uuid => (
                                owned(&[
                                    "123e4567-e89b-12d3-a456-426614174000",
                                    "123E4567-E89B-12D3-A456-426614174000",
                                ]),
                                vec![
                                    "not-a-uuid",
                                    "123e4567e89b12d3a456426614174000",
                                    "123e4567-e89b-12d3-a456-42661417400g",
                                ],
                            ),
                            Accepts::Prefixed(prefix) => {
                                (vec![format!("{prefix}20261006-01")], vec!["20261006-01"])
                            }
                            Accepts::OneOf(members) => {
                                let listed = possible_values(command, &field.flag);
                                if &listed != members {
                                    problems.push(format!(
                                        "`{} {} {} {flag}` lists {listed:?}; the field's type \
                                         declares {members:?}",
                                        spec.binary, group.name, leaf.wire
                                    ));
                                }
                                (members.clone(), vec!["NotAMember"])
                            }
                        };
                        for value in &accepted {
                            expect_accepted(
                                &argv(&base, &[flag.as_str(), value.as_str()]),
                                &mut problems,
                            );
                        }
                        for value in refused {
                            expect_value_refused(
                                &argv(&base, &[flag.as_str(), value]),
                                &mut problems,
                            );
                        }
                    }
                }
                Kind::View => {
                    let listed = possible_values(command, "format");
                    if listed != FORMATS {
                        problems.push(format!(
                            "`{} {} {} --format` lists {listed:?}; every view offers {FORMATS:?}",
                            spec.binary, group.name, leaf.wire
                        ));
                    }
                    for format in FORMATS {
                        expect_accepted(&argv(&base, &["--format", format]), &mut problems);
                    }
                    expect_value_refused(&argv(&base, &["--format", "yaml"]), &mut problems);
                }
                // A local handler may narrow what its field holds (a port is an `Integer` the
                // flag holds to 0..=65535), so only values every such flag takes are accepted
                // here, and only values its field cannot hold are refused.
                Kind::Local { flags } => {
                    for flag in flags {
                        let (accepted, refused): (&[&str], &[&str]) = match flag.holds.as_str() {
                            "integer" => (&["42"], &["4.2", "x"]),
                            "string" => (&["a value"], &[]),
                            "boolean" => (&["true", "false"], &["maybe"]),
                            other => {
                                problems.push(format!(
                                    "`{} {} {} --{}` holds {other}, which this test does not \
                                     map onto a flag; teach it before declaring such a field",
                                    spec.binary, group.name, leaf.wire, flag.long
                                ));
                                continue;
                            }
                        };
                        let word = format!("--{}", flag.long);
                        for value in accepted {
                            expect_accepted(&argv(&base, &[word.as_str(), value]), &mut problems);
                        }
                        for value in refused {
                            expect_value_refused(
                                &argv(&base, &[word.as_str(), value]),
                                &mut problems,
                            );
                        }
                    }
                }
            }
        }
    }
    fail(
        problems,
        "a flag does not accept what its input field holds",
    );
    assert!(
        checked > 0,
        "the cli block places no command or view, so nothing was checked"
    );
}

fn expect_refused_as(argv: &[&str], wanted: ErrorKind, why: &str, problems: &mut Vec<String>) {
    match parses(argv) {
        Err(kind) if kind == wanted => {}
        other => problems.push(format!(
            "`{}` gives {other:?}, expected Err({wanted:?}): {why}",
            argv.join(" ")
        )),
    }
}

/// `name` one character short, unless that is empty or itself one of `names`.
fn shortened<'a>(name: &'a str, names: &[&str]) -> Option<&'a str> {
    let short = name.get(..name.len().saturating_sub(1))?;
    (!short.is_empty() && !names.contains(&short)).then_some(short)
}

/// The structural test compares the words clap was given; this one parses what clap would make
/// of a word it was not given. A catch-all (`external_subcommand`), inferred subcommands
/// (`infer_subcommands`), inferred long flags (`infer_long_args`) or a `help` word would each
/// let such a word through without adding a declared one.
#[test]
fn no_word_outside_the_cli_block_parses() {
    let spec = read_spec();
    let groups = presented(&spec, &read_binding());
    let root = derive_tree();
    let mut problems = Vec::new();

    let mut levels: Vec<(Vec<&str>, Vec<&str>)> = vec![(
        vec![spec.binary.as_str()],
        groups.iter().map(|group| group.name.as_str()).collect(),
    )];
    for group in &groups {
        levels.push((
            vec![spec.binary.as_str(), group.name.as_str()],
            group.leaves.iter().map(|leaf| leaf.wire.as_str()).collect(),
        ));
    }
    for (base, names) in &levels {
        expect_refused_as(
            &argv(base, &["no-such-word"]),
            ErrorKind::InvalidSubcommand,
            "an unknown word is no subcommand",
            &mut problems,
        );
        expect_refused_as(
            &argv(base, &["help"]),
            ErrorKind::InvalidSubcommand,
            "`help` is not a word of the cli block",
            &mut problems,
        );
        for name in names {
            if let Some(short) = shortened(name, names) {
                expect_refused_as(
                    &argv(base, &[short]),
                    ErrorKind::InvalidSubcommand,
                    "a prefix of a subcommand is not that subcommand",
                    &mut problems,
                );
            }
        }
    }

    for group in &groups {
        for leaf in &group.leaves {
            let Some(command) = root
                .find_subcommand(&group.name)
                .and_then(|actual_group| actual_group.find_subcommand(&leaf.wire))
            else {
                problems.push(format!(
                    "`{} {} {}` ({}) is not in the derive tree",
                    spec.binary, group.name, leaf.wire, leaf.qualified
                ));
                continue;
            };
            let base = [
                spec.binary.as_str(),
                group.name.as_str(),
                leaf.wire.as_str(),
            ];
            expect_refused_as(
                &argv(&base, &["stray-word"]),
                ErrorKind::UnknownArgument,
                "a placed command or view takes no positional word",
                &mut problems,
            );
            let flags: Vec<&str> = command
                .get_arguments()
                .filter_map(|arg| arg.get_long())
                .collect();
            for flag in &flags {
                if let Some(short) = shortened(flag, &flags) {
                    let word = format!("--{short}");
                    expect_refused_as(
                        &argv(&base, &[word.as_str()]),
                        ErrorKind::UnknownArgument,
                        "a prefix of a flag is not that flag",
                        &mut problems,
                    );
                }
            }
        }
    }
    fail(problems, "a word outside the cli block parses");
}

/// `ess specify cli` validates the binding against the model's types only: a command path the
/// `cli:` block does not place validates too. So every command the binding presents is checked
/// here to be a placed leaf of the block, under the block's binary and with its display as help,
/// or a command over a `local` callable, which the derive tree presents beside the block
/// (`story:live-dashboard`). Such a command has two words, no alias and only option flags: those
/// are what `derive_tree_is_the_cli_block` compares.
#[test]
fn the_binding_presents_only_placed_leaves_and_local_commands() {
    let spec = read_spec();
    let binding = read_binding();
    let mut problems = Vec::new();
    if binding.binary != spec.binary {
        problems.push(format!(
            "the binding presents binary `{}`, the cli block names `{}`",
            binding.binary, spec.binary
        ));
    }
    assert!(
        !binding.commands.is_empty(),
        "the binding presents no command, so nothing was checked"
    );
    for command in &binding.commands {
        let path = command.path.join(" ");
        let about = &command.about;
        let leaf = match command.path.as_slice() {
            [group, wire] => spec
                .groups
                .iter()
                .filter(|placed| placed.name == *group)
                .flat_map(|placed| &placed.leaves)
                .find(|leaf| leaf.wire == *wire),
            _ => None,
        };
        match leaf {
            Some(leaf) if leaf.display == *about => {}
            Some(leaf) => problems.push(format!(
                "binding command `{path}` reads {about:?}; `{}` displays {:?}",
                leaf.qualified, leaf.display
            )),
            None if command.target == "local" => {
                if command.path.len() != 2 {
                    problems.push(format!(
                        "local binding command `{path}` has {} word(s); the derive tree presents \
                         two (group, command)",
                        command.path.len()
                    ));
                }
                if command.aliases > 0 {
                    problems.push(format!(
                        "local binding command `{path}` declares aliases, which this test does \
                         not compare; teach it before declaring one"
                    ));
                }
                for argument in &command.unmapped {
                    problems.push(format!(
                        "local binding command `{path}` maps {argument}, which is not an option \
                         flag; teach this test before declaring one"
                    ));
                }
            }
            None => problems.push(format!(
                "binding command `{path}` is not a placed command or view of the cli block, \
                 and its callable's target is `{}`, not `local`",
                command.target
            )),
        }
    }
    fail(
        problems,
        "the binding crates/conductor/cli.yaml presents a word the cli block does not place",
    );
}

#[test]
fn derive_tree_is_well_formed() {
    Cli::command().debug_assert();
}
