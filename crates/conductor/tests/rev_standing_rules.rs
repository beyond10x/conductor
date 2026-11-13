//! Review cases for story:instance-rules-text: conformance checks of the frame a session's prompt
//! file holds an instance's standing rules in, of the one writer of the text, and of the doctor's
//! verdict on the right the daemon's write of the text into an instance's Secret takes.
//!
//! The frame is read here as CommonMark reads it ([`blocks`]): a line ending is LF, CR or CRLF,
//! and a closing fence is followed only by spaces or tabs; an HTML block of the comment kind or of
//! the `<pre>` kind runs to the line that ends it. The cases that must hold under any reading
//! (an open backtick or tilde fence, CRLF, a long fence) stand beside the others as controls.

mod active;
mod fake_kube;
mod scratch;
mod served;
mod standing_rules;

use std::ffi::OsString;
use std::fs;
use std::io::{Read as _, Write as _};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use conductor_cli::agent::claude_code;
use conductor_cli::config;
use conductor_cli::daemon::pods::Pods;
use conductor_cli::daemon::reconcile::{Binary, Daemon};
use conductor_cli::daemon::units::Manager;
use conductor_cli::dashboard::Sources;
use conductor_model::instance::ReconcileReason;
use fake_kube::FakeKube;
use scratch::Scratch;
use serde_json::{Value, json};
use standing_rules::{HEADING, RULES};

/// Where a line of a prompt stands, as CommonMark reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    Outside,
    Fence(usize),
    Html(usize),
}

/// The lines of `text`, split at LF, CR and CRLF, each with the block it stands in. Fences are
/// backtick or tilde runs of three or more, indented at most three spaces, a backtick fence's info
/// string holding no backtick; a fence closes on a run of its mark at least as long, followed only
/// by spaces or tabs. An HTML block opens on `<!--` (to the line holding `-->`) or on `<pre`,
/// `<script`, `<style` or `<textarea` (to the line holding its closing tag).
fn blocks(text: &str) -> Vec<(String, Block)> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\n' => lines.push(std::mem::take(&mut line)),
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                lines.push(std::mem::take(&mut line));
            }
            other => line.push(other),
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    let mut out = Vec::new();
    let mut open: Option<(Block, char, usize, &'static str)> = None;
    let mut serial = 0;
    for line in lines {
        let rest = line.trim_start_matches(' ');
        let indent = line.len() - rest.len();
        match open {
            None => {
                let mark = rest.chars().next().filter(|c| *c == '`' || *c == '~');
                if indent <= 3
                    && let Some(mark) = mark
                {
                    let run = rest.chars().take_while(|c| *c == mark).count();
                    if run >= 3 && !(mark == '`' && rest[run..].contains('`')) {
                        serial += 1;
                        open = Some((Block::Fence(serial), mark, run, ""));
                        out.push((line, Block::Fence(serial)));
                        continue;
                    }
                }
                let lower = rest.to_ascii_lowercase();
                let html = if indent > 3 {
                    None
                } else if lower.starts_with("<!--") {
                    Some("-->")
                } else {
                    [
                        ("<pre", "</pre>"),
                        ("<script", "</script>"),
                        ("<style", "</style>"),
                        ("<textarea", "</textarea>"),
                    ]
                    .into_iter()
                    .find(|(start, _)| {
                        lower.starts_with(start)
                            && matches!(lower[start.len()..].chars().next(), None | Some(' ' | '>'))
                    })
                    .map(|(_, end)| end)
                };
                if let Some(end) = html {
                    serial += 1;
                    let ends_here = lower[4..].contains(end);
                    out.push((line, Block::Html(serial)));
                    if !ends_here {
                        open = Some((Block::Html(serial), ' ', 0, end));
                    }
                    continue;
                }
                out.push((line, Block::Outside));
            }
            Some((block @ Block::Fence(_), mark, length, _)) => {
                let run = rest.chars().take_while(|c| *c == mark).count();
                let closes = indent <= 3
                    && run >= length
                    && rest[run..].chars().all(|c| c == ' ' || c == '\t');
                out.push((line, block));
                if closes {
                    open = None;
                }
            }
            Some((block, _, _, end)) => {
                let closes = line.to_ascii_lowercase().contains(end);
                out.push((line, block));
                if closes {
                    open = None;
                }
            }
        }
    }
    out
}

