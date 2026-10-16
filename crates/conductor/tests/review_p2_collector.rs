//! Probes of the external review of 0.14.0 (collector items).

mod active;

use std::process::Command;
use std::time::{Duration, Instant};
use std::{fs, path::PathBuf};

use conductor_cli::collect::{Collector, blockers};
use conductor_cli::{config, snapshot, store, watch};
use conductor_model::config::{GitHubSource, Source};
use conductor_model::observation::SnapshotState;
use serde_json::json;

fn scratch(name: &str) -> PathBuf {
    active::isolate();
    let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("review-collector-probes")
        .join(name);
    if root.exists() {
        fs::remove_dir_all(&root).unwrap();
    }
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn collector_bound_includes_draining_inherited_output_pipes() {
    active::isolate();
    let start = Instant::now();
    let result = conductor_cli::collect::run(
        Command::new("sh").args(["-c", "sleep 2 & exit 0"]),
        Duration::from_millis(100),
    );
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(700),
        "100ms bound took {elapsed:?}; result={result:?}"
    );
}

#[test]
fn blockers_preserve_grouped_repository_identity() {
    let root = scratch("grouped-blockers");
    let members = root.join("members.json");
    for group in ["a", "b"] {
        fs::create_dir_all(root.join(group).join("api")).unwrap();
    }
    fs::write(&members, json!({"members": [
        {"name": "a-api", "source": "../../a/api", "tree": root.join("a/api"), "state": "present"},
        {"name": "b-api", "source": "../../b/api", "tree": root.join("b/api"), "state": "present"}
    ]}).to_string()).unwrap();
    let mut sources = blockers::Sources::exporting(root.clone(), root.join("exports"));
    sources.members = vec!["cat".into(), members.into_os_string()];
    let taken = snapshot::take(
        store::open(&root.join("state")).unwrap(),
        &[Collector::new("blockers", move |recorder| {
            blockers::collect_from(&sources, recorder)
        })],
    )
    .unwrap();
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");
}

#[test]
fn watch_uses_each_repositories_github_owner() {
    let root = scratch("watch-multiple-owners");
    let mut instance = config::active().instance.clone();
    instance.sources = vec![
        Source::GitHub(GitHubSource {
            owner: "example-first".into(),
        }),
        Source::GitHub(GitHubSource {
            owner: "example-second".into(),
        }),
    ];
    let active = config::Active {
        instance,
        from_file: true,
        others: vec![],
        default: None,
    };
    let mut sources = watch::Sources::new(root.join("home"), &active, root.join("state"));
    let agents = root.join("agents.json");
    fs::write(&agents, "[]").unwrap();
    fs::create_dir_all(root.join("example-second")).unwrap();
    fs::write(root.join("example-second/beta.json"), json!([{
        "databaseId": 42, "workflowName": "check", "status": "completed", "conclusion": "success",
        "createdAt": "2026-10-15T10:00:00Z"
    }]).to_string()).unwrap();
    sources.agents = vec!["cat".into(), agents.into_os_string()];
    sources.store = None;
    sources.repositories = vec!["beta".into()];
    sources.runs = vec![
        "cat".into(),
        format!("{}/{{organization}}/{{repository}}.json", root.display()),
    ];
    sources.free_bytes = |_| Ok(1000 * 1024 * 1024 * 1024);
    let mut watch = watch::Watch::new(sources, root.join("watch"));
    let mut notes = Vec::new();
    watch.pass(true, &mut notes).unwrap();
    assert!(notes.is_empty(), "{}", String::from_utf8_lossy(&notes));
    assert!(
        fs::read_to_string(root.join("watch/ci.state"))
            .unwrap()
            .contains("beta|check|42|success")
    );
}
