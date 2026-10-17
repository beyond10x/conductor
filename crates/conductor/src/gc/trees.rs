//! The managed trees `conductor gc` and every reclaim run take back (`story:gc-finished-trees`).
//!
//! The trees are the instance's registered ones, read from `worktree inspect --json --workspace`
//! ([`Sources::inspect`]): lifecycle `active` or `finished`, a plain absolute path under the
//! instance's managed trees that is there. Each is one of:
//! - **finished**: no live session lease holds it (`live_leases`), no live session works in it, no
//!   process on the host has its working directory, program or an open file in it, and, for an
//!   `active` tree, its HEAD is contained in the default branch on origin (`origin/HEAD`, else
//!   `origin/main` or `origin/master`) and so is its branch on origin when there is one, its tree
//!   is clean apart from `target/` and `.check/`, and it was not touched in the last [`RECENT`].
//!   A `finished` record needs only the first three. It is taken back through `worktree finish
//!   <id> --discard-cache --archive` (an `active` one) and `worktree gc --id <id> --apply`;
//!   conductor never deletes a tree itself. When `finish` refuses for anything but
//!   `worktree-in-use`, the tree's cargo `target/` is removed and `finish` runs once more;
//! - **stale**: dirty or unmerged, untouched for longer than `thresholds.tree_stale`, and in use by
//!   nothing: its cargo `target/` (a directory, no link, holding `CACHEDIR.TAG`) is removed, as
//!   `cargo clean` would, and the tree is kept;
//! - **kept**: anything else, with why.
//!
//! A tree is touched at the newest of the registry's `last_seen_at`, the tree's directory, its git
//! `HEAD` and `index`, and every changed or untracked path. Git is read with
//! `GIT_OPTIONAL_LOCKS=0`, so a listing writes nothing.

use std::collections::HashSet;
use std::ffi::OsString;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::reclaim::{RECENT, holder_links, written_within};
use crate::dashboard::Sources;
use crate::dashboard::waste;
use crate::disk;

/// What the row of a finished tree is called.
pub(crate) const FINISHED: &str = "finished tree";
/// What the row of a stale tree's build output is called.
pub(crate) const STALE: &str = "build output of a stale tree";
/// What a kept tree is called among the holders.
pub(crate) const KEPT: &str = "managed tree";

/// What became of a tree's assessment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Verdict {
    Finished,
    /// Why it is not finished.
    Stale(String),
    /// Why it is kept whole.
    Kept(String),
}

/// One managed tree and its verdict.
#[derive(Debug, Clone)]
pub(crate) struct Tree {
    pub(crate) id: String,
    /// The tracked repository, the name of its root under the checkouts root.
    pub(crate) repository: String,
    pub(crate) root: PathBuf,
    pub(crate) path: PathBuf,
    /// The registry has it `finished` already: only `worktree gc` is left.
    pub(crate) registered_finished: bool,
    /// Its allocated bytes, when measured.
    pub(crate) bytes: Option<u64>,
    /// The allocated bytes of its cargo `target/`, when measured and cargo's.
    pub(crate) target_bytes: Option<u64>,
    pub(crate) verdict: Verdict,
}

/// The instance's managed trees, assessed, except those in `skip`; each measured when `measure`.
///
/// # Errors
///
/// `worktree inspect` printed nothing usable, or the live sessions or the processes cannot be
/// read: no tree could then be told to be idle.
pub(crate) fn assess(
    sources: &Sources,
    stale: Duration,
    skip: &HashSet<PathBuf>,
    measure: bool,
) -> Result<Vec<Tree>, String> {
    let inspected = waste::json_of(&sources.inspect, sources.measure_timeout)?;
    let live = waste::live(sources)?;
    let holders = holder_links()?;
    let now = SystemTime::now();
    let mut trees = Vec::new();
    for inspection in inspected["inspections"].as_array().into_iter().flatten() {
        let record = &inspection["record"];
        let registered_finished = match record["lifecycle"].as_str() {
            Some("active") => false,
            Some("finished") => true,
            _ => continue,
        };
        let (Some(id), Some(path), Some(root)) = (
            record["id"].as_str(),
            record["path"].as_str(),
            record["repository_root"].as_str(),
        ) else {
            continue;
        };
        let path = PathBuf::from(path);
        if !path.is_absolute()
            || path.components().any(|part| part == Component::ParentDir)
            || !path.starts_with(&sources.trees)
            || skip.contains(&path)
            || !fs::symlink_metadata(&path).is_ok_and(|meta| meta.is_dir())
        {
            continue;
        }
        let root = PathBuf::from(root);
        let repository = root.file_name().map_or_else(
            || root.display().to_string(),
            |name| name.to_string_lossy().into_owned(),
        );
        let leases = inspection["live_leases"].as_u64().unwrap_or(0);
        let busy = if leases > 0 {
            Some(format!("a session lease holds it ({leases} live)"))
        } else {
            in_use(&path, &live, &holders)
        };
        let last_seen = record["last_seen_at"]
            .as_u64()
            .map(|secs| UNIX_EPOCH + Duration::from_secs(secs));
        let verdict = match busy {
            Some(why) => Verdict::Kept(why),
            None if registered_finished => Verdict::Finished,
            None => judge(&path, last_seen, now, stale),
        };
        let size = |paths: &[PathBuf]| {
            disk::tree_size(paths, Some(Instant::now() + sources.measure_timeout))
                .ok()
                .map(|size| size.allocated)
        };
        let (bytes, target_bytes) = if measure {
            let target = path.join("target");
            (
                size(std::slice::from_ref(&path)),
                cargo_target(&path).ok().and_then(|()| size(&[target])),
            )
        } else {
            (None, None)
        };
        trees.push(Tree {
            id: id.to_owned(),
            repository,
            root,
            path,
            registered_finished,
            bytes,
            target_bytes,
            verdict,
        });
    }
    trees.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(trees)
}

