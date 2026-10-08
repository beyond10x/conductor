//! `conductor resource usage`: token spend per repository and per session, read from the session
//! transcripts (`story:token-spend`).
//!
//! The command is presented by the `ess-cli/1` binding `crates/conductor/cli.yaml`, over the
//! `local` callable `resource-usage` and its input `conductor.observation.ResourceUsage`. Like
//! `dashboard serve`, it is not a placed command of the `cli:` block: it reads the transcripts
//! read-only and prints counts, and nothing it reads enters the store.
//!
//! **Transcripts.** One per session, `~/.claude/projects/<project directory>/<session id>.jsonl`.
//! A session's sub-agents keep theirs under `<project directory>/<session id>/subagents/`, at any
//! depth (a workflow nests its agents under `subagents/workflows/<run>/`): every `*.jsonl` there
//! counts for that parent session, in a table of its own. Every other file is not read.
//!
//! **Calls.** A call is an assistant line carrying a message id, its usage and a timestamp; a
//! synthetic message (model `<synthetic>`) is none. Each message counts once, however many lines
//! and transcripts carry it: the session tool writes a message split into blocks as one line per
//! block, each repeating the usage, and a forked or resumed session repeats the calls it took
//! over, with their timestamps. A message counts at its earliest timestamp, in the first
//! transcript in path order that holds a line of it at that instant. With `--since`, only the calls at
//! or after that instant count, and a transcript last modified before it is not read, since no
//! line in it can be newer.
//!
//! **Names.** A project directory is the working directory of its sessions with every character
//! but an ASCII letter or digit written `-`. It is named after a repository only when it encodes
//! `<checkouts.root>/<repo>`, a directory under it, or a directory under
//! `<checkouts.trees>/<repo>/`, the two roots of the instance the process runs, matched against
//! the names of the directories under `<checkouts.root>`, longest first, because repository names
//! hold dashes. Every other project
//! directory is counted into one row, `other`. Nothing else of it reaches the output or an error:
//! not its name, a session's name or working directory, or any line of a transcript. A session is
//! shown by the first 8 characters of its id, and as `unnamed` when they are not hexadecimal
//! digits.
//!
//! **Rows.** Per repository: sessions, calls, the four token counts and its share of the
//! cache-read tokens of every row. Per session: its repository, its id's first 8 characters,
//! calls, the four counts, and its latest context (input, cache-read and cache-write tokens of its
//! newest call). Per parent session, its sub-agents: their count, calls and the four counts. Each
//! table is ordered by cache-read tokens, largest first, ties by name.
//!
//! A transcript or a line that cannot be read is not counted, and is named by its repository only
//! on standard error after the tables; the command then exits 1. A last line without its line
//! break is a write in progress, not an error.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{self, BufRead as _, BufReader, Write as _};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow};
use clap::ValueEnum as _;
use conductor_model::observation::ResourceUsage;
use serde::Deserialize;
use serde_json::{Map, Value};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{Format, ResourceUsageArgs, ViewArgs};

/// The row every project directory outside the named repositories is counted into.
pub const OTHER: &str = "other";

/// Where the transcripts and the repositories are found; [`Sources::new`] gives the real ones.
#[derive(Debug, Clone)]
pub struct Sources {
    /// The session tool's project directories, `~/.claude/projects`.
    pub projects: PathBuf,
    /// The home directory: the project directories encode paths under it.
    pub home: PathBuf,
    /// The instance's checkouts root, whose directories are the repositories.
    pub root: PathBuf,
    /// The instance's managed trees, `<trees>/<repo>/<tree>`.
    pub trees: PathBuf,
}

impl Sources {
    /// `<home>/.claude/projects`, and the checkouts of the instance the process runs.
    #[must_use]
    pub fn new(home: PathBuf) -> Self {
        let checkouts = &crate::config::active().instance.checkouts;
        Self {
            projects: home.join(".claude/projects"),
            root: PathBuf::from(&checkouts.root),
            trees: PathBuf::from(&checkouts.trees),
            home,
        }
    }
}

