//! The tuner (`story:tuner`, design architecture-next § 9, § 13 row 7; the public page's
//! "Self-improvement over settings"): the host role `tuner`, run as a step of the instance's daily
//! tick after `metric compute` ([`daily`]).
//!
//! **Settle.** Each change it applied whose window ended (the proposal's day plus `window_days`
//! at or before the tick's day) is judged on the metric's newest instance-wide reading after the
//! proposal's day: kept when it improved on the baseline in the target's direction (lower for
//! `AtMost`, higher for `AtLeast`), else reverted to `old` through [`put_config`]. No reading
//! after the proposal's day is no improvement.
//!
//! **Propose.** While no change of the instance is applied and unsettled, and the tick settled
//! none, one proposal per tick: the KPI readings that miss their target, then the metrics of the
//! open findings, by metric id; for the first of them, the first key that is not skipped: the
//! grant's keys (`globals.tuning`) in their order, then the built-in [`LEVERS`]. A key is skipped
//! when a finding the tuner raised for it is open, or its last proposal for that metric was
//! refused within [`WINDOW_DAYS`]. The new value is one more (a whole number) or ten percent more
//! than the effective value, or less after its last change for that metric was reverted, and
//! turned the other way when it would leave the grant's range. A value that is not a number (a
//! duration, a size) is never tuned: a grant key holding one is proposed unchanged and refused by
//! name.
//!
//! **Apply.** A key the grant names goes through [`put_config`] at the instance's level as the
//! tuner, an agent, so `PutConfig` itself holds it to the grant: applied with the revision, or
//! refused with the refusal. A key outside the grant is refused and a friction finding raised for
//! it (or the open one named), so a story can widen the grant; nothing is applied.
//!
//! [`tune`] answers the brief's lines: every proposal it wrote, kept or reverted, and each change
//! still in its window, each with its key, values and readings.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context as _, Result, anyhow, bail};
use conductor_model::agent::{AgentKindName, AgentName};
use conductor_model::behaviour::{
    FrictionFindingStorage, Generated, MetricReadingStorage, MetricStorage, SettingsProposalStorage,
};
use conductor_model::config::{InstanceName, Level, PutConfig, RoleName};
use conductor_model::improvement::obligations::{
    ApplyProposalBehavior, ProposeSettingBehavior, RaiseFindingBehavior, RefuseProposalBehavior,
    SettingsProposalsQuery, SettleProposalBehavior,
};
use conductor_model::improvement::{
    ApplyProposal, FindingId, FrictionFindingSnapshot, FrictionFindingState, MetricId,
    MetricReadingId, MetricReadingSnapshot, MetricReadingState, MetricSnapshot, ProposalId,
    ProposeSetting, ProposeSettingOutcome, RaiseFinding, RaiseFindingOutcome, RefuseProposal,
    SettingsProposalData, SettingsProposalSnapshot, SettingsProposalState, SettleProposal,
    TargetDirection,
};
use conductor_model::operator::{Actor, ActorKind, AgentActor};
use conductor_model::primitives::Timestamp;
use serde_json::Value;
use time::format_description;
use time::{Date, Duration};

use crate::cli::ViewArgs;
use crate::config::put::put_config;
use crate::store::Store;
use crate::store::proposal::proposal_state_name;

/// The role the tuner runs as (`conductor.config.Tuner`, `conductor.improvement.Tuner`).
pub const TUNER: &str = "tuner";

/// How many days after it is applied a change's metric is read again.
pub const WINDOW_DAYS: i64 = 7;

/// The keys the tuner proposes for a metric after the grant's: numeric keys of an instance.
pub const LEVERS: [&str; 1] = ["controllers.max_working"];

/// What one run of the tuner is given.
#[derive(Debug, Clone)]
pub struct Tuning {
    /// The config file, which [`put_config`] changes.
    pub file: PathBuf,
    /// The home directory `~` reads as.
    pub home: PathBuf,
    /// The name of the instance it tunes.
    pub instance: String,
    /// The UTC day of the daily tick (`YYYY-MM-DD`): the day `metric compute` read.
    pub day: String,
    /// When it runs: the time every record it writes carries.
    pub now: Timestamp,
}

