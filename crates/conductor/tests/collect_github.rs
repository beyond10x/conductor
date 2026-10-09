//! `story:collect-repositories`, `story:collect-prs-and-ci` and `story:ci-runs-stale-answer`: the
//! `repositories` and `github` collectors, each over sources the case replaces.
//!
//! Every `gh` command line is `cat` of a fixture under `tests/fixtures/github/`, or `sh` around
//! one where a case counts the calls or answers differently from one call to the next. Every
//! local checkout is a git repository the case builds in its own directory in this test target's
//! temporary directory, beside a bare "origin", so the collector's `git fetch` runs only between
//! those two. No case reaches the network or a real checkout. Each case takes its snapshots
//! through the library ([`snapshot::take`], or [`snapshot::take_in`] with the case's state
//! directory for the `github` collector) and reads what the collector recorded through the
//! generated views, which carry every field.

mod active;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use conductor_cli::collect::{Collector, github, repositories};
use conductor_cli::snapshot::{self, Taken};
use conductor_cli::store;
use conductor_model::behaviour::Generated;
use conductor_model::config::{Catalog, CatalogNames};
use conductor_model::observation::obligations::{
    PullRequestsQuery, RepositoriesQuery, WorkflowRunsQuery,
};
use conductor_model::observation::{
    Mergeability, RecordWorkflowRun, Repositories, RepositoryName, RunConclusion, SnapshotState,
    Visibility,
};
use conductor_model::primitives::Timestamp;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// The clock every case reads: the 7-day window opens at 2026-09-30T12:00:00Z.
const NOW: &str = "2026-10-07T12:00:00Z";

/// Before the window: the date of every repository's first commit.
const OLD: &str = "2026-09-01T09:00:00Z";

// ---------------------------------------------------------------------------------------------
// The world a case builds
// ---------------------------------------------------------------------------------------------

/// One case's directory: `workspace/<repository>` are the local checkouts, `origins/` their bare
/// origins, `others/` clones that move an origin on behind a checkout's back, and `state/` the
/// store.
struct World {
    root: PathBuf,
}

/// A case that passed leaves nothing behind: its repositories carry a `.git` that `worktree gc`
/// will not remove from a finished tree. A failed case keeps its directory to read.
impl Drop for World {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.root);
        }
    }
}

impl World {
    fn new(case: &str) -> Self {
        active::isolate();
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("collect_github")
            .join(case);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        for dir in ["workspace", "origins", "others"] {
            fs::create_dir_all(root.join(dir)).expect("create the case's directories");
        }
        Self { root }
    }

    fn workspace(&self) -> PathBuf {
        self.root.join("workspace")
    }

    fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    fn origin(&self, name: &str) -> PathBuf {
        self.root.join("origins").join(format!("{name}.git"))
    }

    /// A repository `name`: a bare origin, and a checkout of it under the workspace holding one
    /// commit from before the window (`README.md`, `src/lib.rs`) on `main`, pushed.
    fn repository(&self, name: &str) -> PathBuf {
        let origin = self.origin(name);
        fs::create_dir_all(&origin).expect("create the origin");
        git(
            &origin,
            OLD,
            &["init", "--quiet", "--bare", "--initial-branch=main"],
        );
        let dir = self.workspace().join(name);
        fs::create_dir_all(&dir).expect("create the checkout");
        git(&dir, OLD, &["init", "--quiet", "--initial-branch=main"]);
        git(&dir, OLD, &["remote", "add", "origin", text(&origin)]);
        commit(
            &dir,
            OLD,
            "Start the repository",
            &[("README.md", "# fixture\n"), ("src/lib.rs", "")],
        );
        push(&dir, &["main"]);
        dir
    }

    /// The `omega` repository, whose `main` catalogs each of `cataloged` as an `omega.repository`
    /// subject, the way Omega's entity store files one: `catalog/store/subjects/<hex of the
    /// entity>/<hex of the id>.json`.
    fn omega(&self, cataloged: &[&str]) -> PathBuf {
        let dir = self.repository("omega");
        let files: Vec<(String, &str)> = cataloged
            .iter()
            .map(|name| {
                (
                    format!(
                        "catalog/store/subjects/{}/{}.json",
                        hex("omega.repository"),
                        hex(name)
                    ),
                    "{}\n",
                )
            })
            .collect();
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, body)| (path.as_str(), *body))
            .collect();
        commit(&dir, OLD, "Catalog the repositories", &files);
        push(&dir, &["main"]);
        dir
    }

    /// Another clone of `name`'s origin, for commits the checkout has not fetched yet.
    fn other(&self, name: &str) -> PathBuf {
        let dir = self.root.join("others").join(name);
        git(
            &self.root,
            OLD,
            &["clone", "--quiet", text(&self.origin(name)), text(&dir)],
        );
        dir
    }

    /// The commit `reference` names in `name`'s origin.
    fn sha(&self, name: &str, reference: &str) -> String {
        git(&self.origin(name), OLD, &["rev-parse", reference])
            .trim()
            .to_owned()
    }
}

fn text(path: &Path) -> &str {
    path.to_str().expect("the case's path is UTF-8")
}

/// `git <args>` in `dir`, every date `date`, with no configuration but the case's own: the
/// user's global and system files are not read.
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
    for (path, body) in files {
        let path = dir.join(path);
        fs::create_dir_all(path.parent().expect("a file has a parent"))
            .expect("create the file's directory");
        fs::write(&path, body).expect("write the file");
    }
    git(dir, date, &["add", "--all"]);
    git(
        dir,
        date,
        &["commit", "--quiet", "--allow-empty", "-m", subject],
    );
}

fn push(dir: &Path, references: &[&str]) {
    git(
        dir,
        OLD,
        &[&["push", "--quiet", "origin"], references].concat(),
    );
}