/// Whether the prompt `prompt` holds the standing rules as data in their own block ahead of the
/// profile `profile`: the prompt starts with the heading line, outside every block; every line
/// from two lines after it to the end of the rules is one fenced block; that block is closed
/// before the profile's first byte; and an empty line and the profile, unchanged, end the prompt.
fn frame_holds(prompt: &str, profile: &str) -> Result<(), String> {
    let rules_part = prompt
        .strip_suffix(profile)
        .and_then(|head| head.strip_suffix('\n'))
        .ok_or("the profile does not follow the rules' block unchanged after an empty line")?;
    let lines = blocks(rules_part);
    let Some((first, first_block)) = lines.first() else {
        return Err("an empty prompt".to_owned());
    };
    if !first.starts_with(HEADING) {
        return Err(format!(
            "the prompt does not start with the heading line: {first:?}"
        ));
    }
    if *first_block != Block::Outside {
        return Err(format!("the heading line stands in {first_block:?}"));
    }
    let Some((_, Block::Fence(rules))) = lines.get(2) else {
        return Err(format!(
            "the line two after the heading opens no fence: {:?}",
            lines.get(2)
        ));
    };
    let rules = *rules;
    for (line, block) in &lines[2..] {
        if *block != Block::Fence(rules) {
            return Err(format!(
                "a line after the heading stands in {block:?}, not in the rules' fence: {line:?}"
            ));
        }
    }
    let reopened = blocks(&format!("{rules_part}after\n"));
    if reopened.last().map(|(_, block)| *block) != Some(Block::Outside) {
        return Err("the rules' block is not closed before the profile's first byte".to_owned());
    }
    Ok(())
}

/// Each profile of `profiles` whose prompt with [`RULES`] breaks the frame, with why.
fn broken(profiles: &[(&str, &str)]) -> Vec<String> {
    profiles
        .iter()
        .filter_map(|(what, profile)| {
            let prompt = claude_code::prompt_text(Some(profile), Some(RULES)).expect("a prompt");
            frame_holds(&prompt, profile)
                .err()
                .map(|why| format!("{what}: {why}\n  prompt: {prompt:?}"))
        })
        .collect()
}

/// With a role's own profile, the prompt starts with the heading line and the rules' block, which
/// is closed before the profile's first byte, and the profile follows unchanged, whatever line
/// breaks and white space the profile's fences are written with. A profile can end a line with a
/// lone CR (a line ending to CommonMark), and can follow a closing run of backticks with a
/// character other than a space or a tab (no ending to CommonMark: the block stays open); either
/// stays in the profile, after the rules.
#[test]
fn rev_a_profile_left_open_by_its_line_breaks_or_white_space_keeps_the_rules_out_of_their_frame() {
    active::isolate();
    let controls = [
        ("an open backtick fence", "Intro\n\n```\ncode"),
        ("an open tilde fence", "Intro\n\n~~~~\ncode"),
        ("an open fence in CRLF lines", "Intro\r\n\r\n```\r\ncode"),
        (
            "a fence closed with a tab after it",
            "```\ncode\n```\t\nmore",
        ),
        (
            "an open fence of nine backticks",
            "Intro\n\n`````````\ncode",
        ),
    ];
    assert_eq!(
        broken(&controls),
        Vec::<String>::new(),
        "the controls hold: the reading of this file agrees with the code where both should"
    );
    let profiles = [
        ("a fence opened after a lone CR", "Intro\r```\rcode"),
        (
            "a closing run with a no-break space after it",
            "```\ncode\n```\u{a0}\nmore",
        ),
        (
            "a closing run with U+2028 after it",
            "```\ncode\n```\u{2028}\nmore",
        ),
        (
            "a closing run with a form feed after it",
            "```\ncode\n```\u{c}\nmore",
        ),
    ];
    let broken = broken(&profiles);
    assert!(
        broken.is_empty(),
        "profiles after which the heading or the rules leave their frame:\n{}",
        broken.join("\n")
    );
}

