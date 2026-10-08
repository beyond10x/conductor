//! `story:collect-blockers` and `story:collect-specifications`: the `blockers` and
//! `specifications` collectors, over repositories each case builds.
//!
//! A case lays out `example-org/` in its own directory under this test target's temporary
//! directory: `hub/`, whose `.engineering/workspace.yaml` names each repository as
//! `source: ../../<repo>`, as conductor's own file does, and one git checkout per repository. Each
//! checkout is left where a collector must not read it: its `HEAD` and working tree hold an older
//! commit with an uncommitted blocker beside it, and its `refs/remotes/origin/main` names that same
//! older commit. The branch `upstream` holds what the checkout's `origin` has. The fetch is
//! replaced by `git update-ref refs/remotes/origin/main refs/heads/upstream`, which moves the ref
//! as a fetch would, with no network: only a collector that fetches and then reads `origin/main`
//! records what `upstream` holds. No case runs `git fetch`.
//!
//! `aep` and `ess` are the installed ones, run offline on these fixtures: a `cat` of a canned
//! listing could not tell which tree was exported, which is what these cases decide. `gh repo list`
//! is `cat` on `tests/fixtures/specifications/repositories.json`, or on a list the case writes.
//! Snapshots are taken through the library ([`snapshot::take`]). Blockers are read back through
//! the built binary's `snapshot blockers`, specifications through the generated query, since the
//! view `snapshot specifications` is not written yet.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use conductor_cli::collect::{Collector, blockers, specifications};
use conductor_cli::snapshot::{self, Recorder, Taken};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::observation::obligations::SpecificationsQuery;
use conductor_model::observation::{
    SnapshotState, SpecificationPresence, Specifications, ValidationResult,
};
use serde_json::{Value, json};

/// The committed fixtures of the specifications cases.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/specifications");

/// The ways an `AGENTS.md` records an ESS opt-out, each as a repository would write it: the
/// collector records each of these `OptedOut`.
const OPTED_OUT: [&str; 3] = [
    "ESS opt-out: this repository holds no system to specify.",
    "This repository opts out of\nESS: it publishes documents only.",
    "**Opted out of ESS** (2026-10-06).",
];

/// Wording near an opt-out that records none: the collector records each of these `Missing`.
const NOT_OPTED_OUT: [&str; 2] = [
    "Its essential checks are the link checker and the build.",
    "The ESS opt-outs of other repositories are kept in conductor.",
];