/// The daily tick's step: [`tune`] for the active instance, with the config file `config_flag`
/// (the global `--config`) or the one [`crate::config::set_active`] located, never one found
/// again from the environment; the instance's store under `state`, and the daemon's store
/// (`daemon.store`) opened only when a change is written. Never fails the tick: what went wrong
/// is a line of the brief.
#[must_use]
pub fn daily(config_flag: Option<&Path>, state: Option<&Path>, day: &str) -> Vec<String> {
    daily_or(config_flag, state, day)
        .unwrap_or_else(|error| vec![format!("- the tuner did not run: {error:#}")])
}

fn daily_or(config_flag: Option<&Path>, state: Option<&Path>, day: &str) -> Result<Vec<String>> {
    let environment = crate::config::Environment::from_process()?;
    let file = config_flag
        .map(|flag| environment.cwd.join(flag))
        .or_else(|| crate::config::config_file().map(Path::to_owned));
    let active = crate::config::active();
    let Some(file) = file.filter(|file| active.from_file && file.is_file()) else {
        return Ok(vec![
            "- the tuner did not run: the instance is read from no config file".to_owned(),
        ]);
    };
    let home = environment.home()?.to_owned();
    let text = fs::read_to_string(&file)
        .with_context(|| format!("read the config file {}", file.display()))?;
    let config = crate::config::parse(&text, &home).map_err(|problems| {
        let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
        anyhow!("the config file does not validate: {}", said.join("; "))
    })?;
    let mut store = crate::state::open(state)?;
    let tuning = Tuning {
        file,
        home,
        instance: active.instance.name.0.clone(),
        day: day.to_owned(),
        now: crate::records::now()?,
    };
    run(
        &tuning,
        &mut store,
        Daemon::At(config.daemon.store.0.clone(), None),
    )
}

/// The daemon's store: one the caller holds, or the URL `daemon.store` names, opened on the
/// first change, so a run that changes nothing opens none.
enum Daemon<'s> {
    Held(&'s mut Store),
    At(String, Option<Box<Store>>),
}

impl Daemon<'_> {
    fn store(&mut self) -> Result<&mut Store> {
        match self {
            Self::Held(store) => Ok(store),
            Self::At(url, opened) => {
                if opened.is_none() {
                    if let Ok(crate::store::Location::Sqlite(path)) =
                        crate::store::Location::parse(url)
                        && let Some(parent) = path.parent()
                    {
                        fs::create_dir_all(parent).with_context(|| {
                            format!("daemon.store: create {}", parent.display())
                        })?;
                    }
                    *opened = Some(Box::new(
                        crate::store::open_url(url, crate::daemon::TENANT)
                            .map_err(|error| anyhow!("daemon.store: {error:#}"))?,
                    ));
                }
                opened
                    .as_deref_mut()
                    .ok_or_else(|| anyhow!("daemon.store: {url} did not open"))
            }
        }
    }
}

/// The tuner's run, as the module documentation says: settles what is due, then proposes at
/// most one change; the brief's lines. `daemon` is the daemon's store, which [`put_config`]
/// records each change in.
///
/// # Errors
///
/// The config file does not read or names no such instance, a store does not keep a record, or
/// [`put_config`] fails.
pub fn tune(tuning: &Tuning, store: &mut Store, daemon: &mut Store) -> Result<Vec<String>> {
    run(tuning, store, Daemon::Held(daemon))
}

