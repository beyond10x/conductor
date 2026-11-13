//! The two dispatch behaviours the plan owes rather than generates (`crates/conductor-model/PLAN.md`),
//! over the store: `conductor.dispatch.SendDispatch`, kept an obligation by the length of its
//! `reference` and by the rows its row-set guards read, and `conductor.dispatch.RecordDispatchLink`,
//! kept one by the length of its `link` (`story:control-plane-work-handler`).
//!
//! Each decides its outcome in the order the specification's one precedence rule gives: the
//! refusals the input alone decides, in declaration order; then the addressed record's existence;
//! then what it holds; then the rows around it, in declaration order; then the accepting
//! branches.

use conductor_model::behaviour::{DispatchStorage, GoalStorage};
use conductor_model::direction::GoalState;
use conductor_model::dispatch::obligations::{RecordDispatchLinkBehavior, SendDispatchBehavior};
use conductor_model::dispatch::{
    AnyDispatch, Dispatch, DispatchData, DispatchExists, DispatchLinked, DispatchLinksFull,
    DispatchNotFound, DispatchSent, DispatchState, GoalClosed, GoalNotFound,
    GoalOrReferenceMissing, InvalidLink, InvalidPriority, InvalidReference, RecordDispatchLink,
    RecordDispatchLinkOutcome, ReferenceHeld, RepositoryBusy, SendDispatch, SendDispatchOutcome,
    WaitsOnItself,
};
use conductor_model::obligation::UnmetObligation;

use crate::store::Store;

/// The most characters a reference holds.
pub const REFERENCE_MOST: usize = 64;

/// The most characters a link holds.
pub const LINK_MOST: usize = 2048;

/// The most links a dispatch holds.
pub const LINKS_MOST: usize = 32;

/// Whether `reference` is one a dispatch may carry: 1 to [`REFERENCE_MOST`] characters of
/// `A-Z a-z 0-9 . _ -`, the first a letter or a digit (`SendDispatch` `invalid-reference`).
#[must_use]
pub fn valid_reference(reference: &str) -> bool {
    let allowed =
        |character: char| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-');
    let count = reference.chars().count();
    (1..=REFERENCE_MOST).contains(&count)
        && reference.chars().all(allowed)
        && reference
            .chars()
            .next()
            .is_some_and(|first| first.is_ascii_alphanumeric())
}

/// Whether `link` is one a dispatch may hold: an `http://` or `https://` URL, more than its
/// scheme, of at most [`LINK_MOST`] characters, with no space, no control character (C0, DEL and
/// C1, so no tab or line break), no line or paragraph separator and no bidirectional control
/// ([`crate::cli::bidirectional`]): nothing that moves or reorders a terminal that shows it
/// (`RecordDispatchLink` `invalid-link`).
#[must_use]
pub fn valid_link(link: &str) -> bool {
    let refused = |character: char| {
        character == ' '
            || character.is_control()
            || matches!(character, '\u{2028}' | '\u{2029}')
            || crate::cli::bidirectional(character)
    };
    link.chars().count() <= LINK_MOST
        && (link.starts_with("http://") || link.starts_with("https://"))
        && link != "http://"
        && link != "https://"
        && !link.contains(refused)
}

/// The most bytes a `start_ref` holds.
pub const START_REF_MOST: usize = 255;

/// The most bytes an `issue` holds.
pub const ISSUE_MOST: usize = 2048;

/// What a `start_ref` is, as a refusal names it.
pub const START_REF_RULE: &str = "a start_ref is 1 to 255 bytes of a git ref name: no leading -, \
     no control character (C0 or C1), line or paragraph separator, bidirectional control, \
     backtick or space, none of ~ ^ : ? * [ \\, no .., no @{, no leading or trailing /, no //, no \
     trailing ., no path step that starts with . or ends in .lock, not @ alone";

/// What an `issue` is, as a refusal names it.
pub const ISSUE_RULE: &str = "an issue is 1 to 2048 bytes with no control character (C0 or C1), \
     line or paragraph separator, bidirectional control or backtick";

