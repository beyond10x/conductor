//! story:anonymous-pod-no-forge-secret, second round: conformance cases for the
//! contract of `forge_access: anonymous` that the change's own suites do not state.
//!
//! - The pod (`pods::manifests`): a forge token the file names that is not the daemon's own, a
//!   forge token written as a value, git's environment configuration in other spellings and
//!   counts, and variables that point git or curl at a credential or a configuration file.
//! - The anonymous clone and fetch (`checkouts::bring`) against a local HTTP listener: an extra
//!   header of the checkout's own configuration, a template's configuration, a netrc file, and
//!   the proxy of the pod's environment.
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

use conductor_cli::collect::checkouts::{self, Wanted};
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
