//! Adversarial cases for `story:build-shared-target-dir`: the shared target of a repository is
//! reused by another tree only when that is safe.
//!
//! `a_crate_in_a_submodule_is_rebuilt_from_this_tree` calls [`build::run`] in-process, as
//! `build_shared_target.rs` does. `a_killed_build_run_keeps_its_repository_locked` spawns the
//! built `conductor` through `common::conductor` with a temporary `HOME`.

mod active;
mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use conductor_cli::build::{self, Request, Settings, Sources};
use conductor_cli::gc::reclaim::{READINGS_VARIABLE, ReclaimReport, VAR_TMP_VARIABLE};
use time::OffsetDateTime;

const GIB: u64 = 1 << 30;

const ORIGIN: &str = "https://example.org/example-org/adv-example.git";

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "protocol.file.allow=always"])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_AUTHOR_NAME", "example")
        .env("GIT_AUTHOR_EMAIL", "example@example.org")
        .env("GIT_COMMITTER_NAME", "example")
        .env("GIT_COMMITTER_EMAIL", "example@example.org")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} in {}", dir.display());
}

fn case_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("build_shared_target_adv")
        .join(name);
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("create the case directory");
    dir.canonicalize().expect("the case directory")
}

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

fn said_to_file(state: &Path, line: &str) {
    use std::io::Write as _;
    let case = state.parent().expect("the state lies in its case");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(case.join("stderr.txt"))
        .expect("open stderr.txt");
    writeln!(file, "{line}").expect("write stderr.txt");
}

fn sources(dir: &Path) -> Sources {
    Sources {
        disk: dir.join("free.txt"),
        free_bytes: |file| Ok(fs::read_to_string(file)?.trim().parse()?),
        reclaim: reclaim_nothing,
        clock: OffsetDateTime::now_utc,
        every: Duration::from_millis(200),
        poll: Duration::from_millis(20),
        home: Some(dir.join("home")),
        var_tmp: dir.join("var-tmp"),
        scratch: dir.join("scratch"),
        say: said_to_file,
        env: |_| None,
        sessions: Vec::new(),
        walk: build::walk,
        sccache: None,
        targets: Some(dir.join("targets")),
    }
}

fn script(dir: &Path, tree: &str, script: &str) -> Request {
    Request {
        reserve: Some(GIB),
        repository: Some("adv-example".to_owned()),
        session_name: None,
        command: ["sh", "-c", script].map(str::to_owned).to_vec(),
        measure: Vec::new(),
        cwd: dir.join(tree),
        output: None,
    }
}

/// A workspace whose one member, `sub/`, is a git submodule. `tree2` is a second worktree of the
/// repository whose submodule holds other source than `repo`'s (checked out before `repo`
/// builds, as a tree created earlier and built later is). `repo` builds into the shared target,
/// then `tree2` runs its binary through `build run`: it must print `tree2`'s source, not run the
/// binary `repo` built. `SharedTarget::claim` marks only the files `git ls-files` names in the
/// superproject; a submodule is one gitlink entry there, so its files keep their older times and
/// cargo finds `repo`'s artifacts fresh.
#[test]
fn a_crate_in_a_submodule_is_rebuilt_from_this_tree() {
    active::isolate();
    let dir = case_dir("submodule");
    fs::write(dir.join("free.txt"), (100 * GIB).to_string()).expect("free disk");
    let sub = dir.join("subsrc");
    fs::create_dir_all(sub.join("src")).expect("create the submodule source");
    fs::write(
        sub.join("Cargo.toml"),
        "[package]\nname = \"advsub\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write the member manifest");
    fs::write(
        sub.join("src/main.rs"),
        "fn main() {\n    println!(\"repo\");\n}\n",
    )
    .expect("write the member");
    git(&dir, &["init", "-q", "subsrc"]);
    git(&sub, &["add", "."]);
    git(&sub, &["commit", "-q", "-m", "sub"]);

    let repo = dir.join("repo");
    fs::create_dir_all(&repo).expect("create the repository");
    fs::write(
        repo.join("Cargo.toml"),
        "[workspace]\nmembers = [\"sub\"]\nresolver = \"2\"\n",
    )
    .expect("write the workspace");
    fs::write(repo.join(".gitignore"), "/target\n").expect("write .gitignore");
    git(&dir, &["init", "-q", "repo"]);
    git(&repo, &["remote", "add", "origin", ORIGIN]);
    git(
        &repo,
        &["submodule", "add", "-q", sub.to_str().expect("text"), "sub"],
    );
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "workspace"]);
    git(&repo, &["worktree", "add", "-q", "--detach", "../tree2"]);
    let tree2 = dir.join("tree2");
    git(&tree2, &["submodule", "update", "-q", "--init"]);
    // tree2's member differs from repo's, and was written before repo builds.
    fs::write(
        tree2.join("sub/src/main.rs"),
        "fn main() {\n    println!(\"tree2\");\n}\n",
    )
    .expect("write tree2's member");
    std::thread::sleep(Duration::from_millis(50));

    let sources = sources(&dir);
    let state = dir.join("state");
    let host = dir.join("host");
    let built = build::run(
        &state,
        &host,
        &settings(),
        &sources,
        &script(&dir, "repo", "cargo build --offline --quiet"),
    )
    .expect("repo's run ends");
    assert_eq!(built.exit_code(), 0, "{built:?}");
    let ran = build::run(
        &state,
        &host,
        &settings(),
        &sources,
        &script(&dir, "tree2", "cargo run --offline --quiet > printed.txt"),
    )
    .expect("tree2's run ends");
    let said = fs::read_to_string(dir.join("stderr.txt")).unwrap_or_default();
    assert_eq!(ran.exit_code(), 0, "{ran:?}\n{said}");
    let printed = fs::read_to_string(tree2.join("printed.txt")).unwrap_or_default();
    assert_eq!(
        printed.trim(),
        "tree2",
        "tree2's run printed what repo's binary prints: the shared target's artifact of a \
         submodule crate was reused across trees\n{said}"
    );
    let _ = fs::remove_dir_all(&dir);
}

