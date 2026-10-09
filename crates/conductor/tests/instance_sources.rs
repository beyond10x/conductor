//! `story:config-file`, the collectors' part: every collector reads where the repositories come
//! from, where their checkouts and managed trees are, and where the blockers' workspace and
//! exports are, from the instance, and no collector names an organization.
//!
//! The first cases run the built binary's `snapshot start-snapshot` with `CONDUCTOR_CONFIG` naming
//! a config file in the case's directory, `HOME` the case's `home/`, and a `PATH` holding only the
//! `gh`, `aep` and `claude` programs the case writes there: each appends its arguments to a log
//! and prints a fixed answer, with shell built-ins only. The other cases run one collector through
//! the library over the instance's own command lines, each `gh` line wrapped so that it logs its
//! words and answers `cat` of a fixture under `tests/fixtures/instance/`. No case reaches the
//! network, a real checkout or the real session list. Only the catalog cases
//! (`story:catalog-source`) run `git fetch`, in a catalog checkout the case builds, from a bare
//! origin in the case's own directory.

mod active;
mod common;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::collect::{Collector, Origins, blockers, github, repositories, specifications};
use conductor_cli::config::{self, Active};
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::config::Instance;
use conductor_model::observation::obligations::{
    ReleasesQuery, RepositoriesQuery, SpecificationsQuery,
};
use conductor_model::observation::{
    Repositories, SnapshotState, SpecificationPresence, ValidationResult, Visibility,
};
use serde_json::{Value, json};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The fixtures every `gh` line of the library cases answers from.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/instance");

/// The clock: the repositories' 7-day window opens at 2026-09-30T12:00:00Z, and what shipped is
/// read from 2026-09-23.
const NOW: &str = "2026-10-07T12:00:00Z";