fn hex(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02x}")).collect()
}

// ---------------------------------------------------------------------------------------------
// The sources and the snapshot
// ---------------------------------------------------------------------------------------------

fn fixture(name: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/github")
        .join(name);
    path.to_str()
        .expect("the fixture's path is UTF-8")
        .to_owned()
}

/// `cat` of the fixture `name`, whose `{repository}` and `{sha}` the collector fills.
fn cat(name: &str) -> Vec<String> {
    vec!["cat".to_owned(), fixture(name)]
}

fn now() -> OffsetDateTime {
    OffsetDateTime::parse(NOW, &Rfc3339).expect("NOW is RFC 3339")
}

/// The `repositories` sources over `world`'s workspace, every `gh` command line `cat` of a
/// fixture under `dir/`: the organization `organization.json`, no open issue and no release, and
/// GitHub's view of `main` in `<repository>.main.json`, `<repository>.commits.json` and
/// `<repository>.commit.<sha>.json`, which exist only for a repository with no checkout.
fn repository_sources(world: &World, dir: &str) -> repositories::Sources {
    repositories::Sources {
        organization: "example-org".to_owned(),
        workspace: world.workspace(),
        catalog: Some(Catalog {
            repository: "omega".to_owned(),
            path: format!("catalog/store/subjects/{}", hex("omega.repository")),
            names: CatalogNames::Hex,
        }),
        now: now(),
        repository_list: cat(&format!("{dir}/organization.json")),
        open_issues: cat("empty.json"),
        latest_release: cat("empty.json"),
        main_commit: cat(&format!("{dir}/{{repository}}.main.json")),
        main_commits: cat(&format!("{dir}/{{repository}}.commits.json")),
        commit: cat(&format!("{dir}/{{repository}}.commit.{{sha}}.json")),
        width: 8,
        bound: Duration::from_secs(60),
    }
}

/// The `github` sources, every `gh` command line `cat` of a fixture under `dir/`.
fn github_sources(dir: &str) -> github::Sources {
    github::Sources {
        organization: "example-org".to_owned(),
        repository_list: cat(&format!("{dir}/organization.json")),
        pull_requests: cat(&format!("{dir}/{{repository}}.pulls.json")),
        workflow_runs: cat(&format!("{dir}/{{repository}}.runs.json")),
        width: 8,
        bound: Duration::from_secs(60),
    }
}

/// One snapshot of `world` through the `repositories` collector over `sources`.
fn take_repositories(world: &World, sources: repositories::Sources) -> Taken {
    take(
        world,
        Collector::new("repositories", move |record| {
            repositories::collect_from(&sources, record)
        }),
    )
}

/// One snapshot of `world` through the `github` collector over `sources`, taken with the state
/// directory, under which the collector reads the newest complete snapshot's runs.
fn take_github(world: &World, sources: github::Sources) -> Taken {
    let collector = Collector::new("github", move |record| {
        github::collect_from(&sources, record)
    });
    snapshot::take_in(&world.state(), &[collector]).expect("the driver ends the snapshot")
}

fn take(world: &World, collector: Collector) -> Taken {
    let store = store::open(&world.state()).expect("open the store");
    snapshot::take(store, &[collector]).expect("the driver ends the snapshot")
}

fn complete(taken: &Taken) {
    assert_eq!(
        taken.state,
        SnapshotState::Complete,
        "the snapshot failed: {:?}",
        taken.reason
    );
}

fn generated(world: &World) -> Generated<store::Store> {
    Generated::new(store::open_existing(&world.state()).expect("open the store"))
}

/// The repository rows `taken` recorded, in the order they were recorded.
fn repository_rows(world: &World, taken: &Taken) -> Vec<Repositories> {
    generated(world)
        .repositories()
        .expect("the repositories view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .collect()
}

fn names(rows: &[Repositories]) -> Vec<&str> {
    rows.iter().map(|row| row.repository.0.as_str()).collect()
}

fn named<'a>(rows: &'a [Repositories], name: &str) -> &'a Repositories {
    rows.iter()
        .find(|row| row.repository.0 == name)
        .unwrap_or_else(|| panic!("no row for {name}: {rows:#?}"))
}

