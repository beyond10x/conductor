//! story:anonymous-pod-no-forge-secret, second round: conformance cases for the
//! contract of `forge_access: anonymous` that the change's own suites do not state.
//!
//! - The pod (`pods::manifests`): a forge token the file names that is not the daemon's own, a
//!   forge token written as a value, git's environment configuration in other spellings and
//!   counts, and variables that point git or curl at a credential or a configuration file.
//! - The anonymous clone and fetch (`checkouts::bring`) against a local HTTP listener: an extra
//!   header of the checkout's own configuration, a template's configuration, a netrc file, and
//!   the proxy of the pod's environment.
//! - The checkout's own configuration (third round): a key that cannot be removed, or that an
//!   included file names, stops the anonymous fetch; a fetch without the member leaves it alone.
//! - The guard: push spellings beside those the change names, reads that stay allowed, and an
//!   instance without the member.
//! - The doctor's and the collectors' forge programs, and validation without the member.
//!
//! No case calls a real forge or a real cluster; every listener is on 127.0.0.1.

mod active;
mod scratch;

use std::fs;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use conductor_cli::collect::checkouts::{self, Brought, Wanted};
use conductor_cli::config;
use conductor_cli::daemon::pods;
use conductor_cli::guard::{self, Places, SessionList};
use conductor_model::dispatch::Verdict;
use scratch::Scratch;
use serde_json::{Value, json};

/// The words every refusal of a write of an anonymous instance names.
const NAMED: &str = "forge_access: anonymous";

// ---------------------------------------------------------------------------------------------
// The pod
// ---------------------------------------------------------------------------------------------

fn file(env: &[&str], instances: &str) -> String {
    let env: String = env
        .iter()
        .map(|entry| format!("\n      - {entry}"))
        .collect();
    format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 store: sqlite:/var/lib/example/shared.sqlite\n\
         \x20 secrets: {{source: files, dir: /var/lib/example/secrets}}\n\
         \x20 kubernetes:\n\
         \x20   namespace: conductor-instances\n\
         \x20   image: registry.example.com/conductor:1.0.0\n\
         \x20   owner: example-owner\n\
         \x20   api_port: 7310\n\
         \x20   volume: {{size: 20G}}\n\
         \x20   env:{env}\n\
         instances:\n{instances}"
    )
}

fn instance(name: &str, sources: &str, extra: &str) -> String {
    format!(
        "  - name: {name}\n\
         \x20   placement: none\n\
         \x20   session_prefix: {name}\n\
         \x20   sources: [{sources}]\n\
         {extra}"
    )
}

fn anonymous_alpha() -> String {
    instance(
        "alpha",
        "{github: example-owner, include: [one]}",
        "\x20   forge_access: anonymous\n",
    )
}

