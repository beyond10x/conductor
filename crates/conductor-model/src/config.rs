// generated from conductor v1
// contract digest 251aac8e3d7844fa8bf5d80ba2d224b99dac46d204effce0fa0168d87512e308
// do not edit: regenerate with `ess synthesize --layout crate`

//! Config — `conductor.config`.
//!
//! The one file conductord reconciles from, `conductor.config/2`, read from `--config`, else the `conductor-config` setting, else ~/.b10x/conductor/conductor.yaml (story:config-file; docs/design/architecture-next.md § 9). It holds the `daemon` section (its port, TLS listener, reconcile cadence, the one store URL, the secrets source, the dashboard's served path and its operators), `globals` (the defaults every instance starts from) and `instances[]`. An instance is one organization conductor runs: its placement, where its repositories come from, where their checkouts are, where conductor keeps its records, state and cache, which kind and model each role runs on, its cadence, thresholds, reports, and who decides each decision class; every key an instance or a role writes overrides the global one. Precedence is role over instance over global (`Level`), and `EffectiveSettings` shows every key with its effective value and the level it came from. These types hold the effective config, after defaults; the file may leave out every key of an instance but its name, its sources and its checkouts. No field takes a secret: the file names one as `secret:<name>` (`conductor.secret.SecretName`). The config is desired state and is not kept in the store (§ 8); every change through `PutConfig` is validated, written, committed with its actor and reason, recorded as `ConfigChanged` and reconciled. A `conductor.config/1` file is read once by `conductor config migrate`.
//!
//! Everything this bounded context declares that the synthesis plan marks generated.

/// ActivityChange — `conductor.config.ActivityChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityChange {
    /// `repository` — `conductor.direction.MarkedRepository`.
    pub repository: crate::direction::MarkedRepository,
    /// `activity` — `conductor.direction.Activity`.
    pub activity: crate::direction::Activity,
    /// `reason` — `String`.
    pub reason: String,
}

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

/// ChangeConfig — `conductor.config.ChangeConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeConfig {
    /// `level` — `String`.
    pub level: String,
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
    /// `role` — `Optional<String>`.
    pub role: Option<String>,
    /// `path` — `String`.
    pub path: String,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `reason` — `String`.
    pub reason: String,
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
///
/// Checked by [`CommandWord::broken_constraint`]; the generated decoder refuses a value that breaks one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandWord(pub String);

impl CommandWord {
    /// The first constraint `conductor.config.CommandWord` declares that this value breaks — its alphabet, its prefix,
    /// then each invariant — as what a decoder expected instead, or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value, as the conformance
    /// interpreter reads it. A wrapped newtype's own constraints are its own to check.
    pub fn broken_constraint(&self) -> Option<&'static str> {
        if !self.0.chars().all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._:/-".contains(c)) {
            return Some("a value whose every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789._:/-`");
        }
        None
    }
}

/// ConductorSession — `conductor.config.ConductorSession`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConductorSession {
    /// `served_by` — `Optional<conductor.config.InstanceName>`.
    pub served_by: Option<InstanceName>,
    /// `session_name` — `Optional<conductor.config.SessionName>`.
    pub session_name: Option<SessionName>,
}

/// Config — `conductor.config.Config`.
///
/// Every value satisfies `version == "conductor.config/2"` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.name` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.session_prefix` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.state` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct instance in instances by instance.records` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(not (defined(default)) or exists instance in instances: (instance.name == default))` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// `version` — `String`.
    pub version: String,
    /// `daemon` — `conductor.config.Daemon`.
    pub daemon: Daemon,
    /// `globals` — `conductor.config.Globals`.
    pub globals: Globals,
    /// `default` — `Optional<conductor.config.InstanceName>`.
    pub default: Option<InstanceName>,
    /// `instances` — `List<conductor.config.Instance>`.
    pub instances: Vec<Instance>,
}

/// ConfigChange — `conductor.config.ConfigChange`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigChange {
    /// `path` — `String`.
    pub path: String,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
}

/// ConfigEffect — `conductor.config.ConfigEffect`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigEffect {
    /// `WatchTick`.
    WatchTick,
    /// `Cycle`.
    Cycle,
    /// `LoopRestart`.
    LoopRestart,
    /// `Fixed`.
    Fixed,
}

/// ConfigField — `conductor.config.ConfigField`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigField {
    /// `path` — `String`.
    pub path: String,
    /// `value` — `String`.
    pub value: String,
    /// `default` — `Optional<String>`.
    pub default: Option<String>,
    /// `takes_effect` — `conductor.config.ConfigEffect`.
    pub takes_effect: ConfigEffect,
}

/// ConfigInstance — `conductor.config.ConfigInstance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInstance {
    /// `name` — `conductor.config.InstanceName`.
    pub name: InstanceName,
    /// `session_prefix` — `Optional<conductor.config.SessionPrefix>`.
    pub session_prefix: Option<SessionPrefix>,
    /// `forge` — `String`.
    pub forge: String,
    /// `current` — `Boolean`.
    pub current: bool,
}

/// ConfigPage — `conductor.config.ConfigPage`.
///
/// Every value satisfies `index >= 0` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPage {
    /// `file` — `String`.
    pub file: String,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: InstanceName,
    /// `index` — `Integer`.
    pub index: i64,
    /// `fields` — `List<conductor.config.ConfigField>`.
    pub fields: Vec<ConfigField>,
    /// `instances` — `List<conductor.config.ConfigInstance>`.
    pub instances: Vec<ConfigInstance>,
    /// `repositories` — `List<conductor.direction.RepositoryActivityRow>`.
    pub repositories: Vec<crate::direction::RepositoryActivityRow>,
    /// `errors` — `List<String>`.
    pub errors: Vec<String>,
}