/// The four token counts of one call, or of many.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tokens {
    /// Input tokens read without the cache.
    pub input: u64,
    /// Output tokens.
    pub output: u64,
    /// Input tokens read from the cache.
    pub cache_read: u64,
    /// Input tokens written to the cache.
    pub cache_write: u64,
}

impl Tokens {
    fn add(&mut self, other: Self) {
        self.input += other.input;
        self.output += other.output;
        self.cache_read += other.cache_read;
        self.cache_write += other.cache_write;
    }

    /// Each count the larger of the two: lines of one message carry its usage as far as it was
    /// known when each was written.
    fn max(self, other: Self) -> Self {
        Self {
            input: self.input.max(other.input),
            output: self.output.max(other.output),
            cache_read: self.cache_read.max(other.cache_read),
            cache_write: self.cache_write.max(other.cache_write),
        }
    }

    /// The context of a call: everything it read.
    fn context(self) -> u64 {
        self.input + self.cache_read + self.cache_write
    }
}

/// One repository, or [`OTHER`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRow {
    /// The repository's name, or [`OTHER`].
    pub repository: String,
    /// The sessions with a call counted.
    pub sessions: u64,
    /// The calls counted.
    pub calls: u64,
    /// Their tokens.
    pub tokens: Tokens,
}

/// One session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRow {
    /// The repository, or [`OTHER`].
    pub repository: String,
    /// The first 8 characters of the session id.
    pub session: String,
    /// The calls counted.
    pub calls: u64,
    /// Their tokens.
    pub tokens: Tokens,
    /// The context of its newest call.
    pub latest_context: u64,
}

/// The sub-agents of one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubagentRow {
    /// The parent session's repository, or [`OTHER`].
    pub repository: String,
    /// The first 8 characters of the parent session's id.
    pub session: String,
    /// The sub-agent transcripts with a call counted.
    pub agents: u64,
    /// The calls counted.
    pub calls: u64,
    /// Their tokens.
    pub tokens: Tokens,
}

/// What one reading of the transcripts counted.
#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    /// The instant from which calls count, RFC 3339; none counts every call.
    pub since: Option<String>,
    /// One row per repository, largest first.
    pub repositories: Vec<RepositoryRow>,
    /// One row per session, largest first.
    pub sessions: Vec<SessionRow>,
    /// One row per session with sub-agents, largest first.
    pub subagents: Vec<SubagentRow>,
    /// What could not be counted, each naming a repository or [`OTHER`] and never a path.
    pub problems: Vec<String>,
}

/// The columns of the repository table, in order.
pub const REPOSITORY_COLUMNS: [&str; 8] = [
    "repository",
    "sessions",
    "calls",
    "input",
    "output",
    "cache_read",
    "cache_write",
    "cache_read_percent",
];

const SESSION_COLUMNS: [&str; 8] = [
    "repository",
    "session",
    "calls",
    "input",
    "output",
    "cache_read",
    "cache_write",
    "latest_context",
];

const SUBAGENT_COLUMNS: [&str; 8] = [
    "repository",
    "session",
    "agents",
    "calls",
    "input",
    "output",
    "cache_read",
    "cache_write",
];

/// One table: its name in JSON, its heading, its columns and its rows.
struct Table {
    name: &'static str,
    heading: &'static str,
    columns: &'static [&'static str],
    rows: Vec<Vec<Value>>,
}

impl Usage {
    /// The repository rows as cells, in [`REPOSITORY_COLUMNS`] order: each repository's share of
    /// the cache-read tokens of every row is a percentage with one decimal.
    #[must_use]
    pub fn repository_cells(&self) -> Vec<Vec<Value>> {
        let total: u64 = self
            .repositories
            .iter()
            .map(|row| row.tokens.cache_read)
            .sum();
        self.repositories
            .iter()
            .map(|row| {
                let mut cells = vec![
                    Value::from(row.repository.clone()),
                    Value::from(row.sessions),
                    Value::from(row.calls),
                ];
                cells.extend(counts(row.tokens));
                cells.push(Value::from(percent(row.tokens.cache_read, total)));
                cells
            })
            .collect()
    }

