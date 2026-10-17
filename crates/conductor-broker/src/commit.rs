//! `commit -m <message> -- <paths>` (`story:tool-build-commit-report`, design § 6.3). Every path
//! is checked against the grant before anything is staged: one outside the grant's paths or the
//! caller's working directory refuses the whole call `NotGranted`. Then the broker stages the
//! paths, new files included, and commits exactly them through the bot route, `b10x-gates bot`,
//! with the instance's Gates policy in `B10X_GATES_POLICY`, which only the broker holds. It
//! answers `<sha> <n> files`; a commit the route refuses is `CommitRefused`, naming the route's
//! first line, and the paths stay staged.

use std::path::Path;
use std::process::{Command, Output};

use serde_json::Value;

use crate::grant::Refusal;
use crate::verbs::{Context, failure};
use crate::wire::Answer;

pub(crate) fn run(context: &Context<'_>, args: &Value) -> Result<Answer, Refusal> {
    let refused = |reason: &str, output: &str| {
        failure(context.dir, &format!("CommitRefused: {reason}"), output)
    };
    let message = args["message"].as_str().unwrap_or_default();
    let paths: Vec<&str> = args["paths"]
        .as_array()
        .map(|paths| paths.iter().filter_map(Value::as_str).collect())
        .unwrap_or_default();
    if message.trim().is_empty() {
        return Ok(refused("the call names no message: write -m <message>", ""));
    }
    if paths.is_empty() {
        return Ok(refused("the call names no path: write them after --", ""));
    }
    let registration = context.registration;
    let workdir = &registration.workdir;
    let root = workdir.canonicalize().unwrap_or_else(|_| workdir.clone());
    let mut relative = Vec::with_capacity(paths.len());
    for path in &paths {
        let full = registration.grant.resolve(workdir, path)?;
        let inside = full
            .strip_prefix(&root)
            .map(|inside| inside.to_string_lossy().into_owned())
            .unwrap_or_default();
        relative.push(if inside.is_empty() {
            ".".to_owned()
        } else {
            inside
        });
    }
    let bot = &context.services.bot;
    let Some(policy) = &bot.policy else {
        return Ok(refused(
            "the broker holds no Gates policy: start conductor broker serve with \
             B10X_GATES_POLICY naming the instance's policy",
            "",
        ));
    };
    let before = head(workdir);
    let staged = git(workdir, &["add", "-A", "--"], &relative);
    match &staged {
        Ok(output) if output.status.success() => {}
        Ok(output) => {
            return Ok(refused(
                &format!("a path did not stage: {}", first_line(output)),
                &text(output),
            ));
        }
        Err(error) => return Ok(refused(&format!("git cannot run: {error}"), "")),
    }
    let committed = Command::new(&bot.program)
        .arg("bot")
        .arg("--repo")
        .arg(workdir)
        .args(["--", "commit", "-m", message, "--"])
        .args(&relative)
        .current_dir(workdir)
        .env("B10X_GATES_POLICY", policy)
        .output();
    let output = match committed {
        Ok(output) => output,
        Err(error) => {
            return Ok(refused(
                &format!(
                    "the bot route {} cannot run: {error}",
                    bot.program.display()
                ),
                "",
            ));
        }
    };
    if !output.status.success() {
        return Ok(refused(&first_line(&output), &text(&output)));
    }
    let after = head(workdir);
    if after.is_none() || after == before {
        return Ok(refused(
            "the bot route answered success and committed nothing",
            &text(&output),
        ));
    }
    let sha = git(workdir, &["rev-parse", "--short=12", "HEAD"], &[])
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .unwrap_or_default();
    let files = git(
        workdir,
        &[
            "diff-tree",
            "--no-commit-id",
            "--name-only",
            "-r",
            "--root",
            "HEAD",
        ],
        &[],
    )
    .ok()
    .filter(|output| output.status.success())
    .map_or(0, |output| {
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count()
    });
    Ok(Answer::done(format!("{sha} {files} files\n")))
}

/// `git -C <workdir> <args> <paths>`.
fn git(workdir: &Path, args: &[&str], paths: &[String]) -> std::io::Result<Output> {
    Command::new("git")
        .arg("-C")
        .arg(workdir)
        .args(["-c", "core.quotepath=off"])
        .args(args)
        .args(paths)
        .output()
}

/// The commit `HEAD` names, if any.
fn head(workdir: &Path) -> Option<String> {
    git(workdir, &["rev-parse", "--verify", "--quiet", "HEAD"], &[])
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

/// The first line that says something: of standard error, else of standard output, else the exit
/// status.
fn first_line(output: &Output) -> String {
    [&output.stderr, &output.stdout]
        .into_iter()
        .find_map(|bytes| {
            String::from_utf8_lossy(bytes)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_owned)
        })
        .unwrap_or_else(|| format!("it exited with {}", output.status))
}

fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned() + &String::from_utf8_lossy(&output.stdout)
}