/// Before the window.
const OLD: &str = "2026-09-01T09:00:00Z";

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// One case's directory: `home/` with the checkouts root `home/src/` and the managed trees
/// `home/trees/`, the instance's `records/`, `state/` and `cache/`, `bin/` for the programs the
/// binary finds, and an empty `work/` it runs from.
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
            .join("instance_sources")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in [
            "home/src",
            "home/trees",
            "records/.engineering",
            "work",
            "bin",
        ] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        fs::write(
            dir.join("records/.engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers: []\n",
        )
        .expect("write the records' workspace file");
        Self { dir }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn root(&self) -> PathBuf {
        self.home().join("src")
    }

    fn trees(&self) -> PathBuf {
        self.home().join("trees")
    }

    fn records(&self) -> PathBuf {
        self.dir.join("records")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn cache(&self) -> PathBuf {
        self.dir.join("cache")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn log(&self, program: &str) -> PathBuf {
        self.dir.join(format!("{program}.log"))
    }

    /// The config file of one instance, `acme`, whose `sources` are `sources` (a YAML flow
    /// sequence), its checkouts and managed trees under `home/`, and its records, state and cache
    /// in the case's directory.
    fn config(&self, sources: &str) -> PathBuf {
        self.config_with(sources, "")
    }

    /// [`Case::config`], with `extra` lines appended to the instance.
    fn config_with(&self, sources: &str, extra: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: {sources}\n    \
             checkouts:\n      root: {}\n      trees: {}\n    records: {}\n    state: {}\n    \
             cache: {}\n{extra}",
            quoted(&self.root()),
            quoted(&self.trees()),
            quoted(&self.records()),
            quoted(&self.state()),
            quoted(&self.cache()),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// The instance [`Case::config`] names, as the config file reads.
    fn instance(&self, sources: &str) -> Instance {
        self.instance_with(sources, "")
    }

    /// The instance [`Case::config_with`] names, as the config file reads.
    fn instance_with(&self, sources: &str, extra: &str) -> Instance {
        let text =
            fs::read_to_string(self.config_with(sources, extra)).expect("read the config file");
        let mut parsed = config::parse(&text, &self.home())
            .unwrap_or_else(|problems| panic!("the config file is valid: {problems:?}"));
        parsed.instances.remove(0)
    }

    /// The program `name` on the binary's `PATH`: it appends its arguments to its log, then runs
    /// `body`, shell built-ins only.
    fn program(&self, name: &str, body: &str) {
        let file = self.bin().join(name);
        fs::write(
            &file,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{body}\n",
                text(&self.log(name))
            ),
        )
        .expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// `gh` answering every list empty, `aep` an empty workspace and listing, and `claude` the
    /// Claude session list `sessions`.
    fn programs(&self, sessions: &Value) {
        self.program("gh", "printf '%s' '[]'");
        self.program(
            "aep",
            "case \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) printf '%s' \
             '{\"artifacts\": []}' ;;\nesac",
        );
        self.program("claude", &format!("printf '%s' '{sessions}'"));
    }

    /// The built binary run from `work/` with `--state-dir` naming `state/`, `CONDUCTOR_CONFIG`
    /// naming `config`, `HOME` naming `home/` and `PATH` only `bin/`.
    fn conductor(&self, config: &Path, args: &[&str]) -> Output {
        common::conductor(self.home())
            .current_dir(self.dir.join("work"))
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .arg("--state-dir")
            .arg(self.state())
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// `snapshot start-snapshot` through the binary, at [`NOW`].
    fn start_snapshot(&self, config: &Path) -> Output {
        self.conductor(
            config,
            &[
                "snapshot",
                "start-snapshot",
                "--started-at",
                NOW,
                "--disk-free-bytes",
                "100000000000",
            ],
        )
    }

    /// The rows of the view `view`, through the binary, without the identities the store assigns.
    fn rows(&self, config: &Path, view: &str) -> Vec<Value> {
        let output = self.conductor(config, &["snapshot", view, "--format", "jsonl"]);
        assert!(output.status.success(), "{view}: {}", describe(&output));
        String::from_utf8(output.stdout)
            .expect("UTF-8")
            .lines()
            .map(|line| {
                let mut row: Value = serde_json::from_str(line).expect("one JSON object a line");
                let fields = row.as_object_mut().expect("a row is an object");
                fields.remove("observation_id");
                fields.remove("snapshot_id");
                row
            })
            .collect()
    }

    /// The lines `program` logged, in the order it logged them.
    fn lines(&self, program: &str) -> Vec<String> {
        fs::read_to_string(self.log(program))
            .map(|log| log.lines().map(str::to_owned).collect())
            .unwrap_or_default()
    }

    /// One snapshot into `state/` through the one collector `name`, with the state directory.
    fn take(
        &self,
        name: &'static str,
        collect: impl Fn(&mut Recorder<'_>) -> anyhow::Result<()> + 'static,
    ) -> Taken {
        snapshot::take_in(&self.state(), &[Collector::new(name, collect)])
            .expect("the driver ends the snapshot")
    }

    fn generated(&self) -> Generated<store::Store> {
        Generated::new(store::open_existing(&self.state()).expect("open the store"))
    }

    /// The repository rows `taken` recorded, in the order they were recorded.
    fn repositories(&self, taken: &Taken) -> Vec<Repositories> {
        self.generated()
            .repositories()
            .expect("the repositories view answers")
            .into_iter()
            .filter(|row| row.snapshot_id == taken.snapshot_id)
            .collect()
    }
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn now() -> OffsetDateTime {
    OffsetDateTime::parse(NOW, &Rfc3339).expect("NOW is RFC 3339")
}

fn text(path: &Path) -> String {
    path.to_str().expect("the case's path is UTF-8").to_owned()
}

/// `path` as a YAML scalar: a JSON string.
fn quoted(path: &Path) -> String {
    Value::from(text(path)).to_string()
}

fn fixture(name: &str) -> String {
    format!("{FIXTURES}/{name}")
}

/// The script that logs its words after the answer's name to the log `$0`, then answers `cat` of
/// the fixture `$1`.
const LOGGED: &str = "answer=$1; shift; printf '%s\\n' \"$*\" >> \"$0\"; exec cat \"$answer\"";

/// `template`, a `gh` command line, as one that appends its words after `gh` to `log`, filled as
/// the collector fills them, and answers `cat` of the fixture `answer`, whose name the collector
/// fills too (`{organization}`, `{repository}`).
fn logged(template: &[String], log: &Path, answer: &str) -> Vec<String> {
    let (program, words) = template.split_first().expect("a command line");
    assert_eq!(program, "gh", "{template:?}");
    let mut command = vec![
        "sh".to_owned(),
        "-c".to_owned(),
        LOGGED.to_owned(),
        text(log),
        fixture(answer),
    ];
    command.extend(words.iter().cloned());
    command
}

/// [`logged`], for a command line of `OsString`s.
fn logged_os(template: &[OsString], log: &Path, answer: &str) -> Vec<OsString> {
    let template: Vec<String> = template
        .iter()
        .map(|word| word.to_str().expect("a UTF-8 word").to_owned())
        .collect();
    logged(&template, log, answer)
        .into_iter()
        .map(OsString::from)
        .collect()
}

/// `sh -c script`.
fn shell(script: &str) -> Vec<OsString> {
    ["sh", "-c", script].map(OsString::from).to_vec()
}

/// Runs git in `dir` at `date`, with no configuration of the user's, and answers what it printed.
fn git(dir: &Path, date: &str, args: &[&str]) -> String {
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
    String::from_utf8(output.stdout).expect("git prints UTF-8")
}

/// Writes `files` under `dir` and commits every change there as `subject` at `date`.
fn commit(dir: &Path, date: &str, subject: &str, files: &[(&str, &str)]) {
    for (file, content) in files {
        let path = dir.join(file);
        fs::create_dir_all(path.parent().expect("a file has a parent")).expect("create its dir");
        fs::write(path, content).expect("write the file");
    }
    git(dir, date, &["add", "--all"]);
    git(dir, date, &["commit", "--quiet", "-m", subject]);
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

fn complete(taken: &Taken) {
    assert_eq!(
        taken.state,
        SnapshotState::Complete,
        "the snapshot failed: {:?}",
        taken.reason
    );
}

fn names(rows: &[Repositories]) -> Vec<&str> {
    rows.iter().map(|row| row.repository.0.as_str()).collect()
}

fn session(
    session_ref: &str,
    name: Option<&str>,
    cwd: &str,
    repository: Option<&str>,
    activity: &str,
) -> Value {
    json!({
        "harness": "Claude",
        "session_ref": session_ref,
        "name": name,
        "cwd": cwd,
        "repository": repository,
        "role": null,
        "activity": activity,
    })
}

/// [`session`], of a session the instance's records directory places as conductor's own.
fn conductor_session(session_ref: &str, name: Option<&str>, cwd: &str, activity: &str) -> Value {
    let mut row = session(session_ref, name, cwd, None, activity);
    row["role"] = json!("conductor");
    row
}

// ---------------------------------------------------------------------------------------------
// Through the binary
// ---------------------------------------------------------------------------------------------

/// The acceptance: with `sources: [{github: acme}]`, every collector that asks `gh` asks it about
/// `acme`. The repositories, github and specifications collectors each list `acme`'s
/// repositories, in that order, and nothing else is asked; the snapshot completes.
#[test]
fn every_collector_asks_gh_about_the_instances_owner() {
    let case = Case::new("asks-acme");
    case.programs(&json!([]));
    let config = case.config("[{github: acme}]");
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.lines("gh"),
        [
            "repo list acme --limit 200 --json name,visibility,isArchived,hasIssuesEnabled",
            "repo list acme --limit 200 --json name,isArchived",
            "repo list acme --limit 200 --json name,isArchived",
        ]
    );
}

/// The acceptance: `checkouts.root` places sessions. A session in a checkout under the root or in
/// a managed tree under `trees` is bound to its repository, one at the root to none, and one in
/// the directory the built-in instance would place is outside this instance: it keeps its harness
/// and activity only.
#[test]
fn the_checkouts_root_and_trees_place_sessions() {
    let case = Case::new("places");
    case.programs(&json!([
        {"id": "s-root", "name": "alpha-controller", "pid": 101, "status": "busy",
         "cwd": text(&case.root().join("alpha/crates/x"))},
        {"id": "s-tree", "pid": 102, "status": "idle",
         "cwd": text(&case.trees().join("beta/fix-1"))},
        {"id": "s-top", "pid": 103, "status": "waiting", "cwd": text(&case.root())},
        {"id": "s-old", "name": "elsewhere", "pid": 104, "status": "busy",
         "cwd": text(&case.home().join("example-org/gamma"))},
    ]));
    let config = case.config("[{github: acme}]");
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.rows(&config, "sessions"),
        [
            session(
                "s-root",
                Some("alpha-controller"),
                "~/src/alpha/crates/x",
                Some("alpha"),
                "Busy"
            ),
            session("s-tree", None, "~/trees/beta/fix-1", Some("beta"), "Idle"),
            session("s-top", None, "~/src", None, "Waiting"),
            session("", None, "", None, "Busy"),
        ]
    );
}

/// `story:sessions-outside-root-explained`: a session whose working directory is the instance's
/// records directory, or a directory under it, is conductor's own. It keeps its reference and
/// name, its working directory is recorded, it is bound to no repository, and its role is
/// `conductor`.
#[test]
fn a_session_in_the_records_directory_is_conductors_own() {
    let case = Case::new("conductor-session");
    fs::create_dir_all(case.records().join("docs/handoff")).expect("create a records directory");
    case.programs(&json!([
        {"id": "s-conductor", "name": "conductor", "pid": 101, "status": "busy",
         "cwd": text(&case.records())},
        {"id": "s-handoff", "pid": 102, "status": "idle",
         "cwd": text(&case.records().join("docs/handoff"))},
        {"id": "s-root", "name": "alpha-controller", "pid": 103, "status": "waiting",
         "cwd": text(&case.root().join("alpha"))},
    ]));
    let config = case.config("[{github: acme}]");
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.rows(&config, "sessions"),
        [
            conductor_session(
                "s-conductor",
                Some("conductor"),
                &text(&case.records()),
                "Busy"
            ),
            conductor_session(
                "s-handoff",
                None,
                &text(&case.records().join("docs/handoff")),
                "Idle"
            ),
            session(
                "s-root",
                Some("alpha-controller"),
                "~/src/alpha",
                Some("alpha"),
                "Waiting"
            ),
        ]
    );
}

/// `story:sessions-outside-root-explained`: a session outside the checkouts root, their managed
/// trees and the records directory stays redacted, also beside the records directory: the state
/// directory, a sibling whose name begins with the records directory's, and the directory above
/// it. Nothing of it reaches the store.
#[test]
fn a_session_beside_the_records_directory_stays_redacted() {
    let case = Case::new("beside-records");
    let beside = [
        case.state().join("x"),
        case.dir.join("records-old"),
        case.dir.clone(),
    ];
    case.programs(&json!([
        {"id": "s-private-1", "name": "private-name-1", "pid": 101, "status": "busy",
         "cwd": text(&beside[0])},
        {"id": "s-private-2", "name": "private-name-2", "pid": 102, "status": "busy",
         "cwd": text(&beside[1])},
        {"id": "s-private-3", "name": "private-name-3", "pid": 103, "status": "busy",
         "cwd": text(&beside[2])},
    ]));
    let config = case.config("[{github: acme}]");
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.rows(&config, "sessions"),
        [
            session("", None, "", None, "Busy"),
            session("", None, "", None, "Busy"),
            session("", None, "", None, "Busy"),
        ]
    );
    let output = case.conductor(&config, &["snapshot", "sessions", "--format", "json"]);
    let shown = String::from_utf8_lossy(&output.stdout);
    for private in ["s-private", "private-name", "records-old"] {
        assert!(
            !shown.contains(private),
            "the view shows {private:?}: {shown}"
        );
    }
}

/// With a config file, the blockers and specifications collectors read the workspace under the
/// instance's `records`. Its members here are none, so neither exports nor lists anything: nothing
/// is written under the cache or the state directory (`aep` refuses a workspace of no members).
#[test]
fn the_workspace_is_the_records_one_and_a_workspace_of_no_members_exports_nothing() {
    let case = Case::new("records-cache");
    case.programs(&json!([]));
    let config = case.config("[{github: acme}]");
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let members = format!(
        "plan workspace members --format json --root {}",
        text(&case.records())
    );
    assert_eq!(
        case.lines("aep"),
        [members.as_str(), &members],
        "the blockers collector, then the specifications collector"
    );
    assert!(
        !case
            .cache()
            .join("exports/.engineering/workspace.yaml")
            .exists(),
        "no workspace file is written for no members"
    );
    assert!(
        !case.state().join("exports").exists(),
        "nothing is exported under the state directory"
    );
}

// ---------------------------------------------------------------------------------------------
// Through the library
// ---------------------------------------------------------------------------------------------

/// Each `github:` source is an owner whose repositories are listed and read under it; without an
/// Omega checkout under the checkouts root the catalog is not read, and the snapshot completes.
#[test]
fn each_owner_is_listed_and_its_repositories_are_read_under_it() {
    let case = Case::new("owners-repositories");
    let instance = case.instance("[{github: acme}, {github: other}]");
    let origins = Origins::of(&instance);
    assert_eq!(
        origins,
        Origins {
            owners: vec!["acme".to_owned(), "other".to_owned()],
            local: Vec::new(),
        }
    );
    let log = case.log("gh");
    let mut sources = repositories::Sources::of(&instance, now());
    assert_eq!(sources.workspace, case.root(), "the checkouts root");
    sources.repository_list = logged(
        &sources.repository_list,
        &log,
        "{organization}.repositories.json",
    );
    sources.open_issues = logged(&sources.open_issues, &log, "empty.json");
    sources.latest_release = logged(&sources.latest_release, &log, "empty.json");
    sources.main_commit = logged(&sources.main_commit, &log, "{repository}.main.json");
    sources.main_commits = logged(&sources.main_commits, &log, "empty.json");
    sources.commit = logged(&sources.commit, &log, "empty.json");
    let taken = case.take("repositories", move |record| {
        repositories::collect_over(&sources, &origins, record)
    });
    complete(&taken);

    let rows = case.repositories(&taken);
    assert_eq!(names(&rows), ["alpha", "beta"]);
    assert_eq!(rows[0].visibility, Visibility::Public);
    assert_eq!(
        rows[0].main_head.0,
        "a100000000000000000000000000000000000001"
    );
    assert_eq!(rows[1].visibility, Visibility::Private);
    assert_eq!(
        rows[1].main_head.0,
        "b100000000000000000000000000000000000001"
    );
    assert!(
        rows.iter()
            .all(|row| !row.local_checkout && row.in_catalog.is_none()),
        "{rows:#?}"
    );

    let lines = case.lines("gh");
    assert_eq!(
        lines[..2],
        [
            "repo list acme --limit 200 --json name,visibility,isArchived,hasIssuesEnabled",
            "repo list other --limit 200 --json name,visibility,isArchived,hasIssuesEnabled",
        ],
        "{lines:#?}"
    );
    let mut asked = lines[2..].to_vec();
    asked.sort();
    let mut expected = vec![
        "api repos/acme/alpha/commits/main".to_owned(),
        "api --paginate repos/acme/alpha/commits?sha=main&since=2026-09-30T12:00:00Z&per_page=100"
            .to_owned(),
        "release list -R acme/alpha --limit 1 --exclude-drafts --json tagName,publishedAt"
            .to_owned(),
        "issue list -R acme/alpha --state open --limit 1000 --json number,createdAt".to_owned(),
        "api repos/other/beta/commits/main".to_owned(),
        "api --paginate repos/other/beta/commits?sha=main&since=2026-09-30T12:00:00Z&per_page=100"
            .to_owned(),
        "release list -R other/beta --limit 1 --exclude-drafts --json tagName,publishedAt"
            .to_owned(),
    ];
    expected.sort();
    assert_eq!(asked, expected);
}

/// The github collector asks each owner about its own repositories, and a release's link names
/// its owner.
#[test]
fn the_github_collector_asks_each_owner_about_its_own_repositories() {
    let case = Case::new("owners-github");
    let instance = case.instance("[{github: acme}, {github: other}]");
    let origins = Origins::of(&instance);
    let log = case.log("gh");
    let mut sources = github::Sources::of(&instance);
    sources.repository_list = logged(
        &sources.repository_list,
        &log,
        "{organization}.repositories.json",
    );
    sources.pull_requests = logged(&sources.pull_requests, &log, "empty.json");
    sources.workflow_runs = logged(&sources.workflow_runs, &log, "empty.json");
    let mut shipments = github::Shipments::live(now());
    shipments.releases = logged(&shipments.releases, &log, "{repository}.releases.json");
    shipments.merged_pull_requests = logged(&shipments.merged_pull_requests, &log, "empty.json");
    let taken = case.take("github", move |record| {
        github::collect_over(&sources, &shipments, &origins, record)
    });
    complete(&taken);

    let releases: Vec<(String, String)> = case
        .generated()
        .releases()
        .expect("the releases view answers")
        .into_iter()
        .map(|row| (row.repository.0, row.url))
        .collect();
    assert_eq!(
        releases,
        [(
            "alpha".to_owned(),
            "https://github.com/acme/alpha/releases/tag/v1.0.0".to_owned()
        )]
    );

    let lines = case.lines("gh");
    assert_eq!(
        lines[..2],
        [
            "repo list acme --limit 200 --json name,isArchived",
            "repo list other --limit 200 --json name,isArchived",
        ],
        "{lines:#?}"
    );
    let mut asked = lines[2..].to_vec();
    asked.sort();
    let mut expected = Vec::new();
    for repository in ["acme/alpha", "other/beta"] {
        expected.extend([
            format!(
                "pr list -R {repository} --state open --limit 1000 --json \
                 number,title,isDraft,mergeable,statusCheckRollup,createdAt,updatedAt"
            ),
            format!(
                "run list -R {repository} --branch main --limit 20 --json \
                 workflowName,conclusion,status,createdAt"
            ),
            format!(
                "run list -R {repository} --branch main --limit 20 --json \
                 workflowName,conclusion,status,createdAt"
            ),
            format!(
                "release list -R {repository} --exclude-drafts --limit 20 --json \
                 tagName,publishedAt,isPrerelease"
            ),
            format!(
                "pr list -R {repository} --state merged --search merged:>=2026-09-23 --limit 200 \
                 --json number,title,mergedAt,url"
            ),
        ]);
    }
    expected.sort();
    assert_eq!(asked, expected);
}

/// The specifications collector lists each owner's repositories, over the workspace under the
/// instance's records and the exports under its cache.
#[test]
fn the_specifications_collector_lists_each_owner() {
    let case = Case::new("owners-specifications");
    let instance = case.instance("[{github: acme}, {github: other}]");
    let active = Active {
        instance,
        from_file: true,
        others: Vec::new(),
        default: None,
    };
    let mut sources =
        specifications::Sources::of(&active, &case.state()).expect("the instance's sources");
    assert_eq!(sources.owners, ["acme", "other"]);
    assert_eq!(sources.workspace.root, case.records());
    assert_eq!(sources.workspace.exports, case.cache().join("exports"));
    let log = case.log("gh");
    sources.repositories = logged_os(&sources.repositories, &log, "empty.json");
    sources.workspace.members = shell("printf '%s' '{\"members\": []}'");
    sources.workspace.list = shell("printf '%s' '{\"artifacts\": []}'");
    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });
    complete(&taken);
    assert_eq!(
        case.lines("gh"),
        [
            "repo list acme --limit 200 --json name,isArchived",
            "repo list other --limit 200 --json name,isArchived",
        ]
    );
}

/// The blockers' sources follow the instance only when a config file names it: its records'
/// workspace and the exports under its cache. Without a file they are today's: the workspace at
/// or above the working directory, and the exports under the state directory.
#[test]
fn the_blocker_sources_follow_the_instance_only_when_a_file_names_it() {
    let case = Case::new("blocker-sources");
    let instance = case.instance("[{github: acme}]");
    let state = case.state();
    let exports = case.cache().join("exports");
    let filed = blockers::Sources::of(
        &Active {
            instance: instance.clone(),
            from_file: true,
            others: Vec::new(),
            default: None,
        },
        &state,
    )
    .expect("the instance's sources");
    assert_eq!(filed.root, case.records());
    assert_eq!(filed.exports, exports);
    assert_eq!(filed.members.last(), Some(&case.records().into_os_string()));
    assert_eq!(filed.list.last(), Some(&exports.clone().into_os_string()));

    let built_in = blockers::Sources::of(
        &Active {
            instance,
            from_file: false,
            others: Vec::new(),
            default: None,
        },
        &state,
    )
    .expect("today's sources");
    assert_eq!(built_in.exports, state.join("exports"));
    assert!(
        built_in.root.join(blockers::WORKSPACE).is_file(),
        "{}",
        built_in.root.display()
    );
}

/// A name two owners list is one checkout under one root, and one repository a snapshot records:
/// each collector that lists owners fails naming the repository and both owners.
#[test]
fn a_repository_two_owners_list_fails_each_collector_naming_both() {
    let case = Case::new("clash");
    let instance = case.instance("[{github: acme}, {github: other}]");
    let log = case.log("gh");
    let clash = |taken: &Taken| {
        assert_eq!(taken.state, SnapshotState::Failed, "{:?}", taken.reason);
        let reason = taken.reason.clone().unwrap_or_default();
        assert!(
            ["alpha", "acme", "other"]
                .iter()
                .all(|word| reason.contains(word)),
            "{reason}"
        );
    };

    let origins = Origins::of(&instance);
    let mut sources = repositories::Sources::of(&instance, now());
    sources.repository_list = logged(&sources.repository_list, &log, "acme.repositories.json");
    sources.open_issues = logged(&sources.open_issues, &log, "empty.json");
    sources.latest_release = logged(&sources.latest_release, &log, "empty.json");
    sources.main_commit = logged(&sources.main_commit, &log, "{repository}.main.json");
    sources.main_commits = logged(&sources.main_commits, &log, "empty.json");
    sources.commit = logged(&sources.commit, &log, "empty.json");
    clash(&case.take("repositories", move |record| {
        repositories::collect_over(&sources, &origins, record)
    }));

    let origins = Origins::of(&instance);
    let mut sources = github::Sources::of(&instance);
    sources.repository_list = logged(&sources.repository_list, &log, "acme.repositories.json");
    sources.pull_requests = logged(&sources.pull_requests, &log, "empty.json");
    sources.workflow_runs = logged(&sources.workflow_runs, &log, "empty.json");
    let mut shipments = github::Shipments::live(now());
    shipments.releases = logged(&shipments.releases, &log, "empty.json");
    shipments.merged_pull_requests = logged(&shipments.merged_pull_requests, &log, "empty.json");
    clash(&case.take("github", move |record| {
        github::collect_over(&sources, &shipments, &origins, record)
    }));

    let active = Active {
        instance,
        from_file: true,
        others: Vec::new(),
        default: None,
    };
    let mut sources =
        specifications::Sources::of(&active, &case.state()).expect("the instance's sources");
    sources.repositories = logged_os(&sources.repositories, &log, "acme.repositories.json");
    sources.workspace.members = shell("printf '%s' '{\"members\": []}'");
    sources.workspace.list = shell("printf '%s' '{\"artifacts\": []}'");
    clash(&case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    }));
}

