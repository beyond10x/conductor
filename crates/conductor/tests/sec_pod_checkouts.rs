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

// ---------------------------------------------------------------------------------------------
// Round 2: the fix of the `.git` step, the trust at a start, and a stopped snapshot's clones
// ---------------------------------------------------------------------------------------------

/// A git directory whose `config` names `.git/hook` as `core.fsmonitor`, as a repository's own
/// files, the hook writing `marker`.
fn gitdir_shaped(marker: &Path, origin: &str) -> Vec<(&'static str, String, bool)> {
    vec![
        ("HEAD", "ref: refs/heads/main\n".to_owned(), false),
        (
            "config",
            format!(
                "[core]\n\trepositoryformatversion = 0\n\tbare = false\n\tfsmonitor = .git/hook\n\
                 [remote \"origin\"]\n\turl = https://{HOST}/{origin}.git\n"
            ),
            false,
        ),
        ("objects/keep", "x\n".to_owned(), false),
        ("refs/keep", "x\n".to_owned(), false),
        (
            "hook",
            format!("#!/bin/sh\n: > '{}'\nexit 1\n", text(marker)),
            true,
        ),
    ]
}

/// The runtime of a pod, `instance serve` with its loop off, killed when dropped.
struct Started {
    child: std::process::Child,
    printed: std::sync::Arc<std::sync::Mutex<String>>,
}

impl Started {
    fn printed(&self) -> String {
        self.printed
            .lock()
            .map(|printed| printed.clone())
            .unwrap_or_default()
    }
}

impl Drop for Started {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Case {
    /// Every answer of the project 101 as the project `to`'s.
    fn answer_as_101(&self, to: u64) {
        for entry in fs::read_dir(self.dir.join("answers")).expect("read the answers") {
            let path = entry.expect("an answer").path();
            let name = path
                .file_name()
                .and_then(|name| name.to_str())
                .expect("an answer's name")
                .to_owned();
            if let Some(rest) = name.strip_prefix("101.") {
                fs::copy(&path, self.dir.join("answers").join(format!("{to}.{rest}")))
                    .expect("copy an answer");
            }
        }
    }

    /// `conductor <args>` with the environment of [`Case::pass`], not yet run.
    fn pod_command(&self, config: &Path, args: &[&str]) -> Command {
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
            .args(args)
            .stdin(Stdio::null());
        command
    }

