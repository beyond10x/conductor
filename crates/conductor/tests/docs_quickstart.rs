//! `story:quickstart-existing-repo`: a guide takes one task in an existing repository from
//! `conductor setup --apply` to a merged pull request, and the first pages a reader opens link it.
//!
//! - `website/docs/guides/first-task-existing-repo.md` exists, and every `conductor` command it
//!   names, in a code span or a fenced block, resolves against the CLI tree: each word a command
//!   of the group before it, each flag one the command takes.
//! - `website/sidebars.ts` lists the guide first in its Guides category.
//! - The guide names `merge: human`, the controller's path in a repository with no planning store,
//!   and the `to run:` line that activates the worktree workspace.
//! - The first 12 lines of `README.md` link the guide and keep the `SETUP.md` pointer
//!   `docs_setup.rs` holds; the last step of `SETUP.md` links it.
//! - `website/docs/guides/troubleshoot.md` has a table row naming `com.apple.provenance`, its
//!   cause (files written by a process Claude Code started) and the workaround (a plain
//!   terminal).
//!
//! Every file is read relative to the workspace root, and a finding names the file and the line.

use std::fs;
use std::path::{Path, PathBuf};

use clap::CommandFactory;
use conductor_cli::cli::Cli;

/// The guide, relative to the workspace root.
const GUIDE: &str = "website/docs/guides/first-task-existing-repo.md";

/// The guide's document id, as the sidebar and links name it.
const GUIDE_ID: &str = "guides/first-task-existing-repo";

/// How many lines at the top of the README the link to the guide must be within.
const README_TOP: usize = 12;

/// The README's pointer to `SETUP.md`, as `docs_setup.rs` holds it.
const SETUP_POINTER: &str = "[`SETUP.md`](SETUP.md)";

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

/// `text` with every run of whitespace one space, so a phrase broken across lines is still found.
fn flattened(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The spans in backticks of `line`.
fn spans(line: &str) -> Vec<&str> {
    line.split('`')
        .enumerate()
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, span)| span)
        .collect()
}