/// Why `path` is in use now: a live session (`live`) or a process (`holders`, from the
/// `/proc/<pid>` links) works in it, its working directory, program or an open file there.
fn in_use(path: &Path, live: &[PathBuf], holders: &[(u32, PathBuf)]) -> Option<String> {
    if waste::works_in(live, path) {
        return Some("a live session works in it".to_owned());
    }
    let paths = [Some(path.to_path_buf()), fs::canonicalize(path).ok()];
    holders
        .iter()
        .find(|(_, link)| paths.iter().flatten().any(|path| link.starts_with(path)))
        .map(|(pid, _)| {
            format!("process {pid} has its working directory, program or an open file in it")
        })
}

/// The verdict on the active tree `path`, idle by every other check, at `now`.
fn judge(path: &Path, last_seen: Option<SystemTime>, now: SystemTime, stale: Duration) -> Verdict {
    let merged = match merged(path) {
        Ok(merged) => merged,
        Err(error) => return Verdict::Kept(format!("git could not tell: {error}")),
    };
    let changed = match changed(path) {
        Ok(changed) => changed,
        Err(error) => return Verdict::Kept(format!("git could not tell: {error}")),
    };
    let touched = touched(path, last_seen, &changed);
    let age = touched.map_or(Duration::ZERO, |touched| {
        now.duration_since(touched).unwrap_or(Duration::ZERO)
    });
    let hours = |age: Duration| age.as_secs() / 3600;
    let mut why = Vec::new();
    if let Err(unmerged) = &merged {
        why.push(format!("unmerged: {unmerged}"));
    }
    if let Some(first) = changed.first() {
        why.push(format!(
            "dirty: {} changed path(s), {}",
            changed.len(),
            first.display()
        ));
    }
    if why.is_empty() {
        if age > RECENT {
            return Verdict::Finished;
        }
        return Verdict::Kept(format!(
            "merged and clean, but touched in the last {} minutes",
            RECENT.as_secs() / 60
        ));
    }
    let why = why.join("; ");
    if age > stale {
        Verdict::Stale(format!("{why}; untouched for {}h", hours(age)))
    } else {
        Verdict::Kept(format!(
            "{why}; touched {}h ago, stale after {}h",
            hours(age),
            hours(stale)
        ))
    }
}

/// Runs git in `dir` without taking a lock: its exit code and standard output.
fn git(dir: &Path, args: &[&str]) -> Result<(Option<i32>, Vec<u8>), String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|error| format!("git {}: {error}", args.join(" ")))?;
    Ok((output.status.code(), output.stdout))
}

/// The trimmed output of a git command that must succeed.
fn git_text(dir: &Path, args: &[&str]) -> Result<String, String> {
    match git(dir, args)? {
        (Some(0), out) => Ok(String::from_utf8_lossy(&out).trim().to_owned()),
        (code, _) => Err(format!("git {} exited {code:?}", args.join(" "))),
    }
}

/// Whether the commit `refname` names exists.
fn exists(dir: &Path, refname: &str) -> Result<bool, String> {
    Ok(git(
        dir,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{refname}^{{commit}}"),
        ],
    )?
    .0 == Some(0))
}

/// Whether `ancestor` is contained in `of`.
fn contained(dir: &Path, ancestor: &str, of: &str) -> Result<bool, String> {
    match git(dir, &["merge-base", "--is-ancestor", ancestor, of])?.0 {
        Some(0) => Ok(true),
        Some(1) => Ok(false),
        code => Err(format!("git merge-base {ancestor} {of} exited {code:?}")),
    }
}

