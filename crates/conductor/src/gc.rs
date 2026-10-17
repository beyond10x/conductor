//! `conductor gc` (`story:conductor-gc`): removes what the dashboard's disk panel marks
//! reclaimable and strips build output from worktree archives through `worktree`, and nothing
//! else.
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `gc`, its input `conductor.observation.CollectGarbage` and its rows
//! `conductor.observation.GarbageRow`.
//!
//! It measures the tracked repositories over the panel's own sources ([`Sources::active`])
//! through the panel's own classification ([`waste::repositories`]), so it lists and removes
//! exactly the rows the panel marks reclaimable: the build cache of idle managed trees, the trees
//! eligible for removal and a checkout's cargo `target/`, none of them where a live session works.
//! The panel's module documentation says what keeps every other row. Beside them it lists the
//! build output `worktree` would strip from the instance's worktree archives
//! (`story:gc-before-pause`), read archive by archive from `worktree prune-archives
//! --strip-build-output`'s own dry run, which writes nothing below the archive root
//! ([`archives`], `story:gc-archive-measure`). Each archive's size is printed as soon as it is
//! known, as one `archive` line, and cached by its id and modification time under the state
//! directory, so an unchanged archive is not walked again; when the bound
//! (`CONDUCTOR_GC_ARCHIVE_BOUND` seconds, 1800 by default) is reached, the row sums the archives
//! measured and says `partial: <n> of <m> archives`. The archive strip needs a git checkout
//! under the checkouts root, whose `worktree` profile is read; without one it is kept, not
//! measured. With `--dry-run` it prints those rows with the bytes measured and removes nothing;
//! `worktree prune-archives --strip-build-output --apply` runs only without it.
//!
//! Beside them it lists the instance's managed trees ([`trees`], `story:gc-finished-trees`): each
//! finished tree with its whole size, which it takes back through `worktree finish` and `worktree
//! gc --apply`; each stale tree with the size of its cargo `target/`, which it removes and keeps
//! the tree (`stale, kept`); every other tree as a holder, with why. A tree it takes back leaves
//! the panel's cache row, and a tree the panel lists eligible for removal is left to that row. It
//! ends with one `trees  <repository>: …` line per repository: trees finished, stale and kept, each
//! with GiB (`trees` in JSON).
//!
//! After the rows it prints one table of the holders it measures and never reclaims: the panel's
//! shared rows ([`waste::shared_rows`]: shared build targets, sccache, the rustup toolchains, the
//! Go build cache, the agents' scratch and `/var/tmp`), and the archive strip when it could not be
//! listed or has nothing to strip, each with its path, its bytes and why it is kept.
//!
//! Otherwise it takes the rows one by one. It reads the live sessions again and makes every check
//! the listing made again ([`Reclaim::recheck`]): it refuses a row one of whose trees a live session
//! now works in, and a checkout's `target/` when a live session or any process on the host now
//! works in the checkout, or the target is no longer cargo's. Else it runs the row's commands
//! ([`Reclaim::commands`]: `worktree discard-cache`, `worktree gc --apply`, `cargo clean` or
//! `worktree prune-archives --strip-build-output --apply`), each
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
//! refused. A partial archive row is listed, and exits 0.

mod archives;
pub mod reclaim;
pub(crate) mod trees;

use std::collections::{BTreeMap, HashSet};
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::Duration;