/// The manifests of the instance `alpha` of `text`, which validates.
fn made(text: &str) -> pods::Manifests {
    active::isolate();
    let parsed = config::parse(text, Path::new("/srv/example"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    let kubernetes = parsed.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let alpha = parsed
        .instances
        .iter()
        .find(|instance| instance.name.0 == "alpha")
        .expect("alpha");
    pods::manifests(&kubernetes, &parsed, alpha)
}

fn env_of(pod: &Value) -> Vec<Value> {
    pod["spec"]["containers"][0]["env"]
        .as_array()
        .expect("the container's env")
        .clone()
}

/// The env entries of the pod that read the secret `secret`, by name.
fn reading(pod: &Value, secret: &str) -> Vec<String> {
    env_of(pod)
        .iter()
        .filter(|entry| entry["valueFrom"]["secretKeyRef"]["key"] == secret)
        .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
        .collect()
}

/// The `GIT_CONFIG_*` entries of the pod, each `name=value` or `name<-secret`.
fn gits(pod: &Value) -> Vec<String> {
    env_of(pod)
        .iter()
        .filter_map(|entry| {
            let name = entry["name"].as_str()?;
            name.starts_with("GIT_CONFIG_").then(|| {
                entry["value"].as_str().map_or_else(
                    || {
                        format!(
                            "{name}<-{}",
                            entry["valueFrom"]["secretKeyRef"]["key"]
                                .as_str()
                                .unwrap_or_default()
                        )
                    },
                    |value| format!("{name}={value}"),
                )
            })
        })
        .collect()
}

/// A forge token the file names as another instance's `forge_token` is a forge credential the
/// daemon knows of. A `daemon.kubernetes.env` entry under another name, or a kept proxy pair,
/// that names its secret puts it into the anonymous pod.
#[test]
fn another_instances_forge_token_under_a_second_name_stays_out_of_the_anonymous_pod() {
    let text = file(
        &[
            "{name: GH_TOKEN, secret: secret:forge-token}",
            "{name: EXAMPLE_TOKEN, secret: secret:beta-forge}",
            "{name: GIT_CONFIG_COUNT, value: \"1\"}",
            "{name: GIT_CONFIG_KEY_0, value: http.proxy}",
            "{name: GIT_CONFIG_VALUE_0, secret: secret:beta-forge}",
        ],
        &[
            anonymous_alpha(),
            instance(
                "beta",
                "{github: example-owner}",
                "\x20   forge_token: secret:beta-forge\n",
            ),
        ]
        .concat(),
    );
    let made = made(&text);
    let readers = reading(&made.pod, "beta-forge");
    assert!(
        !made.secrets.contains("beta-forge") && readers.is_empty(),
        "the anonymous pod carries beta's forge token: its Secret holds {:?}, its container \
         reads it as {readers:?}",
        made.secrets
    );
}

/// A forge token written as a `value` reaches no anonymous pod under a second name either.
#[test]
fn a_forge_token_written_as_a_value_under_a_second_name_stays_out_of_the_anonymous_pod() {
    let token = "example-forge-value-7777";
    let text = file(
        &[
            &format!("{{name: GH_TOKEN, value: {token}}}"),
            &format!("{{name: EXAMPLE_PAT, value: {token}}}"),
        ],
        &anonymous_alpha(),
    );
    let made = made(&text);
    let carrying: Vec<String> = env_of(&made.pod)
        .iter()
        .filter(|entry| entry["value"] == token)
        .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
        .collect();
    assert!(
        carrying.is_empty(),
        "the anonymous pod carries the value of GH_TOKEN as {carrying:?}"
    );
}

/// git's environment configuration of the installer in other spellings: a section and a name in
/// other cases, a URL between, a pair whose value is the forge token's secret, pairs past the
/// count, and counts that are no number. Only proxy and CA pairs whose value names no forge
/// secret are kept, renumbered under a count that matches.
#[test]
fn gits_environment_configuration_keeps_only_proxy_and_ca_pairs_in_any_spelling() {
    let pairs: [(&str, &str); 8] = [
        ("HTTP.Proxy", "value: \"http://proxy.example.com:3128\""),
        (
            "http.https://github.com/.PROXY",
            "secret: secret:forge-token",
        ),
        (
            "http.https://github.com/.extraHeader",
            "value: \"X-Example: y\"",
        ),
        ("credential.helper", "value: store"),
        ("http.sslCAInfo", "secret: secret:setting"),
        ("http.proxy", "secret: secret:forge-token"),
        ("Http.SslCaPath", "value: /etc/example/certs"),
        ("http.proxy", "value: \"http://past-the-count.example.com\""),
    ];
    let entries = |count: &str| {
        let mut entries = vec![
            "{name: GH_TOKEN, secret: secret:forge-token}".to_owned(),
            format!("{{name: GIT_CONFIG_COUNT, value: {count:?}}}"),
        ];
        for (index, (key, value)) in pairs.iter().enumerate() {
            entries.push(format!("{{name: GIT_CONFIG_KEY_{index}, value: {key:?}}}"));
            entries.push(format!("{{name: GIT_CONFIG_VALUE_{index}, {value}}}"));
        }
        entries
    };
    let render = |count: &str| {
        let entries = entries(count);
        let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
        made(&file(&entries, &anonymous_alpha()))
    };
    let mut wrong = Vec::new();
    let seven = render("7");
    let wanted = [
        "GIT_CONFIG_COUNT=3",
        "GIT_CONFIG_KEY_0=HTTP.Proxy",
        "GIT_CONFIG_VALUE_0=http://proxy.example.com:3128",
        "GIT_CONFIG_KEY_1=http.sslCAInfo",
        "GIT_CONFIG_VALUE_1<-setting",
        "GIT_CONFIG_KEY_2=Http.SslCaPath",
        "GIT_CONFIG_VALUE_2=/etc/example/certs",
    ];
    if gits(&seven.pod) != wanted {
        wrong.push(format!("count 7: {:?}", gits(&seven.pod)));
    }
    for (count, kept) in [("abc", 0), ("-1", 0), (" 7 ", 3), ("99", 4), ("0", 0)] {
        let made = render(count);
        let names = gits(&made.pod);
        let count_entry = names
            .iter()
            .find(|entry| entry.starts_with("GIT_CONFIG_COUNT="))
            .cloned();
        let keys = names
            .iter()
            .filter(|entry| entry.starts_with("GIT_CONFIG_KEY_"))
            .count();
        let expected_count = (kept > 0).then(|| format!("GIT_CONFIG_COUNT={kept}"));
        if keys != kept || count_entry != expected_count {
            wrong.push(format!("count {count:?}: {names:?}"));
        }
        if made.secrets.contains("forge-token") || !reading(&made.pod, "forge-token").is_empty() {
            wrong.push(format!("count {count:?}: the forge token is in the pod"));
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// Two entries of one name are refused at validation, so no duplicate index of git's
/// configuration chooses which pair the pod keeps.
#[test]
fn a_duplicate_git_config_entry_is_refused_at_its_path() {
    active::isolate();
    let text = file(
        &[
            "{name: GIT_CONFIG_COUNT, value: \"1\"}",
            "{name: GIT_CONFIG_KEY_0, value: http.proxy}",
            "{name: GIT_CONFIG_KEY_0, value: credential.helper}",
            "{name: GIT_CONFIG_VALUE_0, value: store}",
        ],
        &anonymous_alpha(),
    );
    let problems = config::parse(&text, Path::new("/srv/example"))
        .err()
        .unwrap_or_else(|| panic!("the file validates:\n{text}"));
    assert!(
        problems
            .iter()
            .any(|problem| problem.path == "daemon.kubernetes.env[2].name"),
        "{problems:?}"
    );
}

/// Variables beside `CREDENTIAL_POINTERS` that point git or its HTTP library at a credential or
/// at a configuration file: a netrc file (`NETRC`, `CURL_HOME`), a template whose `config` the
/// clone copies (`GIT_TEMPLATE_DIR`), and a TLS client key (`GIT_SSL_KEY`, `GIT_SSL_CERT`).
/// Printed: what else of the list the anonymous pod carries.
#[test]
fn variables_that_point_git_or_curl_at_a_credential_file_stay_out_of_the_anonymous_pod() {
    let pointers = [
        "NETRC",
        "CURL_HOME",
        "GIT_TEMPLATE_DIR",
        "GIT_SSL_KEY",
        "GIT_SSL_CERT",
    ];
    let others = [
        "GIT_EXEC_PATH",
        "GIT_PROXY_COMMAND",
        "GCM_CREDENTIAL_STORE",
        "GH_HOST",
        "GH_REPO",
        "GIT_TERMINAL_PROMPT",
        "GIT_SSL_NO_VERIFY",
        "GIT_SSL_CAINFO",
        "GIT_CONFIG_NOSYSTEM",
    ];
    let entries: Vec<String> = pointers
        .iter()
        .chain(&others)
        .map(|name| {
            format!(
                "{{name: {name}, value: /etc/example/{}}}",
                name.to_lowercase()
            )
        })
        .collect();
    let entries: Vec<&str> = entries.iter().map(String::as_str).collect();
    let made = made(&file(&entries, &anonymous_alpha()));
    let names: Vec<String> = env_of(&made.pod)
        .iter()
        .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
        .collect();
    let carried = |list: &[&str]| -> Vec<String> {
        list.iter()
            .filter(|name| names.iter().any(|carried| carried == *name))
            .map(|name| (*name).to_owned())
            .collect()
    };
    eprintln!(
        "the anonymous pod carries, of the others: {:?}",
        carried(&others)
    );
    let carried = carried(&pointers);
    assert!(carried.is_empty(), "the anonymous pod carries {carried:?}");
}

// ---------------------------------------------------------------------------------------------
// The anonymous clone and fetch
// ---------------------------------------------------------------------------------------------

/// The cases that change this process's environment, one at a time.
fn environment() -> MutexGuard<'static, ()> {
    static LOCK: Mutex<()> = Mutex::new(());
    LOCK.lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The variables a case sets, removed again; `HOME` set to `home`, so no file of the real home
/// (its `.netrc`) decides a case.
fn reset_environment(home: &Path) {
    // SAFETY: every case of this binary that reads or writes the environment holds
    // [`environment`]; std's own environment lock orders this against `Command::spawn`.
    unsafe {
        std::env::set_var("HOME", home);
        for variable in [
            "NETRC",
            "CURL_HOME",
            "GIT_TEMPLATE_DIR",
            "GIT_CONFIG_COUNT",
            "GIT_CONFIG_KEY_0",
            "GIT_CONFIG_VALUE_0",
            "GIT_CONFIG_PARAMETERS",
            "GIT_TRACE",
            "http_proxy",
            "https_proxy",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "all_proxy",
            "no_proxy",
            "NO_PROXY",
        ] {
            std::env::remove_var(variable);
        }
    }
}

fn set(variable: &str, value: impl AsRef<std::ffi::OsStr>) {
    // SAFETY: as in [`reset_environment`].
    unsafe { std::env::set_var(variable, value) };
}

/// A listener on 127.0.0.1 that keeps the head of every request it reads, and answers `401` with
/// a Basic challenge to one without `Authorization` when `challenge`, else `404`.
struct Listener {
    port: u16,
    heads: Arc<Mutex<Vec<String>>>,
}

impl Listener {
    fn start(challenge: bool) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind 127.0.0.1");
        let port = listener.local_addr().expect("its address").port();
        let heads = Arc::new(Mutex::new(Vec::new()));
        let kept = Arc::clone(&heads);
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let mut head = Vec::new();
                let mut byte = [0_u8; 1];
                while !head.ends_with(b"\r\n\r\n") {
                    match stream.read(&mut byte) {
                        Ok(1) => head.push(byte[0]),
                        _ => break,
                    }
                }
                let head = String::from_utf8_lossy(&head).into_owned();
                let authorized = head.to_ascii_lowercase().contains("\nauthorization:");
                kept.lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .push(head);
                let answer = if challenge && !authorized {
                    "HTTP/1.1 401 Unauthorized\r\nWWW-Authenticate: Basic realm=\"example\"\r\n\
                     Content-Length: 0\r\nConnection: close\r\n\r\n"
                } else {
                    "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                };
                let _ = stream.write_all(answer.as_bytes());
            }
        });
        Self { port, heads }
    }

    fn heads(&self) -> Vec<String> {
        self.heads
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    fn url(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }
}

fn wanted(url: &str, checkout: Option<PathBuf>) -> Wanted {
    Wanted {
        name: "one".to_owned(),
        path: "one".to_owned(),
        url: url.to_owned(),
        forge: "github",
        host: "github.com".to_owned(),
        checkout,
        archived: false,
        anonymous: true,
    }
}

fn which(name: &str) -> PathBuf {
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .map(|dir| dir.join(name))
        .find(|file| file.is_file())
        .unwrap_or_else(|| panic!("{name} is on PATH"))
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new(which("git"))
        .current_dir(dir)
        .args(args)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env_remove("GIT_CONFIG_COUNT")
        .stdin(Stdio::null())
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        dir.display(),
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The anonymous fetch clears `http.extraHeader` (`-c http.extraHeader=`), and the doc comment of
/// `without_credential` says it empties the list of extra headers. A URL-specific header of the
/// checkout's own configuration is a better match than the empty one of the command line, which
/// git's URL matching therefore skips: the header is sent.
#[test]
fn the_anonymous_fetch_sends_no_extra_header_of_the_checkouts_own_configuration() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/fetch-header");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    fs::create_dir_all(dir.join("root/one")).expect("checkout");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let checkout = dir.join("root/one");
    let origin = format!("{}example-owner/one.git", listener.url());
    git(&checkout, &["init", "--quiet", "--initial-branch=main"]);
    git(&checkout, &["remote", "add", "origin", &origin]);
    git(
        &checkout,
        &["config", "http.extraHeader", "X-Plain: plain-9999"],
    );
    git(
        &checkout,
        &[
            "config",
            &format!("http.{}.extraHeader", listener.url()),
            "X-Url: url-level-8888",
        ],
    );
    let brought = checkouts::bring(
        &dir.join("root"),
        &wanted(&origin, Some(checkout)),
        Some(Duration::from_secs(20)),
    );
    let heads = listener.heads();
    assert!(
        !heads.is_empty(),
        "no request reached the listener: {brought:?}"
    );
    let sent: Vec<&str> = ["plain-9999", "url-level-8888"]
        .into_iter()
        .filter(|header| heads.iter().any(|head| head.contains(header)))
        .collect();
    assert!(
        sent.is_empty(),
        "the anonymous fetch sends the extra headers {sent:?}:\n{}",
        heads.join("---\n")
    );
}

/// A credential helper the checkout's own configuration names for the origin's URL: `-c
/// credential.helper=` empties the list of helpers, so the anonymous fetch answers a Basic
/// challenge with no login.
#[test]
fn the_anonymous_fetch_runs_no_url_specific_helper_of_the_checkouts_own_configuration() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/fetch-helper");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    fs::create_dir_all(dir.join("root/one")).expect("checkout");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(true);
    let checkout = dir.join("root/one");
    let origin = format!("{}example-owner/one.git", listener.url());
    git(&checkout, &["init", "--quiet", "--initial-branch=main"]);
    git(&checkout, &["remote", "add", "origin", &origin]);
    git(
        &checkout,
        &[
            "config",
            &format!("credential.{}.helper", listener.url()),
            "!f() { echo username=example; echo password=helper-secret-4444; }; f",
        ],
    );
    let brought = checkouts::bring(
        &dir.join("root"),
        &wanted(&origin, Some(checkout)),
        Some(Duration::from_secs(20)),
    );
    let heads = listener.heads();
    assert!(
        !heads.is_empty(),
        "no request reached the listener: {brought:?}"
    );
    assert!(
        !heads
            .iter()
            .any(|head| head.to_ascii_lowercase().contains("\nauthorization:")),
        "the anonymous fetch answers with the helper's login: {brought:?}\n{}",
        heads.join("---\n")
    );
}

/// A template directory (`GIT_TEMPLATE_DIR`, which the anonymous pod keeps) whose `config` names a
/// rewrite and an extra header: the anonymous clone reads both, as the clone's own configuration
/// is the template's.
#[test]
fn the_anonymous_clone_reads_no_rewrite_or_header_of_a_template_configuration() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/template");
    let dir = scratch.path().to_owned();
    for sub in ["home", "root", "template"] {
        fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
    }
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    fs::write(
        dir.join("template/config"),
        format!(
            "[url \"{url}\"]\n\tinsteadOf = https://forge.example.invalid/\n\
             [http \"{url}\"]\n\textraHeader = X-Template: template-6666\n",
            url = listener.url()
        ),
    )
    .expect("write the template's config");
    set("GIT_TEMPLATE_DIR", dir.join("template"));
    let brought = checkouts::bring(
        &dir.join("root"),
        &wanted("https://forge.example.invalid/example-owner/one.git", None),
        None,
    );
    reset_environment(&dir.join("home"));
    let heads = listener.heads();
    let header = heads.iter().any(|head| head.contains("template-6666"));
    assert!(
        heads.is_empty() && !header,
        "the anonymous clone was rewritten to the listener ({} requests), header sent: {header}; \
         {brought:?}\n{}",
        heads.len(),
        heads.join("---\n")
    );
}

/// A netrc file gives curl a login for the host: the anonymous clone answers a Basic challenge
/// with none of it, whether the file is the home's `.netrc`, `NETRC` names it, or it is under
/// `CURL_HOME`.
#[test]
fn the_anonymous_clone_sends_no_login_of_a_netrc_file() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/netrc");
    let dir = scratch.path().to_owned();
    let netrc = "machine 127.0.0.1 login example password netrc-secret-5555\n";
    let mut sent = Vec::new();
    for (case, how) in [("home", 0), ("netrc-variable", 1), ("curl-home", 2)] {
        let base = dir.join(case);
        for sub in ["home", "root", "curl"] {
            fs::create_dir_all(base.join(sub)).expect("create the case's directories");
        }
        reset_environment(&base.join("home"));
        match how {
            0 => fs::write(base.join("home/.netrc"), netrc).expect("write .netrc"),
            1 => {
                fs::write(base.join("netrc"), netrc).expect("write netrc");
                set("NETRC", base.join("netrc"));
            }
            _ => {
                fs::write(base.join("curl/.netrc"), netrc).expect("write .netrc");
                set("CURL_HOME", base.join("curl"));
            }
        }
        let listener = Listener::start(true);
        let url = format!("{}example-owner/one.git", listener.url());
        let brought = checkouts::bring(&base.join("root"), &wanted(&url, None), None);
        reset_environment(&base.join("home"));
        let heads = listener.heads();
        eprintln!("{case}: {} requests; {brought:?}", heads.len());
        if heads
            .iter()
            .any(|head| head.to_ascii_lowercase().contains("\nauthorization:"))
        {
            sent.push(case);
        }
    }
    assert!(
        sent.is_empty(),
        "the anonymous clone answers with the netrc login in {sent:?}"
    );
}