/// A profile's HTML block (a comment, a `<pre>`) that runs to its end, and a fence line inside a
/// closed HTML comment, stay in the profile: the prompt starts with the heading line and the rules'
/// block, closed before the profile's first byte, and the profile follows unchanged, with no line
/// added to it.
#[test]
fn rev_a_profile_with_an_html_block_keeps_the_rules_out_of_their_frame() {
    active::isolate();
    let controls = [("a closed HTML comment", "Intro\n\n<!-- a note -->\n\nOutro")];
    assert_eq!(broken(&controls), Vec::<String>::new(), "the control holds");
    let profiles = [
        (
            "a fence line inside a closed HTML comment",
            "Intro\n\n<!--\n```\n-->\n\nOutro",
        ),
        ("an HTML comment left open", "Intro\n\n<!--\na note"),
        ("a <pre> block left open", "Intro\n\n<pre>\ncode"),
    ];
    let broken = broken(&profiles);
    assert!(
        broken.is_empty(),
        "profiles after which the heading or the rules leave their frame:\n{}",
        broken.join("\n")
    );
}

/// A profile that leaves a block open is written into the session's copy only: the file the role
/// names is the same bytes after the copy is written, for each kind of open block.
#[test]
fn rev_closing_an_open_block_of_a_profile_leaves_the_named_file_as_it_was() {
    active::isolate();
    let scratch = Scratch::new("rev-rules/own-profile");
    let state = scratch.path().join("state");
    fs::create_dir_all(&state).expect("create the state");
    for (at, text) in [
        "Intro\n\n```\ncode\n",
        "Intro\n\n~~~~~\ncode",
        "Intro\r\n\r\n````rust\r\ncode\r\n",
    ]
    .into_iter()
    .enumerate()
    {
        let own = scratch.path().join(format!("profile-{at}.md"));
        fs::write(&own, text).expect("write the profile");
        let copy = claude_code::write_own(&state, &format!("session-{at}"), &own, RULES)
            .expect("the session's copy");
        assert_ne!(copy, own, "the copy is another file");
        assert_eq!(
            fs::read_to_string(&own).expect("read the profile"),
            text,
            "the named file changed"
        );
    }
}

/// One config file of the instance `example-org` with standing rules, served by the daemon's
/// loopback listener, whose settings write reaches the instance's own (`PUT
/// /v1/instances/example-org/settings`).
struct Served {
    scratch: Scratch,
}

impl Served {
    fn new(name: &str) -> Self {
        active::isolate();
        let case = Self {
            scratch: Scratch::new(Path::new("rev-rules").join(name)),
        };
        fs::create_dir_all(case.records().join("dispatches")).expect("create dispatches/");
        fs::create_dir_all(case.records().join("decisions")).expect("create decisions/");
        fs::create_dir_all(case.home().join("watch")).expect("create watch/");
        fs::write(case.home().join("agents.json"), "[]").expect("write the session list");
        let text = format!(
            "version: conductor.config/1\n\
             instances:\n\
             \x20 - name: example-org\n\
             \x20   session_prefix: eo\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
             \x20   records: {records:?}\n\
             \x20   state: {state:?}\n\
             \x20   cache: {cache:?}\n\
             \x20   standing_rules: {rules}\n",
            root = case.home().join("example-org"),
            trees = case.home().join("trees"),
            records = case.records(),
            state = case.state(),
            cache = case.dir().join("cache"),
            rules = serde_json::to_string(RULES).expect("JSON"),
        );
        fs::write(case.file(), text).expect("write the config file");
        case
    }

