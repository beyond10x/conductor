//! Adversarial cases for story:instance-rules-text: an instance's `standing_rules`, the frame a
//! session's prompt file holds them in, and the file an instance pod's runtime reads them from.

mod active;
mod scratch;
mod standing_rules;

use std::path::Path;

use conductor_cli::agent::claude_code;
use conductor_cli::config;
use conductor_cli::config::pod;
use scratch::Scratch;
use standing_rules::{HEADING, RULES};

/// A config file whose daemon runs its instances as pods, with the one instance `alpha` and the
/// line `extra` after its keys.
fn pods_file(dir: &Path, extra: &str) -> String {
    format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 store: {store:?}\n\
         \x20 secrets: {{source: files, dir: {secrets:?}}}\n\
         \x20 kubernetes:\n\
         \x20   namespace: conductor-instances\n\
         \x20   image: registry.example/conductor:1\n\
         \x20   owner: daemon-a\n\
         \x20   api_port: 7310\n\
         \x20   volume: {{size: 20G, storage_class: fast-local}}\n\
         \x20   env: []\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{{github: example-org}}]\n\
         \x20   model_token: secret:alpha-model\n\
         {extra}",
        store = format!("sqlite:{}", dir.join("shared.sqlite").display()),
        secrets = dir.join("secrets").display().to_string(),
    )
}

/// The member line of the standing rules `text`, written as JSON, which YAML reads.
fn rules_line(text: &str) -> String {
    format!(
        "\x20   standing_rules: {}\n",
        serde_json::to_string(text).expect("JSON")
    )
}

/// The fenced blocks of `text` as CommonMark reads them: for each line, whether it is inside a
/// fenced code block (its opening and closing lines count as inside).
fn inside_fences(text: &str) -> Vec<(String, bool)> {
    let mut open: Option<(char, usize)> = None;
    let mut out = Vec::new();
    for line in text.split_inclusive('\n') {
        let bare = line.trim_end_matches(['\n', '\r']);
        let indent = bare.len() - bare.trim_start_matches(' ').len();
        let rest = bare.trim_start_matches(' ');
        match open {
            None => {
                let mark = rest.chars().next().filter(|c| *c == '`' || *c == '~');
                if indent <= 3
                    && let Some(mark) = mark
                {
                    let run = rest.chars().take_while(|c| *c == mark).count();
                    if run >= 3 && !(mark == '`' && rest[run..].contains('`')) {
                        open = Some((mark, run));
                        out.push((bare.to_owned(), true));
                        continue;
                    }
                }
                out.push((bare.to_owned(), false));
            }
            Some((mark, length)) => {
                let closes = indent <= 3
                    && !rest.trim_end().is_empty()
                    && rest.trim_end().chars().all(|c| c == mark)
                    && rest.trim_end().len() >= length;
                out.push((bare.to_owned(), true));
                if closes {
                    open = None;
                }
            }
        }
    }
    out
}

/// The text of the standing rules travels into an instance pod in its Secret's rendered file,
/// which the runtime reads them from at each session's start: what a session there reads is the
/// text the mapping holds, byte for byte, for any text the mapping takes (at most 16384 bytes, no
/// NUL). Line breaks other than LF, white space at the ends of lines and of the text, and
/// characters YAML gives a meaning are each text the mapping takes. A text that is only white
/// space gives no rule and reads as absent, in the daemon's file and in the pod's.
#[test]
fn every_text_the_mapping_takes_reaches_the_pods_file_byte_for_byte() {
    active::isolate();
    let scratch = Scratch::new("adv-rules/round-trip");
    let home = scratch.path().join("home");
    std::fs::create_dir_all(&home).expect("create the home");
    let texts = [
        "Rule one.\r\nRule two.\r\n",
        "Rule one.\rRule two.\r",
        "Rule one.\u{85}Rule two.\n",
        "Rule one.\u{2028}Rule two.\u{2029}Rule three.\n",
        "   An indented first line.\nRule two.\n",
        "Rule one.   \nRule two.\t\n",
        "Rule one.\n\n\n",
        "Rule one, and no line break at the end",
        "\n\nRule after two blank lines.\n",
        "---\nRule after a document marker.\n...\n",
        "# Not a comment\n- not a list: [x]\n",
        "\u{feff}Rule after a byte order mark.\n",
        "Rule with an escape \u{1b}[31m inside.\n",
        " ",
        "\t",
    ];
    let mut lost = Vec::new();
    for text in texts {
        let expected = (!text.trim().is_empty()).then_some(text);
        // Written as the daemon writes a mapping a POST or PUT carries: by serde_yaml.
        let mut document: serde_yaml::Value =
            serde_yaml::from_str(&pods_file(scratch.path(), "")).expect("the file");
        document["instances"][0]["standing_rules"] = serde_yaml::Value::from(text);
        let file = serde_yaml::to_string(&document).expect("YAML");
        let config = match config::parse(&file, &home) {
            Ok(config) => config,
            Err(problems) => {
                lost.push(format!("{text:?}: the mapping is refused: {problems:?}"));
                continue;
            }
        };
        if config.instances[0].standing_rules.as_deref() != expected {
            lost.push(format!(
                "{text:?} reads from the daemon's file as {:?}",
                config.instances[0].standing_rules
            ));
            continue;
        }
        let rendered = pod::render(&config, &config.instances[0]);
        let read = match config::parse(&rendered, &home) {
            Ok(read) => read.instances[0].standing_rules.clone(),
            Err(problems) => {
                lost.push(format!(
                    "{text:?}: the pod's file does not read: {problems:?}"
                ));
                continue;
            }
        };
        if read.as_deref() != expected {
            lost.push(format!("{text:?} reads back as {read:?}"));
        }
    }
    assert!(
        lost.is_empty(),
        "texts a session in the pod reads otherwise than the mapping holds them:\n{}",
        lost.join("\n")
    );
}