    /// The repository rows as JSON objects, as `/data.json` carries them.
    #[must_use]
    pub fn repository_objects(&self) -> Value {
        Value::Array(
            self.repository_cells()
                .into_iter()
                .map(|cells| {
                    Value::Object(
                        REPOSITORY_COLUMNS
                            .iter()
                            .map(|column| (*column).to_owned())
                            .zip(cells)
                            .collect::<Map<String, Value>>(),
                    )
                })
                .collect(),
        )
    }

    fn tables(&self) -> [Table; 3] {
        let sessions = self
            .sessions
            .iter()
            .map(|row| {
                let mut cells = vec![
                    Value::from(row.repository.clone()),
                    Value::from(row.session.clone()),
                    Value::from(row.calls),
                ];
                cells.extend(counts(row.tokens));
                cells.push(Value::from(row.latest_context));
                cells
            })
            .collect();
        let subagents = self
            .subagents
            .iter()
            .map(|row| {
                let mut cells = vec![
                    Value::from(row.repository.clone()),
                    Value::from(row.session.clone()),
                    Value::from(row.agents),
                    Value::from(row.calls),
                ];
                cells.extend(counts(row.tokens));
                cells
            })
            .collect();
        [
            Table {
                name: "repositories",
                heading: "Repositories",
                columns: &REPOSITORY_COLUMNS,
                rows: self.repository_cells(),
            },
            Table {
                name: "sessions",
                heading: "Sessions",
                columns: &SESSION_COLUMNS,
                rows: sessions,
            },
            Table {
                name: "subagents",
                heading: "Sub-agents by parent session",
                columns: &SUBAGENT_COLUMNS,
                rows: subagents,
            },
        ]
    }

    /// The three tables as `format` renders them: `text` and `markdown` each under its heading;
    /// `json` one object holding `since` and each table as an array; `jsonl` one object per row,
    /// its first key `table` naming its table.
    #[must_use]
    pub fn render(&self, format: Format) -> String {
        let view = ViewArgs { format };
        let window = self.since.as_ref().map_or_else(
            || "in every transcript".to_owned(),
            |since| format!("since {since}"),
        );
        let tables = self.tables();
        match format {
            Format::Text | Format::Markdown => {
                let mut out = String::new();
                for (index, table) in tables.iter().enumerate() {
                    if index > 0 {
                        out.push('\n');
                    }
                    let _ = if format == Format::Markdown {
                        writeln!(out, "## {} {window}\n", table.heading)
                    } else {
                        writeln!(out, "{} {window}", table.heading)
                    };
                    out.push_str(&view.render(table.columns, &table.rows));
                }
                out
            }
            Format::Json => {
                let mut out = format!("{{\"since\":{}", Value::from(self.since.clone()));
                for table in &tables {
                    let _ = write!(
                        out,
                        ",{}:{}",
                        Value::from(table.name),
                        view.render(table.columns, &table.rows).trim_end()
                    );
                }
                out.push_str("}\n");
                out
            }
            Format::Jsonl => tables
                .iter()
                .map(|table| {
                    let columns: Vec<&str> = std::iter::once("table")
                        .chain(table.columns.iter().copied())
                        .collect();
                    let rows: Vec<Vec<Value>> = table
                        .rows
                        .iter()
                        .map(|row| {
                            std::iter::once(Value::from(table.name))
                                .chain(row.iter().cloned())
                                .collect()
                        })
                        .collect();
                    view.render(&columns, &rows)
                })
                .collect(),
        }
    }
}

fn counts(tokens: Tokens) -> [Value; 4] {
    [
        Value::from(tokens.input),
        Value::from(tokens.output),
        Value::from(tokens.cache_read),
        Value::from(tokens.cache_write),
    ]
}

/// `part` of `total` in percent, rounded to one decimal; 0 when `total` is.
fn percent(part: u64, total: u64) -> f64 {
    if total == 0 {
        return 0.0;
    }
    let share = part as f64 * 100.0 / total as f64;
    (share * 10.0).round() / 10.0
}

