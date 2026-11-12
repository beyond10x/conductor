//! Conformance checks of `GET /v1/instances/{name}/secrets` (`story:instance-secret-state`)
//! against what a caller with the control token writes into a mapping: the rows list nothing of
//! the secrets the daemon's own `daemon.kubernetes.env` entries name.
//!
//! Each case calls the library's `secret::instance_view` over a config file the case writes and a
//! `files` source in the case's directory, with the names the daemon withholds computed from the
//! file's `daemon` section as `daemon serve` computes them.

mod active;

use std::fs;
use std::path::PathBuf;

use conductor_cli::secret;
use conductor_model::config::SecretsSource;
use conductor_model::secret::Source;

struct Case {
    dir: PathBuf,
}

impl Drop for Case {
    fn drop(&mut self) {
        if !std::thread::panicking() {
            let _ = fs::remove_dir_all(&self.dir);
        }
    }
}

impl Case {
    fn new(name: &str) -> Self {
        active::isolate();
        let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
            .join("sec-instance-secrets")
            .join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("home")).expect("create the case's home");
        fs::create_dir_all(dir.join("secrets")).expect("create the secrets directory");
        Self { dir }
    }

    fn secrets(&self) -> PathBuf {
        self.dir.join("secrets")
    }
}

/// Claim E: a caller that names, in its own instance's mapping, the secret of a
/// `daemon.kubernetes.env` entry learns nothing of it through the instance's secrets: either the
/// file does not validate, or no row names that secret.
#[test]
fn a_mapping_learns_nothing_of_a_secret_a_kubernetes_env_entry_names() {
    let case = Case::new("env-entry");
    fs::write(case.secrets().join("control"), "control-value-of-the-case").expect("write");
    fs::write(
        case.secrets().join("operator-extra"),
        "operator-extra-value",
    )
    .expect("write");
    let text = format!(
        "version: conductor.config/2\n\
         daemon: {{port: 7300, address: \"0.0.0.0\", store: {store:?}, \
         control_token: \"secret:control\", secrets: {{source: files, dir: {secrets:?}}}, \
         kubernetes: {{namespace: conductor-instances, \
         image: \"registry.example.com/conductor:1.0.0\", owner: example-org, \
         api_port: 7310, volume: {{size: 20G}}, \
         env: [{{name: EXAMPLE_SERVICE_KEY, secret: secret:operator-extra}}]}}}}\n\
         instances:\n\
         \x20 - name: alpha\n\
         \x20   placement: none\n\
         \x20   session_prefix: alpha\n\
         \x20   sources: [{{github: example-org}}]\n\
         \x20   model_token: secret:operator-extra\n",
        store = format!("sqlite:{}", case.dir.join("daemon.sqlite").display()),
        secrets = case.secrets(),
    );
    let config = match conductor_cli::config::parse(&text, &case.dir.join("home")) {
        Ok(config) => config,
        // Validation refuses an instance that names it: the claim holds for this file.
        Err(_) => return,
    };
    let withheld = secret::withheld(&config.daemon);
    let source = SecretsSource {
        source: Source::Files,
        namespace: None,
        url: None,
        dir: Some(case.secrets().display().to_string()),
    };
    let rows = secret::instance_view(&source, &text, "alpha", &withheld)
        .unwrap_or_else(|_| panic!("the files source did not answer"))
        .expect("alpha is an instance of the file");
    let named: Vec<String> = rows
        .iter()
        .filter(|row| row.secret == "operator-extra")
        .map(|row| {
            format!(
                "{} set {} modified_at {:?}",
                row.key, row.set, row.modified_at
            )
        })
        .collect();
    assert!(
        named.is_empty(),
        "the rows tell a mapping of the operator's env entry's secret: {named:?}"
    );
}