/// ConfigProblem — `conductor.config.ConfigProblem`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigProblem {
    /// `path` — `String`.
    pub path: String,
    /// `message` — `String`.
    pub message: String,
}

/// The states of `conductor.config.ConfigRevision`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `ConfigRevision<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigRevisionState {
    /// `Committed`.
    Committed,
}

/// ConfigWriteRefusal — `conductor.config.ConfigWriteRefusal`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigWriteRefusal {
    /// `NotThisPage`.
    NotThisPage,
    /// `TokenMissing`.
    TokenMissing,
    /// `Invalid`.
    Invalid,
    /// `ReasonMissing`.
    ReasonMissing,
    /// `NothingStaged`.
    NothingStaged,
    /// `NoConfigFile`.
    NoConfigFile,
}

/// ConfigWriteRefused — `conductor.config.ConfigWriteRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWriteRefused {
    /// `refusal` — `conductor.config.ConfigWriteRefusal`.
    pub refusal: ConfigWriteRefusal,
    /// `detail` — `String`.
    pub detail: String,
    /// `problems` — `List<conductor.config.ConfigProblem>`.
    pub problems: Vec<ConfigProblem>,
}

/// ConfigWritten — `conductor.config.ConfigWritten`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigWritten {
    /// `diff` — `String`.
    pub diff: String,
    /// `restart` — `Boolean`.
    pub restart: bool,
    /// `backup` — `Optional<String>`.
    pub backup: Option<String>,
    /// `decision_id` — `Optional<conductor.decision.DecisionId>`.
    pub decision_id: Option<crate::decision::DecisionId>,
    /// `problems` — `List<conductor.config.ConfigProblem>`.
    pub problems: Vec<ConfigProblem>,
}

/// Controllers — `conductor.config.Controllers`.
///
/// Every value satisfies `max_working >= 1` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `max_subagents >= 1` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `max_open_reviews >= 0` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Controllers {
    /// `max_working` — `Integer`.
    pub max_working: i64,
    /// `max_subagents` — `Integer`.
    pub max_subagents: i64,
    /// `max_open_reviews` — `Integer`.
    pub max_open_reviews: i64,
}

/// Daemon — `conductor.config.Daemon`.
///
/// Every value satisfies `port >= 1` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `port <= 65535` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `substrate_slots >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `distinct operator in operators by operator.name` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Daemon {
    /// `port` — `Integer`.
    pub port: i64,
    /// `loopback` — `Boolean`.
    pub loopback: bool,
    /// `tls` — `Optional<conductor.config.TlsListener>`.
    pub tls: Option<TlsListener>,
    /// `cadence` — `conductor.config.DaemonCadence`.
    pub cadence: DaemonCadence,
    /// `store` — `conductor.store.StoreUrl`.
    pub store: crate::store::StoreUrl,
    /// `secrets` — `conductor.config.SecretsSource`.
    pub secrets: SecretsSource,
    /// `dashboard` — `String`.
    pub dashboard: String,
    /// `operators` — `List<conductor.config.DaemonOperator>`.
    pub operators: Vec<DaemonOperator>,
    /// `upgrade_timeout` — `Duration`.
    pub upgrade_timeout: crate::primitives::Duration,
    /// `timeout` — `Duration`.
    pub timeout: crate::primitives::Duration,
    /// `substrate_slots` — `Integer`.
    pub substrate_slots: i64,
}

/// DaemonCadence — `conductor.config.DaemonCadence`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonCadence {
    /// `reconcile` — `Duration`.
    pub reconcile: crate::primitives::Duration,
}

/// DaemonOperator — `conductor.config.DaemonOperator`.
///
/// Every value satisfies `distinct grant in grants` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DaemonOperator {
    /// `name` — `conductor.operator.OperatorName`.
    pub name: crate::operator::OperatorName,
    /// `role` — `conductor.config.OperatorRole`.
    pub role: OperatorRole,
    /// `grants` — `List<conductor.config.OperatorGrant>`.
    pub grants: Vec<OperatorGrant>,
}

/// DoctorReport — `conductor.config.DoctorReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    /// `machine` — `conductor.config.Machine`.
    pub machine: Machine,
    /// `prerequisites` — `List<conductor.config.Prerequisite>`.
    pub prerequisites: Vec<Prerequisite>,
    /// `logins` — `List<conductor.config.ForgeLogin>`.
    pub logins: Vec<ForgeLogin>,
    /// `verdict` — `String`.
    pub verdict: String,
}

/// The states of `conductor.config.EffectiveSetting`, as runtime values.
///
/// Synthesised from the lifecycle, so the two cannot disagree. Which *moves* are legal is not
/// carried here — it is carried by `EffectiveSetting<S>`, where an undeclared move does not compile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectiveSettingState {
    /// `Effective`.
    Effective,
}

/// Forge — `conductor.config.Forge`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forge {
    /// `route` — `conductor.config.ForgeRoute`.
    pub route: ForgeRoute,
}

/// ForgeLogin — `conductor.config.ForgeLogin`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeLogin {
    /// `program` — `String`.
    pub program: String,
    /// `host` — `String`.
    pub host: String,
    /// `state` — `conductor.config.ForgeLoginState`.
    pub state: ForgeLoginState,
    /// `account` — `Optional<String>`.
    pub account: Option<String>,
}

