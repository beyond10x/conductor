//! `story:policy-secret-drift`: an instance's `secret_syncs` name a file an organization's own
//! tool publishes into GitHub Actions secrets; the `secrets` collector records, per sync, the last
//! commit of the source path and the oldest `updated_at` of its secrets; the board flags
//! `secret stale: <sync>` when the source is newer. What the conductor profile does with the flag
//! is held in `tests/profiles.rs`.
//!
//! Every case keeps its files in its own directory in this test target's temporary directory.
//! The source repository is a git checkout the case builds beside a bare origin; `gh` is a fake
//! the case writes, which answers a fixture of `tests/fixtures/secrets/` per endpoint and logs
//! what it was asked. No case reaches the network, a real checkout or the real home.

mod active;
mod common;
mod scratch;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::Duration;

use conductor_cli::collect::{self, Collector, secrets};
use conductor_cli::config;
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::config::{ForgeSecret, LocalSource, SecretSource, SecretSync, Source};
use conductor_model::observation::{
    RecordSecretSync, RepositoryName, SnapshotState, StartSnapshot,
};
use conductor_model::primitives::Timestamp;
use serde_json::Value;

/// Before anything the cases care about: the source repository's first commit.
const OLD: &str = "2026-10-01T09:00:00Z";

/// When the policy file the stale sync publishes was last changed.
const SOURCE: &str = "2026-10-13T10:00:00Z";

/// A later commit that touches another file of the source repository.
const LATER: &str = "2026-10-15T10:00:00Z";

/// A fresh directory for one case, with `home/` and an empty `work/`, removed when the case
/// passes ([`scratch::Scratch`]): the source repository it builds there is a git repository.
fn case_dir(case: &str) -> scratch::Scratch {
    active::isolate();
    let scratch = scratch::Scratch::new(Path::new("secret_syncs").join(case));
    for sub in ["work", "home", "workspace", "origins", "bin"] {
        fs::create_dir_all(scratch.path().join(sub)).expect("create the case's directories");
    }
    scratch
}

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/secrets")
        .join(name)
}

fn text(path: &Path) -> &str {
    path.to_str().expect("the case's path is UTF-8")
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// `git <args>` in `dir`, every date `date`, reading no configuration but the case's own.
fn git(dir: &Path, date: &str, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", date)
        .env("GIT_COMMITTER_DATE", date)
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Writes `files` under `dir` and commits them as `subject` at `date`.
fn commit(dir: &Path, date: &str, subject: &str, files: &[(&str, &str)]) {
    for (path, body) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().expect("a file has a parent"))
            .expect("create the file's directory");
        fs::write(&path, body).expect("write the file");
    }
    git(dir, date, &["add", "--all"]);
    git(dir, date, &["commit", "--quiet", "-m", subject]);
}

/// The source repository `policy` under the case's `workspace/`, pushed to its bare origin:
/// `ci/policy.json` and `docs/policy.json` from [`OLD`], `ci/policy.json` changed at [`SOURCE`],
/// and `README.md` changed at [`LATER`], so the newest commit of the branch touches neither
/// policy file.
fn policy_repository(dir: &Path) -> PathBuf {
    let origin = dir.join("origins/policy.git");
    fs::create_dir_all(&origin).expect("create the origin");
    git(
        &origin,
        OLD,
        &["init", "--quiet", "--bare", "--initial-branch=main"],
    );
    let checkout = dir.join("workspace/policy");
    fs::create_dir_all(&checkout).expect("create the checkout");
    git(
        &checkout,
        OLD,
        &["init", "--quiet", "--initial-branch=main"],
    );
    git(&checkout, OLD, &["remote", "add", "origin", text(&origin)]);
    commit(
        &checkout,
        OLD,
        "Start the policies",
        &[
            ("ci/policy.json", "{}\n"),
            ("docs/policy.json", "{}\n"),
            ("README.md", "# policies\n"),
        ],
    );
    commit(
        &checkout,
        SOURCE,
        "Tighten the CI policy",
        &[("ci/policy.json", "{\"strict\": true}\n")],
    );
    commit(
        &checkout,
        LATER,
        "Reword the README",
        &[("README.md", "# the policies\n")],
    );
    git(&checkout, OLD, &["push", "--quiet", "origin", "main"]);
    checkout
}

