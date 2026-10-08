// generated from conductor v1
// model digest a61944b1d962028b0aff537f36be397359d5ec8be287a7db0368ad0bad816597
// contract digest cf5dcfb046d79cfcab8cd24240fae61c3aac1f945b760367fefff77c0d5a1bd1
// do not edit: regenerate with `ess synthesize --layout crate`

//! Config — `conductor.config`.
//!
//! The one file that names conductor's instances, `conductor.config/1`, read from `--config`, else the `conductor-config` setting, else ~/.b10x/conductor/conductor.yaml (story:config-file). An instance is one organization conductor runs: where its repositories come from, where their checkouts are, where conductor keeps its records, state and cache, which harness and model each role runs on, its cadence, thresholds, reports, and who decides each decision class. These types hold the effective config, after defaults; the file may leave out every key of an instance but its name, its sources and its checkouts. No field takes a secret.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// Authority — `conductor.config.Authority`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Authority {
    /// `class_c` — `conductor.decision.Decider`.
    pub class_c: crate::decision::Decider,
    /// `class_o` — `conductor.decision.Decider`.
    pub class_o: crate::decision::Decider,
}

/// Cadence — `conductor.config.Cadence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cadence {
    /// `cycle` — `Duration`.
    pub cycle: crate::primitives::Duration,
    /// `daily` — `conductor.config.TimeOfDay`.
    pub daily: TimeOfDay,
    /// `watch` — `Duration`.
    pub watch: crate::primitives::Duration,
    /// `ci` — `Duration`.
    pub ci: crate::primitives::Duration,
}

/// Catalog — `conductor.config.Catalog`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Catalog {
    /// `repository` — `String`.
    pub repository: String,
    /// `path` — `String`.
    pub path: String,
    /// `names` — `conductor.config.CatalogNames`.
    pub names: CatalogNames,
}

/// CatalogNames — `conductor.config.CatalogNames`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogNames {
    /// `Plain`.
    Plain,
    /// `Hex`.
    Hex,
}

/// Checkouts — `conductor.config.Checkouts`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checkouts {
    /// `root` — `String`.
    pub root: String,
    /// `trees` — `String`.
    pub trees: String,
}

/// CommandWord — `conductor.config.CommandWord`: a distinct wrapper around `String`.
///
/// Every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._:/-`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandWord(pub String);

/// Config — `conductor.config.Config`.
///
/// Every value satisfies `version == "conductor.config/1"` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.name` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(not (defined(default)) or exists instance in instances: (instance.name == default))` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `version` — `String`.
    pub version: String,
    /// `default` — `Optional<conductor.config.InstanceName>`.
    pub default: Option<InstanceName>,
    /// `instances` — `List<conductor.config.Instance>`.
    pub instances: Vec<Instance>,
}

/// Controllers — `conductor.config.Controllers`.
///
/// Every value satisfies `max_working >= 1` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `max_subagents >= 1` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controllers {
    /// `max_working` — `Integer`.
    pub max_working: i64,
    /// `max_subagents` — `Integer`.
    pub max_subagents: i64,
}

/// Gibibytes — `conductor.config.Gibibytes`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Gibibytes(pub i64);

/// GitHubSource — `conductor.config.GitHubSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubSource {
    /// `owner` — `String`.
    pub owner: String,
}

/// GitLabSource — `conductor.config.GitLabSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitLabSource {
    /// `group` — `String`.
    pub group: String,
    /// `exclude` — `List<String>`.
    pub exclude: Vec<String>,
}

/// Harness — `conductor.config.Harness`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    /// `Claude`.
    Claude,
    /// `Codex`.
    Codex,
}