/// ForgeLoginState — `conductor.config.ForgeLoginState`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeLoginState {
    /// `LoggedIn`.
    LoggedIn,
    /// `NotLoggedIn`.
    NotLoggedIn,
}

/// ForgeRoute — `conductor.config.ForgeRoute`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeRoute {
    /// `Gates`.
    Gates,
    /// `Direct`.
    Direct,
}

/// ForgeSecret — `conductor.config.ForgeSecret`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForgeSecret {
    /// `owner` — `String`.
    pub owner: String,
    /// `repository` — `Optional<String>`.
    pub repository: Option<String>,
    /// `name` — `String`.
    pub name: String,
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
    /// `host` — `String`.
    pub host: String,
    /// `exclude` — `List<String>`.
    pub exclude: Vec<String>,
}

/// Globals — `conductor.config.Globals`.
///
/// Every value satisfies `distinct role in roles by role.role` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Globals {
    /// `roles` — `List<conductor.config.Role>`.
    pub roles: Vec<Role>,
    /// `cadence` — `conductor.config.Cadence`.
    pub cadence: Cadence,
    /// `thresholds` — `conductor.config.Thresholds`.
    pub thresholds: Thresholds,
    /// `limits` — `conductor.config.Limits`.
    pub limits: Limits,
    /// `tuning` — `conductor.improvement.TuningGrant`.
    pub tuning: crate::improvement::TuningGrant,
}

/// Harness — `conductor.config.Harness`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Harness {
    /// `Claude`.
    Claude,
    /// `Codex`.
    Codex,
}

/// InitRecords — `conductor.config.InitRecords`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitRecords {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// Instance — `conductor.config.Instance`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    /// `name` — `conductor.config.InstanceName`.
    pub name: InstanceName,
    /// `placement` — `conductor.config.PlacementName`.
    pub placement: PlacementName,
    /// `remote` — `Optional<conductor.placement.RemoteDaemon>`.
    pub remote: Option<crate::placement::RemoteDaemon>,
    /// `session_prefix` — `Optional<conductor.config.SessionPrefix>`.
    pub session_prefix: Option<SessionPrefix>,
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
    /// `limits` — `conductor.config.Limits`.
    pub limits: Limits,
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
    /// `usage` — `conductor.config.Usage`.
    pub usage: Usage,
    /// `operator` — `Optional<String>`.
    pub operator: Option<String>,
    /// `catalog` — `Optional<conductor.config.Catalog>`.
    pub catalog: Option<Catalog>,
    /// `secret_syncs` — `List<conductor.config.SecretSync>`.
    pub secret_syncs: Vec<SecretSync>,
    /// `gates_policy` — `Optional<String>`.
    pub gates_policy: Option<String>,
    /// `model_token` — `Optional<conductor.secret.SecretName>`.
    pub model_token: Option<crate::secret::SecretName>,
    /// `forge` — `conductor.config.Forge`.
    pub forge: Forge,
    /// `conductor` — `conductor.config.ConductorSession`.
    pub conductor: ConductorSession,
}

/// InstanceName — `conductor.config.InstanceName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceName(pub String);

/// InstanceRefusal — `conductor.config.InstanceRefusal`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstanceRefusal {
    /// `AmbiguousInstance`.
    AmbiguousInstance,
    /// `StateDirNotInstance`.
    StateDirNotInstance,
    /// `ConfigUnloaded`.
    ConfigUnloaded,
}

/// InstanceRefused — `conductor.config.InstanceRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstanceRefused {
    /// `refusal` — `conductor.config.InstanceRefusal`.
    pub refusal: InstanceRefusal,
    /// `detail` — `String`.
    pub detail: String,
}

/// Level — `conductor.config.Level`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// `Default`.
    Default,
    /// `Global`.
    Global,
    /// `Instance`.
    Instance,
    /// `Role`.
    Role,
}

/// Limits — `conductor.config.Limits`.
///
/// Every value satisfies `(not (defined(cpu_weight)) or cpu_weight >= 1)` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(not (defined(cpu_weight)) or cpu_weight <= 10000)` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `(not (defined(tasks_max)) or tasks_max >= 1)` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Limits {
    /// `memory_max` — `Optional<conductor.config.Gibibytes>`.
    pub memory_max: Option<Gibibytes>,
    /// `cpu_weight` — `Optional<Integer>`.
    pub cpu_weight: Option<i64>,
    /// `tasks_max` — `Optional<Integer>`.
    pub tasks_max: Option<i64>,
}

/// LocalSource — `conductor.config.LocalSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocalSource {
    /// `path` — `String`.
    pub path: String,
    /// `exclude` — `List<String>`.
    pub exclude: Vec<String>,
}

/// Machine — `conductor.config.Machine`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    /// `host` — `String`.
    pub host: String,
    /// `system` — `String`.
    pub system: String,
    /// `release` — `String`.
    pub release: String,
    /// `architecture` — `String`.
    pub architecture: String,
    /// `conductor` — `String`.
    pub conductor: String,
}

/// MigrateConfig — `conductor.config.MigrateConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MigrateConfig {
    /// `check` — `Optional<Boolean>`.
    pub check: Option<bool>,
}

/// Network — `conductor.config.Network`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Network {
    /// `Open`.
    Open,
}

/// OperatorGrant — `conductor.config.OperatorGrant`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorGrant {
    /// `AnswerDecisions`.
    AnswerDecisions,
    /// `ChangeSettings`.
    ChangeSettings,
    /// `StartInstances`.
    StartInstances,
    /// `StopInstances`.
    StopInstances,
    /// `UpgradeInstances`.
    UpgradeInstances,
    /// `Purge`.
    Purge,
    /// `Join`.
    Join,
}