fn run(tuning: &Tuning, store: &mut Store, daemon: Daemon<'_>) -> Result<Vec<String>> {
    let text = fs::read_to_string(&tuning.file)
        .with_context(|| format!("read the config file {}", tuning.file.display()))?;
    let config = crate::config::parse(&text, &tuning.home).map_err(|problems| {
        let said: Vec<String> = problems.iter().map(ToString::to_string).collect();
        anyhow!("the config file does not validate: {}", said.join("; "))
    })?;
    let Some(index) = config
        .instances
        .iter()
        .position(|one| one.name.0 == tuning.instance)
    else {
        bail!("the config file names no instance {:?}", tuning.instance);
    };
    let instance = &config.instances[index];
    let file: Value = serde_yaml::from_str(&text).context("read the config file as YAML")?;
    let effective: BTreeMap<String, String> =
        crate::config::effective::settings(&file, index, instance)
            .into_iter()
            .filter(|row| row.role.is_none())
            .map(|row| (row.path, row.value))
            .collect();
    let mut run = Run {
        tuning,
        store,
        daemon,
        actor: Actor {
            kind: ActorKind::Agent,
            operator: None,
            agent: Some(AgentActor {
                instance: InstanceName(tuning.instance.clone()),
                role: RoleName(TUNER.to_owned()),
                name: AgentName(crate::config::session_name(instance, TUNER).0),
                kind: AgentKindName(crate::actor::KIND.to_owned()),
            }),
        },
        grant: config
            .globals
            .tuning
            .keys
            .iter()
            .map(|key| (key.path.clone(), (number(&key.min.0), number(&key.max.0))))
            .collect(),
        effective,
        lines: Vec::new(),
    };
    // A tick that settles a change proposes none: the next reading is of the settled state.
    if !run.settle()? && !run.open() {
        run.propose()?;
    }
    Ok(run.lines)
}

/// One run while it goes.
struct Run<'t, 's> {
    tuning: &'t Tuning,
    store: &'s mut Store,
    daemon: Daemon<'s>,
    actor: Actor,
    /// The grant's keys, in its order, each with its bounds.
    grant: Vec<(String, Range)>,
    /// The instance's effective values by path.
    effective: BTreeMap<String, String>,
    lines: Vec<String>,
}

impl Run<'_, '_> {
    /// The instance's proposals.
    fn proposals(&self) -> Vec<SettingsProposalSnapshot> {
        SettingsProposalStorage::list(&*self.store)
            .into_iter()
            .filter(|held| held.data.instance == self.tuning.instance)
            .collect()
    }

    /// Whether a change of the instance is applied and not settled.
    fn open(&self) -> bool {
        self.proposals()
            .iter()
            .any(|held| held.state == SettingsProposalState::Applied)
    }

