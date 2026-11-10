//! `story:documented-commands-parse`: every `conductor` line of
//! `website/docs/guides/start-conductor.md` runs as written.
//!
//! - Each `conductor` command a page in [`PAGES`] names, in a fenced block or a code span (one
//!   broken across two lines included), is parsed by the CLI's own parser, [`Cli`], with a sample
//!   value for each `<placeholder>`. A word that names no command, a flag the command does not
//!   take and a value the flag refuses are findings.
//! - No flag is required by the parser, because every input field may arrive through
//!   `--input-json -`: the handler decides what is missing. So each command that takes
//!   `--input-json` and is written without it is also run, by the built binary on an empty state
//!   directory in an isolated home, and a refusal that names a flag of the command the line does
//!   not give is a finding: `controller stop-controller: --reason is required`.
//!
//! Every file is read relative to the workspace root, and a finding names the file and the line.

mod common;
mod scratch;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use clap::error::ErrorKind;
use clap::{CommandFactory, Parser};
use conductor_cli::cli::Cli;

use scratch::Scratch;

/// A page whose commands are held, relative to the workspace root, with the fewest `conductor`
/// commands it names and the fewest of them the handler is asked about. A page that names fewer
/// is not being read.
struct Page {
    path: &'static str,
    commands: usize,
    handled: usize,
}

/// The guide of the story, and the guide that names the same `stop-controller` line.
const PAGES: [Page; 2] = [
    Page {
        path: "website/docs/guides/start-conductor.md",
        commands: 20,
        handled: 3,
    },
    Page {
        path: "website/docs/guides/first-task-existing-repo.md",
        commands: 8,
        handled: 3,
    },
];

/// The value a `<placeholder>` after a flag stands for, where `example` is not one the flag takes.
const SAMPLES: [(&str, &str); 2] = [
    ("--controller-id", "00000000-0000-7000-8000-000000000000"),
    ("--tag", "v1.0.0"),
];

/// The flag every command with input fields takes instead of them.
const INPUT_JSON: &str = "input-json";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root")
}

fn read(relative: &str) -> String {
    let path = workspace().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("read {}: {error}", path.display()))
}

/// Every command text of `page`, with the line it starts on: outside fenced blocks each span in
/// backticks, one that ends on a later line included; in a fenced block that is not `text`, each
/// line after a `$ ` prompt, or every line when the block has no prompt.
fn command_texts(page: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut block: Option<(String, Vec<(usize, String)>)> = None;
    let mut span: Option<(usize, String)> = None;
    for (at, line) in page.lines().enumerate() {
        let fence = line.trim_start().starts_with("```");
        match block.take() {
            None if fence => {
                span = None;
                let language = line.trim_start().trim_start_matches('`').trim().to_owned();
                block = Some((language, Vec::new()));
            }
            None => {
                if line.trim().is_empty() {
                    span = None;
                }
                for (index, part) in line.split('`').enumerate() {
                    if index > 0 {
                        match span.take() {
                            Some(done) => out.push(done),
                            None => span = Some((at + 1, String::new())),
                        }
                    }
                    if let Some((_, text)) = span.as_mut() {
                        if !text.is_empty() {
                            text.push(' ');
                        }
                        text.push_str(part.trim());
                    }
                }
            }
            Some((language, lines)) if fence => {
                if language != "text" {
                    let prompted = lines.iter().any(|(_, text)| text.starts_with("$ "));
                    for (number, text) in lines {
                        if !prompted {
                            out.push((number, text));
                        } else if let Some(command) = text.strip_prefix("$ ") {
                            out.push((number, command.to_owned()));
                        }
                    }
                }
            }
            Some((language, mut lines)) => {
                lines.push((at + 1, line.trim_start().to_owned()));
                block = Some((language, lines));
            }
        }
    }
    out
}

/// The words of `text` as a shell reads them: split on whitespace outside quotes and outside a
/// `<placeholder>`, the quotes removed, and nothing from a `#` that starts a word.
fn shell_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut word: Option<String> = None;
    let mut quote: Option<char> = None;
    let mut angle = 0_usize;
    for character in text.chars() {
        match (quote, character) {
            (Some(open), _) if character == open => quote = None,
            (Some(_), _) => word.get_or_insert_with(String::new).push(character),
            (None, '"' | '\'') if angle == 0 => {
                quote = Some(character);
                word.get_or_insert_with(String::new);
            }
            (None, '#') if word.is_none() => break,
            (None, _) if character.is_whitespace() && angle == 0 => words.extend(word.take()),
            (None, _) => {
                match character {
                    '<' => angle += 1,
                    '>' => angle = angle.saturating_sub(1),
                    _ => {}
                }
                word.get_or_insert_with(String::new).push(character);
            }
        }
    }
    words.extend(word);
    words
}

