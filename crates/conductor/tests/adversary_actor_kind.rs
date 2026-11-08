//! Adversary cases for `story:spec-v1`: the actor a command records is the kind of the program
//! that issued it (design `docs/design/architecture-next.md` § 3 "Actors").
//!
//! Every run sets `HOME` to its case directory and passes `--state-dir` under it
//! (`common::conductor`).

mod common;

use std::fs;
use std::path::PathBuf;
use std::process::{Output, Stdio};

use serde_json::Value;

struct Case {
    home: PathBuf,
    work: PathBuf,
    state: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("adversary_actor_kind")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case");
        }
        let work = root.join("work");
        fs::create_dir_all(&work).expect("create work/");
        Self {
            home: root.join("home"),
            work,
            state: root.join("state"),
        }
    }

    fn run(&self, session: Option<&str>, path: Option<&PathBuf>, args: &[&str]) -> Output {
        let mut command = common::conductor(&self.home);
        command
            .current_dir(&self.work)
            .arg("--state-dir")
            .arg(&self.state)
            .args(args)
            .stdin(Stdio::null());
        if let Some(name) = session {
            command.env(conductor_cli::actor::SESSION_VARIABLE, name);
        }
        if let Some(path) = path {
            command.env("PATH", path);
        }
        command.output().expect("the conductor binary runs")
    }
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The run daemon's cycle runs `conductor snapshot start-snapshot` as a child with only the
/// global `--config` and `--state-dir` (`src/run/dispatch.rs` `run_step`, called at
/// `run/dispatch.rs:52`), in a systemd user unit where no `CONDUCTOR_SESSION` is set. The unit's
/// own decision: "Daemon for `conductor run` and its collectors". The snapshot it records must
/// carry the daemon actor, not the Unix user as a human.
#[test]
fn a_snapshot_the_run_daemon_s_cycle_starts_is_recorded_as_the_daemon() {
    let case = Case::new("daemon-snapshot");
    let bin = case.home.with_file_name("offline");
    fs::create_dir_all(&bin).expect("create the offline PATH");
    for program in ["git", "gh", "aep", "ess", "claude"] {
        let fake = bin.join(program);
        fs::write(&fake, "#!/bin/sh\necho offline >&2\nexit 1\n").expect("write a fake");
        let mut permissions = fs::metadata(&fake).expect("stat").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
        fs::set_permissions(&fake, permissions).expect("make it runnable");
    }
    // Exactly the words `run_step` passes: no --actor.
    let started = case.run(None, Some(&bin), &["snapshot", "start-snapshot"]);
    let shown = case.run(None, None, &["snapshot", "snapshots", "--format", "json"]);
    assert!(shown.status.success(), "{}", describe(&shown));
    let rows: Value = serde_json::from_slice(&shown.stdout).expect("JSON rows");
    let rows = rows.as_array().expect("a list");
    assert_eq!(rows.len(), 1, "{rows:?}; start: {}", describe(&started));
    assert_eq!(
        rows[0]["actor"], "daemon",
        "the daemon's own snapshot is recorded under another actor: {rows:?}"
    );
}

/// An agent session (its provider set `CONDUCTOR_SESSION`) cannot take a grant by writing a
/// human actor on the command line: a grant is checked on the kind, and a human and an agent are
/// told apart by type (§ 3). The refusal text no longer tells the agent to "pass --actor with
/// kind Human".
#[test]
fn an_agent_session_cannot_confirm_a_goal_by_claiming_to_be_human() {
    let case = Case::new("agent-claims-human");
    let proposed = case.run(
        None,
        None,
        &[
            "goal",
            "propose-goal",
            "--goal-id",
            "G1",
            "--title",
            "Ship",
            "--exit-evidence",
            "G1 is listed as met",
        ],
    );
    assert!(proposed.status.success(), "{}", describe(&proposed));
    let plain = case.run(
        Some("example-org-controller"),
        None,
        &["goal", "confirm-goal", "--goal-id", "G1"],
    );
    assert_eq!(plain.status.code(), Some(1), "{}", describe(&plain));
    assert!(
        String::from_utf8_lossy(&plain.stderr).contains("ActorNotHuman"),
        "{}",
        describe(&plain)
    );
    let claimed = case.run(
        Some("example-org-controller"),
        None,
        &[
            "goal",
            "confirm-goal",
            "--goal-id",
            "G1",
            "--actor",
            r#"{"kind":"Human","operator":"example-operator"}"#,
        ],
    );
    assert_eq!(
        claimed.status.code(),
        Some(1),
        "an agent session took a person's grant with --actor: {}",
        describe(&claimed)
    );
}
