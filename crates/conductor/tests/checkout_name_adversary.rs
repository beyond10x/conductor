//! Adversary cases for `story:checkout-name-differs-from-project`: what the rule "a GitLab
//! checkout is the repository under its directory name" does to configurations and checkouts
//! roots the unit's own cases do not hold. The harness is the one of `collect_gitlab.rs`: the
//! built binary, a fake `glab` answering from the case's `answers/`, an isolated `HOME`, and no
//! network.

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
  projects/*/repository/tags\?*) file=$id.tags.json ;;
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

/// One case's directory, as in `collect_gitlab.rs`.
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
            .join("checkout_name_adversary")
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
        case.glab(GLAB);
        case.program(
            "aep",
            "case \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) printf '%s' \
             '{\"artifacts\": []}' ;;\nesac",
        );
        case.program("claude", "printf '%s' '[]'");
        case.program(
            "ess",
            "case \"$1\" in\n  specify) exit 0 ;;\n  *) echo '1 scenario(s) (0 authored), 0 \
             refusal(s), written to x' ;;\nesac",
        );
        for name in ["git", "tar"] {
            std::os::unix::fs::symlink(which(name), case.bin().join(name)).expect("link a program");
        }
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

    fn answers(&self) -> PathBuf {
        self.dir.join("answers")
    }

    fn log(&self, program: &str) -> PathBuf {
        self.dir.join(format!("{program}.log"))
    }

    fn script(&self, name: &str, script: &str) {
        let file = self.bin().join(name);
        fs::write(&file, script).expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    fn program(&self, name: &str, body: &str) {
        self.script(
            name,
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{body}\n",
                text(&self.log(name))
            ),
        );
    }

    /// The fake `glab` whose text is `script`, its log and answers filled in.
    fn glab(&self, script: &str) {
        self.script(
            "glab",
            &script
                .replace("{log}", &text(&self.log("glab")))
                .replace("{answers}", &text(&self.answers())),
        );
    }

    /// The checkout `name` under the checkouts root: a git repository with one commit, whose
    /// `origin` is `origin`.
    fn checkout(&self, name: &str, origin: &str) -> PathBuf {
        let dir = self.root().join(name);
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
        git(&dir, &["remote", "add", "origin", origin]);
        dir
    }

    /// The project list `file`: each `(id, full path)` a private project on `main`.
    fn projects(&self, file: &str, projects: &[(u64, &str)]) {
        let listed: Vec<Value> = projects
            .iter()
            .map(|(id, path)| {
                json!({
                    "id": id,
                    "path_with_namespace": path,
                    "archived": false,
                    "visibility": "private",
                    "default_branch": "main",
                    "open_issues_count": 0,
                    "empty_repo": false,
                    "web_url": format!("https://{HOST}/{path}"),
                })
            })
            .collect();
        fs::write(self.answers().join(file), Value::from(listed).to_string())
            .expect("write the project list");
    }

    /// The config file of one instance `acme` with the sources `sources`, the entries of a YAML
    /// flow sequence.
    fn config_of(&self, sources: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: [{sources}]\n    \
             checkouts:\n      root: {}\n      trees: {}\n    \
             records: {}\n    state: {}\n    cache: {}\n",
            quoted(&self.root()),
            quoted(&self.dir.join("trees")),
            quoted(&self.dir.join("records")),
            quoted(&self.dir.join("state")),
            quoted(&self.dir.join("cache")),
        );
        let file = self.dir.join("conductor.yaml");
        fs::write(&file, text).expect("write the config file");
        file
    }

    /// The config file whose one source is the group `example-org` on [`HOST`].
    fn config(&self) -> PathBuf {
        self.config_of(&format!("{{gitlab: example-org, host: {HOST}}}"))
    }

    fn conductor(&self, config: &Path, args: &[&str]) -> Output {
        common::conductor(self.home())
            .current_dir(self.dir.join("work"))
            .env_clear()
            .env("HOME", self.home())
            .env("PATH", self.bin())
            .env("CONDUCTOR_CONFIG", config)
            .arg("--state-dir")
            .arg(self.dir.join("state"))
            .args(args)
            .stdin(Stdio::null())
            .output()
            .expect("the conductor binary runs")
    }

    /// `snapshot start-snapshot` at [`NOW`], whatever it exits with.
    fn snapshot(&self, config: &Path) -> Output {
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

    /// The rows of the view `repositories`.
    fn rows(&self, config: &Path) -> Vec<Value> {
        let output = self.conductor(config, &["snapshot", "repositories", "--format", "jsonl"]);
        assert!(output.status.success(), "{}", describe(&output));
        String::from_utf8(output.stdout)
            .expect("UTF-8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("one JSON object a line"))
            .collect()
    }

    /// Each repository row's name and whether it has a checkout.
    fn repositories(&self, config: &Path) -> Vec<(String, bool)> {
        self.rows(config)
            .iter()
            .map(|row| {
                (
                    row["repository"].as_str().expect("a name").to_owned(),
                    row["local_checkout"].as_bool().expect("a flag"),
                )
            })
            .collect()
    }
}

fn text(path: &Path) -> String {
    path.to_str().expect("the case's path is UTF-8").to_owned()
}

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

fn which(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("{name} is on PATH"))
}

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

fn ssh(project: &str) -> String {
    format!("git@{HOST}:{project}.git")
}

// ---------------------------------------------------------------------------------------------
// Configurations that recorded a snapshot before the rule
// ---------------------------------------------------------------------------------------------

/// Two `gitlab` sources whose groups nest, `example-org` and `example-org/platform`, list the
/// project `example-org/platform/tool` both; with its checkout `tool` under the checkouts root
/// the base recorded a snapshot (rows `platform/tool` and `tool`). Under the rule both sources
/// name the one checkout `tool`: it is one checkout and one repository, so the snapshot
/// completes and records `tool` once.
#[test]
fn two_sources_whose_groups_nest_record_their_shared_checkout_once() {
    let case = Case::new("nested-groups");
    case.checkout("tool", &ssh("example-org/platform/tool"));
    case.projects("projects.json", &[(101, "example-org/platform/tool")]);
    case.projects(
        "platform.projects.json",
        &[(101, "example-org/platform/tool")],
    );
    case.glab(&GLAB.replace(
        "  groups/*/projects\\?*) file=projects.json ;;",
        "  groups/*platform/projects\\?*) file=platform.projects.json ;;\n  \
         groups/*/projects\\?*) file=projects.json ;;",
    ));
    let config = case.config_of(&format!(
        "{{gitlab: example-org, host: {HOST}}}, {{gitlab: example-org/platform, host: {HOST}}}"
    ));
    let output = case.snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(case.repositories(&config), [("tool".to_owned(), true)]);
}