/// A fake `gh` under the case's `bin/`: `gh api <endpoint>` prints the fixture of the endpoint,
/// and any other endpoint is GitHub's 404. Every call is appended to the case's `gh.log`.
fn fake_gh(dir: &Path) -> PathBuf {
    let gh = dir.join("bin/gh");
    let script = format!(
        "#!/bin/sh\n\
         echo \"$*\" >> '{log}'\n\
         case \"$2\" in\n\
         \x20 orgs/example-org/actions/secrets/CI_POLICY) cat '{org}' ;;\n\
         \x20 repos/example-org/ci/actions/secrets/CI_POLICY) cat '{repo}' ;;\n\
         \x20 *) echo 'gh: Not Found (HTTP 404)' >&2; exit 1 ;;\n\
         esac\n",
        log = text(&dir.join("gh.log")),
        org = text(&fixture("org-ci-policy.json")),
        repo = text(&fixture("repo-ci-policy.json")),
    );
    fs::write(&gh, script).expect("write the fake gh");
    let mut permissions = fs::metadata(&gh).expect("stat the fake gh").permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, 0o755);
    fs::set_permissions(&gh, permissions).expect("make the fake gh executable");
    gh
}

fn org_secret(name: &str) -> ForgeSecret {
    ForgeSecret {
        owner: "example-org".to_owned(),
        repository: None,
        name: name.to_owned(),
    }
}

fn repo_secret(repository: &str, name: &str) -> ForgeSecret {
    ForgeSecret {
        owner: "example-org".to_owned(),
        repository: Some(repository.to_owned()),
        name: name.to_owned(),
    }
}

fn sync(name: &str, path: &str, secrets: Vec<ForgeSecret>) -> SecretSync {
    SecretSync {
        name: name.to_owned(),
        source: SecretSource {
            repository: "policy".to_owned(),
            branch: "main".to_owned(),
            path: path.to_owned(),
        },
        secrets,
        command: format!("example-sync publish {path}"),
        dry_run: None,
    }
}

/// Takes one snapshot into the case's store through `collectors`, started at `started_at`.
fn take(dir: &Path, started_at: &str, collectors: &[Collector]) -> Taken {
    let store = store::open(&dir.join("state")).expect("open the store");
    let start = StartSnapshot {
        started_at: Timestamp(started_at.to_owned()),
        disk_free_bytes: 1_000_000,
        budget: None,
    };
    snapshot::take_from(store, start, collectors).expect("the driver ends the snapshot")
}

/// Runs the binary from the case's `work/` over its `state/`, with `HOME` naming its `home/`.
fn conductor(dir: &Path, args: &[&str]) -> Output {
    common::conductor(dir.join("home"))
        .current_dir(dir.join("work"))
        .arg("--state-dir")
        .arg(dir.join("state"))
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs")
}

/// The rows of `snapshot secret-syncs --format json` over the case's store, by sync name.
fn secret_sync_rows(dir: &Path) -> Vec<Value> {
    let output = conductor(dir, &["snapshot", "secret-syncs", "--format", "json"]);
    assert!(output.status.success(), "{}", describe(&output));
    let mut rows: Vec<Value> = match serde_json::from_slice(&output.stdout) {
        Ok(Value::Array(rows)) => rows,
        other => panic!(
            "`snapshot secret-syncs` printed {other:?}: {}",
            describe(&output)
        ),
    };
    rows.sort_by(|left, right| left["sync"].as_str().cmp(&right["sync"].as_str()));
    rows
}

