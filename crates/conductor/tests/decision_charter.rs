//! `story:decision-reads-charter`: `conductor decision record-conductor-decision --repository <R>`
//! prints the "Do not touch" and standing-rule lines of R's charter, `charters/<R>.md` in the
//! records directory the config file names, on standard error before it records.
//!
//! A "Do not touch" line is every line of the paragraph or list item that says "do not touch", in
//! any case and wrapped over lines or not. A standing-rule line is every line of a section whose
//! heading starts with "Standing" (`## Standing decisions`, `## Standing: …`), its heading
//! included. Blank lines are left out. A charter that holds neither, a repository with no charter
//! and a decision about no repository print nothing.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use conductor_cli::config::CONFIG_VARIABLE;

/// The instant the decisions here are decided at.
const AT: &str = "2026-10-07T10:00:00Z";

/// The repository the decisions here are about.
const REPOSITORY: &str = "example-widget";

/// A charter with both kinds of line, and lines that are neither.
const CHARTER: &str = "# example-widget charter\n\
\n\
Goal: G-1. Scope: the widget library.\n\
\n\
## Read first\n\
\n\
Read the operator's standing rules first.\n\
\n\
## Standing decisions\n\
\n\
- Build into the tree's own `target/`.\n\
- No release without a green gate;\n\
\x20 a red gate is reported.\n\
\n\
## Queued for the first dispatch\n\
\n\
1. Fix the parser.\n\
2. Leave the fixture\n\
\x20  alone. Do not touch the vendored copy.\n\
3. Release it.\n\
\n\
Do not\n\
touch the nightly run example-run-1: not its timer,\n\
not its state.\n\
A start by hand breaks it.\n\
\n\
Priority: widget work comes first.\n\
\n\
## Standing: one gate per release\n\
\n\
Run the full gate once, on the release commit.\n\
\n\
## Done\n\
\n\
Nothing here.\n";

/// What `CHARTER` prints after its first line, in the charter's order.
const PRINTED: &str = "## Standing decisions\n\
- Build into the tree's own `target/`.\n\
- No release without a green gate;\n\
\x20 a red gate is reported.\n\
2. Leave the fixture\n\
\x20  alone. Do not touch the vendored copy.\n\
Do not\n\
touch the nightly run example-run-1: not its timer,\n\
not its state.\n\
A start by hand breaks it.\n\
## Standing: one gate per release\n\
Run the full gate once, on the release commit.\n";

/// One case: a home with a config file naming `~/records`, and a state directory.
struct Case {
    root: PathBuf,
}

impl Case {
    fn new(case: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("decision-charter")
            .join(case);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        fs::create_dir_all(root.join("home/records/charters")).expect("create the records");
        fs::create_dir_all(root.join("work")).expect("create the working directory");
        fs::write(
            root.join("conductor.yaml"),
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   sources: [{github: example-org}]\n\
             \x20   checkouts: {root: ~/checkouts, trees: ~/trees}\n\
             \x20   records: ~/records\n\
             \x20   state: ~/state\n",
        )
        .expect("write the config file");
        Self { root }
    }

    fn charter_path(&self, repository: &str) -> PathBuf {
        self.root
            .join("home/records/charters")
            .join(format!("{repository}.md"))
    }

    fn charter(&self, repository: &str, text: &str) {
        fs::write(self.charter_path(repository), text).expect("write the charter");
    }

    /// Runs `record-conductor-decision` of class `class` with `more` after its fields.
    fn record(&self, class: &str, more: &[&str]) -> Output {
        let mut command: Command = common::conductor(self.root.join("home"));
        command
            .current_dir(self.root.join("work"))
            .env(CONFIG_VARIABLE, self.root.join("conductor.yaml"))
            .arg("--state-dir")
            .arg(self.root.join("state"))
            .args([
                "decision",
                "record-conductor-decision",
                "--decision-class",
                class,
                "--question",
                "which way?",
                "--options",
                "A: one way; B: the other",
                "--choice",
                "A",
                "--reason",
                "A is reversible",
                "--evidence",
                "the request's own text",
                "--decided-at",
                AT,
            ])
            .args(more)
            .stdin(Stdio::null());
        command.output().expect("the conductor binary runs")
    }
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("stderr is UTF-8")
}