    /// The generated behaviour over the instance's store.
    fn generated(&mut self) -> Generated<On<'_>> {
        Generated::new(On(self.store))
    }

    fn kept(&self, what: &str) -> Result<()> {
        self.store
            .check()
            .map_err(|error| anyhow!("record the {what}: {error}"))
    }

    // -----------------------------------------------------------------------------------------
    // Settle
    // -----------------------------------------------------------------------------------------

    /// Keeps or reverts each change whose window ended; whether any was.
    fn settle(&mut self) -> Result<bool> {
        let today = date(&self.tuning.day)?;
        let mut settled = false;
        for held in self.proposals() {
            if held.state != SettingsProposalState::Applied {
                continue;
            }
            let data = &held.data;
            let proposed = date(day_of(&data.proposed_at))?;
            let due = proposed + Duration::days(data.window_days);
            if today < due {
                self.lines
                    .push(format!("{}; its window ends {due}", line(data, "Applied")));
                continue;
            }
            let after = self.reading_after(&data.metric_id, &proposed.to_string());
            let (verdict, value) = after.as_ref().map_or_else(
                || {
                    (
                        format!("no reading of {} after {proposed}", data.metric_id.0),
                        None,
                    )
                },
                |reading| {
                    (
                        reading.data.reading_id.0.clone(),
                        Some(reading.data.value.0.clone()),
                    )
                },
            );
            let improved = value.as_deref().is_some_and(|value| {
                self.improved(&data.metric_id, number(&data.baseline.0), number(value))
            });
            let measured = format!(
                "after the window {verdict}{}",
                value.map(|value| format!(" = {value}")).unwrap_or_default()
            );
            if improved {
                self.settled(data, true, verdict, None)?;
                settled = true;
                self.lines
                    .push(format!("{}; {measured}", line(data, "Kept")));
                continue;
            }
            let reason = format!(
                "tuner: revert {}: {} did not improve on {} ({measured})",
                data.proposal_id.0, data.metric_id.0, data.baseline.0
            );
            match self.put(&data.key, &data.old, reason)? {
                Ok(revision) => {
                    self.settled(data, false, verdict, Some(revision))?;
                    settled = true;
                    self.lines
                        .push(format!("{}; {measured}", line(data, "Reverted")));
                }
                Err(refusal) => self.lines.push(format!(
                    "{}; {measured}; the revert was refused: {refusal}",
                    line(data, "Applied")
                )),
            }
        }
        Ok(settled)
    }

    fn settled(
        &mut self,
        data: &SettingsProposalData,
        improved: bool,
        verdict: String,
        revision: Option<String>,
    ) -> Result<()> {
        let input = SettleProposal {
            actor: self.actor.clone(),
            proposal_id: data.proposal_id.clone(),
            improved,
            verdict,
            revision,
        };
        self.generated()
            .settle_proposal(input)
            .map_err(|unmet| anyhow!("settle {}: {unmet}", data.proposal_id.0))?;
        self.kept("settled proposal")
    }

    /// The metric's newest instance-wide reading of a day after `after`.
    fn reading_after(&self, metric: &MetricId, after: &str) -> Option<MetricReadingSnapshot> {
        MetricReadingStorage::list(&*self.store)
            .into_iter()
            .filter(|held| {
                held.data.metric_id == *metric
                    && held.data.scope == "instance"
                    && held.data.day.as_str() > after
            })
            .max_by(|one, other| {
                (&one.data.day, &one.data.recorded_at.0)
                    .cmp(&(&other.data.day, &other.data.recorded_at.0))
            })
    }

    /// Whether `value` improved on `baseline` in the direction of the metric's target.
    fn improved(&self, metric: &MetricId, baseline: Option<f64>, value: Option<f64>) -> bool {
        let direction =
            MetricStorage::get(&*self.store, metric).and_then(|held| held.data.direction);
        match (direction, baseline, value) {
            (Some(TargetDirection::AtMost), Some(baseline), Some(value)) => value < baseline,
            (Some(TargetDirection::AtLeast), Some(baseline), Some(value)) => value > baseline,
            _ => false,
        }
    }

    // -----------------------------------------------------------------------------------------
    // Propose
    // -----------------------------------------------------------------------------------------

    /// The readings a proposal may be for, in order: the KPI readings that miss their target,
    /// then the readings of the metrics of the open findings.
    fn wanted(&self) -> Vec<MetricReadingSnapshot> {
        let recorded: Vec<MetricReadingSnapshot> = MetricReadingStorage::list(&*self.store)
            .into_iter()
            .filter(|held| {
                held.state == MetricReadingState::Recorded
                    && held.data.scope == "instance"
                    && held.data.target.is_some()
            })
            .collect();
        let mut missing: Vec<MetricReadingSnapshot> = recorded
            .iter()
            .filter(|held| held.data.met == Some(false))
            .cloned()
            .collect();
        missing.sort_by(|one, other| one.data.metric_id.0.cmp(&other.data.metric_id.0));
        let mut found: Vec<MetricReadingSnapshot> = self
            .open_findings()
            .iter()
            .filter_map(|finding| {
                recorded
                    .iter()
                    .find(|held| held.data.metric_id == finding.data.metric_id)
                    .cloned()
            })
            .filter(|held| held.data.met != Some(true))
            .collect();
        found.sort_by(|one, other| one.data.metric_id.0.cmp(&other.data.metric_id.0));
        for reading in found {
            if !missing
                .iter()
                .any(|held| held.data.reading_id == reading.data.reading_id)
            {
                missing.push(reading);
            }
        }
        missing
    }

    fn open_findings(&self) -> Vec<FrictionFindingSnapshot> {
        FrictionFindingStorage::list(&*self.store)
            .into_iter()
            .filter(|held| held.state == FrictionFindingState::Open)
            .collect()
    }

    fn signature(&self, key: &str) -> String {
        format!("tuning:{}:{key}", self.tuning.instance)
    }

    /// The keys a proposal may change, in order: the grant's, then the built-in levers.
    fn keys(&self) -> Vec<String> {
        let mut keys: Vec<String> = self.grant.iter().map(|(key, _)| key.clone()).collect();
        for lever in LEVERS {
            if !keys.iter().any(|key| key == lever) {
                keys.push(lever.to_owned());
            }
        }
        keys
    }

    /// Whether `key` waits for `metric`: a finding raised for it is open, or its last proposal
    /// for the metric was refused within [`WINDOW_DAYS`].
    fn waits(&self, key: &str, metric: &MetricId) -> Result<bool> {
        let signature = self.signature(key);
        if self
            .open_findings()
            .iter()
            .any(|held| held.data.signature == signature)
        {
            return Ok(true);
        }
        let today = date(&self.tuning.day)?;
        let last = self.last(key, metric);
        Ok(match last {
            Some(held) if held.state == SettingsProposalState::Refused => {
                date(day_of(&held.data.proposed_at))? + Duration::days(WINDOW_DAYS) > today
            }
            _ => false,
        })
    }

    /// The newest proposal of `key` for `metric`.
    fn last(&self, key: &str, metric: &MetricId) -> Option<SettingsProposalSnapshot> {
        self.proposals()
            .into_iter()
            .filter(|held| held.data.key == key && held.data.metric_id == *metric)
            .max_by(|one, other| one.data.proposed_at.0.cmp(&other.data.proposed_at.0))
    }

    fn propose(&mut self) -> Result<()> {
        for reading in self.wanted() {
            let metric = reading.data.metric_id.clone();
            for key in self.keys() {
                if self.waits(&key, &metric)? {
                    continue;
                }
                let Some(old) = self.effective.get(&key).cloned() else {
                    continue;
                };
                let granted = self
                    .grant
                    .iter()
                    .find(|(path, _)| *path == key)
                    .map(|(_, range)| *range);
                let Some(new) = self.next(&key, &metric, &old, granted) else {
                    continue;
                };
                if self.proposed(&key, &old, &new, &reading)? {
                    let proposal_id = self.id(&key);
                    self.decide(&proposal_id, &key, &new, granted, &reading)?;
                }
                return Ok(());
            }
        }
        Ok(())
    }

    fn id(&self, key: &str) -> ProposalId {
        ProposalId(format!(
            "{}|{key}|{}",
            self.tuning.instance, self.tuning.day
        ))
    }

    /// The value to propose for `key`, or none when no step stays inside the grant's range. A
    /// value that is not a number is proposed unchanged, for a grant key only, to be refused.
    fn next(
        &self,
        key: &str,
        metric: &MetricId,
        old: &str,
        granted: Option<Range>,
    ) -> Option<String> {
        let Some(value) = number(old) else {
            return granted.map(|_| old.to_owned());
        };
        let down = self.last(key, metric).is_some_and(|held| {
            held.state == SettingsProposalState::Reverted
                && number(&held.data.new) > number(&held.data.old)
        });
        let whole = !old.contains('.') && value.fract() == 0.0;
        let step = |sign: f64| {
            if whole {
                value + sign
            } else {
                value * (1.0 + sign * 0.1)
            }
        };
        let inside = |candidate: f64| match granted {
            Some((min, max)) => {
                min.is_some_and(|min| candidate >= min) && max.is_some_and(|max| candidate <= max)
            }
            None => candidate >= 0.0,
        };
        let first = if down { -1.0 } else { 1.0 };
        [first, -first]
            .into_iter()
            .map(step)
            .find(|candidate| inside(*candidate))
            .map(|candidate| render(candidate, whole))
    }

    /// Writes the proposal; whether it was written (a proposal of the same id the same day was
    /// not).
    fn proposed(
        &mut self,
        key: &str,
        old: &str,
        new: &str,
        reading: &MetricReadingSnapshot,
    ) -> Result<bool> {
        let input = ProposeSetting {
            actor: self.actor.clone(),
            proposal_id: self.id(key),
            instance: self.tuning.instance.clone(),
            key: key.to_owned(),
            old: old.to_owned(),
            new: new.to_owned(),
            metric_id: reading.data.metric_id.clone(),
            reading: reading.data.reading_id.clone(),
            baseline: reading.data.value.clone(),
            window_days: WINDOW_DAYS,
            proposed_at: self.tuning.now.clone(),
        };
        let outcome = self
            .generated()
            .propose_setting(input)
            .map_err(|unmet| anyhow!("propose {key}: {unmet}"))?;
        self.kept("proposal")?;
        Ok(matches!(outcome, ProposeSettingOutcome::Proposed { .. }))
    }

    /// Applies the proposal inside the grant, or refuses it.
    fn decide(
        &mut self,
        proposal_id: &ProposalId,
        key: &str,
        new: &str,
        granted: Option<Range>,
        reading: &MetricReadingSnapshot,
    ) -> Result<()> {
        if granted.is_some() && number(new).is_none() {
            return self.refuse(
                proposal_id,
                format!(
                    "{key} holds {new:?}, a duration or a size, not a number: the tuner changes \
                     only numeric keys"
                ),
                None,
            );
        }
        if granted.is_none() {
            let finding = self.finding(proposal_id, key, reading)?;
            return self.refuse(
                proposal_id,
                format!("{key} is not in globals.tuning; a finding asks to widen the grant"),
                Some(finding),
            );
        }
        let reason = format!(
            "tuner: {}: {key} to {new}, to move {} (reading {} = {})",
            proposal_id.0,
            reading.data.metric_id.0,
            reading.data.reading_id.0,
            reading.data.value.0
        );
        match self.put(key, new, reason)? {
            Ok(revision) => {
                let input = ApplyProposal {
                    actor: self.actor.clone(),
                    proposal_id: proposal_id.clone(),
                    revision,
                };
                self.generated()
                    .apply_proposal(input)
                    .map_err(|unmet| anyhow!("apply {}: {unmet}", proposal_id.0))?;
                self.kept("applied proposal")?;
                self.line_of(proposal_id, "")
            }
            Err(refusal) => self.refuse(proposal_id, refusal, None),
        }
    }

    fn refuse(
        &mut self,
        proposal_id: &ProposalId,
        reason: String,
        finding: Option<FindingId>,
    ) -> Result<()> {
        let input = RefuseProposal {
            actor: self.actor.clone(),
            proposal_id: proposal_id.clone(),
            reason: reason.clone(),
            finding,
        };
        self.generated()
            .refuse_proposal(input)
            .map_err(|unmet| anyhow!("refuse {}: {unmet}", proposal_id.0))?;
        self.kept("refused proposal")?;
        self.line_of(proposal_id, &format!(": {reason}"))
    }

    /// The open finding raised for `key`, or a new one.
    fn finding(
        &mut self,
        proposal_id: &ProposalId,
        key: &str,
        reading: &MetricReadingSnapshot,
    ) -> Result<FindingId> {
        let signature = self.signature(key);
        if let Some(held) = self
            .open_findings()
            .into_iter()
            .find(|held| held.data.signature == signature)
        {
            return Ok(held.data.finding_id);
        }
        let finding_id = FindingId(format!(
            "FF-{}-tuning-{}-{key}",
            self.tuning.day, self.tuning.instance
        ));
        let input = RaiseFinding {
            actor: self.actor.clone(),
            finding_id: finding_id.clone(),
            signature,
            evidence: vec![proposal_id.0.clone(), reading.data.reading_id.0.clone()],
            metric_id: reading.data.metric_id.clone(),
            raised_at: self.tuning.now.clone(),
        };
        let outcome = self
            .generated()
            .raise_finding(input)
            .map_err(|unmet| anyhow!("raise {}: {unmet}", finding_id.0))?;
        self.kept("finding")?;
        match outcome {
            RaiseFindingOutcome::Raised { .. } | RaiseFindingOutcome::AlreadyRaised { .. } => {
                Ok(finding_id)
            }
            RaiseFindingOutcome::NoSuchMetric { error } => {
                bail!(
                    "MetricNotFound: no metric has the id {:?}",
                    error.metric_id.0
                )
            }
        }
    }

    /// The brief's line of the proposal as it is held now, with `tail`.
    fn line_of(&mut self, proposal_id: &ProposalId, tail: &str) -> Result<()> {
        let held = SettingsProposalStorage::get(&*self.store, proposal_id)
            .ok_or_else(|| anyhow!("the proposal {} is not held", proposal_id.0))?;
        self.lines.push(format!(
            "{}{tail}",
            line(&held.data, proposal_state_name(held.state))
        ));
        Ok(())
    }

    /// Changes `key` of the instance to `value` through [`put_config`] as the tuner: the
    /// revision's id, or the refusal.
    fn put(&mut self, key: &str, value: &str, reason: String) -> Result<Result<String, String>> {
        let input = PutConfig {
            actor: self.actor.clone(),
            level: Level::Instance,
            instance: Some(InstanceName(self.tuning.instance.clone())),
            role: None,
            path: key.to_owned(),
            value: Some(value.to_owned()),
            reason,
            changed_at: self.tuning.now.clone(),
        };
        Ok(put_config(
            &self.tuning.file,
            &self.tuning.home,
            self.daemon.store()?,
            &input,
        )?
        .map(|revision| revision.revision_id.0.0)
        .map_err(|refusal| refusal.to_string()))
    }
}

