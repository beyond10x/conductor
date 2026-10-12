//! `conductor guard`: the PreToolUse hook of conductor and of every repository controller, and the
//! record of its verdicts.
//!
//! `guard record-guard-decision --from-pre-tool-use` reads the payload Claude Code sends a
//! PreToolUse hook on standard input, decides the call by the rule table of the repository the
//! session runs in ([`decide`]), records a denial as `conductor.dispatch.RecordGuardDecision`, and
//! answers it. It decides by the places of the instance the process runs ([`Places::active`],
//! from `crate::config::active`). The repository is the directory holding `.git` at depth 1 or 2
//! (`<repo>` or `<group>/<repo>`, `crate::collect::repository_of`) that holds the payload's `cwd`
//! under the instance's checkouts root (its checkout) or its managed-worktree root (its managed
//! worktrees, where a controller's workers run), and is decided by the controller rule, unless a
//! source of the instance excludes it.
//! Conductor's rule decides the session in conductor's records: with a config file, a cwd in the
//! instance's `records`; without one, the repository named `conductor`. A cwd in none of them is
//! denied every tool that has a rule. A config file that does not load leaves the hook on the
//! built-in instance (`crate::cli::Cli::run`), whose places are today's. Either way the controller
//! rule keeps a controller from writing conductor's config: the file this process resolves
//! ([`config::locate`] over its own environment, `config_file`), loaded or not, the default file,
//! the default file's directory itself, and the instance's state directory; not the rest of that
//! directory, where an instance's records are by default.
//!
//! Claude Code reads the answer from the exit status alone: 0 lets the call proceed, 2 blocks it
//! and hands standard error to the session, and any other status lets it proceed. So a denial is
//! exit 2 with one line of reason on standard error, an allowance is exit 0 with no output, and
//! everything that goes wrong before a verdict exists (an unreadable payload, no home directory, a
//! panic) is a denial. What this binary cannot answer for itself, such as `conductor` missing
//! from `PATH`, the wired command does: both settings files end it with `|| exit 2`.
//!
//! Only denials are recorded. The store replays every event file each time it opens, unless no
//! file changed since an earlier open kept its index: with 5,000 events stored, opening it took
//! 1.14 s (release build, `tests/store_tree.rs`), while deciding a call takes about
//! 1.5 ms (`tests/guard_latency.rs`). Every tool call of every controller runs this hook, so an
//! allowance is answered without opening the store. A denial is decided first, recording it is
//! given at most [`RECORD_DEADLINE`] (the store's writer lock may be held by another session's
//! hook), and a denial that could not be recorded is still answered, with one line on standard
//! error.
//!
//! A write that conductor's rule allows (Edit, Write or NotebookEdit; SendMessage; a Bash or
//! Monitor command the guard treats as a write, or one that names the lease's file or its state
//! directory) is decided again by the instance's conductor lease (`story:lease-guard`,
//! [`leased`]). While the lease names another session than the payload's
//! `session_id` and the holder's process lives, the write is denied as a second conductor's,
//! naming the holder's session id and pid. The lease is read from one small file under the state
//! directory, the guard's copy that `session claim` writes beside the store
//! (`crate::session::LeaseFile`), never from the store. No file, one that does not read, or a dead
//! holder leaves the verdict as the rule table gave it. Reads, and every call of a controller,
//! never read it.

mod rules;
mod shell;

use std::io::{self, Read as _};
use std::panic;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::behaviour::{Generated, GuardDecisionStorage, TryContext, unmet_context};
use conductor_model::direction::ServingId;
use conductor_model::dispatch::obligations::{GuardDecisionsQuery, RecordGuardDecisionBehavior};
use conductor_model::dispatch::{
    GuardDecisionId, GuardDecisionSnapshot, MessageId, RecordGuardDecision,
    RecordGuardDecisionOutcome, ResourceRequestId, Verdict,
};
use conductor_model::obligation::UnmetObligation;
use conductor_model::observation::{BoardId, ObservationId, RepositoryName, SnapshotId};
use conductor_model::primitives::{Timestamp, Uuid};
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::{RecordGuardDecisionArgs, ViewArgs};
use crate::config;
use crate::not_implemented;
use crate::store::Store;

