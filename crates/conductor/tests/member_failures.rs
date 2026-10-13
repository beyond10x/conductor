//! `story:snapshot-member-failures`: a member a collector cannot read is a recorded member failure,
//! not a failed snapshot.
//!
//! Each case lays out `example-org/` in its own directory under this test target's temporary
//! directory, as `tests/collect_stores.rs` does: `hub/`, whose `.engineering/workspace.yaml` names
//! three members `alpha`, `beta` and `gamma` as `source: ../../<repo>`, and one git checkout per
//! member, whose `refs/remotes/origin/main` names an older commit and whose branch `upstream` holds
//! what its `origin` has. The fetch is a fake `git` the case writes: `git fetch` moves
//! `origin/main` to `upstream` as a fetch would, with no network, except where the case makes it
//! fail for one member. Snapshots are taken through the library ([`snapshot::take`]); the member
//! failures and blockers are read back through the built binary's `snapshot member-failures` and
//! `snapshot blockers`, the specifications through the generated query. `aep` is the installed one.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::collect::{Collector, blockers, specifications};
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::observation::SnapshotState;
use conductor_model::observation::obligations::SpecificationsQuery;
use serde_json::{Value, json};

/// The three members of every case.
const MEMBERS: [&str; 3] = ["alpha", "beta", "gamma"];

/// The first line the fake `git` prints when its fetch fails.
const UNREACHABLE: &str = "fatal: could not read from remote repository";