/// One case's directory: `example-org/hub/` with the workspace file, a checkout per repository
/// beside it, `state/` for the store and the exports, and an empty `work/` the binary runs from.
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
            .join("collect_stores")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        fs::create_dir_all(dir.join("example-org/hub/.engineering")).expect("create the hub");
        fs::create_dir_all(dir.join("work")).expect("create work/");
        Self { dir }
    }

    fn hub(&self) -> PathBuf {
        self.dir.join("example-org/hub")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn checkout(&self, repo: &str) -> PathBuf {
        self.dir.join("example-org").join(repo)
    }

    /// The hub's workspace file: one member per repository, named as conductor's file names it.
    fn workspace(&self, repos: &[&str]) {
        let mut text = String::from("version: aep.workspace/1\nmembers:\n");
        for repo in repos {
            text.push_str(&format!(
                "  - name: {}\n    source: ../../{repo}\n",
                member(repo)
            ));
        }
        write(&self.hub().join(".engineering/workspace.yaml"), &text);
    }

    /// The checkout of `repo`: `old` is what its `HEAD` and working tree show and what its
    /// `origin/main` names before the fetch, with an uncommitted open blocker beside it; `upstream`
    /// is what its origin has, on the branch `upstream`.
    fn repository(&self, repo: &str, old: &[(String, String)], upstream: &[(String, String)]) {
        let dir = self.checkout(repo);
        fs::create_dir_all(&dir).expect("create the checkout");
        git(&dir, &["init", "--quiet"]);
        put(&dir, old);
        git(&dir, &["add", "--all"]);
        git(&dir, &["commit", "--quiet", "--allow-empty", "-m", "old"]);
        let old_commit = git(&dir, &["rev-parse", "HEAD"]);
        git(&dir, &["rm", "-r", "--quiet", "--ignore-unmatch", "."]);
        put(&dir, upstream);
        git(&dir, &["add", "--all"]);
        git(
            &dir,
            &["commit", "--quiet", "--allow-empty", "-m", "upstream"],
        );
        git(&dir, &["branch", "upstream"]);
        git(
            &dir,
            &[
                "update-ref",
                "refs/remotes/origin/main",
                old_commit.as_str(),
            ],
        );
        git(
            &dir,
            &["checkout", "--quiet", "--detach", old_commit.as_str()],
        );
        put(
            &dir,
            &[artifact(
                "decision-blocker:uncommitted",
                "open",
                Some("Only in the working tree"),
            )],
        );
    }

    /// What a collector must leave as it found it: each checkout's `HEAD` and status.
    fn untouched(&self, repos: &[&str]) -> Vec<(String, String)> {
        repos
            .iter()
            .map(|repo| {
                let dir = self.checkout(repo);
                (
                    git(&dir, &["rev-parse", "HEAD"]),
                    git(&dir, &["status", "--porcelain", "--untracked-files=all"]),
                )
            })
            .collect()
    }

    /// The blockers collector's sources over this case, with the fetch replaced.
    fn blocker_sources(&self) -> blockers::Sources {
        let mut sources = blockers::Sources::new(self.hub(), &self.state());
        sources.fetch = fetch();
        sources
    }

    /// The specifications collector's sources over this case, with the fetch replaced and
    /// `gh repo list` answered by `cat` on `repositories`.
    fn specification_sources(&self, repositories: &Path) -> specifications::Sources {
        let mut sources = specifications::Sources::new(self.hub(), &self.state());
        sources.workspace.fetch = fetch();
        sources.repositories = words(&["cat", path(repositories)]);
        sources
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

    /// The recorded blockers, through `snapshot blockers --format json`, without the identities
    /// the store assigns, sorted.
    fn blockers(&self) -> Vec<Value> {
        let output = Command::new(env!("CARGO_BIN_EXE_conductor"))
            .current_dir(self.dir.join("work"))
            .args(["--state-dir", path(&self.state())])
            .args(["snapshot", "blockers", "--format", "json"])
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs");
        assert!(
            output.status.success(),
            "snapshot blockers: {}",
            describe(&output)
        );
        assert_eq!(
            fs::read_dir(self.dir.join("work"))
                .expect("read work/")
                .count(),
            0,
            "`snapshot blockers` planted something in its working directory"
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
                row.remove("observation_id");
                row.remove("snapshot_id");
                Value::Object(row.clone())
            })
            .collect();
        rows.sort_by_key(Value::to_string);
        rows
    }

    /// The recorded specifications, through the generated query, sorted by repository.
    fn specifications(&self) -> Vec<Specifications> {
        let generated = Generated::new(store::open_existing(&self.state()).expect("the store"));
        let mut rows = generated
            .specifications()
            .expect("the generated query answers");
        generated.ports.check().expect("the store read every row");
        rows.sort_by(|a, b| a.repository.0.cmp(&b.repository.0));
        rows
    }
}

/// The member name conductor's workspace file gives `repo`: lower case, a dot as a hyphen.
fn member(repo: &str) -> String {
    repo.to_lowercase().replace('.', "-")
}

/// A fetch that moves `origin/main` to `upstream`, as a fetch from that origin would.
fn fetch() -> Vec<OsString> {
    words(&[
        "git",
        "update-ref",
        "refs/remotes/origin/main",
        "refs/heads/upstream",
    ])
}

fn words(words: &[&str]) -> Vec<OsString> {
    words.iter().map(OsString::from).collect()
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
            "core.excludesFile=/dev/null",
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

/// One artifact of a planning store, at its path.
fn artifact(id: &str, status: &str, title: Option<&str>) -> (String, String) {
    let (kind, slug) = id.split_once(':').expect("an id is kind:slug");
    let title_line = title
        .map(|title| format!("title: {title}\n"))
        .unwrap_or_default();
    (
        format!(".engineering/planning/{kind}/{slug}.md"),
        format!(
            "---\nformat: aep.planning-md/3\nid: {id}\nkind: {kind}\nstatus: {status}\n\
             {title_line}revision: 1\n---\n# {}\n",
            title.unwrap_or(slug)
        ),
    )
}

/// Every file under `dir`, as (path relative to `dir`, text).
fn tree(dir: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut pending = vec![dir.to_owned()];
    while let Some(next) = pending.pop() {
        for entry in fs::read_dir(&next).expect("read a fixture directory") {
            let entry = entry.expect("a fixture entry");
            if entry.file_type().expect("its type").is_dir() {
                pending.push(entry.path());
            } else {
                let relative = entry
                    .path()
                    .strip_prefix(dir)
                    .expect("under the fixture")
                    .to_str()
                    .expect("UTF-8")
                    .to_owned();
                let text = fs::read_to_string(entry.path()).expect("a fixture is text");
                files.push((relative, text));
            }
        }
    }
    files
}

/// The fixture repository `repo` under `tests/fixtures/specifications/`.
fn fixture(repo: &str) -> Vec<(String, String)> {
    tree(&Path::new(FIXTURES).join(repo))
}

fn file(path: &str, text: &str) -> (String, String) {
    (path.to_owned(), text.to_owned())
}

/// The value of the top-level `key:` line of `file`.
fn top_level(file: &Path, key: &str) -> String {
    fs::read_to_string(file)
        .expect("read the file")
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}:")))
        .map(|value| value.trim().to_owned())
        .unwrap_or_else(|| panic!("{} has no top-level {key}:", file.display()))
}