/// Whether the tree's work is in the default branch on origin: `Ok(Err(why not))` when it is not.
fn merged(dir: &Path) -> Result<Result<(), String>, String> {
    let head = git(
        dir,
        &["symbolic-ref", "--quiet", "refs/remotes/origin/HEAD"],
    )?;
    let mut default = (head.0 == Some(0))
        .then(|| String::from_utf8_lossy(&head.1).trim().to_owned())
        .filter(|name| !name.is_empty());
    if default.is_none() {
        for candidate in ["refs/remotes/origin/main", "refs/remotes/origin/master"] {
            if exists(dir, candidate)? {
                default = Some(candidate.to_owned());
                break;
            }
        }
    }
    let Some(default) = default else {
        return Ok(Err("origin has no default branch here".to_owned()));
    };
    let short = default.trim_start_matches("refs/remotes/").to_owned();
    if !contained(dir, "HEAD", &default)? {
        return Ok(Err(format!("HEAD is not in {short}")));
    }
    let branch = git(dir, &["symbolic-ref", "--quiet", "--short", "HEAD"])?;
    if branch.0 == Some(0) {
        let branch = String::from_utf8_lossy(&branch.1).trim().to_owned();
        let remote = format!("refs/remotes/origin/{branch}");
        if !branch.is_empty() && exists(dir, &remote)? && !contained(dir, &remote, &default)? {
            return Ok(Err(format!("origin/{branch} has commits {short} lacks")));
        }
    }
    Ok(Ok(()))
}

/// Whether `path`, relative to the tree, lies in `target/` or `.check/`.
fn excluded(path: &str) -> bool {
    let first = path.split('/').next().unwrap_or_default();
    first == "target" || first == ".check"
}

/// The changed and untracked paths of the tree, apart from `target/` and `.check/`.
fn changed(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let (code, out) = git(
        dir,
        &[
            "status",
            "--porcelain=v1",
            "-z",
            "--untracked-files=normal",
            "--ignore-submodules=none",
        ],
    )?;
    if code != Some(0) {
        return Err(format!("git status exited {code:?}"));
    }
    let mut paths = Vec::new();
    let mut fields = out
        .split(|byte| *byte == 0)
        .filter(|field| !field.is_empty());
    while let Some(field) = fields.next() {
        let text = String::from_utf8_lossy(field);
        let (status, path) = text.split_at(text.len().min(3));
        // A rename or copy names its source in the next field.
        if status.starts_with('R') || status.starts_with('C') {
            fields.next();
        }
        if !excluded(path) {
            paths.push(dir.join(path.trim_end_matches('/')));
        }
    }
    Ok(paths)
}

/// When the tree was last touched: the newest of `last_seen`, the tree's directory, its git `HEAD`
/// and `index`, and each of `changed`.
fn touched(dir: &Path, last_seen: Option<SystemTime>, changed: &[PathBuf]) -> Option<SystemTime> {
    let gitdir = git_text(dir, &["rev-parse", "--absolute-git-dir"])
        .ok()
        .map(PathBuf::from);
    let mut paths = vec![dir.to_path_buf()];
    if let Some(gitdir) = gitdir {
        paths.push(gitdir.join("HEAD"));
        paths.push(gitdir.join("index"));
    }
    paths.extend(changed.iter().cloned());
    paths
        .iter()
        .filter_map(|path| fs::symlink_metadata(path).ok()?.modified().ok())
        .chain(last_seen)
        .max()
}

/// Whether the tree's `target/` is cargo's: a directory, no link, holding `CACHEDIR.TAG`.
fn cargo_target(tree: &Path) -> Result<(), String> {
    let target = tree.join("target");
    match fs::symlink_metadata(&target) {
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err("its target/ is no directory".to_owned()),
        Err(_) => return Err("it has no target/".to_owned()),
    }
    if !fs::symlink_metadata(target.join("CACHEDIR.TAG")).is_ok_and(|meta| meta.is_file()) {
        return Err("its target/ is not cargo's: no CACHEDIR.TAG".to_owned());
    }
    Ok(())
}

/// Removes the tree's cargo `target/`, as `cargo clean` would, unless it is not cargo's or was
/// written in the last [`RECENT`]: the bytes freed.
fn remove_target(tree: &Path) -> Result<u64, String> {
    cargo_target(tree)?;
    let target = tree.join("target");
    if written_within(&target, RECENT) {
        return Err(format!(
            "its target/ was written in the last {} minutes: a gate may be running",
            RECENT.as_secs() / 60
        ));
    }
    let before = allocated(&target);
    fs::remove_dir_all(&target).map_err(|error| format!("remove {}: {error}", target.display()))?;
    Ok(before)
}