/// `(repository, real_commits_7d)` for every row.
fn real(rows: &[Repositories]) -> Vec<(&str, i64)> {
    rows.iter()
        .map(|row| (row.repository.0.as_str(), row.real_commits_7d))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// story:collect-repositories
// ---------------------------------------------------------------------------------------------

/// The story's acceptance fixture: `ci: repin the Pages facade …` in three repositories, a
/// `docs:` commit, a merge, and one `feat:` touching `src/`. The repository with the `feat:`
/// commit records `real_commits_7d` 1 and the others 0. The `feat:` commit sits on a branch and
/// on `main` through the merge, and counts once. The organization lists its repositories out of
/// order; they are recorded by name.
#[test]
fn real_activity_counts_the_feat_commit_and_nothing_the_story_excludes() {
    let world = World::new("activity");
    world.omega(&["alpha", "beta", "gamma"]);
    let alpha = world.repository("alpha");
    let beta = world.repository("beta");
    let gamma = world.repository("gamma");
    for dir in [&alpha, &beta, &gamma] {
        commit(
            dir,
            "2026-10-03T10:00:00Z",
            "ci: repin the Pages facade to 3.1.0",
            &[(".github/workflows/pages.yml", "uses: pages@3.1.0\n")],
        );
    }
    git(
        &alpha,
        OLD,
        &["switch", "--quiet", "--create", "board-header"],
    );
    commit(
        &alpha,
        "2026-10-04T10:00:00Z",
        "feat: read the board header",
        &[("src/board.rs", "pub fn header() {}\n")],
    );
    git(&alpha, OLD, &["switch", "--quiet", "main"]);
    git(
        &alpha,
        "2026-10-05T10:00:00Z",
        &["merge", "--quiet", "--no-ff", "--no-edit", "board-header"],
    );
    push(&alpha, &["main", "board-header"]);
    commit(
        &beta,
        "2026-10-04T11:00:00Z",
        "docs: explain the board header",
        &[("docs/board.md", "# The board header\n")],
    );
    push(&beta, &["main"]);
    push(&gamma, &["main"]);

    let taken = take_repositories(&world, repository_sources(&world, "activity"));
    complete(&taken);
    let rows = repository_rows(&world, &taken);
    assert_eq!(names(&rows), ["alpha", "beta", "gamma", "omega"]);
    assert_eq!(
        real(&rows),
        [("alpha", 1), ("beta", 0), ("gamma", 0), ("omega", 0)]
    );
    let main: Vec<(&str, i64)> = rows
        .iter()
        .map(|row| (row.repository.0.as_str(), row.main_commits_7d))
        .collect();
    assert_eq!(
        main,
        [("alpha", 3), ("beta", 2), ("gamma", 1), ("omega", 0)],
        "every commit on main in the window, merges included"
    );
}

/// Each rule of design § 10 removes its own commits and no other: a subject in three
/// repositories is a sweep (in two it is not), whatever its case, spacing, numbers or pull-request
/// reference; a maintenance subject is not real whatever it touches; a commit touching only docs,
/// CI, planning, markdown or lockfile paths is not real, and one touching any other path is; a
/// commit before the window is not counted.
#[test]
fn each_exclusion_rule_removes_only_its_commits() {
    let world = World::new("exclusions");
    world.omega(&["alpha"]);
    let alpha = world.repository("alpha");
    let beta = world.repository("beta");
    let gamma = world.repository("gamma");
    commit(
        &gamma,
        "2026-09-25T10:00:00Z",
        "feat: build the old board",
        &[("src/old.rs", "")],
    );
    let sweeps: [(&Path, [&str; 2]); 3] = [
        (
            &alpha,
            [
                "Adopt the shared error type",
                "Raise the toolchain to 1.99 (#12)",
            ],
        ),
        (
            &beta,
            [
                "adopt the shared error type",
                "Raise the toolchain to 1.98 (#40)",
            ],
        ),
        (
            &gamma,
            [
                "Adopt the  shared error type",
                "raise the toolchain to 1.97",
            ],
        ),
    ];
    for (dir, [error, toolchain]) in sweeps {
        commit(
            dir,
            "2026-10-02T10:00:00Z",
            error,
            &[("src/error.rs", "pub struct Error;\n")],
        );
        commit(
            dir,
            "2026-10-02T11:00:00Z",
            toolchain,
            &[("rust-toolchain.toml", "[toolchain]\n")],
        );
    }
    commit(
        &alpha,
        "2026-10-03T10:00:00Z",
        "Adopt the shared logger",
        &[("src/log.rs", "")],
    );
    commit(
        &beta,
        "2026-10-03T10:00:00Z",
        "adopt the shared logger",
        &[("src/log.rs", "")],
    );
    let maintenance = [
        ("Publish the site", "site/index.html"),
        ("Bump eventlog to 0.4.0", "Cargo.toml"),
        ("Retrofit the order specification", "spec/order.yaml"),
        ("chore(release): 0.4.0", "Cargo.toml"),
        ("style: format the board", "src/board.rs"),
        ("fix: lint the board", "src/board.rs"),
        ("Pin the facade to 3.1.0", "pages.toml"),
    ];
    let only_excluded_paths: [(&str, &[&str]); 4] = [
        ("fix: refresh the lockfile", &["Cargo.lock"]),
        ("feat: describe the API", &["README.md", "docs/api.txt"]),
        (
            "fix: plan the next wave",
            &[".engineering/planning/wave.yaml"],
        ),
        ("fix: adjust the CI cache", &[".github/workflows/ci.yml"]),
    ];
    let mut hour = 0;
    let mut at = || {
        hour += 1;
        format!("2026-10-04T{hour:02}:00:00Z")
    };
    for (index, (subject, path)) in maintenance.iter().enumerate() {
        commit(
            &gamma,
            &at(),
            subject,
            &[(path, &format!("change {index}\n"))],
        );
    }
    for (index, (subject, paths)) in only_excluded_paths.iter().enumerate() {
        let files: Vec<(&str, String)> = paths
            .iter()
            .map(|path| (*path, format!("change {index}\n")))
            .collect();
        let files: Vec<(&str, &str)> = files
            .iter()
            .map(|(path, body)| (*path, body.as_str()))
            .collect();
        commit(&gamma, &at(), subject, &files);
    }
    commit(
        &gamma,
        &at(),
        "fix: handle an empty board",
        &[
            ("src/board.rs", "pub fn empty() {}\n"),
            ("Cargo.lock", "# 2\n"),
        ],
    );
    for dir in [&alpha, &beta, &gamma] {
        push(dir, &["main"]);
    }

    let taken = take_repositories(&world, repository_sources(&world, "activity"));
    complete(&taken);
    let rows = repository_rows(&world, &taken);
    assert_eq!(
        real(&rows),
        [("alpha", 1), ("beta", 1), ("gamma", 1), ("omega", 0)],
        "alpha and beta: the shared logger (two repositories); gamma: the empty board"
    );
}

/// The story's missing-checkout acceptance: a repository the organization lists and the
/// workspace does not hold is recorded with `local_checkout` false and `behind_main`,
/// `dirty_files` and `worktrees` 0. Its `main` comes from GitHub: the head, its date, the commits
/// in the window (both pages), and its real commits, whose files GitHub is asked for only when
/// the commit is neither a merge nor a maintenance subject.
#[test]
fn a_repository_without_a_checkout_is_read_from_github_with_local_fields_at_zero() {
    let world = World::new("missing");
    world.omega(&["alpha", "delta"]);
    world.repository("alpha");

    let taken = take_repositories(&world, repository_sources(&world, "missing"));
    complete(&taken);
    let rows = repository_rows(&world, &taken);
    assert_eq!(names(&rows), ["alpha", "delta", "omega"]);
    let delta = named(&rows, "delta");
    assert!(!delta.local_checkout, "{delta:#?}");
    assert_eq!(
        (delta.behind_main, delta.dirty_files, delta.worktrees),
        (0, 0, 0),
        "{delta:#?}"
    );
    assert_eq!(
        delta.main_head.0,
        "d300000000000000000000000000000000000003"
    );
    assert_eq!(delta.main_committed_at.0, "2026-10-06T09:30:00Z");
    assert_eq!(delta.main_commits_7d, 4, "both pages, the merge included");
    assert_eq!(
        delta.real_commits_7d, 2,
        "the delta reader and the delta order"
    );
    assert_eq!(delta.visibility, Visibility::Public);
    assert_eq!(
        (
            &delta.latest_release,
            &delta.unreleased_commits,
            &delta.planning_store_version
        ),
        (&None, &None, &None)
    );
    assert_eq!(delta.in_catalog, Some(true), "{delta:#?}");
    assert!(named(&rows, "alpha").local_checkout);
}

/// The story's open-issue acceptance: against a fixture of 4 open issues, the oldest 4 days old,
/// the observation reads 4 and that creation time. A repository with issues turned off has none,
/// and its issues are not asked for (it has no fixture, so asking would fail the snapshot).
#[test]
fn open_issues_are_counted_with_the_oldest_creation_time() {
    let world = World::new("issues");
    world.omega(&["alpha"]);
    world.repository("alpha");
    world.repository("beta");
    let mut sources = repository_sources(&world, "issues");
    sources.open_issues = cat("issues/{repository}.json");

    let taken = take_repositories(&world, sources);
    complete(&taken);
    let rows = repository_rows(&world, &taken);
    let issues: Vec<(&str, i64, Option<&str>)> = rows
        .iter()
        .map(|row| {
            (
                row.repository.0.as_str(),
                row.open_issues,
                row.oldest_open_issue_at.as_ref().map(|at| at.0.as_str()),
            )
        })
        .collect();
    assert_eq!(
        issues,
        [
            ("alpha", 4, Some("2026-10-03T11:00:00Z")),
            ("beta", 0, None),
            ("omega", 0, None),
        ]
    );
}

/// A checkout's own state after `git fetch`: how far `HEAD` is behind `origin/main`, the lines
/// `git status --porcelain` prints, the entries `git worktree list` prints; the latest GitHub
/// Release with its publication date and the commits since its tag, or, with no release, the
/// newest tag on `main` with its commit's date; the planning store's version on `origin/main`
/// (YAML or JSON); whether the Omega catalog names it; its visibility and whether it is archived.
#[test]
fn a_checkout_reports_its_local_state_release_and_planning_store() {
    let world = World::new("local");
    world.omega(&["alpha"]);

    let alpha = world.repository("alpha");
    commit(
        &alpha,
        "2026-10-01T10:00:00Z",
        "Adopt the planning store",
        &[(
            ".engineering/project.yaml",
            "version: aep.project/5\n\n# A comment.\nprotocol: adp/1\n",
        )],
    );
    push(&alpha, &["main"]);
    let other = world.other("alpha");
    commit(
        &other,
        "2026-10-02T10:00:00Z",
        "feat: count the cards",
        &[("src/cards.rs", "")],
    );
    git(&other, "2026-10-02T10:00:00Z", &["tag", "v0.2.0"]);
    commit(
        &other,
        "2026-10-03T10:00:00Z",
        "fix: count no cards",
        &[("src/cards.rs", "pub fn none() {}\n")],
    );
    commit(
        &other,
        "2026-10-04T10:00:00Z",
        "feat: sort the cards",
        &[("src/sort.rs", "")],
    );
    push(&other, &["main", "v0.2.0"]);
    let trees = world.root.join("trees");
    for tree in ["alpha-1", "alpha-2"] {
        git(
            &alpha,
            OLD,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                tree,
                text(&trees.join(tree)),
            ],
        );
    }
    fs::write(alpha.join("README.md"), "# changed\n").expect("change a tracked file");
    fs::write(alpha.join("notes.txt"), "").expect("add an untracked file");
    fs::write(alpha.join("src/new.rs"), "").expect("add an untracked file");

    let beta = world.repository("beta");
    commit(
        &beta,
        "2026-09-20T10:00:00Z",
        "Adopt the planning store",
        &[(
            ".engineering/project.yaml",
            "{\"version\": \"aep.project/4\", \"protocol\": \"adp/1\"}\n",
        )],
    );
    git(
        &beta,
        "2026-09-21T10:00:00Z",
        &["tag", "-a", "0.2.0", "-m", "0.2.0"],
    );
    commit(
        &beta,
        "2026-09-28T10:00:00Z",
        "feat: read the beta feed",
        &[("src/feed.rs", "")],
    );
    git(
        &beta,
        "2026-09-29T10:00:00Z",
        &["tag", "-a", "0.3.0", "-m", "0.3.0"],
    );
    commit(
        &beta,
        "2026-10-02T10:00:00Z",
        "fix: keep the feed order",
        &[("src/feed.rs", "pub fn order() {}\n")],
    );
    push(&beta, &["main", "0.2.0", "0.3.0"]);

    world.repository("gamma");

    let mut sources = repository_sources(&world, "local");
    sources.latest_release = cat("local/{repository}.release.json");
    let taken = take_repositories(&world, sources);
    complete(&taken);
    let rows = repository_rows(&world, &taken);
    let seen: Vec<Seen> = rows.iter().map(Seen::from).collect();
    assert_eq!(
        seen,
        [
            Seen {
                repository: "alpha",
                visibility: Visibility::Private,
                archived: false,
                local_checkout: true,
                main_head: world.sha("alpha", "main"),
                main_committed_at: "2026-10-04T10:00:00Z",
                main_commits_7d: 4,
                real_commits_7d: 3,
                behind_main: 3,
                dirty_files: 3,
                worktrees: 3,
                latest_release: Some("v0.2.0"),
                latest_release_at: Some("2026-10-05T08:00:00Z"),
                unreleased_commits: Some(2),
                planning_store_version: Some("aep.project/5"),
                in_catalog: Some(true),
            },
            Seen {
                repository: "beta",
                visibility: Visibility::Public,
                archived: false,
                local_checkout: true,
                main_head: world.sha("beta", "main"),
                main_committed_at: "2026-10-02T10:00:00Z",
                main_commits_7d: 1,
                real_commits_7d: 1,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 1,
                latest_release: Some("0.3.0"),
                latest_release_at: Some("2026-09-28T10:00:00Z"),
                unreleased_commits: Some(1),
                planning_store_version: Some("aep.project/4"),
                in_catalog: Some(false),
            },
            Seen {
                repository: "gamma",
                visibility: Visibility::Private,
                archived: true,
                local_checkout: true,
                main_head: world.sha("gamma", "main"),
                main_committed_at: OLD,
                main_commits_7d: 0,
                real_commits_7d: 0,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 1,
                latest_release: None,
                latest_release_at: None,
                unreleased_commits: None,
                planning_store_version: None,
                in_catalog: Some(false),
            },
            Seen {
                repository: "omega",
                visibility: Visibility::Public,
                archived: false,
                local_checkout: true,
                main_head: world.sha("omega", "main"),
                main_committed_at: OLD,
                main_commits_7d: 0,
                real_commits_7d: 0,
                behind_main: 0,
                dirty_files: 0,
                worktrees: 1,
                latest_release: Some("v9.9.9"),
                latest_release_at: Some("2026-09-15T08:00:00Z"),
                unreleased_commits: None,
                planning_store_version: None,
                in_catalog: Some(false),
            },
        ]
    );
}

