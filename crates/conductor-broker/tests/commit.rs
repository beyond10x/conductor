//! `commit` (`story:tool-build-commit-report`, acceptance 3; design § 6.3): the broker stages the
//! named paths, new ones included, and commits them through the bot route with the instance's
//! Gates policy, which only it holds; it prints `<sha> <n> files`. A path outside the grant's
//! paths or outside the caller's working directory refuses the call `NotGranted` before anything
//! is staged.
//!
//! The repository is a fixture under the case's directory, never a real one. The bot route is a
//! fake `b10x-gates` the case writes: it records its arguments and the policy it was given, then
//! runs the git command after `--` in the repository `--repo` names, or refuses when the case
//! says so.

mod support;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use conductor_broker::{BotRoute, CallOutcome, FAILURE_LINES, Services, SessionGrant};
use conductor_model::tool::Verb;
use serde_json::json;
use support::{Case, INSTANCE, ask, grant, lines};

fn git(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("run git");
    assert!(output.status.success(), "git {args:?}: {output:?}");
    String::from_utf8(output.stdout).expect("git prints text")
}

/// A repository in the case's directory `name`, one commit holding `a.txt` and `b.txt`.
fn repository(case: &Case, name: &str) -> PathBuf {
    let dir = case.workdir(name, &[("a.txt", "alpha\n"), ("b.txt", "beta\n")]);
    git(&dir, &["init", "--quiet", "--initial-branch=main"]);
    git(&dir, &["config", "user.name", "Example"]);
    git(&dir, &["config", "user.email", "example@example.invalid"]);
    git(&dir, &["config", "commit.gpgsign", "false"]);
    git(&dir, &["add", "a.txt", "b.txt"]);
    git(&dir, &["commit", "--quiet", "-m", "start"]);
    dir
}

/// A fake bot route in the case's `bin/`: it writes its policy and arguments to `route.log` in
/// the case's directory, then refuses with `refusal` when one is given, or runs `git -C <repo>`
/// with the arguments after `--`.
fn route(case: &Case, refusal: Option<&str>) -> PathBuf {
    let bin = case.root.join("bin");
    fs::create_dir_all(&bin).expect("create the case's bin");
    let program = bin.join("b10x-gates");
    let refuse = refusal
        .map(|line| format!("echo '{line}' >&2\nexit 1\n"))
        .unwrap_or_default();
    fs::write(
        &program,
        format!(
            "#!/bin/sh\n\
             {{ printf 'policy=%s\\n' \"$B10X_GATES_POLICY\"; printf 'arg=%s\\n' \"$@\"; }} > '{log}'\n\
             {refuse}\
             repo=.\n\
             while [ \"$1\" != \"--\" ]; do\n\
             \x20 if [ \"$1\" = \"--repo\" ]; then repo=\"$2\"; shift; fi\n\
             \x20 shift\n\
             done\n\
             shift\n\
             exec git -C \"$repo\" \"$@\"\n",
            log = case.root.join("route.log").display()
        ),
    )
    .expect("write the fake bot route");
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))
        .expect("make the fake bot route runnable");
    program
}

fn services(program: PathBuf, policy: Option<PathBuf>) -> Services {
    Services {
        bot: BotRoute { program, policy },
        ..Services::none()
    }
}

/// What the fake route was given: its policy, then each argument, one line each.
fn route_log(case: &Case) -> Option<String> {
    fs::read_to_string(case.root.join("route.log")).ok()
}

fn staged(dir: &Path) -> String {
    git(dir, &["diff", "--cached", "--name-only"])
}

#[test]
fn commit_stages_the_named_paths_and_commits_them_through_the_bot_route() {
    let mut case = Case::new();
    let dir = repository(&case, "r");
    fs::write(dir.join("a.txt"), "alpha, changed\n").expect("change a tracked file");
    fs::write(dir.join("b.txt"), "beta, changed\n").expect("change another tracked file");
    fs::create_dir_all(dir.join("src")).expect("create src");
    fs::write(dir.join("src/new.rs"), "pub fn new() {}\n").expect("write a new file");
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Commit]));
    let program = route(&case, None);
    let policy = case.root.join("policy.json");
    case.serve_with(services(program, Some(policy.clone())));

    let answer = ask(
        &socket,
        "commit",
        json!({"message": "feat: change a", "paths": ["a.txt", "src/new.rs"]}),
    );

    assert_eq!(answer.exit, 0, "{}", answer.text);
    let head = git(&dir, &["rev-parse", "--short=12", "HEAD"]);
    assert_eq!(answer.text, format!("{} 2 files\n", head.trim()));
    assert_eq!(
        git(&dir, &["log", "-1", "--format=%s"]).trim(),
        "feat: change a"
    );
    assert_eq!(
        git(&dir, &["show", "--name-only", "--format=", "HEAD"]),
        "a.txt\nsrc/new.rs\n"
    );
    assert_eq!(
        git(&dir, &["status", "--porcelain"]),
        " M b.txt\n",
        "a path not named stays uncommitted and unstaged"
    );
    let log = route_log(&case).expect("the bot route ran");
    assert!(
        log.starts_with(&format!("policy={}\n", policy.display())),
        "{log}"
    );
    assert!(
        log.contains(&format!(
            "arg=bot\narg=--repo\narg={}\narg=--\narg=commit\narg=-m\narg=feat: change a\narg=--\narg=a.txt\narg=src/new.rs\n",
            dir.display()
        )),
        "{log}"
    );
    let calls = case.recorder.calls();
    assert_eq!(calls[0].verb, Verb::Commit);
    assert_eq!(calls[0].outcome, CallOutcome::Done);
    assert_eq!(calls[0].paths, ["a.txt", "src/new.rs"]);
}