/// `conductor resource usage`: prints the tables of every call in the transcripts under `HOME`,
/// or of those at or after `--since`, as `--format` asks (default `text`), and exits 0; or, when
/// a transcript or a line could not be read, names each repository it belongs to on standard
/// error after the tables and exits 1.
///
/// # Errors
///
/// `--since` is no RFC 3339 instant, `--format` is none of `text`, `json`, `jsonl` and
/// `markdown`, `HOME` is not an absolute path, or the tables cannot be printed.
pub fn resource_usage(args: &ResourceUsageArgs) -> Result<ExitCode> {
    const COMMAND: &str = "resource usage";
    let input = ResourceUsage {
        since: args.since.clone(),
        format: args.format.clone(),
    };
    let since = input
        .since
        .as_deref()
        .map(|text| {
            OffsetDateTime::parse(text, &Rfc3339).map_err(|error| {
                anyhow!("{COMMAND}: --since {text:?} is no RFC 3339 instant: {error}")
            })
        })
        .transpose()?;
    let format = match input.format.as_deref() {
        None => Format::Text,
        Some(text) => Format::from_str(text, false).map_err(|_| {
            anyhow!("{COMMAND}: --format {text:?} is none of text, json, jsonl, markdown")
        })?,
    };
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|home| home.is_absolute())
        .with_context(|| {
            format!("{COMMAND}: HOME is not an absolute path; the transcripts are found under it")
        })?;
    let usage = measure(&Sources::new(home), since);
    let mut out = io::stdout().lock();
    out.write_all(usage.render(format).as_bytes())?;
    out.flush()?;
    drop(out);
    if usage.problems.is_empty() {
        return Ok(ExitCode::SUCCESS);
    }
    let mut err = io::stderr().lock();
    for problem in &usage.problems {
        writeln!(err, "conductor: {COMMAND}: {problem}")?;
    }
    Ok(ExitCode::FAILURE)
}

/// What a transcript holds the calls of.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Session,
    Subagent,
}

/// One transcript to read.
#[derive(Debug)]
struct Transcript {
    path: PathBuf,
    /// The repository, or [`OTHER`].
    repository: String,
    /// The session's id, or for a sub-agent its parent session's.
    session: String,
    kind: Kind,
}

/// One message, where and when it counts.
#[derive(Debug, Clone, Copy)]
struct Call {
    at: OffsetDateTime,
    /// The index of its transcript.
    transcript: usize,
    /// Its order among every line read, to order calls at the same instant.
    order: u64,
    tokens: Tokens,
}

/// What could not be read, per repository.
#[derive(Debug, Default)]
struct Unread {
    transcripts: u64,
    lines: u64,
    listings: u64,
}