/// A repository row without the identities the store assigns and the issue fields, which
/// [`open_issues_are_counted_with_the_oldest_creation_time`] reads.
#[derive(Debug, PartialEq, Eq)]
struct Seen<'a> {
    repository: &'a str,
    visibility: Visibility,
    archived: bool,
    local_checkout: bool,
    main_head: String,
    main_committed_at: &'a str,
    main_commits_7d: i64,
    real_commits_7d: i64,
    behind_main: i64,
    dirty_files: i64,
    worktrees: i64,
    latest_release: Option<&'a str>,
    latest_release_at: Option<&'a str>,
    unreleased_commits: Option<i64>,
    planning_store_version: Option<&'a str>,
    in_catalog: Option<bool>,
}

impl<'a> From<&'a Repositories> for Seen<'a> {
    fn from(row: &'a Repositories) -> Self {
        Self {
            repository: &row.repository.0,
            visibility: row.visibility,
            archived: row.archived,
            local_checkout: row.local_checkout,
            main_head: row.main_head.0.clone(),
            main_committed_at: &row.main_committed_at.0,
            main_commits_7d: row.main_commits_7d,
            real_commits_7d: row.real_commits_7d,
            behind_main: row.behind_main,
            dirty_files: row.dirty_files,
            worktrees: row.worktrees,
            latest_release: row.latest_release.as_deref(),
            latest_release_at: row.latest_release_at.as_ref().map(|at| at.0.as_str()),
            unreleased_commits: row.unreleased_commits,
            planning_store_version: row.planning_store_version.as_deref(),
            in_catalog: row.in_catalog,
        }
    }
}