    fn dir(&self) -> &Path {
        self.scratch.path()
    }

    fn home(&self) -> PathBuf {
        self.dir().join("home")
    }

    fn records(&self) -> PathBuf {
        self.home().join("example-org/conductor")
    }

    fn state(&self) -> PathBuf {
        self.dir().join("state")
    }

    fn file(&self) -> PathBuf {
        self.home().join("conductor.yaml")
    }

    fn instance(&self) -> conductor_model::config::Instance {
        let text = fs::read_to_string(self.file()).expect("read the config file");
        config::parse(&text, &self.home())
            .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}"))
            .instances
            .remove(0)
    }

    fn start(&self) -> u16 {
        let home = self.home();
        let mut sources = Sources::new(home.clone(), self.records());
        let cat = |file: &Path| vec![OsString::from("cat"), file.as_os_str().to_owned()];
        sources.agents = cat(&home.join("agents.json"));
        sources.sweep = cat(Path::new("/dev/null"));
        sources.gc = cat(Path::new("/dev/null"));
        sources.checkouts = home.join("example-org");
        sources.trees = home.join("trees");
        sources.targets = home.join("target");
        sources.watch = home.join("watch");
        sources.ci_state = home.join("watch/ci.state");
        sources.var_tmp = home.join("var-tmp");
        sources.health.state = self.state();
        sources.config_file = Some(self.file());
        sources.instance_name = Some("example-org".to_owned());
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind a free port");
        let port = listener.local_addr().expect("the bound address").port();
        served::serve(listener, sources, Some(self.state()));
        port
    }
}

fn exchange(port: u16, raw: &[u8]) -> (u16, String) {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect to the server");
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .expect("set a read timeout");
    stream.write_all(raw).expect("send the request");
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes).expect("read the answer");
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let (head, body) = text.split_once("\r\n\r\n").unwrap_or((&text, ""));
    let status = head
        .split(' ')
        .nth(1)
        .and_then(|code| code.parse().ok())
        .unwrap_or_else(|| panic!("no status line: {head}"));
    (status, body.to_owned())
}

fn page_token(port: u16) -> String {
    let (status, page) = exchange(
        port,
        format!("GET / HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n").as_bytes(),
    );
    assert_eq!(status, 200, "GET /: {page}");
    let marker = r#"<meta name="conductor-token" content=""#;
    let at = page.find(marker).expect("the page names its token");
    let rest = &page[at + marker.len()..];
    rest[..rest.find('"').expect("the token's closing quote")].to_owned()
}

fn settings_write(port: u16, body: &Value) -> (u16, String) {
    let token = page_token(port);
    let body = serde_json::to_vec(body).expect("a JSON body");
    let mut raw = format!(
        "PUT /v1/instances/example-org/settings HTTP/1.1\r\n\
         Host: 127.0.0.1:{port}\r\n\
         Origin: http://127.0.0.1:{port}\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         X-Conductor-Token: {token}\r\n\
         Connection: close\r\n\r\n",
        body.len()
    )
    .into_bytes();
    raw.extend_from_slice(&body);
    exchange(port, &raw)
}