/// A second clone of a project under a directory whose name is no repository name (`checks
/// copy`) was no repository of its own before the rule and stopped nothing. It must not stop
/// the snapshot of every other repository of the instance.
#[test]
fn a_clone_under_a_name_that_is_no_repository_name_does_not_stop_the_snapshot() {
    let case = Case::new("odd-directory");
    case.checkout("alpha", &ssh("example-org/alpha"));
    case.checkout("checks copy", &ssh("example-org/alpha"));
    case.projects("projects.json", &[(101, "example-org/alpha")]);
    let config = case.config();
    let output = case.snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert!(
        case.repositories(&config)
            .contains(&("alpha".to_owned(), true)),
        "alpha is recorded"
    );
}

/// `exclude: [example-checks]` left the project `example-checks` out before the rule. Its
/// checkout is `checks`, so the entry now names nothing: either the repository is still left
/// out, or the entry that no longer excludes anything is named by `config validate` or by the
/// snapshot.
#[test]
fn an_exclude_entry_naming_a_checked_out_project_by_its_path_excludes_it_or_is_reported() {
    let case = Case::new("exclude-by-project");
    case.checkout("checks", &ssh("example-org/example-checks"));
    case.projects("projects.json", &[(101, "example-org/example-checks")]);
    let config = case.config_of(&format!(
        "{{gitlab: example-org, host: {HOST}, exclude: [example-checks]}}"
    ));
    let validated = case.conductor(&config, &["config", "validate"]);
    let snapshot = case.snapshot(&config);
    let said = format!("{}\n{}", describe(&validated), describe(&snapshot));
    let listed = case.repositories(&config);
    assert!(
        listed.is_empty() || said.contains("example-checks"),
        "the excluded project is recorded as {listed:?} and nothing names the entry: {said}"
    );
}

// ---------------------------------------------------------------------------------------------
// Branches of `Forge::projects` no case of the unit holds
// ---------------------------------------------------------------------------------------------

/// A project the list answers twice (a page that shifted) is one project: its one checkout is
/// one repository, and no name is "held twice". Holds the `seen` set of `Forge::projects`.
#[test]
fn a_project_listed_twice_is_one_repository_under_its_checkout() {
    let case = Case::new("listed-twice");
    case.checkout("checks", &ssh("example-org/example-checks"));
    case.projects(
        "projects.json",
        &[
            (101, "example-org/example-checks"),
            (101, "example-org/example-checks"),
        ],
    );
    let config = case.config();
    let output = case.snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(case.repositories(&config), [("checks".to_owned(), true)]);
}

/// An `origin` that writes the group and the project in another case names the project all the
/// same: the checkout is its repository, and the project is not listed beside it under its own
/// name. Holds the case-blind comparison of `Forge::projects`.
#[test]
fn an_origin_written_in_another_case_is_its_projects_checkout() {
    let case = Case::new("origin-case");
    case.checkout("checks", &ssh("Example-Org/Example-Checks"));
    case.projects("projects.json", &[(101, "example-org/example-checks")]);
    let config = case.config();
    let output = case.snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(case.repositories(&config), [("checks".to_owned(), true)]);
}

/// Checkouts that are no project's: `ghost`, whose `origin` names a project of the group the
/// forge does not list, and `beside`, whose `origin` names a project outside the group, are no
/// repositories; a symlink `link` to the checkout `checks` is a second checkout of its project.
#[test]
fn a_checkout_no_listed_project_owns_is_no_repository_and_a_symlink_is_a_checkout() {
    let case = Case::new("strays");
    let checks = case.checkout("checks", &ssh("example-org/example-checks"));
    case.checkout("ghost", &ssh("example-org/ghost"));
    case.checkout("beside", &ssh("other-org/example-checks"));
    std::os::unix::fs::symlink(&checks, case.root().join("link")).expect("link the checkout");
    case.projects("projects.json", &[(101, "example-org/example-checks")]);
    let config = case.config();
    let output = case.snapshot(&config);
    assert_eq!(output.status.code(), Some(0), "{}", describe(&output));
    assert_eq!(
        case.repositories(&config),
        [("checks".to_owned(), true), ("link".to_owned(), true)]
    );
}