/// `sh -c` around `cat` of the fixture `name`: each call first appends a line to
/// `<calls>/{repository}`, so a case can count how often the collector asked.
fn counted_cat(calls: &Path, name: &str) -> Vec<String> {
    vec![
        "sh".to_owned(),
        "-c".to_owned(),
        "echo called >> \"$0\"; exec cat \"$1\"".to_owned(),
        format!("{}/{{repository}}", text(calls)),
        fixture(name),
    ]
}

/// A source that exits non-zero is asked once more; failing again, the collector fails the
/// snapshot naming the repository and the command, and records nothing: no repository is
/// recorded with 0 open issues for an issue list that did not answer.
#[test]
fn a_source_that_fails_twice_fails_the_snapshot_naming_the_repository_and_command() {
    let world = World::new("failing");
    world.omega(&["alpha"]);
    world.repository("alpha");
    world.repository("beta");
    let calls = world.root.join("calls");
    fs::create_dir_all(&calls).expect("create the call log");
    let mut sources = repository_sources(&world, "failing");
    sources.open_issues = counted_cat(&calls, "failing/{repository}.issues.json");

    let taken = take_repositories(&world, sources);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.clone().unwrap_or_default();
    assert!(
        reason.starts_with("repositories: repository beta: `sh -c "),
        "the reason names the collector, the repository and the command: {reason}"
    );
    assert!(
        reason.contains("failing/beta.issues.json") && reason.contains("exited 1"),
        "{reason}"
    );
    let asked = fs::read_to_string(calls.join("beta")).expect("beta's issues were asked for");
    assert_eq!(asked.lines().count(), 2, "asked once, then once more");
    let rows = repository_rows(&world, &taken);
    assert!(
        rows.is_empty(),
        "nothing is recorded for a snapshot whose source did not answer: {rows:#?}"
    );
}

