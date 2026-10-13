//! `story:setup-guide-for-agents`: `SETUP.md` is the step list an agent follows to set conductor up
//! in a repository, and the README points an agent at it.
//!
//! - `SETUP.md` at the repository root is a numbered list of steps, and every step has a `Check:`,
//!   a `Fix:` and a `Hands:` line.
//! - Every span in backticks on a `Check:` or `Fix:` line is a command. Each of its commands (split
//!   on `&&`, `||`, `|` and `;`, a leading `NAME=value` aside) starts with a tool: `conductor`,
//!   whose words must name a command of the CLI tree and whose flags must be that command's; a
//!   program `b10x.toml` pins; or one of [`TOOLS`].
//! - The steps run in the order the story names: the forge from `git remote get-url origin`, the
//!   forge CLI's login, conductor from its archive and `SHA256SUMS`, the planning store's dry run,
//!   `conductor init`, `conductor trust`, and last `conductor doctor`.
//! - The first lines of `README.md` say an agent asked to set conductor up follows `SETUP.md`, with
//!   its raw URL.
//!
//! Every file is read relative to the workspace root, and a finding names the step and the line.

use std::fs;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use conductor_cli::cli::Cli;

/// The tools a step may name besides `conductor` and the programs `b10x.toml` pins.
const TOOLS: [&str; 8] = [
    "git",
    "gh",
    "glab",
    "cargo",
    "curl",
    "tar",
    "sha256sum",
    "shasum",
];

/// The keys every step carries, each on a line of its own.
const KEYS: [&str; 3] = ["Check:", "Fix:", "Hands:"];

/// How many lines at the top of the README the pointer to `SETUP.md` must be within.
const README_TOP: usize = 12;

/// The start of the raw URL of a file on GitHub, and the end of `SETUP.md`'s on `main`.
const RAW_HOST: &str = "https://raw.githubusercontent.com/";
const RAW_END: &str = "/conductor/main/SETUP.md";

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

/// One numbered step of `SETUP.md`: its first line and every line up to the next step or heading.
#[derive(Debug)]
struct Step {
    /// Its line number, from 1.
    line: usize,
    /// Its title line.
    title: String,
    /// Its lines, the title's included, each with its line number.
    lines: Vec<(usize, String)>,
}

/// Whether `line` starts a numbered step: digits, `.` and a space, at the start of the line.
fn starts_step(line: &str) -> bool {
    let digits: String = line.chars().take_while(char::is_ascii_digit).collect();
    !digits.is_empty() && line[digits.len()..].starts_with(". ")
}

/// The numbered steps of `text`, in order.
fn steps(text: &str) -> Vec<Step> {
    let mut steps: Vec<Step> = Vec::new();
    let mut open = false;
    for (at, line) in text.lines().enumerate() {
        if starts_step(line) {
            steps.push(Step {
                line: at + 1,
                title: line.to_owned(),
                lines: Vec::new(),
            });
            open = true;
        } else if line.starts_with('#') {
            open = false;
        }
        if open && let Some(step) = steps.last_mut() {
            step.lines.push((at + 1, line.to_owned()));
        }
    }
    steps
}

/// The text after `key` when `line`, a list item's `- ` aside, starts with it.
fn keyed<'l>(line: &'l str, key: &str) -> Option<&'l str> {
    let line = line.trim_start();
    let line = line.strip_prefix("- ").unwrap_or(line);
    line.strip_prefix(key)
}

/// The spans in backticks of `text`.
fn spans(text: &str) -> Vec<&str> {
    text.split('`')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, span)| span)
        .collect()
}

/// Each command of a span, as its words, a leading `NAME=value` aside.
fn commands(span: &str) -> Vec<Vec<&str>> {
    span.split("&&")
        .flat_map(|part| part.split("||"))
        .flat_map(|part| part.split(['|', ';']))
        .map(|command| {
            command
                .split_whitespace()
                .skip_while(|word| {
                    word.split_once('=').is_some_and(|(name, _)| {
                        !name.is_empty()
                            && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                    })
                })
                .collect::<Vec<_>>()
        })
        .filter(|words| !words.is_empty())
        .collect()
}

/// The `Check:` and `Fix:` items of `step`: each the text after its key and its continuation
/// lines (up to a blank line or the next list item), joined by spaces, with its first line.
fn items(step: &Step) -> Vec<(usize, String)> {
    let mut out: Vec<(usize, String)> = Vec::new();
    let mut open = false;
    for (line, text) in &step.lines {
        if let Some(rest) = keyed(text, "Check:").or_else(|| keyed(text, "Fix:")) {
            out.push((*line, rest.to_owned()));
            open = true;
        } else if open && !text.trim().is_empty() && !text.trim_start().starts_with("- ") {
            if let Some((_, item)) = out.last_mut() {
                item.push(' ');
                item.push_str(text.trim());
            }
        } else {
            open = false;
        }
    }
    out
}