use anyhow::{Result, anyhow, bail};
use conductor_model::observation::{CollectGarbage, GarbageOutcome, GarbageRow, TreeVerdict};
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
/// `--format` is neither text nor json, `CONDUCTOR_GC_ARCHIVE_BOUND` is not a whole number of
/// seconds, `HOME` is not an absolute path, the working directory cannot be read, the live
/// sessions cannot be read, or the rows cannot be printed.
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
    let bound = archives::bound().map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
    let sources = Sources::active(None).map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
    let measured = waste::repositories(&sources);
    if let Err(error) = &measured.live {
        bail!("{COMMAND}: the live sessions could not be read, so nothing is removed: {error}");
    }
    let state = crate::state::dir(None).map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
    let strip = archive_strip(&sources, &state.join(archives::CACHE), bound, json)?;
    let mut listings: Vec<(GarbageRow, Reclaim)> = measured
        .rows
        .iter()
        .filter_map(|row| {
            let reclaim = row.reclaim.clone()?;
            Some((listed(row, &reclaim), reclaim))
        })
        .collect();
    // The holders gc names and does not reclaim: a strip that could not be listed or has
    // nothing to strip, then the panel's shared rows, measured once the rows above are done.
    let mut holders = Vec::new();
    match strip.listed {
        Ok(listing) => listings.push(listing),
        Err(holder) => holders.push(holder),
    }
    // The managed trees (`story:gc-finished-trees`): a tree the panel lists eligible for removal
    // is left to that row, and a tree this pass takes back leaves the panel's cache row.
    let skip: HashSet<PathBuf> = listings
        .iter()
        .filter_map(|(_, reclaim)| match reclaim {
            Reclaim::RemoveTrees { trees, .. } => Some(trees.clone()),
            _ => None,
        })
        .flatten()
        .collect();
    let mut summary: BTreeMap<String, Summary> = BTreeMap::new();
    let mut tree_rows = Vec::new();
    match trees::assess(&sources, tree_stale(), &skip, true) {
        Ok(assessed) => {
            for tree in assessed {
                summary
                    .entry(tree.repository.clone())
                    .or_default()
                    .add(&tree);
                match tree_row(&tree) {
                    Ok(row) => tree_rows.push((row, tree)),
                    Err(holder) => holders.push(holder),
                }
            }
        }
        Err(error) => holders.push(Holder {
            repository: None,
            what: trees::KEPT.to_owned(),
            path: Some(sources.trees.display().to_string()),
            bytes: None,
            kept: format!("not assessed: {error}"),
        }),
    }
    let claimed: Vec<&trees::Tree> = tree_rows.iter().map(|(_, tree)| tree).collect();
    let listings = unclaimed(listings, &claimed);
    let mut rows: Vec<GarbageRow> = listings
        .into_iter()
        .map(|(row, reclaim)| {
            if dry_run {
                row
            } else {
                remove(&sources, row, &reclaim)
            }
        })
        .collect();
    rows.extend(tree_rows.into_iter().map(|(row, tree)| {
        if dry_run {
            row
        } else {
            take(&sources, row, &tree)
        }
    }));
    let shared = waste::shared_rows(&sources);
    holders.extend(shared.iter().map(Holder::of));
    print(&rows, &holders, &strip.measured, &summary, dry_run, json)?;
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

/// `thresholds.tree_stale` of the active instance.
fn tree_stale() -> Duration {
    let thresholds = &crate::config::active().instance.thresholds;
    Duration::from_secs(
        crate::config::seconds(&thresholds.tree_stale)
            .unwrap_or(crate::config::TREE_STALE_HOURS * 3600),
    )
}

/// One repository's managed trees, counted and summed by verdict: (trees, bytes).
#[derive(Debug, Default)]
struct Summary {
    finished: (u64, u64),
    stale: (u64, u64),
    stale_target: u64,
    kept: (u64, u64),
}

impl Summary {
    fn add(&mut self, tree: &trees::Tree) {
        let bytes = tree.bytes.unwrap_or(0);
        let slot = match tree.verdict {
            trees::Verdict::Finished => &mut self.finished,
            trees::Verdict::Stale(_) => {
                self.stale_target += tree.target_bytes.unwrap_or(0);
                &mut self.stale
            }
            trees::Verdict::Kept(_) => &mut self.kept,
        };
        slot.0 += 1;
        slot.1 += bytes;
    }
}