/// A source that exits non-zero once and answers the second time is not a failure.
#[test]
fn a_source_that_fails_once_is_asked_again() {
    let world = World::new("flaky");
    world.omega(&["alpha"]);
    world.repository("alpha");
    let marks = world.root.join("marks");
    fs::create_dir_all(&marks).expect("create the marks");
    let mut sources = repository_sources(&world, "pair");
    sources.open_issues = vec![
        "sh".to_owned(),
        "-c".to_owned(),
        "if [ -e \"$0\" ]; then exec cat \"$1\"; fi; : > \"$0\"; exit 1".to_owned(),
        format!("{}/{{repository}}", text(&marks)),
        fixture("issues/alpha.json"),
    ];

    let taken = take_repositories(&world, sources);
    complete(&taken);
    let rows = repository_rows(&world, &taken);
    let issues: Vec<(&str, i64)> = rows
        .iter()
        .map(|row| (row.repository.0.as_str(), row.open_issues))
        .collect();
    assert_eq!(issues, [("alpha", 4), ("omega", 4)]);
}

/// A checkout whose origin is gone: `git fetch` fails twice, and the snapshot fails naming the
/// repository and the command.
#[test]
fn a_fetch_that_fails_fails_the_snapshot_naming_the_repository() {
    let world = World::new("no-origin");
    world.omega(&["alpha"]);
    world.repository("alpha");
    fs::rename(world.origin("alpha"), world.root.join("origins/alpha.gone"))
        .expect("move alpha's origin away");

    let taken = take_repositories(&world, repository_sources(&world, "pair"));
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.clone().unwrap_or_default();
    assert!(
        reason.starts_with("repositories: repository alpha: `git fetch --quiet origin` exited "),
        "{reason}"
    );
    let rows = repository_rows(&world, &taken);
    assert!(rows.is_empty(), "{rows:#?}");
}

// ---------------------------------------------------------------------------------------------
// story:collect-prs-and-ci
// ---------------------------------------------------------------------------------------------

/// One row per open pull request and one per workflow's latest `main` run, over the
/// non-archived repositories only (the archived one has no fixture, so asking would fail). The
/// three counts the reference loop in `docs/analysis/check-prs-ci.md` prints over the same
/// fixtures are 4 open, 2 with a `FAILURE`, `TIMED_OUT` or `ACTION_REQUIRED` rollup entry, and 5
/// latest runs neither `success` nor `skipped`.
#[test]
fn pull_requests_and_latest_main_runs_match_the_reference_counts() {
    let world = World::new("pulls");
    let taken = take_github(&world, github_sources("pulls"));
    complete(&taken);
    let generated = generated(&world);

    let pulls: Vec<_> = generated
        .pull_requests()
        .expect("the pull-requests view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .map(|row| {
            (
                row.repository.0,
                row.number,
                row.title,
                row.draft,
                row.mergeability,
                row.failing_checks,
                row.opened_at.0,
                row.updated_at.0,
            )
        })
        .collect();
    let pull = |repository: &str,
                number,
                title: &str,
                draft,
                mergeability,
                failing_checks,
                opened_at: &str,
                updated_at: &str| {
        (
            repository.to_owned(),
            number,
            title.to_owned(),
            draft,
            mergeability,
            failing_checks,
            opened_at.to_owned(),
            updated_at.to_owned(),
        )
    };
    assert_eq!(
        pulls,
        [
            pull(
                "alpha",
                12,
                "fix: count the cards",
                false,
                Mergeability::Mergeable,
                2,
                "2026-10-01T09:00:00Z",
                "2026-10-05T10:00:00Z"
            ),
            pull(
                "alpha",
                15,
                "feat: draw the board",
                true,
                Mergeability::Conflicting,
                1,
                "2026-10-04T09:00:00Z",
                "2026-10-06T10:00:00Z"
            ),
            pull(
                "alpha",
                16,
                "chore: tidy the board",
                false,
                Mergeability::Unknown,
                0,
                "2026-10-06T09:00:00Z",
                "2026-10-06T11:00:00Z"
            ),
            pull(
                "beta",
                3,
                "feat: read the beta feed",
                false,
                Mergeability::Mergeable,
                0,
                "2026-09-30T09:00:00Z",
                "2026-10-02T09:00:00Z"
            ),
        ]
    );

    let runs: Vec<_> = generated
        .workflow_runs()
        .expect("the workflow-runs view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .map(|row| (row.repository.0, row.workflow, row.conclusion, row.run_at.0))
        .collect();
    let run = |repository: &str, workflow: &str, conclusion, run_at: &str| {
        (
            repository.to_owned(),
            workflow.to_owned(),
            conclusion,
            run_at.to_owned(),
        )
    };
    assert_eq!(
        runs,
        [
            run(
                "alpha",
                "CI",
                RunConclusion::Success,
                "2026-10-06T10:00:00Z"
            ),
            run(
                "alpha",
                "Docs",
                RunConclusion::Skipped,
                "2026-10-01T10:00:00Z"
            ),
            run(
                "alpha",
                "Nightly",
                RunConclusion::Failure,
                "2026-10-02T03:00:00Z"
            ),
            run(
                "alpha",
                "Pages",
                RunConclusion::Pending,
                "2026-10-06T12:00:00Z"
            ),
            run(
                "alpha",
                "Release",
                RunConclusion::Failure,
                "2026-10-04T10:00:00Z"
            ),
            run(
                "beta",
                "CI",
                RunConclusion::Cancelled,
                "2026-10-06T10:00:00Z"
            ),
            run(
                "beta",
                "Lint",
                RunConclusion::Neutral,
                "2026-10-05T10:00:00Z"
            ),
        ]
    );

    let open = pulls.len();
    let failing = pulls.iter().filter(|pull| pull.5 > 0).count();
    let red = runs
        .iter()
        .filter(|run| !matches!(run.2, RunConclusion::Success | RunConclusion::Skipped))
        .count();
    assert_eq!((open, failing, red), (4, 2, 5));
}

