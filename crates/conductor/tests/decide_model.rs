//! `story:decide-model-adapter`: a `[DECISION-REQUEST]` handled in code
//! (`docs/design/conductor-run.md` § 3). `decide::handle_request` classifies the request, raises
//! it with its rule, answers class C through the `Decider` adapter with one bounded prompt and
//! records the answer with the adapter's name and the prompt's SHA-256, and escalates class O and
//! H with the request's lettered options, its recommendation as the default and a deadline 48 h
//! out. An answer whose letter is none of the request's options is not recorded: the request is
//! escalated as class O instead.
//!
//! Every case runs the library with the `Fake` decider; nothing here runs `claude`. The instance
//! is the scratch one of `active::isolate`, so the records lines land in its records directory;
//! each case keeps its store in a state directory of its own under this target's temporary
//! directory, and reads it back through the built binary.

mod active;
mod common;

use std::fs;
use std::path::PathBuf;

use conductor_cli::config;
use conductor_cli::decide::model::{
    Answer, ClaudePrint, Decider, Fake, parse_answer, parse_output,
};
use conductor_cli::decide::{
    self, Handled, PROMPT_CAP, PromptInputs, build_prompt, parse_request, prompt_inputs,
    prompt_sha256,
};
use conductor_model::decision::{ClassificationRule, DecisionClass};
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::{Duration, OffsetDateTime};

/// A class-C request: no rule reads its question or options.
const ROUTINE: &str = "[DECISION-REQUEST example-repo req-c1] Run the flaky integration suite in this wave or the next?
A: this wave (cost: one more build slot today)
B: the next wave (cost: a day)
Recommendation: A, the slot is free";

/// A class-O request: a history rewrite, with three lettered options.
const REWRITE: &str = "[DECISION-REQUEST example-repo req-o1] A commit with a 300 MB video landed on main. How is it removed?
A: force-push main without the commit (cost: every clone rebases)
B: revert it in a new commit (cost: the video stays in the history)
C: leave it (cost: clones stay slow)
Recommendation: B, it can be undone";

/// A class-H request: a credential.
const TOKEN: &str = "[DECISION-REQUEST example-repo req-h1] The release job needs a deploy token. Set it as a CI secret?
A: set the secret now (cost: five minutes of the operator)
B: release by hand this time (cost: an hour)
Recommendation: A";

/// One case's directory: its own state directory, and a home for the binary that reads it.
struct Case {
    root: PathBuf,
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let root = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("decide_model")
            .join(name);
        if root.exists() {
            fs::remove_dir_all(&root).expect("clear the case's directory");
        }
        fs::create_dir_all(root.join("home")).expect("create the case's home");
        Self { root }
    }

    fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    /// The rows of the `decision <view>` view of the case's store, read through the binary.
    fn rows(&self, view: &str) -> Vec<Value> {
        let output = common::conductor(self.root.join("home"))
            .arg("--state-dir")
            .arg(self.state())
            .args(["decision", view, "--format", "jsonl"])
            .output()
            .expect("the conductor binary runs");
        assert!(
            output.status.success(),
            "decision {view}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout)
            .expect("UTF-8")
            .lines()
            .map(|line| serde_json::from_str(line).expect("a JSON row"))
            .collect()
    }

    fn request(&self, id: &str) -> Value {
        self.rows("requests")
            .into_iter()
            .find(|row| row["request_id"] == id)
            .unwrap_or_else(|| panic!("the store holds request {id}"))
    }
}

/// The line of the records' decision log whose `id` is `id`, and whose `event` is `event`.
fn logged(id: &str, event: &str) -> Value {
    let dir = PathBuf::from(&config::active().instance.records).join("decisions");
    let mut found = None;
    for entry in fs::read_dir(&dir).expect("the decision log's directory") {
        let text = fs::read_to_string(entry.expect("an entry").path()).expect("a log file");
        for line in text.lines() {
            let line: Value = serde_json::from_str(line).expect("a JSON line");
            if line["id"] == id && line["event"] == event {
                found = Some(line);
            }
        }
    }
    found.unwrap_or_else(|| panic!("no {event} line for {id} in {}", dir.display()))
}