/// A `local:` source's checkouts are read as they are: each directory under it holding a git
/// repository is one repository, read at its `HEAD` with no fetch and nothing asked of GitHub.
/// Without a host it has no issues and no host's release, and is not public.
#[test]
fn a_local_sources_checkouts_are_read_as_they_are() {
    let case = Case::new("local");
    let local = case.dir.join("local");
    let delta = local.join("delta");
    fs::create_dir_all(&delta).expect("create the checkout");
    git(&delta, OLD, &["init", "--quiet", "--initial-branch=main"]);
    commit(
        &delta,
        OLD,
        "Start the repository",
        &[("README.md", "# delta\n"), ("src/lib.rs", "")],
    );
    let released = "2026-10-05T10:00:00Z";
    commit(
        &delta,
        released,
        "feat: add the reader",
        &[("src/reader.rs", "")],
    );
    git(&delta, released, &["tag", "v0.1.0"]);
    let head_at = "2026-10-06T10:00:00Z";
    commit(
        &delta,
        head_at,
        "Record the plan",
        &[(".engineering/project.yaml", "version: aep.project/6\n")],
    );
    fs::write(delta.join("scratch.txt"), "").expect("leave a file uncommitted");
    let head = git(&delta, head_at, &["rev-parse", "HEAD"])
        .trim()
        .to_owned();
    fs::create_dir_all(local.join("notes")).expect("a directory that is no checkout");
    let unborn = local.join("epsilon");
    fs::create_dir_all(&unborn).expect("create a checkout with no commit");
    git(&unborn, OLD, &["init", "--quiet", "--initial-branch=main"]);
    fs::write(local.join("README.md"), "").expect("a file beside the checkouts");

    let instance = case.instance(&format!("[{{local: {}}}]", quoted(&local)));
    let origins = Origins::of(&instance);
    assert_eq!(
        origins,
        Origins {
            owners: Vec::new(),
            local: vec![local.clone()],
        }
    );
    let log = case.log("gh");
    let mut sources = repositories::Sources::of(&instance, now());
    sources.repository_list = logged(&sources.repository_list, &log, "empty.json");
    sources.open_issues = logged(&sources.open_issues, &log, "empty.json");
    sources.latest_release = logged(&sources.latest_release, &log, "empty.json");
    sources.main_commit = logged(&sources.main_commit, &log, "empty.json");
    sources.main_commits = logged(&sources.main_commits, &log, "empty.json");
    sources.commit = logged(&sources.commit, &log, "empty.json");
    let taken = case.take("repositories", move |record| {
        repositories::collect_over(&sources, &origins, record)
    });
    complete(&taken);
    assert_eq!(
        case.lines("gh"),
        Vec::<String>::new(),
        "GitHub is asked nothing"
    );

    let rows = case.repositories(&taken);
    assert_eq!(
        names(&rows),
        ["delta"],
        "a checkout with no commit is no repository yet"
    );
    let row = &rows[0];
    assert_eq!(row.visibility, Visibility::Private);
    assert!(!row.archived && row.local_checkout);
    assert_eq!(
        row.in_catalog, None,
        "an instance that names no catalog records no catalog membership"
    );
    assert_eq!(row.main_head.0, head);
    assert_eq!(row.main_committed_at.0, head_at);
    assert_eq!(row.main_commits_7d, 2);
    assert_eq!(
        row.real_commits_7d, 1,
        "the plan commit touches planning only"
    );
    assert_eq!(row.behind_main, 0);
    assert_eq!(row.dirty_files, 1);
    assert_eq!(row.worktrees, 1);
    assert_eq!(row.open_issues, 0);
    assert_eq!(row.oldest_open_issue_at, None);
    assert_eq!(row.latest_release.as_deref(), Some("v0.1.0"));
    assert_eq!(
        row.latest_release_at.as_ref().map(|at| at.0.as_str()),
        Some(released)
    );
    assert_eq!(row.unreleased_commits, Some(1));
    assert_eq!(row.planning_store_version.as_deref(), Some("aep.project/6"));
}