/// A run list that does not answer fails the snapshot naming the collector, the repository and
/// the command.
#[test]
fn a_github_source_that_fails_fails_the_snapshot_naming_it() {
    let world = World::new("pulls-failing");
    let mut sources = github_sources("pulls");
    sources.workflow_runs = cat("failing/{repository}.runs.json");

    let taken = take_github(&world, sources);
    assert_eq!(taken.state, SnapshotState::Failed, "{taken:?}");
    let reason = taken.reason.clone().unwrap_or_default();
    assert!(
        reason.starts_with("github: repository beta: `cat ")
            && reason.contains("failing/beta.runs.json` exited 1"),
        "{reason}"
    );
}

// ---------------------------------------------------------------------------------------------
// story:ci-runs-stale-answer
// ---------------------------------------------------------------------------------------------

/// `sh -c` around `cat` of one of the fixtures `answers`, whose `{repository}` the collector
/// fills: each call appends a line to `<calls>/{repository}` and prints the answer at its count,
/// the last one for every call past them. It stands in for a GitHub whose run list answers
/// differently from one call to the next.
fn answers(calls: &Path, answers: &[&str]) -> Vec<String> {
    let mut command = vec![
        "sh".to_owned(),
        "-c".to_owned(),
        "echo called >> \"$0\"; n=$(($(wc -l < \"$0\"))); [ \"$n\" -le \"$#\" ] || n=$#; \
         shift $((n - 1)); exec cat \"$1\""
            .to_owned(),
        format!("{}/{{repository}}", text(calls)),
    ];
    command.extend(answers.iter().map(|name| fixture(name)));
    command
}

/// A fresh call log for [`answers`] in `world`'s directory.
fn call_log(world: &World) -> PathBuf {
    let calls = world.root.join("calls");
    fs::create_dir_all(&calls).expect("create the call log");
    calls
}

/// How often `name`'s run list was asked, by the lines [`answers`] appended to its call log.
fn asked(calls: &Path, name: &str) -> usize {
    fs::read_to_string(calls.join(name)).map_or(0, |log| log.lines().count())
}

/// One run row: repository, workflow, conclusion and when it started.
type RunRow = (String, String, RunConclusion, String);

fn row(repository: &str, workflow: &str, conclusion: RunConclusion, run_at: &str) -> RunRow {
    (
        repository.to_owned(),
        workflow.to_owned(),
        conclusion,
        run_at.to_owned(),
    )
}

