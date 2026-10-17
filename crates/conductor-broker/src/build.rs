//! The build verbs (`story:tool-build-commit-report`, design § 6.3): `test [<package>] [--filter
//! f]`, `check [<package>]` and `fmt [<package>]`. Each runs cargo in the caller's working
//! directory through the broker's [`Ledger`](crate::Ledger), which reserves for it, queues it and
//! runs it at nice 19 as `conductor build run --reserve auto` does; its whole output goes to a log
//! under the broker's `logs/`. The answer is one line on success; a failure is the error
//! `BuildFailed` on its first line, then what failed, at most [`FAILURE_LINES`] lines with the log.

use std::fs;
use std::path::Path;

use serde_json::Value;

use crate::services::{BuildEnd, BuildJob};
use crate::verbs::{Context, failure, log_path, logged_failure};
use crate::wire::Answer;
use crate::{FAILURE_LINES, Verb, verb_word};

/// The most lines one failing test shows: its name and place, its panic's first line, and the two
/// lines an `assert_eq!` adds (`left:`, `right:`).
const LINES_PER_TEST: usize = 4;

pub(crate) fn run(context: &Context<'_>, verb: Verb, args: &Value) -> Answer {
    let word = verb_word(verb);
    let command = match command(verb, args) {
        Ok(command) => command,
        Err(why) => return failure(context.dir, &format!("BuildFailed: {word}: {why}"), ""),
    };
    let log = match log_path(context.dir) {
        Ok(log) => log,
        Err(error) => {
            return failure(
                context.dir,
                &format!("BuildFailed: {word}: no log can be written: {error}"),
                "",
            );
        }
    };
    let job = BuildJob {
        session: context.session.to_owned(),
        instance: context.instance.to_owned(),
        command,
        cwd: context.registration.workdir.clone(),
        log: log.clone(),
    };
    match context.services.ledger.run(&job) {
        BuildEnd::Refused { reason } => {
            let head = format!("BuildFailed: {word}: the host ledger refused it: {reason}");
            let kept = fs::write(&log, format!("{}\n{head}\n", job.command.join(" ")));
            logged_failure(
                [head.as_str()],
                kept.map(|()| log.as_path()).map_err(|e| e.to_string()),
            )
        }
        BuildEnd::Ran { code } => {
            let output = fs::read(&log)
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_default();
            let summary = match verb {
                Verb::Test => summarize_test(code, &output),
                Verb::Check => summarize_check(code, &output),
                _ => summarize_fmt(code, &output),
            };
            match summary {
                Ok(line) => Answer::done(format!("{line}\n")),
                Err(lines) => logged_failure(lines.iter().map(String::as_str), Ok(log.as_path())),
            }
        }
    }
}

/// The command a build verb runs: cargo, in the working directory, with `-p <package>` when the
/// call names one. A package or filter that would read as an option is refused.
fn command(verb: Verb, args: &Value) -> Result<Vec<String>, String> {
    let word = |key: &str| -> Result<Option<String>, String> {
        match args[key].as_str().map(str::trim) {
            None | Some("") => Ok(None),
            Some(value) if value.starts_with('-') => Err(format!(
                "{key} {value:?} reads as an option; name it without a leading -"
            )),
            Some(value) => Ok(Some(value.to_owned())),
        }
    };
    let package = word("package")?;
    let mut command: Vec<String> = vec!["cargo".to_owned()];
    let push = |command: &mut Vec<String>, words: &[&str]| {
        command.extend(words.iter().map(|word| (*word).to_owned()));
    };
    match verb {
        Verb::Test => push(&mut command, &["test", "--color", "never"]),
        Verb::Check => push(&mut command, &["clippy", "--color", "never"]),
        _ => push(&mut command, &["fmt"]),
    }
    if let Some(package) = package {
        command.push("-p".to_owned());
        command.push(package);
    }
    match verb {
        Verb::Test => {
            if let Some(filter) = word("filter")? {
                command.push("--".to_owned());
                command.push(filter);
            }
        }
        Verb::Check => push(&mut command, &["--all-targets", "--", "-D", "warnings"]),
        _ => push(&mut command, &["--", "-l"]),
    }
    Ok(command)
}

/// What one `test result:` line of libtest counts: passed and failed.
fn counts(line: &str) -> Option<(u64, u64)> {
    let rest = line.trim().strip_prefix("test result: ")?;
    let number = |name: &str| -> u64 {
        rest.split(';')
            .filter_map(|part| {
                part.trim()
                    .trim_start_matches("ok. ")
                    .trim_start_matches("FAILED. ")
                    .strip_suffix(name)
            })
            .find_map(|count| count.trim().parse().ok())
            .unwrap_or(0)
    };
    Some((number(" passed"), number(" failed")))
}

