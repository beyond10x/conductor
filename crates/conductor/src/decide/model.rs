//! The `Decider` adapter (`story:decide-model-adapter`, `docs/design/conductor-run.md` § 3): one
//! bounded model call answers a class-C decision request with one letter and one line of reason.
//!
//! [`ClaudePrint`] is the first adapter: `claude -p` on the model of the instance's conductor role
//! (`roles[].model`; the controller role's when the instance has no conductor role), the prompt on
//! standard input, `--output-format json`, no tools (`--tools=`) and no settings file
//! (`--setting-sources=`), nor MCP servers, a saved session or skills. It runs in the instance's
//! cache directory, `decide/`, where no project instructions are found. The answer's text is
//! `<LETTER>: <reason>` ([`parse_answer`]); anything else is refused. Whether the letter is one of
//! the request's options is checked by the caller, for every adapter alike
//! ([`super::handle_request`]). [`Fake`] answers what it was built with, for tests and dry runs.
//! `llm` and Loom replace `claude -p` behind the same trait.

use std::io::{Read as _, Write as _};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::config::{Instance, RoleName};
use serde_json::Value;

use crate::config;

/// How long `claude -p` may take before it is stopped and the request escalated.
pub const TIMEOUT: Duration = Duration::from_secs(300);

/// One letter of the request's options, and one line of why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    /// The option's letter, one upper-case ASCII letter.
    pub letter: String,
    /// Why, on one line.
    pub reason: String,
}

/// An adapter that answers a class-C request from one bounded prompt.
pub trait Decider {
    /// The adapter's name, recorded as the decision's `decider_name`.
    fn name(&self) -> String;

    /// The answer to `prompt`.
    ///
    /// # Errors
    ///
    /// The adapter gave no answer of the form `<LETTER>: <reason>`.
    fn decide(&self, prompt: &str) -> Result<Answer>;
}

/// `claude -p` on one model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaudePrint {
    model: String,
}

impl ClaudePrint {
    /// The adapter on `model`.
    #[must_use]
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_owned(),
        }
    }

    /// The adapter on the model of `instance`'s conductor role, or of its controller role when it
    /// has no conductor role.
    ///
    /// # Errors
    ///
    /// The instance has neither role.
    pub fn for_instance(instance: &Instance) -> Result<Self> {
        let role = |name: RoleName| {
            instance
                .roles
                .iter()
                .find(|role| config::is_role(&role.role, name))
        };
        let role = role(RoleName::Director)
            .or_else(|| role(RoleName::Lead))
            .ok_or_else(|| {
                anyhow!(
                    "instance {:?} has neither a conductor nor a controller role, so no model \
                     answers a decision request",
                    instance.name.0
                )
            })?;
        Ok(Self::new(&role.model.0))
    }

    /// The model it runs.
    #[must_use]
    pub fn model(&self) -> &str {
        &self.model
    }

    /// The arguments `claude` is run with; the prompt goes on standard input.
    #[must_use]
    pub fn arguments(&self) -> Vec<String> {
        [
            "-p",
            "--model",
            &self.model,
            "--output-format",
            "json",
            "--tools=",
            "--setting-sources=",
            "--strict-mcp-config",
            "--no-session-persistence",
            "--disable-slash-commands",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    }

    /// Where `claude` runs: `decide/` of the instance's cache directory, made when it is not
    /// there.
    fn dir() -> Result<PathBuf> {
        let dir = PathBuf::from(&config::active().instance.cache).join("decide");
        std::fs::create_dir_all(&dir)
            .with_context(|| format!("make the decider's directory {}", dir.display()))?;
        Ok(dir)
    }
}

impl Decider for ClaudePrint {
    fn name(&self) -> String {
        format!("claude-print/{}", self.model)
    }