/// OperatorRole — `conductor.config.OperatorRole`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorRole(pub String);

/// PlacementName — `conductor.config.PlacementName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlacementName(pub String);

/// Prerequisite — `conductor.config.Prerequisite`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prerequisite {
    /// `program` — `String`.
    pub program: String,
    /// `state` — `conductor.config.PrerequisiteState`.
    pub state: PrerequisiteState,
    /// `version_line` — `Optional<String>`.
    pub version_line: Option<String>,
    /// `version` — `Optional<String>`.
    pub version: Option<String>,
    /// `pin` — `Optional<String>`.
    pub pin: Option<String>,
}

/// PrerequisiteState — `conductor.config.PrerequisiteState`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrerequisiteState {
    /// `Present`.
    Present,
    /// `Missing`.
    Missing,
    /// `OlderThanPin`.
    OlderThanPin,
    /// `VersionUnread`.
    VersionUnread,
    /// `NotNeeded`.
    NotNeeded,
}

/// RepoCache — `conductor.config.RepoCache`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepoCache {
    /// `On`.
    On,
    /// `Off`.
    Off,
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

/// RepositoryCheck — `conductor.config.RepositoryCheck`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryCheck {
    /// `check` — `String`.
    pub check: String,
    /// `ready` — `Boolean`.
    pub ready: bool,
    /// `detail` — `String`.
    pub detail: String,
    /// `fix` — `Optional<String>`.
    pub fix: Option<String>,
}

/// RepositoryRule — `conductor.config.RepositoryRule`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryRule {
    /// `match` — `String`.
    pub r#match: String,
    /// `activity` — `Optional<conductor.direction.Activity>`.
    pub activity: Option<crate::direction::Activity>,
    /// `no_releases` — `Optional<String>`.
    pub no_releases: Option<String>,
}

/// Retention — `conductor.config.Retention`.
///
/// Every value satisfies `snapshots >= 1` — declared here, enforced by whatever behaviour constructs one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Retention {
    /// `snapshots` — `Integer`.
    pub snapshots: i64,
}

/// RevisionId — `conductor.config.RevisionId`: a distinct wrapper around `Uuid`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevisionId(pub crate::primitives::Uuid);

/// Role — `conductor.config.Role`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Role {
    /// `role` — `String`.
    pub role: String,
    /// `kind` — `conductor.agent.AgentKindName`.
    pub kind: crate::agent::AgentKindName,
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
    /// `session_name` — `Optional<conductor.config.SessionName>`.
    pub session_name: Option<SessionName>,
    /// `read_only_tools` — `List<String>`.
    pub read_only_tools: Vec<String>,
    /// `limits` — `conductor.config.Limits`.
    pub limits: Limits,
    /// `writable` — `List<conductor.config.Writable>`.
    pub writable: Vec<Writable>,
    /// `network` — `conductor.config.Network`.
    pub network: Network,
    /// `forge_token_env` — `List<String>`.
    pub forge_token_env: Vec<String>,
    /// `grant` — `conductor.tool.RoleGrant`.
    pub grant: crate::tool::RoleGrant,
    /// `token_owner` — `Optional<conductor.operator.OperatorName>`.
    pub token_owner: Option<crate::operator::OperatorName>,
}

/// RoleName — `conductor.config.RoleName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoleName(pub String);

/// SecretSource — `conductor.config.SecretSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSource {
    /// `repository` — `String`.
    pub repository: String,
    /// `branch` — `String`.
    pub branch: String,
    /// `path` — `String`.
    pub path: String,
}

/// SecretSync — `conductor.config.SecretSync`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretSync {
    /// `name` — `String`.
    pub name: String,
    /// `source` — `conductor.config.SecretSource`.
    pub source: SecretSource,
    /// `secrets` — `List<conductor.config.ForgeSecret>`.
    pub secrets: Vec<ForgeSecret>,
    /// `command` — `String`.
    pub command: String,
    /// `dry_run` — `Optional<String>`.
    pub dry_run: Option<String>,
}

/// SecretsSource — `conductor.config.SecretsSource`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretsSource {
    /// `source` — `conductor.secret.Source`.
    pub source: crate::secret::Source,
    /// `namespace` — `Optional<String>`.
    pub namespace: Option<String>,
    /// `url` — `Optional<String>`.
    pub url: Option<String>,
}

/// SessionName — `conductor.config.SessionName`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionName(pub String);

/// SessionPrefix — `conductor.config.SessionPrefix`: a distinct wrapper around `String`.
///
/// Every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-`.
///
/// Checked by [`SessionPrefix::broken_constraint`]; the generated decoder refuses a value that breaks one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionPrefix(pub String);

impl SessionPrefix {
    /// The first constraint `conductor.config.SessionPrefix` declares that this value breaks — its alphabet, its prefix,
    /// then each invariant — as what a decoder expected instead, or `None` when it breaks none.
    ///
    /// An invariant is broken only when it is false of this value, as the conformance
    /// interpreter reads it. A wrapped newtype's own constraints are its own to check.
    pub fn broken_constraint(&self) -> Option<&'static str> {
        if !self.0.chars().all(|c| "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-".contains(c)) {
            return Some("a value whose every character is one of `ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-`");
        }
        None
    }
}

/// SettingKey — `conductor.config.SettingKey`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingKey(pub String);

/// SetupReport — `conductor.config.SetupReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupReport {
    /// `doctor` — `conductor.config.DoctorReport`.
    pub doctor: DoctorReport,
    /// `repository` — `String`.
    pub repository: String,
    /// `checks` — `List<conductor.config.RepositoryCheck>`.
    pub checks: Vec<RepositoryCheck>,
    /// `verdict` — `String`.
    pub verdict: String,
}

