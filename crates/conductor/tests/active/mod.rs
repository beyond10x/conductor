//! The one way a test that calls the library fixes the instance its process runs
//! (`config::active`): [`isolate`]. `tests/isolated_home.rs` fails on a test file that names a
//! module reading that instance and fixes it neither through [`isolate`] nor through a config
//! file of its own.

use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

use conductor_cli::config;

/// This test binary's scratch directory for the instance: its config file, and the records,
/// state and cache directories the file names.
fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("isolated-active")
        .join(env!("CARGO_CRATE_NAME"))
}

/// Fixes the instance this test binary's process runs to a scratch config file of one instance
/// without a prefix, owner `example-org`, before anything reads it, so a config file in the real
/// home never decides a case. Its checkouts lie under a `home/` of the scratch directory that does
/// not exist, as the built-in instance's do under an empty home; its records, state and cache are
/// directories of the scratch directory, so nothing the library resolves from the instance lies
/// under the real home. The instance takes the name `CONDUCTOR_INSTANCE` gives, if any, so the
/// variable selects it.
///
/// Every case that reaches a reader calls it first: the process's instance is fixed by its first
/// reader. A case that runs after another read it without isolating fails here instead of reading
/// the real home's config.
pub fn isolate() {
    static ISOLATED: OnceLock<()> = OnceLock::new();
    let dir = dir();
    ISOLATED.get_or_init(|| {
        fs::create_dir_all(&dir).expect("create the scratch instance's directory");
        let name = std::env::var(config::INSTANCE_VARIABLE)
            .unwrap_or_else(|_| config::ORGANIZATION.to_owned());
        let home = dir.join("home");
        let file = dir.join("conductor.yaml");
        // Written beside and renamed into place: another process of this test binary (a runner
        // that starts one per case, a case that runs itself again) may be reading it.
        let written = dir.join(format!("conductor.yaml.{}", std::process::id()));
        fs::write(
            &written,
            format!(
                "version: conductor.config/1\n\
                 instances:\n\
                 \x20 - name: {name}\n\
                 \x20   sources: [{{github: {owner}}}]\n\
                 \x20   checkouts: {{root: {root:?}, trees: {trees:?}}}\n\
                 \x20   records: {records:?}\n\
                 \x20   state: {state:?}\n\
                 \x20   cache: {cache:?}\n",
                owner = config::ORGANIZATION,
                root = home.join(config::ORGANIZATION),
                trees = home
                    .join(".local/state/worktree/trees")
                    .join(config::ORGANIZATION),
                records = dir.join("records"),
                state = dir.join("state"),
                cache = dir.join("cache"),
            ),
        )
        .expect("write the scratch config");
        fs::rename(&written, &file).expect("put the scratch config in place");
        config::set_active(Some(&file)).expect("the scratch config loads");
    });
    assert_eq!(
        PathBuf::from(&config::active().instance.state),
        dir.join("state"),
        "the instance of this process was fixed before `active::isolate` ran: a case read it \
         without isolating it first"
    );
}