pub use self::rules::{Places, SessionList};
use self::rules::{Rules, Session};

/// How long recording a verdict may take before the verdict is answered without it.
pub const RECORD_DEADLINE: Duration = Duration::from_secs(2);

/// The exit status that blocks a tool call.
const DENY_EXIT: u8 = 2;

/// The tools the rule table speaks about. Monitor runs a shell command as Bash does, so the Bash
/// rule decides it. Any other tool is allowed only from [`READ_ONLY`] or the `read_only_tools` of
/// the session's role, and denied otherwise (`story:guard-all-tools`).
const RULED: [&str; 6] = [
    "Edit",
    "Write",
    "NotebookEdit",
    "SendMessage",
    "Bash",
    "Monitor",
];

/// The built-in read-only allowlist: the harness's tools that write no file, run no command and
/// reach no other session, so the guard allows them to every session without deciding where it
/// runs; the allowance stays as fast as a ruled one, and opens no store. A role's
/// `read_only_tools` in the config extends it for that role. Each entry, and why it is here:
const READ_ONLY: [&str; 24] = [
    // Reads a file.
    "Read",
    // Lists paths matching a pattern.
    "Glob",
    // Searches file contents.
    "Grep",
    // Lists a directory (an older harness tool).
    "LS",
    // Fetches a URL and reads it; writes nothing locally.
    "WebFetch",
    // Runs a web search; writes nothing locally.
    "WebSearch",
    // The session's own to-do list, kept by the harness.
    "TodoWrite",
    // The session's own task list, kept by the harness: create, read, list, update.
    "TaskCreate",
    "TaskGet",
    "TaskList",
    "TaskUpdate",
    // Reads the output of a background task or shell the session started.
    "TaskOutput",
    "BashOutput",
    // Stops a background task or shell the session started; writes no file.
    "TaskStop",
    "KillShell",
    // Loads a deferred tool's schema.
    "ToolSearch",
    // Lists the harness's sessions; messaging one is SendMessage, which has its rule.
    "ListAgents",
    // Lists and reads an MCP server's resources; an MCP tool call is not among them.
    "ListMcpResourcesTool",
    "ReadMcpResourceTool",
    // Loads a skill's instructions into the session.
    "Skill",
    // The session's own wake-up schedule, a prompt to itself (conductor's cycle jobs,
    // `.agents/conductor.md`): create, list, delete.
    "CronCreate",
    "CronList",
    "CronDelete",
    // The session's own one-shot wake-up, a prompt to itself (`/loop` without an interval).
    "ScheduleWakeup",
];

/// The tools that start a sub-agent (`Agent`, earlier `Task`). A sub-agent's own tool calls reach
/// this same hook and are decided by the cwd they carry, so [`sub_agent`] allows one only where
/// that cwd stays the caller's.
const SUB_AGENT: [&str; 2] = ["Agent", "Task"];

/// The verdict on a [`SUB_AGENT`] call: denied when its `tool_input` sets `cwd`, which moves the
/// sub-agent's calls under another place's rule, or `isolation: "remote"`, which runs them where
/// this hook never sees them; allowed otherwise. Found by the wave 04 review of
/// `story:guard-all-tools`.
fn sub_agent(tool: &str, input: &Value) -> (Verdict, String) {
    if input.get("cwd").is_some_and(|cwd| !cwd.is_null()) {
        return (
            Verdict::Deny,
            format!(
                "the {tool} call sets cwd: a sub-agent's calls are decided by the cwd they carry, \
                 so it would leave the caller's rule; start it without cwd"
            ),
        );
    }
    if input["isolation"].as_str() == Some("remote") {
        return (
            Verdict::Deny,
            format!(
                "the {tool} call sets isolation remote: a remote sub-agent's calls never reach \
                 this guard; start it locally"
            ),
        );
    }
    (
        Verdict::Allow,
        format!(
            "{tool} sets neither cwd nor remote isolation; its sub-agent's calls reach this guard"
        ),
    )
}

