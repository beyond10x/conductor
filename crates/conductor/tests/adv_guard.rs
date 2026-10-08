//! Adversary, wave 04, U11 pass 1 (`story:guard-hook`): calls the guard lets through that its rule
//! table says it denies, and one wiring failure that the guard's own contract says is a denial.
//!
//! Every case decides recorded payloads (`tests/fixtures/guard/`) the way `tests/guard_rules.rs`
//! does: a home of its own under this test target's temporary directory, the probe repository's
//! checkout as the session's cwd, and only the `tool_input` field the rule reads replaced. Each
//! case lists every input it expected a verdict on and reports all that got another one, so one
//! run shows the whole gap; each also holds one control input decided the other way, so a guard
//! that denied everything would not pass it.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use conductor_cli::guard::{self, Decision};
use conductor_model::dispatch::Verdict;
use serde_json::{Map, Value};

/// The repository the probe session ran in.
const PROBE: &str = "~/example-org/.guard-probe-w04";

/// A home directory of its own for one case.
fn home(case: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("adv_guard")
        .join(case)
        .join("home");
    if dir.exists() {
        fs::remove_dir_all(&dir).expect("clear the case's home");
    }
    fs::create_dir_all(&dir).expect("create the case's home");
    dir
}

fn fixture(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/guard")
        .join(format!("{name}.json"));
    serde_json::from_str(&fs::read_to_string(path).expect("read the recorded payload"))
        .expect("the recorded payload is JSON")
}

fn under(home: &Path, value: &str) -> String {
    match value.strip_prefix("~/") {
        Some(rest) => format!("{}/{rest}", home.display()),
        None => value.to_owned(),
    }
}

fn localise(value: Value, home: &Path) -> Value {
    match value {
        Value::String(text) => Value::String(under(home, &text)),
        Value::Array(items) => Value::Array(items.into_iter().map(|v| localise(v, home)).collect()),
        Value::Object(fields) => Value::Object(
            fields
                .into_iter()
                .map(|(key, v)| (key, localise(v, home)))
                .collect::<Map<_, _>>(),
        ),
        other => other,
    }
}

/// The recorded payload `name` from the probe checkout, its `field` set to `value` as written (a
/// Bash command keeps its `~`; a file path's leading `~/` is put under `home`).
fn payload(name: &str, home: &Path, field: &str, value: &str) -> Value {
    let mut payload = localise(fixture(name), home);
    payload["cwd"] = Value::String(under(home, PROBE));
    payload["tool_input"][field] = Value::String(value.to_owned());
    payload
}

fn bash(home: &Path, command: &str) -> Decision {
    guard::decide(&payload("bash", home, "command", command), home)
}

/// Each `(input, decision)` whose decision is not `verdict` with a reason naming `because`.
fn misjudged<'a>(
    decisions: impl IntoIterator<Item = (&'a str, Decision)>,
    verdict: Verdict,
    because: &str,
) -> Vec<String> {
    decisions
        .into_iter()
        .filter(|(_, decision)| decision.verdict != verdict || !decision.reason.contains(because))
        .map(|(input, decision)| {
            format!("{input:?} -> {:?}: {}", decision.verdict, decision.reason)
        })
        .collect()
}

/// Asserts every Bash command in `denied` is denied for `because`, and `control` is allowed.
#[track_caller]
fn assert_bash_denied(case: &str, denied: &[&str], because: &str, control: &str) {
    let home = home(case);
    let allowed = bash(&home, control);
    assert_eq!(
        allowed.verdict,
        Verdict::Allow,
        "control {control:?}: {allowed:#?}"
    );
    let missed = misjudged(
        denied
            .iter()
            .map(|command| (*command, bash(&home, command))),
        Verdict::Deny,
        because,
    );
    assert!(
        missed.is_empty(),
        "{} of {} commands were not denied as {because:?}:\n{}",
        missed.len(),
        denied.len(),
        missed.join("\n")
    );
}

// -------------------------------------------------------------------------------------------------
// Bash, the `gh` write verbs.
// -------------------------------------------------------------------------------------------------

