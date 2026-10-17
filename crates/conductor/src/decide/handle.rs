//! A `[DECISION-REQUEST]` handled in code (`story:decide-model-adapter`,
//! `docs/design/conductor-run.md` § 3): [`classify`] chooses its class by rules over its question
//! and lettered options; [`handle_request`] raises it with that rule, answers class C through the
//! `Decider` adapter ([`super::model`]) with one prompt of at most [`PROMPT_CAP`] characters
//! ([`build_prompt`]) and records the answer, and escalates class O and H to the Decisions page.
//!
//! **The request.** Its first line is `[DECISION-REQUEST <repo> <request-id>] <question>` (design
//! § 6); below it, each option on a line of its own, `A: <label>` or `A) <label>`, optionally led
//! by `- ` and with its cost as `(cost: <cost>)` or `; cost: <cost>`; and a line that opens with
//! `Recommendation` (or `Recommended`) naming one of the options' letters ([`parse_request`]).
//!
//! **Class.** A rule reads whole words of the question and the option lines, never the
//! recommendation or other lines, in this order, the first that matches deciding: `data-loss`
//! (delete, erase, purge, wipe, destroy, discard, the only copy), `history-rewrite` (a force-push,
//! `filter-branch` or `filter-repo`, a rewrite of history, an amend or rebase of what was
//! published or pushed), `repository-lifecycle` (a new, created, renamed, retired, archived,
//! forked, transferred or deprecated repository), `architecture` (an ADR, architecture, a
//! cross-repository contract, a technology or framework choice) → class O; `credential` (a
//! secret, a token, a password, a qualified key such as an API or SSH key, a login, a paid
//! account or plan) and `hands` (a permission only the operator holds, administrator rights, a
//! grant of a permission or bypass) → class H; otherwise class C, rule `none`.
//!
//! **Class C.** The prompt holds the request and its options, the repository's accepted ADR
//! titles (`aep plan artifact list --kind architecture-decision-record --status accepted` in the
//! instance's checkout of it, when it has a planning store), the operator's decision rules
//! (`<records>/rules-v2/all.md` when it exists, else the items under the heading `How the operator
//! decides` of `<records>/rules.md`, any heading that opens with those words), and the
//! repository's conductor decisions of the last 7 days from the records' decision log, oldest
//! first. Over the cap, the oldest decisions are dropped first, then the rules are cut, then ADR
//! titles dropped; the request and the answer's format are never cut. The answer's letter must be
//! one of the request's options; it is recorded with `record-conductor-decision` (class C,
//! evidence `decider=<name> prompt=<sha256>`, and the two as `decider_name` and `prompt_sha256`),
//! the request is answered with it, and the `[DECISION <request-id>] <letter> <reason>
//! (<decision-id>)` line is returned for delivery. An answer whose letter is no option, or no
//! answer, is not recorded: the request is escalated as class O, an unclear class being O
//! (`AGENTS.md` rule 10).
//!
//! **Class O and H.** `escalate-request` with `--kind one-of`, the request's lettered options,
//! its recommendation as the recommended and the default letter, and a deadline
//! [`DEADLINE_HOURS`] from now; class H also says that no dry run was measured.

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::decision::{
    ClassificationRule, DecisionClass, DecisionId, PromptHash, RequestOption,
};
use conductor_model::dispatch::MessageKind;
use conductor_model::observation::RepositoryName;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

use super::model::Decider;
use super::{Adapter, Allocated, Fields, Refusal, conductor_decided, option_value, rule_name};
use crate::cli::{AnswerRequestArgs, EscalateRequestArgs, RaiseDecisionRequestArgs};
use crate::{config, message, records, rules};

/// The most characters a prompt to the `Decider` adapter holds.
pub const PROMPT_CAP: usize = 16_000;

/// How long an escalated request waits for the operator before its default stands.
pub const DEADLINE_HOURS: i64 = 48;

/// How many days back the prompt reads the repository's decisions.
pub const DECISION_DAYS: i64 = 7;