/// Every command line of `page`, with its line number: each code span outside fenced blocks, and
/// in a fenced block that is not `text`, each line after a `$ ` prompt, or every line when the
/// block has no prompt.
fn command_texts(page: &str) -> Vec<(usize, String)> {
    let mut out = Vec::new();
    let mut block: Option<(String, Vec<(usize, String)>)> = None;
    for (at, line) in page.lines().enumerate() {
        let fence = line.trim_start().starts_with("```");
        match block.take() {
            None if fence => {
                let language = line.trim_start().trim_start_matches('`').trim().to_owned();
                block = Some((language, Vec::new()));
            }
            None => {
                for span in spans(line) {
                    out.push((at + 1, span.to_owned()));
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

/// The words after `conductor` of each `conductor` command in `text`, split on `&&`, `||`, `|`,
/// `;` and `$(`, a leading `NAME=value` aside.
fn conductor_commands(text: &str) -> Vec<Vec<String>> {
    text.split("&&")
        .flat_map(|part| part.split("||"))
        .flat_map(|part| part.split(['|', ';']))
        .flat_map(|part| part.split("$("))
        .filter_map(|command| {
            let words: Vec<&str> = command
                .split_whitespace()
                .skip_while(|word| word.contains('=') && !word.starts_with('-'))
                .collect();
            (words.first() == Some(&"conductor")).then(|| {
                words[1..]
                    .iter()
                    .map(|word| word.trim_end_matches(')').to_owned())
                    .collect()
            })
        })
        .collect()
}

/// Why `words`, the words after `conductor`, name no command of `root` or a flag it does not
/// take; `None` when they do. Words stop being commands at the first flag, at a placeholder
/// (`<…>`), or once the walk reaches a command without subcommands; a group named alone is a
/// group the page talks about, and resolves.
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
        if flagged || word.starts_with(['<', '-', '"', '\'', '#']) {
            flagged = true;
            continue;
        }
        let groups = command
            .get_subcommands()
            .any(|sub| sub.get_name() != "help");
        if !groups {
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
    None
}

/// The CLI tree, built, so every command carries the global flags and `--help`.
fn tree() -> clap::Command {
    let mut root = Cli::command();
    root.build();
    root
}

#[test]
fn every_conductor_command_the_guide_names_exists() {
    let root = tree();
    let page = read(GUIDE);
    let mut named = 0;
    let mut findings = Vec::new();
    for (line, text) in command_texts(&page) {
        for words in conductor_commands(&text) {
            if words.is_empty() {
                continue;
            }
            named += 1;
            if let Some(problem) = conductor_problem(&root, &words) {
                findings.push(format!(
                    "{GUIDE}:{line}: `conductor {}`: {problem}",
                    words.join(" ")
                ));
            }
        }
    }
    assert!(
        named >= 8,
        "{GUIDE} names only {named} conductor commands; the guide runs setup, doctor, a goal, a \
         dispatch and its controller"
    );
    assert!(
        findings.is_empty(),
        "{GUIDE} names commands the CLI does not have:\n{}",
        findings.join("\n")
    );
}

#[test]
fn the_sidebar_lists_the_guide_first_under_guides() {
    let guide = workspace().join(GUIDE);
    assert!(guide.is_file(), "{} is not a file", guide.display());
    let sidebar = read("website/sidebars.ts");
    let (_, after) = sidebar
        .split_once("label: 'Guides'")
        .expect("website/sidebars.ts has a category labelled 'Guides'");
    let (_, items) = after
        .split_once("items: [")
        .expect("the Guides category of website/sidebars.ts lists items");
    let first = items
        .split('\'')
        .nth(1)
        .expect("the Guides category of website/sidebars.ts names an item");
    assert_eq!(
        first, GUIDE_ID,
        "website/sidebars.ts lists {first:?} first under Guides, not {GUIDE_ID:?}"
    );
}

#[test]
fn the_guide_names_merge_human_the_no_store_path_and_the_activation_line() {
    let page = flattened(&read(GUIDE));
    for needed in [
        "`merge: human`",
        "no planning store",
        "worktree workspace:",
        "not activated",
        "to run:",
        "worktree activate",
    ] {
        assert!(page.contains(needed), "{GUIDE} does not contain {needed:?}");
    }
}

#[test]
fn the_readme_top_links_the_guide_and_keeps_the_setup_pointer() {
    let readme = read("README.md");
    let top: Vec<&str> = readme.lines().take(README_TOP).collect();
    let top = top.join("\n");
    assert!(
        top.contains(GUIDE_ID),
        "none of the first {README_TOP} lines of README.md links {GUIDE_ID}:\n{top}"
    );
    assert!(
        top.contains(SETUP_POINTER),
        "the first {README_TOP} lines of README.md lost the pointer {SETUP_POINTER}:\n{top}"
    );
}

/// Whether `line` starts a numbered step: digits, `.` and a space, at the start of the line.
fn starts_step(line: &str) -> bool {
    let digits: String = line.chars().take_while(char::is_ascii_digit).collect();
    !digits.is_empty() && line[digits.len()..].starts_with(". ")
}

#[test]
fn the_last_step_of_setup_md_links_the_guide() {
    let setup = read("SETUP.md");
    let lines: Vec<&str> = setup.lines().collect();
    let start = lines
        .iter()
        .rposition(|line| starts_step(line))
        .expect("SETUP.md has numbered steps");
    let step: Vec<&str> = lines[start..]
        .iter()
        .take_while(|line| !line.starts_with('#'))
        .copied()
        .collect();
    assert!(
        step.iter().any(|line| line.contains(GUIDE_ID)),
        "SETUP.md:{}: the last step does not link {GUIDE_ID}:\n{}",
        start + 1,
        step.join("\n")
    );
}

#[test]
fn troubleshooting_has_the_macos_provenance_row() {
    let page = read("website/docs/guides/troubleshoot.md");
    let row = page
        .lines()
        .filter(|line| line.trim_start().starts_with('|'))
        .find(|line| line.contains("com.apple.provenance"))
        .unwrap_or_else(|| {
            panic!(
                "website/docs/guides/troubleshoot.md has no table row naming com.apple.provenance"
            )
        });
    for needed in [
        "`ess generate synthesize`",
        "a process Claude Code started",
        "plain terminal",
    ] {
        assert!(
            row.contains(needed),
            "the com.apple.provenance row of website/docs/guides/troubleshoot.md does not \
             contain {needed:?}: {row}"
        );
    }
}

#[test]
fn the_checks_of_this_test_find_what_they_look_for() {
    let page = "Run `conductor setup --apply`, then `conductor goal goals`.\n\n```console\n\
                $ conductor doctor\nconductor 0.8.2\nready\n```\n\n```text\nconductor nothing here\n\
                ```\n\n```console\nCONDUCTOR_INSTANCE=alpha conductor dispatch nope && conductor \
                spawn --repository <repo>\n```\n";
    let commands: Vec<(usize, Vec<String>)> = command_texts(page)
        .into_iter()
        .flat_map(|(line, text)| {
            conductor_commands(&text)
                .into_iter()
                .map(move |words| (line, words))
        })
        .collect();
    let shown: Vec<String> = commands.iter().map(|(_, words)| words.join(" ")).collect();
    assert_eq!(
        shown,
        [
            "setup --apply",
            "goal goals",
            "doctor",
            "dispatch nope",
            "spawn --repository <repo>",
        ],
        "the commands this test reads from a page"
    );
    let root = tree();
    let problems: Vec<Option<String>> = commands
        .iter()
        .map(|(_, words)| conductor_problem(&root, words))
        .collect();
    assert_eq!(
        problems.iter().filter(|problem| problem.is_some()).count(),
        1,
        "only `conductor dispatch nope` is refused: {problems:?}"
    );
    assert!(
        conductor_problem(&root, &["doctor".to_owned(), "--nope".to_owned()]).is_some(),
        "a flag the command does not take is refused"
    );
    assert!(starts_step("10. **Run doctor.**") && !starts_step("- Check: x"));
}