/// gh 2.100.0 lists `new` under ALIASES of `pr create`, `issue create` and `release create`
/// (`gh pr create --help`), and `gh pr new --help` prints "Create a pull request on GitHub."
#[test]
fn adv_gh_aliases_of_the_listed_write_verbs_are_github_writes() {
    assert_bash_denied(
        "gh-aliases",
        &[
            "gh pr new --fill",
            "gh issue new --title t --body b",
            "gh release new v1.0.0 --notes n",
        ],
        "GitHub write",
        "gh pr view 12",
    );
}

/// cobra takes a persistent flag between the command and its verb: `gh pr -R o/r merge --help`
/// prints "Merge a pull request on GitHub." (gh 2.100.0).
#[test]
fn adv_gh_write_verbs_with_a_flag_before_the_verb_are_github_writes() {
    assert_bash_denied(
        "gh-flag-before-verb",
        &[
            "gh pr -R o/r merge 12 --squash",
            "gh pr --repo=o/r merge 12",
            "gh pr -R o/r create --fill",
        ],
        "GitHub write",
        "gh pr -R o/r view 12",
    );
}

/// The story names seven verbs and `gh api` with a method other than GET; the brief says "`gh`
/// write verbs". These write to GitHub and pass. Whether the seven are the whole rule is the
/// coordinator's call; this case states the wider reading.
#[test]
fn adv_gh_writes_outside_the_story_s_seven_verbs_are_github_writes() {
    assert_bash_denied(
        "gh-unlisted-writes",
        &[
            "gh pr close 12",
            "gh pr edit 12 --add-label x",
            "gh pr review 12 --approve",
            "gh pr ready 12",
            "gh issue comment 5 --body x",
            "gh issue close 5",
            "gh issue edit 5 --title x",
            "gh workflow run ci.yml",
            "gh run cancel 123",
            "gh release delete v1.0.0 --yes",
            "gh release upload v1.0.0 a.tar.gz",
            "gh repo delete o/r --yes",
            "gh label create x",
            "gh secret set X --body y",
        ],
        "GitHub write",
        "gh issue view 5",
    );
}

// -------------------------------------------------------------------------------------------------
// Bash, accepted limits (correction 2, scope change): findings 4, 5, 6, 7, 8 and 11 are not
// fixed. These cases were written to show each gap; they now pin today's verdict, so a change
// to it is seen, and name where the gap is meant to close.
// -------------------------------------------------------------------------------------------------

/// The message every accepted-limit case carries.
const ACCEPTED_LIMIT: &str = "accepted limit of the Bash heuristic (design § 7: catches the common \
                              forms, not all); story:bash-sandbox";

/// Asserts every command in `allowed`, run from `cwd`, is allowed today.
#[track_caller]
fn assert_accepted_limit(case: &str, cwd: &str, allowed: &[&str]) {
    let home = home(case);
    let missed: Vec<String> = allowed
        .iter()
        .map(|command| {
            let mut payload = payload("bash", &home, "command", command);
            payload["cwd"] = Value::String(under(&home, cwd));
            (*command, guard::decide(&payload, &home))
        })
        .filter(|(_, decision)| decision.verdict != Verdict::Allow)
        .map(|(command, decision)| {
            format!("{command:?} -> {:?}: {}", decision.verdict, decision.reason)
        })
        .collect();
    assert!(
        missed.is_empty(),
        "{ACCEPTED_LIMIT}; today's verdict changed:\n{}",
        missed.join("\n")
    );
}

/// Finding 4: `shell.rs` drops a here-document's body as "not commands" and keeps a here-string
/// as a word, but a shell reading its script from standard input runs both.
#[test]
fn adv_a_shell_reading_its_script_from_a_heredoc_or_here_string_is_an_accepted_limit() {
    assert_accepted_limit(
        "shell-stdin-script",
        PROBE,
        &[
            "bash <<'EOF'\ngh pr merge 3\nEOF",
            "sh -s <<EOF\ngh pr create --fill\nEOF",
            "bash <<< 'gh pr merge 3'",
            "bash <<'EOF'\ncd ~/example-org/conductor && git push\nEOF",
            "sh <<< 'git -C ../conductor push'",
        ],
    );
}

