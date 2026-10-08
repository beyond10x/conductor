//! Adversary cases for `story:cli-from-spec` (wave 02, U1, pass 2).
//!
//! Driven from the specification (`ess specify compile --path spec --format json`) and from the
//! generated model, never from `src/cli.rs`.

use std::path::PathBuf;
use std::process::Command as Process;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use conductor_cli::cli::Cli;
use conductor_model::primitives::invariant::{Fact, Op, compare};
use serde_json::Value;

fn compile() -> Value {
    let spec = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec");
    let output = Process::new("ess")
        .args(["specify", "compile", "--path"])
        .arg(&spec)
        .args(["--format", "json"])
        .output()
        .expect("`ess` runs");
    assert!(output.status.success(), "`ess specify compile` failed");
    serde_json::from_slice(&output.stdout).expect("`ess specify compile` prints JSON")
}

fn parse(argv: &[&str]) -> Result<(), ErrorKind> {
    Cli::try_parse_from(argv)
        .map(drop)
        .map_err(|error| error.kind())
}

/// Whether the generated model can read `text` as the instant a `Timestamp` names: the
/// generated invariant reader orders two readable instants and answers `None` (unknown) for an
/// unreadable one (`crates/conductor-model/src/primitives.rs`, `fn rfc3339`).
fn generated_reads(text: &str) -> bool {
    compare(
        Some(Fact::text(text)),
        Op::Le,
        Some(Fact::text(text)),
        true,
        false,
    ) == Some(true)
}

/// Every placed command's (group, wire, qualified) whose input field `name` is a `Timestamp`,
/// with that field's flag.
fn timestamp_flags(model: &Value) -> Vec<(String, String, String, String)> {
    let cli = model["components"]
        .as_object()
        .expect("components")
        .values()
        .find(|component| !component["cli"].is_null())
        .expect("a component with a `cli:` block")["cli"]
        .clone();
    let mut out = Vec::new();
    for group in cli["groups"].as_array().expect("groups") {
        let group_name = group["name"].as_str().expect("group name");
        for qualified in group["commands"].as_array().expect("commands") {
            let qualified = qualified.as_str().expect("qualified");
            let command = &model["commands"][qualified];
            let wire = command["naming"]["wire"].as_str().expect("wire");
            for field in command["input"].as_array().expect("input") {
                let mut type_ref = &field["type_ref"];
                if type_ref["kind"].as_str() == Some("optional") {
                    type_ref = &type_ref["of"];
                }
                if type_ref["kind"].as_str() == Some("primitive")
                    && type_ref["name"].as_str() == Some("timestamp")
                {
                    let name = field["name"].as_str().expect("field name");
                    out.push((
                        group_name.to_owned(),
                        wire.to_owned(),
                        format!("{qualified}.{name}"),
                        format!("--{}", name.replace('_', "-")),
                    ));
                }
            }
        }
    }
    out
}

/// Spellings on both sides of the generated reader's boundary.
const INSTANTS: [&str; 10] = [
    "2026-10-06T17:00:00Z",
    "2026-10-06t17:00:00z",
    "2026-10-06T17:00:00-00:00",
    "2026-10-06T19:00:00.5+02:00",
    "2026-10-06 17:00:00Z",
    "2026-10-06X17:00:00Z",
    "2026-10-06T17:00:00.1234567891Z",
    "2026-12-31T23:59:60Z",
    "2026-02-29T00:00:00Z",
    "yesterday",
];

#[test]
fn adv2_space_separated_instant_is_refused_like_the_generated_model_refuses_it() {
    let value = "2026-10-06 17:00:00Z";
    assert!(
        !generated_reads(value),
        "precondition: the generated Timestamp reader cannot read {value:?}"
    );
    assert_eq!(
        parse(&[
            "conductor",
            "snapshot",
            "start-snapshot",
            "--started-at",
            value
        ]),
        Err(ErrorKind::ValueValidation),
        "`--started-at {value:?}` names no instant the generated model can read, so the flag \
         must refuse it"
    );
}