/// The runtime's settings write refuses a change of `standing_rules` in every spelling a path
/// takes: the key, a deletion of it, a nested path, an index, another case, white space around
/// the key, a YAML merge key, and an empty path that would name the instance's mapping. After each
/// attempt the next session reads the control plane's text.
#[test]
fn rev_no_spelling_of_a_settings_write_path_changes_the_standing_rules() {
    let case = Served::new("settings-spellings");
    let loaded = case.instance();
    assert_eq!(loaded.standing_rules.as_deref(), Some(RULES));
    let port = case.start();
    let other = "\"Another rule.\"";
    let attempts: [(&str, Option<&str>); 12] = [
        ("standing_rules", Some(other)),
        ("standing_rules", None),
        ("standing_rules", Some("\"\"")),
        ("standing_rules.text", Some(other)),
        ("standing_rules[0]", Some(other)),
        ("Standing_Rules", Some(other)),
        ("STANDING_RULES", Some(other)),
        (" standing_rules", Some(other)),
        ("standing_rules ", Some(other)),
        ("<<", Some("{standing_rules: Another rule.}")),
        (
            "",
            Some("{name: example-org, standing_rules: Another rule.}"),
        ),
        (".standing_rules", Some(other)),
    ];
    let mut changed = Vec::new();
    for (path, value) in attempts {
        let mut change = json!({"path": path});
        if let Some(value) = value {
            change["value"] = json!(value);
        }
        let (status, body) = settings_write(
            port,
            &json!({"changes": [change], "reason": "a review of the one writer"}),
        );
        let read = config::standing_rules_in(&case.file(), &case.home(), &loaded);
        if read.as_deref() != Some(RULES) || status == 200 {
            changed.push(format!("{path:?} = {value:?}: {status} {body} -> {read:?}"));
        }
    }
    assert!(
        changed.is_empty(),
        "settings writes that went through or changed the rules:\n{}",
        changed.join("\n")
    );
}

const NAMESPACE: &str = "conductor-instances";
const IMAGE: &str = "registry.example.com/conductor:1.0.0";

/// A daemon that runs `alpha` as a pod against a fake API server.
struct Cluster {
    dir: PathBuf,
    fake: FakeKube,
    _scratch: Scratch,
}

impl Cluster {
    fn new(name: &str) -> Self {
        active::isolate();
        let scratch = Scratch::new(Path::new("rev-rules").join(name));
        let dir = scratch.path().to_owned();
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        fs::write(dir.join("secrets/alpha-model"), "model-token-value-0000")
            .expect("write a secret");
        let fake = FakeKube::start(&dir);
        Self {
            dir,
            fake,
            _scratch: scratch,
        }
    }

    fn home(&self) -> PathBuf {
        self.dir.join("home")
    }

    fn file(&self) -> PathBuf {
        self.dir.join("conductor.yaml")
    }

    /// Writes the config file with `alpha`, its standing rules `rules` where given.
    fn config(&self, rules: Option<&str>) {
        let rules = rules
            .map(|rules| {
                format!(
                    "\x20   standing_rules: {}\n",
                    serde_json::to_string(rules).expect("JSON")
                )
            })
            .unwrap_or_default();
        let text = format!(
            "version: conductor.config/2\n\
             daemon:\n\
             \x20 store: {store:?}\n\
             \x20 secrets: {{source: files, dir: {secrets:?}}}\n\
             \x20 kubernetes:\n\
             \x20   namespace: {NAMESPACE}\n\
             \x20   image: {IMAGE}\n\
             \x20   owner: example-org\n\
             \x20   api_port: 7310\n\
             \x20   volume: {{size: 20G, storage_class: fast-local}}\n\
             \x20   env: []\n\
             instances:\n\
             \x20 - name: alpha\n\
             \x20   placement: none\n\
             \x20   session_prefix: alpha\n\
             \x20   sources: [{{github: example-org}}]\n\
             \x20   model_token: secret:alpha-model\n\
             {rules}",
            store = format!("sqlite:{}", self.dir.join("shared.sqlite").display()),
            secrets = self.dir.join("secrets"),
        );
        fs::write(self.file(), text).expect("write the config file");
    }

    fn daemon(&self) -> Daemon<Manager> {
        Daemon::open(
            self.file(),
            self.home(),
            self.dir.join("daemon"),
            Binary {
                path: PathBuf::from("/opt/conductor/bin/conductor"),
                version: "1.0.0".to_owned(),
            },
            Manager::Pods(Box::new(Pods::new(self.fake.client()))),
        )
        .expect("open the daemon")
    }