/// Finding 5: `bash -ce 'script'` and `sh -ce 'script'` run `script` (bash and sh take the first
/// operand as the script when `c` is anywhere in the cluster); `argument_scripts` reads only a
/// cluster ending in `c`.
#[test]
fn adv_a_shell_flag_cluster_with_c_not_last_is_an_accepted_limit() {
    assert_accepted_limit(
        "shell-ce",
        PROBE,
        &[
            "bash -ce 'gh pr merge 3'",
            "sh -ce 'cd ../conductor && git push'",
        ],
    );
}

/// Finding 6: `env -C <dir>` (GNU coreutils `--chdir`) runs the command in `dir`: with `../b` a
/// git repository, `env -C ../b git rev-parse --show-toplevel` prints `b`'s path.
#[test]
fn adv_env_chdir_into_another_repository_is_an_accepted_limit() {
    assert_accepted_limit(
        "env-chdir",
        PROBE,
        &[
            "env -C ../conductor git push",
            "env --chdir=../conductor git status",
            "env -C ~/example-org/conductor git log",
        ],
    );
}

/// Finding 7: `GIT_DIR` and `GIT_WORK_TREE` are what `--git-dir` and `--work-tree` set: with
/// `../b` a git repository, `GIT_DIR=../b/.git git rev-parse --git-dir` prints `../b/.git`.
#[test]
fn adv_git_dir_and_work_tree_from_the_environment_are_an_accepted_limit() {
    assert_accepted_limit(
        "git-env",
        PROBE,
        &[
            "GIT_DIR=../conductor/.git git log",
            "GIT_DIR=~/example-org/conductor/.git GIT_WORK_TREE=~/example-org/conductor git status",
            "env GIT_DIR=../conductor/.git git push",
        ],
    );
}

/// Finding 8: `$PWD` and `$(pwd)` are the directory the command runs in, which the rule already
/// tracks as `here`; `word_path` gives up on any `$`, so the `..` after them is never compared.
#[test]
fn adv_pwd_relative_targets_are_an_accepted_limit() {
    assert_accepted_limit(
        "pwd-relative",
        PROBE,
        &[
            "cd \"$PWD/../conductor\" && git push",
            "git -C \"$PWD/../conductor\" status",
            "git -C \"$(pwd)/../conductor\" status",
        ],
    );
}

/// Finding 11: conductor's Bash rule denies only GitHub writes and writes to a settings file that
/// wires the guard, so a shell write edits conductor's other files although its Write, Edit and
/// NotebookEdit are held to its records. Its settings file left this limit with the
/// pre-publication review's finding 2: `tests/guard_rules.rs`,
/// `no_session_writes_a_guard_settings_file_through_bash`, denies it.
#[test]
fn adv_conductor_editing_its_own_files_through_bash_is_an_accepted_limit() {
    assert_accepted_limit(
        "conductor-bash-self-edit",
        "~/example-org/conductor",
        &[
            "sed -i 's/a/b/' crates/conductor/src/guard.rs",
            "echo x > crates/x",
        ],
    );
}

/// Pre-publication review, finding 2: the heredoc write finding 11 accepted for conductor's own
/// settings file is denied.
#[test]
fn adv_conductor_writing_its_settings_through_bash_is_denied() {
    let home = home("conductor-bash-settings");
    let mut payload = payload(
        "bash",
        &home,
        "command",
        "cat > .claude/conductor-settings.json <<'EOF'\n{}\nEOF",
    );
    payload["cwd"] = Value::String(under(&home, "~/example-org/conductor"));
    let decision = guard::decide(&payload, &home);
    assert_eq!(decision.verdict, Verdict::Deny, "{decision:#?}");
    assert!(decision.reason.contains("settings"), "{decision:#?}");
}

