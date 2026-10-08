//! Adversary, wave 06 U1 (`story:decision-commands`, the decision group), pass 1: the decision
//! views' renderings of free text.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

fn run(state: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_conductor"))
        .arg("--state-dir")
        .arg(state)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

/// The cells of one GitHub-flavoured Markdown table row: split on every `|` that is not escaped,
/// where a `\` escapes the character after it, a `\` included.
fn gfm_cells(row: &str) -> usize {
    let inner = row
        .trim_end()
        .strip_prefix('|')
        .and_then(|row| row.strip_suffix('|'))
        .expect("a row starts and ends with |");
    let mut cells = 1;
    let mut escaped = false;
    for character in inner.chars() {
        match (escaped, character) {
            (true, _) => escaped = false,
            (false, '\\') => escaped = true,
            (false, '|') => cells += 1,
            (false, _) => {}
        }
    }
    cells
}

/// `--format markdown` escapes `|` as `\|` and leaves `\` as it is, so a question holding `\|`
/// (a BRE alternation such as `grep 'a\|b'`) renders as `\\|`: an escaped backslash and then a
/// cell separator. The row gets 11 cells under a header of 10, and every column after the
/// question shifts by one.
#[test]
fn adv_a_backslash_before_a_pipe_keeps_the_markdown_row_at_its_columns() {
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_decision_views")
        .join("markdown-backslash-pipe");
    if root.exists() {
        fs::remove_dir_all(&root).expect("clear the case's directory");
    }
    let state = root.join("state");
    let recorded = run(
        &state,
        &[
            "decision",
            "record-operator-decision",
            "--decision-class",
            "C",
            "--question",
            r"match with grep 'a\|b'?",
            "--options",
            "A or B",
            "--choice",
            "A",
            "--reason",
            "r",
            "--evidence",
            "e",
            "--decided-at",
            "2026-10-07T10:00:00Z",
        ],
    );
    assert!(recorded.status.success(), "the decision is recorded");
    let output = run(&state, &["decision", "decisions", "--format", "markdown"]);
    assert!(output.status.success());
    let table = String::from_utf8(output.stdout).expect("UTF-8");
    let lines: Vec<&str> = table.lines().collect();
    assert_eq!(lines.len(), 3, "header, separator, one row: {table:?}");
    assert_eq!(
        gfm_cells(lines[2]),
        gfm_cells(lines[0]),
        "the row has as many cells as the header: {:?}",
        lines[2]
    );
}