#[test]
fn a_path_outside_the_grant_refuses_the_commit_before_anything_is_staged() {
    let mut case = Case::new();
    let dir = repository(&case, "r");
    fs::create_dir_all(dir.join("src")).expect("create src");
    fs::write(dir.join("src/new.rs"), "pub fn new() {}\n").expect("write a new file");
    fs::write(dir.join("a.txt"), "alpha, changed\n").expect("change a tracked file");
    let socket = case.register(
        "example-org-a",
        INSTANCE,
        &dir,
        SessionGrant {
            paths: vec!["src".to_owned()],
            ..grant(&[Verb::Commit])
        },
    );
    let program = route(&case, None);
    case.serve_with(services(program, Some(case.root.join("policy.json"))));

    let answer = ask(
        &socket,
        "commit",
        json!({"message": "feat: x", "paths": ["src/new.rs", "a.txt"]}),
    );

    assert_eq!(answer.text, "NotGranted: paths: src\n");
    assert_eq!(answer.exit, 1);
    assert_eq!(staged(&dir), "", "nothing was staged");
    assert!(route_log(&case).is_none(), "the bot route never ran");
    let calls = case.recorder.calls();
    assert_eq!(calls[0].outcome, CallOutcome::NotGranted);
    assert_eq!(calls[0].rule.as_deref(), Some("paths: src"));
    assert_eq!(calls[0].paths, ["src/new.rs", "a.txt"]);
}

#[test]
fn a_path_outside_the_working_directory_is_refused() {
    let mut case = Case::new();
    let dir = repository(&case, "r");
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Commit]));
    let program = route(&case, None);
    case.serve_with(services(program, Some(case.root.join("policy.json"))));

    for outside in ["../elsewhere.txt", "/etc/hostname"] {
        let answer = ask(
            &socket,
            "commit",
            json!({"message": "feat: x", "paths": [outside]}),
        );

        assert_eq!(answer.exit, 1, "{outside}: {}", answer.text);
        assert!(
            answer.text.starts_with("NotGranted: paths: "),
            "{outside}: {}",
            answer.text
        );
    }
    assert!(route_log(&case).is_none(), "the bot route never ran");
}

#[test]
fn a_broker_without_a_gates_policy_refuses_every_commit_and_stages_nothing() {
    let mut case = Case::new();
    let dir = repository(&case, "r");
    fs::write(dir.join("a.txt"), "alpha, changed\n").expect("change a tracked file");
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Commit]));
    let program = route(&case, None);
    case.serve_with(services(program, None));

    let answer = ask(
        &socket,
        "commit",
        json!({"message": "feat: x", "paths": ["a.txt"]}),
    );

    assert_eq!(answer.exit, 1, "{}", answer.text);
    assert!(
        lines(&answer)[0].starts_with("CommitRefused: the broker holds no Gates policy"),
        "{}",
        answer.text
    );
    assert_eq!(staged(&dir), "");
    assert!(route_log(&case).is_none());
    assert_eq!(case.recorder.calls()[0].outcome, CallOutcome::Failed);
}

#[test]
fn a_commit_the_bot_route_refuses_names_its_reason_and_the_log() {
    let mut case = Case::new();
    let dir = repository(&case, "r");
    fs::write(dir.join("a.txt"), "alpha, changed\n").expect("change a tracked file");
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Commit]));
    let program = route(&case, Some("gates: a secret in a.txt"));
    case.serve_with(services(program, Some(case.root.join("policy.json"))));

    let answer = ask(
        &socket,
        "commit",
        json!({"message": "feat: x", "paths": ["a.txt"]}),
    );

    assert_eq!(answer.exit, 1, "{}", answer.text);
    let shown = lines(&answer);
    assert!(shown.len() <= FAILURE_LINES);
    assert_eq!(shown[0], "CommitRefused: gates: a secret in a.txt");
    assert!(shown.last().is_some_and(|line| line.starts_with("log: ")));
    assert_eq!(
        git(&dir, &["rev-list", "--count", "HEAD"]).trim(),
        "1",
        "nothing was committed"
    );
    assert_eq!(case.recorder.calls()[0].outcome, CallOutcome::Failed);
}

#[test]
fn a_commit_without_a_message_or_a_path_runs_nothing() {
    let mut case = Case::new();
    let dir = repository(&case, "r");
    let socket = case.register("example-org-a", INSTANCE, &dir, grant(&[Verb::Commit]));
    let program = route(&case, None);
    case.serve_with(services(program, Some(case.root.join("policy.json"))));

    let no_message = ask(
        &socket,
        "commit",
        json!({"message": "", "paths": ["a.txt"]}),
    );
    let no_path = ask(
        &socket,
        "commit",
        json!({"message": "feat: x", "paths": []}),
    );

    assert_eq!(no_message.exit, 1, "{}", no_message.text);
    assert_eq!(no_path.exit, 1, "{}", no_path.text);
    assert!(route_log(&case).is_none());
}
