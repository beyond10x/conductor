//! `story:need-id-unique`: `conductor need new` prints the next `N-<date>-<n>` no earlier NEED
//! used. The ids it printed are kept in the state directory, and the ids the records' hand-overs,
//! decisions and dispatches already cite are skipped, so a restarted session never repeats one.
//!
//! Each case is a directory of its own under this test target's temporary directory: `home/`,
//! `records/`, `state/` and the config file `conductor.yaml` naming them.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::Stdio;

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
            .join("need_id")
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
        fs::write(
            case.dir.join("conductor.yaml"),
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
                records = case.dir.join("records").display(),
                state = case.dir.join("state").display(),
            ),
        )
        .expect("write the config file");
        case
    }

    /// `conductor need new <args>`: its exit success, standard output and standard error.
    fn new_id(&self, args: &[&str]) -> (bool, String, String) {
        let output = common::conductor(self.dir.join("home"))
            .args(["need", "new"])
            .args(args)
            .current_dir(self.dir.join("home"))
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs");
        (
            output.status.success(),
            String::from_utf8(output.stdout).expect("utf-8"),
            String::from_utf8(output.stderr).expect("utf-8"),
        )
    }

    fn id(&self, args: &[&str]) -> String {
        let (ok, out, err) = self.new_id(args);
        assert!(ok, "need new failed: {err}");
        out
    }
}

#[test]
fn consecutive_calls_print_different_ids() {
    let case = Case::new("consecutive");
    assert_eq!(case.id(&["--day", "2026-10-15"]), "N-20261015-01\n");
    assert_eq!(case.id(&["--day", "2026-10-15"]), "N-20261015-02\n");
    assert_eq!(case.id(&["--day", "2026-10-16"]), "N-20261016-01\n");
}

#[test]
fn an_id_a_handover_cites_is_never_printed_again() {
    let case = Case::new("handover");
    let handoff = case.dir.join("records/docs/handoff");
    fs::create_dir_all(&handoff).expect("create the hand-over directory");
    fs::write(
        handoff.join("2026-10-15-0129.md"),
        "Sent `[NEED conductor N-20261015-02] conductor-dev restart: context 900k`.\n",
    )
    .expect("write a hand-over");
    // Nothing was minted before, as after a restart that lost the state: the hand-over decides.
    assert_eq!(case.id(&["--day", "2026-10-15"]), "N-20261015-03\n");
}

#[test]
fn a_day_that_is_no_date_is_refused() {
    let case = Case::new("bad-day");
    let (ok, out, err) = case.new_id(&["--day", "yesterday"]);
    assert!(!ok && out.is_empty(), "no id: {out:?}");
    assert!(
        err.contains("need new") && err.contains("yesterday"),
        "{err}"
    );
}