// -------------------------------------------------------------------------------------------------
// Write and Edit, the file that switches the guard off.
// -------------------------------------------------------------------------------------------------

/// Claude Code 2.1.292 drops every hook of the `--settings` file when the merged settings carry
/// `disableAllHooks: true`; its launcher says so: "a repo .claude/settings.json or
/// settings.local.json carries disableAllHooks:true — the child will drop every flagSettings
/// hook". Both files are inside the controller's checkout and its managed worktrees, so the file
/// rule lets the controller write the line that removes the guard.
#[test]
fn adv_a_controller_cannot_write_the_project_settings_that_disable_its_hooks() {
    let home = home("project-settings");
    let file = |name: &str, path: &str| {
        guard::decide(
            &payload(name, &home, "file_path", &under(&home, path)),
            &home,
        )
    };
    let control = file("write", "~/example-org/.guard-probe-w04/src/lib.rs");
    assert_eq!(control.verdict, Verdict::Allow, "control: {control:#?}");
    let targets = [
        "~/example-org/.guard-probe-w04/.claude/settings.local.json",
        "~/example-org/.guard-probe-w04/.claude/settings.json",
        "~/.local/state/worktree/trees/example-org/.guard-probe-w04/t/.claude/settings.local.json",
    ];
    let missed = misjudged(
        targets
            .iter()
            .flat_map(|path| [("write", *path), ("edit", *path)])
            .map(|(tool, path)| (path, file(tool, path))),
        Verdict::Deny,
        "",
    );
    assert!(
        missed.is_empty(),
        "a controller may write the settings that switch its guard off:\n{}",
        missed.join("\n")
    );
}

// -------------------------------------------------------------------------------------------------
// The wiring: every failure is a denial.
// -------------------------------------------------------------------------------------------------

/// `guard.rs` says everything that goes wrong before a verdict exists is a denial, because
/// Claude Code lets a call proceed on any exit status but 2. The wired command runs `conductor`
/// from `PATH` under `sh`; when the shell cannot find it, `sh` exits 127 and the call proceeds.
/// The control runs the same command with the built binary on `PATH` and gets 2.
#[test]
fn adv_the_wired_command_denies_when_conductor_cannot_be_run() {
    let settings: Value = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../.claude/controller-settings.json"),
        )
        .expect("read the controller settings"),
    )
    .expect("the settings are JSON");
    let command = settings["hooks"]["PreToolUse"][0]["hooks"][0]["command"]
        .as_str()
        .expect("the first PreToolUse hook's command")
        .to_owned();

    let home = home("wired-command");
    let checkout = home.join("example-org/.guard-probe-w04");
    fs::create_dir_all(&checkout).expect("create the probe checkout");
    let bin = Path::new(env!("CARGO_BIN_EXE_conductor"))
        .parent()
        .expect("the binary's directory")
        .to_owned();
    let empty = home.join("empty-path");
    fs::create_dir_all(&empty).expect("create an empty PATH directory");
    let stdin = serde_json::to_vec(&payload("send-message", &home, "to", "delta"))
        .expect("serialise the payload");

    let run = |path: &Path| {
        let mut child = Command::new("/bin/sh")
            .args(["-c", &command])
            .current_dir(&checkout)
            .env_clear()
            .env("HOME", &home)
            .env("PATH", path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("sh runs");
        // A shell that never reads its standard input may have exited already: a refused write
        // is that case, and the exit status below says what happened.
        let _ = std::io::Write::write_all(&mut child.stdin.take().expect("stdin"), &stdin);
        child.wait_with_output().expect("sh ends")
    };

    let wired = run(&bin);
    assert_eq!(
        wired.status.code(),
        Some(2),
        "control, conductor on PATH: {}",
        String::from_utf8_lossy(&wired.stderr)
    );
    let missing = run(&empty);
    assert_eq!(
        missing.status.code(),
        Some(2),
        "conductor not on PATH: the call proceeds; stderr: {}",
        String::from_utf8_lossy(&missing.stderr)
    );
}
