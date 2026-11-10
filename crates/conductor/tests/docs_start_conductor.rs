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
//! - A command that does not take `--input-json` is never run as written, because it acts
//!   (`session start`, `spawn`, `daemon serve`). [`DECLARED`] names each such command the pages
//!   show with the flags its handler refuses to go without, and a line in a fenced block, or one
//!   that gives any flag, without one of them is a finding; a command of that kind [`DECLARED`]
//!   does not name is a finding too. Each declared flag is held to the handler: the binary runs
//!   the command with every other declared flag and without this one, and must name it.
//! - Each page holds exactly the commands counted in [`PAGES`], and the lines it names there are
//!   among those read.
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

/// A page whose commands are held, relative to the workspace root, with the count of `conductor`
/// commands it names, the count of them the built binary is run for, and commands that are among
/// those read, as the words after `conductor`. A page that holds another count is not being read
/// as it was, or has changed: the counts are then taken from the failure.
struct Page {
    path: &'static str,
    commands: usize,
    handled: usize,
    reads: &'static [&'static str],
}

/// The guide of the story, and the guide that names the same `stop-controller` line.
const PAGES: [Page; 2] = [
    Page {
        path: "website/docs/guides/start-conductor.md",
        commands: 29,
        handled: 3,
        reads: &[
            "controller stop-controller --controller-id <id> --reason <why>",
            "spawn --repository <repo>",
            "session start --role <role>",
            "daemon start-role --role supervisor",
        ],
    },
    Page {
        path: "website/docs/guides/first-task-existing-repo.md",
        commands: 26,
        handled: 5,
        reads: &["controller stop-controller --controller-id <id> --reason <why>"],
    },
];