/// The words after `conductor` of each `conductor` command in `text`: its shell words, split at
/// `&&`, `||`, `|` and `;`, a `$(` before and a `)` after a command dropped, and a leading
/// `NAME=value` aside.
fn conductor_commands(text: &str) -> Vec<Vec<String>> {
    shell_words(text)
        .split(|word| matches!(word.as_str(), "&&" | "||" | "|" | ";"))
        .filter_map(|command| {
            let words: Vec<&str> = command
                .iter()
                .map(|word| word.trim_start_matches("$("))
                .skip_while(|word| word.contains('=') && !word.starts_with('-'))
                .collect();
            (words.first() == Some(&"conductor")).then(|| {
                words[1..]
                    .iter()
                    .map(|word| word.trim_end_matches([')', ';']).to_owned())
                    .collect()
            })
        })
        .filter(|words: &Vec<String>| !words.is_empty())
        .collect()
}

/// `words` with a sample value for each word that holds a `<placeholder>`: the one [`SAMPLES`]
/// names for the flag before it, else `example`.
fn filled(words: &[String]) -> Vec<String> {
    let mut flag = "";
    words
        .iter()
        .map(|word| {
            if word.starts_with("--") {
                flag = word.split('=').next().unwrap_or_default();
                return word.clone();
            }
            if !(word.contains('<') && word.contains('>')) {
                return word.clone();
            }
            SAMPLES
                .iter()
                .find(|(of, _)| *of == flag)
                .map_or("example", |(_, sample)| sample)
                .to_owned()
        })
        .collect()
}

/// Why the CLI's parser refuses `words`, the words after `conductor`; `None` when it takes them.
/// A group named alone, and `--help`, are the page talking about the command, and are taken.
fn parse_problem(words: &[String]) -> Option<String> {
    let argv = std::iter::once("conductor".to_owned()).chain(filled(words));
    let error = Cli::try_parse_from(argv).err()?;
    match error.kind() {
        ErrorKind::DisplayHelp
        | ErrorKind::DisplayVersion
        | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        | ErrorKind::MissingSubcommand => None,
        _ => Some(
            error
                .to_string()
                .lines()
                .next()
                .unwrap_or_default()
                .to_owned(),
        ),
    }
}

/// The CLI tree, built, so every command carries the global flags and `--help`.
fn tree() -> clap::Command {
    let mut root = Cli::command();
    root.build();
    root
}

/// The command `words` name in `root`: the walk stops at the first word that is no subcommand.
fn leaf<'a>(root: &'a clap::Command, words: &[String]) -> &'a clap::Command {
    let mut command = root;
    for word in words {
        match command.find_subcommand(word) {
            Some(sub) => command = sub,
            None => break,
        }
    }
    command
}

/// The flags `words` give, without `--` and without a value.
fn given(words: &[String]) -> Vec<&str> {
    words
        .iter()
        .filter_map(|word| word.strip_prefix("--"))
        .map(|flag| flag.split('=').next().unwrap_or_default())
        .collect()
}

/// Whether the handler of `command` decides which of its flags `words` must give: it takes
/// `--input-json`, and the line does not hand its input that way.
fn handler_decides(command: &clap::Command, words: &[String]) -> bool {
    command
        .get_arguments()
        .any(|arg| arg.get_long() == Some(INPUT_JSON))
        && !given(words).contains(&INPUT_JSON)
}

/// Whether `text` names the flag `--long`, and not a longer one that starts with it.
fn names_flag(text: &str, long: &str) -> bool {
    let flag = format!("--{long}");
    text.match_indices(&flag).any(|(at, _)| {
        !text[at + flag.len()..]
            .starts_with(|next: char| next.is_ascii_alphanumeric() || next == '-')
    })
}

/// The flags of `command` the handler names when it runs `words` on the empty state directory of
/// `home`, and the line does not give: the ones it cannot do without.
fn handler_problem(command: &clap::Command, words: &[String], home: &Path) -> Option<String> {
    fs::create_dir_all(home).expect("create the case's home");
    let output = common::conductor(home)
        .current_dir(home)
        .arg("--state-dir")
        .arg(home.join("state"))
        .args(filled(words))
        .stdin(Stdio::null())
        .output()
        .expect("run conductor");
    let said = String::from_utf8_lossy(&output.stderr);
    let given = given(words);
    let missing: Vec<String> = command
        .get_arguments()
        .filter(|arg| !arg.is_global_set())
        .filter_map(clap::Arg::get_long)
        .filter(|long| !given.contains(long) && names_flag(&said, long))
        .map(|long| format!("`--{long}`"))
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "the handler asks for {}: {}",
            missing.join(", "),
            said.trim()
        )
    })
}