fn at(text: &str) -> OffsetDateTime {
    OffsetDateTime::parse(text, &Rfc3339).expect("an RFC 3339 instant")
}

#[test]
fn a_class_c_request_is_answered_by_the_decider_and_recorded_with_its_name_and_prompt_hash() {
    let case = Case::new("class-c");
    let fake = Fake::answering("A", "the slot is free and the suite is short");
    let handled = decide::handle_request(Some(&case.state()), ROUTINE, &fake)
        .expect("a class-C request is handled");
    let Handled::Decided {
        request_id,
        decision_id,
        line,
    } = handled
    else {
        panic!("a class-C request is decided, not {handled:?}");
    };
    assert_eq!(request_id, "req-c1");
    assert_eq!(
        line,
        format!("[DECISION req-c1] A the slot is free and the suite is short ({decision_id})")
    );
    assert!(
        conductor_cli::message::read(&line).parsed,
        "the line for delivery has the § 6 DECISION form: {line}"
    );

    let prompts = fake.prompts();
    assert_eq!(prompts.len(), 1, "one call to the decider");
    let hash = prompt_sha256(&prompts[0]);
    assert!(prompts[0].contains("Run the flaky integration suite"));
    assert!(prompts[0].chars().count() <= PROMPT_CAP);

    let decision = case
        .rows("decisions")
        .into_iter()
        .find(|row| row["decision_id"] == decision_id.as_str())
        .expect("the store holds the decision");
    assert_eq!(decision["decision_class"], "C");
    assert_eq!(decision["decided_by"], "Conductor");
    assert_eq!(decision["choice"], "A");
    assert_eq!(
        decision["reason"],
        "the slot is free and the suite is short"
    );
    assert_eq!(decision["decider_name"], "fake");
    assert_eq!(decision["prompt_sha256"], hash.as_str());
    assert_eq!(decision["evidence"], format!("decider=fake prompt={hash}"));
    assert_eq!(decision["repository"], "example-repo");

    let request = case.request("req-c1");
    assert_eq!(request["classification_rule"], "none");
    assert_eq!(request["state"], "Answered");
    assert_eq!(request["decision_id"], decision_id.as_str());

    let decided = logged(&decision_id, "decided");
    assert_eq!(decided["decider_name"], "fake");
    assert_eq!(decided["prompt_sha256"], hash.as_str());
    assert_eq!(logged("req-c1", "requested")["classification_rule"], "none");
}

