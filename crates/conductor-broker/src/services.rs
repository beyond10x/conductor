//! What the build, commit and team verbs need that the broker crate does not hold
//! (`story:tool-build-commit-report`): the host ledger, the bot route with the instance's Gates
//! policy, and the store the messages are recorded in. `conductor broker serve` gives the
//! instance's own; a case gives fakes.

use std::path::PathBuf;
use std::sync::Arc;

use crate::ReportStatus;

/// One build the broker asks the host ledger to run: `command` in `cwd`, for `session` of
/// `instance`, its standard output and error written to `log`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildJob {
    pub session: String,
    pub instance: String,
    pub command: Vec<String>,
    pub cwd: PathBuf,
    pub log: PathBuf,
}

/// How a build ended: it ran, with the command's exit status; or the ledger refused it, and the
/// command never ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildEnd {
    Ran { code: i32 },
    Refused { reason: String },
}

/// The host ledger as the build verbs use it: a reservation for the command, a wait in the queue
/// for its turn, the run at nice 19, and the release. `conductor broker serve` runs it through
/// `conductor build run`'s own path, with `--reserve auto`.
pub trait Ledger: Send + Sync {
    /// Runs `job` under the ledger and answers how it ended. Blocks while the job waits.
    fn run(&self, job: &BuildJob) -> BuildEnd;
}

/// The bot route `commit` commits through: `<program> bot --repo <workdir> -- commit …`, with
/// `B10X_GATES_POLICY` naming `policy`. None: the broker holds no policy, and every commit is
/// refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotRoute {
    pub program: PathBuf,
    pub policy: Option<PathBuf>,
}

/// A message between two sessions of one team, as `send` records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMessage {
    pub sender: String,
    pub recipient: String,
    pub team: String,
    pub text: String,
    /// When the broker took it, RFC 3339, UTC.
    pub at: String,
}

/// A report one level up, as `report` records it. `lead` is whether the sender is a team's lead,
/// whose report moves its team's open dispatch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    pub sender: String,
    pub recipient: String,
    pub team: Option<String>,
    pub lead: bool,
    pub status: ReportStatus,
    pub line: String,
    /// When the broker took it, RFC 3339, UTC.
    pub at: String,
}

/// What recording a report did: the message's id, and the dispatch it moved, if any, or why it
/// moved none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    pub message: String,
    pub dispatch: Option<String>,
    pub note: Option<String>,
}

/// The store `send` and `report` record in.
pub trait Messages: Send + Sync {
    /// Records `message`; answers its id.
    ///
    /// # Errors
    ///
    /// What kept the store from recording it, on one line.
    fn send(&self, message: &TeamMessage) -> Result<String, String>;

    /// Records `report`, moving the sender's open dispatch when it is a lead's; answers what it
    /// did.
    ///
    /// # Errors
    ///
    /// What kept the store from recording it or moving the dispatch, on one line.
    fn report(&self, report: &Report) -> Result<Reported, String>;
}

/// Everything the broker's verbs reach beyond its own directory.
#[derive(Clone)]
pub struct Services {
    pub ledger: Arc<dyn Ledger>,
    pub bot: BotRoute,
    pub messages: Arc<dyn Messages>,
}

impl Services {
    /// No ledger, no policy and no store: every build, commit, send and report fails, naming what
    /// is missing. The read verbs need none of them.
    #[must_use]
    pub fn none() -> Self {
        Self {
            ledger: Arc::new(Missing),
            bot: BotRoute {
                program: PathBuf::from("b10x-gates"),
                policy: None,
            },
            messages: Arc::new(Missing),
        }
    }
}

/// The ledger and store of a broker started without them.
struct Missing;

impl Ledger for Missing {
    fn run(&self, _job: &BuildJob) -> BuildEnd {
        BuildEnd::Refused {
            reason: "this broker runs no host ledger".to_owned(),
        }
    }
}

impl Messages for Missing {
    fn send(&self, _message: &TeamMessage) -> Result<String, String> {
        Err("this broker records in no store".to_owned())
    }

    fn report(&self, _report: &Report) -> Result<Reported, String> {
        Err("this broker records in no store".to_owned())
    }
}
