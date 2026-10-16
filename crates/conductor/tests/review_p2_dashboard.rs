//! Probes of the external review of 0.14.0 (rollup, dashboard health and trust items).
mod active;
mod common;
mod series_fixture;

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpStream};
use std::path::{Path, PathBuf};
use std::process::{Child, Stdio};
use std::time::Duration;

use conductor_cli::collect::{Collector, OverBudget};
use conductor_cli::snapshot;
use conductor_model::observation::{SnapshotState, StartSnapshot};
use conductor_model::primitives::Timestamp;
use serde_json::{Value, json};

#[test]
fn an_over_budget_forge_does_not_count_open_prs_as_closed() {
    let case = series_fixture::Case::new("review-dashboard", "over-budget-rollup");
    case.take(series_fixture::A, series_fixture::day_a());
    let repositories = series_fixture::collector(vec![], vec![], vec![], vec![]);
    let forge = Collector::new("github", |_| Err(anyhow::Error::new(OverBudget)));
    let taken = snapshot::take_from(
        case.store(),
        StartSnapshot {
            started_at: Timestamp(series_fixture::B.to_owned()),
            disk_free_bytes: 1_000_000,
            budget: None,
        },
        &[repositories, forge],
    )
    .expect("budget exhaustion still completes the snapshot");
    assert_eq!(taken.state, SnapshotState::Complete);
    let closed: Vec<_> = case
        .store()
        .daily_series()
        .into_iter()
        .filter(|held| held.data.series == "prs_closed_unmerged")
        .map(|held| (held.data.key, held.data.items))
        .collect();
    assert!(
        closed.is_empty(),
        "no closure was observed, but rollups contain {closed:?}"
    );
}

fn config(home: &Path) -> PathBuf {
    fs::create_dir_all(home.join("checkouts/group/repo/.git")).unwrap();
    fs::write(
        home.join("checkouts/group/repo/.git/config"),
        "[remote \"origin\"]\nurl = git@gitlab.example.invalid:group/repo.git\n",
    )
    .unwrap();
    fs::create_dir_all(home.join("records")).unwrap();
    fs::create_dir_all(home.join("empty-path")).unwrap();
    let file = home.join("custom.json");
    fs::write(
        &file,
        serde_json::to_vec_pretty(&json!({
            "version": "conductor.config/1",
            "instances": [{
                "name": "example-org",
                "sources": [{"gitlab": "group", "host": "gitlab.example.invalid"}],
                "checkouts": {"root": home.join("checkouts"), "trees": home.join("trees")},
                "records": home.join("records"),
                "state": home.join("state"),
                "cache": home.join("cache")
            }]
        }))
        .unwrap(),
    )
    .unwrap();
    file
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn dashboard_doctor_checks_the_explicit_config_file() {
    active::isolate();
    let case = series_fixture::Case::new("review-dashboard", "explicit-config");
    let home = case.dir.join("home");
    let file = config(&home);
    let child = common::conductor(&home)
        .args([
            "--config",
            file.to_str().unwrap(),
            "dashboard",
            "serve",
            "--port",
            "0",
        ])
        .env("PATH", home.join("empty-path"))
        .current_dir(&home)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut server = Server(child);
    let mut line = String::new();
    BufReader::new(server.0.stdout.take().unwrap())
        .read_line(&mut line)
        .unwrap();
    let address = line
        .trim()
        .strip_prefix("listening on http://")
        .expect("server bound");
    let mut latest = Value::Null;
    for _ in 0..50 {
        let mut stream = TcpStream::connect(address).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        write!(
            stream,
            "GET /api/health HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n"
        )
        .unwrap();
        stream.shutdown(Shutdown::Write).unwrap();
        let mut response = String::new();
        stream.read_to_string(&mut response).unwrap();
        latest = serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
        if !latest["doctor"]["taken_at"].is_null() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    let rows = latest["doctor"]["rows"]
        .as_array()
        .expect("doctor completed");
    let glab = rows
        .iter()
        .filter_map(Value::as_str)
        .find(|line| line.starts_with("glab:"))
        .expect("glab row");
    assert!(
        !glab.contains("not needed"),
        "explicit config has a GitLab source, but health reports {glab}"
    );
}

#[test]
fn trust_covers_nested_gitlab_checkouts() {
    active::isolate();
    let case = series_fixture::Case::new("review-dashboard", "nested-trust");
    let home = case.dir.join("home");
    let file = config(&home);
    fs::write(home.join(".claude.json"), "{\"projects\":{}}\n").unwrap();
    let parsed = conductor_cli::config::parse(&fs::read_to_string(&file).unwrap(), &home).unwrap();
    assert_eq!(
        conductor_cli::config::instance_checkouts(&parsed.instances[0], 0).unwrap(),
        vec![home.join("checkouts/group/repo")],
        "this is an actual instance member"
    );
    let output = common::conductor(&home)
        .args(["--config", file.to_str().unwrap(), "trust"])
        .current_dir(&home)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let written: Value =
        serde_json::from_slice(&fs::read(home.join(".claude.json")).unwrap()).unwrap();
    let checkout = fs::canonicalize(home.join("checkouts/group/repo")).unwrap();
    assert_eq!(
        written["projects"][checkout.to_str().unwrap()]["hasTrustDialogAccepted"],
        true,
        "trust exited successfully but skipped the configured GitLab checkout: {written}"
    );
}