/// Reads the transcripts under `sources` and counts every call, or those at or after `since`.
#[must_use]
pub fn measure(sources: &Sources, since: Option<OffsetDateTime>) -> Usage {
    let names = Names::new(&sources.root, &sources.trees);
    let mut unread: BTreeMap<String, Unread> = BTreeMap::new();
    let transcripts = transcripts(&sources.projects, &names, &mut unread);

    let mut calls: HashMap<String, Call> = HashMap::new();
    let mut order = 0_u64;
    for (index, transcript) in transcripts.iter().enumerate() {
        if let Some(since) = since
            && fs::metadata(&transcript.path)
                .and_then(|metadata| metadata.modified())
                .is_ok_and(|modified| OffsetDateTime::from(modified) < since)
        {
            continue;
        }
        let outcome = read(&transcript.path, &mut |id, at, tokens| {
            order += 1;
            let call = Call {
                at,
                transcript: index,
                order,
                tokens,
            };
            match calls.get_mut(&id) {
                None => {
                    calls.insert(id, call);
                }
                Some(known) if known.transcript == index => {
                    known.tokens = known.tokens.max(tokens);
                    if at < known.at {
                        known.at = at;
                    }
                }
                Some(known) if at < known.at => *known = call,
                Some(_) => {}
            }
        });
        let entry = unread.entry(transcript.repository.clone()).or_default();
        match outcome {
            Ok(lines) => entry.lines += lines,
            Err(_) => entry.transcripts += 1,
        }
    }

    let mut sessions: BTreeMap<(&str, &str), (SessionRow, OffsetDateTime, u64)> = BTreeMap::new();
    let mut subagents: BTreeMap<(&str, &str), (SubagentRow, BTreeSet<usize>)> = BTreeMap::new();
    for call in calls.values() {
        if since.is_some_and(|since| call.at < since) {
            continue;
        }
        let transcript = &transcripts[call.transcript];
        let key = (transcript.repository.as_str(), transcript.session.as_str());
        match transcript.kind {
            Kind::Session => {
                let (row, at, order) = sessions.entry(key).or_insert_with(|| {
                    (
                        SessionRow {
                            repository: transcript.repository.clone(),
                            session: short(&transcript.session),
                            calls: 0,
                            tokens: Tokens::default(),
                            latest_context: 0,
                        },
                        call.at,
                        call.order,
                    )
                });
                row.calls += 1;
                row.tokens.add(call.tokens);
                if (call.at, call.order) >= (*at, *order) {
                    *at = call.at;
                    *order = call.order;
                    row.latest_context = call.tokens.context();
                }
            }
            Kind::Subagent => {
                let (row, agents) = subagents.entry(key).or_insert_with(|| {
                    (
                        SubagentRow {
                            repository: transcript.repository.clone(),
                            session: short(&transcript.session),
                            agents: 0,
                            calls: 0,
                            tokens: Tokens::default(),
                        },
                        BTreeSet::new(),
                    )
                });
                agents.insert(call.transcript);
                row.agents = agents.len() as u64;
                row.calls += 1;
                row.tokens.add(call.tokens);
            }
        }
    }

    let mut repositories: BTreeMap<&str, RepositoryRow> = BTreeMap::new();
    for (&(repository, _), (row, _, _)) in &sessions {
        let total = repositories
            .entry(repository)
            .or_insert_with(|| RepositoryRow {
                repository: (*repository).to_owned(),
                sessions: 0,
                calls: 0,
                tokens: Tokens::default(),
            });
        total.sessions += 1;
        total.calls += row.calls;
        total.tokens.add(row.tokens);
    }

    let mut repositories: Vec<RepositoryRow> = repositories.into_values().collect();
    repositories.sort_by(|a, b| {
        b.tokens
            .cache_read
            .cmp(&a.tokens.cache_read)
            .then_with(|| a.repository.cmp(&b.repository))
    });
    let mut sessions: Vec<(&(&str, &str), SessionRow)> = sessions
        .iter()
        .map(|(key, (row, _, _))| (key, row.clone()))
        .collect();
    sessions.sort_by(|(a_key, a), (b_key, b)| {
        b.tokens
            .cache_read
            .cmp(&a.tokens.cache_read)
            .then_with(|| a_key.cmp(b_key))
    });
    let mut subagents: Vec<(&(&str, &str), SubagentRow)> = subagents
        .iter()
        .map(|(key, (row, _))| (key, row.clone()))
        .collect();
    subagents.sort_by(|(a_key, a), (b_key, b)| {
        b.tokens
            .cache_read
            .cmp(&a.tokens.cache_read)
            .then_with(|| a_key.cmp(b_key))
    });

    let mut problems = Vec::new();
    for (repository, unread) in &unread {
        if unread.listings > 0 {
            problems.push(format!(
                "{repository}: {} director(ies) of transcripts could not be listed",
                unread.listings
            ));
        }
        if unread.transcripts > 0 {
            problems.push(format!(
                "{repository}: {} transcript(s) could not be read",
                unread.transcripts
            ));
        }
        if unread.lines > 0 {
            problems.push(format!(
                "{repository}: {} transcript line(s) could not be read",
                unread.lines
            ));
        }
    }

    Usage {
        since: since.and_then(|since| since.format(&Rfc3339).ok()),
        repositories,
        sessions: sessions.into_iter().map(|(_, row)| row).collect(),
        subagents: subagents.into_iter().map(|(_, row)| row).collect(),
        problems,
    }
}