/// One proposal as the brief writes it.
fn line(data: &SettingsProposalData, state: &str) -> String {
    format!(
        "- {state} {} {} -> {} on {}, to move {} (reading {} = {})",
        data.key,
        data.old,
        data.new,
        data.instance,
        data.metric_id.0,
        data.reading.0,
        data.baseline.0
    )
}

fn number(text: &str) -> Option<f64> {
    text.trim()
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

fn render(value: f64, whole: bool) -> String {
    if whole {
        format!("{value:.0}")
    } else {
        let text = format!("{value:.3}");
        text.trim_end_matches('0').trim_end_matches('.').to_owned()
    }
}

/// The `YYYY-MM-DD` of a timestamp.
fn day_of(at: &Timestamp) -> &str {
    at.0.get(..10).unwrap_or(&at.0)
}

fn date(day: &str) -> Result<Date> {
    let format = format_description::parse_borrowed::<2>("[year]-[month]-[day]")
        .context("the day's format")?;
    Date::parse(day, &format).with_context(|| format!("{day:?} is not a day, YYYY-MM-DD"))
}

/// The bounds of a key of the grant, each none where it does not read as a number.
type Range = (Option<f64>, Option<f64>);

/// The generated behaviour's ports over a borrowed store.
struct On<'s>(&'s mut Store);