/// The proxy of the pod's own environment configuration (`GIT_CONFIG_COUNT`), which the anonymous
/// pod keeps, still reaches the anonymous clone.
#[test]
fn the_proxy_of_the_pods_environment_reaches_the_anonymous_clone() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/proxy");
    let dir = scratch.path().to_owned();
    for sub in ["home", "root"] {
        fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
    }
    reset_environment(&dir.join("home"));
    let proxy = Listener::start(false);
    set("GIT_CONFIG_COUNT", "1");
    set("GIT_CONFIG_KEY_0", "http.proxy");
    set("GIT_CONFIG_VALUE_0", proxy.url());
    let brought = checkouts::bring(
        &dir.join("root"),
        &wanted("http://forge.example.invalid/example-owner/one.git", None),
        None,
    );
    reset_environment(&dir.join("home"));
    let heads = proxy.heads();
    assert!(
        heads
            .iter()
            .any(|head| head.contains("forge.example.invalid/example-owner/one.git")),
        "the clone did not go through the proxy: {brought:?}\n{}",
        heads.join("---\n")
    );
}

// ---------------------------------------------------------------------------------------------
// The checkout's own configuration that cannot be cleared
// ---------------------------------------------------------------------------------------------

/// The lines of the trace git wrote to `trace` (`GIT_TRACE`) that start a fetch.
fn fetches(trace: &Path) -> Vec<String> {
    fs::read_to_string(trace)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("trace: built-in: git fetch"))
        .map(str::to_owned)
        .collect()
}

/// The lines of the trace git wrote to `trace` that read or change a configuration file.
fn config_calls(trace: &Path) -> Vec<String> {
    fs::read_to_string(trace)
        .unwrap_or_default()
        .lines()
        .filter(|line| {
            line.contains("trace: built-in: git config")
                && (line.contains("--get-regexp") || line.contains("--unset"))
        })
        .map(str::to_owned)
        .collect()
}

/// A checkout's `.git` and its `config` made read-only (`0555`, `0444`) while it lives, and
/// writable again when it is dropped, a failed case included.
struct ReadOnly(PathBuf);

impl ReadOnly {
    fn new(checkout: &Path) -> Self {
        use std::os::unix::fs::PermissionsExt as _;
        let git = checkout.join(".git");
        fs::set_permissions(git.join("config"), fs::Permissions::from_mode(0o444))
            .expect("make the config read-only");
        fs::set_permissions(&git, fs::Permissions::from_mode(0o555)).expect("make .git read-only");
        Self(git)
    }
}

impl Drop for ReadOnly {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
        let _ = fs::set_permissions(self.0.join("config"), fs::Permissions::from_mode(0o644));
    }
}

/// A checkout whose own configuration cannot be written: its extra header cannot be removed, so
/// the anonymous fetch is not run, and the member failure names the key and not its value.
#[test]
fn an_extra_header_that_cannot_be_removed_stops_the_anonymous_fetch() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/read-only");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    fs::create_dir_all(dir.join("root/one")).expect("checkout");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let checkout = dir.join("root/one");
    let origin = format!("{}example-owner/one.git", listener.url());
    git(&checkout, &["init", "--quiet", "--initial-branch=main"]);
    git(&checkout, &["remote", "add", "origin", &origin]);
    git(
        &checkout,
        &["config", "http.extraHeader", "X-Plain: read-only-7777"],
    );
    let trace = dir.join("trace");
    let _read_only = ReadOnly::new(&checkout);
    assert!(
        fs::write(checkout.join(".git/config.lock"), "").is_err(),
        "the case needs a .git this process cannot write in"
    );
    set("GIT_TRACE", &trace);
    let brought = checkouts::bring(
        &dir.join("root"),
        &wanted(&origin, Some(checkout)),
        Some(Duration::from_secs(20)),
    );
    reset_environment(&dir.join("home"));
    let heads = listener.heads();
    let fetched = fetches(&trace);
    assert!(
        heads.is_empty() && fetched.is_empty(),
        "the anonymous fetch ran ({} requests, fetches {fetched:?}): {brought:?}",
        heads.len()
    );
    let checkouts::Brought::Failed(line) = &brought else {
        panic!("not one member failure: {brought:?}");
    };
    assert!(
        line.contains("`http.extraheader`")
            && line.contains("was not run")
            && line.contains("still holds")
            && !line.contains("read-only-7777"),
        "the failure does not name the key, or names its value: {line}"
    );
}