/// The scenarios and refusals the last line of `ess verify conform synthesize` names:
/// `N scenario(s) (A authored), M refusal(s), written to …`.
fn synthesis_counts(stdout: &str) -> (i64, i64) {
    let last = stdout
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .expect("synthesis printed a line");
    let words: Vec<&str> = last.split_whitespace().collect();
    let before = |unit: &str| -> i64 {
        let at = words
            .iter()
            .position(|word| word.starts_with(unit))
            .unwrap_or_else(|| panic!("{last:?} names no {unit}"));
        words[at - 1].parse().expect("a count")
    };
    (before("scenario(s)"), before("refusal(s)"))
}

// ---------------------------------------------------------------------------------------------
// story:collect-blockers
// ---------------------------------------------------------------------------------------------

#[test]
fn blockers_are_the_open_blocker_artifacts_each_store_holds_at_origin_main_after_a_fetch() {
    let case = Case::new("blockers_at_origin_main");
    case.workspace(&["alpha", "beta.io"]);
    case.repository(
        "alpha",
        &[
            project("alpha"),
            artifact("decision-blocker:cleared-upstream", "open", Some("Cleared")),
            artifact("operator-blocker:only-in-checkout", "open", Some("Stale")),
        ],
        &[
            project("alpha"),
            artifact(
                "decision-blocker:cleared-upstream",
                "cleared",
                Some("Cleared"),
            ),
            artifact(
                "decision-blocker:opened-upstream",
                "open",
                Some("Pick a store"),
            ),
            artifact("decision-blocker:untitled", "open", None),
            artifact("operator-blocker:hands", "open", Some("Needs the operator")),
            artifact("blocker:plain", "open", Some("Plain")),
            artifact("story:do-it", "draft", Some("Do it")),
            artifact("review-result:open-review", "open", Some("Not a blocker")),
        ],
    );
    case.repository(
        "beta.io",
        &[],
        &[
            project("beta"),
            artifact("operator-blocker:restart", "open", Some("Restart")),
            artifact(
                "executable-system-specification:beta",
                "validated",
                Some("Spec"),
            ),
        ],
    );
    let before = case.untouched(&["alpha", "beta.io"]);

    let sources = case.blocker_sources();
    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let mut expected = vec![
        json!({"repository": "alpha", "reference": "blocker:plain",
               "blocker_kind": "blocker", "title": "Plain"}),
        json!({"repository": "alpha", "reference": "decision-blocker:opened-upstream",
               "blocker_kind": "decision-blocker", "title": "Pick a store"}),
        json!({"repository": "alpha", "reference": "decision-blocker:untitled",
               "blocker_kind": "decision-blocker", "title": ""}),
        json!({"repository": "alpha", "reference": "operator-blocker:hands",
               "blocker_kind": "operator-blocker", "title": "Needs the operator"}),
        json!({"repository": "beta.io", "reference": "operator-blocker:restart",
               "blocker_kind": "operator-blocker", "title": "Restart"}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(case.blockers(), expected);
    assert_eq!(
        case.untouched(&["alpha", "beta.io"]),
        before,
        "a checkout's HEAD or working tree changed"
    );
    for repo in ["alpha", "beta.io"] {
        assert!(
            case.state()
                .join("exports")
                .join(repo)
                .join(".engineering/project.yaml")
                .is_file(),
            "{repo} is exported under state/exports/{repo}/"
        );
    }
}

#[test]
fn a_fetch_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command() {
    let case = Case::new("blockers_fetch_fails");
    case.workspace(&["alpha", "beta.io"]);
    for repo in ["alpha", "beta.io"] {
        case.repository(
            repo,
            &[],
            &[
                project(&member(repo)),
                artifact("decision-blocker:x", "open", Some("X")),
            ],
        );
    }
    let log = case.dir.join("fetches.log");
    let mut sources = case.blocker_sources();
    sources.fetch = words(&["sh", "-c", "pwd >> \"$0\"; exit 1", path(&log)]);

    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Failed);
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(reason.starts_with("blockers: "), "{reason}");
    assert!(reason.contains("alpha"), "names the repository: {reason}");
    assert!(
        reason.contains(&format!("pwd >> \"$0\"; exit 1 {}", path(&log))),
        "names the command: {reason}"
    );
    let fetches = fs::read_to_string(&log).expect("the fetch ran");
    let alpha = fs::canonicalize(case.checkout("alpha")).expect("alpha's checkout");
    assert_eq!(
        fetches.lines().collect::<Vec<_>>(),
        vec![path(&alpha); 2],
        "the fetch ran in alpha's checkout, once and once more, and no further"
    );
    assert_eq!(case.blockers(), Vec::<Value>::new());
}

#[test]
fn a_workspace_listing_that_fails_twice_fails_the_snapshot_naming_the_command() {
    let case = Case::new("blockers_listing_fails");
    case.workspace(&["alpha"]);
    case.repository(
        "alpha",
        &[],
        &[
            project("alpha"),
            artifact("decision-blocker:x", "open", Some("X")),
        ],
    );
    let log = case.dir.join("listings.log");
    let mut sources = case.blocker_sources();
    sources.list = words(&["sh", "-c", "echo listed >> \"$0\"; exit 3", path(&log)]);

    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Failed);
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(
        reason.contains(&format!("echo listed >> \"$0\"; exit 3 {}", path(&log))),
        "names the command: {reason}"
    );
    assert_eq!(
        fs::read_to_string(&log)
            .expect("the listing ran")
            .lines()
            .count(),
        2,
        "the listing ran once and once more"
    );
}