/// Every command named in a `Check:` or `Fix:` item of `steps`, with the step and line it is on.
fn named(steps: &[Step]) -> Vec<(usize, usize, Vec<String>)> {
    let mut out = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        for (line, rest) in items(step) {
            for span in spans(&rest) {
                for words in commands(span) {
                    out.push((
                        index + 1,
                        line,
                        words.into_iter().map(str::to_owned).collect(),
                    ));
                }
            }
        }
    }
    out
}

/// Why `words`, the words after `conductor`, name no command of `root` or a flag it does not
/// take; `None` when they name one. A word that is no subcommand is a value once the walk reaches
/// a command without subcommands.
fn conductor_problem(root: &clap::Command, words: &[String]) -> Option<String> {
    let mut command = root;
    let mut flagged = false;
    for word in words {
        if let Some(flag) = word.strip_prefix("--") {
            let flag = flag.split('=').next().unwrap_or_default();
            if !command
                .get_arguments()
                .any(|arg| arg.get_long() == Some(flag))
            {
                return Some(format!(
                    "`{}` takes no `--{flag}`",
                    command.get_bin_name().unwrap_or(command.get_name())
                ));
            }
            flagged = true;
            continue;
        }
        let subcommands: Vec<&str> = command
            .get_subcommands()
            .map(clap::Command::get_name)
            .filter(|name| *name != "help")
            .collect();
        if subcommands.is_empty() || flagged {
            // A value: of the flag before it, or of a command that takes no word.
            continue;
        }
        match command.find_subcommand(word) {
            Some(sub) => command = sub,
            None => {
                return Some(format!(
                    "`{}` has no command `{word}`",
                    command.get_bin_name().unwrap_or(command.get_name())
                ));
            }
        }
    }
    let grouped = command
        .get_subcommands()
        .any(|sub| sub.get_name() != "help");
    (grouped && !flagged).then(|| {
        format!(
            "`{}` is a group, not a command",
            command.get_bin_name().unwrap_or(command.get_name())
        )
    })
}

/// The programs the repository's `b10x.toml` pins.
fn pinned() -> Vec<String> {
    let file = workspace().join("b10x.toml");
    let text = fs::read_to_string(&file).expect("the repository has a b10x.toml");
    conductor_cli::doctor::pins(&text, &file)
        .expect("the repository's pins read")
        .into_iter()
        .map(|(program, _)| program)
        .collect()
}