/// The heading of `rules.md` whose items the prompt carries.
pub const OPERATOR_DECIDES: &str = "How the operator decides";

/// The cost an option that names none is given on the Decisions page.
const NO_COST: &str = "not stated in the request";

/// What a cut part of the prompt ends with.
const CUT: &str = "\n[cut at the prompt's 16,000-character cap]\n";

/// Phrases of whole lower-case words, each a reason for its rule.
const DATA_LOSS: &[&str] = &[
    "delete",
    "deletes",
    "deleted",
    "deleting",
    "deletion",
    "erase",
    "erases",
    "erased",
    "erasing",
    "erasure",
    "purge",
    "purges",
    "purged",
    "purging",
    "wipe",
    "wipes",
    "wiped",
    "wiping",
    "destroy",
    "destroys",
    "destroyed",
    "destroying",
    "discard",
    "discards",
    "discarded",
    "discarding",
    "rm rf",
    "only copy",
];

const HISTORY_REWRITE: &[&str] = &[
    "force push",
    "force pushed",
    "force pushes",
    "force pushing",
    "push force",
    "push f",
    "force with lease",
    "filter branch",
    "filter repo",
    "history rewrite",
];

/// Words that rewrite history when the text also names `history`.
const REWRITE: &[&str] = &["rewrite", "rewrites", "rewriting", "rewritten"];

/// Words that rewrite a commit, a history rewrite when the text also names it [`PUBLISHED`].
const AMEND: &[&str] = &[
    "amend", "amends", "amended", "amending", "rebase", "rebases", "rebased", "rebasing",
];

const PUBLISHED: &[&str] = &["published", "pushed", "released"];

const REPOSITORY: &[&str] = &["repository", "repositories", "repo", "repos"];

/// Words that start, rename or end a repository, read up to three words before a
/// [`REPOSITORY`] word or two after it.
const LIFECYCLE: &[&str] = &[
    "new",
    "create",
    "creates",
    "created",
    "creating",
    "rename",
    "renames",
    "renamed",
    "renaming",
    "retire",
    "retires",
    "retired",
    "retiring",
    "archive",
    "archives",
    "archived",
    "archiving",
    "fork",
    "forks",
    "forked",
    "forking",
    "transfer",
    "transfers",
    "transferred",
    "deprecate",
    "deprecated",
    "sunset",
];

const ARCHITECTURE: &[&str] = &[
    "adr",
    "adrs",
    "architecture",
    "architectural",
    "cross repository",
    "cross repo",
    "technology",
    "technologies",
    "tech stack",
    "framework",
    "frameworks",
    "programming language",
];

const CREDENTIAL: &[&str] = &[
    "secret",
    "secrets",
    "token",
    "tokens",
    "password",
    "passwords",
    "passphrase",
    "credential",
    "credentials",
    "login",
    "logins",
    "log in",
    "logs in",
    "logged in",
    "sign in",
    "signs in",
    "api key",
    "ssh key",
    "deploy key",
    "signing key",
    "private key",
    "access key",
    "secret key",
    "gpg key",
    "license key",
    "key pair",
    "paid account",
    "paid plan",
    "paid tier",
    "subscription",
    "billing",
    "credit card",
];

const HANDS: &[&str] = &[
    "only the operator",
    "operator holds",
    "operator s hands",
    "operator s permission",
    "admin permission",
    "admin rights",
    "admin access",
    "administrator",
    "owner role",
    "organization owner",
    "org owner",
    "permission only",
    "permissions only",
];

/// Words that grant, a hands case when the text also names a [`PERMISSION`].
const GRANT: &[&str] = &["grant", "grants", "granted", "granting"];

const PERMISSION: &[&str] = &["permission", "permissions", "bypass", "rights"];

/// The question and lettered options of a request's text, lower case, every run of characters
/// that are not letters or digits one space, with a space at either end.
struct Words {
    text: String,
}