/// One case's directory.
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
    /// The case `name`: the hub naming the three members, and a checkout of each whose `origin/main`
    /// holds an open blocker `decision-blocker:old` and whose `upstream` holds
    /// `decision-blocker:new`, each titled with the member's name.
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("member_failures")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("example-org/hub/.engineering")).expect("create the hub");
        fs::create_dir_all(dir.join("work")).expect("create work/");
        let case = Self { dir };
        let mut text = String::from("version: aep.workspace/1\nmembers:\n");
        for member in MEMBERS {
            text.push_str(&format!("  - name: {member}\n    source: ../../{member}\n"));
            case.repository(member);
        }
        write(&case.hub().join(".engineering/workspace.yaml"), &text);
        case
    }

    fn hub(&self) -> PathBuf {
        self.dir.join("example-org/hub")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn checkout(&self, member: &str) -> PathBuf {
        self.dir.join("example-org").join(member)
    }

    /// The checkout of `member`: `origin/main` and `HEAD` at an older commit, `upstream` ahead.
    fn repository(&self, member: &str) {
        let dir = self.checkout(member);
        fs::create_dir_all(&dir).expect("create the checkout");
        git(&dir, &["init", "--quiet"]);
        put(&dir, &planning_store(member, "old"));
        git(&dir, &["add", "--all"]);
        git(&dir, &["commit", "--quiet", "-m", "old"]);
        let old = git(&dir, &["rev-parse", "HEAD"]);
        git(&dir, &["rm", "-r", "--quiet", "."]);
        put(&dir, &planning_store(member, "new"));
        git(&dir, &["add", "--all"]);
        git(&dir, &["commit", "--quiet", "-m", "upstream"]);
        git(&dir, &["branch", "upstream"]);
        git(
            &dir,
            &["update-ref", "refs/remotes/origin/main", old.as_str()],
        );
        git(&dir, &["checkout", "--quiet", "--detach", old.as_str()]);
    }

    /// A fake `git` under the case's `fake/`: `fetch` in the checkout of `failing` prints
    /// [`UNREACHABLE`] and a second line and exits 128, or, `quietly`, does nothing and exits 0;
    /// in any other checkout it moves `origin/main` to `upstream`. Answers the fetch command line.
    fn fake_fetch(&self, failing: &str, quietly: bool) -> Vec<OsString> {
        let bin = self.dir.join("fake");
        fs::create_dir_all(&bin).expect("create fake/");
        let fake = bin.join("git");
        let failure = if quietly {
            "exit 0".to_owned()
        } else {
            format!("echo '{UNREACHABLE}' >&2; echo 'a second line' >&2; exit 128")
        };
        fs::write(
            &fake,
            format!(
                "#!/bin/sh\ncase \"$(pwd)\" in\n  */{failing}) {failure} ;;\nesac\n\
                 exec git update-ref refs/remotes/origin/main refs/heads/upstream\n"
            ),
        )
        .expect("write the fake git");
        fs::set_permissions(&fake, fs::Permissions::from_mode(0o755)).expect("make it runnable");
        vec![
            fake.into_os_string(),
            OsString::from("fetch"),
            OsString::from("--quiet"),
            OsString::from("origin"),
        ]
    }

    /// Takes one snapshot into the case's store through the one collector `name`.
    fn take(
        &self,
        name: &'static str,
        collect: impl Fn(&mut Recorder<'_>) -> anyhow::Result<()> + 'static,
    ) -> Taken {
        let store = store::open(&self.state()).expect("open the store");
        snapshot::take(store, &[Collector::new(name, collect)])
            .expect("the driver ends the snapshot")
    }

    /// The rows of `snapshot <view> --format json`, sorted.
    fn rows(&self, view: &str) -> Vec<Value> {
        let output = common::conductor(self.dir.join("home"))
            .current_dir(self.dir.join("work"))
            .args(["--state-dir", path(&self.state())])
            .args(["snapshot", view, "--format", "json"])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert!(
            output.status.success(),
            "snapshot {view}: {}",
            describe(&output)
        );
        let Value::Array(mut rows) = serde_json::from_slice(&output.stdout).expect("JSON rows")
        else {
            panic!("`snapshot {view}` printed no array: {}", describe(&output));
        };
        rows.sort_by_key(Value::to_string);
        rows
    }

    /// The blockers recorded under `taken`, as (repository, reference, title), sorted.
    fn blockers(&self, taken: &Taken) -> Vec<(String, String, String)> {
        let mut rows: Vec<_> = self
            .rows("blockers")
            .into_iter()
            .filter(|row| row["snapshot_id"].as_str() == Some(taken.snapshot_id.0.0.as_str()))
            .map(|row| {
                let text = |field: &str| row[field].as_str().expect("a text field").to_owned();
                (text("repository"), text("reference"), text("title"))
            })
            .collect();
        rows.sort();
        rows
    }

    /// The member failures recorded under `taken`, as (member, collector, error line).
    fn failures(&self, taken: &Taken) -> Vec<(String, String, String)> {
        self.rows("member-failures")
            .into_iter()
            .filter(|row| row["snapshot_id"].as_str() == Some(taken.snapshot_id.0.0.as_str()))
            .map(|row| {
                let text = |field: &str| row[field].as_str().expect("a text field").to_owned();
                (text("member"), text("collector"), text("error_line"))
            })
            .collect()
    }
}

/// The planning store of `member` at one commit: its project file and one open blocker
/// `decision-blocker:<which>`, titled `<member> <which>`.
fn planning_store(member: &str, which: &str) -> Vec<(String, String)> {
    vec![
        (
            ".engineering/project.yaml".to_owned(),
            format!(
                "version: aep.project/5\nprotocol: adp/1\nprofile: development.standard\n\
                 planning_scope: \"{member}\"\nstore:\n  git: {{}}\n"
            ),
        ),
        (
            format!(".engineering/planning/decision-blocker/{which}.md"),
            format!(
                "---\nformat: aep.planning-md/3\nid: decision-blocker:{which}\n\
                 kind: decision-blocker\nstatus: open\ntitle: {member} {which}\nrevision: 1\n---\n\
                 # {member} {which}\n"
            ),
        ),
    ]
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

/// The blockers collector, one of three members whose fetch fails: the snapshot completes with
/// the other two members' blockers, `gamma`'s blockers as last fetched beside them, and one member
/// failure naming `gamma`, `blockers` and the fetch's first error line.
#[test]
fn a_member_whose_fetch_fails_is_a_member_failure_and_the_others_are_read() {
    let case = Case::new("blockers_fetch");
    let mut sources = blockers::Sources::new(case.hub(), &case.state());
    sources.fetch = case.fake_fetch("gamma", false);

    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let row = |member: &str, which: &str| {
        (
            member.to_owned(),
            format!("decision-blocker:{which}"),
            format!("{member} {which}"),
        )
    };
    assert_eq!(
        case.blockers(&taken),
        vec![row("alpha", "new"), row("beta", "new"), row("gamma", "old")],
        "alpha's and beta's blockers at origin/main after the fetch, gamma's as last fetched"
    );
    let failures = case.failures(&taken);
    assert_eq!(failures.len(), 1, "one member failure: {failures:?}");
    let (member, collector, line) = &failures[0];
    assert_eq!(member, "gamma");
    assert_eq!(collector, "blockers");
    assert!(
        line.ends_with(&format!("fetch --quiet origin` exited 128: {UNREACHABLE}")),
        "the fetch and its first error line, and no more: {line}"
    );
}

/// The specifications collector, one of three members whose `git ls-tree` fails (its checkout
/// has no `origin/main`, and its fetch brings none): the snapshot completes with the other two
/// members' specifications, and one member failure names `gamma`, `specifications` and the first
/// line of the error.
#[test]
fn a_member_whose_ls_tree_fails_is_a_member_failure_and_the_others_are_read() {
    let case = Case::new("specifications_ls_tree");
    git(
        &case.checkout("gamma"),
        &["update-ref", "-d", "refs/remotes/origin/main"],
    );
    let repositories = case.dir.join("repositories.json");
    write(
        &repositories,
        &json!(
            MEMBERS
                .iter()
                .map(|name| json!({"name": name, "isArchived": false}))
                .collect::<Vec<_>>()
        )
        .to_string(),
    );
    let mut sources = specifications::Sources::new(case.hub(), &case.state());
    sources.workspace.fetch = case.fake_fetch("gamma", true);
    sources.repositories = vec![OsString::from("cat"), repositories.into_os_string()];

    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let generated = Generated::new(store::open_existing(&case.state()).expect("the store"));
    let mut read: Vec<String> = generated
        .specifications()
        .expect("the generated query answers")
        .into_iter()
        .map(|row| row.repository.0)
        .collect();
    read.sort();
    assert_eq!(read, vec!["alpha", "beta"]);
    let failures = case.failures(&taken);
    assert_eq!(failures.len(), 1, "one member failure: {failures:?}");
    let (member, collector, line) = &failures[0];
    assert_eq!(member, "gamma");
    assert_eq!(collector, "specifications");
    assert!(
        line.starts_with("`git ls-tree -r -z --name-only origin/main` exited 128: fatal: "),
        "the command and its first error line: {line}"
    );
    assert!(!line.contains('\n'), "one line: {line:?}");
}

/// A snapshot whose members are all read records no member failure: the view prints no row of it.
#[test]
fn a_snapshot_with_no_failure_prints_no_member_failure() {
    let case = Case::new("no_failure");
    let mut sources = blockers::Sources::new(case.hub(), &case.state());
    sources.fetch = case.fake_fetch("no-such-member", false);

    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    assert_eq!(case.blockers(&taken).len(), 3);
    assert_eq!(case.rows("member-failures"), Vec::<Value>::new());
}