/// The first 8 characters of a session id, or `unnamed` when they are not hexadecimal digits:
/// a file name that is no session id may be a name.
fn short(session: &str) -> String {
    let head: String = session.chars().take(8).collect();
    if head.len() == 8 && head.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        head
    } else {
        "unnamed".to_owned()
    }
}

/// A path as the session tool names its project directory: every character but an ASCII letter or
/// digit written `-`.
fn encode(path: &Path) -> String {
    path.to_string_lossy()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

/// The repositories a project directory may be named after.
struct Names {
    /// The encoded checkouts root and managed-trees root, each with its `-`.
    prefixes: [String; 2],
    /// Each directory under the checkouts root: its name encoded, and its name; longest first.
    repositories: Vec<(String, String)>,
}

impl Names {
    fn new(root: &Path, trees: &Path) -> Self {
        let mut repositories: Vec<(String, String)> = fs::read_dir(root)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok().map(|entry| entry.path()))
                    .filter(|path| path.is_dir())
                    .filter_map(|path| {
                        let name = path.file_name()?.to_string_lossy().into_owned();
                        Some((encode(Path::new(&name)), name))
                    })
                    .collect()
            })
            .unwrap_or_default();
        repositories.sort_by(|(a, a_name), (b, b_name)| {
            b.len().cmp(&a.len()).then_with(|| a_name.cmp(b_name))
        });
        Self {
            prefixes: [format!("{}-", encode(root)), format!("{}-", encode(trees))],
            repositories,
        }
    }

    /// The repository the project directory `directory` belongs to, or [`OTHER`].
    fn repository(&self, directory: &str) -> String {
        for prefix in &self.prefixes {
            let Some(rest) = directory.strip_prefix(prefix.as_str()) else {
                continue;
            };
            for (encoded, name) in &self.repositories {
                let under = rest
                    .strip_prefix(encoded.as_str())
                    .is_some_and(|after| after.is_empty() || after.starts_with('-'));
                if under {
                    return name.clone();
                }
            }
        }
        OTHER.to_owned()
    }
}

/// The entries of `dir`, in path order.
fn entries(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

fn jsonl(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension == "jsonl")
        && path.is_file()
}

/// Every transcript under `projects`, in path order: each project directory's `*.jsonl`, and
/// every `*.jsonl` at any depth under each of its `<session id>/subagents/`. A directory that
/// cannot be listed is counted in `unread` for its repository.
fn transcripts(
    projects: &Path,
    names: &Names,
    unread: &mut BTreeMap<String, Unread>,
) -> Vec<Transcript> {
    let mut found = Vec::new();
    let directories = match entries(projects) {
        Ok(directories) => directories,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return found,
        Err(_) => {
            unread.entry(OTHER.to_owned()).or_default().listings += 1;
            return found;
        }
    };
    for directory in directories.into_iter().filter(|path| path.is_dir()) {
        let Some(name) = directory.file_name() else {
            continue;
        };
        let repository = names.repository(&name.to_string_lossy());
        let Ok(children) = entries(&directory) else {
            unread.entry(repository).or_default().listings += 1;
            continue;
        };
        for child in children {
            let Some(stem) = child
                .file_stem()
                .map(|stem| stem.to_string_lossy().into_owned())
            else {
                continue;
            };
            if jsonl(&child) {
                found.push(Transcript {
                    path: child,
                    repository: repository.clone(),
                    session: stem,
                    kind: Kind::Session,
                });
            } else if child.join("subagents").is_dir() {
                let session = child
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let mut stack = vec![child.join("subagents")];
                while let Some(dir) = stack.pop() {
                    let Ok(inner) = entries(&dir) else {
                        unread.entry(repository.clone()).or_default().listings += 1;
                        continue;
                    };
                    let mut nested = Vec::new();
                    for path in inner {
                        if jsonl(&path) {
                            found.push(Transcript {
                                path,
                                repository: repository.clone(),
                                session: session.clone(),
                                kind: Kind::Subagent,
                            });
                        } else if path.is_dir() {
                            nested.push(path);
                        }
                    }
                    stack.extend(nested);
                }
            }
        }
    }
    found.sort_by(|a, b| a.path.cmp(&b.path));
    found
}

