//! `conductor build run` gives each repository one target directory, shared by its trees under a
//! lock (`story:build-shared-target-dir`).
//!
//! Each case runs [`build::run`] over a case directory, as `build_run.rs` does, in a git
//! repository of the case (`repo/`, origin `https://example.org/example-org/example.git`) or a
//! second worktree of it (`tree2/`). The shared target directories lie under `targets/` of the
//! case (`Sources::targets`), never under the instance's real cache.
//!
//! The repository is a Cargo package whose binary prints the directory of the tree it was built
//! in, and whose one test waits a few seconds, then runs the binary cargo uplifted
//! (`CARGO_BIN_EXE_example`) and asserts it prints the test's own tree. Two trees of one
//! repository sharing one target without a lock over the whole command overwrite each other's
//! binary while the other's test waits.

mod active;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use conductor_cli::build::{self, Outcome, Request, Settings, Sources};
use conductor_cli::gc::reclaim::ReclaimReport;
use time::OffsetDateTime;

const GIB: u64 = 1 << 30;

/// The origin of the case's repository.
const ORIGIN: &str = "https://example.org/example-org/example.git";

const MANIFEST: &str = "[package]\nname = \"example\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
                        [workspace]\n";

const MAIN: &str = "fn main() {\n    println!(\"{}\", env!(\"CARGO_MANIFEST_DIR\"));\n}\n";

/// The test reads the tree it runs for from its working directory, which cargo sets to the tree it
/// was run in; it asserts that the test binary itself was built from that tree, waits long enough
/// for the other tree's build to finish beside it, then asserts the binary the shared target
/// holds prints that tree too.
const TEST: &str = "#[test]\nfn the_binary_is_this_trees() {\n    \
                    let here = std::env::current_dir().expect(\"the working directory\");\n    \
                    let here = here.to_str().expect(\"text\");\n    \
                    assert_eq!(env!(\"CARGO_MANIFEST_DIR\"), here, \"the test binary\");\n    \
                    std::thread::sleep(std::time::Duration::from_secs(4));\n    \
                    let out = std::process::Command::new(env!(\"CARGO_BIN_EXE_example\"))\n        \
                    .output()\n        .expect(\"the binary runs\");\n    \
                    let printed = String::from_utf8(out.stdout).expect(\"text\");\n    \
                    assert_eq!(printed.trim(), here, \"the binary\");\n}\n";

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
    /// A case with a git repository `repo/` holding one commit of the Cargo package, its origin
    /// [`ORIGIN`], and a second worktree of it, `tree2/`.
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("build_shared_target")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("repo/src")).expect("create the case's repository");
        fs::create_dir_all(dir.join("repo/tests")).expect("create the case's tests");
        fs::write(dir.join("repo/Cargo.toml"), MANIFEST).expect("write the manifest");
        fs::write(dir.join("repo/src/main.rs"), MAIN).expect("write the binary");
        fs::write(dir.join("repo/tests/own.rs"), TEST).expect("write the test");
        fs::write(dir.join("repo/.gitignore"), "/target\n").expect("write .gitignore");
        fs::write(dir.join("free.txt"), (100 * GIB).to_string()).expect("write free disk");
        let case = Self {
            dir: dir.canonicalize().expect("the case's directory"),
        };
        case.git(&["init", "-q", "repo"]);
        case.git(&["-C", "repo", "remote", "add", "origin", ORIGIN]);
        case.git(&["-C", "repo", "add", "."]);
        case.git(&["-C", "repo", "commit", "-q", "-m", "example"]);
        case.git(&[
            "-C", "repo", "worktree", "add", "-q", "--detach", "../tree2",
        ]);
        case
    }

    /// Runs git in the case directory, reading no configuration of the host's user.
    fn git(&self, args: &[&str]) {
        let status = Command::new("git")
            .args(args)
            .current_dir(&self.dir)
            .env("GIT_CONFIG_GLOBAL", "/dev/null")
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_AUTHOR_NAME", "example")
            .env("GIT_AUTHOR_EMAIL", "example@example.org")
            .env("GIT_COMMITTER_NAME", "example")
            .env("GIT_COMMITTER_EMAIL", "example@example.org")
            .status()
            .expect("git runs");
        assert!(status.success(), "git {args:?}");
    }

    fn state(&self) -> PathBuf {
        self.dir.join("state")
    }

    fn host(&self) -> PathBuf {
        self.dir.join("host")
    }

    /// The directory holding the repositories' shared targets.
    fn targets(&self) -> PathBuf {
        self.dir.join("targets")
    }

    /// The shared target directories under [`Case::targets`]: every entry but the locks.
    fn shared(&self) -> Vec<PathBuf> {
        let mut found: Vec<PathBuf> = fs::read_dir(self.targets())
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .filter(|path| path.file_name().is_some_and(|name| name != ".locks"))
                    .collect()
            })
            .unwrap_or_default();
        found.sort();
        found
    }

    fn sources(&self) -> Sources {
        Sources {
            disk: self.dir.join("free.txt"),
            free_bytes: |file| Ok(fs::read_to_string(file)?.trim().parse()?),
            reclaim: reclaim_nothing,
            clock: OffsetDateTime::now_utc,
            every: Duration::from_millis(200),
            poll: Duration::from_millis(20),
            home: Some(self.dir.join("home")),
            var_tmp: self.dir.join("var-tmp"),
            scratch: self.dir.join("scratch"),
            say: said_to_file,
            env: |_| None,
            sessions: Vec::new(),
            walk: build::walk,
            sccache: None,
            targets: Some(self.targets()),
        }
    }

    /// `command` in `tree`.
    fn request(&self, tree: &str, command: &[&str]) -> Request {
        Request {
            reserve: Some(GIB),
            repository: Some("example-repo".to_owned()),
            session_name: None,
            command: command.iter().map(|word| (*word).to_owned()).collect(),
            measure: Vec::new(),
            cwd: self.dir.join(tree),
            output: None,
        }
    }

    /// `sh -c <script>` in `tree`; `# runs cargo` makes it a cargo-based command.
    fn script(&self, tree: &str, script: &str) -> Request {
        self.request(tree, &["sh", "-c", script])
    }

    fn run(&self, sources: &Sources, request: &Request) -> Outcome {
        build::run(&self.state(), &self.host(), &settings(), sources, request)
            .expect("the run ends")
    }

    fn read(&self, path: &str) -> String {
        fs::read_to_string(self.dir.join(path)).unwrap_or_default()
    }

    /// What the runs of the case wrote to standard error.
    fn said(&self) -> String {
        self.read("stderr.txt")
    }
}

