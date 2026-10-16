//! Probes of the external review of 0.14.0 (storage items): the records log and the goal evidence.

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

struct Case {
    root: PathBuf,
    records: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("review_storage_probes")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).unwrap();
        }
        for directory in [
            "home",
            "records",
            "state",
            "var-tmp",
            "bin",
            "checkouts",
            "trees",
        ] {
            fs::create_dir_all(root.join(directory)).unwrap();
        }
        let root = root.canonicalize().unwrap();
        let records = root.join("records");
        let case = Self { root, records };
        case.config();
        case
    }
    fn config(&self) {
        fs::write(self.root.join("conductor.yaml"), format!(
            "version: conductor.config/1\ninstances:\n  - name: example-org\n    sources: [{{github: example-org}}]\n    checkouts: {{root: {:?}, trees: {:?}}}\n    records: {:?}\n    state: {:?}\n    cache: {:?}\n",
            self.root.join("checkouts"), self.root.join("trees"), self.records,
            self.root.join("state"), self.root.join("cache"))).unwrap();
    }
    fn command(&self) -> Command {
        let mut command = common::conductor(self.root.join("home"));
        command
            .current_dir(&self.root)
            .env("CONDUCTOR_CONFIG", self.root.join("conductor.yaml"));
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> String {
        let out = self.run(args);
        assert!(out.status.success(), "{}", describe(&out));
        String::from_utf8(out.stdout).unwrap().trim().to_owned()
    }
}

fn describe(out: &Output) -> String {
    format!(
        "exit {:?}\nstdout={}\nstderr={}",
        out.status.code(),
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn report_done_can_recover_its_evidence_after_a_records_log_write_failure() {
    let case = Case::new("report-log-failure");
    case.ok(&[
        "goal",
        "propose-goal",
        "--goal-id",
        "G1",
        "--title",
        "pilot",
        "--exit-evidence",
        "works",
    ]);
    let id = case.ok(&[
        "dispatch",
        "send-dispatch",
        "--repository",
        "alpha",
        "--goal-id",
        "G1",
        "--brief",
        "ship",
        "--sent-at",
        "2026-10-15T00:00:00Z",
    ]);
    let logfile = fs::read_dir(case.records.join("dispatches"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let saved = case.root.join("saved-log");
    fs::rename(&logfile, &saved).unwrap();
    fs::create_dir(&logfile).unwrap();
    let args = [
        "dispatch",
        "report-done",
        "--dispatch-id",
        id.as_str(),
        "--evidence",
        "exact-release-proof",
    ];
    let first = case.run(&args);
    assert!(
        !first.status.success(),
        "The fixture must fail the records write: {}",
        describe(&first)
    );
    fs::remove_dir(&logfile).unwrap();
    fs::rename(&saved, &logfile).unwrap();
    let retried = case.run(&args);
    let log = fs::read_to_string(&logfile).unwrap();
    assert!(
        retried.status.success() && log.contains("exact-release-proof"),
        "The committed completion cannot recover its evidence. First: {}\nRetry: {}\nLog: {}",
        describe(&first),
        describe(&retried),
        log
    );
}
