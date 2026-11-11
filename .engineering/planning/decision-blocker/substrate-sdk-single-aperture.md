---
format: aep.planning-md/3
id: decision-blocker:substrate-sdk-single-aperture
kind: decision-blocker
status: open
title: The pinned substrate SDK sends one aperture per session, so a box request cannot name a forge host
relations:
- blocks: story:box-reaches-forge-and-daemon
withholds: test_result
revision: 1
---
## What is stopped

The first half of the acceptance of `story:box-reaches-forge-and-daemon`: "the box request names
egress to each forge host the instance's repositories use and its cwd". Unit U4 of wave 51 built
the second half (the daemon's token reaches the runtime in a local box through a sealed slot) and
could not build this one.

## Why

The substrate SDK conductor pins (`b10x-substrate-sdk` 0.20.0, revision `546693082`, in
`crates/conductor/Cargo.toml`) takes one aperture per session, and one aperture is one
destination:

- its session builders hold `aperture: Option<String>`, and `.aperture(name)` overwrites it;
- `start()` sends `aperture: self.aperture, apertures: Vec::new()`; the function that sends the
  mutation is private, so a caller has no other route;
- the wire type has `apertures: Vec<String>` and the daemon lists apertures for an exec, but no
  revision of the SDK sets the field. Read on 2026-11-11: the SDK on the substrate repository's
  `main` at `8b194eaf6` still sends `apertures: Vec::new()` in its three start paths.

The SDK has no field for the working directory either (`workdir: None` in both start paths), so a
box's runtime starts in the workspace directory; `BoxRequest.cwd` is set and never sent.

## What clears it

A release of the substrate SDK whose session builder takes several apertures (and, for the second
part, a working directory), and the pin in `crates/conductor/Cargo.toml` and `b10x.toml` moved to
it. Then the box request names one aperture per forge host beside `conductor`, as the wave's
decision for U4 has it, and the tests against the fake substrate hold it.

## Until then

A boxed session reaches no forge. `decision-blocker:box-api-unix-socket-denied` is open as well:
on substrate-daemon 0.20.0 the live box does not serve its API at all.
