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
//! Snapshots are taken through the library ([`snapshot::take`]). Blockers and member failures are
//! read back through the built binary's `snapshot blockers` and `snapshot member-failures`,
//! specifications through the generated query.

mod active;
mod common;
mod scratch;

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use conductor_cli::collect::{Collector, blockers, repositories, specifications};
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
        active::isolate();
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

    /// Gives the checkout of `repo` an `origin` a real `git fetch` reads with no network: the bare
    /// repository `forge/<repo>.git` of the case, whose `main` is the checkout's `upstream`.
    fn origin(&self, repo: &str) {
        let forge = self.dir.join("forge");
        fs::create_dir_all(&forge).expect("create forge/");
        let bare = forge.join(format!("{repo}.git"));
        git(&forge, &["init", "--bare", "--quiet", path(&bare)]);
        let checkout = self.checkout(repo);
        // Pushed to the path, not to `origin`, so `refs/remotes/origin/main` stays where it was.
        git(
            &checkout,
            &["push", "--quiet", path(&bare), "upstream:main"],
        );
        git(&checkout, &["remote", "add", "origin", path(&bare)]);
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
        self.view("blockers")
    }

    /// The recorded member failures, through `snapshot member-failures --format json`, without
    /// the identities the store assigns, sorted (`story:snapshot-member-failures`).
    fn member_failures(&self) -> Vec<Value> {
        self.view("member-failures")
    }

    /// The rows of `snapshot <view> --format json`, without the identities the store assigns,
    /// sorted.
    fn view(&self, view: &str) -> Vec<Value> {
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
        assert_eq!(
            fs::read_dir(self.dir.join("work"))
                .expect("read work/")
                .count(),
            0,
            "`snapshot {view}` planted something in its working directory"
        );
        let Value::Array(rows) = serde_json::from_slice(&output.stdout).expect("JSON rows") else {
            panic!("`snapshot {view}` printed no array: {}", describe(&output));
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

/// A fetch that fails twice is a member failure, not a failed snapshot
/// (`story:snapshot-member-failures`): the snapshot completes, one member failure names the
/// repository, the collector and the command's first error line, and the member's blockers are
/// read at its `origin/main` as last fetched, beside the other member's.
#[test]
fn a_fetch_that_fails_twice_is_a_member_failure_naming_the_repository_and_the_command() {
    let case = Case::new("blockers_fetch_fails");
    case.workspace(&["alpha", "beta.io"]);
    for repo in ["alpha", "beta.io"] {
        case.repository(
            repo,
            &[
                project(&member(repo)),
                artifact(
                    "decision-blocker:last-fetched",
                    "open",
                    Some("Last fetched"),
                ),
            ],
            &[
                project(&member(repo)),
                artifact("decision-blocker:x", "open", Some("X")),
            ],
        );
    }
    let log = case.dir.join("fetches.log");
    let mut sources = case.blocker_sources();
    // The fetch fails in alpha's checkout, printing two lines; in beta.io's it moves the ref.
    let script = "pwd >> \"$0\"; case \"$(pwd)\" in */alpha) \
                  echo 'fatal: could not read from remote repository' >&2; \
                  echo 'Please make sure you have the correct access rights' >&2; exit 1;; esac; \
                  exec git update-ref refs/remotes/origin/main refs/heads/upstream";
    sources.fetch = words(&["sh", "-c", script, path(&log)]);

    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let fetches = fs::read_to_string(&log).expect("the fetch ran");
    let alpha = fs::canonicalize(case.checkout("alpha")).expect("alpha's checkout");
    let beta = fs::canonicalize(case.checkout("beta.io")).expect("beta.io's checkout");
    assert_eq!(
        fetches.lines().collect::<Vec<_>>(),
        vec![path(&alpha), path(&alpha), path(&beta)],
        "the fetch ran in alpha's checkout once and once more, then in beta.io's once"
    );
    let failures = case.member_failures();
    assert_eq!(failures.len(), 1, "one member failure: {failures:?}");
    let failure = &failures[0];
    assert_eq!(failure["member"], json!("alpha"), "{failure}");
    assert_eq!(failure["collector"], json!("blockers"), "{failure}");
    let line = failure["error_line"].as_str().expect("an error line");
    assert!(
        line.contains(&format!("{script} {}", path(&log))),
        "names the command: {line}"
    );
    assert!(
        line.ends_with("exited 1: fatal: could not read from remote repository"),
        "the command's first error line, and no more: {line}"
    );
    let mut expected = vec![
        json!({"repository": "alpha", "reference": "decision-blocker:last-fetched",
               "blocker_kind": "decision-blocker", "title": "Last fetched"}),
        json!({"repository": "beta.io", "reference": "decision-blocker:x",
               "blocker_kind": "decision-blocker", "title": "X"}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(case.blockers(), expected);
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

/// An `ess` that fails twice over one repository is a member failure, not a failed snapshot
/// (`story:snapshot-member-failures`): the snapshot completes, one member failure names the
/// repository, the collector and the command's first error line, no specification of it is
/// recorded, and the other repository's is.
#[test]
fn an_ess_that_fails_twice_is_a_member_failure_naming_the_repository_and_the_command() {
    let case = Case::new("specifications_ess_fails");
    case.workspace(&["alpha", "beta.io"]);
    case.repository("alpha", &[], &fixture("alpha"));
    case.repository("beta.io", &[], &fixture("beta.io"));
    let repositories = case.dir.join("repositories.json");
    write(
        &repositories,
        r#"[{"name": "alpha", "isArchived": false}, {"name": "beta.io", "isArchived": false}]"#,
    );
    let log = case.dir.join("ess.log");
    let mut sources = case.specification_sources(&repositories);
    let script = "echo ran >> \"$0\"; echo 'error: ess broke' >&2; echo 'more' >&2; exit 3";
    sources.ess = words(&["sh", "-c", script, path(&log)]);

    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    assert_eq!(
        fs::read_to_string(&log).expect("ess ran").lines().count(),
        2,
        "ess ran once and once more, over alpha only"
    );
    let failures = case.member_failures();
    assert_eq!(failures.len(), 1, "one member failure: {failures:?}");
    let failure = &failures[0];
    assert_eq!(failure["member"], json!("alpha"), "{failure}");
    assert_eq!(failure["collector"], json!("specifications"), "{failure}");
    let line = failure["error_line"].as_str().expect("an error line");
    assert!(
        line.contains(&format!("{script} {}", path(&log))),
        "names the command: {line}"
    );
    assert!(
        line.ends_with("exited 3: error: ess broke"),
        "the command's first error line, and no more: {line}"
    );
    let rows = case.specifications();
    assert_eq!(
        rows.iter()
            .map(|row| (row.repository.0.as_str(), row.presence))
            .collect::<Vec<_>>(),
        [("beta.io", SpecificationPresence::Missing)]
    );
}

// story:snapshot-absent-member
// ---------------------------------------------------------------------------------------------

/// The environment variable that makes
/// [`a_member_whose_checkout_is_absent_is_skipped_and_the_snapshot_completes`] run its child.
const ABSENT_CHILD: &str = "CONDUCTOR_COLLECT_STORES_ABSENT_CHILD";

/// Three members, the checkout of `gamma` absent: `aep plan workspace members` reports it
/// `"state": "absent"`, with exit 0. The blockers and specifications collectors skip it, the
/// snapshot completes with alpha's and beta.io's blockers and specifications, and the skipped
/// member is named on one line of standard error.
///
/// The collectors write that line on the process's standard error, which the test harness
/// captures where no case can read it. So the case runs itself again in a child process with
/// `--nocapture` ([`absent_member_snapshot`]) and reads the child's standard error.
#[test]
fn a_member_whose_checkout_is_absent_is_skipped_and_the_snapshot_completes() {
    const NAME: &str = "a_member_whose_checkout_is_absent_is_skipped_and_the_snapshot_completes";
    if std::env::var_os(ABSENT_CHILD).is_some() {
        absent_member_snapshot();
        return;
    }
    let output = Command::new(std::env::current_exe().expect("the test binary's path"))
        .args([NAME, "--exact", "--nocapture", "--test-threads=1"])
        .env(ABSENT_CHILD, "1")
        .stdin(Stdio::null())
        .output()
        .expect("the test binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the child case failed:\n{stdout}\n{stderr}"
    );
    assert!(
        stdout.contains("test result: ok. 1 passed"),
        "the child ran the case:\n{stdout}"
    );
    assert_eq!(
        stderr, "member gamma: checkout absent, skipped\n",
        "one line on standard error, naming the absent member once"
    );
}

/// What [`a_member_whose_checkout_is_absent_is_skipped_and_the_snapshot_completes`] runs in its
/// child process.
fn absent_member_snapshot() {
    let case = Case::new("absent_member");
    case.workspace(&["alpha", "beta.io", "gamma"]);
    for (repo, blocker) in [("alpha", "Pick a store"), ("beta.io", "Restart")] {
        case.repository(
            repo,
            &[],
            &[
                fixture(repo),
                vec![
                    project(&member(repo)),
                    artifact("operator-blocker:waiting", "open", Some(blocker)),
                ],
            ]
            .concat(),
        );
    }
    assert!(
        !case.checkout("gamma").exists(),
        "gamma's checkout is absent"
    );
    let repositories = case.dir.join("repositories.json");
    write(
        &repositories,
        r#"[{"name": "alpha", "isArchived": false}, {"name": "beta.io", "isArchived": false},
            {"name": "gamma", "isArchived": false}]"#,
    );
    let blocker_sources = case.blocker_sources();
    let specification_sources = case.specification_sources(&repositories);

    let store = store::open(&case.state()).expect("open the store");
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
    let mut expected = vec![
        json!({"repository": "alpha", "reference": "operator-blocker:waiting",
               "blocker_kind": "operator-blocker", "title": "Pick a store"}),
        json!({"repository": "beta.io", "reference": "operator-blocker:waiting",
               "blocker_kind": "operator-blocker", "title": "Restart"}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(case.blockers(), expected);
    let repositories: Vec<String> = case
        .specifications()
        .into_iter()
        .map(|row| row.repository.0)
        .collect();
    assert_eq!(repositories, vec!["alpha", "beta.io"]);
}

/// A checkout whose directory is not its repository's name on the forge (the records checkout
/// `records` is `conductor-archive` there) is matched to the owner's listing through the
/// repository its `origin` remote names, and the snapshot completes (conductor's NEED
/// N-20261012-03).
#[test]
fn a_member_whose_directory_differs_from_its_forge_name_is_matched_by_its_origin() {
    let case = Case::new("forge_name");
    case.workspace(&["alpha", "beta.io"]);
    for repo in ["alpha", "beta.io"] {
        case.repository(
            repo,
            &[],
            &[fixture(repo), vec![project(&member(repo))]].concat(),
        );
    }
    git(
        &case.checkout("beta.io"),
        &[
            "remote",
            "add",
            "origin",
            "https://example.invalid/example-org/forge-name.git",
        ],
    );
    let repositories = case.dir.join("repositories.json");
    write(
        &repositories,
        r#"[{"name": "alpha", "isArchived": false},
            {"name": "forge-name", "isArchived": false}]"#,
    );
    let specification_sources = case.specification_sources(&repositories);

    let taken = case.take("specifications", move |record| {
        specifications::collect_from(&specification_sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let mut repositories: Vec<String> = case
        .specifications()
        .into_iter()
        .map(|row| row.repository.0)
        .collect();
    repositories.sort();
    assert_eq!(repositories, vec!["alpha", "forge-name"]);
}

// ---------------------------------------------------------------------------------------------
// story:fetch-without-prompts
// ---------------------------------------------------------------------------------------------

/// The environment variable that makes a case of [`git_environment_of`] run its snapshots in a
/// child process; it names the directory of the fake `git`.
const PROMPT_CHILD: &str = "CONDUCTOR_COLLECT_STORES_PROMPT_CHILD";

/// The ssh options every git command of a collector runs with.
const SSH_OPTIONS: &str = "-o BatchMode=yes -o ConnectTimeout=10";

/// The collectors whose git commands [`git_environment_of`] records, in the order it runs them.
const GIT_COLLECTORS: [&str; 3] = ["repositories", "blockers", "specifications"];

/// (a) Every git command the repositories, blockers and specifications collectors run has
/// `GIT_TERMINAL_PROMPT=0`, and a `GIT_SSH_COMMAND` of `ssh` with batch mode and a connect
/// timeout, when the caller names no ssh command of its own.
#[test]
fn every_collector_git_command_runs_with_no_prompt_and_ssh_in_batch_mode() {
    const NAME: &str = "every_collector_git_command_runs_with_no_prompt_and_ssh_in_batch_mode";
    if let Some(fake) = std::env::var_os(PROMPT_CHILD) {
        prompted_snapshots("prompt_default", Path::new(&fake));
        return;
    }
    let expected = format!("ssh {SSH_OPTIONS}");
    for (collector, lines) in git_environment_of("prompt_default", NAME, None) {
        for line in lines {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.get(1..),
                Some(&["0", expected.as_str()][..]),
                "{collector}: {line:?}"
            );
        }
    }
}

/// (b) A `GIT_SSH_COMMAND` the caller sets is kept, with batch mode and the connect timeout after
/// it.
#[test]
fn a_callers_ssh_command_is_kept_with_batch_mode_after_it() {
    const NAME: &str = "a_callers_ssh_command_is_kept_with_batch_mode_after_it";
    if let Some(fake) = std::env::var_os(PROMPT_CHILD) {
        prompted_snapshots("prompt_caller", Path::new(&fake));
        return;
    }
    let expected = format!("ssh -i /tmp/key {SSH_OPTIONS}");
    for (collector, lines) in git_environment_of("prompt_caller", NAME, Some("ssh -i /tmp/key")) {
        for line in lines {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.get(1..),
                Some(&["0", expected.as_str()][..]),
                "{collector}: {line:?}"
            );
        }
    }
}

/// (c) With no `GIT_SSH_COMMAND` of the caller's, the `core.sshCommand` git's config gives in the
/// repository is kept, so a key named there still reaches ssh; the caller's variable wins over
/// it, as in git, and without either the command is `ssh`.
#[test]
fn a_repositorys_core_ssh_command_is_kept_and_the_callers_wins() {
    let case = scratch::Scratch::new("collect_stores/core-ssh-command");
    let dir = case.path().to_owned();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(&dir)
            .output()
            .expect("git runs");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            describe(&output)
        );
    };
    git(&["init", "--quiet"]);
    assert_eq!(
        conductor_cli::collect::ssh_command(Some("  ".to_owned()), Some(&dir)),
        conductor_cli::collect::ssh_command(None, Some(&dir)),
        "a blank caller's command is no command"
    );
    git(&["config", "core.sshCommand", "ssh -i /tmp/repo-key"]);
    assert_eq!(
        conductor_cli::collect::ssh_command(None, Some(&dir)),
        "ssh -i /tmp/repo-key"
    );
    assert_eq!(
        conductor_cli::collect::ssh_command(Some("ssh -i /tmp/key".to_owned()), Some(&dir)),
        "ssh -i /tmp/key"
    );
}

/// Runs the case `test` of this file again in a child process whose `PATH` starts with a fake
/// `git`, which writes its arguments, `GIT_TERMINAL_PROMPT` and `GIT_SSH_COMMAND` (tab-separated,
/// `unset` for a variable that is not set) on one line of its log and then runs the real `git`.
/// The child's `GIT_SSH_COMMAND` is `ssh` when given, else not set. Answers each collector of
/// [`GIT_COLLECTORS`] with the lines its git commands wrote, each collector having run at least
/// one.
fn git_environment_of(case: &str, test: &str, ssh: Option<&str>) -> Vec<(String, Vec<String>)> {
    let fake_dir = scratch::Scratch::new(format!("collect_stores/{case}-fake-git"));
    let fake = fake_dir.path().to_owned();
    let path_variable = std::env::var_os("PATH").unwrap_or_default();
    let real = std::env::split_paths(&path_variable)
        .map(|dir| dir.join("git"))
        .find(|git| git.is_file())
        .expect("git is on PATH");
    let script = fake.join("git");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nprintf '%s\\t%s\\t%s\\n' \"$*\" \"${{GIT_TERMINAL_PROMPT-unset}}\" \
             \"${{GIT_SSH_COMMAND-unset}}\" >> '{}'\nexec '{}' \"$@\"\n",
            path(&fake.join("git.log")),
            path(&real)
        ),
    )
    .expect("write the fake git");
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    let child_path = std::env::join_paths(
        std::iter::once(fake.clone()).chain(std::env::split_paths(&path_variable)),
    )
    .expect("a PATH");
    let mut child = Command::new(std::env::current_exe().expect("the test binary's path"));
    child
        .args([test, "--exact", "--nocapture", "--test-threads=1"])
        .env(PROMPT_CHILD, &fake)
        .env("PATH", child_path)
        .stdin(Stdio::null());
    match ssh {
        Some(ssh) => child.env("GIT_SSH_COMMAND", ssh),
        None => child.env_remove("GIT_SSH_COMMAND"),
    };
    let output = child.output().expect("the test binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed"),
        "the child case failed: {}",
        describe(&output)
    );
    GIT_COLLECTORS
        .iter()
        .map(|collector| {
            let lines: Vec<String> = fs::read_to_string(fake.join(format!("{collector}.log")))
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect();
            assert!(!lines.is_empty(), "the {collector} collector ran no git");
            ((*collector).to_owned(), lines)
        })
        .collect()
}

/// What [`git_environment_of`] runs in its child process: two members whose `origin` is a bare
/// repository of the case, read by one snapshot of each collector of [`GIT_COLLECTORS`] with its
/// real git commands, the fetch included. After each snapshot the fake `git`'s log under `fake` is
/// kept as `<collector>.log`.
fn prompted_snapshots(name: &str, fake: &Path) {
    let case = Case::new(name);
    let repos = ["alpha", "beta.io"];
    case.workspace(&repos);
    for repo in repos {
        case.repository(
            repo,
            &[project(&member(repo))],
            &[
                project(&member(repo)),
                artifact("decision-blocker:x", "open", Some("X")),
            ],
        );
        case.origin(repo);
    }
    let listing = case.dir.join("listing.json");
    write(
        &listing,
        r#"[{"name": "alpha", "visibility": "PUBLIC", "isArchived": false, "hasIssuesEnabled": false},
            {"name": "beta.io", "visibility": "PUBLIC", "isArchived": false, "hasIssuesEnabled": false}]"#,
    );
    let empty = case.dir.join("empty.json");
    write(&empty, "[]");
    let repositories_json = case.dir.join("repositories.json");
    write(
        &repositories_json,
        r#"[{"name": "alpha", "isArchived": false}, {"name": "beta.io", "isArchived": false}]"#,
    );
    let cat = |file: &Path| vec!["cat".to_owned(), path(file).to_owned()];
    let repository_sources = repositories::Sources {
        organization: "example-org".to_owned(),
        workspace: case.dir.join("example-org"),
        catalog: None,
        now: time::OffsetDateTime::now_utc(),
        repository_list: cat(&listing),
        open_issues: cat(&empty),
        latest_release: cat(&empty),
        main_commit: vec!["false".to_owned()],
        main_commits: vec!["false".to_owned()],
        commit: vec!["false".to_owned()],
        gates: vec!["false".to_owned()],
        waves: repositories::waves_command(),
        width: 2,
        bound: Duration::from_secs(60),
    };
    let real_fetch = words(&["git", "fetch", "--quiet", "origin"]);
    let mut blocker_sources = case.blocker_sources();
    blocker_sources.fetch = real_fetch.clone();
    let mut specification_sources = case.specification_sources(&repositories_json);
    specification_sources.workspace.fetch = real_fetch;
    // The fixtures' own git commands ran through the fake too: only the collectors' are kept.
    let log = fake.join("git.log");
    let _ = fs::remove_file(&log);

    let collectors: [(&'static str, Collector); 3] = [
        (
            "repositories",
            Collector::new("repositories", move |record| {
                repositories::collect_from(&repository_sources, record)
            }),
        ),
        (
            "blockers",
            Collector::new("blockers", move |record| {
                blockers::collect_from(&blocker_sources, record)
            }),
        ),
        (
            "specifications",
            Collector::new("specifications", move |record| {
                specifications::collect_from(&specification_sources, record)
            }),
        ),
    ];
    for (collector, run) in collectors {
        let store = store::open(&case.state()).expect("open the store");
        let taken = snapshot::take(store, &[run]).expect("the driver ends the snapshot");
        assert_eq!(
            taken.state,
            SnapshotState::Complete,
            "{collector}: {:?}",
            taken.reason
        );
        let kept = fake.join(format!("{collector}.log"));
        if log.exists() {
            fs::rename(&log, &kept).expect("keep the collector's log");
        } else {
            fs::write(&kept, "").expect("write an empty log");
        }
    }
    assert!(
        case.member_failures().is_empty(),
        "every member was read: {:?}",
        case.member_failures()
    );
}

/// (c) A fetch that hangs past the collector's bound is stopped there, and is a member failure
/// naming the member, `blockers` and the bound, the text [`collect::run`] gives; the snapshot
/// completes, with the member's blockers as last fetched beside the other member's.
///
/// [`collect::run`]: conductor_cli::collect::run
#[test]
fn a_fetch_that_hangs_past_the_bound_is_a_member_failure_and_the_snapshot_completes() {
    let case = Case::new("fetch_hangs");
    case.workspace(&["alpha", "beta.io"]);
    for repo in ["alpha", "beta.io"] {
        case.repository(
            repo,
            &[
                project(&member(repo)),
                artifact(
                    "decision-blocker:last-fetched",
                    "open",
                    Some("Last fetched"),
                ),
            ],
            &[
                project(&member(repo)),
                artifact("decision-blocker:x", "open", Some("X")),
            ],
        );
    }
    let mut sources = case.blocker_sources();
    sources.bound = Duration::from_secs(3);
    sources.fetch = words(&[
        "sh",
        "-c",
        "case \"$(pwd)\" in */alpha) exec sleep 60;; esac; \
         exec git update-ref refs/remotes/origin/main refs/heads/upstream",
    ]);

    let started = Instant::now();
    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    assert!(
        started.elapsed() < Duration::from_secs(40),
        "the hanging fetch was not stopped at its bound: {:?}",
        started.elapsed()
    );
    let failures = case.member_failures();
    assert_eq!(failures.len(), 1, "one member failure: {failures:?}");
    assert_eq!(failures[0]["member"], json!("alpha"));
    assert_eq!(failures[0]["collector"], json!("blockers"));
    let line = failures[0]["error_line"].as_str().expect("an error line");
    assert!(
        line.contains("gave no answer within 3 s"),
        "says it timed out: {line}"
    );
    let mut expected = vec![
        json!({"repository": "alpha", "reference": "decision-blocker:last-fetched",
               "blocker_kind": "decision-blocker", "title": "Last fetched"}),
        json!({"repository": "beta.io", "reference": "decision-blocker:x",
               "blocker_kind": "decision-blocker", "title": "X"}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(case.blockers(), expected);
}

/// (d) A member whose real `git fetch --quiet origin` is refused (its `origin` is no repository)
/// is a member failure naming the fetch's first error line, and its blockers are read at its
/// `refs/remotes/origin/main` as last fetched; the other member is fetched from its `origin`.
#[test]
fn a_member_whose_fetch_is_refused_is_read_as_last_fetched() {
    let case = Case::new("fetch_refused");
    case.workspace(&["alpha", "beta.io"]);
    for repo in ["alpha", "beta.io"] {
        case.repository(
            repo,
            &[
                project(&member(repo)),
                artifact(
                    "decision-blocker:last-fetched",
                    "open",
                    Some("Last fetched"),
                ),
            ],
            &[
                project(&member(repo)),
                artifact("decision-blocker:x", "open", Some("X")),
            ],
        );
    }
    case.origin("beta.io");
    git(
        &case.checkout("alpha"),
        &[
            "remote",
            "add",
            "origin",
            path(&case.dir.join("forge/no-such-repository.git")),
        ],
    );
    let sources = blockers::Sources::new(case.hub(), &case.state());
    assert_eq!(sources.fetch, words(&["git", "fetch", "--quiet", "origin"]));

    let taken = case.take("blockers", move |record| {
        blockers::collect_from(&sources, record)
    });

    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);
    let failures = case.member_failures();
    assert_eq!(failures.len(), 1, "one member failure: {failures:?}");
    assert_eq!(failures[0]["member"], json!("alpha"));
    assert_eq!(failures[0]["collector"], json!("blockers"));
    let line = failures[0]["error_line"].as_str().expect("an error line");
    assert!(
        line.starts_with("`git fetch --quiet origin` exited 128: fatal: "),
        "the fetch and its first error line: {line}"
    );
    let mut expected = vec![
        json!({"repository": "alpha", "reference": "decision-blocker:last-fetched",
               "blocker_kind": "decision-blocker", "title": "Last fetched"}),
        json!({"repository": "beta.io", "reference": "decision-blocker:x",
               "blocker_kind": "decision-blocker", "title": "X"}),
    ];
    expected.sort_by_key(Value::to_string);
    assert_eq!(case.blockers(), expected);
}

// ---------------------------------------------------------------------------------------------
// Gates hooks (story:gates-hooks-fresh)
// ---------------------------------------------------------------------------------------------

/// The repositories collector records the version a checkout's Gates hook prints
/// (`.git/b10x-gates-hooks/b10x-gates --version`) and the one `b10x-gates --version` on `PATH`
/// prints; the board's deviations name `stale gates hooks` for the checkout whose hook prints an
/// older version, and not for the one without a hook. Both fakes are written by this test.
#[test]
fn a_checkout_whose_gates_hook_lags_is_recorded_and_the_board_names_it_stale() {
    let case = Case::new("gates_hooks");
    let repos = ["alpha", "beta.io"];
    for repo in repos {
        case.repository(repo, &[project(&member(repo))], &[project(&member(repo))]);
        case.origin(repo);
    }
    let hooks = case.checkout("alpha").join(".git/b10x-gates-hooks");
    fs::create_dir_all(&hooks).expect("create the hooks directory");
    let hook = hooks.join("b10x-gates");
    write(&hook, "#!/bin/sh\necho 'b10x-gates 0.1.14'\n");
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).expect("make the hook run");
    let listing = case.dir.join("listing.json");
    write(
        &listing,
        r#"[{"name": "alpha", "visibility": "PUBLIC", "isArchived": false, "hasIssuesEnabled": false},
            {"name": "beta.io", "visibility": "PUBLIC", "isArchived": false, "hasIssuesEnabled": false}]"#,
    );
    let empty = case.dir.join("empty.json");
    write(&empty, "[]");
    let cat = |file: &Path| vec!["cat".to_owned(), path(file).to_owned()];
    let sources = repositories::Sources {
        organization: "example-org".to_owned(),
        workspace: case.dir.join("example-org"),
        catalog: None,
        now: time::OffsetDateTime::now_utc(),
        repository_list: cat(&listing),
        open_issues: cat(&empty),
        latest_release: cat(&empty),
        main_commit: vec!["false".to_owned()],
        main_commits: vec!["false".to_owned()],
        commit: vec!["false".to_owned()],
        gates: vec!["echo".to_owned(), "b10x-gates 0.1.17".to_owned()],
        waves: repositories::waves_command(),
        width: 2,
        bound: Duration::from_secs(60),
    };

    let taken = case.take("repositories", move |record| {
        repositories::collect_from(&sources, record)
    });
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);

    let mut recorded: Vec<(Value, Value, Value)> = case
        .view("repositories")
        .into_iter()
        .map(|row| {
            (
                row["repository"].clone(),
                row["gates_hooks_version"].clone(),
                row["gates_version"].clone(),
            )
        })
        .collect();
    recorded.sort_by_key(|row| row.0.to_string());
    assert_eq!(
        recorded,
        [
            (json!("alpha"), json!("0.1.14"), json!("0.1.17")),
            (json!("beta.io"), Value::Null, json!("0.1.17")),
        ]
    );

    let board = case.dir.join("STATUS.md");
    let output = common::conductor(case.dir.join("home"))
        .current_dir(case.dir.join("work"))
        .args(["--state-dir", path(&case.state())])
        .args(["snapshot", "publish-board", "--path", path(&board)])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert!(output.status.success(), "{}", describe(&output));
    let board = fs::read_to_string(&board).expect("read the board");
    let deviations: Vec<&str> = board
        .lines()
        .filter(|line| line.starts_with("| alpha |") || line.starts_with("| beta.io |"))
        .filter(|line| line.contains("stale gates hooks"))
        .collect();
    assert_eq!(deviations.len(), 1, "{board}");
    assert!(deviations[0].starts_with("| alpha |"), "{board}");
}