/// A workspace of no members (no repository of the instance has a planning store) holds no
/// blocker: nothing is exported or listed, and the snapshot completes.
#[test]
fn a_workspace_of_no_members_records_no_blocker_and_lists_nothing() {
    let case = Case::new("blockers_no_members");
    let log = case.dir.join("listings.log");
    let mut sources = case.blocker_sources();
    sources.members = words(&["sh", "-c", "printf '%s' '{\"members\": []}'"]);
    sources.list = words(&["sh", "-c", "echo listed >> \"$0\"; exit 3", path(&log)]);

    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_ne!(taken.state, SnapshotState::Failed, "{:?}", taken.reason);
    assert!(!log.exists(), "the workspace was listed");
}

/// An export holds only what the collectors read at `origin/main`: the planning store
/// `.engineering/`, `AGENTS.md`, every directory holding an `ess-inputs.yaml` or `system.yaml`
/// outside `target/` and `node_modules/`, and each path an input manifest lists. A specification
/// at the repository's top makes the whole tree its root, and a repository with none of these
/// exports to an empty directory.
#[test]
fn an_export_holds_only_the_store_agents_md_and_each_specification_root() {
    let case = Case::new("narrow_exports");
    case.workspace(&["alpha", "top", "bare"]);
    case.repository(
        "alpha",
        &[file("src/old.rs", "// only in the checkout\n")],
        &[
            project("alpha"),
            artifact("decision-blocker:pick", "open", Some("Pick a store")),
            file("AGENTS.md", "# alpha\n"),
            file("README.md", "# alpha\n"),
            file("src/main.rs", "fn main() {}\n"),
            file("spec/system.yaml", "format: ess/22\n"),
            file(
                "spec/ess-inputs.yaml",
                "format: ess-inputs/2\nspecification:\n  - system.yaml\n  - \
                 ../contracts/shared.yaml\n",
            ),
            file("spec/domains/a.yaml", "domain: a\n"),
            file("tools/model/system.yaml", "format: ess/21\n"),
            file("contracts/shared.yaml", "shared: 1\n"),
            file("contracts/other.yaml", "other: 2\n"),
            file("target/spec/system.yaml", "format: ess/22\n"),
            file("node_modules/pkg/ess-inputs.yaml", "format: ess-inputs/2\n"),
            file("docs/target/system.yaml", "format: ess/22\n"),
        ],
    );
    case.repository(
        "top",
        &[],
        &[
            file("system.yaml", "format: ess/22\n"),
            file("domains/a.yaml", "domain: a\n"),
        ],
    );
    case.repository(
        "bare",
        &[],
        &[
            file("README.md", "# bare\n"),
            file("src/lib.rs", "// code\n"),
        ],
    );

    let sources = case.blocker_sources();
    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let exports = case.state().join("exports");
    assert_eq!(
        files_under(&exports.join("alpha")),
        [
            ".engineering/planning/decision-blocker/pick.md",
            ".engineering/project.yaml",
            "AGENTS.md",
            "contracts/shared.yaml",
            "spec/domains/a.yaml",
            "spec/ess-inputs.yaml",
            "spec/system.yaml",
            "tools/model/system.yaml",
        ]
    );
    assert_eq!(
        files_under(&exports.join("top")),
        ["domains/a.yaml", "system.yaml"]
    );
    assert!(
        exports.join("bare").is_dir() && files_under(&exports.join("bare")).is_empty(),
        "bare exports to an empty directory: {:?}",
        files_under(&exports.join("bare"))
    );
    assert_eq!(
        case.blockers(),
        vec![
            json!({"repository": "alpha", "reference": "decision-blocker:pick",
                    "blocker_kind": "decision-blocker", "title": "Pick a store"})
        ]
    );
}