fn allocated(path: &Path) -> u64 {
    disk::tree_size(&[path.to_path_buf()], None).map_or(0, |size| size.allocated)
}

/// Runs `worktree <args>` in `dir` to its end: its JSON answer, or the refusal's code (when it
/// printed one) and what failed.
fn worktree(args: &[OsString], dir: &Path) -> Result<Value, (Option<String>, String)> {
    let shown = format!(
        "worktree {}",
        args.iter()
            .map(|arg| arg.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    let mut command = Command::new("worktree");
    command.args(args).stdin(Stdio::null());
    if dir.is_dir() {
        command.current_dir(dir);
    }
    let output = command
        .output()
        .map_err(|error| (None, format!("`{shown}`: {error}")))?;
    let answer: Option<Value> = serde_json::from_slice(&output.stdout).ok();
    let code = answer
        .as_ref()
        .and_then(|answer| answer["code"].as_str())
        .map(str::to_owned);
    if !output.status.success() || answer.as_ref().is_some_and(|answer| answer["ok"] == false) {
        let why = answer
            .as_ref()
            .and_then(|answer| answer["message"].as_str())
            .map(str::to_owned)
            .unwrap_or_else(|| {
                String::from_utf8_lossy(&output.stderr)
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_owned()
            });
        return Err((code, format!("`{shown}` refused: {why}")));
    }
    Ok(answer.unwrap_or(Value::Null))
}

/// What taking one tree back did: the bytes freed, and a note (a retry, a refusal).
pub(crate) struct Taken {
    pub(crate) freed: u64,
    pub(crate) note: Option<String>,
}

/// Takes the tree back as its verdict says: a finished tree through `worktree`, a stale tree's
/// `target/` by removal. Every check the listing made against use is made again first.
///
/// # Errors
///
/// Why it was refused, with what it freed meanwhile.
pub(crate) fn take(sources: &Sources, tree: &Tree) -> Result<Taken, (String, u64)> {
    let live = waste::live(sources).map_err(|error| {
        (
            format!("the live sessions could not be read again: {error}"),
            0,
        )
    })?;
    let holders = holder_links().map_err(|error| (error, 0))?;
    if let Some(why) = in_use(&tree.path, &live, &holders) {
        return Err((format!("now in use: {why}"), 0));
    }
    match &tree.verdict {
        Verdict::Kept(why) => Err((why.clone(), 0)),
        Verdict::Stale(_) => remove_target(&tree.path)
            .map(|freed| Taken { freed, note: None })
            .map_err(|why| (why, 0)),
        Verdict::Finished => finish(tree),
    }
}

fn finish(tree: &Tree) -> Result<Taken, (String, u64)> {
    let before = allocated(&tree.path);
    let os = OsString::from;
    let mut note = None;
    let mut freed_early = 0;
    if !tree.registered_finished {
        let args = [
            os("finish"),
            os("--json"),
            os("--discard-cache"),
            os("--archive"),
            os(&tree.id),
        ];
        if let Err((code, why)) = worktree(&args, &tree.root) {
            if code.as_deref() == Some("worktree-in-use") || cargo_target(&tree.path).is_err() {
                return Err((why, 0));
            }
            freed_early = remove_target(&tree.path)
                .map_err(|error| (format!("{why}; target/ not removed: {error}"), 0))?;
            note = Some(format!(
                "{} refused; its target/ was removed and finish retried once",
                code.as_deref().unwrap_or("finish")
            ));
            worktree(&args, &tree.root).map_err(|(_, again)| {
                (
                    format!("{}; again: {again}", note.clone().unwrap_or_default()),
                    freed_early,
                )
            })?;
        }
    }
    let args = [
        os("gc"),
        os("--repo"),
        tree.root.clone().into_os_string(),
        os("--json"),
        os("--id"),
        os(&tree.id),
        os("--apply"),
    ];
    let answer = worktree(&args, &tree.root).map_err(|(_, why)| (why, freed_early))?;
    if fs::symlink_metadata(&tree.path).is_ok() {
        let refusal = answer["assessments"]
            .as_array()
            .into_iter()
            .flatten()
            .find_map(|assessment| assessment["refusal"]["code"].as_str())
            .unwrap_or("no reason given");
        let after = allocated(&tree.path);
        return Err((
            format!("worktree gc kept it: {refusal}"),
            before.saturating_sub(after),
        ));
    }
    // `before` was measured with `target/` still there.
    Ok(Taken {
        freed: before,
        note,
    })
}