/// A checkout whose own configuration includes a file (`include.path`, `includeIf`) that names an
/// extra header or a rewrite for the origin: the anonymous fetch would follow it, nothing in the
/// checkout's own file can remove it, so the fetch is not run, and the failure names the key.
#[test]
fn a_header_or_rewrite_of_an_included_file_stops_the_anonymous_fetch() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/included");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let header = format!(
        "[http \"{}\"]\n\textraHeader = X-Included: included-3333\n",
        listener.url()
    );
    let rewrite = format!(
        "[url \"{}\"]\n\tinsteadOf = https://forge.example.invalid/\n",
        listener.url()
    );
    fs::write(dir.join("header.cfg"), header).expect("write the included header");
    fs::write(dir.join("rewrite.cfg"), rewrite).expect("write the included rewrite");
    let mut wrong = Vec::new();
    for (name, origin, key, value, include) in [
        (
            "one",
            format!("{}example-owner/one.git", listener.url()),
            format!("`http.{}.extraheader`", listener.url()),
            "included-3333",
            ("include.path".to_owned(), dir.join("header.cfg")),
        ),
        (
            "two",
            "https://forge.example.invalid/example-owner/two.git".to_owned(),
            format!("`url.{}.insteadof`", listener.url()),
            "forge.example.invalid",
            (
                format!("includeIf.gitdir:{}/.path", text_of(&dir.join("root/two"))),
                dir.join("rewrite.cfg"),
            ),
        ),
    ] {
        let checkout = dir.join("root").join(name);
        fs::create_dir_all(&checkout).expect("checkout");
        git(&checkout, &["init", "--quiet", "--initial-branch=main"]);
        git(&checkout, &["remote", "add", "origin", &origin]);
        git(&checkout, &["config", &include.0, &text_of(&include.1)]);
        let trace = dir.join(format!("{name}.trace"));
        set("GIT_TRACE", &trace);
        let brought = checkouts::bring(
            &dir.join("root"),
            &wanted(&origin, Some(checkout)),
            Some(Duration::from_secs(20)),
        );
        reset_environment(&dir.join("home"));
        let fetched = fetches(&trace);
        let refused = matches!(&brought, checkouts::Brought::Failed(line)
            if line.contains(&key) && line.contains("was not run") && !line.contains(value));
        if !fetched.is_empty() || !refused {
            wrong.push(format!("{name}: fetches {fetched:?}; {brought:?}"));
        }
    }
    let heads = listener.heads();
    assert!(
        wrong.is_empty() && heads.is_empty(),
        "{} requests reached the listener; {wrong:#?}",
        heads.len()
    );
}