impl MetricStorage for On<'_> {
    fn get(&self, identity: &MetricId) -> Option<MetricSnapshot> {
        MetricStorage::get(&*self.0, identity)
    }
    fn put(&mut self, snapshot: MetricSnapshot) {
        MetricStorage::put(self.0, snapshot);
    }
    fn delete(&mut self, identity: &MetricId) {
        MetricStorage::delete(self.0, identity);
    }
    fn list(&self) -> Vec<MetricSnapshot> {
        MetricStorage::list(&*self.0)
    }
}

impl MetricReadingStorage for On<'_> {
    fn get(&self, identity: &MetricReadingId) -> Option<MetricReadingSnapshot> {
        MetricReadingStorage::get(&*self.0, identity)
    }
    fn put(&mut self, snapshot: MetricReadingSnapshot) {
        MetricReadingStorage::put(self.0, snapshot);
    }
    fn delete(&mut self, identity: &MetricReadingId) {
        MetricReadingStorage::delete(self.0, identity);
    }
    fn list(&self) -> Vec<MetricReadingSnapshot> {
        MetricReadingStorage::list(&*self.0)
    }
}

impl SettingsProposalStorage for On<'_> {
    fn get(&self, identity: &ProposalId) -> Option<SettingsProposalSnapshot> {
        SettingsProposalStorage::get(&*self.0, identity)
    }
    fn put(&mut self, snapshot: SettingsProposalSnapshot) {
        SettingsProposalStorage::put(self.0, snapshot);
    }
    fn delete(&mut self, identity: &ProposalId) {
        SettingsProposalStorage::delete(self.0, identity);
    }
    fn list(&self) -> Vec<SettingsProposalSnapshot> {
        SettingsProposalStorage::list(&*self.0)
    }
}