/// The answer of `test` whose command exited `code` with `output`: `ok test <n> passed`, or the
/// failure's lines, the first naming the error.
fn summarize_test(code: i32, output: &str) -> Result<String, Vec<String>> {
    let (passed, failed) = output
        .lines()
        .filter_map(counts)
        .fold((0, 0), |(passed, failed), (p, f)| (passed + p, failed + f));
    if code == 0 {
        return Ok(format!("ok test {passed} passed"));
    }
    if failed == 0 {
        let mut lines = vec![format!(
            "BuildFailed: test: it did not build or run (exit {code})"
        )];
        lines.extend(diagnostics(output));
        return Err(lines);
    }
    let mut lines = vec![format!(
        "BuildFailed: test: {failed} failed, {passed} passed"
    )];
    lines.extend(failing_tests(output));
    Err(lines)
}

/// Each failing test of libtest's output: its name and where it panicked, then the first lines of
/// its panic, at most [`LINES_PER_TEST`] lines each.
fn failing_tests(output: &str) -> Vec<String> {
    let all: Vec<&str> = output.lines().collect();
    let mut shown = Vec::new();
    for (at, line) in all.iter().enumerate() {
        let Some(name) = line
            .strip_prefix("---- ")
            .and_then(|rest| rest.strip_suffix(" stdout ----"))
        else {
            continue;
        };
        let mut block = Vec::new();
        let mut place = String::new();
        for next in &all[at + 1..] {
            if next.starts_with("---- ") || next.trim() == "failures:" {
                break;
            }
            // `thread '<name>' panicked at <file>:<line>:<column>:`, or with the thread's id,
            // `thread '<name>' (<id>) panicked at …`; the message on the lines after it.
            if let Some(rest) = next.strip_prefix("thread '") {
                if let Some((_, at)) = rest.split_once(" panicked at ") {
                    place = at.trim_end_matches(':').to_owned();
                }
                continue;
            }
            if next.starts_with("note: ") || next.trim().is_empty() {
                if block.is_empty() {
                    continue;
                }
                break;
            }
            if !place.is_empty() {
                block.push(format!("  {}", next.trim_end()));
            }
            if block.len() + 1 >= LINES_PER_TEST {
                break;
            }
        }
        if place.is_empty() {
            shown.push(format!("FAILED {name}"));
        } else {
            shown.push(format!("FAILED {name} at {place}"));
        }
        shown.extend(block);
    }
    if shown.is_empty() {
        // No `---- <name> stdout ----` block: name the tests libtest lists as failing.
        let mut listing = false;
        for line in output.lines() {
            if line.trim() == "failures:" {
                listing = true;
                continue;
            }
            if listing {
                match line.strip_prefix("    ") {
                    Some(name) if !name.trim().is_empty() => {
                        shown.push(format!("FAILED {}", name.trim()))
                    }
                    _ if line.trim().is_empty() => {}
                    _ => listing = false,
                }
            }
        }
    }
    shown
}

/// The compiler's and the linter's errors and warnings in `output`, each once, on one line with
/// the place its ` --> ` line names; cargo's own closing lines are left out.
fn diagnostics(output: &str) -> Vec<String> {
    const CLOSING: [&str; 6] = [
        "error: could not compile",
        "error: aborting due to",
        "error: test failed",
        "warning: build failed",
        "error: process didn't exit successfully",
        "error: Recipe",
    ];
    let lines: Vec<&str> = output.lines().collect();
    let mut found = Vec::new();
    for (at, line) in lines.iter().enumerate() {
        let opens = line.starts_with("error") || line.starts_with("warning");
        if !opens || !line.contains(':') || CLOSING.iter().any(|closing| line.starts_with(closing))
        {
            continue;
        }
        if line.starts_with("warning: `") && line.contains("generated") {
            continue;
        }
        let place = lines[at + 1..]
            .iter()
            .take(3)
            .find_map(|next| next.trim_start().strip_prefix("--> "));
        let shown = match place {
            Some(place) => format!("{line} --> {}", place.trim()),
            None => (*line).to_owned(),
        };
        // The library and its test target report the same error once each.
        if !found.contains(&shown) {
            found.push(shown);
        }
        if found.len() >= FAILURE_LINES {
            break;
        }
    }
    found
}

/// The answer of `check` whose command exited `code` with `output`: `ok check`, or its errors.
fn summarize_check(code: i32, output: &str) -> Result<String, Vec<String>> {
    if code == 0 {
        return Ok("ok check".to_owned());
    }
    let found = diagnostics(output);
    let errors = found
        .iter()
        .filter(|line| line.starts_with("error"))
        .count();
    let mut lines = vec![format!("BuildFailed: check: {errors} errors (exit {code})")];
    lines.extend(found);
    Err(lines)
}

/// The answer of `fmt` whose command exited `code` with `output`: `ok fmt <n> files changed`,
/// counting the files rustfmt names (`-l`), or its errors.
fn summarize_fmt(code: i32, output: &str) -> Result<String, Vec<String>> {
    if code == 0 {
        let changed = output
            .lines()
            .filter(|line| {
                Path::new(line.trim())
                    .extension()
                    .is_some_and(|ext| ext == "rs")
            })
            .count();
        return Ok(format!("ok fmt {changed} files changed"));
    }
    let mut lines = vec![format!("BuildFailed: fmt: rustfmt failed (exit {code})")];
    lines.extend(output.lines().map(str::to_owned));
    Err(lines)
}