/// What a page was held to: how many `conductor` commands it names, how many of them the handler
/// was asked about, and each finding.
struct Held {
    commands: usize,
    handled: usize,
    findings: Vec<String>,
}

/// Holds every `conductor` command of `page`, read as the file `name`, in the case directory
/// `scratch`.
fn hold(name: &str, page: &str, scratch: &Path) -> Held {
    let root = tree();
    let mut held = Held {
        commands: 0,
        handled: 0,
        findings: Vec::new(),
    };
    for (line, text) in command_texts(page) {
        for words in conductor_commands(&text) {
            held.commands += 1;
            let shown = format!("{name}:{line}: `conductor {}`", words.join(" "));
            if let Some(problem) = parse_problem(&words) {
                held.findings.push(format!("{shown}: {problem}"));
                continue;
            }
            let command = leaf(&root, &words);
            if !handler_decides(command, &words) {
                continue;
            }
            held.handled += 1;
            let home = scratch.join(format!("home-{}", held.handled));
            if let Some(problem) = handler_problem(command, &words, &home) {
                held.findings.push(format!("{shown}: {problem}"));
            }
        }
    }
    held
}

#[test]
fn every_conductor_line_of_the_guides_runs_as_written() {
    let scratch = Scratch::new("docs_start_conductor/guides");
    let mut findings = Vec::new();
    for (index, page) in PAGES.iter().enumerate() {
        let held = hold(
            page.path,
            &read(page.path),
            &scratch.path().join(index.to_string()),
        );
        assert!(
            held.commands >= page.commands && held.handled >= page.handled,
            "{} names only {} conductor commands, {} of them asked of their handler; at least {} \
             and {} are held",
            page.path,
            held.commands,
            held.handled,
            page.commands,
            page.handled
        );
        findings.extend(held.findings);
    }
    assert!(
        findings.is_empty(),
        "the guides show commands that do not run as written:\n{}",
        findings.join("\n")
    );
}

#[test]
fn the_checks_of_this_test_find_what_they_look_for() {
    let page = "Stop it with `conductor controller stop-controller --controller-id <id>\n\
                --reason <why>`, or `conductor controller` for the group.\n\n\
                ```console\n\
                conductor controller stop-controller --controller-id <id>   # no reason\n\
                conductor controller pause-controller --controller-id <id>\n\
                conductor controller pause-controller --controller-id not-an-id\n\
                conductor controller controllers --nope\n\
                CONDUCTOR_INSTANCE=alpha conductor doctor && conductor session nope\n\
                conductor repository activate-repository --repository alpha --marked-by Operator --reason \"first task\"\n\
                ```\n\n\
                ```text\nconductor nothing here\n```\n";
    let shown: Vec<(usize, String)> = command_texts(page)
        .into_iter()
        .flat_map(|(line, text)| {
            conductor_commands(&text)
                .into_iter()
                .map(move |words| (line, words.join("|")))
        })
        .collect();
    assert_eq!(
        shown,
        [
            (
                1,
                "controller|stop-controller|--controller-id|<id>|--reason|<why>".to_owned()
            ),
            (2, "controller".to_owned()),
            (
                5,
                "controller|stop-controller|--controller-id|<id>".to_owned()
            ),
            (
                6,
                "controller|pause-controller|--controller-id|<id>".to_owned()
            ),
            (
                7,
                "controller|pause-controller|--controller-id|not-an-id".to_owned()
            ),
            (8, "controller|controllers|--nope".to_owned()),
            (9, "doctor".to_owned()),
            (9, "session|nope".to_owned()),
            (
                10,
                "repository|activate-repository|--repository|alpha|--marked-by|Operator|--reason|first task"
                    .to_owned()
            ),
        ],
        "the commands this test reads from a page"
    );

    let scratch = Scratch::new("docs_start_conductor/checks");
    let held = hold("page.md", page, scratch.path());
    assert_eq!(held.commands, 9, "the commands held");
    let lines: Vec<&str> = held
        .findings
        .iter()
        .map(|finding| {
            finding
                .split(": ")
                .next()
                .expect("a finding names its place")
        })
        .collect();
    assert_eq!(
        lines,
        ["page.md:5", "page.md:7", "page.md:8", "page.md:9"],
        "a missing --reason, a value the flag refuses, a flag the command does not take and a \
         word that is no command are found, and nothing else: {:#?}",
        held.findings
    );
    assert!(
        held.findings[0].contains("the handler asks for `--reason`"),
        "the stop without a reason is refused by its handler: {}",
        held.findings[0]
    );
    assert!(
        names_flag("x: --reason is required", "reason") && !names_flag("--reason-text", "reason"),
        "a flag is named whole"
    );
}