/// SetupRepository — `conductor.config.SetupRepository`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupRepository {
    /// `repository` — `Optional<String>`.
    pub repository: Option<String>,
    /// `apply` — `Optional<Boolean>`.
    pub apply: Option<bool>,
}

/// ShowConfig — `conductor.config.ShowConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowConfig {
    /// `format` — `Optional<String>`.
    pub format: Option<String>,
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
    /// `effective` — `Optional<Boolean>`.
    pub effective: Option<bool>,
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

/// SuggestedPrefix — `conductor.config.SuggestedPrefix`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuggestedPrefix {
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: InstanceName,
    /// `session_prefix` — `conductor.config.SessionPrefix`.
    pub session_prefix: SessionPrefix,
}

/// Thresholds — `conductor.config.Thresholds`.
///
/// Every value satisfies `gate_resume > gate_stop` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `cpu_reserve >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `build_slots >= 0` — declared here, enforced by whatever behaviour constructs one.
/// Every value satisfies `short_lane_slots >= 0` — declared here, enforced by whatever behaviour constructs one.
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
    /// `silent_session` — `Duration`.
    pub silent_session: crate::primitives::Duration,
    /// `cpu_reserve` — `Integer`.
    pub cpu_reserve: i64,
    /// `build_slots` — `Integer`.
    pub build_slots: i64,
    /// `repo_cache` — `conductor.config.RepoCache`.
    pub repo_cache: RepoCache,
    /// `repo_cache_size` — `conductor.config.Gibibytes`.
    pub repo_cache_size: Gibibytes,
    /// `lean_debuginfo` — `Boolean`.
    pub lean_debuginfo: bool,
    /// `short_lane_slots` — `Integer`.
    pub short_lane_slots: i64,
    /// `short_lane_max` — `Duration`.
    pub short_lane_max: crate::primitives::Duration,
    /// `stalled_session` — `Duration`.
    pub stalled_session: crate::primitives::Duration,
    /// `build_queued` — `Duration`.
    pub build_queued: crate::primitives::Duration,
    /// `tree_stale` — `Duration`.
    pub tree_stale: crate::primitives::Duration,
}

/// TimeOfDay — `conductor.config.TimeOfDay`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimeOfDay(pub String);

/// TlsListener — `conductor.config.TlsListener`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TlsListener {
    /// `listen` — `String`.
    pub listen: String,
    /// `cert` — `Optional<String>`.
    pub cert: Option<String>,
    /// `key` — `Optional<String>`.
    pub key: Option<String>,
    /// `identity` — `Optional<String>`.
    pub identity: Option<String>,
}

/// Tokens — `conductor.config.Tokens`: a distinct wrapper around `Integer`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tokens(pub i64);

/// TrustWorkspaces — `conductor.config.TrustWorkspaces`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrustWorkspaces {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// UpgradeBinary — `conductor.config.UpgradeBinary`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeBinary {
    /// `check` — `Optional<Boolean>`.
    pub check: Option<bool>,
    /// `release` — `Optional<String>`.
    pub release: Option<String>,
}

/// UpgradeOutcome — `conductor.config.UpgradeOutcome`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeOutcome {
    /// `UpgradeAvailable`.
    UpgradeAvailable,
    /// `NoneAvailable`.
    NoneAvailable,
    /// `Upgraded`.
    Upgraded,
    /// `AlreadyCurrent`.
    AlreadyCurrent,
}

/// UpgradeRefusal — `conductor.config.UpgradeRefusal`: one of a closed set of names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeRefusal {
    /// `ReleaseUnread`.
    ReleaseUnread,
    /// `NoArchiveForTarget`.
    NoArchiveForTarget,
    /// `ChecksumMismatch`.
    ChecksumMismatch,
    /// `NewBinaryDoesNotRun`.
    NewBinaryDoesNotRun,
    /// `BinaryNotWritable`.
    BinaryNotWritable,
}

/// UpgradeRefused — `conductor.config.UpgradeRefused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeRefused {
    /// `release` — `String`.
    pub release: String,
    /// `target` — `String`.
    pub target: String,
    /// `refusal` — `conductor.config.UpgradeRefusal`.
    pub refusal: UpgradeRefusal,
    /// `detail` — `String`.
    pub detail: String,
}

/// UpgradeReport — `conductor.config.UpgradeReport`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpgradeReport {
    /// `installed` — `String`.
    pub installed: String,
    /// `release` — `String`.
    pub release: String,
    /// `outcome` — `conductor.config.UpgradeOutcome`.
    pub outcome: UpgradeOutcome,
    /// `binary` — `String`.
    pub binary: String,
}

/// Usage — `conductor.config.Usage`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage {
    /// `probe` — `Optional<String>`.
    pub probe: Option<String>,
    /// `watcher` — `Optional<String>`.
    pub watcher: Option<String>,
    /// `rotate` — `Optional<String>`.
    pub rotate: Option<String>,
}

/// ValidateConfig — `conductor.config.ValidateConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidateConfig {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
}

/// WorkspaceMember — `conductor.config.WorkspaceMember`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceMember {
    /// `name` — `String`.
    pub name: String,
    /// `source` — `String`.
    pub source: String,
}

/// Writable — `conductor.config.Writable`: a distinct wrapper around `String`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Writable(pub String);