/// The guard's verdict on one PreToolUse payload, with the fields its record keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// The repository the session runs in: `<repo>` or `<group>/<repo>` of its cwd under the
    /// checkouts root or the managed-worktree root, `conductor` for a session in conductor's
    /// records, empty when the cwd is in none of them.
    pub repository: String,
    /// The session's id, as the payload names it.
    pub session_ref: String,
    /// The tool called.
    pub tool: String,
    /// What the call targets: the file for Edit, Write and NotebookEdit, the recipient for
    /// SendMessage, the command for Bash and Monitor; empty for any other tool.
    pub target: String,
    /// Allow or deny.
    pub verdict: Verdict,
    /// Why, in one sentence the session is shown on a denial.
    pub reason: String,
}

impl Decision {
    fn denied(reason: impl Into<String>) -> Self {
        Self {
            repository: String::new(),
            session_ref: String::new(),
            tool: String::new(),
            target: String::new(),
            verdict: Verdict::Deny,
            reason: reason.into(),
        }
    }
}

/// Decides one PreToolUse `payload` by the rule table in the built-in places under `home`
/// ([`Places::built_in`], which apply without a config file), with [`SessionList::default`] as the
/// session list.
#[must_use]
pub fn decide(payload: &Value, home: &Path) -> Decision {
    decide_with(payload, home, &SessionList::default())
}

/// [`decide`], reading the session list from `sessions`: once, and only for a controller's
/// SendMessage to a socket address.
#[must_use]
pub fn decide_with(payload: &Value, home: &Path, sessions: &SessionList) -> Decision {
    decide_in(payload, &Places::built_in(home), sessions)
}

/// Decides one PreToolUse `payload` by the rule table in `places`, reading the session list from
/// `sessions`. The hook gives it the places of the instance the process runs
/// ([`Places::active`]).
#[must_use]
pub fn decide_in(payload: &Value, places: &Places, sessions: &SessionList) -> Decision {
    decided(payload, places, sessions).decision
}

/// What the rule table decided of one call, and whether it is a write the conductor lease decides
/// further ([`leased`]).
struct Decided {
    decision: Decision,
    /// The rule table allows the call, it comes from a session conductor's rule decides, and it
    /// writes: Edit, Write or NotebookEdit; SendMessage; or Bash or Monitor running a command the
    /// guard treats as a write ([`rules::bash_writes`]).
    conductor_write: bool,
    /// The rule table allows the call, it comes from a session conductor's rule decides, and it
    /// is Bash or Monitor: a command that is a write to the lease when it names the lease's file
    /// or its state directory ([`Places::names_lease`]), which only the hook knows.
    conductor_command: bool,
}

impl From<Decision> for Decided {
    fn from(decision: Decision) -> Self {
        Self {
            decision,
            conductor_write: false,
            conductor_command: false,
        }
    }
}

