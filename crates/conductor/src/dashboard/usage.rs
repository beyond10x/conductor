//! The token spend per repository (`story:token-spend`): the repository table `conductor resource
//! usage` prints, counted from 00:00Z of the day it is measured on. It is measured on a thread of
//! its own on the disk measurement's cadence, at most every [`Sources::measure_every`], and each
//! request shows the last complete measurement with its age. Like the command, it reads the
//! transcripts under [`Sources::home`] and writes nothing.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use serde_json::{Value, json};
use time::{OffsetDateTime, Time};

use super::{Sources, ago, rfc3339};
use crate::usage::{self, Usage};

/// The last complete measurement, and whether the next one runs; shared by every request.
#[derive(Debug, Clone, Default)]
pub(super) struct Cache(Arc<Mutex<State>>);

#[derive(Debug, Default)]
struct State {
    last: Option<Measurement>,
    running: bool,
}

#[derive(Debug)]
struct Measurement {
    at: OffsetDateTime,
    taken: Instant,
    usage: Usage,
}

impl Cache {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Starts a measurement of the transcripts under `sources.home` on a thread of its own,
    /// unless one runs or the last is younger than [`Sources::measure_every`]. Returns at once.
    pub(super) fn refresh(&self, sources: &Sources) {
        let mut state = self.lock();
        let fresh = state
            .last
            .as_ref()
            .is_some_and(|last| last.taken.elapsed() < sources.measure_every);
        if state.running || fresh {
            return;
        }
        state.running = true;
        drop(state);
        let cache = self.clone();
        let usage_sources = usage::Sources {
            projects: sources.home.join(".claude/projects"),
            home: sources.home.clone(),
            root: sources.checkouts.clone(),
            trees: sources.trees.clone(),
        };
        std::thread::spawn(move || {
            let since = OffsetDateTime::now_utc().replace_time(Time::MIDNIGHT);
            let measured = catch_unwind(AssertUnwindSafe(|| {
                usage::measure(&usage_sources, Some(since))
            }));
            let mut state = cache.lock();
            state.running = false;
            if let Ok(usage) = measured {
                state.last = Some(Measurement {
                    at: OffsetDateTime::now_utc(),
                    taken: Instant::now(),
                    usage,
                });
            }
        });
    }

    /// The `usage` object of `/data.json` for the last complete measurement, at `now`: the
    /// instant its calls count from, when it was measured, whether the next one runs, the
    /// repository rows, and what could not be read, by repository.
    pub(super) fn view(&self, now: OffsetDateTime) -> Value {
        let state = self.lock();
        let last = state.last.as_ref();
        json!({
            "since": last.and_then(|last| last.usage.since.clone()),
            "measured_at": last.map(|last| rfc3339(last.at)),
            "measured_ago": last.map(|last| ago(now - last.at)),
            "measuring": state.running,
            "rows": last.map_or_else(|| json!([]), |last| last.usage.repository_objects()),
            "errors": last.map_or_else(Vec::new, |last| last.usage.problems.clone()),
        })
    }
}