/// WriteConfig — `conductor.config.WriteConfig`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteConfig {
    /// `changes` — `List<conductor.config.ConfigChange>`.
    pub changes: Vec<ConfigChange>,
    /// `activity` — `List<conductor.config.ActivityChange>`.
    pub activity: Vec<ActivityChange>,
    /// `reason` — `Optional<String>`.
    pub reason: Option<String>,
    /// `check` — `Optional<Boolean>`.
    pub check: Option<bool>,
}

/// WriteWorkspace — `conductor.config.WriteWorkspace`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WriteWorkspace {
    /// `instance` — `Optional<String>`.
    pub instance: Option<String>,
    /// `check` — `Optional<Boolean>`.
    pub check: Option<bool>,
}

/// What ConfigRevision — `conductor.config.ConfigRevision` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`ConfigRevision<S>`], and at a boundary by [`ConfigRevisionSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRevisionData {
    /// The identity: `revision_id` — `conductor.config.RevisionId`.
    pub revision_id: RevisionId,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `level` — `conductor.config.Level`.
    pub level: Level,
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<InstanceName>,
    /// `role` — `Optional<conductor.config.RoleName>`.
    pub role: Option<RoleName>,
    /// `path` — `String`.
    pub path: String,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `reason` — `String`.
    pub reason: String,
    /// `commit` — `String`.
    pub commit: String,
    /// `changed_at` — `Timestamp`.
    pub changed_at: crate::primitives::Timestamp,
}

/// The states of `conductor.config.ConfigRevision`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](config_revision_state::Marker), so [`ConfigRevision<S>`](ConfigRevision) can only ever rest in a real state.
pub mod config_revision_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Committed {}
    }

    /// A declared state of `ConfigRevision`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::ConfigRevisionState;
    }

    /// `Committed`. Where a new instance starts.
    pub struct Committed;

    impl Marker for Committed {
        const STATE: super::ConfigRevisionState = super::ConfigRevisionState::Committed;
    }
}

/// ConfigRevision — `conductor.config.ConfigRevision` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Committed`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`ConfigRevisionSnapshot`]
/// and [`ConfigRevisionSnapshot::refine`].
pub struct ConfigRevision<S: config_revision_state::Marker> {
    data: ConfigRevisionData,
    state: core::marker::PhantomData<S>,
}

impl<S: config_revision_state::Marker> ConfigRevision<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> ConfigRevisionState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &ConfigRevisionData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> ConfigRevisionData {
        self.data
    }
}

