//! `story:workspace-snapshot-end-to-end`: the workspace file `conductor config workspace` writes is
//! read by both the blockers and the specifications collectors in one snapshot.
//!
//! The fixture instance holds what each of three live snapshots failed on once (conductor's NEED
//! N-20261012-01, -02 and -03): the records repository, whose member the command writes and whose
//! directory `records` is `example-archive` on the forge; a checkout with no `origin` remote; and
//! a checkout with a dot in its name. Each checkout's `origin` is a bare repository under the
//! case's `forge/`, named as the forge names it, so the collectors' own `git fetch --quiet origin`
//! runs as it does live, with no network: it succeeds for each checkout with an origin and fails
//! for one without, as it did in snapshot 01a12d8a.
//!
//! The command is the built binary, run through [`common::conductor`] with the case's config
//! file. The snapshot is taken through the library ([`snapshot::take`]) with both collectors over
//! the written file; `aep` and `ess` are the installed ones, and `gh repo list` is `cat` on a
//! listing the case writes. Blockers are read back through `snapshot blockers`, specifications
//! through the generated query.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::collect::{Collector, blockers, specifications};
use conductor_cli::snapshot::{self, Taken};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::observation::SnapshotState;
use conductor_model::observation::obligations::SpecificationsQuery;
use serde_json::{Value, json};

/// The case's directory: `forge/` with one bare repository per forge name, `checkouts/` (the
/// instance's `checkouts.root`), `records/`, `state/`, `home/` and the config file.
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
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("workspace_snapshot")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "forge", "checkouts", "records", "state"] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        let dir = dir.canonicalize().expect("the case's directory");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    /// A git repository at `dir` holding a planning store of `scope` and `files`, committed on
    /// `main`. With `forge`, its `origin` is the bare repository `forge/<forge>.git`, and `main` is
    /// pushed there; without, it has no remote.
    fn repository(&self, dir: &Path, scope: &str, files: &[(String, String)], forge: Option<&str>) {
        fs::create_dir_all(dir).expect("create the checkout");
        git(dir, &["init", "--quiet"]);
        put(dir, &[vec![project(scope)], files.to_vec()].concat());
        git(dir, &["add", "--all"]);
        git(dir, &["commit", "--quiet", "-m", "main"]);
        if let Some(forge) = forge {
            let bare = self.dir.join("forge").join(format!("{forge}.git"));
            git(&self.dir, &["init", "--quiet", "--bare", path(&bare)]);
            git(dir, &["remote", "add", "origin", path(&bare)]);
            git(dir, &["push", "--quiet", "origin", "main"]);
        }
    }

    /// The config file of the one instance `example-org`.
    fn config(&self) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   checkouts:\n\
             \x20     root: {root}\n\
             \x20     trees: {trees}\n\
             \x20   records: {records}\n\
             \x20   sources:\n\
             \x20     - github: example-org\n",
            root = self.dir.join("checkouts").display(),
            trees = self.dir.join("trees").display(),
            records = self.records().display(),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// `conductor <args>` under the case's home, with `config` as the config file.
    fn conductor(&self, config: Option<&Path>, args: &[&str]) -> Output {
        let mut command = common::conductor(self.home());
        if let Some(config) = config {
            command.env("CONDUCTOR_CONFIG", config);
        }
        command
            .current_dir(&self.dir)
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }
}

fn path(path: &Path) -> &str {
    path.to_str().expect("the case's path is UTF-8")
}

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().expect("a file has a parent")).expect("create the parent");
    fs::write(path, text).expect("write the file");
}

fn put(dir: &Path, files: &[(String, String)]) {
    for (file, text) in files {
        write(&dir.join(file), text);
    }
}

fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
}

/// Runs git in `dir`, with no configuration of the user's, and answers its standard output.
fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "commit.gpgsign=false",
            "-c",
            "init.defaultBranch=main",
            "-c",
            "core.hooksPath=/dev/null",
        ])
        .args(args)
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {}: {}",
        args.join(" "),
        describe(&output)
    );
    String::from_utf8(output.stdout)
        .expect("git prints UTF-8")
        .trim()
        .to_owned()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A planning store's project file.
fn project(scope: &str) -> (String, String) {
    (
        ".engineering/project.yaml".to_owned(),
        format!(
            "version: aep.project/5\nprotocol: adp/1\nprofile: development.standard\n\
             planning_scope: \"{scope}\"\nstore:\n  git: {{}}\n"
        ),
    )
}

