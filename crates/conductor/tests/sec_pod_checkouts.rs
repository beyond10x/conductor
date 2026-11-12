//! Conformance checks of `story:pod-source-checkouts` against the listing a `gitlab` source's
//! host answers: a runtime alone in a pod clones only under its checkouts root, at the path the
//! collector reads, runs no code a cloned repository carries, and leaves no temporary directory
//! when a clone is stopped at the collector's budget.
//!
//! Each case runs the built binary with a cleared environment, as `tests/pod_checkouts.rs` does:
//! `HOME` the case's `home/`, `PATH` only its `bin/` (a fake `glab` answering from a copy of
//! `tests/fixtures/gitlab/`, and `git`, a wrapper that logs its words and runs the real git), and
//! the forge a directory of bare repositories under `forge/`, which git reaches through a rewrite
//! of `https://gitlab.example.com/` in its environment. No case reaches the network.

mod active;
mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use serde_json::{Value, json};

const NOW: &str = "2026-10-07T12:00:00Z";
const OLD: &str = "2026-09-01T09:00:00Z";
const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/gitlab");
const HOST: &str = "gitlab.example.com";
const LISTEN: &str = "CONDUCTOR_API_LISTEN";

/// The fake `glab` of `tests/pod_checkouts.rs`.
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

fn token() -> String {
    ["fixture", "-", "value", "-", "7Qx2", "Lm9", "Vb4", "Zr8"].concat()
}

