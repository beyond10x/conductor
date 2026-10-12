//! `conductor gc` (`story:conductor-gc`): removes what the dashboard's disk panel marks
//! reclaimable, and nothing else.
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `gc`, its input `conductor.observation.CollectGarbage` and its rows
//! `conductor.observation.GarbageRow`.
//!
//! It measures the tracked repositories over the panel's own sources ([`Sources::active`])
//! through the panel's own classification ([`waste::repositories`]), so it lists and removes
//! exactly the rows the panel marks reclaimable: the build cache of idle managed trees, the trees
//! eligible for removal and a checkout's cargo `target/`, none of them where a live session works.
//! The panel's module documentation says what keeps every other row. With `--dry-run` it prints
//! those rows with the bytes the panel measures and removes nothing.
//!
//! Otherwise it takes the rows one by one. It reads the live sessions again and makes every check
//! the listing made again ([`Reclaim::recheck`]): it refuses a row one of whose trees a live session
//! now works in, and a checkout's `target/` when a live session or any process on the host now
//! works in the checkout, or the target is no longer cargo's. Else it runs the row's commands
//! ([`Reclaim::commands`]: `worktree discard-cache`, `worktree gc --apply` or `cargo clean`), each
//! to its end without a time limit, so no removal is stopped half-way, and never deletes a path
//! itself. The bytes freed are the bytes under the row's paths before less the bytes after, both
//! walked by [`disk::tree_size`]. A row is removed when every command exited 0 without answering
//! `"ok": false` and every path its removal must take is gone; else it is refused, naming the
//! first failure on standard output and on one line of standard error.
//!
//! It refuses as a whole, before it lists or removes anything, when the live sessions cannot be
//! read: a tree a live session works in could not be told from an idle one.
//!
//! It exits 0 when every row was listed or removed, and 1 when a row was refused or the command
//! refused.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};

use anyhow::{Result, anyhow, bail};
use conductor_model::observation::{CollectGarbage, GarbageOutcome, GarbageRow};
use serde_json::{Value, json};

use crate::cli::GcArgs;
use crate::dashboard::Sources;
use crate::dashboard::waste::{self, Reclaim, Row};
use crate::disk;

const COMMAND: &str = "gc";

/// `conductor gc`: lists, with `--dry-run`, or removes the rows the disk panel marks reclaimable,
/// and prints them as text or, with `--format json`, one JSON object; the last text line of a
/// removal is `freed <n> bytes`.
///
/// # Errors
///
/// `--format` is neither text nor json, `HOME` is not an absolute path, the working directory
/// cannot be read, the live sessions cannot be read, or the rows cannot be printed.
pub fn gc(args: &GcArgs) -> Result<ExitCode> {
    let input = CollectGarbage {
        dry_run: args.dry_run,
        format: args.format.clone(),
    };
    let json = match input.format.as_deref() {
        None | Some("text") => false,
        Some("json") => true,
        Some(other) => bail!("{COMMAND}: --format {other:?} is neither text nor json"),
    };
    let dry_run = input.dry_run.unwrap_or(false);
    let sources = Sources::active(None).map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
    let measured = waste::repositories(&sources);
    if let Err(error) = &measured.live {
        bail!("{COMMAND}: the live sessions could not be read, so nothing is removed: {error}");
    }
    let rows: Vec<GarbageRow> = measured
        .rows
        .iter()
        .filter_map(|row| {
            let reclaim = row.reclaim.as_ref()?;
            Some(if dry_run {
                listed(row, reclaim)
            } else {
                remove(&sources, row, reclaim)
            })
        })
        .collect();
    print(&rows, dry_run, json)?;
    let mut refused = false;
    let mut err = io::stderr().lock();
    for row in rows
        .iter()
        .filter(|row| row.outcome == GarbageOutcome::Refused)
    {
        refused = true;
        writeln!(
            err,
            "conductor: {COMMAND}: refused {} of {}: {}",
            row.what,
            row.repository.as_deref().unwrap_or("-"),
            row.detail.as_deref().unwrap_or_default()
        )?;
    }
    Ok(if refused {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    })
}

/// The reclaimable `row`, as the panel measured it, before anything is removed.
fn listed(row: &Row, reclaim: &Reclaim) -> GarbageRow {
    GarbageRow {
        repository: row.repository.clone(),
        what: row.what.to_owned(),
        paths: reclaim
            .paths()
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        count: row.count.and_then(|count| i64::try_from(count).ok()),
        bytes: row.bytes.and_then(|bytes| i64::try_from(bytes).ok()),
        freed: None,
        outcome: GarbageOutcome::Reclaimable,
        detail: None,
    }
}