impl ConfigRevision<config_revision_state::Committed> {
    /// A new instance, resting in `Committed` — the only state the lifecycle starts one in.
    pub fn new(data: ConfigRevisionData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.config.ConfigRevision` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`ConfigRevisionSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRevisionSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: ConfigRevisionState,
    /// What it holds.
    pub data: ConfigRevisionData,
}

/// An `ConfigRevision` in whichever declared state it was found.
pub enum AnyConfigRevision {
    /// Resting in `Committed`.
    Committed(ConfigRevision<config_revision_state::Committed>),
}

impl ConfigRevisionSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `ConfigRevisionState` cannot spell one.
    pub fn refine(self) -> AnyConfigRevision {
        match self.state {
            ConfigRevisionState::Committed => AnyConfigRevision::Committed(ConfigRevision {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyConfigRevision {
    /// The state, as the runtime value.
    pub fn state(&self) -> ConfigRevisionState {
        match self {
            Self::Committed(_) => ConfigRevisionState::Committed,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> ConfigRevisionSnapshot {
        match self {
            Self::Committed(instance) => ConfigRevisionSnapshot {
                state: ConfigRevisionState::Committed,
                data: instance.into_data(),
            },
        }
    }
}

/// What EffectiveSetting — `conductor.config.EffectiveSetting` — holds, apart from where it is in its lifecycle.
///
/// The identity and every declared field. The state is deliberately not one: inside the domain it
/// is carried by the type parameter of [`EffectiveSetting<S>`], and at a boundary by [`EffectiveSettingSnapshot::state`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveSettingData {
    /// The identity: `setting_key` — `conductor.config.SettingKey`.
    pub setting_key: SettingKey,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: InstanceName,
    /// `role` — `Optional<conductor.config.RoleName>`.
    pub role: Option<RoleName>,
    /// `path` — `String`.
    pub path: String,
    /// `value` — `String`.
    pub value: String,
    /// `level` — `conductor.config.Level`.
    pub level: Level,
}

/// The states of `conductor.config.EffectiveSetting`, at the type level.
///
/// One marker type per declared state, sealed: a state the lifecycle does not declare cannot
/// implement [`Marker`](effective_setting_state::Marker), so [`EffectiveSetting<S>`](EffectiveSetting) can only ever rest in a real state.
pub mod effective_setting_state {
    /// Closes [`Marker`] over the declared states.
    mod sealed {
        /// Implemented only by the marker types beside this module.
        pub trait Sealed {}
        impl Sealed for super::Effective {}
    }

    /// A declared state of `EffectiveSetting`, as a type.
    pub trait Marker: sealed::Sealed {
        /// The same state, as the runtime value.
        const STATE: super::EffectiveSettingState;
    }

    /// `Effective`. Where a new instance starts.
    pub struct Effective;

    impl Marker for Effective {
        const STATE: super::EffectiveSettingState = super::EffectiveSettingState::Effective;
    }
}

/// EffectiveSetting — `conductor.config.EffectiveSetting` — with its lifecycle state carried by the type.
///
/// The one constructor rests in `Effective`, and the only way to change `S` is a method generated from
/// a declared transition. A move the specification does not declare is therefore not an error
/// case: it does not compile. Where the state is data — wire, storage — use [`EffectiveSettingSnapshot`]
/// and [`EffectiveSettingSnapshot::refine`].
pub struct EffectiveSetting<S: effective_setting_state::Marker> {
    data: EffectiveSettingData,
    state: core::marker::PhantomData<S>,
}

impl<S: effective_setting_state::Marker> EffectiveSetting<S> {
    /// The state this instance rests in, as the runtime value.
    pub fn state(&self) -> EffectiveSettingState {
        S::STATE
    }

    /// What it holds.
    pub fn data(&self) -> &EffectiveSettingData {
        &self.data
    }

    /// Hands the data back, giving up the typed state.
    pub fn into_data(self) -> EffectiveSettingData {
        self.data
    }
}

impl EffectiveSetting<effective_setting_state::Effective> {
    /// A new instance, resting in `Effective` — the only state the lifecycle starts one in.
    pub fn new(data: EffectiveSettingData) -> Self {
        Self {
            data,
            state: core::marker::PhantomData,
        }
    }
}

/// `conductor.config.EffectiveSetting` as it crosses a boundary: the state as a value beside the data.
///
/// Wire and storage know states only at runtime; [`EffectiveSettingSnapshot::refine`] is the one door back
/// into the typed lifecycle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveSettingSnapshot {
    /// Where the instance is in its lifecycle.
    pub state: EffectiveSettingState,
    /// What it holds.
    pub data: EffectiveSettingData,
}

/// An `EffectiveSetting` in whichever declared state it was found.
pub enum AnyEffectiveSetting {
    /// Resting in `Effective`.
    Effective(EffectiveSetting<effective_setting_state::Effective>),
}

impl EffectiveSettingSnapshot {
    /// Refines the runtime state into the typed one.
    ///
    /// Total: every declared state has an arm, and an undeclared state cannot reach here because
    /// `EffectiveSettingState` cannot spell one.
    pub fn refine(self) -> AnyEffectiveSetting {
        match self.state {
            EffectiveSettingState::Effective => AnyEffectiveSetting::Effective(EffectiveSetting {
                data: self.data,
                state: core::marker::PhantomData,
            }),
        }
    }
}

impl AnyEffectiveSetting {
    /// The state, as the runtime value.
    pub fn state(&self) -> EffectiveSettingState {
        match self {
            Self::Effective(_) => EffectiveSettingState::Effective,
        }
    }

    /// Back to the boundary shape.
    pub fn snapshot(self) -> EffectiveSettingSnapshot {
        match self {
            Self::Effective(instance) => EffectiveSettingSnapshot {
                state: EffectiveSettingState::Effective,
                data: instance.into_data(),
            },
        }
    }
}

/// Change the config — the input of `conductor.config.PutConfig`.
///
/// Everything it can result in is [`PutConfigOutcome`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PutConfig {
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `level` — `conductor.config.Level`.
    pub level: Level,
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<InstanceName>,
    /// `role` — `Optional<conductor.config.RoleName>`.
    pub role: Option<RoleName>,
    /// `path` — `String`.
    pub path: String,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `reason` — `String`.
    pub reason: String,
    /// `changed_at` — `Timestamp`.
    pub changed_at: crate::primitives::Timestamp,
}

/// Everything `conductor.config.PutConfig` can result in — one variant per declared outcome.
///
/// An infrastructure failure is deliberately not in here: a refusal is a fact about the domain,
/// a transport fault is a fact about the run, and conflating the two is what the declared
/// outcomes exist to prevent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PutConfigOutcome {
    /// `level-scope-mismatch` — when `(level == Default or (level == Global and defined(instance)) or (level == Global and defined(role)) or (level == Instance and not (defined(instance))) or (level == Instance and defined(role)) or (level == Role and not (defined(instance))) or (level == Role and not (defined(role))))`.
    ///
    /// The level and the instance or role named do not fit; nothing was written.
    LevelScopeMismatch {
        /// Why it was refused: `conductor.config.LevelScopeMismatch`.
        error: LevelScopeMismatch,
    },
    /// `actor-not-human` — externally decided (the change lies outside the tuning grant) when eligible (actor.kind != Human).
    ///
    /// An agent's change of a key the tuning grant does not name, or to a value outside its range, is refused; nothing was written.
    ActorNotHuman {
        /// Why it was refused: `conductor.operator.ActorNotHuman`.
        error: crate::operator::ActorNotHuman,
    },
    /// `invalid` — externally decided (the config with the change does not validate).
    ///
    /// The changed config does not validate; nothing was written.
    Invalid {
        /// Why it was refused: `conductor.config.ConfigInvalid`.
        error: ConfigInvalid,
    },
    /// `changed` — otherwise.
    ///
    /// The key is written at its level and committed with the actor and the reason; the daemon reconciles the instances the change touches.
    Changed {
        /// The `conductor.config.ConfigChanged` this outcome publishes.
        config_changed: ConfigChanged,
    },
}

/// ConfigChanged — the event `conductor.config.ConfigChanged`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigChanged {
    /// `revision_id` — `conductor.config.RevisionId`.
    pub revision_id: RevisionId,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `level` — `conductor.config.Level`.
    pub level: Level,
    /// `instance_name` — `Optional<conductor.config.InstanceName>`.
    pub instance_name: Option<InstanceName>,
    /// `role` — `Optional<conductor.config.RoleName>`.
    pub role: Option<RoleName>,
    /// `path` — `String`.
    pub path: String,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `reason` — `String`.
    pub reason: String,
}

/// The declared error `conductor.config.ConfigInvalid`.
///
/// The config with this change does not validate; nothing was written, committed or reconciled, and the problem is named by its YAML path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigInvalid {
    /// `path` — `String`.
    pub path: String,
}

/// The declared error `conductor.config.LevelScopeMismatch`.
///
/// A `Global` change names no instance and no role, an `Instance` change names an instance and no role, a `Role` change names both, and no change is written at `Default`; this one does not fit, and nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LevelScopeMismatch {
    /// `level` — `conductor.config.Level`.
    pub level: Level,
}