/// Every run `taken` recorded, in the order it was recorded.
fn runs_of(world: &World, taken: &Taken) -> Vec<RunRow> {
    generated(world)
        .workflow_runs()
        .expect("the workflow-runs view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .map(|row| (row.repository.0, row.workflow, row.conclusion, row.run_at.0))
        .collect()
}

/// How many open pull requests `taken` recorded.
fn pulls_of(world: &World, taken: &Taken) -> usize {
    generated(world)
        .pull_requests()
        .expect("the pull-requests view answers")
        .into_iter()
        .filter(|row| row.snapshot_id == taken.snapshot_id)
        .count()
}

/// The `github` sources over `behind/`: the organization holds alpha only, with its open pull
/// requests from `pulls/` and its run list answering current (`behind/alpha.now.json`): `Gate`
/// at 2026-10-07T07:06:31Z, `Publish` at 2026-10-06T11:06:32Z.
fn behind_sources() -> github::Sources {
    github::Sources {
        pull_requests: cat("pulls/{repository}.pulls.json"),
        workflow_runs: cat("behind/{repository}.now.json"),
        ..github_sources("behind")
    }
}

/// The snapshot the next one is compared with: `behind_sources`, complete.
fn seed(world: &World) -> Taken {
    let taken = take_github(world, behind_sources());
    complete(&taken);
    taken
}

/// The story's first acceptance: each repository's run list is asked twice, and the newest run of
/// each workflow over both answers is recorded. alpha's first answer is stale, lacking the newest
/// `Gate` and `Publish` runs, and its second is current. Each of beta's answers holds the newest
/// run of one workflow only, so no single answer gives what is recorded.
#[test]
fn a_stale_first_answer_is_merged_with_the_second_and_the_newest_runs_are_recorded() {
    let world = World::new("stale");
    let calls = call_log(&world);
    let mut sources = github_sources("stale");
    sources.pull_requests = cat("empty.json");
    sources.workflow_runs = answers(
        &calls,
        &[
            "stale/{repository}.first.json",
            "stale/{repository}.second.json",
        ],
    );

    let taken = take_github(&world, sources);
    complete(&taken);
    assert_eq!(
        runs_of(&world, &taken),
        [
            row(
                "alpha",
                "Gate",
                RunConclusion::Success,
                "2026-10-07T07:06:31Z"
            ),
            row(
                "alpha",
                "Publish",
                RunConclusion::Failure,
                "2026-10-06T11:06:32Z"
            ),
            row("beta", "CI", RunConclusion::Success, "2026-10-06T10:00:00Z"),
            row(
                "beta",
                "Lint",
                RunConclusion::Success,
                "2026-10-05T10:00:00Z"
            ),
        ]
    );
    assert_eq!(
        (asked(&calls, "alpha"), asked(&calls, "beta")),
        (2, 2),
        "each run list is asked twice"
    );
}

/// Set in the child process [`four_answers_older_than_the_newest_complete_snapshot_are_recorded_with_a_warning`]
/// runs itself in.
const CHILD: &str = "CONDUCTOR_COLLECT_GITHUB_CHILD";

/// The story's second acceptance, as the coordinator changed it: a run GitHub no longer answers (a
/// deleted run, a recreated workflow) never fails a snapshot. The newest complete snapshot holds
/// alpha's `Gate` run of 2026-10-07T07:06:31Z, and four answers in a row hold none newer than
/// 2026-10-05T04:28:24Z: the run list is asked twice, then two more times, what the four answers
/// give is recorded, alpha's open pull requests too, and the snapshot completes with one line on
/// standard error naming the repository, the workflow, both instants and the snapshot compared
/// with. `Publish`, answered at the instant stored, is not behind.
///
/// The collector writes that line on the process's standard error, which the test harness
/// captures where no case can read it. So the case runs itself again in a child process with
/// `--nocapture` ([`behind_in_four_answers`]) and reads the child's standard error; the child
/// prints the seed snapshot's id on its standard output.
#[test]
fn four_answers_older_than_the_newest_complete_snapshot_are_recorded_with_a_warning() {
    const NAME: &str =
        "four_answers_older_than_the_newest_complete_snapshot_are_recorded_with_a_warning";
    if std::env::var_os(CHILD).is_some() {
        behind_in_four_answers();
        return;
    }
    let output = Command::new(std::env::current_exe().expect("the test binary's path"))
        .args([NAME, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD, "1")
        .stdin(Stdio::null())
        .output()
        .expect("the test binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "the child case failed:\n{stdout}\n{stderr}"
    );
    let seed = stdout
        .lines()
        .find_map(|line| line.strip_prefix("seed="))
        .unwrap_or_else(|| panic!("the child names the seed snapshot:\n{stdout}"));
    assert_eq!(
        stderr,
        format!(
            "repository alpha: workflow \"Gate\": GitHub's run list answered \
             2026-10-05T04:28:24Z in 4 answers, older than 2026-10-07T07:06:31Z in snapshot \
             {seed}; recorded what it answered\n"
        ),
        "one line on standard error, for Gate only"
    );
}

/// What [`four_answers_older_than_the_newest_complete_snapshot_are_recorded_with_a_warning`] runs
/// in its child process.
fn behind_in_four_answers() {
    let world = World::new("behind");
    let seeded = seed(&world);
    println!("\nseed={}", seeded.snapshot_id.0.0);
    let calls = call_log(&world);
    let mut sources = behind_sources();
    sources.workflow_runs = answers(&calls, &["behind/{repository}.then.json"]);

    let taken = take_github(&world, sources);
    complete(&taken);
    assert_eq!(
        asked(&calls, "alpha"),
        4,
        "asked twice, then two more times"
    );
    assert_eq!(
        runs_of(&world, &taken),
        [
            row(
                "alpha",
                "Gate",
                RunConclusion::Failure,
                "2026-10-05T04:28:24Z"
            ),
            row(
                "alpha",
                "Publish",
                RunConclusion::Failure,
                "2026-10-06T11:06:32Z"
            ),
        ],
        "what the four answers give"
    );
    assert_eq!(
        pulls_of(&world, &taken),
        3,
        "alpha's open pull requests are recorded"
    );
}

/// A run list behind the newest complete snapshot in its first two answers and caught up in its
/// third is recorded from the third, and is not asked a fourth time.
#[test]
fn a_run_list_behind_the_newest_complete_snapshot_is_asked_again_until_it_catches_up() {
    let world = World::new("behind-caught-up");
    seed(&world);
    let calls = call_log(&world);
    let mut sources = behind_sources();
    sources.workflow_runs = answers(
        &calls,
        &[
            "behind/{repository}.then.json",
            "behind/{repository}.then.json",
            "behind/{repository}.now.json",
        ],
    );

    let taken = take_github(&world, sources);
    complete(&taken);
    assert_eq!(
        runs_of(&world, &taken),
        [
            row(
                "alpha",
                "Gate",
                RunConclusion::Success,
                "2026-10-07T07:06:31Z"
            ),
            row(
                "alpha",
                "Publish",
                RunConclusion::Failure,
                "2026-10-06T11:06:32Z"
            ),
        ]
    );
    assert_eq!(asked(&calls, "alpha"), 3, "caught up in the third answer");
}

/// Only a complete snapshot is compared with: a failed one that holds a later `Gate` run than
/// GitHub answers does not fail the next snapshot, whose run list is asked twice only.
#[test]
fn a_failed_snapshot_is_not_compared_with() {
    let world = World::new("behind-failed");
    seed(&world);
    let later = Collector::new("github", |record| {
        let snapshot_id = record.snapshot().clone();
        record.workflow_run(RecordWorkflowRun {
            snapshot_id,
            repository: RepositoryName("alpha".to_owned()),
            workflow: "Gate".to_owned(),
            conclusion: RunConclusion::Success,
            run_at: Timestamp("2026-10-07T11:00:00Z".to_owned()),
        })?;
        anyhow::bail!("fail after recording a later run")
    });
    let failed = snapshot::take_in(&world.state(), &[later]).expect("the driver ends the snapshot");
    assert_eq!(failed.state, SnapshotState::Failed, "{failed:?}");

    let calls = call_log(&world);
    let mut sources = behind_sources();
    sources.workflow_runs = answers(&calls, &["behind/{repository}.now.json"]);
    let taken = take_github(&world, sources);
    complete(&taken);
    assert_eq!(
        runs_of(&world, &taken),
        [
            row(
                "alpha",
                "Gate",
                RunConclusion::Success,
                "2026-10-07T07:06:31Z"
            ),
            row(
                "alpha",
                "Publish",
                RunConclusion::Failure,
                "2026-10-06T11:06:32Z"
            ),
        ]
    );
    assert_eq!(asked(&calls, "alpha"), 2);
}
