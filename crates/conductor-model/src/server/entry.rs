// generated from conductor v1
// contract digest 0608eb83c8c48e1f8808cd29d3621f78091b34cac43c2b2e9143a335707a4202
// do not edit: regenerate with `ess synthesize --layout crate`

//! The transport-free entry point's refusals.
//!
//! Each served component's `handle` runs a command or view by its qualified name, through the
//! same code the HTTP routes run. What it cannot answer with a declared outcome it answers with a
//! [`Refused`]: the same refusal a route answers, without the status, which [`Refused::status`]
//! still names for a caller that serves it.

/// Why `handle` answered no declared outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refused {
    /// The surface declares no command and no view by this qualified name.
    Unknown(String),
    /// The input is not the command's declared input; the route answers this `400`.
    Input(String),
    /// A port reported an unmet obligation, and nothing was written; the route answers this
    /// `501`, which the contract declares, with `committed: false`.
    Unmet(String),
    /// The command's effect and events were committed, and delivering what it published to a
    /// binding failed; the route answers this `501` with `committed: true`.
    ///
    /// The detail begins `delivering what the command published`. Not to be retried: a retry
    /// performs the command twice.
    Undelivered(String),
    /// The caller is no actor, or one the specification does not grant the command; checked
    /// before the command runs, and the route answers this `403` with the standard refusal,
    /// `{"refused": "not granted", "actor": <name or null>}`. Carries the actor's qualified name,
    /// `None` where the call was authenticated as no actor.
    NotGranted(Option<String>),
}

impl Refused {
    /// The status the HTTP surface answers this refusal with.
    pub fn status(&self) -> u16 {
        match self {
            Self::Unknown(_) => 404,
            Self::Input(_) => 400,
            Self::Unmet(_) | Self::Undelivered(_) => 501,
            Self::NotGranted(_) => 403,
        }
    }

    /// `true` when the command's effect was committed before the refusal: the `501` body's
    /// `committed` member.
    pub fn committed(&self) -> bool {
        matches!(self, Self::Undelivered(_))
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unknown(name) => write!(
                f,
                "`{name}` is not a command or view this surface declares"
            ),
            Self::Input(detail) | Self::Unmet(detail) | Self::Undelivered(detail) => {
                f.write_str(detail)
            }
            Self::NotGranted(Some(actor)) => write!(f, "not granted: `{actor}`"),
            Self::NotGranted(None) => f.write_str("not granted: no actor"),
        }
    }
}

impl std::error::Error for Refused {}

/// A rendered outcome, read back as the value it renders.
///
/// The body is written by this crate's own encoders; reading it back, rather than building the
/// value a second way, keeps one rendering of every outcome. It is read with
/// [`crate::server::json::parse_written`], which has no depth limit: a `Json` value intake admitted sits
/// deeper in the answer than it arrived.
///
/// # Errors
///
/// The reader's refusal when the answer is not JSON, which only a `Json` value the realization
/// built can make it: a number spelled the way JSON does not spell one.
pub(crate) fn read(body: &str) -> Result<crate::server::json::Value, crate::server::json::ParseError> {
    crate::server::json::parse_written(body)
}