fn recorded(output: &Output) -> String {
    assert!(
        output.status.success(),
        "the decision is recorded; exit {:?}, stderr {}",
        output.status.code(),
        stderr(output)
    );
    let stdout = String::from_utf8(output.stdout.clone()).expect("stdout is UTF-8");
    assert!(
        stdout.starts_with("DEC-20261007-") && stdout.lines().count() == 1,
        "stdout is the decision's id alone: {stdout:?}"
    );
    stdout
}

/// The first line names the repository and its charter's path; the charter's lines follow.
fn assert_charter_printed(case: &Case, stderr: &str) {
    let (first, rest) = stderr
        .split_once('\n')
        .unwrap_or_else(|| panic!("stderr has a first line: {stderr:?}"));
    assert!(
        first.starts_with("decision record-conductor-decision: ")
            && first.contains(REPOSITORY)
            && first.contains(&case.charter_path(REPOSITORY).display().to_string()),
        "the first line names the command, the repository and the charter: {first:?}"
    );
    assert!(
        rest.starts_with(PRINTED),
        "the charter's \"Do not touch\" and standing-rule lines follow, in order; stderr \
         {stderr:?}"
    );
}

#[test]
fn a_decision_about_a_repository_prints_its_charter_s_do_not_touch_and_standing_lines() {
    let case = Case::new("printed");
    case.charter(REPOSITORY, CHARTER);
    let output = case.record("C", &["--repository", REPOSITORY]);
    recorded(&output);
    let stderr = stderr(&output);
    assert_charter_printed(&case, &stderr);
    assert_eq!(
        stderr.lines().count(),
        1 + PRINTED.lines().count(),
        "nothing else is printed: {stderr:?}"
    );
}

/// The lines come before the record: a decision id already held is refused after them.
#[test]
fn the_charter_is_printed_before_the_store_answers() {
    let case = Case::new("before");
    case.charter(REPOSITORY, CHARTER);
    let given = [
        "--repository",
        REPOSITORY,
        "--decision-id",
        "DEC-20261007-01",
    ];
    recorded(&case.record("C", &given));
    let output = case.record("C", &given);
    assert_eq!(output.status.code(), Some(1), "the held id is refused");
    let stderr = stderr(&output);
    assert_charter_printed(&case, &stderr);
    assert!(
        stderr
            .lines()
            .last()
            .is_some_and(|line| line.contains("already exists")),
        "the refusal comes last: {stderr:?}"
    );
}

#[test]
fn a_charter_without_such_lines_no_charter_and_no_repository_print_nothing() {
    let case = Case::new("nothing");
    case.charter(
        REPOSITORY,
        "# example-widget charter\n\n## Read first\n\nRead the standing rules first.\n\n\
         ## Queued\n\n- Fix the parser.\n",
    );
    case.charter("example-gadget", CHARTER);
    for more in [
        &["--repository", REPOSITORY][..],
        &["--repository", "example-other"][..],
        &[][..],
    ] {
        let output = case.record("C", more);
        recorded(&output);
        assert_eq!(stderr(&output), "", "nothing on stderr for {more:?}");
    }
}

/// A class O refusal stays one line: the charter is read only for a decision that is recorded.
#[test]
fn a_reserved_class_is_refused_on_one_line() {
    let case = Case::new("reserved");
    case.charter(REPOSITORY, CHARTER);
    let output = case.record("O", &["--repository", REPOSITORY]);
    assert_eq!(output.status.code(), Some(1), "class O is refused");
    let stderr = stderr(&output);
    assert_eq!(stderr.lines().count(), 1, "one line: {stderr:?}");
    assert!(stderr.contains("ReservedForOperator"), "{stderr:?}");
}