/// The frame holds the text "in a form the text cannot break out of" whatever the profile: the
/// rules come first, so a role's own `profile` file whose last fenced block is still open where
/// the file ends (a fence opened and never closed) reaches neither the heading line nor the rules.
/// The prompt starts with the heading line outside every fence and the rules' block, the block is
/// closed before the profile's first byte, and the profile follows it unchanged.
#[test]
fn the_rules_stay_inside_their_fence_ahead_of_a_profile_that_ends_inside_an_open_fence() {
    active::isolate();
    let base = "# Repository controller\n\nAn example:\n\n```\nconductor spawn\n";
    let prompt = claude_code::prompt_text(Some(base), Some(RULES)).expect("a prompt");
    let block = prompt
        .strip_suffix(&format!("\n{base}"))
        .unwrap_or_else(|| panic!("the profile ends the prompt unchanged:\n{prompt}"));
    let lines = inside_fences(block);
    assert!(
        lines[0].0.starts_with(HEADING) && !lines[0].1,
        "the prompt starts with the heading line, outside every fence:\n{prompt}"
    );
    let outside: Vec<&str> = lines[2..]
        .iter()
        .filter(|(_, inside)| !inside)
        .map(|(line, _)| line.as_str())
        .collect();
    assert!(
        outside.is_empty(),
        "lines of the rules' block outside its fence: {outside:?}\n{prompt}"
    );
    assert_eq!(
        inside_fences(&format!("{block}after\n")).last(),
        Some(&("after".to_owned(), false)),
        "the rules' block is closed before the profile's first byte:\n{prompt}"
    );
}

/// The specification declares the refusals of a mapping's `standing_rules`
/// (`spec/domains/config.yaml`): its size, a NUL, and the config file's secret scan, which holds
/// for this member as for every value of the file. A rule that names a word that looks like a
/// token, or that writes a URL with a password in it, is refused at the member's path, and no
/// problem quotes the text.
#[test]
fn a_rule_with_a_word_that_looks_like_a_token_or_a_url_with_a_password_is_refused_unquoted() {
    active::isolate();
    let scratch = Scratch::new("adv-rules/token-words");
    let home = scratch.path().join("home");
    std::fs::create_dir_all(&home).expect("create the home");
    for (text, quoted) in [
        (
            "Never commit a token: nothing that starts with ghp_ or glpat- goes into a file.\n",
            ["Never commit", "ghp_"],
        ),
        (
            "Never push to https://user:password@forge.example/; use the credential helper.\n",
            ["Never push", "user:password"],
        ),
    ] {
        let file = pods_file(scratch.path(), &rules_line(text));
        let problems = config::parse(&file, &home)
            .err()
            .unwrap_or_else(|| panic!("{text:?} is refused"));
        assert!(
            problems
                .iter()
                .any(|problem| problem.path == "instances[0].standing_rules"),
            "{text:?}: {problems:?}"
        );
        for problem in &problems {
            let shown = format!("{} {}", problem.path, problem.message);
            for part in quoted {
                assert!(!shown.contains(part), "a problem quotes the text: {shown}");
            }
        }
    }
}

/// "Absent, every session starts as before." A control plane that writes the member with no text
/// (the zero value of a string field, `""`) has given no rule: the prompt a session starts with
/// is the one it had, with no heading line and no empty frame, and the director gets no prompt
/// file beside its agent.
#[test]
fn an_empty_rules_text_adds_no_heading_and_no_empty_frame() {
    active::isolate();
    let base = "# Repository controller\n";
    assert_eq!(
        claude_code::prompt_text(Some(base), Some("")).as_deref(),
        Some(base),
        "a profile followed by an empty text"
    );
    assert_eq!(
        claude_code::prompt_text(None, Some("")),
        None,
        "the director's prompt with an empty text"
    );
}
