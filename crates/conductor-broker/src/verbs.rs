//! The read verbs (design § 6.3) and the bounded form of every answer: a success is the verb's
//! result, cut at the call's limit, then one summary line; a failure is at most
//! [`FAILURE_LINES`] lines, the last naming the log that holds all of it.

use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, PoisonError};

use serde_json::Value;
use time::OffsetDateTime;

use crate::Verb;
use crate::grant::Refusal;
use crate::registry::Registration;
use crate::server::{Reads, rfc3339};
use crate::wire::Answer;

/// The most lines a failure prints, its log line included.
pub const FAILURE_LINES: usize = 40;

/// The longest line `search` prints of a match, in bytes; a longer one is cut.
const MATCH_LINE: usize = 240;

/// Directories `search` never enters: a repository's own data and build output.
const SKIPPED: [&str; 3] = [".git", "target", "node_modules"];

/// What a verb runs with: the broker's directory, the calling session, its registration and the
/// session's earlier reads.
pub(crate) struct Context<'a> {
    pub dir: &'a Path,
    pub session: &'a str,
    pub registration: &'a Registration,
    pub reads: &'a Mutex<Reads>,
    pub now: OffsetDateTime,
}

/// The paths a call names, as the call writes them: all of its argument text a record keeps.
pub(crate) fn paths(verb: Verb, args: &Value) -> Vec<String> {
    match verb {
        Verb::Show | Verb::Search => args["path"]
            .as_str()
            .map(|path| vec![path.to_owned()])
            .unwrap_or_default(),
        Verb::Diff => Vec::new(),
    }
}

/// Runs `verb` with `args`: its answer, or the refusal of a path outside the grant.
pub(crate) fn run(context: &Context<'_>, verb: Verb, args: &Value) -> Result<Answer, Refusal> {
    match verb {
        Verb::Show => show(context, args),
        Verb::Search => search(context, args),
        Verb::Diff => Ok(diff(context, args)),
    }
}

/// The answer to a verb that failed: `head`, then the first lines of `text`, at most
/// [`FAILURE_LINES`] in all with the last, `log: <path>`, naming a file under `dir`'s `logs/`
/// that holds `head` and all of `text`. Exit status 1.
#[must_use]
pub fn failure(dir: &Path, head: &str, text: &str) -> Answer {
    let mut whole = format!("{head}\n{text}");
    if !whole.ends_with('\n') {
        whole.push('\n');
    }
    let log = write_log(dir, &whole);
    let mut shown: Vec<&str> = whole.lines().take(FAILURE_LINES - 1).collect();
    let reference = match &log {
        Ok(path) => format!("log: {}", path.display()),
        Err(error) => format!("log: none, it could not be written: {error}"),
    };
    shown.push(&reference);
    Answer {
        text: shown.join("\n") + "\n",
        exit: 1,
    }
}