    fn decide(&self, prompt: &str) -> Result<Answer> {
        let mut child = Command::new("claude")
            .args(self.arguments())
            .current_dir(Self::dir()?)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .context("start `claude -p`")?;
        let mut stdin = child
            .stdin
            .take()
            .context("the standard input of `claude -p`")?;
        let prompt = prompt.to_owned();
        let writer = thread::spawn(move || stdin.write_all(prompt.as_bytes()));
        let mut stdout = child.stdout.take().context("the output of `claude -p`")?;
        let reader = thread::spawn(move || {
            let mut text = String::new();
            stdout.read_to_string(&mut text).map(|_| text)
        });
        let mut stderr = child.stderr.take().context("the errors of `claude -p`")?;
        let errors = thread::spawn(move || {
            let mut text = String::new();
            stderr.read_to_string(&mut text).map(|_| text)
        });
        let started = Instant::now();
        let status = loop {
            if let Some(status) = child.try_wait().context("wait for `claude -p`")? {
                break status;
            }
            if started.elapsed() > TIMEOUT {
                // The child is this process's own, killed by its handle.
                child.kill().context("stop `claude -p`")?;
                child.wait().context("reap `claude -p`")?;
                bail!("`claude -p` gave no answer in {} s", TIMEOUT.as_secs());
            }
            thread::sleep(Duration::from_millis(100));
        };
        writer
            .join()
            .map_err(|_| anyhow!("the prompt's writer panicked"))?
            .context("write the prompt to `claude -p`")?;
        let output = reader
            .join()
            .map_err(|_| anyhow!("the answer's reader panicked"))?
            .context("read the answer of `claude -p`")?;
        let errors = errors
            .join()
            .map_err(|_| anyhow!("the error reader panicked"))?
            .unwrap_or_default();
        if !status.success() {
            bail!(
                "`claude -p` exited {status}: {}",
                errors.lines().next().unwrap_or_default()
            );
        }
        parse_output(&output)
    }
}

/// A decider that answers what it was built with and keeps every prompt it was given.
#[derive(Debug)]
pub struct Fake {
    answer: Answer,
    prompts: Mutex<Vec<String>>,
}

impl Fake {
    /// A fake that answers `letter` for `reason`.
    #[must_use]
    pub fn answering(letter: &str, reason: &str) -> Self {
        Self {
            answer: Answer {
                letter: letter.to_owned(),
                reason: reason.to_owned(),
            },
            prompts: Mutex::new(Vec::new()),
        }
    }

    /// Every prompt it was given, in order.
    #[must_use]
    pub fn prompts(&self) -> Vec<String> {
        self.prompts
            .lock()
            .map(|prompts| prompts.clone())
            .unwrap_or_default()
    }
}

impl Decider for Fake {
    fn name(&self) -> String {
        "fake".to_owned()
    }

    fn decide(&self, prompt: &str) -> Result<Answer> {
        self.prompts
            .lock()
            .map_err(|_| anyhow!("the fake decider's prompts are poisoned"))?
            .push(prompt.to_owned());
        Ok(self.answer.clone())
    }
}

/// The answer in `text`: its first line that holds anything, `<LETTER>: <reason>`, the letter one
/// upper-case ASCII letter (emphasis around it, `**A**` or `` `A` ``, is dropped) and the reason
/// not blank.
///
/// # Errors
///
/// The text holds no such line.
pub fn parse_answer(text: &str) -> Result<Answer> {
    let line = text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .ok_or_else(|| anyhow!("the answer is empty; it is `<LETTER>: <reason>`"))?;
    let refused = || anyhow!("the answer {line:?} is not `<LETTER>: <reason>`");
    let (letter, reason) = line.split_once(':').ok_or_else(refused)?;
    let letter = letter.trim().trim_matches(['*', '`', '_']);
    let reason = reason.trim();
    let mut chars = letter.chars();
    match (chars.next(), chars.next()) {
        (Some(first), None) if first.is_ascii_uppercase() && !reason.is_empty() => Ok(Answer {
            letter: letter.to_owned(),
            reason: reason.to_owned(),
        }),
        _ => Err(refused()),
    }
}

/// The answer in the standard output of `claude -p --output-format json`: one JSON object whose
/// `result` holds the answer's text ([`parse_answer`]).
///
/// # Errors
///
/// The output is no JSON object, it says the run failed (`is_error`, or a `subtype` other than
/// `success`), or its `result` is no answer.
pub fn parse_output(stdout: &str) -> Result<Answer> {
    let output: Value = serde_json::from_str(stdout.trim())
        .context("the output of `claude -p` is no JSON object")?;
    let subtype = output["subtype"].as_str().unwrap_or_default();
    if output["is_error"].as_bool() == Some(true) || subtype != "success" {
        bail!("`claude -p` failed: {subtype:?}");
    }
    let result = output["result"]
        .as_str()
        .ok_or_else(|| anyhow!("the output of `claude -p` holds no `result`"))?;
    parse_answer(result)
}