/// A fetch of an instance without the member neither reads nor changes the checkout's own
/// configuration for keys a fetch would send: its extra header stays, and the fetch runs.
#[test]
fn a_fetch_without_the_member_leaves_the_checkouts_own_configuration_alone() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/fetch-without-member");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    fs::create_dir_all(dir.join("root/one")).expect("checkout");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let checkout = dir.join("root/one");
    let origin = format!("{}example-owner/one.git", listener.url());
    git(&checkout, &["init", "--quiet", "--initial-branch=main"]);
    git(&checkout, &["remote", "add", "origin", &origin]);
    git(
        &checkout,
        &["config", "http.extraHeader", "X-Plain: kept-2222"],
    );
    let before = fs::read(checkout.join(".git/config")).expect("read the config");
    let trace = dir.join("trace");
    set("GIT_TRACE", &trace);
    let brought = checkouts::bring(
        &dir.join("root"),
        &Wanted {
            anonymous: false,
            ..wanted(&origin, Some(checkout.clone()))
        },
        Some(Duration::from_secs(20)),
    );
    reset_environment(&dir.join("home"));
    let after = fs::read(checkout.join(".git/config")).expect("read the config");
    let calls = config_calls(&trace);
    assert!(
        before == after && calls.is_empty(),
        "the fetch without the member read or changed the checkout's configuration: {calls:?}"
    );
    assert!(
        !fetches(&trace).is_empty() && !listener.heads().is_empty(),
        "the fetch without the member did not run: {brought:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// The guard
// ---------------------------------------------------------------------------------------------

fn text_of(path: &Path) -> String {
    path.to_str().expect("the case's path is UTF-8").to_owned()
}

/// The guard's places of the instance `alpha` in `dir`, under the `gates` route, with the member
/// when `anonymous`.
fn places(dir: &Path, anonymous: bool) -> Places {
    let member = if anonymous {
        "\x20   forge_access: anonymous\n"
    } else {
        ""
    };
    let text = format!(
        "version: conductor.config/2\n\
         daemon:\n\
         \x20 store: sqlite:/var/lib/example/shared.sqlite\n\
         \x20 secrets: {{source: files, dir: /var/lib/example/secrets}}\n\
         \x20 kubernetes:\n\
         \x20   namespace: conductor-instances\n\
         \x20   image: registry.example.com/conductor:1.0.0\n\
         \x20   owner: example-owner\n\
         \x20   api_port: 7310\n\
         \x20   volume: {{size: 20G}}\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{{github: example-owner, include: [one]}}]\n\
         {member}\
         \x20   forge: {{route: gates}}\n"
    );
    let file = dir.join("conductor.yaml");
    fs::write(&file, &text).expect("write the config file");
    let parsed = config::parse(&text, &dir.join("home")).unwrap_or_else(|p| panic!("{p:?}"));
    let mut instance = parsed.instances[0].clone();
    instance.checkouts.root = text_of(&dir.join("root"));
    instance.checkouts.trees = text_of(&dir.join("trees"));
    instance.records = text_of(&dir.join("records"));
    instance.state = text_of(&dir.join("state"));
    instance.cache = text_of(&dir.join("cache"));
    let active = config::Active {
        instance,
        from_file: true,
        others: Vec::new(),
        default: None,
        daemon_port: config::DAEMON_PORT,
    };
    Places::active(&dir.join("home"), &active).with_config_file(&file)
}

fn guard_case(name: &str, anonymous: bool) -> (Scratch, PathBuf, Places) {
    active::isolate();
    let scratch = Scratch::new(Path::new("faa-review").join(name));
    let dir = fs::canonicalize(scratch.path()).expect("the case's directory");
    for sub in ["home", "root/one", "trees", "records", "state", "cache"] {
        fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
    }
    git(
        &dir.join("root/one"),
        &["init", "--quiet", "--initial-branch=main"],
    );
    git(
        &dir.join("root/one"),
        &[
            "remote",
            "add",
            "origin",
            "https://github.com/example-owner/one.git",
        ],
    );
    let places = places(&dir, anonymous);
    (scratch, dir, places)
}

fn decide(places: &Places, cwd: &Path, command: &str) -> guard::Decision {
    let payload = json!({
        "session_id": "00000000-0000-4000-8000-0000000000c1",
        "cwd": text_of(cwd),
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": command},
    });
    guard::decide_in(&payload, places, &SessionList::default())
}

/// Each command of `commands` whose decision in the checkout is no denial naming the member, with
/// its verdict and reason.
fn not_denied(places: &Places, dir: &Path, commands: &[&str]) -> Vec<String> {
    commands
        .iter()
        .filter_map(|command| {
            let decision = decide(places, &dir.join("root/one"), command);
            (!(decision.verdict == Verdict::Deny && decision.reason.contains(NAMED))).then(|| {
                format!(
                    "{command} | {:?} | {}",
                    decision.verdict,
                    decision.reason.chars().take(160).collect::<String>()
                )
            })
        })
        .collect()
}

/// The spellings the brief and the change name as held, beyond the change's own list.
#[test]
fn push_spellings_after_shells_and_launchers_are_denied_naming_the_member() {
    let (_scratch, dir, places) = guard_case("held", true);
    let commands = [
        "bash -lc 'git push origin main'",
        "sh -c \"git push origin main\"",
        "busybox sh -c 'git push origin main'",
        "eval git push origin main",
        "eval 'git push origin main'",
        "git -C . push origin main",
        "git --git-dir .git push origin main",
        "command git push origin main",
        "sudo git push origin main",
        "nice -n 10 git push origin main",
        "stdbuf -oL git push origin main",
        "setsid git push origin main",
        "git -c Alias.P=push P origin main",
        "gh api -X POST repos/example-owner/one/issues",
        "gh api --method=POST repos/example-owner/one/issues",
        "gh api repos/example-owner/one/issues -f title=x",
        "gh pr create -R example-owner/one --fill",
    ];
    let missed = not_denied(&places, &dir, &commands);
    assert!(missed.is_empty(), "{}", missed.join("\n"));
}

/// Push spellings the guide covers in words (a push behind a launcher with its options,
/// `command`, `exec`, `time`; a pushing alias; a shell's `-c`): a launcher option with a value
/// that is no number, `command -p`, `exec -a`, `time -p`, `git --attr-source <tree>`, `sh -c --`,
/// and an alias given with `--config-env`.
#[test]
fn push_spellings_after_launcher_options_and_keyword_options_are_denied() {
    let (_scratch, dir, places) = guard_case("missed", true);
    let commands = [
        "sudo -u root git push origin main",
        "timeout -s KILL 60 git push origin main",
        "env -u GH_TOKEN git push origin main",
        "env -C . git push origin main",
        "stdbuf -o L git push origin main",
        "echo main | xargs -I % git push origin %",
        "command -p git push origin main",
        "exec -a pusher git push origin main",
        "time -p git push origin main",
        "git --attr-source HEAD push origin main",
        "sh -c -- 'git push origin main'",
        "PUSHING=push git --config-env alias.p=PUSHING p origin main",
        "git --config-env=alias.p=PUSHING p origin main",
    ];
    let missed = not_denied(&places, &dir, &commands);
    assert!(
        missed.is_empty(),
        "{} of {} are no denial naming the member:\n{}",
        missed.len(),
        commands.len(),
        missed.join("\n")
    );
}

/// The guide's section "Anonymous forge access", on one line.
fn guide_section() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../website/docs/guides/run-in-kubernetes.md");
    let text = fs::read_to_string(&path).expect("the Kubernetes guide");
    let start = text
        .find("## Anonymous forge access")
        .expect("the guide has the section");
    let rest = &text[start + 2..];
    let end = rest.find("\n## ").map_or(rest.len(), |end| end + 2);
    text[start..start + end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// A script on a shell's standard input, a launcher outside the guard's list, and `curl` to the
/// forge's API are not read as a push, and the guard does not deny them: the guide says it
/// refuses the spellings it lists only, names each of these, and says that such a write finds no
/// credential in the pod.
#[test]
fn writes_the_guard_cannot_read_are_named_by_the_guide_as_finding_no_credential() {
    let (_scratch, dir, places) = guard_case("unread", true);
    let commands = [
        "echo 'git push origin main' | sh",
        "flock .lock git push origin main",
        "curl -X POST https://api.github.com/repos/example-owner/one/issues -d '{}'",
    ];
    let denied: Vec<&&str> = commands
        .iter()
        .filter(|command| not_denied(&places, &dir, &[**command]).is_empty())
        .collect();
    assert!(
        denied.is_empty(),
        "the guard reads {denied:?} now; name it among the refused spellings of the guide"
    );
    let section = guide_section();
    for words in [
        "refuses these spellings only",
        "a script on a shell's standard input",
        "a launcher outside that list",
        "`curl` to the forge's API",
        "finds no credential in the pod",
        "`--config-env alias.<name>=`",
        "`sudo`",
        "`command`, `exec` or `time`",
    ] {
        assert!(
            section.contains(words),
            "the guide says {words:?}:\n{section}"
        );
    }
}

/// Reads of an anonymous instance stay allowed by the member's rule: no decision names it.
#[test]
fn reads_of_an_anonymous_instance_are_not_refused_as_writes() {
    let (_scratch, dir, places) = guard_case("reads", true);
    let mut named = Vec::new();
    for command in [
        "git remote update",
        "git fetch origin",
        "git fetch --all --prune",
        "git stash push -m example",
        "gh pr view 1 -R example-owner/one",
        "gh api repos/example-owner/one",
        "gh api -X GET repos/example-owner/one/pulls",
        "timeout 60 git fetch origin",
    ] {
        let decision = decide(&places, &dir.join("root/one"), command);
        if decision.reason.contains(NAMED) {
            named.push(format!(
                "{command} | {:?} | {}",
                decision.verdict, decision.reason
            ));
        }
    }
    assert!(named.is_empty(), "{}", named.join("\n"));
}

/// An instance without the member: no decision names it, so its pushes are decided by the
/// rules before the change.
#[test]
fn an_instance_without_the_member_gets_no_anonymous_denial() {
    let (_scratch, dir, places) = guard_case("not-anonymous", false);
    let mut named = Vec::new();
    for command in [
        "git push origin main",
        "timeout 60 git push origin main",
        "env GIT_TRACE=1 git push origin main",
        "git -c alias.p=push p origin main",
        "git lfs push origin main",
        "git subtree push --prefix=docs origin main",
        "b10x-gates bot push origin main",
        "b10x-gates api --method POST --path repos/example-owner/one/issues --output out.json",
        "gh pr view 1 -R example-owner/one",
    ] {
        let decision = decide(&places, &dir.join("root/one"), command);
        eprintln!(
            "without the member: {command} | {:?} | {}",
            decision.verdict,
            decision.reason.chars().take(120).collect::<String>()
        );
        if decision.reason.contains(NAMED) {
            named.push(format!(
                "{command} | {:?} | {}",
                decision.verdict, decision.reason
            ));
        }
    }
    assert!(named.is_empty(), "{}", named.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// Doctor, collectors, validation
// ---------------------------------------------------------------------------------------------

fn parsed_alpha(extra: &str) -> conductor_model::config::Instance {
    active::isolate();
    let text = file(
        &["{name: GH_TOKEN, secret: secret:forge-token}"],
        &instance("alpha", "{github: example-owner, include: [one]}", extra),
    );
    let parsed = config::parse(&text, Path::new("/srv/example"))
        .unwrap_or_else(|problems| panic!("the file does not validate: {problems:?}\n{text}"));
    parsed.instances[0].clone()
}

/// The doctor checks `gh` and the collectors start it for an instance without the member, as
/// before; for the anonymous instance neither does.
#[test]
fn the_doctor_and_the_collectors_still_run_gh_without_the_member() {
    let plain = parsed_alpha("");
    let anonymous = parsed_alpha("\x20   forge_access: anonymous\n");
    let seen = (
        conductor_cli::doctor::forges(&plain),
        conductor_cli::collect::programs(&plain).contains(&"gh"),
        conductor_cli::doctor::forges(&anonymous),
        conductor_cli::collect::programs(&anonymous).contains(&"gh"),
    );
    assert_eq!(seen, (vec!["gh"], true, Vec::<&str>::new(), false));
}

/// A `secret_syncs` entry is refused only beside the member: without it, the file validates.
#[test]
fn secret_syncs_without_the_member_still_validates() {
    let plain = parsed_alpha(
        "\x20   secret_syncs:\n\
         \x20     - name: ci-policy\n\
         \x20       source: {repository: one, path: ci/policy.json}\n\
         \x20       secrets:\n\
         \x20         - {owner: example-owner, name: CI_POLICY}\n\
         \x20       command: example-sync publish ci/policy.json\n",
    );
    assert_eq!(plain.secret_syncs.len(), 1);
}
// ---------------------------------------------------------------------------------------------
// The second review
// ---------------------------------------------------------------------------------------------

/// An instance's `forge_token` written as plain text in the mapping, not `secret:<name>`: refused,
/// or else it reaches no anonymous pod.
#[test]
fn a_forge_token_written_as_plain_text_in_a_mapping_reaches_no_anonymous_pod() {
    active::isolate();
    let text = file(
        &["{name: GH_TOKEN, secret: secret:forge-token}"],
        &[
            anonymous_alpha(),
            instance(
                "beta",
                "{github: example-owner}",
                "\x20   forge_token: example-plain-forge-5151\n",
            ),
        ]
        .concat(),
    );
    match config::parse(&text, Path::new("/srv/example")) {
        Err(problems) => eprintln!("refused: {problems:?}"),
        Ok(_) => {
            let made = made(&text);
            let pod = made.pod.to_string();
            assert!(
                !pod.contains("example-plain-forge-5151")
                    && !made.rendered.contains("example-plain-forge-5151"),
                "the anonymous pod carries beta's plain forge token"
            );
        }
    }
}

/// Another instance's `forge_token` secret named by a key of the anonymous instance's own mapping
/// (`model_token`): the rendered file puts that secret into the anonymous pod's Secret and
/// volume, by a route `ForgeCredentials` does not read. Refused at validation, or left out.
#[test]
fn another_instances_forge_token_named_by_the_anonymous_mapping_reaches_no_pod() {
    active::isolate();
    let text = file(
        &["{name: GH_TOKEN, secret: secret:forge-token}"],
        &[
            instance(
                "alpha",
                "{github: example-owner, include: [one]}",
                "\x20   forge_access: anonymous\n\x20   model_token: secret:beta-forge\n",
            ),
            instance(
                "beta",
                "{github: example-owner}",
                "\x20   forge_token: secret:beta-forge\n",
            ),
        ]
        .concat(),
    );
    match config::parse(&text, Path::new("/srv/example")) {
        Err(problems) => eprintln!("refused: {problems:?}"),
        Ok(_) => {
            let made = made(&text);
            assert!(
                !made.secrets.contains("beta-forge"),
                "the anonymous pod's Secret holds beta's forge token: {:?}",
                made.secrets
            );
        }
    }
}

/// The value comparison of a forge token written as a value: a copy with white space around it,
/// one inside a header's text. (An empty value is refused at validation: `is empty`.) The copies
/// carry the token.
#[test]
fn a_forge_token_value_in_another_spelling_reaches_no_anonymous_pod() {
    let token = "example-forge-value-7777";
    let text = file(
        &[
            &format!("{{name: GH_TOKEN, value: {token}}}"),
            &format!("{{name: EXAMPLE_PADDED, value: \" {token} \"}}"),
            &format!("{{name: EXAMPLE_BEARER, value: \"Bearer {token}\"}}"),
        ],
        &anonymous_alpha(),
    );
    let made = made(&text);
    let carrying: Vec<String> = env_of(&made.pod)
        .iter()
        .filter(|entry| entry["value"].as_str().is_some_and(|v| v.contains(token)))
        .filter_map(|entry| entry["name"].as_str().map(str::to_owned))
        .collect();
    assert!(
        carrying.is_empty(),
        "the anonymous pod carries the token's text as {carrying:?}"
    );
}

/// A kept proxy pair whose URL holds the forge token as its user information: refused at
/// validation, or left out.
#[test]
fn a_proxy_url_holding_the_forge_token_reaches_no_anonymous_pod() {
    active::isolate();
    let token = "example-forge-value-7777";
    let text = file(
        &[
            &format!("{{name: GH_TOKEN, value: {token}}}"),
            "{name: GIT_CONFIG_COUNT, value: \"1\"}",
            "{name: GIT_CONFIG_KEY_0, value: http.proxy}",
            &format!(
                "{{name: GIT_CONFIG_VALUE_0, value: \"http://x-access-token:{token}@proxy.example.com:3128\"}}"
            ),
        ],
        &anonymous_alpha(),
    );
    match config::parse(&text, Path::new("/srv/example")) {
        Err(problems) => eprintln!("refused: {problems:?}"),
        Ok(_) => {
            let made = made(&text);
            assert!(
                !gits(&made.pod).iter().any(|entry| entry.contains(token)),
                "the anonymous pod keeps the proxy with the token: {:?}",
                gits(&made.pod)
            );
        }
    }
}

/// The key of a route that writes to the forge, under a name the daemon does not know: the daemon
/// cannot tell what it holds, so it stays in the anonymous pod, and the guide tells the operator
/// to give no such key an entry.
#[test]
fn a_route_key_under_an_unknown_name_stays_and_the_guide_says_to_give_it_no_entry() {
    let made = made(&file(
        &["{name: EXAMPLE_ROUTE_KEY, secret: secret:route-key}"],
        &anonymous_alpha(),
    ));
    assert_eq!(
        reading(&made.pod, "route-key"),
        ["EXAMPLE_ROUTE_KEY"],
        "an entry under an unknown name is kept"
    );
    let section = guide_section();
    assert!(
        section.contains("nor the key of any route that writes to it"),
        "the guide does not tell the operator to give such a key no entry"
    );
}

/// A checkout with origin on `listener`, its `.git/config` with `extra` appended.
fn checkout_with(dir: &Path, listener: &Listener, extra: &str) -> (PathBuf, String) {
    let checkout = dir.join("root/one");
    fs::create_dir_all(&checkout).expect("checkout");
    let origin = format!("{}example-owner/one.git", listener.url());
    git(&checkout, &["init", "--quiet", "--initial-branch=main"]);
    git(&checkout, &["remote", "add", "origin", &origin]);
    let config = checkout.join(".git").join("config");
    let mut text = fs::read_to_string(&config).expect("the checkout's config");
    text.push_str(extra);
    fs::write(&config, text).expect("write the checkout's config");
    (checkout, origin)
}

fn fetch(dir: &Path, checkout: &Path, origin: &str, anonymous: bool) -> Brought {
    let mut wanted = wanted(origin, Some(checkout.to_owned()));
    wanted.anonymous = anonymous;
    checkouts::bring(&dir.join("root"), &wanted, Some(Duration::from_secs(20)))
}

/// Keys in the spelling git normalises (section and name in any case, the URL's scheme in capitals)
/// and a key of two values: the anonymous fetch sends none, and the checkout's config holds none
/// after it.
#[test]
fn the_anonymous_fetch_clears_headers_in_any_spelling_and_every_value() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r2-spelling");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let upper = format!("HTTP://127.0.0.1:{}/", listener.port);
    let (checkout, origin) = checkout_with(
        &dir,
        &listener,
        &format!(
            "[HTTP \"{upper}\"]\n\tExtraHeader = X-One: spelled-1111\n\tEXTRAHEADER = X-Two: \
             spelled-1112\n[Http]\n\tExtraHeader = X-Three: spelled-1113\n"
        ),
    );
    let brought = fetch(&dir, &checkout, &origin, true);
    let heads = listener.heads();
    let sent: Vec<&str> = ["spelled-1111", "spelled-1112", "spelled-1113"]
        .into_iter()
        .filter(|header| heads.iter().any(|head| head.contains(header)))
        .collect();
    let left = fs::read_to_string(checkout.join(".git/config")).unwrap_or_default();
    assert!(
        !heads.is_empty() && sent.is_empty() && !left.contains("spelled-"),
        "requests {}, sent {sent:?}; {brought:?}\nconfig after:\n{left}",
        heads.len()
    );
}

/// An `include.path` of the checkout's config naming a file with an extra header: `--local`
/// alone lists the including file only, and the fetch follows the include. The header is not
/// sent: the fetch runs without it, or it is not run, one member failure that names the key.
#[test]
fn the_anonymous_fetch_sends_no_header_of_an_included_file() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r2-include");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let included = dir.join("included.conf");
    fs::write(
        &included,
        format!(
            "[http \"{}\"]\n\textraHeader = X-Included: included-2222\n",
            listener.url()
        ),
    )
    .expect("write the included file");
    let (checkout, origin) = checkout_with(
        &dir,
        &listener,
        &format!("[include]\n\tpath = {}\n", included.display()),
    );
    let brought = fetch(&dir, &checkout, &origin, true);
    let heads = listener.heads();
    let key = format!("`http.{}.extraheader`", listener.url());
    let refused = matches!(&brought, Brought::Failed(line)
        if line.contains(&key) && !line.contains("included-2222"));
    assert!(
        (heads.is_empty() && refused) || (!heads.is_empty() && !refused),
        "neither a fetch nor a refusal naming {key}: {} requests; {brought:?}",
        heads.len()
    );
    assert!(
        !heads.iter().any(|head| head.contains("included-2222")),
        "the anonymous fetch sends the included header:\n{}",
        heads.join("---\n")
    );
}

/// `core.askPass` of the checkout's config: git asks it for the login of a Basic challenge
/// whatever `GIT_TERMINAL_PROMPT` says, and `OWN_CREDENTIAL_KEYS` does not name it.
#[test]
fn the_anonymous_fetch_asks_no_askpass_of_the_checkouts_config() {
    use std::os::unix::fs::PermissionsExt;
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r2-askpass");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    reset_environment(&dir.join("home"));
    let askpass = dir.join("askpass");
    fs::write(&askpass, "#!/bin/sh\necho askpass-secret-3333\n").expect("write askpass");
    fs::set_permissions(&askpass, fs::Permissions::from_mode(0o755)).expect("chmod");
    let listener = Listener::start(true);
    let (checkout, origin) = checkout_with(
        &dir,
        &listener,
        &format!("[core]\n\taskPass = {}\n", askpass.display()),
    );
    let brought = fetch(&dir, &checkout, &origin, true);
    let heads = listener.heads();
    assert!(
        !heads.is_empty(),
        "no request reached the listener: {brought:?}"
    );
    assert!(
        !heads
            .iter()
            .any(|head| head.to_ascii_lowercase().contains("\nauthorization:")),
        "the anonymous fetch answers with the askpass login: {brought:?}"
    );
}

/// A checkout whose `.git` the runtime cannot write: the header cannot be removed, and it is not
/// sent.
#[test]
fn a_header_the_runtime_cannot_remove_is_not_sent() {
    use std::os::unix::fs::PermissionsExt;
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r2-read-only");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let (checkout, origin) = checkout_with(
        &dir,
        &listener,
        &format!(
            "[http \"{}\"]\n\textraHeader = X-Stuck: read-only-4141\n",
            listener.url()
        ),
    );
    let git_dir = checkout.join(".git");
    fs::set_permissions(&git_dir, fs::Permissions::from_mode(0o555)).expect("chmod .git");
    let brought = fetch(&dir, &checkout, &origin, true);
    fs::set_permissions(&git_dir, fs::Permissions::from_mode(0o755)).expect("chmod .git back");
    let heads = listener.heads();
    assert!(
        !heads.iter().any(|head| head.contains("read-only-4141")),
        "with .git read-only the anonymous fetch sends the header ({} requests): {brought:?}",
        heads.len()
    );
}

/// A fetch of an instance without the member leaves the checkout's config as it was.
#[test]
fn a_fetch_without_the_member_changes_nothing_in_the_checkouts_config() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r2-fetch-without-member");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let (checkout, origin) = checkout_with(
        &dir,
        &listener,
        &format!(
            "[http \"{}\"]\n\textraHeader = X-Kept: kept-4242\n[url \"{}\"]\n\tinsteadOf = \
             https://forge.example.invalid/\n",
            listener.url(),
            listener.url()
        ),
    );
    let before = fs::read_to_string(checkout.join(".git/config")).expect("config");
    let _ = fetch(&dir, &checkout, &origin, false);
    let after = fs::read_to_string(checkout.join(".git/config")).expect("config");
    assert_eq!(before, after);
    assert!(
        listener
            .heads()
            .iter()
            .any(|head| head.contains("kept-4242")),
        "the fetch without the member sends the checkout's header, as before"
    );
}