// ---------------------------------------------------------------------------------------------
// Wave width (story:wave-width)
// ---------------------------------------------------------------------------------------------

/// One story of a planning store at its path: `status`, one `scope` path, `depends_on` entries,
/// and one move to `active` at each of `active_at`.
fn story(
    slug: &str,
    status: &str,
    scope: &str,
    depends_on: &[&str],
    active_at: &[&str],
) -> (String, String) {
    let mut text = format!(
        "---\nformat: aep.planning-md/3\nid: story:{slug}\nkind: story\nstatus: {status}\n\
         title: {slug}\n"
    );
    if !depends_on.is_empty() {
        text.push_str("relations:\n");
        for target in depends_on {
            text.push_str(&format!("- depends_on: story:{target}\n"));
        }
    }
    text.push_str(&format!(
        "scope:\n- confidence: cited\n  path: {scope}\nrevision: 2\n"
    ));
    if !active_at.is_empty() {
        text.push_str("transitions:\n");
        for at in active_at {
            text.push_str(&format!(
                "- {{from: \"proposed\", to: \"active\", at: \"{at}\", actor: \"human:fixture\", \
                 revision: 2}}\n"
            ));
        }
    }
    text.push_str("---\n## Acceptance\n\n- It holds.\n");
    (format!(".engineering/planning/story/{slug}.md"), text)
}

