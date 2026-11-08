//! Conductor's own `systemd` session provider is gone (`story:placement-substrate-local`,
//! acceptance line 4): every placement is a substrate box, and conductor starts no session in a
//! `systemd-run --user` unit of its own.
//!
//! - The `systemd` provider is never available and names the substrate gap
//!   ([`confine::SESSIONS_UNSERVED`]); every start, prompt, resume and measure through it is
//!   refused. Under `strict` a session asking for it is refused `ProviderUnavailable`; under
//!   `report` it starts through `none`, its record naming why, as for any unavailable provider.
//! - The names the units conductor still runs outside any box take, in [`systemd`]: an
//!   instance runtime's, `conductor run`'s and a host role's unit and slice.
//!
//! The cases that asserted conductor's own `systemd-run` argument vector, its slice limits, its
//! cgroup measurement, its delegated-controllers probe, its pause, resume and stop through a
//! unit, and `docs/config.md` naming the provider as the limits' reader describe nothing that
//! runs any more and are gone with the provider.

mod active;

use conductor_cli::confine::{
    self, Confinement, ConfinementMode, ConfinementProvider, Host, Launch, Providers as _, Refusal,
};
use conductor_cli::systemd;
use conductor_model::config::{Limits, Network, SessionName};
use conductor_model::session::SessionRequest;

fn no_limits() -> Limits {
    Limits {
        memory_max: None,
        cpu_weight: None,
        tasks_max: None,
    }
}

fn launch() -> Launch {
    Launch {
        request: SessionRequest {
            role: "conductor".to_owned(),
            session_name: SessionName("conductor".to_owned()),
            cwd: "/nonexistent".to_owned(),
            limits: no_limits(),
            writable: Vec::new(),
            network: Network::Open,
            forge_token_env: Vec::new(),
        },
        program: "claude".to_owned(),
        args: Vec::new(),
        env: Vec::new(),
        session_prefix: None,
        instance_limits: no_limits(),
        state: "/nonexistent".into(),
    }
}

#[test]
fn the_systemd_provider_is_not_served_and_names_the_substrate_gap() {
    active::isolate();
    let mut provider = Host.provider(ConfinementProvider::Systemd);
    let answer = provider.available();
    assert_eq!(answer.provider, "systemd");
    assert!(!answer.available, "{answer:?}");
    assert_eq!(answer.reason.as_deref(), Some(confine::SESSIONS_UNSERVED));
    let refused = provider
        .start(&launch())
        .expect_err("conductor starts no systemd session");
    assert!(
        refused.to_string().contains("substrate"),
        "the refusal names the substrate gap: {refused:#}"
    );
}

#[test]
fn strict_refuses_a_systemd_session_and_report_starts_it_through_none_saying_why() {
    active::isolate();
    let strict = Confinement {
        mode: ConfinementMode::Strict,
        provider: ConfinementProvider::Systemd,
        limits: no_limits(),
    };
    match confine::choose(&Host, &strict) {
        Err(Refusal::ProviderUnavailable(reason)) => {
            assert!(reason.contains(confine::SESSIONS_UNSERVED), "{reason}");
        }
        other => panic!("strict refuses the unserved provider: {other:?}"),
    }
    let report = Confinement {
        mode: ConfinementMode::Report,
        ..strict
    };
    let chosen = confine::choose(&Host, &report).expect("report starts the session");
    assert_eq!(chosen.name(), "none");
}

/// The unit and slice names: `conductor_<session name>.service`, a grouped repository's `/`
/// escaped as systemd escapes it; `conductor_<prefix>.slice`, or `conductor.slice`.
#[test]
fn unit_and_slice_names() {
    assert_eq!(
        systemd::unit_name(&SessionName("ex-alpha".to_owned())),
        "conductor_ex-alpha.service"
    );
    assert_eq!(
        systemd::unit_name(&SessionName("ex-group/alpha".to_owned())),
        "conductor_ex-group\\x2falpha.service"
    );
    assert_eq!(systemd::slice_name(Some("ex")), "conductor_ex.slice");
    assert_eq!(systemd::slice_name(None), "conductor.slice");
}

/// systemd reads each `-` of a slice's name as one step down: `conductor-ex.slice` is a child of
/// `conductor.slice`, the slice of the instance without a prefix, whose limits would then cap it.
/// No instance's slice has a `-` in its name, a prefix's own `-` escaped as `\x2d`, so every
/// instance's slice is a child of the root slice, and a sibling of every other.
#[test]
fn every_instance_slice_is_a_sibling_of_the_others() {
    let slices: Vec<String> = [None, Some("ex"), Some("ex-a"), Some("lrun"), Some("a-b-c")]
        .into_iter()
        .map(systemd::slice_name)
        .collect();
    for slice in &slices {
        let stem = slice.strip_suffix(".slice").expect("a slice");
        assert!(
            !stem.contains('-'),
            "{slice} would nest inside another slice"
        );
    }
    assert_eq!(slices[2], "conductor_ex\\x2da.slice");
    let mut distinct = slices.clone();
    distinct.sort();
    distinct.dedup();
    assert_eq!(distinct.len(), slices.len(), "{slices:?}");
}