/// Acceptance 1, the snapshot: per sync, the committer time of the last commit on
/// `origin/<branch>` that touched the source path, and the oldest `updated_at` of its secrets,
/// each in UTC; a secret GitHub does not answer leaves the oldest write absent and names the read
/// that failed, and the snapshot completes.
#[test]
fn the_collector_records_each_sync_s_source_commit_and_its_oldest_secret_write() {
    let case = case_dir("collector");
    let dir = case.path();
    policy_repository(dir);
    let gh = fake_gh(dir);
    let sources = secrets::Sources {
        syncs: vec![
            sync(
                "ci-policy",
                "ci/policy.json",
                vec![org_secret("CI_POLICY"), repo_secret("ci", "CI_POLICY")],
            ),
            sync(
                "docs-policy",
                "docs/policy.json",
                vec![repo_secret("ci", "CI_POLICY")],
            ),
            sync("missing", "ci/policy.json", vec![org_secret("NO_SUCH")]),
        ],
        root: dir.join("workspace"),
        secret: vec![
            text(&gh).to_owned(),
            "api".to_owned(),
            "{endpoint}".to_owned(),
        ],
        bound: Duration::from_secs(60),
    };
    let taken = take(
        dir,
        "2026-10-16T08:00:00Z",
        &[Collector::new("secrets", move |record| {
            secrets::collect_from(&sources, record)
        })],
    );
    assert_eq!(taken.state, SnapshotState::Complete, "{taken:?}");

    let rows = secret_sync_rows(dir);
    let synced: Vec<(&str, &str, &str, &str, Value, Value)> = rows
        .iter()
        .map(|row| {
            (
                row["sync"].as_str().unwrap_or_default(),
                row["repository"].as_str().unwrap_or_default(),
                row["branch"].as_str().unwrap_or_default(),
                row["path"].as_str().unwrap_or_default(),
                row["source_committed_at"].clone(),
                row["secrets_updated_at"].clone(),
            )
        })
        .collect();
    assert_eq!(
        synced,
        vec![
            (
                "ci-policy",
                "policy",
                "main",
                "ci/policy.json",
                Value::from(SOURCE),
                Value::from("2026-10-12T08:00:00Z"),
            ),
            (
                "docs-policy",
                "policy",
                "main",
                "docs/policy.json",
                Value::from(OLD),
                // `+02:00` in the fixture, recorded in UTC.
                Value::from("2026-10-14T07:00:00Z"),
            ),
            (
                "missing",
                "policy",
                "main",
                "ci/policy.json",
                Value::from(SOURCE),
                Value::Null,
            ),
        ],
        "{rows:#?}"
    );
    assert_eq!(rows[0]["error_line"], Value::Null, "{rows:#?}");
    let error = rows[2]["error_line"].as_str().unwrap_or_default();
    assert!(
        error.contains("api orgs/example-org/actions/secrets/NO_SUCH") && error.contains("404"),
        "the failed read is named: {error:?}"
    );

    // Reads only: every call is `gh api <endpoint>`, a GET.
    let log = fs::read_to_string(dir.join("gh.log")).expect("read the fake gh's log");
    assert!(!log.is_empty(), "gh was asked");
    for line in log.lines() {
        assert!(
            line.starts_with("api ")
                && !line.contains("--method")
                && !line.contains("-X")
                && !line.contains("-f "),
            "a read only: {line:?}"
        );
    }
}

/// The live sources read each secret through `gh api` at its GitHub endpoint, and an instance
/// with a secret sync starts `gh` even without a `github` source.
#[test]
fn the_live_sources_read_secrets_with_gh_api_and_need_gh() {
    let case = case_dir("live");
    let dir = case.path();
    let mut instance = config::built_in(&dir.join("home"), &dir.join("work"));
    instance.sources = vec![Source::Local(LocalSource {
        path: text(&dir.join("workspace")).to_owned(),
        exclude: Vec::new(),
    })];
    assert!(
        !collect::programs(&instance).contains(&"gh"),
        "no github source and no sync: no gh"
    );
    instance.secret_syncs = vec![sync("ci-policy", "ci/policy.json", vec![])];
    assert!(
        collect::programs(&instance).contains(&"gh"),
        "a sync's secrets are read with gh"
    );

    let sources = secrets::Sources::of(&instance);
    assert_eq!(sources.secret, ["gh", "api", "{endpoint}"]);
    assert_eq!(sources.root, PathBuf::from(&instance.checkouts.root));
    assert_eq!(sources.syncs, instance.secret_syncs);
    assert_eq!(
        secrets::endpoint(&org_secret("CI_POLICY")),
        "orgs/example-org/actions/secrets/CI_POLICY"
    );
    assert_eq!(
        secrets::endpoint(&repo_secret("ci", "CI_POLICY")),
        "repos/example-org/ci/actions/secrets/CI_POLICY"
    );
    assert!(
        collect::registered()
            .iter()
            .any(|collector| collector.name() == "secrets"),
        "start-snapshot runs the secrets collector"
    );
}

