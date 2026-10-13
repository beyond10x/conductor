//! `conductor spawn` records the session id `claude --bg` printed, not its whole banner.

mod active;

use conductor_cli::spawn::session_id;

#[test]
fn the_id_is_read_from_the_backgrounded_line_of_the_banner() {
    active::isolate();
    let banner = "backgrounded · 27734a97 · alpha\n  claude agents             list sessions\n  \
                  claude attach 27734a97    open in this terminal\n";
    assert_eq!(session_id(banner), "27734a97");
}

#[test]
fn a_harness_that_prints_only_the_id_is_read_as_it_is() {
    active::isolate();
    assert_eq!(session_id("27734a97\n"), "27734a97");
}