// ---------------------------------------------------------------------------------------------
// story:specifications-every-source: the specifications collector reads every source
// ---------------------------------------------------------------------------------------------

/// The specifications collector lists the repositories the repositories collector lists: each
/// owner's through `gh`, asked only for their archived flag, and each repository under a `local:`
/// source but those its `exclude` names, read at its `HEAD` with no fetch. None of them is a
/// member of the instance's workspace. Each gets one row: `alpha`, whose `ess/` holds a
/// specification, `Present` and validated with the installed `ess`; `beta`, whose only
/// `ess/system.yaml` is uncommitted, `Missing`; the grouped `team/gamma`, whose `AGENTS.md` opts
/// out, `OptedOut`. No checkout is changed.
#[test]
fn the_specifications_collector_gives_each_repository_of_a_local_source_a_row() {
    let case = Case::new("local-specifications");
    let local = case.dir.join("local");
    let checkout = |name: &str, files: &[(&str, &str)]| {
        let dir = local.join(name);
        fs::create_dir_all(&dir).expect("create the checkout");
        git(&dir, OLD, &["init", "--quiet", "--initial-branch=main"]);
        commit(&dir, OLD, "Start the repository", files);
        dir
    };
    let alpha = checkout(
        "alpha",
        &[
            ("README.md", "# alpha\n"),
            (
                "ess/system.yaml",
                "format: ess/22\nsystem: alpha\nversion: v1\n\ndomains: []\n",
            ),
            (
                "ess/ess-inputs.yaml",
                "format: ess-inputs/2\nrequires: ess 0.55\nspecification:\n  - system.yaml\n\
                 scenarios: []\n",
            ),
        ],
    );
    let beta = checkout("beta", &[("README.md", "# beta\n")]);
    fs::create_dir_all(beta.join("ess")).expect("create beta's ess/");
    fs::write(beta.join("ess/system.yaml"), "format: ess/22\n").expect("leave a file uncommitted");
    let gamma = checkout(
        "team/gamma",
        &[(
            "AGENTS.md",
            "# gamma\n\nESS opt-out: it publishes documents only.\n",
        )],
    );
    checkout("skipped", &[("ess/system.yaml", "format: ess/22\n")]);
    let untouched = || {
        [&alpha, &beta, &gamma].map(|dir| {
            (
                git(dir, OLD, &["rev-parse", "HEAD"]),
                git(
                    dir,
                    OLD,
                    &["status", "--porcelain", "--untracked-files=all"],
                ),
            )
        })
    };
    let before = untouched();

    let instance = case.instance(&format!(
        "[{{github: acme}}, {{local: {}, exclude: [skipped]}}]",
        quoted(&local)
    ));
    let active = Active {
        instance,
        from_file: true,
        others: Vec::new(),
        default: None,
    };
    let mut sources =
        specifications::Sources::of(&active, &case.state()).expect("the instance's sources");
    let log = case.log("gh");
    sources.repositories = logged_os(&sources.repositories, &log, "empty.json");
    sources.workspace.members = shell("printf '%s' '{\"members\": []}'");
    sources.workspace.list = shell("printf '%s' '{\"artifacts\": []}'");
    sources.workspace.fetch = shell(&format!("pwd >> '{}'; exit 1", text(&case.log("fetch"))));
    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });
    complete(&taken);

    assert_eq!(
        case.lines("gh"),
        ["repo list acme --limit 200 --json name,isArchived"],
        "GitHub is asked for acme's list only"
    );
    assert_eq!(
        case.lines("fetch"),
        Vec::<String>::new(),
        "nothing is fetched"
    );
    let generated = case.generated();
    let mut rows: Vec<_> = generated
        .specifications()
        .expect("the specifications view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .collect();
    generated.ports.check().expect("the store read every row");
    rows.sort_by(|a, b| a.repository.0.cmp(&b.repository.0));
    let observed: Vec<_> = rows
        .iter()
        .map(|row| {
            (
                row.repository.0.as_str(),
                row.presence,
                row.path.as_deref(),
                row.format.as_deref(),
                row.required_ess.as_deref(),
                row.validation,
                row.validation_refusals,
                row.scenarios,
                row.synthesis_refusals,
                row.conformance_status.as_deref(),
            )
        })
        .collect();
    assert_eq!(
        observed,
        [
            (
                "alpha",
                SpecificationPresence::Present,
                Some("ess"),
                Some("ess/22"),
                Some("ess 0.55"),
                ValidationResult::Valid,
                0,
                Some(0),
                Some(0),
                None,
            ),
            (
                "beta",
                SpecificationPresence::Missing,
                None,
                None,
                None,
                ValidationResult::NotRun,
                0,
                None,
                None,
                None,
            ),
            (
                "team/gamma",
                SpecificationPresence::OptedOut,
                None,
                None,
                None,
                ValidationResult::NotRun,
                0,
                None,
                None,
                None,
            ),
        ]
    );
    assert_eq!(
        untouched(),
        before,
        "a checkout's HEAD or working tree changed"
    );
}