/// A collector that records each row of the fixture `name` as a `RecordSecretSync`.
fn from_fixture(name: &str) -> Collector {
    let rows: Vec<Value> =
        serde_json::from_str(&fs::read_to_string(fixture(name)).expect("read the fixture"))
            .expect("the fixture is JSON");
    Collector::new("fixture", move |record: &mut Recorder<'_>| {
        let snapshot = record.snapshot().clone();
        let word = |row: &Value, key: &str| row[key].as_str().map(str::to_owned);
        for row in &rows {
            record.secret_sync(RecordSecretSync {
                snapshot_id: snapshot.clone(),
                sync: word(row, "sync").expect("a sync"),
                repository: RepositoryName(word(row, "repository").expect("a repository")),
                branch: word(row, "branch").expect("a branch"),
                path: word(row, "path").expect("a path"),
                source_committed_at: word(row, "source_committed_at").map(Timestamp),
                secrets_updated_at: word(row, "secrets_updated_at").map(Timestamp),
                error_line: word(row, "error_line"),
            })?;
        }
        Ok(())
    })
}

/// The rows of the Markdown table under `heading` in `board`, header first, cells trimmed.
fn table(board: &str, heading: &str) -> Vec<Vec<String>> {
    board
        .lines()
        .skip_while(|line| *line != heading)
        .skip(1)
        .take_while(|line| !line.starts_with('#'))
        .filter(|line| line.starts_with('|'))
        .map(|line| {
            line.trim()
                .trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().to_owned())
                .collect::<Vec<String>>()
        })
        .filter(|cells| !cells.iter().all(|cell| cell.chars().all(|c| c == '-')))
        .collect()
}

/// Acceptance 1, the board: over fixtures, the sync whose source was committed after its oldest
/// secret was written is `secret stale: <sync>` on its source repository's row, one whose secret
/// could not be read is `secret unread: <sync>`, a fresh one is not named, and the delta against
/// a snapshot where it was fresh adds the flag.
#[test]
fn the_board_flags_a_secret_older_than_its_source() {
    let case = case_dir("board");
    let dir = case.path();
    let fresh = take(
        dir,
        "2026-10-12T10:00:00Z",
        &[from_fixture("board-fresh.json")],
    );
    assert_eq!(fresh.state, SnapshotState::Complete, "{fresh:?}");
    let stale = take(
        dir,
        "2026-10-14T10:00:00Z",
        &[from_fixture("board-stale.json")],
    );
    assert_eq!(stale.state, SnapshotState::Complete, "{stale:?}");

    let publish = |snapshot: &str| {
        let board = dir.join("out").join(format!("{snapshot}.md"));
        let output = conductor(
            dir,
            &[
                "snapshot",
                "publish-board",
                "--snapshot-id",
                snapshot,
                "--path",
                text(&board),
            ],
        );
        assert!(output.status.success(), "{}", describe(&output));
        fs::read_to_string(&board).expect("read the board")
    };

    let board = publish(&fresh.snapshot_id.0.0);
    assert!(
        !board.contains("secret stale") && !board.contains("secret unread"),
        "no sync is stale in the fresh snapshot:\n{board}"
    );

    let board = publish(&stale.snapshot_id.0.0);
    let deviations = table(&board, "## Deviations");
    assert_eq!(
        deviations,
        vec![
            vec![
                "repository".to_owned(),
                "deviations".to_owned(),
                "failing checks".to_owned()
            ],
            vec![
                "policy".to_owned(),
                "secret stale: ci-policy, secret unread: missing".to_owned(),
                String::new()
            ],
        ],
        "{board}"
    );
    let delta = table(&board, "## Delta");
    assert_eq!(delta[1][0], "policy", "{board}");
    assert_eq!(
        delta[1][1], "+secret stale: ci-policy, +secret unread: missing",
        "{board}"
    );
}