/// The blockers collector marks its exports, once they are complete, with the id of the snapshot
/// that made them: `exports/.snapshot`.
#[test]
fn the_blockers_collector_marks_its_exports_with_its_snapshot() {
    let case = Case::new("exports_marked");
    case.workspace(&["alpha"]);
    case.repository(
        "alpha",
        &[],
        &[
            project("alpha"),
            artifact("decision-blocker:x", "open", Some("X")),
        ],
    );
    let sources = case.blocker_sources();
    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    assert_eq!(
        fs::read_to_string(case.state().join("exports/.snapshot")).expect("the exports' mark"),
        taken.snapshot_id.0.0
    );
}

/// One fetch and one export per repository per snapshot: the specifications collector that runs
/// after the blockers collector in the same snapshot reads their exports, fetching and exporting
/// nothing itself. A file a collector between them leaves in an export is still there.
#[test]
fn the_specifications_collector_reuses_the_exports_of_its_own_snapshot() {
    let case = Case::new("exports_reused");
    case.workspace(&["alpha", "beta.io"]);
    case.repository(
        "alpha",
        &[],
        &[fixture("alpha"), vec![project("alpha")]].concat(),
    );
    case.repository("beta.io", &[], &fixture("beta.io"));
    let repositories = case.dir.join("repositories.json");
    write(
        &repositories,
        r#"[{"name": "alpha", "isArchived": false}, {"name": "beta.io", "isArchived": false}]"#,
    );
    let log = case.dir.join("fetches.log");
    let mut blocker_sources = case.blocker_sources();
    blocker_sources.fetch = counted_fetch(&log);
    let mut specification_sources = case.specification_sources(&repositories);
    specification_sources.workspace.fetch = counted_fetch(&log);
    let sentinel = case.state().join("exports/alpha/left-between");

    let store = store::open(&case.state()).expect("open the store");
    let left = sentinel.clone();
    let taken = snapshot::take(
        store,
        &[
            Collector::new("blockers", move |record| {
                blockers::collect_from(&blocker_sources, record)
            }),
            Collector::new("between", move |_| {
                fs::write(&left, "left by the collector in between\n")?;
                Ok(())
            }),
            Collector::new("specifications", move |record| {
                specifications::collect_from(&specification_sources, record)
            }),
        ],
    )
    .expect("the driver ends the snapshot");

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let fetches = fs::read_to_string(&log).expect("the fetch ran");
    assert_eq!(
        fetches.lines().count(),
        2,
        "one fetch per repository in the snapshot: {fetches:?}"
    );
    assert!(
        sentinel.is_file(),
        "the specifications collector exported alpha again"
    );
    let rows = case.specifications();
    assert_eq!(
        rows.iter()
            .map(|row| (row.repository.0.as_str(), row.presence, row.validation))
            .collect::<Vec<_>>(),
        [
            (
                "alpha",
                SpecificationPresence::Present,
                ValidationResult::Valid
            ),
            (
                "beta.io",
                SpecificationPresence::Missing,
                ValidationResult::NotRun
            ),
        ]
    );
}