fn write_log(dir: &Path, text: &str) -> std::io::Result<PathBuf> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let logs = dir.join("logs");
    fs::create_dir_all(&logs)?;
    let stamp = OffsetDateTime::now_utc().unix_timestamp_nanos();
    let path = logs.join(format!(
        "{stamp}-{}-{}.log",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    fs::write(&path, text)?;
    Ok(path)
}

/// `body` cut at `limit` bytes, on a character boundary, ending in a newline; and whether it was
/// cut.
fn bounded(body: &str, limit: usize) -> (String, bool) {
    if body.len() <= limit {
        let mut out = body.to_owned();
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        return (out, false);
    }
    let mut end = limit;
    while !body.is_char_boundary(end) {
        end -= 1;
    }
    let mut out = body[..end].to_owned();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    (out, true)
}

/// The limit of one answer: the call's `max` when it names a smaller one, else the grant's.
fn limit(context: &Context<'_>, args: &Value) -> usize {
    let granted = usize::try_from(context.registration.grant.max_output_bytes).unwrap_or(1);
    args["max"]
        .as_u64()
        .and_then(|max| usize::try_from(max).ok())
        .map_or(granted, |max| max.min(granted))
}

fn cut_note(cut: bool, limit: usize) -> String {
    if cut {
        format!(", cut at {limit} bytes")
    } else {
        String::new()
    }
}

fn show(context: &Context<'_>, args: &Value) -> Result<Answer, Refusal> {
    let Some(path) = args["path"].as_str() else {
        return Ok(failure(context.dir, "show: the call names no path", ""));
    };
    let grant = &context.registration.grant;
    let file = grant.resolve(&context.registration.workdir, path)?;
    let content = match fs::read(&file) {
        Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
        Err(error) => {
            return Ok(failure(
                context.dir,
                &format!("show {path}: cannot read it: {error}"),
                "",
            ));
        }
    };
    let lines: Vec<&str> = content.split_inclusive('\n').collect();
    let total = lines.len();
    let (first, last, key) = if let Some(symbol) = args["symbol"].as_str() {
        match block(&lines, symbol) {
            Some((first, last)) => (first, last, format!("fn {symbol}")),
            None => {
                return Ok(failure(
                    context.dir,
                    &format!("show {path}: no item names {symbol}"),
                    "",
                ));
            }
        }
    } else if let Some(range) = args["lines"].as_str() {
        match parse_range(range, total) {
            Some((first, last)) => (first, last, format!("lines {first}:{last}")),
            None => {
                return Ok(failure(
                    context.dir,
                    &format!(
                        "show {path}: --lines {range} is not a:b within the file's {total} lines"
                    ),
                    "",
                ));
            }
        }
    } else {
        (1, total, "whole".to_owned())
    };
    let selected: String = if total == 0 {
        String::new()
    } else {
        lines[first - 1..last].concat()
    };
    let mut hasher = DefaultHasher::new();
    selected.hash(&mut hasher);
    let hash = hasher.finish();
    {
        let mut reads = context.reads.lock().unwrap_or_else(PoisonError::into_inner);
        let entry = (context.session.to_owned(), format!("{path}#{key}"));
        match reads.get(&entry) {
            Some((seen, at)) if *seen == hash => {
                return Ok(Answer::done(format!("show {path}: unchanged since {at}\n")));
            }
            _ => {
                reads.insert(entry, (hash, rfc3339(context.now)));
            }
        }
    }
    let limit = limit(context, args);
    let (body, cut) = bounded(&selected, limit);
    Ok(Answer::done(format!(
        "{body}show {path}: lines {first}-{last} of {total}, {} bytes{}\n",
        selected.len(),
        cut_note(cut, limit)
    )))
}

/// `a:b`, 1-based, both ends included, either end left out for the file's start or end, within
/// `total` lines.
fn parse_range(range: &str, total: usize) -> Option<(usize, usize)> {
    let (from, to) = range.split_once(':')?;
    let first = if from.trim().is_empty() {
        1
    } else {
        from.trim().parse().ok()?
    };
    let last = if to.trim().is_empty() {
        total
    } else {
        to.trim().parse::<usize>().ok()?.min(total)
    };
    (first >= 1 && first <= last).then_some((first, last))
}

/// The words that open a named item, in the languages a checkout is likely to hold.
const ITEM_WORDS: [&str; 14] = [
    "fn",
    "struct",
    "enum",
    "trait",
    "type",
    "mod",
    "const",
    "static",
    "impl",
    "macro_rules!",
    "class",
    "def",
    "function",
    "interface",
];

fn ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Whether `line` opens the item `symbol`: one of [`ITEM_WORDS`], then the symbol as a whole
/// word.
fn opens(line: &str, symbol: &str) -> bool {
    ITEM_WORDS.iter().any(|word| {
        let needle = format!("{word} {symbol}");
        line.match_indices(&needle).any(|(at, _)| {
            let before = line[..at].chars().next_back();
            let after = line[at + needle.len()..].chars().next();
            !before.is_some_and(ident) && !after.is_some_and(ident)
        })
    })
}

/// The 1-based first and last line of the item `symbol`: from the line that opens it to the
/// brace that closes its block, or, for a line ending in `:`, to the last line indented under it.
fn block(lines: &[&str], symbol: &str) -> Option<(usize, usize)> {
    let start = lines.iter().position(|line| opens(line, symbol))?;
    let head = lines[start].trim_end();
    if head.ends_with(':') {
        let indent = |line: &str| line.len() - line.trim_start().len();
        let base = indent(lines[start]);
        let mut last = start;
        for (at, line) in lines.iter().enumerate().skip(start + 1) {
            if line.trim().is_empty() {
                continue;
            }
            if indent(line) <= base {
                break;
            }
            last = at;
        }
        return Some((start + 1, last + 1));
    }
    let mut depth = 0_i64;
    let mut opened = false;
    for (at, line) in lines.iter().enumerate().skip(start) {
        for c in line.chars() {
            match c {
                '{' => {
                    depth += 1;
                    opened = true;
                }
                '}' => depth -= 1,
                ';' if !opened && depth == 0 => return Some((start + 1, at + 1)),
                _ => {}
            }
        }
        if opened && depth <= 0 {
            return Some((start + 1, at + 1));
        }
    }
    Some((start + 1, start + 1))
}

fn search(context: &Context<'_>, args: &Value) -> Result<Answer, Refusal> {
    let pattern = match args["pattern"].as_str() {
        Some(pattern) if !pattern.is_empty() => pattern,
        _ => {
            return Ok(failure(
                context.dir,
                "search: the call names no pattern",
                "",
            ));
        }
    };
    let grant = &context.registration.grant;
    let workdir = &context.registration.workdir;
    let root = workdir.canonicalize().unwrap_or_else(|_| workdir.clone());
    let roots: Vec<PathBuf> = match args["path"].as_str() {
        Some(path) => vec![grant.resolve(workdir, path)?],
        None if grant.paths.is_empty() => vec![root.clone()],
        None => grant
            .paths
            .iter()
            .filter_map(|path| grant.resolve(workdir, path).ok())
            .collect(),
    };
    let mut files = Vec::new();
    for at in roots {
        walk(&at, &mut files);
    }
    let mut found = String::new();
    let mut matches = 0_usize;
    let mut matched_files = 0_usize;
    for file in files {
        let Ok(bytes) = fs::read(&file) else { continue };
        if bytes.iter().take(8000).any(|byte| *byte == 0) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        let shown = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .to_string_lossy()
            .replace('\\', "/");
        let before = matches;
        for (number, line) in text.lines().enumerate() {
            if line.contains(pattern) {
                matches += 1;
                let mut line = line.trim_end();
                if line.len() > MATCH_LINE {
                    let mut end = MATCH_LINE;
                    while !line.is_char_boundary(end) {
                        end -= 1;
                    }
                    line = &line[..end];
                }
                found.push_str(&format!("{shown}:{}: {line}\n", number + 1));
            }
        }
        if matches > before {
            matched_files += 1;
        }
    }
    let limit = limit(context, args);
    let (body, cut) = bounded(&found, limit);
    Ok(Answer::done(format!(
        "{body}search {pattern}: {matches} matches in {matched_files} files{}\n",
        cut_note(cut, limit)
    )))
}

/// Every file under `at`, in name order, through no link and into none of [`SKIPPED`].
fn walk(at: &Path, out: &mut Vec<PathBuf>) {
    let Ok(meta) = fs::symlink_metadata(at) else {
        return;
    };
    if meta.is_file() {
        out.push(at.to_owned());
        return;
    }
    if !meta.is_dir() {
        return;
    }
    let Ok(entries) = fs::read_dir(at) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_none_or(|name| !SKIPPED.contains(&name))
        })
        .map(|entry| entry.path())
        .collect();
    entries.sort();
    for entry in entries {
        walk(&entry, out);
    }
}