#[test]
fn every_step_has_a_check_a_fix_and_a_hands_line() {
    let steps = steps(&read("SETUP.md"));
    assert!(
        steps.len() >= 9,
        "SETUP.md has {} numbered steps; the story names nine",
        steps.len()
    );
    let mut problems = Vec::new();
    for (index, step) in steps.iter().enumerate() {
        for key in KEYS {
            let found = step
                .lines
                .iter()
                .filter_map(|(_, text)| keyed(text, key))
                .any(|rest| !rest.trim().is_empty());
            if !found {
                problems.push(format!(
                    "SETUP.md:{}: step {} ({}) has no `{key}` line",
                    step.line,
                    index + 1,
                    step.title.trim()
                ));
            }
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

#[test]
fn every_tool_a_step_names_is_conductor_a_pinned_program_or_a_known_one() {
    let steps = steps(&read("SETUP.md"));
    let pinned = pinned();
    let mut root = Cli::command();
    root.build();
    let mut problems = Vec::new();
    let named = named(&steps);
    assert!(
        named.len() >= 9,
        "SETUP.md names only {} commands on its Check: and Fix: lines",
        named.len()
    );
    for (step, line, words) in &named {
        let tool = words[0].as_str();
        if tool == "conductor" {
            if let Some(problem) = conductor_problem(&root, &words[1..]) {
                problems.push(format!(
                    "SETUP.md:{line}: step {step}: `{}`: {problem}",
                    words.join(" ")
                ));
            }
        } else if !TOOLS.contains(&tool) && !pinned.iter().any(|program| program == tool) {
            problems.push(format!(
                "SETUP.md:{line}: step {step}: `{tool}` is not conductor, a program b10x.toml \
                 pins, or one of {TOOLS:?}"
            ));
        }
    }
    assert!(problems.is_empty(), "{}", problems.join("\n"));
}

/// The commands that mark the story's steps, in the order the steps take them.
const ORDER: [&str; 8] = [
    "git remote get-url origin",
    "gh auth status",
    "glab auth status",
    "sha256sum --check",
    "aep plan store migrate git --dry-run",
    "conductor init",
    "conductor trust",
    "conductor doctor",
];

#[test]
fn the_steps_run_in_the_story_s_order_and_end_with_doctor() {
    let steps = steps(&read("SETUP.md"));
    let named = named(&steps);
    let joined: Vec<(usize, String)> = named
        .iter()
        .map(|(step, _, words)| (*step, words.join(" ")))
        .collect();
    let mut last = 0;
    for marker in ORDER {
        let at = joined
            .iter()
            .find(|(step, command)| *step >= last && command.starts_with(marker))
            .map(|(step, _)| *step);
        let Some(step) = at else {
            panic!(
                "no step of SETUP.md from step {last} on names `{marker}`; the steps name the \
                 commands {ORDER:?} in that order"
            );
        };
        last = step;
    }
    let migrate_verify = joined
        .iter()
        .any(|(_, command)| command.starts_with("aep plan store migrate git --verify"));
    assert!(
        migrate_verify,
        "SETUP.md names no `aep plan store migrate git --verify` after its dry run"
    );
    let final_step = steps.last().expect("a step");
    let checks_doctor = final_step
        .lines
        .iter()
        .filter_map(|(_, text)| keyed(text, "Check:"))
        .any(|rest| {
            spans(rest)
                .iter()
                .any(|span| span.trim() == "conductor doctor")
        });
    assert!(
        checks_doctor,
        "the last step of SETUP.md ({}) does not check `conductor doctor`",
        final_step.title.trim()
    );
    let ends_ready = final_step
        .lines
        .iter()
        .any(|(_, text)| text.contains("`ready`"));
    assert!(
        ends_ready,
        "the last step of SETUP.md does not say doctor ends `ready`"
    );
}

/// `story:setup-command`: the step before the last checks with `conductor setup` and fixes with
/// `conductor setup --apply`.
#[test]
fn the_step_before_the_last_applies_conductor_setup() {
    let steps = steps(&read("SETUP.md"));
    assert!(steps.len() >= 2, "SETUP.md has {} steps", steps.len());
    let step = &steps[steps.len() - 2];
    let commands = |key: &str| -> Vec<String> {
        step.lines
            .iter()
            .filter_map(|(_, text)| keyed(text, key))
            .flat_map(|rest| spans(rest).into_iter().map(|span| span.trim().to_owned()))
            .collect()
    };
    assert!(
        commands("Check:")
            .iter()
            .any(|command| command == "conductor setup"),
        "SETUP.md:{}: the step before the last ({}) does not check `conductor setup`",
        step.line,
        step.title.trim()
    );
    assert!(
        commands("Fix:")
            .iter()
            .any(|command| command == "conductor setup --apply"),
        "SETUP.md:{}: the step before the last ({}) does not fix with `conductor setup --apply`",
        step.line,
        step.title.trim()
    );
    let hands = step
        .lines
        .iter()
        .filter_map(|(_, text)| keyed(text, "Hands:"))
        .any(|rest| !rest.trim().is_empty());
    assert!(
        hands,
        "SETUP.md:{}: the step before the last has no Hands: line",
        step.line
    );
}

#[test]
fn the_readme_points_an_agent_at_setup_md_with_its_raw_url() {
    let readme = read("README.md");
    let top: Vec<&str> = readme.lines().take(README_TOP).collect();
    // The sentence may wrap: its paragraph is read as one text.
    let paragraphs: Vec<String> = top
        .split(|line| line.trim().is_empty())
        .map(|lines| lines.join(" "))
        .collect();
    let pointer = paragraphs.iter().find(|paragraph| {
        paragraph.contains("[`SETUP.md`](SETUP.md)")
            && paragraph.contains("agent")
            && paragraph
                .split(['(', ')', '<', '>', ' '])
                .any(|word| word.starts_with(RAW_HOST) && word.ends_with(RAW_END))
    });
    assert!(
        pointer.is_some(),
        "none of the first {README_TOP} lines of README.md tells an agent to follow SETUP.md \
         with its raw URL ({RAW_HOST}…{RAW_END}):\n{}",
        top.join("\n")
    );
}

#[test]
fn the_checks_of_this_test_find_what_they_look_for() {
    let text = "# Setup\n\n1. **One.**\n   - Check: `git remote get-url origin`\n   - Fix: none\n\
                \n2. **Two.**\n   - Check: `conductor doctor`\n   - Fix: `v=1 curl -fsSLO x && \
                tar -xzf y`, then\n     `cargo install --locked z`\n   - Hands: none\n\n\
                ## After\n\n3 not a step\n";
    let steps = steps(text);
    assert_eq!(steps.len(), 2);
    assert_eq!(steps[0].line, 3);
    assert!(
        steps[0]
            .lines
            .iter()
            .all(|(_, line)| keyed(line, "Hands:").is_none())
    );
    let named: Vec<String> = named(&steps)
        .into_iter()
        .map(|(_, _, words)| words[0].clone())
        .collect();
    assert_eq!(named, ["git", "conductor", "curl", "tar", "cargo"]);

    let mut root = Cli::command();
    root.build();
    let words =
        |text: &str| -> Vec<String> { text.split_whitespace().map(str::to_owned).collect() };
    assert_eq!(conductor_problem(&root, &words("doctor")), None);
    assert_eq!(conductor_problem(&root, &words("--version")), None);
    assert_eq!(
        conductor_problem(&root, &words("config show --format json")),
        None
    );
    assert!(conductor_problem(&root, &words("config")).is_some());
    assert!(conductor_problem(&root, &words("no-such-command")).is_some());
    assert!(conductor_problem(&root, &words("doctor --no-such-flag")).is_some());
}
