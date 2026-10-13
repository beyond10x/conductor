//! `story:gitlab-repositories` and `story:gitlab-merge-requests`: a snapshot of an instance with a
//! `gitlab` source records one repository row and one specification row per project of the group
//! and its subgroups, and for each project that is not archived its open and recently merged merge
//! requests, its recent releases and its newest default-branch pipeline, read with
//! `glab api --hostname <host>`.
//!
//! Each case runs the built binary's `snapshot start-snapshot` with `CONDUCTOR_CONFIG` naming a
//! config file in the case's directory: one instance, `acme`, whose one source is the group
//! `example-org` on `gitlab.example.com`. `HOME` names the case's `home/`, and `PATH` only its
//! `bin/`: a fake `glab` that logs its words and answers with the file under the case's
//! `answers/` (a copy of `tests/fixtures/gitlab/`, which a case may change), chosen by the API
//! path it is asked; `aep` an empty workspace; `claude` no session; `ess` a valid specification;
//! and the real `git` and `tar`. No `gh` is on `PATH`, so a case that asked it would fail.
//!
//! The checkouts root holds `alpha`, a git repository whose `origin` is
//! `git@gitlab.example.com:example-org/alpha.git` and whose `HEAD` holds `spec/`; the project
//! `beta` has no checkout. Every SHA is a short neutral word (`abc123`). No case reaches the
//! network, a real checkout or the real session list.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{Value, json};

/// The clock of the snapshots: the repositories' window opens at 2026-09-30T12:00:00Z, and what
/// shipped is read from 2026-09-23T12:00:00Z.
const NOW: &str = "2026-10-07T12:00:00Z";

/// The date of every commit the cases make.
const OLD: &str = "2026-09-01T09:00:00Z";

/// The answers the fake `glab` gives, copied into each case.
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/gitlab");

/// The forge the instance's group is on.
const HOST: &str = "gitlab.example.com";