/// The git a clone and a fetch run, recorded by a wrapper ahead of the real one on `PATH`: its
/// arguments and the variables the anonymous route sets. Without the member, neither the
/// arguments nor the environment carry anything of the anonymous route, and no `git config
/// --local` runs; with it (the control), they do.
#[test]
fn a_clone_and_a_fetch_without_the_member_run_git_as_before() {
    use std::os::unix::fs::PermissionsExt;
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r2-wrapper");
    let dir = scratch.path().to_owned();
    for sub in ["home", "bin", "plain", "anon"] {
        fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
    }
    reset_environment(&dir.join("home"));
    let real = which("git");
    let log = dir.join("git.log");
    fs::write(
        dir.join("bin/git"),
        format!(
            "#!/bin/sh\n{{ printf 'args:'; for a in \"$@\"; do printf ' [%s]' \"$a\"; done; \
             printf '\\n'; env | grep -E \
             '^(NETRC|GIT_TEMPLATE_DIR|GIT_CONFIG_GLOBAL|GIT_CONFIG_SYSTEM|GIT_CONFIG_NOSYSTEM|GH_TOKEN)=' \
             | sed 's/^/env: /'; }} >> '{log}'\nexec '{real}' \"$@\"\n",
            log = log.display(),
            real = real.display()
        ),
    )
    .expect("write the wrapper");
    fs::set_permissions(dir.join("bin/git"), fs::Permissions::from_mode(0o755)).expect("chmod");
    let listener = Listener::start(false);
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut paths = vec![dir.join("bin")];
    paths.extend(std::env::split_paths(&path));
    let run = |base: &Path, anonymous: bool| -> String {
        let _ = fs::remove_file(&log);
        let (checkout, origin) = checkout_with(base, &listener, "");
        set("PATH", std::env::join_paths(&paths).expect("PATH"));
        set("GH_TOKEN", "example-daemon-token-0000");
        let mut clone = wanted(&format!("{}example-owner/two.git", listener.url()), None);
        clone.anonymous = anonymous;
        clone.path = "two".to_owned();
        let _ = checkouts::bring(&base.join("root"), &clone, None);
        let _ = fetch(base, &checkout, &origin, anonymous);
        set("PATH", &path);
        // SAFETY: as in [`reset_environment`].
        unsafe { std::env::remove_var("GH_TOKEN") };
        fs::read_to_string(&log).unwrap_or_default()
    };
    // Only the records of the clone, the fetch and the runtime's `config --local`: a case of this
    // binary that runs at the same time may start git through the wrapper too.
    let records = |log: String| -> String {
        log.split("args:")
            .filter(|record| {
                ["[clone]", "[fetch]", "[--local]"]
                    .iter()
                    .any(|mark| record.lines().next().unwrap_or_default().contains(mark))
            })
            .collect::<Vec<_>>()
            .join("args:")
    };
    let plain = records(run(&dir.join("plain"), false));
    let anon = records(run(&dir.join("anon"), true));
    let anon_network: String = anon
        .split("args:")
        .filter(|record| !record.contains("[--local]"))
        .collect();
    let marks = [
        "[credential.helper=]",
        "[http.extraHeader=]",
        "env: NETRC=",
        "env: GIT_TEMPLATE_DIR=",
        "env: GIT_CONFIG_GLOBAL=",
        "env: GIT_CONFIG_NOSYSTEM=",
        "[--local]",
    ];
    let in_plain: Vec<&str> = marks
        .iter()
        .copied()
        .filter(|m| plain.contains(m))
        .collect();
    let in_anon: Vec<&str> = marks.iter().copied().filter(|m| anon.contains(m)).collect();
    assert!(
        in_plain.is_empty()
            && plain.contains("[clone]")
            && plain.contains("[fetch]")
            && plain.contains("env: GH_TOKEN=")
            && in_anon.len() == marks.len()
            && !anon_network.contains("env: GH_TOKEN="),
        "without the member: {in_plain:?}\n{plain}\nwith it: {in_anon:?}\n{anon}"
    );
}