/// One open blocker artifact of a planning store, at its path.
fn blocker(id: &str, title: &str) -> (String, String) {
    let (kind, slug) = id.split_once(':').expect("an id is kind:slug");
    (
        format!(".engineering/planning/{kind}/{slug}.md"),
        format!(
            "---\nformat: aep.planning-md/3\nid: {id}\nkind: {kind}\nstatus: open\n\
             title: {title}\nrevision: 1\n---\n# {title}\n"
        ),
    )
}

#[test]
fn the_workspace_config_workspace_writes_is_read_by_both_collectors_in_one_snapshot() {
    let case = Case::new("end_to_end");
    let agents = |name: &str| (String::from("AGENTS.md"), format!("# {name}\n"));
    case.repository(
        &case.dir.join("checkouts/alpha"),
        "alpha",
        &[
            agents("alpha"),
            blocker("operator-blocker:pick", "Pick a store"),
        ],
        Some("alpha"),
    );
    // A dotted checkout name: the member is `beta-io`, the repository `beta.io`.
    case.repository(
        &case.dir.join("checkouts/beta.io"),
        "beta-io",
        &[agents("beta.io")],
        Some("beta.io"),
    );
    // A local-only checkout: no `origin`, so no `origin/main` to read (N-20261012-02).
    case.repository(
        &case.dir.join("checkouts/scratch"),
        "scratch",
        &[agents("scratch")],
        None,
    );
    // The records: `records` on disk, `example-archive` on the forge (N-20261012-01, -03).
    case.repository(
        &case.records(),
        "records",
        &[
            agents("records"),
            blocker("decision-blocker:archive", "Keep the archive"),
        ],
        Some("example-archive"),
    );

    // The owner's listing names each repository as the forge does.
    let taken = case.snapshot(
        r#"[{"name": "alpha", "isArchived": false}, {"name": "beta.io", "isArchived": false},
            {"name": "example-archive", "isArchived": false}]"#,
    );

    // The file the snapshot read: the records member ends in its own name, and the local-only
    // checkout is no member.
    assert_eq!(
        case.members(),
        vec![
            ("alpha".to_owned(), "../../checkouts/alpha".to_owned()),
            ("beta-io".to_owned(), "../../checkouts/beta.io".to_owned()),
            ("records".to_owned(), "../../records".to_owned()),
        ]
    );
    assert_eq!(
        case.specified(&taken),
        vec!["alpha", "beta.io", "example-archive"],
        "one specification row per listed repository"
    );

    let mut expected = vec![
        json!({"repository": "alpha", "reference": "operator-blocker:pick",
               "blocker_kind": "operator-blocker", "title": "Pick a store"}),
        json!({"repository": "records", "reference": "decision-blocker:archive",
               "blocker_kind": "decision-blocker", "title": "Keep the archive"}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(case.blockers(&taken), expected);
}

/// `story:init-records-without-origin`: the records `conductor init` makes have no `origin` until
/// the operator adds one, yet `config workspace` writes them as a member. The snapshot completes:
/// the blockers collector reads the records' store at their `HEAD`, as committed, and runs no
/// `git fetch` there, which would fail for want of an `origin`.
#[test]
fn records_with_no_origin_are_read_at_their_head_without_a_fetch() {
    let case = Case::new("records_without_origin");
    let agents = |name: &str| (String::from("AGENTS.md"), format!("# {name}\n"));
    case.repository(
        &case.dir.join("checkouts/alpha"),
        "alpha",
        &[
            agents("alpha"),
            blocker("operator-blocker:pick", "Pick a store"),
        ],
        Some("alpha"),
    );
    // The records as `conductor init` leaves them: committed, with no remote.
    case.repository(
        &case.records(),
        "records",
        &[
            agents("records"),
            blocker("decision-blocker:archive", "Keep the archive"),
        ],
        None,
    );
    // A blocker in the records' working tree, never committed: `HEAD` does not hold it.
    put(
        &case.records(),
        &[blocker("decision-blocker:draft", "Not committed")],
    );

    // The forge lists only what has an origin there; the records are not on it yet.
    let taken = case.snapshot(r#"[{"name": "alpha", "isArchived": false}]"#);

    assert_eq!(
        case.members(),
        vec![
            ("alpha".to_owned(), "../../checkouts/alpha".to_owned()),
            ("records".to_owned(), "../../records".to_owned()),
        ]
    );
    assert_eq!(
        case.specified(&taken),
        vec!["alpha"],
        "one specification row per listed repository"
    );

    let mut expected = vec![
        json!({"repository": "alpha", "reference": "operator-blocker:pick",
               "blocker_kind": "operator-blocker", "title": "Pick a store"}),
        json!({"repository": "records", "reference": "decision-blocker:archive",
               "blocker_kind": "decision-blocker", "title": "Keep the archive"}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(case.blockers(&taken), expected);
}

impl Case {
    /// `config workspace` under the case's config file, then one snapshot of the blockers and the
    /// specifications collectors over the file it wrote, `gh repo list` answering `listing`;
    /// asserted complete.
    fn snapshot(&self, listing: &str) -> Taken {
        let config = self.config();
        let output = self.conductor(Some(&config), &["config", "workspace"]);
        assert!(
            output.status.success(),
            "config workspace: {}",
            describe(&output)
        );

        let repositories = self.dir.join("repositories.json");
        write(&repositories, listing);
        let blocker_sources = blockers::Sources::new(self.records(), &self.state());
        let mut specification_sources = specifications::Sources::new(self.records(), &self.state());
        specification_sources.repositories = words(&["cat", path(&repositories)]);
        assert_eq!(
            blocker_sources.fetch,
            words(&["git", "fetch", "--quiet", "origin"]),
            "the collectors fetch as they do live"
        );

        let store = store::open(&self.state()).expect("open the store");
        let taken = snapshot::take(
            store,
            &[
                Collector::new("blockers", move |record| {
                    blockers::collect_from(&blocker_sources, record)
                }),
                Collector::new("specifications", move |record| {
                    specifications::collect_from(&specification_sources, record)
                }),
            ],
        )
        .expect("the driver ends the snapshot");

        assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
        taken
    }

    /// Each member of the workspace file `config workspace` wrote, as (name, source), sorted.
    fn members(&self) -> Vec<(String, String)> {
        let written = self.records().join(blockers::WORKSPACE);
        let text = fs::read_to_string(&written).expect("the workspace file is written");
        let document: serde_yaml::Value = serde_yaml::from_str(&text).expect("YAML");
        let mut members: Vec<(String, String)> = document["members"]
            .as_sequence()
            .expect("a members list")
            .iter()
            .map(|member| {
                (
                    member["name"].as_str().expect("a name").to_owned(),
                    member["source"].as_str().expect("a source").to_owned(),
                )
            })
            .collect();
        members.sort();
        members
    }

    /// The repository of each specification row, through the generated query, sorted; every row
    /// filed under `taken`.
    fn specified(&self, taken: &Taken) -> Vec<String> {
        let generated = Generated::new(store::open_existing(&self.state()).expect("the store"));
        let rows = generated
            .specifications()
            .expect("the generated query answers");
        generated.ports.check().expect("the store read every row");
        assert!(
            rows.iter().all(|row| row.snapshot_id == taken.snapshot_id),
            "every row is filed under the snapshot taken"
        );
        let mut specified: Vec<String> = rows.into_iter().map(|row| row.repository.0).collect();
        specified.sort();
        specified
    }

    /// The rows `snapshot blockers` prints, each filed under `taken`, without their snapshot and
    /// observation ids, sorted.
    fn blockers(&self, taken: &Taken) -> Vec<Value> {
        let output = self.conductor(
            None,
            &[
                "--state-dir",
                path(&self.state()),
                "snapshot",
                "blockers",
                "--format",
                "json",
            ],
        );
        assert!(
            output.status.success(),
            "snapshot blockers: {}",
            describe(&output)
        );
        let Value::Array(rows) = serde_json::from_slice(&output.stdout).expect("JSON rows") else {
            panic!(
                "`snapshot blockers` printed no array: {}",
                describe(&output)
            );
        };
        let mut rows: Vec<Value> = rows
            .into_iter()
            .map(|mut row| {
                let row = row.as_object_mut().expect("a row is an object");
                assert_eq!(
                    row.remove("snapshot_id"),
                    Some(json!(taken.snapshot_id.0.0)),
                    "the blocker is filed under the snapshot taken"
                );
                row.remove("observation_id");
                Value::Object(row.clone())
            })
            .collect();
        rows.sort_by_key(Value::to_string);
        rows
    }
}