/// The fake `glab`: it logs its words, then answers with the file under `answers/` the API
/// path it is asked (its last word) names. `{log}` and `{answers}` are filled in.
const GLAB: &str = r#"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
for last in "$@"; do :; done
id=${last#projects/}
id=${id%%/*}
case "$last" in
  groups/*/projects\?*) file=projects.json ;;
  projects/*/repository/commits/*/diff*)
    sha=${last#*/repository/commits/}
    file=${sha%%/*}.diff.json ;;
  projects/*/repository/commits\?*) file=$id.commits.json ;;
  projects/*/repository/commits/*) file=$id.commit.json ;;
  projects/*/repository/compare\?*) file=$id.compare.json ;;
  projects/*/releases\?*) file=$id.releases.json ;;
  projects/*/issues\?*) file=$id.issues.json ;;
  projects/*/merge_requests\?state=opened*) file=$id.opened.json ;;
  projects/*/merge_requests\?state=merged*) file=$id.merged.json ;;
  projects/*/merge_requests/*) file=$id.mr-${last##*/}.json ;;
  projects/*/pipelines\?*) file=$id.pipelines.json ;;
  *) printf 'no answer for %s\n' "$last" >&2; exit 1 ;;
esac
if [ -f '{answers}'/"$file" ]; then
  while IFS= read -r line || [ -n "$line" ]; do printf '%s\n' "$line"; done < '{answers}'/"$file"
  exit 0
fi
printf 'no answer file %s for %s\n' "$file" "$last" >&2
exit 1
"#;

// ---------------------------------------------------------------------------------------------
// The case
// ---------------------------------------------------------------------------------------------

/// One case's directory: `home/`, the checkouts root `root/`, the managed trees `trees/`,
/// `records/`, `state/`, `cache/`, `bin/` for the programs the binary finds, `answers/` for the
/// fake `glab`, and an empty `work/` the binary runs from.
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
            .join("collect_gitlab")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in [
            "home",
            "root",
            "trees",
            "records/.engineering",
            "work",
            "bin",
            "answers",
        ] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        fs::write(
            dir.join("records/.engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers: []\n",
        )
        .expect("write the records' workspace file");
        let dir = fs::canonicalize(&dir).expect("the case's directory exists");
        let case = Self { dir };
        for entry in fs::read_dir(FIXTURES).expect("read the fixtures") {
            let path = entry.expect("a fixture").path();
            let name = path.file_name().expect("a fixture's name");
            fs::copy(&path, case.answers().join(name)).expect("copy a fixture");
        }
        case.programs();
        case
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn root(&self) -> PathBuf {
        self.dir.join("root")
    }

    fn bin(&self) -> PathBuf {
        self.dir.join("bin")
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn answers(&self) -> PathBuf {
        self.dir.join("answers")
    }

    fn log(&self, program: &str) -> PathBuf {
        self.dir.join(format!("{program}.log"))
    }

    /// The program `name` on the binary's `PATH`, whose whole text is `script`.
    fn script(&self, name: &str, script: &str) {
        let file = self.bin().join(name);
        fs::write(&file, script).expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    /// The program `name` on the binary's `PATH`: it appends its arguments to its log, then runs
    /// `body`, shell built-ins only.
    fn program(&self, name: &str, body: &str) {
        self.script(
            name,
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{body}\n",
                text(&self.log(name))
            ),
        );
    }

    /// The fake `glab`, `aep` an empty workspace and listing, `claude` no session, `ess` a valid
    /// specification of one scenario, and the real `git` and `tar`.
    fn programs(&self) {
        self.script(
            "glab",
            &GLAB
                .replace("{log}", &text(&self.log("glab")))
                .replace("{answers}", &text(&self.answers())),
        );
        self.program(
            "aep",
            "case \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) printf '%s' \
             '{\"artifacts\": []}' ;;\nesac",
        );
        self.program("claude", "printf '%s' '[]'");
        self.program(
            "ess",
            "case \"$1\" in\n  specify) exit 0 ;;\n  *) echo '1 scenario(s) (0 authored), 0 \
             refusal(s), written to x' ;;\nesac",
        );
        for name in ["git", "tar"] {
            std::os::unix::fs::symlink(which(name), self.bin().join(name)).expect("link a program");
        }
    }

    /// The checkout `alpha` under the checkouts root: a git repository with one commit holding a
    /// specification under `spec/`, whose `origin` is the project `example-org/alpha` on
    /// [`HOST`]. It is never fetched.
    fn alpha(&self) -> PathBuf {
        let dir = self.root().join("alpha");
        fs::create_dir_all(dir.join("spec")).expect("create the checkout");
        git(&dir, &["init", "--quiet", "--initial-branch=main"]);
        fs::write(dir.join("README.md"), "# alpha\n").expect("write a file");
        fs::write(
            dir.join("spec/ess-inputs.yaml"),
            "format: ess-inputs/1\nrequires: ess/1\nspecification:\n  - system.yaml\n",
        )
        .expect("write the manifest");
        fs::write(dir.join("spec/system.yaml"), "format: ess/1\n").expect("write the system");
        git(&dir, &["add", "--all"]);
        git(&dir, &["commit", "--quiet", "-m", "Start alpha"]);
        git(
            &dir,
            &[
                "remote",
                "add",
                "origin",
                &format!("git@{HOST}:example-org/alpha.git"),
            ],
        );
        dir
    }

    /// The config file of one instance `acme` whose one source is the group `example-org` on
    /// [`HOST`], with the case's checkouts root, trees, records, state and cache.
    fn config(&self) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: [{{gitlab: \
             example-org, host: {HOST}}}]\n    checkouts:\n      root: {}\n      trees: {}\n    \
             records: {}\n    state: {}\n    cache: {}\n",
            quoted(&self.root()),
            quoted(&self.dir.join("trees")),
            quoted(&self.dir.join("records")),
            quoted(&self.state()),
            quoted(&self.dir.join("cache")),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
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

    /// `snapshot start-snapshot` at [`NOW`], which completes.
    fn snapshot(&self, config: &Path) {
        let output = self.conductor(
            config,
            &[
                "snapshot",
                "start-snapshot",
                "--started-at",
                NOW,
                "--disk-free-bytes",
                "100000000000",
            ],
        );
        assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    }

    /// The rows of the view `view`, without the identities the store assigns.
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
}

// ---------------------------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------------------------

fn text(path: &Path) -> String {
    path.to_str().expect("the case's path is UTF-8").to_owned()
}

/// `path` as a YAML scalar: a JSON string.
fn quoted(path: &Path) -> String {
    Value::from(text(path)).to_string()
}

fn describe(output: &Output) -> String {
    format!(
        "exit {:?}, stdout {:?}, stderr {:?}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The program `name` on this process's `PATH`.
fn which(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("{name} is on PATH"))
}

/// Runs git in `dir` at [`OLD`], with no configuration of the user's.
fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "Fixture")
        .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
        .env("GIT_COMMITTER_NAME", "Fixture")
        .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
        .env("GIT_AUTHOR_DATE", OLD)
        .env("GIT_COMMITTER_DATE", OLD)
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

// ---------------------------------------------------------------------------------------------
// story:gitlab-repositories
// ---------------------------------------------------------------------------------------------

/// Acceptance 1: one repository row per project, archived ones included, read with `glab api
/// --hostname gitlab.example.com`: `alpha` with 3 open issues, release `v1.2.0` of 2026-10-01, 2
/// commits after it and its checkout; `beta` archived, without one.
#[test]
fn a_snapshot_records_one_repository_row_per_project_of_the_group() {
    let case = Case::new("repositories");
    case.alpha();
    let config = case.config();
    case.snapshot(&config);

    let rows = case.rows(&config, "repositories");
    let names: Vec<&Value> = rows.iter().map(|row| &row["repository"]).collect();
    assert_eq!(names, [&json!("alpha"), &json!("beta")], "{rows:#?}");

    let alpha = &rows[0];
    assert_eq!(alpha["archived"], json!(false), "{alpha:#}");
    assert_eq!(alpha["visibility"], json!("Private"), "{alpha:#}");
    assert_eq!(alpha["open_issues"], json!(3), "{alpha:#}");
    assert_eq!(
        alpha["oldest_open_issue_at"],
        json!("2026-09-20T08:00:00Z"),
        "{alpha:#}"
    );
    assert_eq!(alpha["latest_release"], json!("v1.2.0"), "{alpha:#}");
    assert_eq!(
        alpha["latest_release_at"],
        json!("2026-10-01T10:00:00Z"),
        "{alpha:#}"
    );
    assert_eq!(alpha["unreleased_commits"], json!(2), "{alpha:#}");
    assert_eq!(alpha["local_checkout"], json!(true), "{alpha:#}");
    assert_eq!(alpha["main_head"], json!("abc123"), "{alpha:#}");
    assert_eq!(
        alpha["main_committed_at"],
        json!("2026-10-06T10:00:00Z"),
        "{alpha:#}"
    );
    assert_eq!(alpha["main_commits_7d"], json!(2), "{alpha:#}");
    assert_eq!(
        alpha["real_commits_7d"],
        json!(1),
        "the docs commit is maintenance: {alpha:#}"
    );
    assert_eq!(alpha["dirty_files"], json!(0), "{alpha:#}");

    let beta = &rows[1];
    assert_eq!(beta["archived"], json!(true), "{beta:#}");
    assert_eq!(beta["visibility"], json!("Public"), "{beta:#}");
    assert_eq!(beta["local_checkout"], json!(false), "{beta:#}");
    assert_eq!(beta["main_head"], json!("bcd234"), "{beta:#}");
    assert_eq!(beta["latest_release"], Value::Null, "{beta:#}");
    assert_eq!(beta["open_issues"], json!(0), "{beta:#}");

    let asked = case.lines("glab");
    let projects = format!(
        "api --hostname {HOST} groups/example-org/projects?include_subgroups=true&\
         with_shared=false&per_page=100&page=1"
    );
    assert!(asked.contains(&projects), "{asked:#?}");
    for line in &asked {
        assert!(
            line.starts_with(&format!("api --hostname {HOST} ")),
            "every read is `glab api --hostname {HOST}`: {line}"
        );
    }
}

/// Acceptance 2: one specification row per project: `alpha`'s checkout holds `spec/`, so it is
/// `Present` at `spec`; `beta` has no checkout, so it is `Missing`.
#[test]
fn a_snapshot_records_one_specification_row_per_project_of_the_group() {
    let case = Case::new("specifications");
    case.alpha();
    let config = case.config();
    case.snapshot(&config);

    let rows = case.rows(&config, "specifications");
    let read: Vec<(&Value, &Value, &Value)> = rows
        .iter()
        .map(|row| (&row["repository"], &row["presence"], &row["path"]))
        .collect();
    assert_eq!(
        read,
        [
            (&json!("alpha"), &json!("Present"), &json!("spec")),
            (&json!("beta"), &json!("Missing"), &Value::Null),
        ],
        "{rows:#?}"
    );
    assert_eq!(rows[0]["validation"], json!("Valid"), "{rows:#?}");
}

// ---------------------------------------------------------------------------------------------
// story:gitlab-merge-requests
// ---------------------------------------------------------------------------------------------

/// Acceptance 1: each open merge request of a project that is not archived is a pull request
/// row, failing when its head pipeline failed: `alpha`'s `!1` passes, its `!2` fails. The
/// archived `beta` is asked nothing (the fake `glab` holds no answer for it).
#[test]
fn an_open_merge_request_is_a_pull_request_row_failing_when_its_head_pipeline_failed() {
    let case = Case::new("merge-requests");
    case.alpha();
    let config = case.config();
    case.snapshot(&config);

    assert_eq!(
        case.rows(&config, "pull-requests"),
        [
            json!({
                "repository": "alpha",
                "number": 1,
                "title": "Read the board header",
                "draft": false,
                "mergeability": "Mergeable",
                "failing_checks": 0,
                "opened_at": "2026-10-05T09:00:00Z",
                "updated_at": "2026-10-06T09:00:00Z",
            }),
            json!({
                "repository": "alpha",
                "number": 2,
                "title": "Draw the board",
                "draft": true,
                "mergeability": "Conflicting",
                "failing_checks": 1,
                "opened_at": "2026-10-04T09:00:00Z",
                "updated_at": "2026-10-06T11:00:00Z",
            }),
        ]
    );
    let asked = case.lines("glab");
    for path in [
        "projects/101/merge_requests?state=opened&per_page=100&page=1",
        "projects/101/merge_requests/1",
        "projects/101/merge_requests/2",
    ] {
        let line = format!("api --hostname {HOST} {path}");
        assert!(asked.contains(&line), "{line}: {asked:#?}");
    }
    assert!(
        !asked
            .iter()
            .any(|line| line.contains("projects/102/merge_requests")),
        "the archived beta is not asked: {asked:#?}"
    );
}

/// Acceptance 2: a merge request merged within the collector's 14-day window is a merged pull
/// request row: `alpha`'s `!3`, merged 2026-10-02; `!4`, merged 2026-09-10, is not.
#[test]
fn a_merge_request_merged_within_the_window_is_a_merged_pull_request_row() {
    let case = Case::new("merged");
    case.alpha();
    let config = case.config();
    case.snapshot(&config);

    assert_eq!(
        case.rows(&config, "merged-pull-requests"),
        [json!({
            "repository": "alpha",
            "number": 3,
            "title": "Start the board",
            "merged_at": "2026-10-02T09:00:00Z",
            "url": "https://gitlab.example.com/example-org/alpha/-/merge_requests/3",
        })]
    );
    let asked = case.lines("glab");
    let line = format!(
        "api --hostname {HOST} projects/101/merge_requests?state=merged&\
         updated_after=2026-09-23T12%3A00%3A00Z&per_page=100&page=1"
    );
    assert!(asked.contains(&line), "{line}: {asked:#?}");
}

/// Acceptance 3: a release published within the window is a release row: `alpha`'s `v1.2.0` of
/// 2026-10-01; `v1.1.0`, of 2026-09-01, is not.
#[test]
fn a_release_published_within_the_window_is_a_release_row() {
    let case = Case::new("releases");
    case.alpha();
    let config = case.config();
    case.snapshot(&config);

    assert_eq!(
        case.rows(&config, "releases"),
        [json!({
            "repository": "alpha",
            "tag": "v1.2.0",
            "published_at": "2026-10-01T10:00:00Z",
            "url": "https://gitlab.example.com/example-org/alpha/-/releases/v1.2.0",
        })]
    );
}

/// Acceptance 4: the newest default-branch pipeline of each project that is not archived is a
/// workflow run, its status mapped to the GitHub collector's conclusions: `alpha`'s `success` on
/// `main` is `Success`, `gamma`'s `failed` on `trunk` is `Failure`.
#[test]
fn the_newest_default_branch_pipeline_is_a_workflow_run_with_its_status_mapped() {
    let case = Case::new("pipelines");
    case.alpha();
    fs::copy(
        case.answers().join("projects-with-gamma.json"),
        case.answers().join("projects.json"),
    )
    .expect("answer the group with gamma too");
    let config = case.config();
    case.snapshot(&config);

    assert_eq!(
        case.rows(&config, "workflow-runs"),
        [
            json!({
                "repository": "alpha",
                "workflow": "pipeline",
                "conclusion": "Success",
                "run_at": "2026-10-07T08:00:00Z",
            }),
            json!({
                "repository": "gamma",
                "workflow": "pipeline",
                "conclusion": "Failure",
                "run_at": "2026-10-07T09:00:00Z",
            }),
        ]
    );
    let asked = case.lines("glab");
    for path in [
        "projects/101/pipelines?ref=main&per_page=1",
        "projects/103/pipelines?ref=trunk&per_page=1",
    ] {
        let line = format!("api --hostname {HOST} {path}");
        assert!(asked.contains(&line), "{line}: {asked:#?}");
    }
}