/// Whether `character` could break the frame a controller's first prompt writes a member in
/// (`issue`, `start_ref`), each on its own line after a fixed label: a control character (C0 and
/// C1), U+2028, U+2029, a bidirectional control ([`crate::cli::bidirectional`]) or a backtick.
fn breaks_the_frame(character: char) -> bool {
    character.is_control()
        || matches!(character, '\u{2028}' | '\u{2029}' | '`')
        || crate::cli::bidirectional(character)
}

/// The longest run of backticks a brief may hold: the fence a controller's first prompt writes
/// the brief in is one longer, and stays a small part of the one argument the prompt is.
pub const BRIEF_BACKTICKS_MOST: usize = 255;

/// Why `brief` is no brief a controller can be started with, when it is none: it holds a NUL,
/// which no argument of a process can carry, or a run of more than [`BRIEF_BACKTICKS_MOST`]
/// backticks. The work call and `dispatch send-dispatch` refuse it as `InvalidBrief`. Its length
/// is the work call's to bound.
#[must_use]
pub fn refused_brief(brief: &str) -> Option<String> {
    if brief.contains('\0') {
        return Some("the brief holds a NUL character, which no session can be given".to_owned());
    }
    let longest = brief.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    (longest > BRIEF_BACKTICKS_MOST).then(|| {
        format!(
            "the brief holds a run of {longest} backticks; a brief holds runs of at most \
             {BRIEF_BACKTICKS_MOST}"
        )
    })
}