impl Words {
    fn of(request: &str) -> Self {
        let mut lines = request.lines();
        let first = lines.next().unwrap_or_default();
        let question = first.split_once(']').map_or(first, |(_, rest)| rest);
        let mut read = String::from(question);
        for line in lines {
            if option_line(line).is_some() {
                read.push(' ');
                read.push_str(line);
            }
        }
        let mut text = String::from(" ");
        for c in read.chars().flat_map(char::to_lowercase) {
            if c.is_alphanumeric() {
                text.push(c);
            } else if !text.ends_with(' ') {
                text.push(' ');
            }
        }
        if !text.ends_with(' ') {
            text.push(' ');
        }
        Self { text }
    }

    fn has(&self, phrases: &[&str]) -> bool {
        phrases
            .iter()
            .any(|phrase| self.text.contains(&format!(" {phrase} ")))
    }

    /// Whether a word of `before` stands up to three words before a word of `of`, or one of
    /// `after` up to two after it.
    fn near(&self, of: &[&str], before: &[&str], after: &[&str]) -> bool {
        let words: Vec<&str> = self.text.split_whitespace().collect();
        words.iter().enumerate().any(|(at, word)| {
            of.contains(word)
                && (words[at.saturating_sub(3)..at]
                    .iter()
                    .any(|word| before.contains(word))
                    || words[at + 1..words.len().min(at + 3)]
                        .iter()
                        .any(|word| after.contains(word)))
        })
    }
}

/// The class and rule of the request `text` holds, as the module documentation lists the rules.
#[must_use]
pub fn classify(text: &str) -> (DecisionClass, ClassificationRule) {
    let words = Words::of(text);
    let rule = if words.has(DATA_LOSS) {
        ClassificationRule::DataLoss
    } else if words.has(HISTORY_REWRITE)
        || (words.has(REWRITE) && words.has(&["history"]))
        || (words.has(AMEND) && words.has(PUBLISHED))
    {
        ClassificationRule::HistoryRewrite
    } else if words.near(REPOSITORY, LIFECYCLE, LIFECYCLE) {
        ClassificationRule::RepositoryLifecycle
    } else if words.has(ARCHITECTURE) {
        ClassificationRule::Architecture
    } else if words.has(CREDENTIAL) {
        ClassificationRule::Credential
    } else if words.has(HANDS) || (words.has(GRANT) && words.has(PERMISSION)) {
        ClassificationRule::Hands
    } else {
        ClassificationRule::None
    };
    (class_of(rule), rule)
}

/// The class `rule` gives a request.
#[must_use]
pub const fn class_of(rule: ClassificationRule) -> DecisionClass {
    match rule {
        ClassificationRule::DataLoss
        | ClassificationRule::HistoryRewrite
        | ClassificationRule::RepositoryLifecycle
        | ClassificationRule::Architecture => DecisionClass::O,
        ClassificationRule::Credential | ClassificationRule::Hands => DecisionClass::H,
        ClassificationRule::None => DecisionClass::C,
    }
}

/// A request read from its text ([`parse_request`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedRequest {
    /// The repository its first line names.
    pub repository: String,
    /// The request's id, from its first line.
    pub request_id: String,
    /// The words after the first line's brackets.
    pub question: String,
    /// The lettered options, in the order written.
    pub options: Vec<RequestOption>,
    /// The option lines as written, trimmed.
    pub option_lines: Vec<String>,
    /// What the recommendation line says after its first `:`.
    pub recommendation: String,
    /// The option letter the recommendation names.
    pub recommended: String,
}