/// [`decide_in`], saying also whether the call is a conductor-role write ([`Decided`]).
fn decided(payload: &Value, places: &Places, sessions: &SessionList) -> Decided {
    let text = |value: &Value| value.as_str().map(str::to_owned);
    let input = &payload["tool_input"];
    let session_ref = text(&payload["session_id"]).unwrap_or_default();
    let Some(tool) = text(&payload["tool_name"]) else {
        return Decision {
            session_ref,
            ..Decision::denied("the payload names no tool_name")
        }
        .into();
    };
    let field = match tool.as_str() {
        "Edit" | "Write" => Some("file_path"),
        "NotebookEdit" => Some("notebook_path"),
        "SendMessage" => Some("to"),
        "Bash" | "Monitor" => Some("command"),
        _ => None,
    };
    let target = field.and_then(|field| text(&input[field]));
    let mut decision = Decision {
        repository: String::new(),
        session_ref,
        tool: tool.clone(),
        target: target.clone().unwrap_or_default(),
        verdict: Verdict::Deny,
        reason: String::new(),
    };
    if SUB_AGENT.contains(&tool.as_str()) {
        let (verdict, reason) = sub_agent(&tool, input);
        decision.verdict = verdict;
        decision.reason = reason;
        return decision.into();
    }
    if READ_ONLY.contains(&tool.as_str()) {
        decision.verdict = Verdict::Allow;
        decision.reason = format!("{tool} has no rule and is on the built-in read-only allowlist");
        return decision.into();
    }
    let ruled = RULED.contains(&tool.as_str());
    let session = match session(payload, places, sessions) {
        Ok(session) => session,
        Err(reason) if ruled => {
            decision.reason = format!("{reason}, so {tool} is denied");
            return decision.into();
        }
        Err(reason) => {
            decision.reason = format!(
                "{tool} has no rule and is not on the built-in read-only allowlist ({}); {reason}, \
                 so no role's read_only_tools apply",
                READ_ONLY.join(", ")
            );
            return decision.into();
        }
    };
    decision.repository.clone_from(&session.repository);
    if !ruled {
        let extra = places.read_only_tools(session.rules);
        if extra.contains(&tool) {
            decision.verdict = Verdict::Allow;
            decision.reason = format!("{tool} has no rule and is on the role's read_only_tools");
        } else {
            decision.reason = format!(
                "{tool} has no rule and is on no read-only allowlist: neither the built-in one \
                 ({}) nor the {} role's read_only_tools ({}); a role's read_only_tools in the \
                 config extends it",
                READ_ONLY.join(", "),
                session.rules.role(),
                if extra.is_empty() {
                    "none".to_owned()
                } else {
                    extra.join(", ")
                }
            );
        }
        return decision.into();
    }
    let (verdict, reason) = match (tool.as_str(), target.as_deref()) {
        ("SendMessage", to) => session.message(to, input["recipient"].as_str()),
        (_, None) => (
            Verdict::Deny,
            format!(
                "the {tool} call names no {} in tool_input",
                field.unwrap_or("target")
            ),
        ),
        ("Bash" | "Monitor", Some(command)) => session.bash(command),
        (_, Some(path)) => session.file(&tool, path),
    };
    let conductor = verdict == Verdict::Allow && session.rules == Rules::Conductor;
    let command = matches!(tool.as_str(), "Bash" | "Monitor");
    let conductor_write =
        conductor && (!command || target.as_deref().is_some_and(rules::bash_writes));
    decision.verdict = verdict;
    decision.reason = reason;
    Decided {
        decision,
        conductor_write,
        conductor_command: conductor && command,
    }
}

/// Where the payload's session runs, or why no rule set applies to it.
fn session<'a>(
    payload: &Value,
    places: &'a Places,
    sessions: &'a SessionList,
) -> Result<Session<'a>, String> {
    let Some(cwd) = payload["cwd"].as_str() else {
        return Err("the payload names no cwd".to_owned());
    };
    if !Path::new(cwd).is_absolute() {
        return Err(format!("the session's cwd {cwd:?} is not an absolute path"));
    }
    let real = rules::real(Path::new(cwd));
    let Some((repository, rules)) = places.session(&real) else {
        return Err(format!(
            "the session's cwd {cwd} is outside {}, where every rule set applies",
            places.described()
        ));
    };
    Ok(Session {
        places,
        cwd: real,
        repository,
        rules,
        sessions,
    })
}

