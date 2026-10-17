//! What the broker's tests share: a case directory that removes itself, a session registered with
//! a grant, and a broker serving that directory with a recorder the case reads back.
//!
//! The case directory lies under the system's temporary directory, not under `target/tmp`: a Unix
//! socket's path holds at most 107 bytes, and the tree's `target/tmp` alone can take most of them.
//! Nothing here reads the home directory or the operator's config.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use conductor_broker::{
    Answer, Broker, MemoryRecorder, Placement, Registration, Request, Services, SessionGrant,
    broker_dir, call, register,
};
use conductor_model::tool::Verb;

/// The instance every case's broker serves.
pub const INSTANCE: &str = "example-org";

/// A directory of one case: the instance's state and the sessions' working directories. Removed
/// when the case ends.
pub struct Case {
    pub root: PathBuf,
    pub broker: Option<Broker>,
    pub recorder: Arc<MemoryRecorder>,
}

impl Case {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "cb-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("state")).expect("create the case's state");
        Self {
            root,
            broker: None,
            recorder: Arc::new(MemoryRecorder::default()),
        }
    }

    pub fn state(&self) -> PathBuf {
        self.root.join("state")
    }

    pub fn dir(&self) -> PathBuf {
        broker_dir(&self.state())
    }

    /// A working directory named `name` holding `files`, each a relative path and its text.
    pub fn workdir(&self, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = self.root.join(name);
        for (path, text) in files {
            let path = dir.join(path);
            fs::create_dir_all(path.parent().expect("a file has a parent"))
                .expect("create a fixture directory");
            fs::write(&path, text).expect("write a fixture file");
        }
        fs::create_dir_all(&dir).expect("create the working directory");
        dir
    }

    /// Registers `session` of `instance` in `workdir` with `grant`, and answers its socket.
    pub fn register(
        &self,
        session: &str,
        instance: &str,
        workdir: &Path,
        grant: SessionGrant,
    ) -> PathBuf {
        self.register_placed(session, instance, workdir, grant, Placement::default())
    }

    /// [`Case::register`], at `place`.
    pub fn register_placed(
        &self,
        session: &str,
        instance: &str,
        workdir: &Path,
        grant: SessionGrant,
        place: Placement,
    ) -> PathBuf {
        register(
            &self.dir(),
            &Registration {
                session: session.to_owned(),
                instance: instance.to_owned(),
                workdir: workdir.to_owned(),
                grant,
                started_at: "2026-10-17T00:00:00Z".to_owned(),
                place,
            },
        )
        .expect("register the session")
    }

    /// Starts the instance's broker over the case's state.
    pub fn serve(&mut self) {
        self.serve_with(Services::none());
    }

    /// Starts the instance's broker over the case's state, reaching `services`.
    pub fn serve_with(&mut self, services: Services) {
        let recorder: Arc<dyn conductor_broker::Recorder> = self.recorder.clone();
        self.broker = Some(
            Broker::start_with(&self.dir(), INSTANCE, recorder, services)
                .expect("start the broker"),
        );
    }
}

impl Drop for Case {
    fn drop(&mut self) {
        if let Some(broker) = self.broker.take() {
            broker.shutdown();
        }
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// A grant of `tools` over the whole working directory, 65536 bytes per answer, for the
/// session's life.
pub fn grant(tools: &[Verb]) -> SessionGrant {
    SessionGrant {
        tools: tools.to_vec(),
        repositories: Vec::new(),
        paths: Vec::new(),
        max_output_bytes: 65_536,
        expires_at: None,
    }
}

/// Every verb of the read group.
pub fn reader() -> SessionGrant {
    grant(&[Verb::Show, Verb::Search, Verb::Diff])
}

/// One call of `verb` with `args` on `socket`.
pub fn ask(socket: &Path, verb: &str, args: serde_json::Value) -> Answer {
    call(
        socket,
        &Request {
            verb: verb.to_owned(),
            args,
        },
    )
}

/// The lines of an answer's text.
pub fn lines(answer: &Answer) -> Vec<&str> {
    answer.text.lines().collect()
}