/// A `build run` killed with SIGKILL leaves its command running (it is started in its own process
/// group), but the repository's lock is a `flock` on a descriptor only `build run` held, so the
/// kernel lets it go at once. A run from another tree then takes the lock and starts while the
/// orphaned command still builds and tests in the shared target: the overlap the lock exists to
/// prevent. The second run must not start before the orphan ended.
#[test]
fn a_killed_build_run_keeps_its_repository_locked() {
    let dir = case_dir("killed");
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create the home");
    let repo = dir.join("repo");
    fs::create_dir_all(&repo).expect("create the repository");
    fs::write(repo.join("README"), "example\n").expect("write a file");
    git(&dir, &["init", "-q", "repo"]);
    git(&repo, &["remote", "add", "origin", ORIGIN]);
    git(&repo, &["add", "."]);
    git(&repo, &["commit", "-q", "-m", "example"]);
    git(&repo, &["worktree", "add", "-q", "--detach", "../tree2"]);
    let readings = dir.join("readings.txt");
    fs::write(&readings, format!("{}\n", 1000 * GIB)).expect("write readings");
    let run = |tree: &str, script: &str| {
        let mut command = common::conductor(&home);
        command
            .arg("--state-dir")
            .arg(dir.join("state"))
            .args(["build", "run", "--reserve", "1G", "--", "sh", "-c", script])
            .env(READINGS_VARIABLE, &readings)
            .env(VAR_TMP_VARIABLE, dir.join("var-tmp"))
            .env_remove("CARGO_TARGET_DIR")
            .current_dir(dir.join(tree))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());
        command
    };
    let first = format!(
        "echo $$ > {d}/orphan.pid; touch {d}/a.start; sleep 6; touch {d}/a.end # runs cargo",
        d = dir.display()
    );
    let mut killed = run("repo", &first).spawn().expect("conductor runs");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !dir.join("a.start").exists() {
        assert!(Instant::now() < deadline, "the first command never started");
        std::thread::sleep(Duration::from_millis(20));
    }
    // SIGKILL to this case's own `build run`, by its pid.
    killed.kill().expect("kill the first build run");
    let _ = killed.wait();
    let second = format!(
        "if [ -e {d}/a.end ]; then echo after; else echo during; fi > {d}/b.txt # runs cargo",
        d = dir.display()
    );
    let output = run("tree2", &second).output().expect("conductor runs");
    // The orphan ends by itself; the case waits for it rather than signal anything.
    let deadline = Instant::now() + Duration::from_secs(30);
    while !dir.join("a.end").exists() && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    let seen = fs::read_to_string(dir.join("b.txt")).unwrap_or_default();
    assert_eq!(
        seen.trim(),
        "after",
        "tree2's run started while the killed run's command still ran in the shared target\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(&dir);
}