/// `conductor guard record-guard-decision`: the command `conductor.dispatch.RecordGuardDecision`.
///
/// With `--from-pre-tool-use` it is the PreToolUse hook: it reads the payload on standard input,
/// decides it, records a denial in the store under the state directory (`--state-dir`, else the
/// instance's state when a config file names one, else nowhere: [`recorded_in`]) and answers it
/// as the exit status, 0 or 2. An allowance is
/// answered without opening the store; a conductor write reads the guard's copy of the conductor
/// lease first ([`leased`]). It always answers; every failure before a verdict exists is a denial.
///
/// # Errors
///
/// [`NotImplemented`](crate::NotImplemented) without `--from-pre-tool-use`: recording a verdict
/// given as flags is not written yet.
pub fn record_guard_decision(
    state: Option<&Path>,
    args: RecordGuardDecisionArgs,
) -> Result<ExitCode> {
    if !args.from_pre_tool_use {
        return not_implemented("guard record-guard-decision");
    }
    Ok(hook(state))
}

/// The hook: decide; for a denial, record it within the deadline; answer.
fn hook(state: Option<&Path>) -> ExitCode {
    panic::set_hook(Box::new(|info| {
        eprintln!("conductor guard: {}", one_line(&info.to_string()));
    }));
    let decision = panic::catch_unwind(|| {
        let mut payload = Vec::new();
        if let Err(error) = io::stdin().read_to_end(&mut payload) {
            return Decision::denied(format!("the PreToolUse payload could not be read: {error}"));
        }
        let payload: Value = match serde_json::from_slice(&payload) {
            Ok(payload) => payload,
            Err(error) => {
                return Decision::denied(format!(
                    "the PreToolUse payload on standard input is not JSON: {error}"
                ));
            }
        };
        match std::env::var_os("HOME").map(PathBuf::from) {
            Some(home) if home.is_absolute() => {
                let places = Places::active(&home, crate::config::active())
                    .with_config_file(&config_file(&home))
                    .with_tmp(std::env::var_os("TMPDIR").map(PathBuf::from).as_deref());
                let Decided {
                    decision,
                    conductor_write,
                    conductor_command,
                } = decided(&payload, &places, &SessionList::default());
                if conductor_write
                    || (conductor_command
                        && places.names_lease(&decision.target, recorded_in(state).as_deref()))
                {
                    leased(state, decision)
                } else {
                    decision
                }
            }
            _ => Decision {
                session_ref: payload["session_id"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
                tool: payload["tool_name"].as_str().unwrap_or_default().to_owned(),
                ..Decision::denied("HOME is not an absolute path, so no rule can be applied")
            },
        }
    })
    .unwrap_or_else(|_| Decision::denied("the guard failed while deciding the call"));

    match decision.verdict {
        Verdict::Allow => ExitCode::SUCCESS,
        Verdict::Deny => {
            match recorded_in(state) {
                Some(dir) => {
                    if let Err(error) = record_within(Some(dir), decision.clone()) {
                        eprintln!(
                            "conductor guard: the verdict was not recorded: {}",
                            one_line(&format!("{error:#}"))
                        );
                    }
                }
                None => eprintln!(
                    "conductor guard: the verdict was not recorded: no --state-dir is given and \
                     no config file names a state directory"
                ),
            }
            eprintln!("conductor guard: denied: {}", one_line(&decision.reason));
            ExitCode::from(DENY_EXIT)
        }
    }
}

/// The state directory a denial is recorded in: `--state-dir`, else the state of the instance a
/// config file names. Without either there is none: the hook runs in any session's working
/// directory, and `state/` under it would plant a store in that session's repository.
fn recorded_in(state: Option<&Path>) -> Option<PathBuf> {
    state.map(Path::to_path_buf).or_else(|| {
        let active = crate::config::active();
        active
            .from_file
            .then(|| PathBuf::from(&active.instance.state))
    })
}

/// `decision`, a conductor-role write the rule table allows, decided again by the conductor lease
/// of the instance this process runs (`story:lease-guard`): denied when the lease names another
/// session than the payload's and the holder's process lives ([`crate::session::lives`]), naming
/// the holder's session id and pid; otherwise `decision` as it is.
///
/// The lease is read from the guard's copy of it, [`crate::session::LEASE_FILE`] under the state
/// directory ([`recorded_in`]), which `session claim` writes once the store kept the claim; the
/// store is not opened. No state directory, no such regular file, one that does not read, or one
/// naming another instance leaves `decision` as it is: the SessionStart claim
/// (`crate::session::claim`) is where a session takes the lease.
fn leased(state: Option<&Path>, decision: Decision) -> Decision {
    let Some(holder) = recorded_in(state).and_then(|dir| crate::session::LeaseFile::read(&dir))
    else {
        return decision;
    };
    if holder.instance != crate::config::active().instance.name.0
        || holder.session_id == decision.session_ref
        || !crate::session::lives(holder.pid)
    {
        return decision;
    }
    let reason = format!(
        "{} is denied: session {} (pid {}) holds the conductor lease of instance {} and its \
         process lives: this session is a second conductor of the instance and must stop; it \
         may read, and writes nothing",
        decision.tool, holder.session_id, holder.pid, holder.instance
    );
    Decision {
        verdict: Verdict::Deny,
        reason,
        ..decision
    }
}

/// The config file this process resolves: [`config::locate`] with no flag over its own
/// environment, so `CONDUCTOR_CONFIG`, else [`config::DEFAULT_FILE`] under `home`. A relative
/// `CONDUCTOR_CONFIG` is taken from the working directory, where reading it takes it. When the
/// environment cannot be read (no working directory), no file was read either, and the default
/// file is the one a write could make the next process read.
fn config_file(home: &Path) -> PathBuf {
    config::Environment::from_process()
        .ok()
        .and_then(|environment| {
            let (file, _) = config::locate(None, &environment).ok()?;
            Some(environment.cwd.join(file))
        })
        .unwrap_or_else(|| home.join(config::DEFAULT_FILE))
}

/// Records `decision` on a thread of its own, and waits for it at most [`RECORD_DEADLINE`]. A
/// recording still waiting for the store's writer lock, or still replaying the store's files as
/// it opens, is then abandoned with the process; an append cut off leaves event files no commit
/// names, which the store never reads.
fn record_within(state: Option<PathBuf>, decision: Decision) -> Result<()> {
    let decided_at = OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .context("format the time of the verdict")?;
    let (sender, receiver) = mpsc::channel();
    thread::Builder::new()
        .name("guard-record".to_owned())
        .spawn(move || {
            let _ = sender.send(record(state.as_deref(), decision, decided_at));
        })
        .context("start recording the verdict")?;
    match receiver.recv_timeout(RECORD_DEADLINE) {
        Ok(recorded) => recorded,
        Err(RecvTimeoutError::Timeout) => bail!(
            "the store did not take it within {} s: another writer holds its lock, or opening it \
             took that long",
            RECORD_DEADLINE.as_secs()
        ),
        Err(RecvTimeoutError::Disconnected) => bail!("recording it failed before it answered"),
    }
}

/// Records `decision` through the generated `RecordGuardDecision` behaviour.
fn record(state: Option<&Path>, decision: Decision, decided_at: String) -> Result<()> {
    let mut generated = Generated::new(Ports {
        store: crate::state::open(state)?,
    });
    let RecordGuardDecisionOutcome::Recorded { .. } = generated
        .record_guard_decision(RecordGuardDecision {
            repository: RepositoryName(decision.repository),
            session_ref: decision.session_ref,
            tool: decision.tool,
            target: decision.target,
            verdict: decision.verdict,
            reason: decision.reason,
            decided_at: Timestamp(decided_at),
        })
        .map_err(|unmet| anyhow!("guard record-guard-decision: {unmet}"))?;
    generated.ports.store.check().context("keep the verdict")?;
    Ok(())
}

/// The ports `RecordGuardDecision` runs over: the store, and the identity it assigns.
struct Ports {
    store: Store,
}

impl GuardDecisionStorage for Ports {
    fn get(&self, identity: &GuardDecisionId) -> Option<GuardDecisionSnapshot> {
        GuardDecisionStorage::get(&self.store, identity)
    }

    fn put(&mut self, snapshot: GuardDecisionSnapshot) {
        GuardDecisionStorage::put(&mut self.store, snapshot);
    }

    fn delete(&mut self, identity: &GuardDecisionId) {
        GuardDecisionStorage::delete(&mut self.store, identity);
    }

    fn list(&self) -> Vec<GuardDecisionSnapshot> {
        GuardDecisionStorage::list(&self.store)
    }
}

impl TryContext for Ports {
    fn try_generate_conductor_direction_serving_id(
        &mut self,
    ) -> Result<ServingId, UnmetObligation> {
        Err(unmet_context("conductor.direction.ServingId"))
    }

    fn try_generate_conductor_dispatch_guard_decision_id(
        &mut self,
    ) -> Result<GuardDecisionId, UnmetObligation> {
        Ok(GuardDecisionId(Uuid(eventlog_core::new_event_id())))
    }

    fn try_generate_conductor_dispatch_message_id(&mut self) -> Result<MessageId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.MessageId"))
    }

    fn try_generate_conductor_dispatch_resource_request_id(
        &mut self,
    ) -> Result<ResourceRequestId, UnmetObligation> {
        Err(unmet_context("conductor.dispatch.ResourceRequestId"))
    }

    fn try_generate_conductor_observation_board_id(&mut self) -> Result<BoardId, UnmetObligation> {
        Err(unmet_context("conductor.observation.BoardId"))
    }

    fn try_generate_conductor_observation_observation_id(
        &mut self,
    ) -> Result<ObservationId, UnmetObligation> {
        Err(unmet_context("conductor.observation.ObservationId"))
    }

    fn try_generate_conductor_observation_snapshot_id(
        &mut self,
    ) -> Result<SnapshotId, UnmetObligation> {
        Err(unmet_context("conductor.observation.SnapshotId"))
    }
}