/// Every command without `--input-json` a page may show, as the words after `conductor`, with
/// the flags its handler refuses to go without. Such a command is never run as written: it acts.
/// One with flags here answers a missing one before it does anything else, and
/// `every_declared_flag_is_one_the_handler_asks_for` runs it without each.
const DECLARED: [(&str, &[&str]); 23] = [
    ("config show", &[]),
    ("config validate", &[]),
    ("controller controllers", &[]),
    ("daemon serve", &[]),
    ("daemon start-role", &["role"]),
    ("dispatch dispatches", &[]),
    ("doctor", &[]),
    ("goal goals", &[]),
    ("init", &[]),
    ("instance serve", &[]),
    ("instance upgrade", &["tag"]),
    ("message send", &["to", "from", "text"]),
    ("repository activity", &[]),
    ("run", &[]),
    ("session attach", &[]),
    ("session deliver", &["name", "text"]),
    ("session list", &[]),
    ("session run-ticks", &[]),
    ("session start", &["role"]),
    ("setup", &[]),
    ("spawn", &["repository"]),
    ("trust", &[]),
    ("upgrade", &[]),
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

/// Every command text of `page`, with the line it starts on: see [`texts`].
fn command_texts(page: &str) -> Vec<(usize, String)> {
    texts(page)
        .into_iter()
        .map(|(line, text, _)| (line, text))
        .collect()
}

/// `line` without the `>` marks of a blockquote before it.
fn unquoted(line: &str) -> &str {
    let mut line = line.trim_start();
    while let Some(rest) = line.strip_prefix('>') {
        line = rest.trim_start();
    }
    line
}

/// Every command text of `page`, with the line it starts on and whether a fenced block holds it:
/// outside fenced blocks each span in backticks, one that ends on a later line included; in a
/// fenced block that is not `text`, each line after a `$ ` prompt, or every line when the block
/// has no prompt, a line that ends in a backslash joined with the one after it. A blockquote's
/// `>` marks are not part of a line.
fn texts(page: &str) -> Vec<(usize, String, bool)> {
    let mut out = Vec::new();
    let mut block: Option<(String, Vec<(usize, String)>)> = None;
    let mut span: Option<(usize, String)> = None;
    for (at, line) in page.lines().enumerate() {
        let line = unquoted(line);
        let fence = line.starts_with("```");
        match block.take() {
            None if fence => {
                span = None;
                let language = line.trim_start_matches('`').trim().to_owned();
                block = Some((language, Vec::new()));
            }
            None => {
                if line.trim().is_empty() {
                    span = None;
                }
                for (index, part) in line.split('`').enumerate() {
                    if index > 0 {
                        match span.take() {
                            Some((number, text)) => out.push((number, text, false)),
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
                            out.push((number, text, true));
                        } else if let Some(command) = text.strip_prefix("$ ") {
                            out.push((number, command.to_owned(), true));
                        }
                    }
                }
            }
            Some((language, mut lines)) => {
                match lines.last_mut() {
                    Some((_, text)) if text.ends_with('\\') => {
                        text.pop();
                        let kept = text.trim_end().len();
                        text.truncate(kept);
                        text.push(' ');
                        text.push_str(line.trim());
                    }
                    _ => lines.push((at + 1, line.to_owned())),
                }
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
/// `&&`, `||`, `|` and `;`, a word read from its last `$(` on and a `)` after a command dropped.
/// The command starts at the first word `conductor`, so a `NAME=value` and another program
/// (`watch -n 5`) before it are aside.
fn conductor_commands(text: &str) -> Vec<Vec<String>> {
    shell_words(text)
        .split(|word| matches!(word.as_str(), "&&" | "||" | "|" | ";"))
        .filter_map(|command| {
            let words: Vec<&str> = command
                .iter()
                .map(|word| word.rfind("$(").map_or(word.as_str(), |at| &word[at + 2..]))
                .skip_while(|word| *word != "conductor")
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

/// The command `words` name in `root`, and its words. A global flag before a subcommand, and its
/// value, are stepped over; the walk stops at the first other word that is no subcommand.
fn leaf<'a>(root: &'a clap::Command, words: &[String]) -> (&'a clap::Command, Vec<&'a str>) {
    let mut command = root;
    let mut path = Vec::new();
    let mut words = words.iter();
    while let Some(word) = words.next() {
        if let Some(flag) = word.strip_prefix("--") {
            let (long, value) = match flag.split_once('=') {
                Some((long, _)) => (long, true),
                None => (flag, false),
            };
            let Some(global) = root
                .get_arguments()
                .find(|arg| arg.is_global_set() && arg.get_long() == Some(long))
            else {
                break;
            };
            if !value && global.get_action().takes_values() {
                words.next();
            }
            continue;
        }
        match command.find_subcommand(word) {
            Some(sub) => {
                command = sub;
                path.push(sub.get_name());
            }
            None => break,
        }
    }
    (command, path)
}

/// Whether `command` is a group: it has a subcommand besides `help`.
fn is_group(command: &clap::Command) -> bool {
    command
        .get_subcommands()
        .any(|sub| sub.get_name() != "help")
}

/// Why a line that names the command `path`, one without `--input-json`, does not run as written:
/// [`DECLARED`] does not name the command, or the line lacks a flag declared there. A span outside
/// a fenced block that gives no flag at all is the page naming the command, and is taken.
fn declared_problem(path: &[&str], words: &[String], fenced: bool) -> Option<String> {
    let name = path.join(" ");
    let Some((_, required)) = DECLARED.iter().find(|(of, _)| *of == name) else {
        return Some(format!(
            "`conductor {name}` takes no `--{INPUT_JSON}` and is not in DECLARED: name it there \
             with the flags its handler requires"
        ));
    };
    let given = given(words);
    if !fenced && given.is_empty() {
        return None;
    }
    let missing: Vec<String> = required
        .iter()
        .filter(|long| !given.contains(*long))
        .map(|long| format!("`--{long}`"))
        .collect();
    (!missing.is_empty()).then(|| {
        format!(
            "the handler of `conductor {name}` asks for {}",
            missing.join(", ")
        )
    })
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
/// was asked about, each command read as the words after `conductor`, and each finding.
struct Held {
    commands: usize,
    handled: usize,
    read: Vec<String>,
    findings: Vec<String>,
}

/// Holds every `conductor` command of `page`, read as the file `name`, in the case directory
/// `scratch`.
fn hold(name: &str, page: &str, scratch: &Path) -> Held {
    let root = tree();
    let mut held = Held {
        commands: 0,
        handled: 0,
        read: Vec::new(),
        findings: Vec::new(),
    };
    for (line, text, fenced) in texts(page) {
        for words in conductor_commands(&text) {
            held.commands += 1;
            held.read.push(words.join(" "));
            let shown = format!("{name}:{line}: `conductor {}`", words.join(" "));
            if let Some(problem) = parse_problem(&words) {
                held.findings.push(format!("{shown}: {problem}"));
                continue;
            }
            let (command, path) = leaf(&root, &words);
            if !handler_decides(command, &words) {
                if !is_group(command)
                    && let Some(problem) = declared_problem(&path, &words, fenced)
                {
                    held.findings.push(format!("{shown}: {problem}"));
                }
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
        assert_eq!(
            (held.commands, held.handled),
            (page.commands, page.handled),
            "{}: the conductor commands read, and those of them asked of their handler, against \
             the counts PAGES holds the page to",
            page.path
        );
        for line in page.reads {
            assert!(
                held.read.iter().any(|read| read == line),
                "{}: `conductor {line}` is not among the commands read: {:#?}",
                page.path,
                held.read
            );
        }
        findings.extend(held.findings);
    }
    assert!(
        findings.is_empty(),
        "the guides show commands that do not run as written:\n{}",
        findings.join("\n")
    );
}

/// Each flag [`DECLARED`] gives a command is one its handler asks for: run without it, and with a
/// sample for every other declared flag, the built binary fails and names it. No complete line
/// is run, so nothing is started.
#[test]
fn every_declared_flag_is_one_the_handler_asks_for() {
    let scratch = Scratch::new("docs_start_conductor/declared");
    let root = tree();
    let mut problems = Vec::new();
    let mut asked = 0;
    for (index, (name, required)) in DECLARED.iter().enumerate() {
        let path: Vec<String> = name.split(' ').map(str::to_owned).collect();
        let (command, found) = leaf(&root, &path);
        if found.len() != path.len() || is_group(command) {
            problems.push(format!("`conductor {name}` is no command of the CLI"));
            continue;
        }
        if command
            .get_arguments()
            .any(|arg| arg.get_long() == Some(INPUT_JSON))
        {
            problems.push(format!(
                "`conductor {name}` takes `--{INPUT_JSON}`: its handler is asked, not DECLARED"
            ));
        }
        for missing in *required {
            asked += 1;
            let home = scratch.path().join(format!("home-{index}-{missing}"));
            fs::create_dir_all(&home).expect("create the case's home");
            let mut run = common::conductor(&home);
            run.current_dir(&home)
                .arg("--state-dir")
                .arg(home.join("state"))
                .args(&path);
            for other in required.iter().filter(|other| *other != missing) {
                let flag = format!("--{other}");
                let sample = SAMPLES
                    .iter()
                    .find(|(of, _)| *of == flag)
                    .map_or("example", |(_, sample)| sample);
                run.arg(flag).arg(sample);
            }
            let output = run.stdin(Stdio::null()).output().expect("run conductor");
            let said = String::from_utf8_lossy(&output.stderr);
            if output.status.success() || !names_flag(&said, missing) {
                problems.push(format!(
                    "`conductor {name}` without `--{missing}` exits {:?} and does not name it: {}",
                    output.status.code(),
                    said.trim()
                ));
            }
        }
    }
    assert!(asked >= 9, "only {asked} declared flags were asked for");
    assert!(
        problems.is_empty(),
        "DECLARED does not say what the handlers do:\n{}",
        problems.join("\n")
    );
}

/// A command without `--input-json` is held to [`DECLARED`]: a span that only names it is taken,
/// a span or a fenced line that lacks a declared flag is found, and so is a command the table
/// does not name.
#[test]
fn a_command_without_input_json_is_held_to_the_declared_flags() {
    let page = "The loop runs `conductor spawn`, and `conductor session deliver --name <name>`\n\
                types into `conductor message send --to conductor --from <name> --text -`.\n\n\
                ```console\n\
                conductor message send --to conductor --text -\n\
                conductor spawn --repository <repo>\n\
                conductor role show --role maintainer\n\
                ```\n";
    let scratch = Scratch::new("docs_start_conductor/declared_lines");
    let held = hold("page.md", page, scratch.path());
    assert_eq!((held.commands, held.handled), (6, 0), "the commands held");
    let found: Vec<(&str, bool)> = held
        .findings
        .iter()
        .map(|finding| {
            (
                finding
                    .split(": ")
                    .next()
                    .expect("a finding names its place"),
                finding.contains("is not in DECLARED"),
            )
        })
        .collect();
    assert_eq!(
        found,
        [
            ("page.md:1", false),
            ("page.md:5", false),
            ("page.md:7", true)
        ],
        "a deliver without --text, a send without --from and a command DECLARED does not name: \
         {:#?}",
        held.findings
    );
    assert!(
        held.findings[0].contains("`--text`") && held.findings[1].contains("`--from`"),
        "each finding names the flag: {:#?}",
        held.findings
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

/// Adversary cases of wave 49, unit U23, pass 1. Each is red on the tree it was written against.
mod adversary {
    use super::*;

    /// The place, `page.md:<line>`, of each finding.
    fn places(held: &Held) -> Vec<String> {
        held.findings
            .iter()
            .map(|finding| finding.split(": ").next().unwrap_or_default().to_owned())
            .collect()
    }

    /// What the built binary answers to `words`, in an isolated home, with no flag added.
    fn answer(home: &Path, words: &[&str]) -> String {
        fs::create_dir_all(home).expect("create the case's home");
        let output = common::conductor(home)
            .current_dir(home)
            .args(words)
            .stdin(Stdio::null())
            .output()
            .expect("run conductor");
        format!(
            "conductor {} -> {:?}: {}",
            words.join(" "),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        )
    }

    /// The acceptance: the test "fails on a missing required flag". `start-conductor.md` names
    /// `conductor session start --role <role>` (lines 103 and 108), `conductor daemon start-role
    /// --role supervisor` (109) and `conductor spawn --repository <repo>` (64). Each handler
    /// refuses the command without that flag, and none of the three takes `--input-json`.
    #[test]
    fn adv_a_a_missing_flag_of_a_command_without_input_json_is_found() {
        let page = "```console\n\
                    conductor session start\n\
                    conductor daemon start-role\n\
                    conductor spawn\n\
                    ```\n";
        let scratch = Scratch::new("docs_start_conductor/adv_a");
        let answers = [
            answer(&scratch.path().join("direct-1"), &["session", "start"]),
            answer(&scratch.path().join("direct-2"), &["daemon", "start-role"]),
            answer(&scratch.path().join("direct-3"), &["spawn"]),
        ];
        let held = hold("page.md", page, &scratch.path().join("held"));
        assert_eq!(held.commands, 3, "the commands held");
        assert_eq!(
            places(&held),
            ["page.md:2", "page.md:3", "page.md:4"],
            "each line lacks the flag its handler requires; handled {}, findings {:#?}; the \
             binary answers {answers:#?}",
            held.handled,
            held.findings
        );
    }

    /// `--state-dir` is global, "before or after any subcommand" (`cli.rs`). Written on the line,
    /// it keeps the handler's answer from the test: after the subcommand the run gives it twice,
    /// before it the walk to the leaf stops at the root.
    #[test]
    fn adv_b_a_state_dir_on_the_line_does_not_hide_the_missing_reason() {
        let page = "```console\n\
                    conductor controller stop-controller --controller-id <id>\n\
                    conductor controller stop-controller --controller-id <id> --state-dir <dir>\n\
                    conductor --state-dir <dir> controller stop-controller --controller-id <id>\n\
                    ```\n";
        let scratch = Scratch::new("docs_start_conductor/adv_b");
        let held = hold("page.md", page, scratch.path());
        assert_eq!(held.commands, 3, "the commands held");
        assert_eq!(
            places(&held),
            ["page.md:2", "page.md:3", "page.md:4"],
            "each line stops a controller without --reason; handled {}, findings {:#?}",
            held.handled,
            held.findings
        );
    }

    /// The floor of a page is what the page holds: a floor below it lets that many commands go
    /// unread with the guides case green, the `stop-controller` line of the second guide among
    /// them.
    #[test]
    fn adv_c_the_floor_of_each_page_is_what_the_page_holds() {
        let scratch = Scratch::new("docs_start_conductor/adv_c");
        let mut slack = Vec::new();
        for (index, page) in PAGES.iter().enumerate() {
            let text = read(page.path);
            let held = hold(page.path, &text, &scratch.path().join(index.to_string()));
            if (held.commands, held.handled) != (page.commands, page.handled) {
                slack.push(format!(
                    "{}: holds {} commands, {} handled; the floor is {} and {}",
                    page.path, held.commands, held.handled, page.commands, page.handled
                ));
            }
            // The page without its `stop-controller` lines: the mutant of an extractor that
            // does not read them.
            let without: String = text
                .lines()
                .filter(|line| !line.contains("stop-controller"))
                .flat_map(|line| [line, "\n"])
                .collect();
            let unread = hold(
                page.path,
                &without,
                &scratch.path().join(format!("{index}-without")),
            );
            if unread.commands >= page.commands && unread.handled >= page.handled {
                slack.push(format!(
                    "{}: with no stop-controller line read ({} commands, {} handled) the floor \
                     {} and {} still holds",
                    page.path, unread.commands, unread.handled, page.commands, page.handled
                ));
            }
        }
        assert!(
            slack.is_empty(),
            "the floors do not hold the pages:\n{}",
            slack.join("\n")
        );
    }

    /// A `conductor` command in a command substitution after `name=`, and one after another
    /// program, are commands of the page: `getting-started.md` writes `state=$(conductor config
    /// show --format json | jq ...)`.
    #[test]
    fn adv_d_a_conductor_command_behind_an_assignment_or_a_program_is_read() {
        let page = "```console\n\
                    id=$(conductor controller controllers --nope)\n\
                    watch -n 5 conductor session list --nope\n\
                    ```\n\n\
                    > ```console\n\
                    > conductor controller controllers --nope\n\
                    > ```\n";
        let scratch = Scratch::new("docs_start_conductor/adv_d");
        let held = hold("page.md", page, scratch.path());
        assert_eq!(
            places(&held),
            ["page.md:2", "page.md:3", "page.md:7"],
            "each line gives a flag its command does not take; commands {}, findings {:#?}",
            held.commands,
            held.findings
        );
    }

    /// A command continued with a backslash is one command, and its flags are those of both
    /// lines.
    #[test]
    fn adv_e_a_command_continued_with_a_backslash_is_one_command() {
        let page = "```console\n\
                    conductor controller stop-controller --controller-id <id> \\\n\
                    \x20   --reason <why>\n\
                    conductor controller pause-controller --controller-id <id> \\\n\
                    \x20   --nope\n\
                    ```\n";
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
                    2,
                    "controller|stop-controller|--controller-id|<id>|--reason|<why>".to_owned()
                ),
                (
                    4,
                    "controller|pause-controller|--controller-id|<id>|--nope".to_owned()
                ),
            ],
            "the commands read from a block with continued lines"
        );
    }
}
