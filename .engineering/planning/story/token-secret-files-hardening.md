---
format: aep.planning-md/3
id: story:token-secret-files-hardening
kind: story
status: draft
title: 'Pool token files: no symlinked parent, leave reports each failure, answers name no secret path'
summary: Hardening of the files-source writes of join and leave, and of the token routes' errors, found in review
relations:
- decomposes: epic:architecture-next
- informed_by: story:host-token-pool
revision: 2
---
## Why

Adversary review of `story:host-token-pool` (2026-11-18), on the code of
`story:pool-token-kind-files-source`:

- `join` creates the parent directories with a recursive builder that follows a symlinked
  `claude-token/` or instance directory, and leaves an existing directory's mode as it is
  (`crates/conductor/src/secret.rs`); `leave` deletes the file at a recorded name even when it did
  not write it, and removes empty directories upward that the operator made.
- A failed removal answers 500 after the withdraw is recorded, and the remaining tokens are left.
- The token routes answer the whole error chain, with the secrets directory's paths
  (`crates/conductor/src/token.rs` `failed`).
- The dial listener starts a thread per connection with no cap (`daemon/dial.rs`).

## Outcome

- Each parent is opened without following a symlink, owned by the user and not writable by group
  or others, else join refuses naming it; leave removes only a file join wrote under `files`.
- Leave goes on past a failed removal and reports each one.
- The routes answer a generic error and log the detail.
- The dial listener holds at most a bounded number of greetings at once.

## Acceptance

- `join_refuses_a_symlinked_token_directory`, `leave_reports_a_failed_removal_and_withdraws_the_rest`,
  `a_token_route_error_names_no_secret_path`, `the_dial_listener_bounds_its_open_greetings`.
