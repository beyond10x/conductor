//! `story:terminal-log-name`: the session recording is the terminal log. No page under
//! `website/docs/` and no `--help` text says "typescript" except once, to name the old file
//! `<session>.typescript` for the migration. The match is on the lower-case word: "TypeScript",
//! the language the dashboard app is written in, is another word.

use std::fs;
use std::path::{Path, PathBuf};

use clap::{Command, CommandFactory as _};
use conductor_cli::cli::Cli;

/// The word the story retires.
const WORD: &str = "typescript";

/// The `--help` text of `command` and of every command below it.
fn helps(command: &mut Command, out: &mut Vec<(String, String)>, path: &str) {
    let path = format!("{path} {}", command.get_name());
    out.push((path.clone(), command.render_long_help().to_string()));
    for sub in command.get_subcommands_mut() {
        helps(sub, out, &path);
    }
}

/// Every Markdown page under `dir`.
fn pages(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read the docs directory") {
        let path = entry.expect("a docs entry").path();
        if path.is_dir() {
            pages(&path, out);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "md" || ext == "mdx")
        {
            out.push(path);
        }
    }
}

#[test]
fn no_help_text_says_typescript() {
    let mut command = Cli::command();
    command.build();
    let mut all = Vec::new();
    helps(&mut command, &mut all, "");
    assert!(all.len() > 10, "the walk reached the subcommands");
    let saying: Vec<&String> = all
        .iter()
        .filter(|(_, help)| help.contains(WORD))
        .map(|(path, _)| path)
        .collect();
    assert!(saying.is_empty(), "--help says {WORD}: {saying:?}");
}

#[test]
fn the_docs_say_typescript_once_to_name_the_old_file() {
    let docs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../website/docs");
    let mut all = Vec::new();
    pages(&docs, &mut all);
    assert!(!all.is_empty(), "pages under {}", docs.display());
    let mut mentions = Vec::new();
    for page in &all {
        let text = fs::read_to_string(page).expect("read a page");
        for line in text.lines().filter(|line| line.contains(WORD)) {
            mentions.push((page.display().to_string(), line.to_owned()));
        }
    }
    assert!(mentions.len() <= 1, "more than one mention: {mentions:#?}");
    for (page, line) in &mentions {
        assert_eq!(
            line.matches(WORD).count(),
            1,
            "{page}: one mention, the old file: {line}"
        );
        assert!(
            line.contains("<session>.typescript"),
            "{page}: the one mention names the old file: {line}"
        );
    }
}