/// `git` in the session's working directory with `args`, limited to the grant's paths: its
/// output, or the failure answer.
fn git(context: &Context<'_>, args: &[&str]) -> Result<String, Answer> {
    let grant = &context.registration.grant;
    let mut command = Command::new("git");
    command
        .arg("-C")
        .arg(&context.registration.workdir)
        .args(["-c", "core.quotepath=off"])
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0");
    if !grant.paths.is_empty() {
        command.arg("--").args(&grant.paths);
    }
    match command.output() {
        Ok(output) if output.status.success() => {
            Ok(String::from_utf8_lossy(&output.stdout).into_owned())
        }
        Ok(output) => Err(failure(
            context.dir,
            &format!("diff: git {} failed ({})", args.join(" "), output.status),
            &(String::from_utf8_lossy(&output.stderr).into_owned()
                + &String::from_utf8_lossy(&output.stdout)),
        )),
        Err(error) => Err(failure(
            context.dir,
            &format!("diff: git cannot run: {error}"),
            "",
        )),
    }
}

fn diff(context: &Context<'_>, args: &Value) -> Answer {
    let patch = args["patch"].as_bool().unwrap_or(false);
    let summary = match git(
        context,
        &["diff", "--no-color", "--no-ext-diff", "--shortstat"],
    ) {
        Ok(text) => text.trim().to_owned(),
        Err(answer) => return answer,
    };
    let body = if patch {
        git(context, &["diff", "--no-color", "--no-ext-diff"])
    } else {
        git(context, &["diff", "--no-color", "--no-ext-diff", "--stat"]).map(|stat| {
            stat.lines()
                .filter(|line| line.trim() != summary)
                .map(|line| format!("{line}\n"))
                .collect()
        })
    };
    let body = match body {
        Ok(body) => body,
        Err(answer) => return answer,
    };
    let limit = limit(context, args);
    let (body, cut) = bounded(&body, limit);
    let summary = if summary.is_empty() {
        "no change".to_owned()
    } else {
        summary
    };
    Answer::done(format!("{body}diff: {summary}{}\n", cut_note(cut, limit)))
}