/// Instance — `conductor.config.Instance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// `name` — `conductor.config.InstanceName`.
    pub name: InstanceName,
    /// `sources` — `List<conductor.config.Source>`.
    pub sources: Vec<Source>,
    /// `checkouts` — `conductor.config.Checkouts`.
    pub checkouts: Checkouts,
    /// `records` — `String`.
    pub records: String,
    /// `state` — `String`.
    pub state: String,
    /// `cache` — `String`.
    pub cache: String,
    /// `roles` — `List<conductor.config.Role>`.
    pub roles: Vec<Role>,
    /// `controllers` — `conductor.config.Controllers`.
    pub controllers: Controllers,
    /// `repositories` — `List<conductor.config.RepositoryRule>`.
    pub repositories: Vec<RepositoryRule>,
    /// `cadence` — `conductor.config.Cadence`.
    pub cadence: Cadence,
    /// `thresholds` — `conductor.config.Thresholds`.
    pub thresholds: Thresholds,
    /// `retention` — `conductor.config.Retention`.
    pub retention: Retention,
    /// `reports` — `List<conductor.config.Report>`.
    pub reports: Vec<Report>,
    /// `authority` — `conductor.config.Authority`.
    pub authority: Authority,
    /// `operator` — `Optional<String>`.
    pub operator: Option<String>,
    /// `catalog` — `Optional<conductor.config.Catalog>`.
    pub catalog: Option<Catalog>,
}

/// InstanceName — `conductor.config.InstanceName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceName(pub String);

/// LocalSource — `conductor.config.LocalSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSource {
    /// `path` — `String`.
    pub path: String,
    /// `exclude` — `List<String>`.
    pub exclude: Vec<String>,
}

/// Report — `conductor.config.Report`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// `channel` — `String`.
    pub channel: String,
    /// `as` — `String`.
    pub r#as: String,
    /// `when` — `List<String>`.
    pub when: Vec<String>,
}

/// RepositoryRule — `conductor.config.RepositoryRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRule {
    /// `match` — `String`.
    pub r#match: String,
    /// `activity` — `conductor.direction.Activity`.
    pub activity: crate::direction::Activity,
}

/// Retention — `conductor.config.Retention`.
///
/// Every value satisfies `snapshots >= 1` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retention {
    /// `snapshots` — `Integer`.
    pub snapshots: i64,
}

/// Role — `conductor.config.Role`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// `role` — `String`.
    pub role: String,
    /// `harness` — `conductor.config.Harness`.
    pub harness: Harness,
    /// `model` — `conductor.config.CommandWord`.
    pub model: CommandWord,
    /// `agent` — `Optional<conductor.config.CommandWord>`.
    pub agent: Option<CommandWord>,
    /// `settings` — `Optional<conductor.config.CommandWord>`.
    pub settings: Option<CommandWord>,
    /// `profile` — `Optional<conductor.config.CommandWord>`.
    pub profile: Option<CommandWord>,
}

/// ShowConfig — `conductor.config.ShowConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowConfig {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// Source — `conductor.config.Source`: one of a fixed set of shapes, tagged on the wire by `kind`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    /// Tagged `GitHub` — `conductor.config.GitHubSource`.
    GitHub(GitHubSource),
    /// Tagged `GitLab` — `conductor.config.GitLabSource`.
    GitLab(GitLabSource),
    /// Tagged `Local` — `conductor.config.LocalSource`.
    Local(LocalSource),
}

/// Thresholds — `conductor.config.Thresholds`.
///
/// Every value satisfies `gate_resume > gate_stop` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thresholds {
    /// `context_handover` — `conductor.config.Tokens`.
    pub context_handover: Tokens,
    /// `disk_path` — `String`.
    pub disk_path: String,
    /// `disk_low` — `conductor.config.Gibibytes`.
    pub disk_low: Gibibytes,
    /// `disk_clear` — `conductor.config.Gibibytes`.
    pub disk_clear: Gibibytes,
    /// `disk_admit` — `conductor.config.Gibibytes`.
    pub disk_admit: Gibibytes,
    /// `build_slot` — `conductor.config.Gibibytes`.
    pub build_slot: Gibibytes,
    /// `build_size` — `conductor.config.Gibibytes`.
    pub build_size: Gibibytes,
    /// `gate_stop` — `conductor.config.Gibibytes`.
    pub gate_stop: Gibibytes,
    /// `gate_resume` — `conductor.config.Gibibytes`.
    pub gate_resume: Gibibytes,
    /// `watchdog_every` — `Duration`.
    pub watchdog_every: crate::primitives::Duration,
}

/// TimeOfDay — `conductor.config.TimeOfDay`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeOfDay(pub String);

/// Tokens — `conductor.config.Tokens`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens(pub i64);

/// ValidateConfig — `conductor.config.ValidateConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateConfig {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}