/// No compile cache and no lean debug information: the cases see only the target directory.
fn settings() -> Settings {
    Settings {
        instance: "example-org".to_owned(),
        fall_key: "example-org".to_owned(),
        floor: 10 * GIB,
        clear: 20 * GIB,
        gate_stop: 5 * GIB,
        gate_resume: 8 * GIB,
        wait: Duration::ZERO,
        cores: 20,
        cpu_reserve: 2,
        slots: 0,
        repo_cache: false,
        repo_cache_size: 2 * GIB,
        lean_debuginfo: false,
        short_slots: 0,
        short_max: Duration::from_secs(180),
    }
}

fn reclaim_nothing(_state: &Path, _need: u64) -> anyhow::Result<ReclaimReport> {
    Ok(ReclaimReport {
        reclaimed_bytes: 0,
        steps: Vec::new(),
        left: Vec::new(),
        reached: false,
    })
}

/// Appends `line` to `stderr.txt` of the case, the state directory's parent.
fn said_to_file(state: &Path, line: &str) {
    use std::io::Write as _;
    let case = state.parent().expect("the state lies in its case");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(case.join("stderr.txt"))
        .expect("open the case's stderr.txt");
    writeln!(file, "{line}").expect("write the case's stderr.txt");
}

/// The script that writes the target directory the command sees to `env.txt` of its tree.
const TARGET: &str = "echo \"${CARGO_TARGET_DIR-unset}\" > env.txt # runs cargo";