// ---------------------------------------------------------------------------------------------
// story:catalog-source: the instance's catalog, when it names one
// ---------------------------------------------------------------------------------------------

/// `text`'s bytes as lower-case hex.
fn hex(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02x}")).collect()
}

/// A `local:` source under the case's `local/` holding two repositories, `delta` and `zeta`, each
/// with one commit; answered as the source's YAML flow sequence.
fn two_local_repositories(case: &Case) -> String {
    let local = case.dir.join("local");
    for name in ["delta", "zeta"] {
        let dir = local.join(name);
        fs::create_dir_all(&dir).expect("create the checkout");
        git(&dir, OLD, &["init", "--quiet", "--initial-branch=main"]);
        commit(&dir, OLD, "Start the repository", &[("README.md", "# x\n")]);
    }
    format!("[{{local: {}}}]", quoted(&local))
}

/// The catalog repository `registry` under the checkouts root, with a bare origin in the case's
/// `origins/`: `pushed` is committed and pushed to `origin/main`; `unpushed` is committed after
/// it but not pushed, so it is in the checkout's working tree and on its `main`, and not on
/// `origin/main`.
fn registry(case: &Case, pushed: &[&str], unpushed: &[&str]) {
    let origin = case.dir.join("origins/registry.git");
    fs::create_dir_all(&origin).expect("create the origin");
    git(
        &origin,
        OLD,
        &["init", "--quiet", "--bare", "--initial-branch=main"],
    );
    let dir = case.root().join("registry");
    fs::create_dir_all(&dir).expect("create the checkout");
    git(&dir, OLD, &["init", "--quiet", "--initial-branch=main"]);
    git(&dir, OLD, &["remote", "add", "origin", &text(&origin)]);
    let files = |paths: &[&str]| -> Vec<(String, String)> {
        let mut files = vec![("README.md".to_owned(), "# registry\n".to_owned())];
        files.extend(
            paths
                .iter()
                .map(|path| ((*path).to_owned(), "{}\n".to_owned())),
        );
        files
    };
    let pushed = files(pushed);
    let pushed: Vec<(&str, &str)> = pushed
        .iter()
        .map(|(p, b)| (p.as_str(), b.as_str()))
        .collect();
    commit(&dir, OLD, "Catalog the repositories", &pushed);
    git(&dir, OLD, &["push", "--quiet", "origin", "main"]);
    if !unpushed.is_empty() {
        let later = files(unpushed);
        let later: Vec<(&str, &str)> = later
            .iter()
            .map(|(p, b)| (p.as_str(), b.as_str()))
            .collect();
        commit(&dir, OLD, "Catalog one more, not pushed", &later);
    }
}