/// The row of a tree gc takes back, before anything is removed; or the holder of a tree it keeps
/// (a stale one without cargo's `target/` among them).
fn tree_row(tree: &trees::Tree) -> Result<GarbageRow, Holder> {
    let row = |what: &str, path: PathBuf, bytes: Option<u64>, verdict, detail| GarbageRow {
        repository: Some(tree.repository.clone()),
        what: what.to_owned(),
        paths: vec![path.display().to_string()],
        count: Some(1),
        bytes: bytes.and_then(|bytes| i64::try_from(bytes).ok()),
        freed: None,
        outcome: GarbageOutcome::Reclaimable,
        detail,
        tree: Some(tree.id.clone()),
        verdict: Some(verdict),
    };
    let holder = |kept: String| Holder {
        repository: Some(tree.repository.clone()),
        what: trees::KEPT.to_owned(),
        path: Some(tree.path.display().to_string()),
        bytes: tree.bytes,
        kept,
    };
    match &tree.verdict {
        trees::Verdict::Finished => Ok(row(
            trees::FINISHED,
            tree.path.clone(),
            tree.bytes,
            TreeVerdict::Finished,
            None,
        )),
        trees::Verdict::Stale(why) if tree.target_bytes.is_some() => Ok(row(
            trees::STALE,
            tree.path.join("target"),
            tree.target_bytes,
            TreeVerdict::Stale,
            Some(format!("stale, kept: {why}")),
        )),
        trees::Verdict::Stale(why) => Err(holder(format!(
            "stale, kept: {why}; no cargo target/ to remove"
        ))),
        trees::Verdict::Kept(why) => Err(holder(why.clone())),
    }
}

/// `listings` without the trees `claimed` takes back: they leave the panel's cache rows, whose
/// bytes lose each one's `target/`; a row left with no tree goes.
fn unclaimed(
    listings: Vec<(GarbageRow, Reclaim)>,
    claimed: &[&trees::Tree],
) -> Vec<(GarbageRow, Reclaim)> {
    listings
        .into_iter()
        .filter_map(|(mut row, reclaim)| {
            let Reclaim::DiscardCache(cached) = reclaim else {
                return Some((row, reclaim));
            };
            let (gone, kept): (Vec<PathBuf>, Vec<PathBuf>) = cached
                .into_iter()
                .partition(|path| claimed.iter().any(|tree| &tree.path == path));
            if kept.is_empty() {
                return None;
            }
            let less: u64 = gone
                .iter()
                .map(|path| allocated(&[path.join("target")]))
                .sum();
            row.bytes = row
                .bytes
                .map(|bytes| bytes.saturating_sub(i64::try_from(less).unwrap_or(0)));
            row.count = i64::try_from(kept.len()).ok();
            row.paths = kept.iter().map(|path| path.display().to_string()).collect();
            Some((row, Reclaim::DiscardCache(kept)))
        })
        .collect()
}

/// Takes the listed tree row `out` back through [`trees::take`].
fn take(sources: &Sources, mut out: GarbageRow, tree: &trees::Tree) -> GarbageRow {
    let signed = |bytes: u64| i64::try_from(bytes).ok();
    match trees::take(sources, tree) {
        Ok(taken) => {
            out.outcome = GarbageOutcome::Removed;
            out.freed = signed(taken.freed);
            if let Some(note) = taken.note {
                out.detail = Some(match out.detail.take() {
                    Some(detail) => format!("{detail}; {note}"),
                    None => note,
                });
            }
        }
        Err((why, freed)) => {
            out.outcome = GarbageOutcome::Refused;
            out.freed = signed(freed);
            out.detail = Some(why);
        }
    }
    out
}

/// A holder gc measures and never reclaims, with why it is kept.
struct Holder {
    repository: Option<String>,
    what: String,
    path: Option<String>,
    bytes: Option<u64>,
    kept: String,
}

