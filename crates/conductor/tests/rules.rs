//! `story:rules-show`: `conductor rules show` prints items of the instance's `rules.md` verbatim,
//! by number (`--item 43,49,51`) or by heading (`--section <heading>`), wherever they sit in the
//! file, and exits 1 naming an item number or heading the file does not hold. It reads `rules.md`
//! only: it writes nothing and opens no store.
//!
//! Each case is a directory of its own under this test target's temporary directory: `home/`, the
//! `HOME` the binary runs with; `records/`, the instance's records directory, holding a copy of
//! `tests/fixtures/rules/rules.md`, whose items are out of numeric order; and the config file
//! `conductor.yaml` naming them. The state directory it names is never created.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Output, Stdio};

use serde_json::{Value, json};

/// Item 1 of the fixture, byte for byte.
const ITEM_1: &str = "\
1. Read the newest hand-over first, then the open dispatches.
   Source: the operator, 2026-10-01.
";

/// Item 2 of the fixture, which sits after item 43.
const ITEM_2: &str = "\
2. Answer a report within one cycle.
";

/// Item 43 of the fixture: two paragraphs, the second after a blank line.
const ITEM_43: &str = "\
43. **Never discard uncommitted changes.** A tree with changes is archived, never removed:
   `worktree finish --archive`.

   Ask before removing a tree that is not yours.
   Source: `decisions/2026-10.jsonl`, 2026-10-03.
";

/// Item 49 of the fixture, under the controller's section, after item 51.
const ITEM_49: &str = "\
49. **No work files in `/tmp`.** Scratch lives under `$TMPDIR` or `~/.cache/<repo>-<purpose>/`.
   Source: the operator, 2026-10-02.
";

/// Item 51 of the fixture, with a nested list.
const ITEM_51: &str = "\
51. **Builds and disk.** Build into the tree's own `target/`:
    - never set `CARGO_TARGET_DIR`;
    - every build runs under `conductor build run`.
    Source: the operator, 2026-10-05.
";

/// Item 102 of the fixture, under a second heading named `Conductor`.
const ITEM_102: &str = "\
102. **A decision never waits on a session that is gone.** Re-dispatch it.
     Source: the operator, 2026-10-09.
";

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rules/rules.md")
}

struct Case {
    dir: PathBuf,
}