/// The kind of a transcript line; every other field is skipped.
#[derive(Deserialize)]
struct Head {
    #[serde(rename = "type")]
    kind: Option<Value>,
}

/// The fields of an assistant line a call is read from; every other field is skipped.
#[derive(Deserialize)]
struct Line {
    timestamp: Option<String>,
    message: Option<Message>,
}

#[derive(Deserialize)]
struct Message {
    id: Option<String>,
    model: Option<String>,
    usage: Option<Counted>,
}

#[derive(Deserialize)]
struct Counted {
    input_tokens: Option<u64>,
    output_tokens: Option<u64>,
    cache_read_input_tokens: Option<u64>,
    cache_creation_input_tokens: Option<u64>,
}

/// What a line that names an assistant is.
enum Read {
    /// A call: its message id, instant and tokens.
    Call(String, OffsetDateTime, Tokens),
    /// No call: another kind of line, or a synthetic message.
    Skip,
    /// A line that cannot be read.
    Unreadable,
}

/// One line that names an assistant: a JSON object of another kind is no call, and an assistant
/// line is read only for its message id, model, usage and timestamp.
fn line(text: &str) -> Read {
    let Ok(head) = serde_json::from_str::<Head>(text) else {
        return Read::Unreadable;
    };
    if head.kind.as_ref().and_then(Value::as_str) != Some("assistant") {
        return Read::Skip;
    }
    let Ok(line) = serde_json::from_str::<Line>(text) else {
        return Read::Unreadable;
    };
    let Some(message) = line.message else {
        return Read::Unreadable;
    };
    if message.model.as_deref() == Some("<synthetic>") {
        return Read::Skip;
    }
    let (Some(id), Some(usage), Some(at)) = (
        message.id,
        message.usage,
        line.timestamp
            .and_then(|at| OffsetDateTime::parse(&at, &Rfc3339).ok()),
    ) else {
        return Read::Unreadable;
    };
    Read::Call(
        id,
        at,
        Tokens {
            input: usage.input_tokens.unwrap_or(0),
            output: usage.output_tokens.unwrap_or(0),
            cache_read: usage.cache_read_input_tokens.unwrap_or(0),
            cache_write: usage.cache_creation_input_tokens.unwrap_or(0),
        },
    )
}

/// Reads the transcript at `path`, handing each call to `call`; answers how many lines naming an
/// assistant could not be read. A last line without its line break is a write in progress and is
/// not counted as one.
fn read(path: &Path, call: &mut dyn FnMut(String, OffsetDateTime, Tokens)) -> io::Result<u64> {
    const MARK: &str = "\"assistant\"";
    let mut reader = BufReader::with_capacity(1 << 16, File::open(path)?);
    let mut bytes = Vec::new();
    let mut unreadable = 0;
    loop {
        bytes.clear();
        if reader.read_until(b'\n', &mut bytes)? == 0 {
            return Ok(unreadable);
        }
        let complete = bytes.last() == Some(&b'\n');
        let read = match std::str::from_utf8(&bytes) {
            Ok(text) if !text.contains(MARK) => Read::Skip,
            Ok(text) => line(text),
            Err(_)
                if bytes
                    .windows(MARK.len())
                    .any(|window| window == MARK.as_bytes()) =>
            {
                Read::Unreadable
            }
            Err(_) => Read::Skip,
        };
        match read {
            Read::Call(id, at, tokens) => call(id, at, tokens),
            Read::Unreadable if complete => unreadable += 1,
            Read::Unreadable | Read::Skip => {}
        }
    }
}