impl Holder {
    /// The panel's row `row`, which gc keeps.
    fn of(row: &Row) -> Self {
        Self {
            repository: row.repository.clone(),
            what: row.what.to_owned(),
            path: row.path.clone(),
            bytes: row.bytes,
            kept: kept_because(row),
        }
    }
}

/// The archive strip: its reclaimable row and how gc strips it, or the holder gc keeps; and each
/// archive measured.
struct ArchiveStrip {
    listed: Result<(GarbageRow, Reclaim), Holder>,
    measured: Vec<archives::Measured>,
}

/// The build output `worktree` would strip from the instance's archives, measured archive by
/// archive within `bound` through the cache `cache` ([`archives::measure`]); in text, each
/// archive's line is printed as soon as it is known. The row sums the strippable archives, and
/// says `partial: <n> of <m> archives` when the bound or a failure left some unmeasured; with
/// nothing to strip, or no checkout whose profile `worktree` reads, the strip is kept.
fn archive_strip(
    sources: &Sources,
    cache: &Path,
    bound: Duration,
    json: bool,
) -> Result<ArchiveStrip> {
    let root = sources.home.join(".local/state/worktree/archives");
    let holder = |bytes: Option<u64>, kept: String| Holder {
        repository: None,
        what: archives::WHAT.to_owned(),
        path: Some(root.display().to_string()),
        bytes,
        kept,
    };
    let Some(checkout) = waste::profile_checkout(&sources.checkouts) else {
        let kept = format!(
            "not measured (failed: no git checkout under the checkouts root {} names the \
             worktree profile)",
            sources.checkouts.display()
        );
        return Ok(ArchiveStrip {
            listed: Err(holder(None, kept)),
            measured: Vec::new(),
        });
    };
    let found = archives::list(&root, &sources.trees, &sources.checkouts);
    let mut out = io::stdout().lock();
    let mut unprinted = None;
    let strip = archives::measure(&checkout, &found, cache, bound, &mut |archive| {
        if json || unprinted.is_some() {
            return;
        }
        let printed = writeln!(
            out,
            "archive  {} bytes  {}  {}: {}{}",
            archive.strip_bytes,
            archive.id,
            archive.path.display(),
            archive.verdict,
            if archive.cached { " (cached)" } else { "" },
        )
        .and_then(|()| out.flush());
        if let Err(error) = printed {
            unprinted = Some(error);
        }
    });
    drop(out);
    if let Some(error) = unprinted {
        return Err(error.into());
    }
    if let Some(error) = &strip.unsaved {
        writeln!(
            io::stderr().lock(),
            "conductor: {COMMAND}: the archive sizes could not be cached: {error}"
        )?;
    }
    let partial = strip.partial();
    let strippable: Vec<&archives::Measured> = strip
        .measured
        .iter()
        .filter(|archive| archive.verdict == "strippable")
        .collect();
    let listed = if strip.measured.is_empty() && strip.total > 0 {
        Err(holder(
            None,
            format!("not measured ({})", partial.unwrap_or_default()),
        ))
    } else if strippable.is_empty() {
        let kept = match partial {
            Some(partial) => format!("nothing to strip ({partial})"),
            None => "nothing to strip".to_owned(),
        };
        Err(holder(Some(0), kept))
    } else {
        let paths: Vec<PathBuf> = strippable
            .iter()
            .map(|archive| archive.path.clone())
            .collect();
        let bytes: u64 = strippable.iter().map(|archive| archive.strip_bytes).sum();
        let row = GarbageRow {
            repository: None,
            what: archives::WHAT.to_owned(),
            paths: paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            count: i64::try_from(paths.len()).ok(),
            bytes: i64::try_from(bytes).ok(),
            freed: None,
            outcome: GarbageOutcome::Reclaimable,
            detail: partial,
            tree: None,
            verdict: None,
        };
        Ok((
            row,
            Reclaim::StripArchives {
                checkout,
                archives: paths,
            },
        ))
    };
    Ok(ArchiveStrip {
        listed,
        measured: strip.measured,
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
        tree: None,
        verdict: None,
    }
}

/// Removes the listed row `out` through `reclaim`, unless a live session now works in one of its
/// paths, and answers what became of it and the bytes it freed.
fn remove(sources: &Sources, mut out: GarbageRow, reclaim: &Reclaim) -> GarbageRow {
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

/// Why `conductor gc` keeps the holder `row`, with its measurement's state when that is not
/// `measured`.
fn kept_because(row: &Row) -> String {
    let why = row.gc.strip_prefix("kept: ").unwrap_or(row.gc.as_str());
    if row.state == "measured" {
        why.to_owned()
    } else {
        format!("{why} ({})", row.state)
    }
}

/// Prints `rows` and `holders` on standard output: one line each, the holders under a header of
/// their own, and a total; or one JSON object, which also carries each archive measured,
/// `archives` (in text their lines were printed as they were measured).
fn print(
    rows: &[GarbageRow],
    holders: &[Holder],
    archives: &[archives::Measured],
    summary: &BTreeMap<String, Summary>,
    dry_run: bool,
    json: bool,
) -> Result<()> {
    let reclaimable: i64 = rows.iter().filter_map(|row| row.bytes).sum();
    let freed: i64 = rows.iter().filter_map(|row| row.freed).sum();
    let mut out = io::stdout().lock();
    if json {
        let holders: Vec<Value> = holders
            .iter()
            .map(|row| {
                json!({
                    "repository": row.repository,
                    "what": row.what,
                    "path": row.path,
                    "bytes": row.bytes,
                    "kept": row.kept,
                })
            })
            .collect();
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
                    "tree": row.tree,
                    "verdict": row.verdict.map(|verdict| match verdict {
                        TreeVerdict::Finished => "finished",
                        TreeVerdict::Stale => "stale",
                    }),
                })
            })
            .collect();
        let archives: Vec<Value> = archives
            .iter()
            .map(|archive| {
                json!({
                    "id": archive.id,
                    "path": archive.path.display().to_string(),
                    "verdict": archive.verdict,
                    "strip_bytes": archive.strip_bytes,
                    "cached": archive.cached,
                })
            })
            .collect();
        let report = json!({
            "dry_run": dry_run,
            "rows": rows,
            "reclaimable_bytes": reclaimable,
            "freed_bytes": (!dry_run).then_some(freed),
            "holders": holders,
            "archives": archives,
            "trees": summary
                .iter()
                .map(|(repository, sum)| {
                    json!({
                        "repository": repository,
                        "finished": sum.finished.0,
                        "finished_bytes": sum.finished.1,
                        "stale": sum.stale.0,
                        "stale_bytes": sum.stale.1,
                        "stale_target_bytes": sum.stale_target,
                        "kept": sum.kept.0,
                        "kept_bytes": sum.kept.1,
                    })
                })
                .collect::<Vec<Value>>(),
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
    if !holders.is_empty() {
        writeln!(out, "measured and not reclaimed by gc:")?;
    }
    for row in holders {
        writeln!(
            out,
            "kept  {} bytes  {}  {}  {}: {}",
            row.bytes
                .map_or_else(|| "?".to_owned(), |bytes| bytes.to_string()),
            row.repository.as_deref().unwrap_or("-"),
            row.what,
            row.path.as_deref().unwrap_or("-"),
            row.kept,
        )?;
    }
    let gib = |bytes: u64| format!("{:.2} GiB", bytes as f64 / f64::from(1u32 << 30));
    for (repository, sum) in summary {
        writeln!(
            out,
            "trees  {repository}: finished {} ({}), stale {} ({}, {} target/), kept {} ({})",
            sum.finished.0,
            gib(sum.finished.1),
            sum.stale.0,
            gib(sum.stale.1),
            gib(sum.stale_target),
            sum.kept.0,
            gib(sum.kept.1),
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