/// Acceptance 1, the config: `secret_syncs` lists each sync's source (`branch` `main` when
/// absent), the secrets it feeds and the command that publishes it; `config show` writes them,
/// and an instance without the key has none.
#[test]
fn config_show_writes_each_secret_sync_and_fills_the_branch() {
    let case = case_dir("config");
    let dir = case.path();
    let file = dir.join("conductor.yaml");
    let write = |syncs: &str| {
        fs::write(
            &file,
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: example-org\n\
                 \x20   sources: [{{github: example-org}}]\n\
                 \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
                 {syncs}",
                root = dir.join("workspace"),
                trees = dir.join("trees"),
            ),
        )
        .expect("write the config file");
    };
    let show = || {
        common::conductor(dir.join("home"))
            .current_dir(dir.join("work"))
            .env("CONDUCTOR_CONFIG", &file)
            .args(["config", "show", "--format", "json"])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    };

    write("");
    let output = show();
    assert!(output.status.success(), "{}", describe(&output));
    let shown: Value = serde_json::from_slice(&output.stdout).expect("config show prints JSON");
    assert_eq!(
        shown["instances"][0]["secret_syncs"],
        serde_json::json!([]),
        "{shown:#}"
    );

    write(
        "\x20   secret_syncs:\n\
         \x20     - name: ci-policy\n\
         \x20       source: {repository: policy, path: ci/policy.json}\n\
         \x20       secrets:\n\
         \x20         - {owner: example-org, name: CI_POLICY}\n\
         \x20         - {owner: example-org, repository: ci, name: CI_POLICY}\n\
         \x20       command: example-sync publish ci/policy.json\n\
         \x20       dry_run: example-sync publish ci/policy.json --dry-run\n",
    );
    let output = show();
    assert!(output.status.success(), "{}", describe(&output));
    let shown: Value = serde_json::from_slice(&output.stdout).expect("config show prints JSON");
    assert_eq!(
        shown["instances"][0]["secret_syncs"],
        serde_json::json!([{
            "name": "ci-policy",
            "source": {"repository": "policy", "branch": "main", "path": "ci/policy.json"},
            "secrets": [
                {"owner": "example-org", "repository": null, "name": "CI_POLICY"},
                {"owner": "example-org", "repository": "ci", "name": "CI_POLICY"},
            ],
            "command": "example-sync publish ci/policy.json",
            "dry_run": "example-sync publish ci/policy.json --dry-run",
        }]),
        "{shown:#}"
    );
}

/// A sync whose source path climbs out of its repository, one with no secret, and two syncs of
/// one name are each refused by their YAML path.
#[test]
fn a_secret_sync_that_cannot_be_read_is_refused_by_its_path() {
    let case = case_dir("refused");
    let dir = case.path();
    let file = dir.join("conductor.yaml");
    fs::write(
        &file,
        format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   secret_syncs:\n\
             \x20     - name: ci-policy\n\
             \x20       source: {{repository: policy, path: ../ci/policy.json}}\n\
             \x20       secrets: [{{owner: example-org, name: CI_POLICY}}]\n\
             \x20       command: example-sync publish\n\
             \x20     - name: ci-policy\n\
             \x20       source: {{repository: policy, path: ci/policy.json}}\n\
             \x20       secrets: []\n\
             \x20       command: example-sync publish\n",
            root = dir.join("workspace"),
            trees = dir.join("trees"),
        ),
    )
    .expect("write the config file");
    let output = common::conductor(dir.join("home"))
        .current_dir(dir.join("work"))
        .env("CONDUCTOR_CONFIG", &file)
        .args(["config", "validate"])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert_eq!(output.status.code(), Some(1), "{}", describe(&output));
    let stderr = String::from_utf8_lossy(&output.stderr);
    for path in [
        "instances[0].secret_syncs[0].source.path",
        "instances[0].secret_syncs[1].secrets",
        "instances[0].secret_syncs[1].name",
    ] {
        assert!(stderr.contains(path), "names {path}: {stderr}");
    }
}