/// Exports another snapshot made are not this snapshot's: the specifications collector fetches
/// and exports itself, and marks the exports with its own snapshot.
#[test]
fn the_specifications_collector_exports_itself_over_another_snapshots_exports() {
    let case = Case::new("exports_stale");
    case.workspace(&["alpha"]);
    case.repository(
        "alpha",
        &[],
        &[fixture("alpha"), vec![project("alpha")]].concat(),
    );
    let repositories = case.dir.join("repositories.json");
    write(&repositories, r#"[{"name": "alpha", "isArchived": false}]"#);
    let log = case.dir.join("fetches.log");
    let mut blocker_sources = case.blocker_sources();
    blocker_sources.fetch = counted_fetch(&log);
    let mut specification_sources = case.specification_sources(&repositories);
    specification_sources.workspace.fetch = counted_fetch(&log);

    let first = case.take("blockers", move |record| {
        blockers::collect_from(&blocker_sources, record)
    });
    assert_eq!(first.state, SnapshotState::Complete, "{:?}", first.reason);
    let mark = case.state().join("exports/.snapshot");
    assert_eq!(
        fs::read_to_string(&mark).expect("the exports' mark"),
        first.snapshot_id.0.0
    );

    let second = case.take("specifications", move |record| {
        specifications::collect_from(&specification_sources, record)
    });
    assert_eq!(second.state, SnapshotState::Complete, "{:?}", second.reason);
    assert_eq!(
        fs::read_to_string(&log)
            .expect("the fetch ran")
            .lines()
            .count(),
        2,
        "the second snapshot fetched alpha again"
    );
    assert_eq!(
        fs::read_to_string(&mark).expect("the exports' mark"),
        second.snapshot_id.0.0
    );
    let rows = case.specifications();
    assert_eq!(rows.len(), 1, "{rows:?}");
    assert_eq!(rows[0].snapshot_id, second.snapshot_id);
    assert_eq!(rows[0].validation, ValidationResult::Valid);
}

/// A fetch that logs the directory it ran in to `log`, then moves `origin/main` as [`fetch`] does.
fn counted_fetch(log: &Path) -> Vec<OsString> {
    words(&[
        "sh",
        "-c",
        "pwd >> \"$0\" && exec git update-ref refs/remotes/origin/main refs/heads/upstream",
        path(log),
    ])
}

/// Every file under `dir`, relative to it, `/`-separated and sorted.
fn files_under(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = tree(dir).into_iter().map(|(file, _)| file).collect();
    files.sort();
    files
}

#[test]
fn the_organization_workspace_names_each_repository_as_a_sibling_checkout() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.engineering/workspace.yaml");
    let text = fs::read_to_string(&file).expect("conductor has .engineering/workspace.yaml");
    let mut lines = text.lines();
    assert_eq!(lines.next(), Some("version: aep.workspace/1"));
    assert_eq!(lines.next(), Some("members:"));
    let lines: Vec<&str> = lines.collect();
    assert!(!lines.is_empty(), "the workspace has members");
    assert_eq!(
        lines.len() % 2,
        0,
        "each member is a name line and a source line"
    );
    let mut repositories = BTreeSet::new();
    for pair in lines.chunks(2) {
        let name = pair[0].strip_prefix("  - name: ").expect("a name line");
        let repository = pair[1]
            .strip_prefix("    source: ../../")
            .expect("a source line naming a sibling checkout");
        assert!(!repository.contains('/'), "{repository} is one directory");
        assert_eq!(name, member(repository), "the member name of {repository}");
        assert!(
            repositories.insert(repository),
            "{repository} is listed once"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// story:collect-specifications
// ---------------------------------------------------------------------------------------------

#[test]
fn specifications_are_read_at_origin_main_one_per_repository_not_archived() {
    let case = Case::new("specifications_at_origin_main");
    let repos = ["alpha", "beta.io", "gamma", "delta", "epsilon", "zeta"];
    case.workspace(&repos);
    case.repository(
        "alpha",
        &[file("AGENTS.md", "# alpha\n")],
        &[
            fixture("alpha"),
            vec![
                project("alpha"),
                artifact(
                    "executable-system-specification:alpha",
                    "validated",
                    Some("Alpha's specification"),
                ),
            ],
        ]
        .concat(),
    );
    case.repository(
        "beta.io",
        &[file("spec/system.yaml", "format: ess/22\n")],
        &[
            fixture("beta.io"),
            vec![
                file("target/spec/system.yaml", "format: ess/22\n"),
                file(
                    "node_modules/tool/ess-inputs.yaml",
                    "format: ess-inputs/2\n",
                ),
                file("docs/target/system.yaml", "format: ess/22\n"),
            ],
        ]
        .concat(),
    );
    case.repository(
        "gamma",
        &[file("spec/system.yaml", "format: ess/22\n")],
        &fixture("gamma"),
    );
    case.repository("delta", &[], &fixture("delta"));
    case.repository("epsilon", &[], &fixture("epsilon"));
    case.repository("zeta", &[], &fixture("zeta"));
    let before = case.untouched(&repos);

    let sources = case.specification_sources(&Path::new(FIXTURES).join("repositories.json"));
    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let rows = case.specifications();
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
        vec![
            (
                "alpha",
                SpecificationPresence::Present,
                Some("spec"),
                Some("ess/22"),
                Some("ess 0.55"),
                ValidationResult::Valid,
                0,
                Some(0),
                Some(0),
                Some("validated"),
            ),
            (
                "beta.io",
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
                "epsilon",
                SpecificationPresence::Present,
                Some("."),
                Some("ess/22"),
                None,
                ValidationResult::Refused,
                1,
                None,
                None,
                None,
            ),
            (
                "gamma",
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
            (
                "zeta",
                SpecificationPresence::Present,
                Some("model"),
                Some("ess/21"),
                Some("ess 0.55"),
                ValidationResult::Refused,
                1,
                None,
                None,
                None,
            ),
        ]
    );
    assert!(
        rows.iter().all(|row| row.snapshot_id == taken.snapshot_id),
        "every row is filed under the snapshot taken"
    );

    // The acceptance's count, as its live check takes it: every repository not archived whose
    // origin/main lists an `ess-inputs.yaml` or `system.yaml` outside `target/` and `node_modules/`.
    let listed: Vec<Value> = serde_json::from_str(
        &fs::read_to_string(Path::new(FIXTURES).join("repositories.json")).expect("the fixture"),
    )
    .expect("JSON");
    let present = listed
        .iter()
        .filter(|repository| repository["isArchived"] == json!(false))
        .filter(|repository| {
            let dir = case.checkout(repository["name"].as_str().expect("a name"));
            git(&dir, &["ls-tree", "-r", "--name-only", "upstream"])
                .lines()
                .filter(|path| {
                    !path
                        .split('/')
                        .any(|part| part == "target" || part == "node_modules")
                })
                .any(|path| {
                    path.rsplit('/')
                        .next()
                        .is_some_and(|name| name == "ess-inputs.yaml" || name == "system.yaml")
                })
        })
        .count();
    assert_eq!(
        rows.iter()
            .filter(|row| row.presence == SpecificationPresence::Present)
            .count(),
        present
    );
    assert_eq!(
        case.untouched(&repos),
        before,
        "a checkout's HEAD or working tree changed"
    );
    assert!(
        case.state().join("specs/alpha.json").is_file(),
        "alpha's synthesis is written under state/specs/"
    );
}

#[test]
fn conductors_own_specification_reads_present_valid_with_the_counts_synthesis_prints() {
    let case = Case::new("specifications_conductor");
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec");
    case.workspace(&["conductor"]);
    let mut upstream: Vec<(String, String)> = tree(&spec)
        .into_iter()
        .map(|(file, text)| (format!("spec/{file}"), text))
        .collect();
    upstream.push(project("conductor"));
    upstream.push(artifact(
        "executable-system-specification:conductor",
        "validated",
        Some("Conductor specification (spec/)"),
    ));
    case.repository("conductor", &[], &upstream);
    let repositories = case.dir.join("repositories.json");
    write(
        &repositories,
        r#"[{"name": "conductor", "isArchived": false}]"#,
    );

    let synthesis = Command::new("ess")
        .args([
            "verify",
            "conform",
            "synthesize",
            "--path",
            path(&spec),
            "--out",
        ])
        .arg(case.dir.join("expected.json"))
        .stdin(Stdio::null())
        .output()
        .expect("ess runs");
    assert!(synthesis.status.success(), "{}", describe(&synthesis));
    let (scenarios, refusals) = synthesis_counts(&String::from_utf8_lossy(&synthesis.stdout));

    let sources = case.specification_sources(&repositories);
    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let rows = case.specifications();
    assert_eq!(rows.len(), 1, "{rows:?}");
    let row = &rows[0];
    assert_eq!(row.repository.0, "conductor");
    assert_eq!(row.presence, SpecificationPresence::Present);
    assert_eq!(row.path.as_deref(), Some("spec"));
    assert_eq!(
        row.format.as_deref(),
        Some(top_level(&spec.join("system.yaml"), "format").as_str())
    );
    assert!(
        row.format
            .as_deref()
            .is_some_and(|format| format.starts_with("ess/"))
    );
    assert_eq!(
        row.required_ess.as_deref(),
        Some(top_level(&spec.join("ess-inputs.yaml"), "requires").as_str())
    );
    assert_eq!(row.validation, ValidationResult::Valid);
    assert_eq!(row.validation_refusals, 0);
    assert_eq!(row.scenarios, Some(scenarios));
    assert_eq!(row.synthesis_refusals, Some(refusals));
    assert_eq!(row.conformance_status.as_deref(), Some("validated"));
}

#[test]
fn an_opt_out_is_each_phrasing_the_list_keeps_and_no_wording_near_one() {
    let case = Case::new("specifications_opt_out");
    let mut repos = Vec::new();
    let mut listed = Vec::new();
    for (index, text) in OPTED_OUT.iter().enumerate() {
        repos.push((format!("opted-{index}"), *text));
    }
    for (index, text) in NOT_OPTED_OUT.iter().enumerate() {
        repos.push((format!("near-{index}"), *text));
    }
    let names: Vec<&str> = repos.iter().map(|(name, _)| name.as_str()).collect();
    case.workspace(&names);
    for (name, text) in &repos {
        case.repository(
            name,
            &[],
            &[file(
                "AGENTS.md",
                &format!("# AGENTS.md: {name}\n\n{text}\n"),
            )],
        );
        listed.push(json!({"name": name, "isArchived": false}));
    }
    let repositories = case.dir.join("repositories.json");
    write(&repositories, &Value::Array(listed).to_string());

    let sources = case.specification_sources(&repositories);
    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let observed: Vec<(String, SpecificationPresence)> = case
        .specifications()
        .into_iter()
        .map(|row| (row.repository.0, row.presence))
        .collect();
    let mut expected: Vec<(String, SpecificationPresence)> = repos
        .iter()
        .map(|(name, _)| {
            let presence = if name.starts_with("opted-") {
                SpecificationPresence::OptedOut
            } else {
                SpecificationPresence::Missing
            };
            (name.clone(), presence)
        })
        .collect();
    expected.sort_by(|a, b| a.0.cmp(&b.0));
    assert_eq!(observed, expected);
}

#[test]
fn a_repository_not_in_the_workspace_fails_the_snapshot_naming_it() {
    let case = Case::new("specifications_unknown_repository");
    case.workspace(&["alpha"]);
    case.repository("alpha", &[], &fixture("alpha"));
    let repositories = case.dir.join("repositories.json");
    write(
        &repositories,
        r#"[{"name": "alpha", "isArchived": false}, {"name": "omega", "isArchived": false}]"#,
    );

    let sources = case.specification_sources(&repositories);
    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Failed);
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(reason.starts_with("specifications: "), "{reason}");
    assert!(reason.contains("omega"), "names the repository: {reason}");
}

#[test]
fn an_ess_that_fails_twice_fails_the_snapshot_naming_the_repository_and_the_command() {
    let case = Case::new("specifications_ess_fails");
    case.workspace(&["alpha"]);
    case.repository("alpha", &[], &fixture("alpha"));
    let repositories = case.dir.join("repositories.json");
    write(&repositories, r#"[{"name": "alpha", "isArchived": false}]"#);
    let log = case.dir.join("ess.log");
    let mut sources = case.specification_sources(&repositories);
    sources.ess = words(&["sh", "-c", "echo ran >> \"$0\"; exit 3", path(&log)]);

    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Failed);
    let reason = taken.reason.expect("a failed snapshot has a reason");
    assert!(reason.contains("alpha"), "names the repository: {reason}");
    assert!(
        reason.contains(&format!("echo ran >> \"$0\"; exit 3 {}", path(&log))),
        "names the command: {reason}"
    );
    assert_eq!(
        fs::read_to_string(&log).expect("ess ran").lines().count(),
        2,
        "ess ran once and once more"
    );
    assert!(case.specifications().is_empty());
}