/// Removes `row` through `reclaim`, unless a live session now works in one of its paths, and
/// answers what became of it and the bytes it freed.
fn remove(sources: &Sources, row: &Row, reclaim: &Reclaim) -> GarbageRow {
    let mut out = listed(row, reclaim);
    let paths = reclaim.paths();
    // A session or a process may have started where the row is since the listing, or the
    // target may have changed: every check the listing made is made again.
    let busy = match waste::live(sources) {
        Err(error) => Some(format!(
            "the live sessions could not be read again: {error}"
        )),
        Ok(live) => reclaim.recheck(&live),
    };
    if let Some(detail) = busy {
        out.freed = Some(0);
        out.outcome = GarbageOutcome::Refused;
        out.detail = Some(detail);
        return out;
    }
    let before = allocated(&paths);
    let mut failure = None;
    for (command, dir) in reclaim.commands() {
        if let Err(error) = run(&command, dir.as_deref()) {
            failure.get_or_insert(error);
        }
    }
    let after = allocated(&paths);
    out.freed = i64::try_from(before.saturating_sub(after)).ok();
    let failure = failure.or_else(|| {
        reclaim
            .removed()
            .iter()
            .find(|path| fs::symlink_metadata(path).is_ok())
            .map(|path| format!("{} is still there", path.display()))
    });
    match failure {
        Some(detail) => {
            out.outcome = GarbageOutcome::Refused;
            out.detail = Some(detail);
        }
        None => out.outcome = GarbageOutcome::Removed,
    }
    out
}

/// The bytes allocated under `paths` together; a path that is gone adds nothing.
fn allocated(paths: &[PathBuf]) -> u64 {
    disk::tree_size(paths, None).map_or(0, |size| size.allocated)
}

/// Runs `command` in `dir`, if any, with no input, to its end: nothing when it exited 0 and, if
/// it printed a JSON object, that object does not answer `"ok": false`; else the failure.
fn run(command: &[OsString], dir: Option<&Path>) -> Result<(), String> {
    let shown = command
        .iter()
        .map(|word| word.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    let Some((program, arguments)) = command.split_first() else {
        return Err("no command".to_owned());
    };
    let mut process = Command::new(program);
    process.args(arguments).stdin(Stdio::null());
    if let Some(dir) = dir {
        process.current_dir(dir);
    }
    let output = process
        .output()
        .map_err(|error| format!("`{shown}`: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "`{shown}`: {}: {}",
            output.status,
            stderr.lines().next().unwrap_or_default().trim()
        ));
    }
    let answer: Option<Value> = serde_json::from_slice(&output.stdout).ok();
    if answer.is_some_and(|answer| answer["ok"] == false) {
        return Err(format!("`{shown}` answered \"ok\": false"));
    }
    Ok(())
}

fn outcome(outcome: GarbageOutcome) -> &'static str {
    match outcome {
        GarbageOutcome::Reclaimable => "reclaimable",
        GarbageOutcome::Removed => "removed",
        GarbageOutcome::Refused => "refused",
    }
}

fn number(value: Option<i64>) -> String {
    value.map_or_else(|| "?".to_owned(), |value| value.to_string())
}

/// Prints `rows` on standard output: one line each and a total, or one JSON object.
fn print(rows: &[GarbageRow], dry_run: bool, json: bool) -> Result<()> {
    let reclaimable: i64 = rows.iter().filter_map(|row| row.bytes).sum();
    let freed: i64 = rows.iter().filter_map(|row| row.freed).sum();
    let mut out = io::stdout().lock();
    if json {
        let rows: Vec<Value> = rows
            .iter()
            .map(|row| {
                json!({
                    "repository": row.repository,
                    "what": row.what,
                    "paths": row.paths,
                    "count": row.count,
                    "bytes": row.bytes,
                    "freed": row.freed,
                    "outcome": outcome(row.outcome),
                    "detail": row.detail,
                })
            })
            .collect();
        let report = json!({
            "dry_run": dry_run,
            "rows": rows,
            "reclaimable_bytes": reclaimable,
            "freed_bytes": (!dry_run).then_some(freed),
        });
        writeln!(out, "{report}")?;
        out.flush()?;
        return Ok(());
    }
    for row in rows {
        let size = if dry_run {
            format!("{} bytes", number(row.bytes))
        } else {
            format!("{} bytes freed of {}", number(row.freed), number(row.bytes))
        };
        let detail = row
            .detail
            .as_deref()
            .map(|detail| format!(": {detail}"))
            .unwrap_or_default();
        writeln!(
            out,
            "{}  {size}  {}  {}  {}{detail}",
            outcome(row.outcome),
            row.repository.as_deref().unwrap_or("-"),
            row.what,
            row.paths.join(" "),
        )?;
    }
    if dry_run {
        writeln!(
            out,
            "{reclaimable} bytes reclaimable in {} rows; --dry-run removed nothing",
            rows.len()
        )?;
    } else {
        writeln!(out, "freed {freed} bytes")?;
    }
    out.flush()?;
    Ok(())
}