#[test]
fn a_class_o_request_is_escalated_with_its_parsed_options_default_and_deadline() {
    let case = Case::new("class-o");
    let fake = Fake::answering("B", "never asked");
    let before = OffsetDateTime::now_utc();
    let handled = decide::handle_request(Some(&case.state()), REWRITE, &fake)
        .expect("a class-O request is handled");
    let after = OffsetDateTime::now_utc();
    assert_eq!(
        handled,
        Handled::Escalated {
            request_id: "req-o1".to_owned(),
            class: DecisionClass::O,
            rule: ClassificationRule::HistoryRewrite,
        }
    );
    assert!(fake.prompts().is_empty(), "class O asks no model");

    let request = case.request("req-o1");
    assert_eq!(request["state"], "Escalated");
    assert_eq!(request["escalation_class"], "O");
    assert_eq!(request["classification_rule"], "history-rewrite");
    assert_eq!(request["kind"], "one-of");
    let options: Vec<(String, String, String)> = request["lettered_options"]
        .as_array()
        .expect("lettered options")
        .iter()
        .map(|option| {
            (
                option["letter"].as_str().unwrap_or_default().to_owned(),
                option["label"].as_str().unwrap_or_default().to_owned(),
                option["cost"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    assert_eq!(
        options,
        [
            (
                "A".to_owned(),
                "force-push main without the commit".to_owned(),
                "every clone rebases".to_owned()
            ),
            (
                "B".to_owned(),
                "revert it in a new commit".to_owned(),
                "the video stays in the history".to_owned()
            ),
            (
                "C".to_owned(),
                "leave it".to_owned(),
                "clones stay slow".to_owned()
            ),
        ]
    );
    assert_eq!(request["recommended_letters"], "B");
    assert_eq!(request["default_letters"], "B");
    let deadline = at(request["deadline"].as_str().expect("a deadline"));
    assert!(
        deadline >= before + Duration::hours(48) - Duration::seconds(1)
            && deadline <= after + Duration::hours(48) + Duration::seconds(1),
        "the deadline is 48 h after the request was handled: {deadline}"
    );
    assert!(
        case.rows("decisions").is_empty(),
        "nothing is decided for an escalated request"
    );
    assert_eq!(
        logged("req-o1", "escalated")["escalation_class"],
        "O",
        "the escalation is logged"
    );
}

#[test]
fn a_class_h_request_is_escalated_as_h() {
    let case = Case::new("class-h");
    let fake = Fake::answering("A", "never asked");
    let handled = decide::handle_request(Some(&case.state()), TOKEN, &fake)
        .expect("a class-H request is handled");
    assert_eq!(
        handled,
        Handled::Escalated {
            request_id: "req-h1".to_owned(),
            class: DecisionClass::H,
            rule: ClassificationRule::Credential,
        }
    );
    let request = case.request("req-h1");
    assert_eq!(request["state"], "Escalated");
    assert_eq!(request["escalation_class"], "H");
    assert_eq!(request["classification_rule"], "credential");
    assert!(fake.prompts().is_empty(), "class H asks no model");
}

#[test]
fn an_answer_naming_no_option_is_refused_and_the_request_escalated_as_o() {
    let case = Case::new("refused");
    let fake = Fake::answering("Z", "a letter the request does not offer");
    let handled = decide::handle_request(Some(&case.state()), ROUTINE, &fake)
        .expect("a refused answer is handled");
    assert_eq!(
        handled,
        Handled::Escalated {
            request_id: "req-c1".to_owned(),
            class: DecisionClass::O,
            rule: ClassificationRule::None,
        }
    );
    assert_eq!(fake.prompts().len(), 1, "the decider was asked once");
    assert!(
        case.rows("decisions").is_empty(),
        "the refused answer is not recorded"
    );
    let request = case.request("req-c1");
    assert_eq!(request["state"], "Escalated");
    assert_eq!(request["escalation_class"], "O");
    assert_eq!(request["default_letters"], "A");
    let reason = request["escalation_reason"].as_str().unwrap_or_default();
    assert!(
        reason.contains("\"Z\""),
        "the reason names the refused letter: {reason}"
    );
}

/// The flags the specification added: `raise-decision-request --classification-rule` and
/// `record-conductor-decision --decider-name --prompt-sha256`, both or neither, the hash 64
/// lower-case hexadecimal characters.
#[test]
fn the_rule_and_the_deciders_evidence_are_flags_of_the_two_commands() {
    let case = Case::new("flags");
    let run = |args: &[&str]| {
        common::conductor(case.root.join("home"))
            .current_dir(&case.root)
            .arg("--state-dir")
            .arg(case.state())
            .args(args)
            .output()
            .expect("the conductor binary runs")
    };
    let raised = run(&[
        "decision",
        "raise-decision-request",
        "--request-id",
        "req-f1",
        "--repository",
        "example-repo",
        "--question",
        "which?",
        "--options",
        "A: one | B: two",
        "--recommendation",
        "A",
        "--raised-at",
        "2026-10-07T10:00:00Z",
        "--classification-rule",
        "none",
    ]);
    assert!(raised.status.success(), "{raised:?}");
    assert_eq!(case.request("req-f1")["classification_rule"], "none");

    let hash = prompt_sha256("a prompt");
    let record = |extra: &[&str]| {
        let mut args = vec![
            "decision",
            "record-conductor-decision",
            "--decision-class",
            "C",
            "--question",
            "which?",
            "--options",
            "A: one | B: two",
            "--choice",
            "A",
            "--reason",
            "it can be undone",
            "--evidence",
            "a test",
            "--decided-at",
            "2026-10-07T10:00:00Z",
        ];
        args.extend_from_slice(extra);
        run(&args)
    };
    let recorded = record(&["--decider-name", "fake", "--prompt-sha256", &hash]);
    assert!(recorded.status.success(), "{recorded:?}");
    let id = String::from_utf8(recorded.stdout).expect("UTF-8");
    let row = case
        .rows("decisions")
        .into_iter()
        .find(|row| row["decision_id"] == id.trim())
        .expect("the decision is stored");
    assert_eq!(row["decider_name"], "fake");
    assert_eq!(row["prompt_sha256"], hash.as_str());

    for (extra, named) in [
        (vec!["--decider-name", "fake"], "--prompt-sha256"),
        (vec!["--prompt-sha256", hash.as_str()], "--decider-name"),
        (
            vec!["--decider-name", "fake", "--prompt-sha256", "ABC"],
            "PromptHash",
        ),
    ] {
        let refused = record(&extra);
        let stderr = String::from_utf8_lossy(&refused.stderr);
        assert!(
            refused.status.code() == Some(1) && stderr.contains(named),
            "{extra:?} is refused naming {named}: {stderr}"
        );
    }
    assert_eq!(case.rows("decisions").len(), 1, "a refusal records nothing");
}

#[test]
fn claude_print_runs_print_mode_with_no_tools_and_no_setting_sources() {
    active::isolate();
    let claude = ClaudePrint::new("some-model");
    let arguments = claude.arguments();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    for pair in [["--model", "some-model"], ["--output-format", "json"]] {
        assert!(
            words.windows(2).any(|window| window == pair),
            "{pair:?} in {words:?}"
        );
    }
    for flag in ["-p", "--tools=", "--setting-sources="] {
        assert!(words.contains(&flag), "{flag} in {words:?}");
    }
    assert_eq!(claude.name(), "claude-print/some-model");
}

#[test]
fn claude_print_takes_the_conductor_roles_model_else_the_controllers() {
    active::isolate();
    let mut instance = config::active().instance.clone();
    for role in &mut instance.roles {
        role.model.0 = format!("model-of-{}", role.role);
    }
    let conductor = instance
        .roles
        .iter()
        .find(|role| config::is_role(&role.role, conductor_model::config::RoleName::Director))
        .expect("the scratch instance has a conductor role")
        .model
        .0
        .clone();
    assert_eq!(
        ClaudePrint::for_instance(&instance)
            .expect("a model")
            .model(),
        conductor
    );
    instance
        .roles
        .retain(|role| !config::is_role(&role.role, conductor_model::config::RoleName::Director));
    let controller = instance
        .roles
        .iter()
        .find(|role| config::is_role(&role.role, conductor_model::config::RoleName::Lead))
        .expect("the scratch instance has a controller role")
        .model
        .0
        .clone();
    assert_eq!(
        ClaudePrint::for_instance(&instance)
            .expect("a model")
            .model(),
        controller
    );
    instance.roles.clear();
    assert!(ClaudePrint::for_instance(&instance).is_err());
}

#[test]
fn the_answer_is_one_letter_and_a_reason_and_anything_else_is_refused() {
    active::isolate();
    assert_eq!(
        parse_answer("B: the file store is enough\n").expect("an answer"),
        Answer {
            letter: "B".to_owned(),
            reason: "the file store is enough".to_owned()
        }
    );
    assert_eq!(
        parse_answer("**A**: undo is one command")
            .expect("an answer")
            .letter,
        "A"
    );
    for text in [
        "",
        "B",
        "B:",
        "the file store: B",
        "BB: two letters",
        "b: lower case",
    ] {
        assert!(parse_answer(text).is_err(), "{text:?} is refused");
    }
    let output =
        r#"{"type":"result","subtype":"success","is_error":false,"result":"A: it can be undone"}"#;
    assert_eq!(parse_output(output).expect("an answer").letter, "A");
    let failed = r#"{"type":"result","subtype":"error_max_turns","is_error":true,"result":""}"#;
    assert!(parse_output(failed).is_err());
    assert!(parse_output("not json").is_err());
}

#[test]
fn an_option_line_gives_its_label_and_cost_whatever_its_letters() {
    active::isolate();
    let request = parse_request(
        "[DECISION-REQUEST example-repo req-u1] Which mirror?\n\
         - A: İİİ mirror (Cost: none)\n\
         B) the default mirror; cost: slower\n\
         C: no cost named\n\
         Recommended: C",
    )
    .expect("the request parses");
    let options: Vec<(&str, &str, &str)> = request
        .options
        .iter()
        .map(|option| {
            (
                option.letter.as_str(),
                option.label.as_str(),
                option.cost.as_str(),
            )
        })
        .collect();
    assert_eq!(
        options,
        [
            ("A", "İİİ mirror", "none"),
            ("B", "the default mirror", "slower"),
            ("C", "no cost named", "not stated in the request"),
        ]
    );
    assert_eq!(request.recommended, "C");
    for broken in [
        "[DECISION-REQUEST example-repo req-u2] No options?\nRecommendation: A",
        "[DECISION-REQUEST example-repo req-u3] No recommendation?\nA: one\nB: two",
        "[DECISION-REQUEST example-repo req-u4] Twice?\nA: one\nA: two\nRecommendation: A",
        "[DECISION example-repo req-u5] Not a request\nA: one\nRecommendation: A",
    ] {
        assert!(parse_request(broken).is_err(), "{broken:?} is refused");
    }
}

#[test]
fn the_prompt_hash_is_the_sha256_of_the_prompt() {
    assert_eq!(
        prompt_sha256("abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

#[test]
fn the_prompt_keeps_its_sections_in_order_and_under_the_cap_dropping_the_oldest_decisions() {
    active::isolate();
    let request = parse_request(ROUTINE).expect("the request parses");
    let decisions: Vec<String> = (0..200)
        .map(|n| format!("decision number {n:03}: {}", "x".repeat(200)))
        .collect();
    let inputs = PromptInputs {
        adr_titles: vec!["0001 events kept per stream".to_owned()],
        rules: "1. Work flowing beats a safety cap.\n".to_owned(),
        decisions,
    };
    let prompt = build_prompt(&request, &inputs);
    assert!(
        prompt.chars().count() <= PROMPT_CAP,
        "{} characters",
        prompt.chars().count()
    );
    assert!(
        prompt.contains("decision number 199"),
        "the newest decision stays"
    );
    assert!(
        !prompt.contains("decision number 000"),
        "the oldest is dropped"
    );
    let order: Vec<usize> = [
        "Run the flaky integration suite",
        "A: this wave",
        "0001 events kept per stream",
        "Work flowing beats a safety cap",
        "decision number 199",
    ]
    .iter()
    .map(|needle| {
        prompt
            .find(needle)
            .unwrap_or_else(|| panic!("{needle:?} in the prompt"))
    })
    .collect();
    assert!(
        order.windows(2).all(|pair| pair[0] < pair[1]),
        "request, options, ADR titles, rules, decisions in that order: {order:?}"
    );
    let last = prompt.lines().last().unwrap_or_default();
    assert!(
        last.contains("A, B"),
        "the prompt ends asking for one of the letters: {last:?}"
    );

    // Rules over the cap, ending in blank lines: the rules are cut to fit, and nothing else is.
    let long = PromptInputs {
        adr_titles: vec!["0002 a title that stays".to_owned()],
        rules: format!("{}\n\n\n", "r".repeat(3 * PROMPT_CAP)),
        decisions: Vec::new(),
    };
    let prompt = build_prompt(&request, &long);
    assert!(prompt.chars().count() <= PROMPT_CAP);
    assert!(
        prompt.contains("B: the next wave"),
        "the request is never cut"
    );
    assert!(
        prompt.contains("0002 a title that stays"),
        "cutting the rules is enough"
    );
    assert!(prompt.lines().last().unwrap_or_default().contains("A, B"));
}

#[test]
fn the_prompt_reads_the_rules_and_the_repositorys_decisions_of_the_last_seven_days() {
    active::isolate();
    let records = PathBuf::from(&config::active().instance.records);
    fs::create_dir_all(records.join("decisions")).expect("the decision log's directory");
    fs::write(
        records.join("rules.md"),
        "# Rules\n\n## Conductor\n\n### How the operator decides (extracted from his answers)\n\n\
         10. **Undo beats asking.** Take the option that can be undone.\n\n\
         ### Cleanup\n\n11. Remove merged trees.\n",
    )
    .expect("write rules.md");
    let now = OffsetDateTime::now_utc();
    let day = |days: i64| {
        (now - Duration::days(days))
            .format(&Rfc3339)
            .expect("an instant")
    };
    // A repository of this run alone: the scratch records outlive a run.
    let repo = format!("prompt-repo-{}", now.unix_timestamp_nanos());
    let line = |at: String, repo: &str, id: &str, question: &str| {
        serde_json::json!({
            "at": at, "event": "decided", "repo": repo, "id": id, "class": "C",
            "decided_by": "Conductor", "question": question, "options": "A | B",
            "choice": "A", "reason": "it can be undone", "evidence": "a test line",
        })
        .to_string()
    };
    let lines = [
        line(
            day(2),
            &repo,
            "DEC-20000101-01",
            "a recent question of this repository",
        ),
        line(
            day(9),
            &repo,
            "DEC-20000101-02",
            "an old question of this repository",
        ),
        line(
            day(1),
            "other-repo",
            "DEC-20000101-03",
            "a question of another repository",
        ),
    ];
    let month = format!("{:04}-{:02}.jsonl", now.year(), u8::from(now.month()));
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(records.join("decisions").join(month))
        .expect("open this month's decision log");
    std::io::Write::write_all(&mut log, format!("{}\n", lines.join("\n")).as_bytes())
        .expect("append to the decision log");
    let request = parse_request(&format!(
        "[DECISION-REQUEST {repo} req-p1] Which story goes first?\nA: parser\nB: renderer\nRecommendation: A"
    ))
    .expect("the request parses");

    let inputs = prompt_inputs(&request, now);
    assert!(
        inputs.adr_titles.is_empty(),
        "no checkout, no store, no titles"
    );
    assert!(
        inputs.rules.contains("Undo beats asking"),
        "{}",
        inputs.rules
    );
    assert!(
        !inputs.rules.contains("Remove merged trees"),
        "{}",
        inputs.rules
    );
    assert_eq!(inputs.decisions.len(), 1, "{:?}", inputs.decisions);
    assert!(inputs.decisions[0].contains("a recent question of this repository"));

    fs::create_dir_all(records.join("rules-v2")).expect("rules-v2");
    fs::write(
        records.join("rules-v2").join("all.md"),
        "1. The switched rules.\n",
    )
    .expect("write rules-v2/all.md");
    let inputs = prompt_inputs(&request, now);
    fs::remove_dir_all(records.join("rules-v2")).expect("remove rules-v2");
    assert!(
        inputs.rules.contains("The switched rules"),
        "{}",
        inputs.rules
    );
}