/// Acceptance 1, 2 and 3: two trees of one repository run `build run -- cargo test` at once. Both
/// build into the one shared target, neither tree's own `target/` is written, the second run
/// queues behind the first and says its position once, and each tree's test runs the binary
/// built from its own tree.
#[test]
fn two_trees_run_cargo_test_at_once_and_each_runs_its_own_binary() {
    let case = Case::new("two-trees");
    let sources = case.sources();
    let outcomes = std::thread::scope(|scope| {
        let runs: Vec<_> = ["repo", "tree2"]
            .map(|tree| {
                let request = case.request(tree, &["cargo", "test", "--offline", "--quiet"]);
                let (case, sources) = (&case, &sources);
                scope.spawn(move || case.run(sources, &request))
            })
            .into_iter()
            .collect();
        runs.into_iter()
            .map(|run| run.join().expect("the run's thread ends"))
            .collect::<Vec<_>>()
    });
    for outcome in &outcomes {
        assert_eq!(outcome.exit_code(), 0, "{outcomes:?}\n{}", case.said());
    }
    let shared = case.shared();
    assert_eq!(shared.len(), 1, "one target for the repository: {shared:?}");
    assert!(
        shared[0].join("debug").is_dir(),
        "the build went into the shared target {}",
        shared[0].display()
    );
    for tree in ["repo", "tree2"] {
        assert!(
            !case.dir.join(tree).join("target").exists(),
            "{tree}'s own target/ stays empty"
        );
    }
    let said = case.said();
    let queued: Vec<&str> = said
        .lines()
        .filter(|line| line.contains("queue position"))
        .collect();
    assert_eq!(
        queued.len(),
        1,
        "the waiting run says its position once:\n{said}"
    );
}

/// Acceptance 1: the shared target is the repository's, keyed by its origin: every tree and every
/// clone of one origin gets the same directory, another origin another one.
#[test]
fn the_target_is_keyed_by_the_repositorys_origin() {
    let case = Case::new("keyed");
    case.git(&["clone", "-q", "repo", "clone"]);
    case.git(&["-C", "clone", "remote", "set-url", "origin", ORIGIN]);
    case.git(&["init", "-q", "other"]);
    case.git(&[
        "-C",
        "other",
        "remote",
        "add",
        "origin",
        "https://example.org/example-org/other.git",
    ]);
    let sources = case.sources();
    for tree in ["repo", "tree2", "clone", "other"] {
        let outcome = case.run(&sources, &case.script(tree, TARGET));
        assert_eq!(outcome.exit_code(), 0, "{outcome:?}: {}", case.said());
    }
    let seen = |tree: &str| case.read(&format!("{tree}/env.txt")).trim().to_owned();
    let shared = PathBuf::from(seen("repo"));
    assert_eq!(
        shared.parent(),
        Some(case.targets().as_path()),
        "{shared:?}"
    );
    assert_eq!(seen("tree2"), seen("repo"));
    assert_eq!(seen("clone"), seen("repo"));
    assert_ne!(seen("other"), seen("repo"));
    assert_eq!(
        PathBuf::from(seen("other")).parent(),
        Some(case.targets().as_path())
    );
}

/// Acceptance 1: a `CARGO_TARGET_DIR` the caller set is kept, and standard error names it.
#[test]
fn a_target_dir_the_caller_set_is_kept_and_named() {
    let case = Case::new("kept");
    let sources = Sources {
        env: |name| (name == "CARGO_TARGET_DIR").then(|| "/elsewhere/target".to_owned()),
        ..case.sources()
    };
    let outcome = case.run(&sources, &case.script("repo", TARGET));
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}: {}", case.said());
    assert!(
        case.shared().is_empty(),
        "no shared target: {:?}",
        case.shared()
    );
    let seen = case.read("repo/env.txt");
    assert!(
        !seen
            .trim()
            .starts_with(&case.targets().display().to_string()),
        "{seen}"
    );
    let said = case.said();
    assert!(
        said.lines().any(|line| line.contains("CARGO_TARGET_DIR")
            && line.contains("/elsewhere/target")
            && line.contains("kept")),
        "{said}"
    );
}

/// A command that does not run cargo gets no target directory and takes no lock.
#[test]
fn a_command_without_cargo_gets_no_target() {
    let case = Case::new("no-cargo");
    let script = "echo \"${CARGO_TARGET_DIR-unset}\" > env.txt";
    let outcome = case.run(&case.sources(), &case.script("repo", script));
    assert_eq!(outcome.exit_code(), 0, "{outcome:?}: {}", case.said());
    assert!(!case.targets().exists(), "nothing under the targets");
    assert!(
        !case
            .read("repo/env.txt")
            .contains(&case.targets().display().to_string())
    );
}