/// Reads behind the launcher options and keywords the change reads now stay allowed by the
/// member's rule.
#[test]
fn the_new_launcher_and_keyword_reading_denies_no_read() {
    let (_scratch, dir, places) = guard_case("r2-reads", true);
    let mut named = Vec::new();
    for command in [
        "env -u GH_TOKEN git fetch origin",
        "timeout -s KILL 60 git status",
        "sudo -u root gh pr view 1 -R example-owner/one",
        "command -v git",
        "command -p git log --oneline",
        "time -p cargo test",
        "echo main | xargs -I {} git log {}",
        "exec -a puller git pull origin main",
        "sh -c -- 'git log --oneline'",
        "git --attr-source HEAD log",
    ] {
        let decision = decide(&places, &dir.join("root/one"), command);
        if decision.reason.contains(NAMED) {
            named.push(format!(
                "{command} | {:?} | {}",
                decision.verdict, decision.reason
            ));
        }
    }
    assert!(named.is_empty(), "{}", named.join("\n"));
}

/// An alias given with `--config-env alias.<name>=` takes its text from a variable the guard does
/// not read: it is refused whatever it runs, a read included, and the guide says so.
#[test]
fn an_alias_given_by_config_env_is_refused_even_for_a_read() {
    let (_scratch, dir, places) = guard_case("r2-config-env-read", true);
    let commands = [
        "PAGER=cat git --config-env alias.l=PAGER l",
        "PAGER=cat git --config-env=alias.l=PAGER l",
    ];
    let missed = not_denied(&places, &dir, &commands);
    assert!(
        missed.is_empty(),
        "an alias given by --config-env is not refused:\n{}",
        missed.join("\n")
    );
    let section = guide_section();
    assert!(
        section.contains("`--config-env alias.<name>=`") && section.contains("a read included"),
        "the guide does not say an alias given by --config-env is refused, a read included"
    );
}

/// Push spellings within the guide's list ("after `env`, `sudo` … with their own options, after
/// `command`, `exec`") that the new reading still passes: a cluster of options whose last takes a
/// value, and `--` after `command` or `exec`.
#[test]
fn push_spellings_within_the_guides_list_that_pass() {
    let (_scratch, dir, places) = guard_case("r2-missed", true);
    let commands = [
        "sudo -Eu root git push origin main",
        "env -iu GH_TOKEN git push origin main",
        "command -- git push origin main",
        "exec -- git push origin main",
        "exec -a pusher -- git push origin main",
        "timeout --signal KILL 60 git push origin main",
    ];
    let missed = not_denied(&places, &dir, &commands);
    assert!(
        missed.is_empty(),
        "{} of {} are no denial naming the member:\n{}",
        missed.len(),
        commands.len(),
        missed.join("\n")
    );
}

/// An instance without the member under every spelling the change reads now: no decision names
/// the member. Printed: each verdict.
#[test]
fn an_instance_without_the_member_gets_no_anonymous_denial_for_new_spellings() {
    let (_scratch, dir, places) = guard_case("r2-not-anonymous", false);
    let mut named = Vec::new();
    for command in [
        "sudo -u root git push origin main",
        "timeout -s KILL 60 git push origin main",
        "env -u GH_TOKEN git push origin main",
        "env -C . git push origin main",
        "stdbuf -o L git push origin main",
        "echo main | xargs -I % git push origin %",
        "command -p git push origin main",
        "exec -a pusher git push origin main",
        "time -p git push origin main",
        "git --attr-source HEAD push origin main",
        "sh -c -- 'git push origin main'",
        "PUSHING=push git --config-env alias.p=PUSHING p origin main",
        "command -p gh pr create -R example-owner/one --fill",
        "time -p gh pr view 1 -R example-owner/one",
    ] {
        let decision = decide(&places, &dir.join("root/one"), command);
        eprintln!(
            "without the member: {command} | {:?} | {}",
            decision.verdict,
            decision.reason.chars().take(100).collect::<String>()
        );
        if decision.reason.contains(NAMED) {
            named.push(format!(
                "{command} | {:?} | {}",
                decision.verdict, decision.reason
            ));
        }
    }
    assert!(named.is_empty(), "{}", named.join("\n"));
}

// ---------------------------------------------------------------------------------------------
// The third review
// ---------------------------------------------------------------------------------------------

/// The blockers collector fetches each workspace member's checkout with its own `git fetch
/// --quiet origin` (`blockers::Sources::of`), and after an anonymous clone the workspace file
/// names the anonymous checkouts (`checkouts::refresh`). Driven as the snapshot drives it, over an
/// anonymous instance whose checkout's config names an extra header and whose home holds a
/// `.netrc`: no header and no login reach the forge from that fetch either.
#[test]
fn the_blockers_collectors_fetch_of_an_anonymous_checkout_sends_no_credential() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r3-blockers");
    let dir = fs::canonicalize(scratch.path()).expect("the case's directory");
    for sub in ["home", "root", "records", "state", "cache"] {
        fs::create_dir_all(dir.join(sub)).expect("create the case's directories");
    }
    reset_environment(&dir.join("home"));
    let listener = Listener::start(true);
    fs::write(
        dir.join("home/.netrc"),
        "machine 127.0.0.1 login example password blockers-netrc-5454\n",
    )
    .expect("write .netrc");
    let (_checkout, _origin) = checkout_with(
        &dir,
        &listener,
        &format!(
            "[http \"{}\"]\n\textraHeader = X-Blockers: blockers-5353\n",
            listener.url()
        ),
    );
    git(
        &dir.join("records"),
        &["init", "--quiet", "--initial-branch=main"],
    );
    let parsed = config::parse(
        &file(
            &["{name: GH_TOKEN, secret: secret:forge-token}"],
            &anonymous_alpha(),
        ),
        &dir.join("home"),
    )
    .unwrap_or_else(|p| panic!("{p:?}"));
    let mut instance = parsed.instances[0].clone();
    instance.checkouts.root = text_of(&dir.join("root"));
    instance.records = text_of(&dir.join("records"));
    instance.state = text_of(&dir.join("state"));
    instance.cache = text_of(&dir.join("cache"));
    let members = config::workspace_members(&instance, 0).expect("the workspace members");
    let workspace = dir
        .join("records")
        .join(conductor_cli::collect::blockers::WORKSPACE);
    fs::create_dir_all(workspace.parent().expect("parent")).expect("create .engineering");
    fs::write(&workspace, config::workspace_text(&members)).expect("write the workspace file");
    let active = config::Active {
        instance,
        from_file: true,
        others: Vec::new(),
        default: None,
        daemon_port: config::DAEMON_PORT,
    };
    let sources = conductor_cli::collect::blockers::Sources::of(&active, &dir.join("state"))
        .expect("the blockers sources");
    eprintln!("the blockers collector fetches with {:?}", sources.fetch);
    let store = conductor_cli::store::open(&dir.join("state")).expect("open the store");
    let taken = conductor_cli::snapshot::take(
        store,
        &[conductor_cli::collect::Collector::new(
            "blockers",
            move |record| conductor_cli::collect::blockers::collect_from(&sources, record),
        )],
    )
    .expect("the snapshot ends");
    let heads = listener.heads();
    eprintln!(
        "snapshot {:?} {:?}; {} requests reached the forge's stand-in",
        taken.state,
        taken.reason,
        heads.len()
    );
    let header = heads.iter().any(|head| head.contains("blockers-5353"));
    let login = heads
        .iter()
        .any(|head| head.to_ascii_lowercase().contains("\nauthorization:"));
    assert!(
        !header && !login,
        "the blockers collector's fetch of the anonymous checkout sends the header: {header}, a \
         netrc login: {login}\n{}",
        heads.join("---\n")
    );
}