/// `paths` as the text slices [`registry`] takes.
fn refs(paths: &[String]) -> Vec<&str> {
    paths.iter().map(String::as_str).collect()
}

/// One snapshot through the `repositories` collector over `instance`'s own sources, every `gh`
/// line answering an empty list.
fn take_repositories(case: &Case, instance: &Instance) -> Taken {
    let origins = Origins::of(instance);
    let log = case.log("gh");
    let mut sources = repositories::Sources::of(instance, now());
    sources.repository_list = logged(&sources.repository_list, &log, "empty.json");
    sources.open_issues = logged(&sources.open_issues, &log, "empty.json");
    sources.latest_release = logged(&sources.latest_release, &log, "empty.json");
    sources.main_commit = logged(&sources.main_commit, &log, "empty.json");
    sources.main_commits = logged(&sources.main_commits, &log, "empty.json");
    sources.commit = logged(&sources.commit, &log, "empty.json");
    case.take("repositories", move |record| {
        repositories::collect_over(&sources, &origins, record)
    })
}

/// `(repository, in_catalog)` for every row `taken` recorded.
fn membership(case: &Case, taken: &Taken) -> Vec<(String, Option<bool>)> {
    case.repositories(taken)
        .into_iter()
        .map(|row| (row.repository.0, row.in_catalog))
        .collect()
}

