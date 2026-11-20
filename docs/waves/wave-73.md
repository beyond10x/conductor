# Wave 73 (2026-11-20: config put edits only its path; the supervisor's cap goes through the file's owner)

The eighteenth wave of the goal that fixes the friction the agent box filed: issues #263 and #264.
It forks from `main` at release 1.22.1. The critic panel's result: `review-result:w73-critic-panel`.

Width: two units, every ready story; they share no file.

## Units

| unit | stories | issues | stage |
|---|---|---|---|
| U1 | config-put-touches-only-its-path | #263 | merged |
| U2 | supervisor-cap-through-config-owner | #264 | merged |

## Outcome

`conductor config put` and the instance calls (add, replace, remove) change only the text of the
named path or of the one instance's list item: a new safe-Rust locator, `config/edit.rs`, edits the
value in the style it stands in (flow or block), inserts a missing key in its mapping's style and
removes one entry only; comments, key order, quoting and blank lines stay byte for byte. Every
edited text is read back against the changed tree; a mismatch, or a shape the locator does not
read on the path (a document marker, an anchor, alias, merge key or tag, an explicit key, a
multi-line flow collection or scalar), writes the file whole as before and says so in one line on
standard error and in the commit body. A put that leaves the file as it was commits nothing and
prints `unchanged`; a removal no longer leaves empty mappings on its path.

The supervisor profile's cap example says a cap decision is carried out through the config file's
owner when the host's own file names one; `tests/profiles.rs` holds the row and the profile's
line cap as written.

In the box: the locator's unit tests, `config_globals`, the profile, roles and no-organization
suites, and clippy; the instance-route cases that bind a Unix socket need CI.