    /// The pod's runtime started over `config`, until it says its seeding ended.
    fn start_pod(&self, config: &Path) -> Started {
        use std::io::Read as _;
        let mut command = self.pod_command(config, &["instance", "serve"]);
        command
            .env(conductor_cli::instance::TOKEN_VARIABLE, "instance-token")
            .env(conductor_cli::instance::LOOP_VARIABLE, "off")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn().expect("instance serve starts");
        let printed = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
        let pipes: [Box<dyn std::io::Read + Send>; 2] = [
            Box::new(child.stdout.take().expect("stdout")),
            Box::new(child.stderr.take().expect("stderr")),
        ];
        for mut pipe in pipes {
            let printed = std::sync::Arc::clone(&printed);
            std::thread::spawn(move || {
                let mut chunk = [0_u8; 1024];
                while let Ok(read @ 1..) = pipe.read(&mut chunk) {
                    if let Ok(mut printed) = printed.lock() {
                        printed.push_str(&String::from_utf8_lossy(&chunk[..read]));
                    }
                }
            });
        }
        let mut started = Started { child, printed };
        let deadline = Instant::now() + Duration::from_secs(120);
        loop {
            let printed = started.printed();
            if printed.contains("seeded its instance") || printed.contains("the seeding failed") {
                return started;
            }
            if let Ok(Some(status)) = started.child.try_wait() {
                panic!("instance serve exited {status}: {printed}");
            }
            assert!(Instant::now() < deadline, "never seeded: {printed}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// The paths `~/.claude.json` of the case's home trusts.
    fn trusted(&self) -> Vec<String> {
        let file = self
            .dir
            .join("home")
            .join(conductor_cli::trust::CLAUDE_JSON);
        let document: Value = fs::read_to_string(&file)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or(Value::Null);
        document["projects"]
            .as_object()
            .map(|projects| {
                projects
                    .iter()
                    .filter(|(_, project)| project["hasTrustDialogAccepted"] == json!(true))
                    .map(|(path, _)| path.clone())
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Round 2, claim A: the step `.git` written in another letter case, and `.git.`, make no clone's
/// work tree a git directory, and no repository's fsmonitor program runs at the next pass.
#[test]
fn a_dot_git_step_in_another_case_or_with_a_trailing_dot_runs_no_code() {
    let case = Case::new("dot-git-variants");
    let marker = case.dir.join("fsmonitor-ran");
    case.projects(&[
        (101, "example-org/platform/.GIT"),
        (102, "example-org/platform/.git."),
    ]);
    case.answer_as_101(102);
    case.bare(
        "example-org/platform/.GIT",
        &gitdir_shaped(&marker, "example-org/platform/.GIT"),
    );
    case.bare(
        "example-org/platform/.git.",
        &gitdir_shaped(&marker, "example-org/platform/.git."),
    );
    let config = case.config("platform");

    let first = case.pass(&config, &[]);
    let second = case.pass(&config, &[]);
    let platform = case.root().join("platform");
    assert!(
        !marker.exists(),
        "a repository's fsmonitor program ran\nfirst: {}\nsecond: {}",
        describe(&first),
        describe(&second)
    );
    assert!(!platform.join(".GIT").exists(), "{}", describe(&first));
    assert!(!platform.join(".git").exists(), "{}", describe(&first));
}

/// Round 2, claim B: a checkout the pod cloned while an `include` entry covered it, and which no
/// entry covers since, is not trusted again by a later start (whose home holds no trust yet, as
/// a pod's home off the volume claim does).
#[test]
fn a_start_trusts_no_checkout_its_include_no_longer_covers() {
    let case = Case::new("start-trust");
    case.projects(&[(101, "example-org/alpha"), (102, "example-org/beta")]);
    case.answer_as_101(102);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    case.bare("example-org/beta", &[("README", "x\n".to_owned(), false)]);

    let config = case.config("alpha, beta");
    let first = case.start_pod(&config);
    let printed = first.printed();
    drop(first);
    assert!(
        case.root().join("beta/.git").is_dir(),
        "the first start cloned beta: {printed}"
    );

    let config = case.config("alpha");
    let _ = fs::remove_file(
        case.dir
            .join("home")
            .join(conductor_cli::trust::CLAUDE_JSON),
    );
    let second = case.start_pod(&config);
    let printed = second.printed();
    drop(second);
    let beta = text(&fs::canonicalize(case.root().join("beta")).expect("beta is there"));
    let trusted = case.trusted();
    assert!(
        !trusted.contains(&beta),
        "the start trusted a checkout no include entry covers: {trusted:#?}\n{printed}"
    );
}

/// Round 2, claim D: a snapshot stopped with its process group (as `collect::run` stops the
/// start's snapshot after an hour and the cycle's step after its bound) leaves no clone running.
/// Each command a collector runs leads a process group of its own, so a stop of the snapshot's
/// group does not reach it.
#[test]
fn a_stopped_snapshot_leaves_no_clone_running() {
    use std::os::unix::process::CommandExt as _;
    let case = Case::new("orphan-clone");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    let pid_file = case.dir.join("clone.pid");
    case.git_wrapper(&format!(
        "if [ \"$1\" = clone ]; then printf '%s' $$ > '{}'; exec '{}' 300; fi",
        text(&pid_file),
        text(&which("sleep"))
    ));
    let config = case.config("alpha");

    let mut command = case.pod_command(
        &config,
        &[
            "snapshot",
            "start-snapshot",
            "--started-at",
            NOW,
            "--disk-free-bytes",
            "100000000000",
        ],
    );
    command
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0);
    let mut snapshot = command.spawn().expect("the snapshot starts");
    let group = i32::try_from(snapshot.id()).expect("a process id");
    let deadline = Instant::now() + Duration::from_secs(60);
    let clone = loop {
        if let Some(pid) = fs::read_to_string(&pid_file)
            .ok()
            .and_then(|text| text.trim().parse::<i32>().ok())
        {
            break pid;
        }
        if Instant::now() >= deadline {
            let _ = nix::sys::signal::killpg(
                nix::unistd::Pid::from_raw(group),
                nix::sys::signal::Signal::SIGKILL,
            );
            let _ = snapshot.wait();
            panic!("no clone began within 60 s");
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let _ = nix::sys::signal::killpg(
        nix::unistd::Pid::from_raw(group),
        nix::sys::signal::Signal::SIGKILL,
    );
    let _ = snapshot.wait();
    std::thread::sleep(Duration::from_millis(500));
    let running = Path::new(&format!("/proc/{clone}")).exists();
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(clone),
        nix::sys::signal::Signal::SIGKILL,
    );
    assert!(
        !running,
        "the clone (process {clone}) still ran after the snapshot's process group was stopped"
    );
}

// ---------------------------------------------------------------------------------------------
// The fixes of round 2: the runner's bound, the budget, and what a start still trusts
// ---------------------------------------------------------------------------------------------

/// Whether the process `pid` runs: it is there and no zombie.
fn alive(pid: i32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.rsplit_once(')')
            .and_then(|(_, rest)| rest.split_whitespace().next())
            .is_some_and(|state| state != "Z")
    })
}

/// The process id the file `file` holds, once a command wrote it, within 60 s.
fn written_pid(file: &Path) -> Option<i32> {
    let deadline = Instant::now() + Duration::from_secs(60);
    loop {
        if let Some(pid) = fs::read_to_string(file)
            .ok()
            .and_then(|text| text.trim().parse::<i32>().ok())
        {
            return Some(pid);
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

impl Case {
    /// The config file of [`Case::config`] with the sources `sources`, a YAML flow list.
    fn config_sources(&self, sources: &str) -> PathBuf {
        let text = format!(
            "version: conductor.config/1\ninstances:\n  - name: acme\n    sources: {sources}\n    \
             checkouts:\n      root: {}\n      trees: {}\n    records: {}\n    state: {}\n    \
             cache: {}\n",
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

    /// An operator's hand-made checkout `<root>/<name>`: a git directory with no `origin`.
    fn hand_made(&self, name: &str) -> String {
        let dir = self.root().join(name);
        fs::create_dir_all(&dir).expect("create the checkout");
        git(&dir, &["init", "--quiet", "--initial-branch=main"]);
        text(&fs::canonicalize(&dir).expect("the checkout is there"))
    }
}

/// The cycle's snapshot step, run as the cycle and a pod's start run it (`collect::run`: the
/// step leads a process group of its own, which is stopped at the step's bound), leaves no clone
/// of its collectors running once it is stopped at that bound.
#[test]
fn a_snapshot_step_stopped_at_its_bound_leaves_no_clone_running() {
    let case = Case::new("step-bound");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    let pid_file = case.dir.join("clone.pid");
    case.git_wrapper(&format!(
        "if [ \"$1\" = clone ]; then printf '%s' $$ > '{}'; exec '{}' 300; fi",
        text(&pid_file),
        text(&which("sleep"))
    ));
    let config = case.config("alpha");
    let mut step = case.pod_command(
        &config,
        &[
            "snapshot",
            "start-snapshot",
            "--started-at",
            NOW,
            "--disk-free-bytes",
            "100000000000",
        ],
    );

    let stopped = conductor_cli::collect::run(&mut step, Duration::from_secs(15));
    let Some(clone) = written_pid(&pid_file) else {
        panic!("no clone began within the step's bound: {stopped:?}");
    };
    std::thread::sleep(Duration::from_millis(500));
    let running = alive(clone);
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(clone),
        nix::sys::signal::Signal::SIGKILL,
    );
    assert!(stopped.is_err(), "the step was not stopped: {stopped:?}");
    assert!(
        !running,
        "the clone (process {clone}) still ran after the step was stopped at its bound"
    );
}

/// A clone stopped at the collector's budget ends with every process it started, not only the
/// one the collector started: a command the snapshot runs stays in the snapshot's process group,
/// and the budget stops that command and each process under it.
#[test]
fn a_clone_stopped_at_the_budget_leaves_none_of_its_processes_running() {
    let case = Case::new("budget-tree");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    let pid_file = case.dir.join("helper.pid");
    case.git_wrapper(&format!(
        "if [ \"$1\" = clone ]; then '{}' 300 & printf '%s' $! > '{}'; wait; exit 1; fi",
        text(&which("sleep")),
        text(&pid_file)
    ));
    let config = case.config("alpha");

    let output = case.pass(&config, &["--budget", "3"]);
    let Some(helper) = written_pid(&pid_file) else {
        panic!("no clone began: {}", describe(&output));
    };
    std::thread::sleep(Duration::from_millis(500));
    let running = alive(helper);
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(helper),
        nix::sys::signal::Signal::SIGKILL,
    );
    assert!(
        !running,
        "a process the clone started (process {helper}) still ran after the budget stopped the \
         clone: {}",
        describe(&output)
    );
    assert!(case.partials().is_empty(), "{:?}", case.partials());
}

/// A start trusts a checkout under the root that no `include` entry covers where a source on
/// the root names no `include`: an operator's hand-made checkout stays usable.
#[test]
fn a_start_trusts_a_checkout_under_a_source_without_include() {
    let case = Case::new("start-trust-every");
    case.projects(&[(101, "example-org/alpha")]);
    let hand = case.hand_made("hand");
    let config = case.config_sources(&format!("[{{gitlab: example-org, host: {HOST}}}]"));
    let started = case.start_pod(&config);
    let printed = started.printed();
    drop(started);
    let trusted = case.trusted();
    assert!(
        trusted.contains(&hand),
        "the start did not trust {hand}: {trusted:#?}\n{printed}"
    );
}

/// A start trusts a checkout under the root that no `include` entry covers where a `local`
/// source's path is the root, beside a `gitlab` source that names `include`.
#[test]
fn a_start_trusts_a_checkout_under_a_local_source() {
    let case = Case::new("start-trust-local");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    let hand = case.hand_made("hand");
    let config = case.config_sources(&format!(
        "[{{gitlab: example-org, host: {HOST}, include: [alpha]}}, {{local: {}}}]",
        quoted(&case.root())
    ));
    let started = case.start_pod(&config);
    let printed = started.printed();
    drop(started);
    let trusted = case.trusted();
    assert!(
        trusted.contains(&hand),
        "the start did not trust {hand}: {trusted:#?}\n{printed}"
    );
}

// ---------------------------------------------------------------------------------------------
// Round 3: a process a timed-out command left behind, inside a snapshot that ends normally
// ---------------------------------------------------------------------------------------------

/// Round 3, claim B: a clone that exits at once and leaves a process of its own on its pipes is
/// stopped at the collector's budget, and that process does not run on after the snapshot ended
/// normally. Before the snapshot kept its commands in its own group, the clone's own group was
/// stopped at the budget, and the process with it.
#[test]
fn a_helper_left_on_a_timed_out_clones_pipes_does_not_outlive_the_snapshot() {
    let case = Case::new("helper-on-pipes");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    let pid_file = case.dir.join("helper.pid");
    case.git_wrapper(&format!(
        "if [ \"$1\" = clone ]; then '{}' 300 & printf '%s' $! > '{}'; exit 0; fi",
        text(&which("sleep")),
        text(&pid_file)
    ));
    let config = case.config("alpha");

    let output = case.pass(&config, &["--budget", "3"]);
    let Some(helper) = written_pid(&pid_file) else {
        panic!("no clone began: {}", describe(&output));
    };
    std::thread::sleep(Duration::from_millis(500));
    let running = alive(helper);
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(helper),
        nix::sys::signal::Signal::SIGKILL,
    );
    assert!(
        !running,
        "a process the clone left on its pipes (process {helper}) still ran after the snapshot \
         ended: {}",
        describe(&output)
    );
}

/// Round 3, the other path: a clone that starts a process which leaves it (its parent exits, so
/// it is re-parented, and it holds none of the clone's pipes) and then runs past the collector's
/// budget. Stopping the clone and the processes under it misses that process; it does not run on
/// after the snapshot ended normally.
#[test]
fn a_helper_a_timed_out_clone_detached_does_not_outlive_the_snapshot() {
    let case = Case::new("helper-detached");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    let pid_file = case.dir.join("helper.pid");
    let sleep = text(&which("sleep"));
    case.git_wrapper(&format!(
        "if [ \"$1\" = clone ]; then ( '{sleep}' 300 </dev/null >/dev/null 2>&1 & printf '%s' $! > \
         '{}' ); exec '{sleep}' 30; fi",
        text(&pid_file)
    ));
    let config = case.config("alpha");

    let output = case.pass(&config, &["--budget", "3"]);
    let Some(helper) = written_pid(&pid_file) else {
        panic!("no clone began: {}", describe(&output));
    };
    std::thread::sleep(Duration::from_millis(500));
    let running = alive(helper);
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(helper),
        nix::sys::signal::Signal::SIGKILL,
    );
    assert!(
        !running,
        "a process the clone detached (process {helper}) still ran after the snapshot ended: {}",
        describe(&output)
    );
}

/// Round 3 re-check, claim E: a process a `git fetch` of the pass detaches for work it leaves to
/// run on (git's own `git maintenance run --auto --detach` after a fetch: a fork whose parent
/// exits, in a session of its own) is not stopped when the snapshot ends. The wrapper starts it
/// as git's daemonize does, through `setsid -f`, its standard streams on `/dev/null`.
#[test]
fn a_process_a_fetch_detaches_is_not_stopped_when_the_snapshot_ends() {
    let case = Case::new("fetch-detached");
    case.projects(&[(101, "example-org/alpha")]);
    case.bare("example-org/alpha", &[("README", "x\n".to_owned(), false)]);
    let config = case.config("alpha");
    case.pass(&config, &[]);
    assert!(
        case.root().join("alpha/.git").is_dir(),
        "the first pass cloned alpha"
    );

    let pid_file = case.dir.join("maintenance.pid");
    case.git_wrapper(&format!(
        "if [ \"$1\" = fetch ]; then '{setsid}' -f '{sh}' -c 'printf %s $$ > \"$0\"; exec \"$1\" 120' \
         '{pid}' '{sleep}' </dev/null >/dev/null 2>&1; fi",
        setsid = text(&which("setsid")),
        sh = text(&which("sh")),
        pid = text(&pid_file),
        sleep = text(&which("sleep")),
    ));
    let output = case.pass(&config, &[]);
    let Some(detached) = written_pid(&pid_file) else {
        panic!("no fetch detached a process: {}", describe(&output));
    };
    std::thread::sleep(Duration::from_millis(500));
    let running = alive(detached);
    let _ = nix::sys::signal::kill(
        nix::unistd::Pid::from_raw(detached),
        nix::sys::signal::Signal::SIGKILL,
    );
    assert!(
        running,
        "the process the fetch detached (process {detached}) was stopped when the snapshot \
         ended: {}",
        describe(&output)
    );
}