/// The store every case repository of the wave-width test holds on `origin/main`: two stories
/// moved to `active` four minutes apart (the newest wave, width 2), one long before them, and
/// drafts whose first derived wave is three (`d4` waits on `d1`, `d5` shares `d1`'s scope).
fn wave_store(repo: &str) -> Vec<(String, String)> {
    vec![
        project(&member(repo)),
        story(
            "old",
            "implemented",
            "src/old.rs",
            &[],
            &["2026-10-01T08:00:00Z"],
        ),
        story("w1", "active", "src/w1.rs", &[], &["2026-10-02T10:00:00Z"]),
        story("w2", "active", "src/w2.rs", &[], &["2026-10-02T10:04:00Z"]),
        story("d1", "draft", "src/a.rs", &[], &[]),
        story("d2", "draft", "src/b.rs", &[], &[]),
        story("d3", "draft", "src/c.rs", &[], &[]),
        story("d4", "draft", "src/d.rs", &["d1"], &[]),
        story("d5", "draft", "src/a.rs", &[], &[]),
    ]
}

/// The repositories collector records each repository's newest wave width, the ready units the
/// store's first derived wave offers (`aep plan artifact waves --kind story --status draft`, the
/// installed `aep`) and the reason its newest wave page gives, all read at `origin/main` after a
/// fetch; the board names `narrow wave` for the repository whose width is under both the ready
/// units and `controllers.max_subagents` (the built-in 4) with a page that gives no reason, and
/// for none other.
#[test]
fn a_wave_narrower_than_its_ready_units_is_recorded_and_the_board_names_it_narrow() {
    let case = Case::new("wave_width");
    let repos = ["alpha", "beta.io", "gamma", "delta"];
    // alpha: the newest page is wave-10 (not wave-9, which a text order would pick), and it gives
    // no reason.
    let mut alpha = wave_store("alpha");
    alpha.push(file(
        "docs/waves/wave-9.md",
        "# Wave 9\n\nWidth: one unit, the rest collide.\n",
    ));
    alpha.push(file(
        "docs/waves/wave-10.md",
        "# Wave 10\n\n## Units\n\n| A | w1 |\n| B | w2 |\n",
    ));
    // beta.io: the same store, and its newest page gives a reason.
    let mut beta = wave_store("beta.io");
    beta.push(file(
        "docs/waves/wave-10.md",
        "# Wave 10\n\nWidth: two units share one crate with the drafts.\n",
    ));
    // gamma: two started together and one draft ready: not narrow.
    let gamma = vec![
        project(&member("gamma")),
        story("w1", "active", "src/w1.rs", &[], &["2026-10-02T10:00:00Z"]),
        story("w2", "active", "src/w2.rs", &[], &["2026-10-02T10:00:30Z"]),
        story("d1", "draft", "src/a.rs", &[], &[]),
    ];
    // Each checkout's working tree holds an older store with no story: only `origin/main` counts.
    for (repo, upstream) in [
        ("alpha", alpha),
        ("beta.io", beta),
        ("gamma", gamma),
        ("delta", vec![file("README.md", "# delta\n")]),
    ] {
        case.repository(repo, &[project(&member(repo))], &upstream);
        case.origin(repo);
    }
    let listing = case.dir.join("listing.json");
    let entries: Vec<String> = repos
        .iter()
        .map(|repo| {
            format!(
                r#"{{"name": "{repo}", "visibility": "PUBLIC", "isArchived": false, "hasIssuesEnabled": false}}"#
            )
        })
        .collect();
    write(&listing, &format!("[{}]", entries.join(",")));
    let empty = case.dir.join("empty.json");
    write(&empty, "[]");
    let cat = |file: &Path| vec!["cat".to_owned(), path(file).to_owned()];
    let sources = repositories::Sources {
        organization: "example-org".to_owned(),
        workspace: case.dir.join("example-org"),
        catalog: None,
        now: time::OffsetDateTime::now_utc(),
        repository_list: cat(&listing),
        open_issues: cat(&empty),
        latest_release: cat(&empty),
        main_commit: vec!["false".to_owned()],
        main_commits: vec!["false".to_owned()],
        commit: vec!["false".to_owned()],
        gates: vec!["false".to_owned()],
        waves: repositories::waves_command(),
        width: 2,
        bound: Duration::from_secs(60),
    };

    let taken = case.take("repositories", move |record| {
        repositories::collect_from(&sources, record)
    });
    assert_eq!(taken.state, SnapshotState::Complete, "{:?}", taken.reason);

    let mut recorded: Vec<(Value, Value, Value, Value)> = case
        .view("repositories")
        .into_iter()
        .map(|row| {
            (
                row["repository"].clone(),
                row["wave_width"].clone(),
                row["wave_ready"].clone(),
                row["wave_reason"].clone(),
            )
        })
        .collect();
    recorded.sort_by_key(|row| row.0.to_string());
    assert_eq!(
        recorded,
        [
            (json!("alpha"), json!(2), json!(3), Value::Null),
            (
                json!("beta.io"),
                json!(2),
                json!(3),
                json!("two units share one crate with the drafts.")
            ),
            (json!("delta"), Value::Null, Value::Null, Value::Null),
            (json!("gamma"), json!(2), json!(1), Value::Null),
        ]
    );

    let board = case.dir.join("STATUS.md");
    let output = common::conductor(case.dir.join("home"))
        .current_dir(case.dir.join("work"))
        .args(["--state-dir", path(&case.state())])
        .args(["snapshot", "publish-board", "--path", path(&board)])
        .stdin(Stdio::null())
        .output()
        .expect("the conductor binary runs");
    assert!(output.status.success(), "{}", describe(&output));
    let board = fs::read_to_string(&board).expect("read the board");
    let narrow: Vec<&str> = board
        .lines()
        .filter(|line| line.contains("narrow wave"))
        .collect();
    assert_eq!(narrow.len(), 1, "{board}");
    assert!(narrow[0].starts_with("| alpha |"), "{board}");
}