impl FrictionFindingStorage for On<'_> {
    fn get(&self, identity: &FindingId) -> Option<FrictionFindingSnapshot> {
        FrictionFindingStorage::get(&*self.0, identity)
    }
    fn put(&mut self, snapshot: FrictionFindingSnapshot) {
        FrictionFindingStorage::put(self.0, snapshot);
    }
    fn delete(&mut self, identity: &FindingId) {
        FrictionFindingStorage::delete(self.0, identity);
    }
    fn list(&self) -> Vec<FrictionFindingSnapshot> {
        FrictionFindingStorage::list(&*self.0)
    }
}

/// `conductor tuner settings-proposals`: the view `conductor.improvement.SettingsProposals`,
/// every proposal by its day and key.
///
/// # Errors
///
/// There is no store, it does not open, or the rows cannot be written.
pub fn settings_proposals(state: Option<&Path>, view: ViewArgs) -> Result<ExitCode> {
    const COLUMNS: [&str; 15] = [
        "proposal_id",
        "state",
        "instance",
        "key",
        "old",
        "new",
        "metric_id",
        "reading",
        "baseline",
        "window_days",
        "proposed_at",
        "revision",
        "reason",
        "finding",
        "verdict",
    ];
    let generated = Generated::new(crate::state::open_reader(state)?);
    let mut rows = generated
        .settings_proposals()
        .map_err(|unmet| anyhow!("tuner settings-proposals: {unmet}"))?;
    rows.sort_by(|one, other| {
        (&one.proposed_at.0, &one.proposal_id.0).cmp(&(&other.proposed_at.0, &other.proposal_id.0))
    });
    let rows: Vec<Vec<Value>> = rows
        .into_iter()
        .map(|row| {
            vec![
                Value::from(row.proposal_id.0),
                Value::from(proposal_state_name(row.state)),
                Value::from(row.instance),
                Value::from(row.key),
                Value::from(row.old),
                Value::from(row.new),
                Value::from(row.metric_id.0),
                Value::from(row.reading.0),
                Value::from(row.baseline.0),
                Value::from(row.window_days),
                Value::from(row.proposed_at.0),
                Value::from(row.revision),
                Value::from(row.reason),
                Value::from(row.finding.map(|id| id.0)),
                Value::from(row.verdict),
            ]
        })
        .collect();
    crate::store::show(
        &generated.ports,
        "settings proposals",
        &view.render(&COLUMNS, &rows),
    )
}

#[cfg(test)]
mod tests {
    use super::{number, render};

    #[test]
    fn a_step_is_written_as_the_config_reads_it() {
        assert_eq!(render(3.0, true), "3");
        assert_eq!(render(0.55, false), "0.55");
        assert_eq!(render(1.1, false), "1.1");
        assert_eq!(number("2h"), None);
        assert_eq!(number("15G"), None);
        assert_eq!(number("0.5"), Some(0.5));
    }
}