impl ParsedRequest {
    /// The options' letters, joined by `, `.
    #[must_use]
    pub fn letters(&self) -> String {
        self.options
            .iter()
            .map(|option| option.letter.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The options as one text, as `raise-decision-request --options` and the decision record
    /// hold them: the option lines joined by ` | `.
    #[must_use]
    pub fn options_text(&self) -> String {
        self.option_lines.join(" | ")
    }

    fn offers(&self, letter: &str) -> bool {
        self.options.iter().any(|option| option.letter == letter)
    }
}

/// The letter and the rest of an option line: `A: …` or `A) …`, optionally led by `- ` or `* `.
fn option_line(line: &str) -> Option<(String, String)> {
    let line = line.trim();
    let line = line
        .strip_prefix("- ")
        .or_else(|| line.strip_prefix("* "))
        .unwrap_or(line)
        .trim_start();
    let mut chars = line.chars();
    let letter = chars.next().filter(char::is_ascii_uppercase)?;
    let rest = chars.as_str();
    let rest = rest.strip_prefix(':').or_else(|| rest.strip_prefix(')'))?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let text = rest.trim();
    (!text.is_empty()).then(|| (letter.to_string(), text.to_owned()))
}

/// An option's label and cost, from the text after its letter: the cost follows `cost:`, in
/// any case, inside parentheses or after `;`; without one the cost is [`NO_COST`].
fn label_and_cost(text: &str) -> (String, String) {
    // ASCII lower case keeps every byte where it was, so `at` indexes `text` too.
    let Some(at) = text.to_ascii_lowercase().find("cost:") else {
        return (text.to_owned(), NO_COST.to_owned());
    };
    let label = text[..at]
        .trim_end_matches(|c: char| c.is_whitespace() || "(;,—–-".contains(c))
        .to_owned();
    let cost = text[at + "cost:".len()..]
        .trim()
        .trim_end_matches(')')
        .trim()
        .to_owned();
    let label = if label.is_empty() {
        text.to_owned()
    } else {
        label
    };
    let cost = if cost.is_empty() {
        NO_COST.to_owned()
    } else {
        cost
    };
    (label, cost)
}

/// The request `text` holds, as the module documentation describes it.
///
/// # Errors
///
/// Its first line is no `[DECISION-REQUEST <repo> <request-id>] <question>`, it lists no lettered
/// option or a letter twice, or no recommendation line names one of the options' letters.
pub fn parse_request(text: &str) -> Result<ParsedRequest> {
    let first = text.lines().next().unwrap_or_default().trim();
    let reading = message::read(first);
    if reading.kind != Some(MessageKind::DecisionRequest) || !reading.parsed {
        bail!("the first line {first:?} is no `[DECISION-REQUEST <repo> <request-id>] <question>`");
    }
    let (inside, question) = first
        .trim_start_matches('[')
        .split_once(']')
        .ok_or_else(|| anyhow!("the first line {first:?} has no `]`"))?;
    let mut fields = inside.split_whitespace().skip(1);
    let (Some(repository), Some(request_id)) = (fields.next(), fields.next()) else {
        bail!("the first line {first:?} names no repository and request id");
    };
    let mut options: Vec<RequestOption> = Vec::new();
    let mut option_lines = Vec::new();
    let mut recommendation = None;
    for line in text.lines().skip(1) {
        let lower = line.trim().to_lowercase();
        if lower.starts_with("recommend") && recommendation.is_none() {
            let line = line.trim();
            let said = line.split_once(':').map_or(line, |(_, rest)| rest).trim();
            recommendation = Some(said.to_owned());
        } else if let Some((letter, rest)) = option_line(line) {
            if options.iter().any(|option| option.letter == letter) {
                bail!("request {request_id:?} lists the option {letter:?} twice");
            }
            let (label, cost) = label_and_cost(&rest);
            options.push(RequestOption {
                letter,
                label,
                cost,
            });
            option_lines.push(line.trim().to_owned());
        }
    }
    if options.is_empty() {
        bail!(
            "request {request_id:?} lists no lettered option: one line each, `A: <label> (cost: \
             <cost>)`"
        );
    }
    let recommendation = recommendation.ok_or_else(|| {
        anyhow!("request {request_id:?} has no `Recommendation: <letter>, <why>` line")
    })?;
    let recommended = recommendation
        .split(|c: char| !c.is_ascii_alphanumeric())
        .find(|word| options.iter().any(|option| option.letter == *word))
        .ok_or_else(|| {
            anyhow!(
                "the recommendation of request {request_id:?}, {recommendation:?}, names none of \
                 its letters"
            )
        })?
        .to_owned();
    Ok(ParsedRequest {
        repository: repository.to_owned(),
        request_id: request_id.to_owned(),
        question: question.trim().to_owned(),
        options,
        option_lines,
        recommendation,
        recommended,
    })
}

/// What the prompt holds besides the request ([`prompt_inputs`]).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PromptInputs {
    /// The repository's accepted ADRs, one line each.
    pub adr_titles: Vec<String>,
    /// The operator's decision rules.
    pub rules: String,
    /// The repository's conductor decisions of the last [`DECISION_DAYS`] days, one line each,
    /// oldest first.
    pub decisions: Vec<String>,
}

/// The prompt for `request`, at most [`PROMPT_CAP`] characters, in this order: what is asked,
/// the request and its options, the accepted ADR titles, the operator's rules, the decisions of
/// the last days, and the answer's format with the letters it may name. Over the cap, the oldest
/// decisions are dropped first, then the rules cut, then ADR titles dropped from the last; the
/// request is cut only when it alone is over the cap, and the answer's format never.
#[must_use]
pub fn build_prompt(request: &ParsedRequest, inputs: &PromptInputs) -> String {
    let head = format!(
        "You answer one decision request for conductor, which coordinates coding-agent sessions \
         across one organization's repositories. The request is class C: conductor decides it \
         on the operator's standing grant, with the option the operator would choose, and the \
         one that can be undone when they are otherwise equal.\n\n## Request\n\nRepository: \
         {}\nRequest: {}\nQuestion: {}\nOptions:\n{}\nRecommendation: {}\n",
        request.repository,
        request.request_id,
        request.question,
        request
            .options
            .iter()
            .map(|option| format!(
                "{}: {} (cost: {})\n",
                option.letter, option.label, option.cost
            ))
            .collect::<String>(),
        request.recommendation,
    );
    let tail = format!(
        "\nAnswer with exactly one line, `<LETTER>: <one-line reason>`, LETTER one of {}.\n",
        request.letters()
    );
    let mut adr_titles = inputs.adr_titles.clone();
    // Trimmed here as it is printed, so the cut below counts exactly what the prompt holds.
    let mut rules = inputs.rules.trim_end().to_owned();
    let mut decisions: &[String] = &inputs.decisions;
    let assemble = |adr_titles: &[String], rules: &str, decisions: &[String], head: &str| {
        let list = |lines: &[String]| {
            if lines.is_empty() {
                "none\n".to_owned()
            } else {
                lines.iter().map(|line| format!("- {line}\n")).collect()
            }
        };
        format!(
            "{head}\n## Accepted architecture decision records of {repo}\n\n{adrs}\n## How the \
             operator decides\n\n{rules}\n\n## Conductor's decisions on {repo} in the last \
             {DECISION_DAYS} days, oldest first\n\n{decisions}{tail}",
            repo = request.repository,
            adrs = list(adr_titles),
            rules = if rules.trim().is_empty() {
                "none"
            } else {
                rules
            },
            decisions = list(decisions),
        )
    };
    let size = |text: &str| text.chars().count();
    let mut prompt = assemble(&adr_titles, &rules, decisions, &head);
    while size(&prompt) > PROMPT_CAP && !decisions.is_empty() {
        decisions = &decisions[1..];
        prompt = assemble(&adr_titles, &rules, decisions, &head);
    }
    if size(&prompt) > PROMPT_CAP {
        let over = size(&prompt) - PROMPT_CAP + size(CUT);
        let keep = size(&rules).saturating_sub(over);
        rules = format!("{}{CUT}", rules.chars().take(keep).collect::<String>());
        prompt = assemble(&adr_titles, &rules, decisions, &head);
    }
    while size(&prompt) > PROMPT_CAP && adr_titles.pop().is_some() {
        prompt = assemble(&adr_titles, &rules, decisions, &head);
    }
    if size(&prompt) > PROMPT_CAP {
        let over = size(&prompt) - PROMPT_CAP + size(CUT);
        let keep = size(&head).saturating_sub(over);
        let head = format!("{}{CUT}", head.chars().take(keep).collect::<String>());
        prompt = assemble(&adr_titles, &rules, decisions, &head);
    }
    prompt
}

/// What the prompt for `request` holds besides it, read at `now` from the instance this process
/// runs ([`config::active`]), as the module documentation lists it. A source that is not there
/// gives nothing.
#[must_use]
pub fn prompt_inputs(request: &ParsedRequest, now: OffsetDateTime) -> PromptInputs {
    let instance = &config::active().instance;
    let records = PathBuf::from(&instance.records);
    PromptInputs {
        adr_titles: adr_titles(&Path::new(&instance.checkouts.root).join(&request.repository)),
        rules: operator_rules(&records),
        decisions: recent_decisions(&records, &request.repository, now),
    }
}

/// The accepted ADRs of the planning store of the checkout `checkout`, `<title> (<id>)` each;
/// none when it has no store or `aep` does not answer. Read-only.
fn adr_titles(checkout: &Path) -> Vec<String> {
    if !checkout.join(".engineering").join("planning").is_dir() {
        return Vec::new();
    }
    let Ok(output) = Command::new("aep")
        .args([
            "plan",
            "artifact",
            "list",
            "--kind",
            "architecture-decision-record",
            "--status",
            "accepted",
            "--format",
            "json",
        ])
        .current_dir(checkout)
        .output()
    else {
        return Vec::new();
    };
    if !output.status.success() {
        return Vec::new();
    }
    serde_json::from_slice::<Vec<Value>>(&output.stdout)
        .unwrap_or_default()
        .iter()
        .filter_map(|adr| {
            Some(format!(
                "{} ({})",
                adr["title"].as_str()?,
                adr["id"].as_str().unwrap_or_default()
            ))
        })
        .collect()
}

/// The operator's decision rules: `rules-v2/all.md` of the records when it exists, else every
/// item of `rules.md` under a heading that opens with [`OPERATOR_DECIDES`], in file order.
fn operator_rules(records: &Path) -> String {
    if let Ok(text) = fs::read_to_string(records.join("rules-v2").join("all.md")) {
        return text;
    }
    let Ok(text) = fs::read_to_string(records.join(rules::RULES_FILE)) else {
        return String::new();
    };
    let wanted = OPERATOR_DECIDES.to_lowercase();
    rules::parse(&text)
        .iter()
        .filter(|item| {
            item.headings
                .iter()
                .any(|heading| heading.to_lowercase().starts_with(&wanted))
        })
        .map(|item| item.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Conductor's decisions on `repository` of the last [`DECISION_DAYS`] days before `now`, from
/// the decision log's files of the months they fall in, oldest first: `<at> <id> (class
/// <class>): <question> → <choice>: <reason>`. A line that does not read is passed over.
fn recent_decisions(records: &Path, repository: &str, now: OffsetDateTime) -> Vec<String> {
    let since = now - Duration::days(DECISION_DAYS);
    let month = |at: OffsetDateTime| format!("{:04}-{:02}.jsonl", at.year(), u8::from(at.month()));
    let mut files = vec![month(since), month(now)];
    files.dedup();
    let mut found: Vec<(OffsetDateTime, String)> = Vec::new();
    for file in files {
        let Ok(text) = fs::read_to_string(records.join(records::DECISIONS).join(file)) else {
            continue;
        };
        for line in text.lines() {
            let Ok(line) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let field = |key: &str| line[key].as_str().unwrap_or_default();
            if field("event") != "decided" || field("repo") != repository {
                continue;
            }
            let Ok(at) = OffsetDateTime::parse(field("at"), &Rfc3339) else {
                continue;
            };
            if at < since {
                continue;
            }
            found.push((
                at,
                format!(
                    "{} {} (class {}): {} → {}: {}",
                    field("at"),
                    field("id"),
                    field("class"),
                    one_line(field("question")),
                    field("choice"),
                    one_line(field("reason")),
                ),
            ));
        }
    }
    found.sort_by_key(|(at, _)| *at);
    found.into_iter().map(|(_, line)| line).collect()
}

/// `text` with every line break a space.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

/// The SHA-256 of `prompt`'s UTF-8 bytes, 64 lower-case hexadecimal characters: a
/// `PromptHash`.
#[must_use]
pub fn prompt_sha256(prompt: &str) -> String {
    eventlog_core::blob_integrity_sha256(prompt.as_bytes())
}

/// What [`handle_request`] did with a request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Handled {
    /// Class C, answered and recorded: `line` is the `[DECISION …]` line for the controller.
    Decided {
        /// The request's id.
        request_id: String,
        /// The recorded decision's id.
        decision_id: String,
        /// `[DECISION <request-id>] <letter> <reason> (<decision-id>)`.
        line: String,
    },
    /// Escalated to the operator, as class O or H; its id is the request's.
    Escalated {
        /// The request's id, which the Decisions page lists.
        request_id: String,
        /// The class it was escalated as.
        class: DecisionClass,
        /// The rule that classified it; `none` when a class-C answer was refused.
        rule: ClassificationRule,
    },
}

/// Handles the request `text` as the module documentation describes, over the store under
/// `state` ([`crate::state::open`]), asking `decider` for a class-C answer.
///
/// # Errors
///
/// The request does not read ([`parse_request`]), a request already has its id, or a command
/// fails: the store does not open or keep a write, or a records line cannot be appended.
pub fn handle_request(state: Option<&Path>, text: &str, decider: &dyn Decider) -> Result<Handled> {
    const COMMAND: &str = "decision handle";
    let request = parse_request(text)?;
    let (class, rule) = classify(text);
    let now = OffsetDateTime::now_utc();
    let at = now.format(&Rfc3339).context("format the time")?;
    super::raise_decision_request(
        state,
        RaiseDecisionRequestArgs {
            request_id: Some(request.request_id.clone()),
            repository: Some(request.repository.clone()),
            question: Some(request.question.clone()),
            options: Some(request.options_text()),
            recommendation: Some(request.recommendation.clone()),
            raised_at: Some(at),
            blocker: None,
            classification_rule: Some(rule_name(rule).to_owned()),
            input_json: None,
        },
    )?;
    if class != DecisionClass::C {
        let reason = format!(
            "the request matches the {} rule (decide::classify), so it is class {} (design § 5)",
            rule_name(rule),
            super::class_name(class)
        );
        return escalate(state, &request, class, rule, reason, now);
    }
    let prompt = build_prompt(&request, &prompt_inputs(&request, now));
    let name = decider.name();
    let answer = match decider.decide(&prompt) {
        Ok(answer) if request.offers(&answer.letter) => answer,
        Ok(answer) => {
            let reason = format!(
                "the {name} decider answered {:?}, which is none of the request's letters ({}); \
                 an unclear answer is the operator's",
                answer.letter,
                request.letters()
            );
            return escalate(state, &request, DecisionClass::O, rule, reason, now);
        }
        Err(failure) => {
            let reason = format!(
                "the {name} decider gave no answer ({}); an unclear answer is the operator's",
                one_line(&format!("{failure:#}"))
            );
            return escalate(state, &request, DecisionClass::O, rule, reason, now);
        }
    };
    let hash = prompt_sha256(&prompt);
    let reason = one_line(answer.reason.trim());
    let fields = Fields {
        decision_id: None,
        decision_class: DecisionClass::C,
        question: request.question.clone(),
        options: request.options_text(),
        choice: answer.letter.clone(),
        reason: reason.clone(),
        evidence: format!("decider={name} prompt={hash}"),
        decided_at: records::now()?,
        repository: Some(RepositoryName(request.repository.clone())),
    };
    let adapter = Adapter {
        name,
        prompt_sha256: PromptHash(hash),
    };
    let decision_id: DecisionId = match conductor_decided(COMMAND, state, &fields, Some(&adapter))?
    {
        Allocated::Recorded(_, decision_id) => decision_id,
        Allocated::Refused(Refusal { error, detail }) => bail!("{COMMAND}: {error}: {detail}"),
        Allocated::Held(decision_id) => {
            bail!("{COMMAND}: decision {:?} already exists", decision_id.0)
        }
    };
    super::answer_request(
        state,
        AnswerRequestArgs {
            request_id: Some(request.request_id.clone()),
            decision_id: Some(decision_id.0.clone()),
            input_json: None,
        },
    )?;
    Ok(Handled::Decided {
        line: format!(
            "[DECISION {}] {} {reason} ({})",
            request.request_id, answer.letter, decision_id.0
        ),
        request_id: request.request_id,
        decision_id: decision_id.0,
    })
}

/// `conductor decision handle --request <file> [--decider claude|fake]`: handles the request the
/// file holds (`-`: standard input) with `decider`, `claude` ([`super::model::ClaudePrint`] on the
/// instance's model) by default or `fake` (it answers the request's recommendation, for tests and
/// dry runs), and prints the `[DECISION …]` line for a decided request or the § 6 line
/// `[ESCALATION <request-id>] <O|H> <question>` for an escalated one.
///
/// # Errors
///
/// The file cannot be read, `decider` names no decider, or [`handle_request`] fails.
pub fn handle(state: Option<&Path>, request: &str, decider: Option<&str>) -> Result<ExitCode> {
    const COMMAND: &str = "decision handle";
    let text = if request == "-" {
        std::io::read_to_string(std::io::stdin())
            .with_context(|| format!("{COMMAND}: read the request from standard input"))?
    } else {
        fs::read_to_string(request)
            .with_context(|| format!("{COMMAND}: --request: read {request:?}"))?
    };
    let decider: Box<dyn Decider> = match decider.unwrap_or("claude") {
        "claude" => Box::new(super::model::ClaudePrint::for_instance(
            &config::active().instance,
        )?),
        "fake" => {
            let parsed = parse_request(&text).map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
            Box::new(super::model::Fake::answering(
                &parsed.recommended,
                "the fake decider answers the request's recommendation",
            ))
        }
        other => bail!("{COMMAND}: --decider {other:?} is neither claude nor fake"),
    };
    let handled = handle_request(state, &text, decider.as_ref())
        .map_err(|error| anyhow!("{COMMAND}: {error:#}"))?;
    let line = match handled {
        Handled::Decided { line, .. } => line,
        Handled::Escalated {
            request_id, class, ..
        } => {
            let question = parse_request(&text)?.question;
            format!(
                "[ESCALATION {request_id}] {} {question}",
                super::class_name(class)
            )
        }
    };
    writeln!(std::io::stdout().lock(), "{line}")
        .with_context(|| format!("{COMMAND}: write to standard output"))?;
    Ok(ExitCode::SUCCESS)
}

/// Escalates `request` as `class` for `reason`, as the module documentation describes.
fn escalate(
    state: Option<&Path>,
    request: &ParsedRequest,
    class: DecisionClass,
    rule: ClassificationRule,
    reason: String,
    now: OffsetDateTime,
) -> Result<Handled> {
    let deadline = (now + Duration::hours(DEADLINE_HOURS))
        .format(&Rfc3339)
        .context("format the deadline")?;
    let options = Value::Array(request.options.iter().map(option_value).collect()).to_string();
    super::escalate_request(
        state,
        EscalateRequestArgs {
            request_id: Some(request.request_id.clone()),
            escalation_class: Some(super::class_name(class).to_owned()),
            reason: Some(reason),
            dry_run: None,
            no_dry_run: (class == DecisionClass::H).then(|| {
                "a controller's decision request: conductor ran no step of it, so it measured no \
                 dry run"
                    .to_owned()
            }),
            kind: Some("one-of".to_owned()),
            lettered_options: Some(options),
            recommended_letters: Some(request.recommended.clone()),
            default_letters: Some(request.recommended.clone()),
            deadline: Some(deadline),
            input_json: None,
        },
    )?;
    Ok(Handled::Escalated {
        request_id: request.request_id.clone(),
        class,
        rule,
    })
}