    fn set_running(&self) {
        let mut pod = self
            .fake
            .object(NAMESPACE, "pods", "conductor-instance-alpha")
            .expect("the pod");
        pod["status"] = json!({
            "phase": "Running",
            "podIP": "127.0.0.1",
            "containerStatuses": [
                {"name": "runtime", "ready": true, "state": {"running": {"startedAt": "now"}}}
            ],
        });
        self.fake.put(NAMESPACE, "pods", pod);
    }

    fn held_rules(&self) -> Option<String> {
        let secret = self
            .fake
            .object(NAMESPACE, "secrets", "conductor-instance-alpha")
            .expect("the Secret");
        let encoded = secret["data"]["conductor.yaml"]
            .as_str()
            .expect("the rendered file");
        let text =
            String::from_utf8(conductor_cli::daemon::pods::base64_decode(encoded).expect("base64"))
                .expect("UTF-8");
        let file: serde_yaml::Value = serde_yaml::from_str(&text).expect("YAML");
        file["instances"][0]["standing_rules"]
            .as_str()
            .map(str::to_owned)
    }
}

/// The doctor's verdict on `update` on Secrets follows what the daemon needs: where an instance's
/// standing rules were removed while its pod runs, the daemon writes the rendered file without
/// them into the Secret, which takes `update`. With a Role of the base, which grants every other
/// right, that write is refused and the instance's `detail` names `update`; the doctor, reading
/// the same config, in which no instance names `standing_rules` any more, must not pass the Role.
#[test]
fn rev_the_doctor_fails_a_role_without_update_while_a_removal_of_the_rules_waits_on_it() {
    let case = Cluster::new("doctor-removal");
    case.config(Some(RULES));
    let mut daemon = case.daemon();
    let start = Instant::now();
    daemon
        .pass(ReconcileReason::Cadence, start)
        .expect("a pass");
    case.set_running();
    daemon
        .pass(ReconcileReason::Cadence, start + Duration::from_secs(2))
        .expect("a pass");
    assert_eq!(case.held_rules().as_deref(), Some(RULES));

    case.fake.forbid("PUT", "secrets");
    case.config(None);
    daemon
        .pass(
            ReconcileReason::ConfigChange,
            start + Duration::from_secs(4),
        )
        .expect("a pass");
    let row = daemon
        .rows()
        .into_iter()
        .find(|row| row["name"] == "alpha")
        .expect("the row of alpha");
    let detail = row["detail"].as_str().unwrap_or_default().to_owned();
    assert!(
        detail.contains("update"),
        "the daemon needs update on Secrets for the removal: {row}"
    );
    assert_eq!(
        case.held_rules().as_deref(),
        Some(RULES),
        "the pod's sessions still read the removed text"
    );

    for resource in ["pods", "persistentvolumeclaims", "secrets"] {
        for verb in ["create", "get", "list", "delete"] {
            case.fake.allow(NAMESPACE, verb, resource);
        }
    }
    case.fake.allow(NAMESPACE, "get", "pods/log");
    let text = fs::read_to_string(case.file()).expect("the config file");
    let config = config::parse(&text, &case.home()).expect("a valid file");
    let lines = conductor_cli::doctor::pod_rights_lines(&config, Ok(&case.fake.client()));
    let failures: Vec<String> = lines
        .iter()
        .filter_map(|(_, failure)| failure.clone())
        .collect();
    let row = lines
        .iter()
        .find(|(line, _)| line.contains("update secrets"))
        .map(|(line, _)| line.clone())
        .unwrap_or_default();
    assert!(
        !failures.is_empty(),
        "the doctor passes a Role the daemon's write is refused under; its row: {row:?}; the \
         instance's detail: {detail:?}"
    );
}