/// The declared error `conductor.config.StateDirNotInstance`.
///
/// `--state-dir` names a directory that is not the state of the instance the config file names, and the command writes that instance's records; nothing was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateDirNotInstance {
    /// `state_dir` — `String`.
    pub state_dir: String,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: InstanceName,
    /// `state` — `String`.
    pub state: String,
}

/// Config changes — one row of the view `conductor.config.ConfigRevisions`.
///
/// Projects `conductor.config.ConfigRevision` at `read_your_writes` consistency.
/// The specification fully determines every row, so its query is generated over the storage port —
/// see the plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigRevisions {
    /// `revision_id` — `conductor.config.RevisionId`.
    pub revision_id: RevisionId,
    /// `state` — `conductor.config.ConfigRevision.State`.
    pub state: ConfigRevisionState,
    /// `actor` — `conductor.operator.Actor`.
    pub actor: crate::operator::Actor,
    /// `level` — `conductor.config.Level`.
    pub level: Level,
    /// `instance` — `Optional<conductor.config.InstanceName>`.
    pub instance: Option<InstanceName>,
    /// `role` — `Optional<conductor.config.RoleName>`.
    pub role: Option<RoleName>,
    /// `path` — `String`.
    pub path: String,
    /// `value` — `Optional<String>`.
    pub value: Option<String>,
    /// `reason` — `String`.
    pub reason: String,
    /// `commit` — `String`.
    pub commit: String,
    /// `changed_at` — `Timestamp`.
    pub changed_at: crate::primitives::Timestamp,
}

/// Effective settings — one row of the view `conductor.config.EffectiveSettings`.
///
/// Projects `conductor.config.EffectiveSetting` at `eventual` consistency, containing instances where `instance == param.instance`.
/// Serving it is an implementation obligation — see the plan — because how a projection is kept
/// current is a storage decision the specification does not take.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveSettings {
    /// `setting_key` — `conductor.config.SettingKey`.
    pub setting_key: SettingKey,
    /// `state` — `conductor.config.EffectiveSetting.State`.
    pub state: EffectiveSettingState,
    /// `instance` — `conductor.config.InstanceName`.
    pub instance: InstanceName,
    /// `role` — `Optional<conductor.config.RoleName>`.
    pub role: Option<RoleName>,
    /// `path` — `String`.
    pub path: String,
    /// `value` — `String`.
    pub value: String,
    /// `level` — `conductor.config.Level`.
    pub level: Level,
}

/// What this bounded context owes its implementor, and the seams of what is generated.
///
/// One trait per obligation in the synthesis plan, each carrying the plan's own contract, and one
/// per generated behaviour, which [`Generated`](crate::behaviour::Generated) implements.
/// [`Unimplemented`](obligations::Unimplemented) satisfies every owed trait by refusing in the type system.
pub mod obligations {
    /// The behaviour `conductor.config.PutConfig` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage and context ports. Implement it yourself to replace that behaviour.
    pub trait PutConfigBehavior {
        /// Decides and enacts exactly one declared outcome of `conductor.config.PutConfig`.
        ///
        /// `Err` is the typed refusal of a request the model declares no outcome for.
        fn put_config(&mut self, input: super::PutConfig) -> Result<super::PutConfigOutcome, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.config.ConfigRevisions` — generated.
    ///
    /// The specification fully determines it: [`crate::behaviour::Generated`] implements it
    /// over the storage port. Implement it yourself to replace that query.
    pub trait ConfigRevisionsQuery {
        /// Serves `conductor.config.ConfigRevisions` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of a row whose declared type cannot hold its value.
        fn config_revisions(&self) -> Result<Vec<super::ConfigRevisions>, crate::obligation::UnmetObligation>;
    }

    /// The query `conductor.config.EffectiveSettings` — an implementation obligation.
    ///
    /// Why it is not generated: kept an obligation by a view parameter (`params:`), which a generated query does not apply.
    ///
    /// Contract: a query answering `conductor.config.EffectiveSettings` with rows projected from `conductor.config.EffectiveSetting` at `eventual` consistency, containing instances where `instance == param.instance`.
    pub trait EffectiveSettingsQuery {
        /// Serves `conductor.config.EffectiveSettings` rows at the view's declared consistency.
        ///
        /// `Err` is the typed refusal of an obligation nothing has satisfied; a satisfying
        /// implementation never returns it.
        fn effective_settings(&self, instance: crate::config::InstanceName) -> Result<Vec<super::EffectiveSettings>, crate::obligation::UnmetObligation>;
    }

    /// Every obligation of this bounded context, refused in the type system.
    ///
    /// Each method returns the typed refusal naming what is owed — never a panic, never a guessed
    /// value — so a workspace built on this stub compiles and reports its own gaps.
    pub struct Unimplemented;

    impl EffectiveSettingsQuery for Unimplemented {
        fn effective_settings(&self, _instance: crate::config::InstanceName) -> Result<Vec<super::EffectiveSettings>, crate::obligation::UnmetObligation> {
            Err(crate::obligation::UnmetObligation { capability: "view query", source: "conductor.config.EffectiveSettings" })
        }
    }
}