/// `text` on one line: every line break a space.
fn one_line(text: &str) -> String {
    text.split(['\n', '\r'])
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

/// `conductor guard guard-decisions`: the view `conductor.dispatch.GuardDecisions`, served by the
/// generated query over the existing store under the state directory (`--state-dir`, else
/// `state/`; [`crate::state::dir`]) and rendered as `--format` asks. A view creates no store.
///
/// Lists every decision it can read, and names each record it cannot on standard error, after
/// them, exiting 1 ([`crate::store::show`]).
///
/// # Errors
///
/// There is no store, it does not open, a read of it fails for a reason other than a record this
/// build cannot read, the generated query refuses a row, or the rows cannot be written to standard
/// output.
pub fn guard_decisions(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    /// The view's fields, in the specification's order (`spec/domains/dispatch.yaml`).
    const COLUMNS: [&str; 8] = [
        "guard_decision_id",
        "repository",
        "session_ref",
        "tool",
        "target",
        "verdict",
        "reason",
        "decided_at",
    ];

    let generated = Generated::new(crate::state::open_existing(state)?);
    let rows = generated
        .guard_decisions()
        .map_err(|unmet| anyhow!("guard guard-decisions: {unmet}"))?;
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.guard_decision_id.0.0),
                Value::from(row.repository.0),
                Value::from(row.session_ref),
                Value::from(row.tool),
                Value::from(row.target),
                Value::from(verdict_name(row.verdict)),
                Value::from(row.reason),
                Value::from(row.decided_at.0),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "guard decisions",
        &view.render(&COLUMNS, &rows),
    )
}

/// The verdict's name as the specification spells it. Exhaustive, so a verdict the specification
/// adds fails to compile here rather than rendering under a guessed name.
fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Allow => "Allow",
        Verdict::Deny => "Deny",
    }
}
