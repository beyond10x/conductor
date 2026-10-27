//! `story:slice-remove`: `conductor host slices` lists each systemd user slice of the
//! `conductor` family with its unit count and the instance it belongs to, and `--gc` stops each
//! empty slice that no instance of the config file names, never one that still holds a unit.
//!
//! The commands run the built binary with a fake `systemctl` first on `PATH`. It appends its
//! arguments to `calls` in the case's `fake/` directory, answers `list-units` with the file
//! `list-units` and `show` with the file `show`, and succeeds at anything else, `stop` included.
//! No real systemd is asked.
//!
//! The config file names one instance, `example-org`, whose `session_prefix` is `ex`, so its
//! slice is `conductor_ex.slice`.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::Stdio;

use serde_json::{Value, json};

/// The fake `systemctl`.
const FAKE: &str = r#"#!/bin/sh
echo "$*" >> "$FAKE_DIR/calls"
case "$*" in
  *list-units*) cat "$FAKE_DIR/list-units" ;;
  *" show "*) cat "$FAKE_DIR/show" ;;
esac
exit 0
"#;

/// What the manager has loaded: the slice of the instance (empty), a slice of an instance that is
/// gone and empty, a slice of one that is gone and still holding a unit, and a slice of another
/// tool.
const LIST_UNITS: &str = "\
conductor_ex.slice loaded active active Slice of ex
conductor_gone.slice loaded inactive dead Slice of gone
conductor_busy.slice loaded active active Slice of busy
other.slice loaded active active Slice of another tool
";

/// The manager's units with their slice: one in `conductor_busy.slice`, one in another tool's.
const SHOW: &str = "\
Id=conductor_busy-conductor.service
Slice=conductor_busy.slice

Id=other-thing.service
Slice=other.slice

Id=conductor_gone.slice
Slice=-.slice

";

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
            .join("host_slices")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "records", "bin", "fake"] {
            fs::create_dir_all(dir.join(sub)).expect("create a case directory");
        }
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        let systemctl = case.dir.join("bin/systemctl");
        fs::write(&systemctl, FAKE).expect("write the fake systemctl");
        fs::set_permissions(&systemctl, fs::Permissions::from_mode(0o755))
            .expect("make the fake systemctl run");
        fs::write(case.dir.join("fake/list-units"), LIST_UNITS).expect("write list-units");
        fs::write(case.dir.join("fake/show"), SHOW).expect("write show");
        fs::write(
            case.dir.join("conductor.yaml"),
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   session_prefix: ex\n\
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

    /// `conductor host slices <args>`: its exit success, standard output and standard error.
    fn slices(&self, args: &[&str]) -> (bool, String, String) {
        let path = format!(
            "{}:{}",
            self.dir.join("bin").display(),
            std::env::var("PATH").unwrap_or_default()
        );
        let output = common::conductor(self.dir.join("home"))
            .args(["host", "slices"])
            .args(args)
            .current_dir(self.dir.join("home"))
            .env("CONDUCTOR_CONFIG", self.dir.join("conductor.yaml"))
            .env("PATH", path)
            .env("FAKE_DIR", self.dir.join("fake"))
            .stdin(Stdio::null())
            .output()
            .expect("conductor runs");
        (
            output.status.success(),
            String::from_utf8(output.stdout).expect("utf-8"),
            String::from_utf8(output.stderr).expect("utf-8"),
        )
    }

    /// The calls the fake `systemctl` got, one line each.
    fn calls(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("fake/calls"))
            .unwrap_or_default()
            .lines()
            .map(str::to_owned)
            .collect()
    }

    fn stops(&self) -> Vec<String> {
        self.calls()
            .into_iter()
            .filter(|call| call.contains(" stop "))
            .collect()
    }
}

#[test]
fn slices_lists_each_slice_with_its_units_and_instance() {
    let case = Case::new("list");
    let (ok, out, err) = case.slices(&["--format", "json"]);
    assert!(ok, "host slices failed: {err}");
    let rows: Value = serde_json::from_str(&out).expect("a JSON list");
    assert_eq!(
        rows,
        json!([
            {"slice": "conductor_busy.slice", "instance": null, "units": 1, "stopped": false},
            {"slice": "conductor_ex.slice", "instance": "example-org", "units": 0, "stopped": false},
            {"slice": "conductor_gone.slice", "instance": null, "units": 0, "stopped": false},
        ])
    );
    assert!(case.stops().is_empty(), "a list stops nothing");

    let (ok, text, err) = case.slices(&[]);
    assert!(ok, "host slices failed: {err}");
    assert_eq!(
        text,
        "conductor_busy.slice\t1\t-\n\
         conductor_ex.slice\t0\texample-org\n\
         conductor_gone.slice\t0\t-\n"
    );
}

#[test]
fn gc_stops_only_an_empty_slice_no_instance_names() {
    let case = Case::new("gc");
    let (ok, out, err) = case.slices(&["--gc"]);
    assert!(ok, "host slices --gc failed: {err}");
    assert_eq!(
        case.stops(),
        ["--user stop conductor_gone.slice"],
        "the empty slice of an instance that is gone, and nothing else"
    );
    assert_eq!(
        out,
        "conductor_busy.slice\t1\t-\n\
         conductor_ex.slice\t0\texample-org\n\
         conductor_gone.slice\t0\t-\tstopped\n"
    );
}

#[test]
fn a_format_that_is_neither_text_nor_json_is_refused() {
    let case = Case::new("format");
    let (ok, out, err) = case.slices(&["--format", "yaml"]);
    assert!(!ok && out.is_empty(), "no rows: {out:?}");
    assert!(err.contains("host slices") && err.contains("yaml"), "{err}");
    assert!(case.calls().is_empty(), "nothing was asked of systemd");
}