/// The acceptance: a catalog the instance names is read on its repository's `origin/main`, and
/// a repository its directory names one `.json` file for is in the catalog, every other one is
/// not. With `names: plain` the stem is the name; with `names: hex` it is the name's hex. Only
/// `.json` entries count, only `origin/main` is read (not a commit that was not pushed, nor the
/// working tree), and in hex a stem that spells no name names none.
#[test]
fn a_catalog_the_instance_names_marks_the_repositories_it_lists() {
    for (names, pushed, unpushed) in [
        (
            "plain",
            vec![
                "entries/delta.json".to_owned(),
                "entries/zeta.md".to_owned(),
            ],
            vec!["entries/zeta.json".to_owned()],
        ),
        (
            "hex",
            vec![
                format!("entries/{}.json", hex("delta")),
                "entries/zz.json".to_owned(),
                format!("entries/{}.md", hex("zeta")),
            ],
            vec![format!("entries/{}.json", hex("zeta"))],
        ),
    ] {
        let case = Case::new(&format!("catalog-{names}"));
        let local = two_local_repositories(&case);
        registry(&case, &refs(&pushed), &refs(&unpushed));
        let instance = case.instance_with(
            &local,
            &format!("    catalog: {{repository: registry, path: entries, names: {names}}}\n"),
        );
        let taken = take_repositories(&case, &instance);
        complete(&taken);
        assert_eq!(
            membership(&case, &taken),
            [
                ("delta".to_owned(), Some(true)),
                ("zeta".to_owned(), Some(false)),
            ],
            "names: {names}"
        );
    }
}

/// Without a `catalog` key, or with one whose repository has no checkout under the checkouts
/// root, no repository's catalog membership is known: it is absent, not false, and the
/// directory a catalog would be read from is not looked at.
#[test]
fn without_a_readable_catalog_catalog_membership_is_absent() {
    for (name, extra) in [
        ("no-catalog", String::new()),
        (
            "no-checkout",
            "    catalog: {repository: elsewhere, path: entries}\n".to_owned(),
        ),
    ] {
        let case = Case::new(&format!("catalog-{name}"));
        let local = two_local_repositories(&case);
        registry(&case, &["entries/delta.json"], &[]);
        let instance = case.instance_with(&local, &extra);
        let taken = take_repositories(&case, &instance);
        complete(&taken);
        assert_eq!(
            membership(&case, &taken),
            [("delta".to_owned(), None), ("zeta".to_owned(), None)],
            "{name}"
        );
    }
}

/// A catalog whose directory names no repository on `origin/main` fails the snapshot naming the
/// repository and the directory, rather than recording every repository as outside it.
#[test]
fn a_catalog_that_names_no_repository_fails_the_snapshot() {
    let case = Case::new("catalog-empty");
    let local = two_local_repositories(&case);
    registry(&case, &["elsewhere/delta.json"], &[]);
    let instance = case.instance_with(
        &local,
        "    catalog: {repository: registry, path: entries}\n",
    );
    let taken = take_repositories(&case, &instance);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.as_deref().unwrap_or_default();
    assert!(
        reason.contains("registry") && reason.contains("catalogs no repository under entries"),
        "{reason}"
    );
    assert!(case.repositories(&taken).is_empty());
}

