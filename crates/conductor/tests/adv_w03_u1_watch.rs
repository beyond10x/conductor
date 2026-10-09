//! Adversary, wave 03 U1 (`story:instance-session-names`): the watch's context labels with two
//! instances on one host.
//!
//! [`Watch::pass`] labels a session's context line by [`conductor_cli::config::active`], which is
//! process-wide; this binary holds one case only, and fixes the active instance before anything
//! reads it, with `HOME` and both `CONDUCTOR_*` variables isolated, so the real home's config is
//! never read.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use conductor_cli::config;
use conductor_cli::watch::{Sources, Watch};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The checkouts root of instance `alpha`, which the fixture sessions live under; it does not
/// exist. The context fixture's transcript is filed under `<ROOT>/conductor`.
const ROOT: &str = "/fixture-home/example-org";

const TREES: &str = "/fixture-home/.local/state/worktree/trees/example-org";

fn noon() -> OffsetDateTime {
    OffsetDateTime::parse("2026-10-07T12:00:00Z", &Rfc3339).expect("a fixed instant")
}

fn free(_: &Path) -> anyhow::Result<u64> {
    Ok(100 << 30)
}

/// Two instances share the host: `alpha` (`session_prefix: a`, the default, the one this
/// watch runs) and `beta` (`session_prefix: b`). Both start conductor-dev with `task dev:start`
/// from the one conductor checkout, `<alpha root>/conductor`: the task has no `cd`. beta's
/// conductor-dev is named `b-conductor-dev`. It works in alpha's checkouts root, so alpha's watch
/// lists it, and over the handover threshold it must not read `context: conductor`, which tells
/// alpha's conductor to hand itself over (`.agents/conductor.md`). Before the story it read
/// `context: conductor-dev`, by its role name.
#[test]
fn adv_w03_u1_another_instance_s_conductor_dev_is_not_labelled_conductor() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv-w03-u1-watch")
        .join("other-dev");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's directory");
    }
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create the case's home");
    let config_file = dir.join("conductor.yaml");
    fs::write(
        &config_file,
        format!(
            "version: conductor.config/1\n\
             default: alpha\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   session_prefix: a\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {ROOT}, trees: {TREES}}}\n\
             \x20 - name: beta\n\
             \x20   session_prefix: b\n\
             \x20   sources: [{{github: beta-org}}]\n\
             \x20   checkouts: {{root: /fixture-home/beta, trees: /fixture-home/beta-trees}}\n"
        ),
    )
    .expect("write the config file");
    // SAFETY: this binary runs this one test; nothing else reads the environment concurrently.
    unsafe {
        std::env::set_var("HOME", &home);
        std::env::remove_var(config::CONFIG_VARIABLE);
        std::env::remove_var(config::INSTANCE_VARIABLE);
    }
    config::set_active(Some(&config_file)).expect("the config file loads");
    assert_eq!(
        config::active()
            .instance
            .session_prefix
            .as_ref()
            .map(|prefix| prefix.0.as_str()),
        Some("a"),
        "the watch runs alpha"
    );

    let dev: Value = json!({
        "id": "bg-b-conductor-dev", "sessionId": "c3c3c3c3-0000-4000-8000-000000000003",
        "name": "b-conductor-dev", "kind": "background", "state": "working", "status": "busy",
        "pid": 2103, "startedAt": 1_791_370_800_000_u64, "cwd": format!("{ROOT}/conductor"),
    });
    fs::write(dir.join("agents.json"), Value::from(vec![dev]).to_string())
        .expect("write the session list");
    let cat = |file: &str| vec![OsString::from("cat"), dir.join(file).into_os_string()];
    let sources = Sources {
        agents: cat("agents.json"),
        root: PathBuf::from(ROOT),
        trees: PathBuf::from(TREES),
        records: None,
        projects: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/watch/context"),
        dispatches: dir.join("dispatches"),
        organization: "example-org".to_owned(),
        store: None,
        repositories: Vec::new(),
        runs: vec!["false".to_owned()],
        disk: dir.join("free.txt"),
        free_bytes: free,
        notify: vec![OsString::from("true")],
        clock: noon,
        bound: Duration::from_secs(10),
    };
    let mut watch = Watch::new(sources, dir.join("state/watch"));
    let mut notes = Vec::new();
    let lines = watch.pass(false, &mut notes).expect("the pass ends");
    assert!(
        !lines
            .iter()
            .any(|line| line.starts_with("context: conductor ")),
        "beta's conductor-dev hands alpha's conductor over: {lines:?}"
    );
}