/// The member failure of a refused fetch names the key that stays, never its value; a key's own
/// name may hold a credential as a URL's user information, which is taken out too.
#[test]
fn a_refused_fetch_names_no_credential_held_in_a_key_name() {
    let _lock = environment();
    active::isolate();
    let scratch = Scratch::new("faa-review/r3-redaction");
    let dir = scratch.path().to_owned();
    fs::create_dir_all(dir.join("home")).expect("home");
    reset_environment(&dir.join("home"));
    let listener = Listener::start(false);
    let included = dir.join("included.conf");
    fs::write(
        &included,
        format!(
            "[http \"http://example:name-secret-7171@127.0.0.1:{port}/\"]\n\textraHeader = X: \
             value-secret-7070\n[url \"http://tok-7272@127.0.0.1:{port}/\"]\n\tinsteadOf = \
             https://forge.example.invalid/\n",
            port = listener.port
        ),
    )
    .expect("write the included file");
    let (checkout, origin) = checkout_with(
        &dir,
        &listener,
        &format!("[include]\n\tpath = {}\n", included.display()),
    );
    let brought = fetch(&dir, &checkout, &origin, true);
    let Brought::Failed(line) = &brought else {
        panic!("the fetch was not refused: {brought:?}");
    };
    let shown: Vec<&str> = ["name-secret-7171", "tok-7272", "value-secret-7070"]
        .into_iter()
        .filter(|secret| line.contains(secret))
        .collect();
    assert!(
        shown.is_empty() && listener.heads().is_empty(),
        "the refusal shows {shown:?}: {line}"
    );
}

/// A forge token value that is a short word other entries hold: the anonymous pod leaves out a
/// proxy that holds it, which carries no credential; a pod without the member keeps it.
#[test]
fn a_short_forge_token_value_removes_no_unrelated_entry() {
    let text = file(
        &[
            "{name: GH_TOKEN, value: example}",
            "{name: HTTPS_PROXY, value: \"http://proxy.example.com:3128\"}",
            "{name: GIT_CONFIG_COUNT, value: \"1\"}",
            "{name: GIT_CONFIG_KEY_0, value: http.proxy}",
            "{name: GIT_CONFIG_VALUE_0, value: \"http://proxy.example.com:3128\"}",
        ],
        &[
            anonymous_alpha(),
            instance("beta", "{github: example-owner}", ""),
        ]
        .concat(),
    );
    let alpha = made(&text);
    active::isolate();
    let parsed = config::parse(&text, Path::new("/srv/example")).expect("validates");
    let kubernetes = parsed.daemon.kubernetes.clone().expect("daemon.kubernetes");
    let beta = pods::manifests(&kubernetes, &parsed, &parsed.instances[1]);
    let has = |pod: &Value, name: &str| env_of(pod).iter().any(|entry| entry["name"] == name);
    let seen = (
        has(&alpha.pod, "HTTPS_PROXY"),
        gits(&alpha.pod),
        has(&beta.pod, "HTTPS_PROXY"),
    );
    assert!(
        seen.0 && !seen.1.is_empty() && seen.2,
        "anonymous keeps HTTPS_PROXY: {}, its git settings: {:?}; without the member: {}",
        seen.0,
        seen.1,
        seen.2
    );
}

/// Another instance's `forge_token` secret named deeper in the anonymous mapping (a role's key)
/// or in `globals` beside it: refused at validation by the new rule, or else not in the pod.
#[test]
fn another_instances_forge_token_nested_in_the_mapping_or_globals_is_refused() {
    active::isolate();
    let beta = instance(
        "beta",
        "{github: example-owner}",
        "\x20   forge_token: secret:beta-forge\n",
    );
    let nested = file(
        &["{name: GH_TOKEN, secret: secret:forge-token}"],
        &[
            instance(
                "alpha",
                "{github: example-owner, include: [one]}",
                "\x20   forge_access: anonymous\n\
                 \x20   roles:\n\
                 \x20     - {role: conductor, profile: \"secret:beta-forge\"}\n",
            ),
            beta.clone(),
        ]
        .concat(),
    );
    let globals = file(
        &["{name: GH_TOKEN, secret: secret:forge-token}"],
        &[anonymous_alpha(), beta].concat(),
    )
    .replace(
        "instances:\n",
        "globals:\n  roles:\n    - {role: conductor, profile: \"secret:beta-forge\"}\ninstances:\n",
    );
    let mut wrong = Vec::new();
    for (case, text) in [("nested", nested), ("globals", globals)] {
        match config::parse(&text, Path::new("/srv/example")) {
            Err(problems) => {
                eprintln!("{case}: {problems:?}");
                if !problems
                    .iter()
                    .any(|problem| problem.message.contains("forge_token"))
                {
                    wrong.push(format!("{case}: refused, but not for the forge token"));
                }
            }
            Ok(_) => {
                let made = made(&text);
                if made.secrets.contains("beta-forge")
                    || made.rendered.contains("secret:beta-forge")
                {
                    wrong.push(format!("{case}: validates and the pod gets beta-forge"));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// The reads the brief names, and reads with `--` and option clusters: none is refused as a
/// write; the push spellings with `--` and clusters are.
#[test]
fn dashes_and_clusters_deny_pushes_and_no_reads() {
    let (_scratch, dir, places) = guard_case("r3-dashes", true);
    let mut wrong = Vec::new();
    for command in [
        "env -u X git fetch origin",
        "timeout -s KILL 60 git status",
        "sudo -u root gh pr view 1 -R example-owner/one",
        "command -v git",
        "time -p cargo test",
        "echo main | xargs -I {} git log {}",
        "exec -a x git pull origin main",
        "env -- git status",
        "sudo -Eu root git log",
        "grep -- push README.md",
        "git log -- push",
        "nice -- git log --oneline",
        "command -- git fetch origin",
        "env -- EXAMPLE=1 git status",
        "timeout -- 60 git log --oneline",
    ] {
        let decision = decide(&places, &dir.join("root/one"), command);
        if decision.reason.contains(NAMED) {
            wrong.push(format!("read denied: {command} | {}", decision.reason));
        }
    }
    let pushes = [
        "command -- git push origin main",
        "exec -- git push origin main",
        "exec -a x -- git push origin main",
        "sudo -Eu root git push origin main",
        "env -iu GH_TOKEN git push origin main",
        "env -u GH_TOKEN -- git push origin main",
        "env -- EXAMPLE=1 git push origin main",
        "sudo -- EXAMPLE=1 git push origin main",
        "timeout -- 60 git push origin main",
    ];
    wrong.extend(
        not_denied(&places, &dir, &pushes)
            .into_iter()
            .map(|line| format!("push allowed: {line}")),
    );
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

/// An instance without the member under the spellings this head reads: no decision names the
/// member.
#[test]
fn an_instance_without_the_member_gets_no_denial_for_dashes_and_clusters() {
    let (_scratch, dir, places) = guard_case("r3-not-anonymous", false);
    let mut named = Vec::new();
    for command in [
        "command -- git push origin main",
        "exec -- git push origin main",
        "exec -a x -- git push origin main",
        "sudo -Eu root git push origin main",
        "env -iu GH_TOKEN git push origin main",
        "env -- git status",
        "PAGER=cat git --config-env alias.l=PAGER l",
    ] {
        let decision = decide(&places, &dir.join("root/one"), command);
        eprintln!(
            "without the member: {command} | {:?} | {}",
            decision.verdict,
            decision.reason.chars().take(100).collect::<String>()
        );
        if decision.reason.contains(NAMED) {
            named.push(format!(
                "{command} | {:?} | {}",
                decision.verdict, decision.reason
            ));
        }
    }
    assert!(named.is_empty(), "{}", named.join("\n"));
}