/// A case that passed leaves nothing behind; a failed one keeps its directory to read.
impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("rules")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "records"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        fs::copy(fixture(), case.records().join("rules.md")).expect("copy the fixture");
        fs::write(
            case.config(),
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root}, trees: {trees}}}\n\
                 \x20   records: {records}\n\
                 \x20   state: {state}\n",
                root = case.dir.join("home/example-org").display(),
                trees = case.dir.join("trees").display(),
                records = case.records().display(),
                state = case.state().display(),
            ),
        )
        .expect("write the config file");
        case
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn config(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// `conductor rules show <args>` from the case's home, with its config file.
    fn show(&self, args: &[&str]) -> Output {
        common::conductor(self.dir.join("home"))
            .args(["rules", "show"])
            .args(args)
            .current_dir(self.dir.join("home"))
            .env("CONDUCTOR_CONFIG", self.config())
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs")
    }

    /// Runs `rules show` that must succeed, and answers its standard output as written.
    fn ok(&self, args: &[&str]) -> String {
        let output = self.show(args);
        assert!(
            output.status.success(),
            "`conductor rules show {}`: {}",
            args.join(" "),
            describe(&output)
        );
        String::from_utf8(output.stdout).expect("stdout is UTF-8")
    }

    /// Runs `rules show` that must exit 1 with nothing on standard output, and answers its
    /// standard error.
    fn refused(&self, args: &[&str]) -> String {
        let output = self.show(args);
        assert_eq!(
            output.status.code(),
            Some(1),
            "`conductor rules show {}` exits 1: {}",
            args.join(" "),
            describe(&output)
        );
        assert!(
            output.stdout.is_empty(),
            "a refused `rules show` prints no item: {}",
            describe(&output)
        );
        String::from_utf8(output.stderr).expect("stderr is UTF-8")
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}\nstdout:\n{}\nstderr:\n{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Items printed together: each verbatim, one blank line between two.
fn joined(items: &[&str]) -> String {
    items.join("\n")
}

/// Every constant above is the fixture's own bytes, so an output equal to them is verbatim.
#[test]
fn the_expected_items_are_the_fixture_s_bytes() {
    let text = fs::read_to_string(fixture()).expect("read the fixture");
    for item in [ITEM_1, ITEM_2, ITEM_43, ITEM_49, ITEM_51, ITEM_102] {
        assert!(
            text.contains(item),
            "the fixture holds, byte for byte:\n{item}"
        );
    }
    let at = |item: &str| text.find(item).expect("the item is in the fixture");
    assert!(
        at(ITEM_43) < at(ITEM_2) && at(ITEM_51) < at(ITEM_49),
        "the fixture's items are out of numeric order"
    );
}

#[test]
fn items_print_verbatim_in_the_order_asked_wherever_they_sit() {
    let case = Case::new("items");
    assert_eq!(
        case.ok(&["--item", "43,49,51"]),
        joined(&[ITEM_43, ITEM_49, ITEM_51])
    );
    assert_eq!(case.ok(&["--item", "2,43"]), joined(&[ITEM_2, ITEM_43]));
    assert_eq!(case.ok(&["--item", "51,1"]), joined(&[ITEM_51, ITEM_1]));
    assert_eq!(case.ok(&["--item", "102"]), ITEM_102);
    assert_eq!(case.ok(&["--item", " 3 , 2 "]).lines().count(), 3);
}

#[test]
fn a_section_prints_every_item_under_each_heading_of_that_name() {
    let case = Case::new("sections");
    // Both headings named `Conductor`, with their subsections, in file order; any case.
    let conductor = joined(&[ITEM_1, ITEM_43, ITEM_2, ITEM_51, ITEM_102]);
    assert_eq!(case.ok(&["--section", "conductor"]), conductor);
    assert_eq!(case.ok(&["--section", "Conductor"]), conductor);
    assert_eq!(case.ok(&["--section", "Briefs"]), ITEM_51);
    assert_eq!(
        case.ok(&["--section", "repository controller"]),
        joined(&[ITEM_49, "3. Report `started` before the first change.\n"])
    );
}

#[test]
fn an_item_or_section_the_file_does_not_hold_exits_1_and_is_named() {
    let case = Case::new("missing");
    let error = case.refused(&["--item", "43,999,51"]);
    assert!(
        error.contains("RuleItemNotFound") && error.contains("999"),
        "the refusal names the error and the item: {error}"
    );
    assert_eq!(error.trim().lines().count(), 1, "one line: {error}");
    let error = case.refused(&["--section", "Release"]);
    assert!(
        error.contains("RuleSectionNotFound") && error.contains("Release"),
        "the refusal names the error and the heading: {error}"
    );
    let error = case.refused(&["--item", "forty"]);
    assert!(error.contains("forty"), "names the word: {error}");
}

#[test]
fn json_rows_carry_item_title_section_source_and_text() {
    let case = Case::new("json");
    let rows: Value = serde_json::from_str(&case.ok(&["--item", "43,2", "--format", "json"]))
        .expect("--format json prints JSON");
    assert_eq!(
        rows,
        json!([
            {
                "item": 43,
                "title": "Never discard uncommitted changes",
                "section": "Conductor / Session start",
                "source": "`decisions/2026-10.jsonl`, 2026-10-03.",
                "text": ITEM_43,
            },
            {
                "item": 2,
                "title": "Answer a report within one cycle",
                "section": "Conductor / Session start",
                "source": null,
                "text": ITEM_2,
            },
        ])
    );
}

#[test]
fn it_reads_rules_md_only_and_creates_nothing() {
    let case = Case::new("reads-only");
    let before = listing(&case.dir);
    case.ok(&["--section", "conductor"]);
    case.ok(&["--item", "1"]);
    assert!(
        !case.state().exists(),
        "rules show created the state directory"
    );
    assert_eq!(listing(&case.dir), before, "rules show wrote a file");
}

#[test]
fn a_missing_rules_md_is_named() {
    let case = Case::new("no-file");
    fs::remove_file(case.records().join("rules.md")).expect("remove rules.md");
    let output = case.show(&["--item", "1"]);
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("rules.md"),
        "{}",
        describe(&output)
    );
}

/// Every path under `dir`, sorted.
fn listing(dir: &Path) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(next) = stack.pop() {
        for entry in fs::read_dir(&next).expect("read a directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                stack.push(path.clone());
            }
            paths.push(path);
        }
    }
    paths.sort();
    paths
}