/// Whether `start_ref` is one a dispatch may carry: 1 to [`START_REF_MOST`] bytes that `git
/// check-ref-format --allow-onelevel` admits in substance, decided here without running git
/// ([`START_REF_RULE`]).
#[must_use]
pub fn valid_start_ref(start_ref: &str) -> bool {
    let refused = |character: char| {
        breaks_the_frame(character)
            || matches!(character, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\')
    };
    (1..=START_REF_MOST).contains(&start_ref.len())
        && start_ref != "@"
        && !start_ref.starts_with(['-', '/'])
        && !start_ref.ends_with(['/', '.'])
        && !start_ref.contains("..")
        && !start_ref.contains("@{")
        && !start_ref.contains("//")
        && !start_ref.contains(refused)
        && start_ref
            .split('/')
            .all(|step| !step.starts_with('.') && !step.ends_with(".lock"))
}

/// Whether `issue` is one a dispatch may carry: 1 to [`ISSUE_MOST`] bytes with no character that
/// breaks the frame ([`ISSUE_RULE`]); a `#`, as in `#123`, is taken.
#[must_use]
pub fn valid_issue(issue: &str) -> bool {
    (1..=ISSUE_MOST).contains(&issue.len()) && !issue.contains(breaks_the_frame)
}

/// The first of `start_ref` and `issue` a dispatch does not carry, by its member's name, with the
/// rule it breaks; `None` when each is absent or taken. The work call and `dispatch
/// send-dispatch` refuse it as `InvalidWorkRequest`.
#[must_use]
pub fn refused_member(
    start_ref: Option<&str>,
    issue: Option<&str>,
) -> Option<(&'static str, &'static str)> {
    if start_ref.is_some_and(|start_ref| !valid_start_ref(start_ref)) {
        return Some(("start_ref", START_REF_RULE));
    }
    if issue.is_some_and(|issue| !valid_issue(issue)) {
        return Some(("issue", ISSUE_RULE));
    }
    None
}

/// Whether a dispatch is open: sent, started or blocked.
const fn open(state: DispatchState) -> bool {
    matches!(
        state,
        DispatchState::Sent | DispatchState::Started | DispatchState::Blocked
    )
}

/// `conductor.dispatch.SendDispatch` over the store.
impl SendDispatchBehavior for Store {
    fn send_dispatch(
        &mut self,
        input: SendDispatch,
    ) -> Result<SendDispatchOutcome, UnmetObligation> {
        send_dispatch(self, input)
    }
}

/// `conductor.dispatch.RecordDispatchLink` over the store.
impl RecordDispatchLinkBehavior for Store {
    fn record_dispatch_link(
        &mut self,
        input: RecordDispatchLink,
    ) -> Result<RecordDispatchLinkOutcome, UnmetObligation> {
        record_dispatch_link(self, input)
    }
}

/// `conductor.dispatch.SendDispatch`, as its contract in `crates/conductor-model/PLAN.md` states it,
/// over any ports that hold dispatches and goals.
///
/// # Errors
///
/// The dispatch it would create breaks an invariant of its entity.
pub fn send_dispatch<P: DispatchStorage + GoalStorage>(
    ports: &mut P,
    input: SendDispatch,
) -> Result<SendDispatchOutcome, UnmetObligation> {
    // The request alone, in declaration order.
    if input.priority < 0 {
        return Ok(SendDispatchOutcome::NegativePriority {
            error: InvalidPriority {
                priority: input.priority,
            },
        });
    }
    if input.waits_on.as_ref() == Some(&input.dispatch_id) {
        return Ok(SendDispatchOutcome::WaitsOnItself {
            error: WaitsOnItself {
                dispatch_id: input.dispatch_id,
            },
        });
    }
    if let Some(reference) = &input.reference
        && !valid_reference(reference)
    {
        return Ok(SendDispatchOutcome::InvalidReference {
            error: InvalidReference {
                reference: reference.clone(),
            },
        });
    }
    if input.goal_id.is_none() && input.reference.is_none() {
        return Ok(SendDispatchOutcome::GoalOrReferenceMissing {
            error: GoalOrReferenceMissing {
                repository: input.repository,
            },
        });
    }
    // The addressed record's existence.
    if DispatchStorage::get(ports, &input.dispatch_id).is_some() {
        return Ok(SendDispatchOutcome::AlreadySent {
            error: DispatchExists {
                dispatch_id: input.dispatch_id,
            },
        });
    }
    // The rows around it, in declaration order.
    let dispatches = DispatchStorage::list(ports);
    if let Some(reference) = &input.reference
        && dispatches
            .iter()
            .any(|held| held.data.reference.as_ref() == Some(reference))
    {
        return Ok(SendDispatchOutcome::ReferenceHeld {
            error: ReferenceHeld {
                reference: reference.clone(),
            },
        });
    }
    if let Some(goal_id) = &input.goal_id {
        let Some(goal) = GoalStorage::get(ports, goal_id) else {
            return Ok(SendDispatchOutcome::NoSuchGoal {
                error: GoalNotFound {
                    goal_id: goal_id.clone(),
                },
            });
        };
        if matches!(goal.state, GoalState::Met | GoalState::Dropped) {
            return Ok(SendDispatchOutcome::GoalClosed {
                error: GoalClosed {
                    goal_id: goal_id.clone(),
                },
            });
        }
    }
    if input.reference.is_some()
        && dispatches
            .iter()
            .any(|held| held.data.repository == input.repository && open(held.state))
    {
        return Ok(SendDispatchOutcome::RepositoryBusy {
            error: RepositoryBusy {
                repository: input.repository,
            },
        });
    }
    // The accepting branches: `sent-for-reference` without a goal, `sent` with one.
    let data = DispatchData {
        dispatch_id: input.dispatch_id.clone(),
        actor: input.actor.clone(),
        repository: input.repository.clone(),
        goal_id: input.goal_id.clone(),
        brief: input.brief,
        sent_at: input.sent_at,
        priority: input.priority,
        waits_on: input.waits_on,
        reference: input.reference.clone(),
        start_ref: input.start_ref,
        issue: input.issue,
        links: Vec::new(),
    };
    if let Some(broken) = data.broken_invariant() {
        return Err(UnmetObligation {
            capability: "entity invariant",
            source: broken,
        });
    }
    let dispatch_sent = DispatchSent {
        actor: input.actor,
        dispatch_id: input.dispatch_id,
        repository: input.repository,
        goal_id: input.goal_id,
        reference: input.reference,
    };
    DispatchStorage::put(ports, AnyDispatch::Sent(Dispatch::new(data)).snapshot());
    Ok(if dispatch_sent.goal_id.is_none() {
        SendDispatchOutcome::SentForReference { dispatch_sent }
    } else {
        SendDispatchOutcome::Sent { dispatch_sent }
    })
}

/// `conductor.dispatch.RecordDispatchLink`, as its contract in `crates/conductor-model/PLAN.md`
/// states it, over any ports that hold dispatches.
///
/// # Errors
///
/// The dispatch with the link appended breaks an invariant of its entity.
pub fn record_dispatch_link<P: DispatchStorage>(
    ports: &mut P,
    input: RecordDispatchLink,
) -> Result<RecordDispatchLinkOutcome, UnmetObligation> {
    if !valid_link(&input.link) {
        return Ok(RecordDispatchLinkOutcome::InvalidLink {
            error: InvalidLink { link: input.link },
        });
    }
    let Some(mut held) = DispatchStorage::get(ports, &input.dispatch_id) else {
        return Ok(RecordDispatchLinkOutcome::NoSuchDispatch {
            error: DispatchNotFound {
                dispatch_id: input.dispatch_id,
            },
        });
    };
    if held.data.links.contains(&input.link) {
        return Ok(RecordDispatchLinkOutcome::AlreadyLinked);
    }
    if held.data.links.len() >= LINKS_MOST {
        return Ok(RecordDispatchLinkOutcome::LinksFull {
            error: DispatchLinksFull {
                dispatch_id: input.dispatch_id,
            },
        });
    }
    held.data.links.push(input.link.clone());
    if let Some(broken) = held.data.broken_invariant() {
        return Err(UnmetObligation {
            capability: "entity invariant",
            source: broken,
        });
    }
    DispatchStorage::put(ports, held);
    Ok(RecordDispatchLinkOutcome::Linked {
        dispatch_linked: DispatchLinked {
            actor: input.actor,
            dispatch_id: input.dispatch_id,
            link: input.link,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::{valid_link, valid_reference};

    #[test]
    fn a_reference_holds_its_characters_and_length_only() {
        for good in ["a", "WI-1042", "9z.Y_x-0", &"A".repeat(64)] {
            assert!(valid_reference(good), "{good:?}");
        }
        for bad in [
            "",
            &"a".repeat(65),
            "-a",
            ".a",
            "_a",
            "a/b",
            "a?b",
            "a b",
            "ä",
            "a\u{0}",
            "a\u{7f}",
        ] {
            assert!(!valid_reference(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_link_is_an_http_or_https_url_of_bounded_length() {
        let longest = format!("https://example.org/{}", "a".repeat(2048 - 20));
        for good in ["http://a", "https://example.org/x", longest.as_str()] {
            assert!(valid_link(good), "{good:?}");
        }
        let longer = format!("{longest}a");
        for bad in [
            "",
            "http://",
            "https://",
            "ftp://example.org",
            "example.org",
            "https://example.org/a b",
            "https://example.org/a\nb",
            longer.as_str(),
        ] {
            assert!(!valid_link(bad), "{bad:?}");
        }
    }

    /// No character a terminal would act on: every control character (C0, DEL and C1), the line
    /// and paragraph separators and every bidirectional control. A character past ASCII that is
    /// none of them is taken.
    #[test]
    fn a_link_holds_no_control_or_bidirectional_character() {
        let refused = (0..=0x1f_u32)
            .chain(0x7f..=0x9f)
            .chain(0x202a..=0x202e)
            .chain(0x2066..=0x2069)
            .chain([0x2028, 0x2029])
            .filter_map(char::from_u32);
        for character in refused {
            let link = format!("https://example.org/a{character}b");
            assert!(!valid_link(&link), "{link:?}");
        }
        assert!(valid_link("https://example.org/straße/ä"));
    }
}