#[test]
fn adv2_timestamp_flags_accept_exactly_what_the_generated_model_reads() {
    let model = compile();
    let flags = timestamp_flags(&model);
    assert!(!flags.is_empty(), "no timestamp flag was found");
    let mut problems = Vec::new();
    let mut checked = 0;
    for (group, wire, field, flag) in &flags {
        for value in INSTANTS {
            checked += 1;
            let generated = generated_reads(value);
            let cli = parse(&["conductor", group, wire, flag, value]).is_ok();
            if generated != cli {
                problems.push(format!(
                    "`conductor {group} {wire} {flag} {value:?}`: the flag {} it, the generated \
                     model {} it ({field})",
                    if cli { "accepts" } else { "refuses" },
                    if generated { "reads" } else { "cannot read" },
                ));
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} of {checked} (flag, value) pairs disagree with the generated Timestamp reader:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}

#[test]
fn adv2_no_value_word_outside_the_cli_block() {
    let model = compile();
    let root = Cli::command();
    let mut problems = Vec::new();
    // No possible value of any flag carries an alias.
    for group in root.get_subcommands() {
        for leaf in group.get_subcommands() {
            for arg in leaf.get_arguments() {
                for value in arg.get_possible_values() {
                    let words: Vec<&str> = value.get_name_and_aliases().collect();
                    if words.len() > 1 {
                        problems.push(format!(
                            "`conductor {} {} --{}` accepts {words:?} for one value",
                            group.get_name(),
                            leaf.get_name(),
                            arg.get_long().unwrap_or("?")
                        ));
                    }
                }
            }
        }
    }
    // Case is part of the word: `--format JSON` and a re-cased enum variant are refused.
    let mut expect_refused = |argv: Vec<String>| {
        let words: Vec<&str> = argv.iter().map(String::as_str).collect();
        match parse(&words) {
            Err(ErrorKind::InvalidValue | ErrorKind::ValueValidation) => {}
            other => problems.push(format!("`{}` gives {other:?}", words.join(" "))),
        }
    };
    for format in ["JSON", "Markdown", "TEXT"] {
        expect_refused(
            ["conductor", "goal", "goals", "--format", format]
                .map(str::to_owned)
                .to_vec(),
        );
    }
    let cli = model["components"]
        .as_object()
        .expect("components")
        .values()
        .find(|component| !component["cli"].is_null())
        .expect("cli")["cli"]
        .clone();
    for group in cli["groups"].as_array().expect("groups") {
        let group_name = group["name"].as_str().expect("group name");
        for qualified in group["commands"].as_array().expect("commands") {
            let command = &model["commands"][qualified.as_str().expect("qualified")];
            let wire = command["naming"]["wire"].as_str().expect("wire");
            for field in command["input"].as_array().expect("input") {
                let mut type_ref = &field["type_ref"];
                if type_ref["kind"].as_str() == Some("optional") {
                    type_ref = &type_ref["of"];
                }
                if type_ref["kind"].as_str() != Some("declared") {
                    continue;
                }
                let body = &model["types"][type_ref["name"].as_str().expect("name")]["body"];
                if body["kind"].as_str() != Some("enum") {
                    continue;
                }
                let flag = format!(
                    "--{}",
                    field["name"].as_str().expect("name").replace('_', "-")
                );
                for variant in body["variants"].as_array().expect("variants") {
                    let variant = variant.as_str().expect("variant");
                    for recased in [variant.to_lowercase(), variant.to_uppercase()] {
                        if recased != variant {
                            expect_refused(vec![
                                "conductor".to_owned(),
                                group_name.to_owned(),
                                wire.to_owned(),
                                flag.clone(),
                                recased,
                            ]);
                        }
                    }
                }
            }
        }
    }
    assert!(
        problems.is_empty(),
        "{} value word(s) outside the cli block parse:\n  - {}",
        problems.len(),
        problems.join("\n  - ")
    );
}