// ---------------------------------------------------------------------------------------------
// No organization in the collectors
// ---------------------------------------------------------------------------------------------

/// No collector, and not the disk-waste measurement, names the organization or its short name:
/// both come from the instance, whose built-in values are in `src/config.rs` alone.
#[test]
fn no_collector_names_an_organization() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = vec![src.join("dashboard/waste.rs")];
    for entry in fs::read_dir(src.join("collect")).expect("list src/collect") {
        let path = entry.expect("an entry").path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    assert!(files.len() > 5, "{files:?}");
    let mut hits = Vec::new();
    for file in &files {
        let content = fs::read_to_string(file).expect("read the file");
        for (at, line) in content.lines().enumerate() {
            let lower = line.to_lowercase();
            if lower.contains("example-org") {
                hits.push(format!("{}:{}: {line}", file.display(), at + 1));
            }
        }
    }
    assert!(hits.is_empty(), "{}", hits.join("\n"));
}

/// `story:instance-session-names`: a session whose name carries the instance's `session_prefix` is
/// the instance's wherever its directory is, and `<prefix>-conductor` is its conductor; a session
/// whose name carries another instance's prefix stays redacted, even in this instance's
/// checkouts. A name that carries no instance's prefix is placed by its directory, as before.
#[test]
fn a_session_is_placed_by_its_instance_s_prefix() {
    let case = Case::new("session-prefix");
    let elsewhere = case.home().join("elsewhere");
    case.programs(&json!([
        {"id": "s-own-conductor", "name": "a-conductor", "pid": 101, "status": "busy",
         "cwd": text(&elsewhere)},
        {"id": "s-other-conductor", "name": "b-conductor", "pid": 102, "status": "busy",
         "cwd": text(&elsewhere)},
        {"id": "s-other-in-root", "name": "b-alpha", "pid": 103, "status": "idle",
         "cwd": text(&case.root().join("alpha"))},
        {"id": "s-own-in-root", "name": "a-alpha", "pid": 104, "status": "idle",
         "cwd": text(&case.root().join("alpha"))},
        {"id": "s-unprefixed", "name": "alpha-controller", "pid": 105, "status": "waiting",
         "cwd": text(&case.root().join("alpha"))},
    ]));
    let beta = case.dir.join("beta");
    let config = case.config_with(
        "[{github: acme}]",
        &format!(
            "    session_prefix: a\n  - name: beta\n    session_prefix: b\n    sources: \
             [{{github: beta}}]\n    checkouts: {{root: {}, trees: {}}}\ndefault: acme\n",
            quoted(&beta.join("root")),
            quoted(&beta.join("trees")),
        ),
    );
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    let mut own_conductor = session(
        "s-own-conductor",
        Some("a-conductor"),
        "~/elsewhere",
        None,
        "Busy",
    );
    own_conductor["role"] = json!("conductor");
    assert_eq!(
        case.rows(&config, "sessions"),
        [
            own_conductor,
            session("", None, "", None, "Busy"),
            session("", None, "", None, "Idle"),
            session(
                "s-own-in-root",
                Some("a-alpha"),
                "~/src/alpha",
                Some("alpha"),
                "Idle"
            ),
            session(
                "s-unprefixed",
                Some("alpha-controller"),
                "~/src/alpha",
                Some("alpha"),
                "Waiting"
            ),
        ]
    );
    let output = case.conductor(&config, &["snapshot", "sessions", "--format", "json"]);
    let shown = String::from_utf8_lossy(&output.stdout);
    for private in ["s-other", "b-conductor", "b-alpha"] {
        assert!(
            !shown.contains(private),
            "the view shows {private:?}: {shown}"
        );
    }
}

/// A session whose name carries the instance's prefix stays redacted under an `exclude` entry:
/// the exclusion keeps that repository out of the instance's records whatever a name says.
#[test]
fn an_own_prefix_does_not_place_a_session_under_an_excluded_repository() {
    let case = Case::new("session-prefix-excluded");
    case.programs(&json!([
        {"id": "s-excluded", "name": "a-y", "pid": 101, "status": "busy",
         "cwd": text(&case.root().join("x/y"))},
        {"id": "s-placed", "name": "a-z", "pid": 102, "status": "idle",
         "cwd": text(&case.home().join("elsewhere"))},
    ]));
    let config = case.config_with(
        "[{gitlab: acme-group, exclude: [x]}]",
        "    session_prefix: a\n",
    );
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.rows(&config, "sessions"),
        [
            session("", None, "", None, "Busy"),
            session("s-placed", Some("a-z"), "~/elsewhere", None, "Idle"),
        ]
    );
}

// ---------------------------------------------------------------------------------------------
// Adversary, wave 03 U1 (`story:instance-session-names`).
// ---------------------------------------------------------------------------------------------

/// The instance without a prefix names a controller after its repository, `<repository>`
/// (`spec/domains/config.yaml`, `Controller.session_name`). Its repository `b-tools`, checked out
/// in its own root, has the controller session `b-tools`: the instance's own session, in its own
/// checkout, placed by its directory as before this story. Beside an instance with
/// `session_prefix: b`, the name carries `b`, and the collector redacts the bare instance's own
/// controller as "another instance's".
#[test]
fn adv_w03_u1_the_bare_instance_s_own_controller_is_not_redacted_by_another_prefix() {
    let case = Case::new("adv-w03-u1-bare-own-controller");
    case.programs(&json!([
        {"id": "s-bare-own", "name": "b-tools", "pid": 101, "status": "idle",
         "cwd": text(&case.root().join("b-tools"))},
    ]));
    let beta = case.dir.join("beta");
    let config = case.config_with(
        "[{github: acme}]",
        &format!(
            "  - name: beta\n    session_prefix: b\n    sources: [{{github: beta}}]\n    \
             checkouts: {{root: {}, trees: {}}}\ndefault: acme\n",
            quoted(&beta.join("root")),
            quoted(&beta.join("trees")),
        ),
    );
    let output = case.start_snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.rows(&config, "sessions"),
        [session(
            "s-bare-own",
            Some("b-tools"),
            "~/src/b-tools",
            Some("b-tools"),
            "Idle"
        )]
    );
}