/// One case's directory; a case that passed leaves nothing behind.
struct Case {
    dir: PathBuf,
}

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
            .join("sec_pod_checkouts")
            .join(name);
        if dir.exists() {
            fs::remove_dir_all(&dir).expect("clear the case's directory");
        }
        for sub in ["home", "root", "trees", "work", "bin", "answers", "forge"] {
            fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
        }
        fs::create_dir_all(dir.join("records/.engineering")).expect("create the records");
        fs::write(
            dir.join("records/.engineering/workspace.yaml"),
            "version: aep.workspace/1\nmembers: []\n",
        )
        .expect("write the workspace file");
        let dir = fs::canonicalize(&dir).expect("the case's directory exists");
        let case = Self { dir };
        for entry in fs::read_dir(FIXTURES).expect("read the fixtures") {
            let path = entry.expect("a fixture").path();
            let name = path.file_name().expect("a fixture's name");
            fs::copy(&path, case.dir.join("answers").join(name)).expect("copy a fixture");
        }
        case.script(
            "glab",
            &GLAB
                .replace("{log}", &text(&case.dir.join("glab.log")))
                .replace("{answers}", &text(&case.dir.join("answers"))),
        );
        for (name, body) in [
            (
                "aep",
                "#!/bin/sh\ncase \"$3\" in\n  members) printf '%s' '{\"members\": []}' ;;\n  *) \
                 printf '%s' '{\"artifacts\": []}' ;;\nesac\n",
            ),
            ("claude", "#!/bin/sh\nprintf '%s' '[]'\n"),
            ("gh", "#!/bin/sh\nprintf '%s\\n' '[]'\n"),
            (
                "ess",
                "#!/bin/sh\ncase \"$1\" in\n  specify) exit 0 ;;\n  *) echo '1 scenario(s) (0 \
                 authored), 0 refusal(s), written to x' ;;\nesac\n",
            ),
        ] {
            case.script(name, body);
        }
        case.git_wrapper("");
        case
    }

    fn root(&self) -> PathBuf {
        self.dir.join("root")
    }

    fn script(&self, name: &str, script: &str) {
        let file = self.dir.join("bin").join(name);
        fs::write(&file, script).expect("write the program");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o755)).expect("make it runnable");
    }

    fn git_wrapper(&self, before: &str) {
        self.script(
            "git",
            &format!(
                "#!/bin/sh\nprintf '%s\\n' \"$*\" >> '{}'\n{before}\nexec '{}' \"$@\"\n",
                text(&self.dir.join("git.log")),
                text(&which("git"))
            ),
        );
    }

    fn git_lines(&self) -> Vec<String> {
        fs::read_to_string(self.dir.join("git.log"))
            .map(|log| log.lines().map(str::to_owned).collect())
            .unwrap_or_default()
    }

    /// The project list of the fake `glab`: each `(id, path_with_namespace)` a private project
    /// on `main`, its other answers those of the project 101.
    fn projects(&self, projects: &[(u64, &str)]) {
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
        fs::write(
            self.dir.join("answers/projects.json"),
            Value::from(listed).to_string(),
        )
        .expect("write the project list");
    }

    /// The bare repository `forge/gitlab/<full>.git` holding, on `main`, the files `files`
    /// (`(path, text, executable)`) in one commit.
    fn bare(&self, full: &str, files: &[(&str, String, bool)]) -> PathBuf {
        let bare = self.dir.join("forge/gitlab").join(format!("{full}.git"));
        fs::create_dir_all(&bare).expect("create the bare repository");
        git(
            &bare,
            &["init", "--quiet", "--bare", "--initial-branch=main"],
        );
        let work = self.dir.join("pushing");
        let _ = fs::remove_dir_all(&work);
        fs::create_dir_all(&work).expect("create the work tree");
        git(&work, &["init", "--quiet", "--initial-branch=main"]);
        for (path, body, executable) in files {
            let file = work.join(path);
            fs::create_dir_all(file.parent().expect("a parent")).expect("create a directory");
            fs::write(&file, body).expect("write a file");
            if *executable {
                fs::set_permissions(&file, fs::Permissions::from_mode(0o755))
                    .expect("make it runnable");
            }
        }
        git(&work, &["add", "--all"]);
        git(&work, &["commit", "--quiet", "-m", "Start"]);
        let url = format!("file://{}", text(&bare));
        git(&work, &["push", "--quiet", &url, "HEAD:main"]);
        fs::remove_dir_all(&work).expect("remove the work tree");
        bare
    }

    fn config(&self, include: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: [{{gitlab: \
             example-org, host: {HOST}, include: [{include}]}}]\n    checkouts:\n      root: \
             {}\n      trees: {}\n    records: {}\n    state: {}\n    cache: {}\n",
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

    /// One collect pass in a pod, `snapshot start-snapshot` with `extra` arguments.
    fn pass(&self, config: &Path, extra: &[&str]) -> Output {
        let forge = self.dir.join("forge");
        let mut command = common::conductor(self.dir.join("home"));
        command
            .env_clear()
            .env("HOME", self.dir.join("home"))
            .env("PATH", self.dir.join("bin"))
            .env("CONDUCTOR_CONFIG", config)
            .env("GIT_AUTHOR_NAME", "Fixture")
            .env("GIT_AUTHOR_EMAIL", "fixture@example.invalid")
            .env("GIT_COMMITTER_NAME", "Fixture")
            .env("GIT_COMMITTER_EMAIL", "fixture@example.invalid")
            .env("GIT_CONFIG_COUNT", "1")
            .env(
                "GIT_CONFIG_KEY_0",
                format!("url.file://{}/gitlab/.insteadOf", text(&forge)),
            )
            .env("GIT_CONFIG_VALUE_0", format!("https://{HOST}/"))
            .env(LISTEN, "127.0.0.1:0")
            .env("GITLAB_TOKEN", token())
            .current_dir(self.dir.join("work"))
            .args([
                "snapshot",
                "start-snapshot",
                "--started-at",
                NOW,
                "--disk-free-bytes",
                "100000000000",
            ])
            .args(extra)
            .stdin(Stdio::null());
        command.output().expect("the conductor binary runs")
    }

    fn partials(&self) -> Vec<String> {
        fs::read_dir(self.root())
            .expect("read the checkouts root")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(conductor_cli::collect::checkouts::PARTIAL))
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
    let output = Command::new(which("git"))
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_CONFIG_COUNT")
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

/// Invariants 1 and 3: a project the host lists as `example-org/platform/.git`, which the entry
/// `platform` covers, is not cloned into `<root>/platform/.git`. Cloned there, its work tree is a
/// git directory of `<root>/platform` whose `config` the repository wrote, and the next pass's
/// `git status` in that checkout runs the repository's `core.fsmonitor` program.
#[test]
fn a_listed_path_step_named_dot_git_runs_no_code_from_the_repository() {
    let case = Case::new("dot-git-step");
    let marker = case.dir.join("fsmonitor-ran");
    case.projects(&[(101, "example-org/platform/.git")]);
    let config_text = format!(
        "[core]\n\trepositoryformatversion = 0\n\tbare = false\n\tfsmonitor = .git/hook\n\
         [remote \"origin\"]\n\turl = https://{HOST}/example-org/platform/.git.git\n"
    );
    case.bare(
        "example-org/platform/.git",
        &[
            ("HEAD", "ref: refs/heads/main\n".to_owned(), false),
            ("config", config_text, false),
            ("objects/keep", "x\n".to_owned(), false),
            ("refs/keep", "x\n".to_owned(), false),
            (
                "hook",
                format!("#!/bin/sh\n: > '{}'\nexit 1\n", text(&marker)),
                true,
            ),
        ],
    );
    let config = case.config("platform");

    let first = case.pass(&config, &[]);
    let nested = case.root().join("platform/.git/.git").exists();
    let second = case.pass(&config, &[]);
    assert!(
        !marker.exists(),
        "the repository's fsmonitor program ran in the pod; the clone made \
         platform/.git/.git: {nested}; clones: {:?}\nfirst pass: {}\nsecond pass: {}",
        case.git_lines()
            .iter()
            .filter(|line| line.starts_with("clone"))
            .collect::<Vec<_>>(),
        describe(&first),
        describe(&second)
    );
    assert!(!nested, "a clone's work tree was written at a .git path");
}

/// Invariant 1, the coordinator's first question: a listed path with a `..` step clones nothing
/// and writes nothing beside the checkouts root; one whose step starts with `-` is cloned under
/// the root, the URL after `--`, so git takes no option from it.
#[test]
fn hostile_listing_paths_write_nothing_outside_the_checkouts_root() {
    let case = Case::new("hostile-paths");
    let config = case.config("sub");

    case.projects(&[(101, "example-org/sub/../../escape")]);
    case.bare("escape", &[("README", "x\n".to_owned(), false)]);
    let output = case.pass(&config, &[]);
    assert!(!case.dir.join("escape").exists(), "{}", describe(&output));
    assert!(
        !case.root().join("escape").exists(),
        "{}",
        describe(&output)
    );
    assert!(
        !case
            .git_lines()
            .iter()
            .any(|line| line.starts_with("clone")),
        "{:?}",
        case.git_lines()
    );

    case.projects(&[(101, "/example-org/sub/-x")]);
    case.bare("example-org/sub/-x", &[("README", "x\n".to_owned(), false)]);
    let output = case.pass(&config, &[]);
    assert!(
        case.root().join("sub/-x/.git").is_dir(),
        "{}",
        describe(&output)
    );
    let clone = case
        .git_lines()
        .into_iter()
        .find(|line| line.starts_with("clone"))
        .expect("a clone ran");
    assert!(
        clone.starts_with(&format!(
            "clone --quiet -- https://{HOST}/example-org/sub/-x.git "
        )),
        "{clone}"
    );
    assert!(case.partials().is_empty(), "{:?}", case.partials());
}

/// The coordinator's third question: a clone still running when the collector's budget is spent
/// is stopped with its process group, and its temporary directory under the checkouts root is
/// removed; no checkout is left at its path.
#[test]
fn a_clone_stopped_at_the_collector_budget_leaves_no_temporary_directory() {
    let case = Case::new("budget");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    case.git_wrapper(&format!(
        "if [ \"$1\" = clone ]; then exec '{}' 60; fi",
        text(&which("sleep"))
    ));
    let config = case.config("alpha");

    let started = Instant::now();
    let output = case.pass(&config, &["--budget", "3"]);
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(40),
        "took {took:?}: {}",
        describe(&output)
    );
    assert!(case.partials().is_empty(), "{:?}", case.partials());
    assert!(!case.root().join("alpha").exists(), "{}", describe(&output));
}
