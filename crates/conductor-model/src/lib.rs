// generated from conductor v1
// model digest 05f93c828432a41a286312a5a65d4f6d32a0dd0bc1b9e98c52d85aa6850de2c0
// contract digest f333b90124a79bfaf580f3d0b8e7e705416bbc5494abfcd08c16de9ac13549f7
// do not edit: regenerate with `ess synthesize --layout crate`

//! Semantic types synthesised from the `conductor` specification, v1.
//!
//! Generated, not written: the specification is the source of truth, and the door to changing
//! anything here is `ess synthesize`. What is deliberately absent — behaviour, queries,
//! escalations — is listed with reasons in the `PLAN.md` beside this workspace, and every entry
//! there is owed through a typed seam in an `obligations` module here.

// `deny`, not the source workspace's lint set: this crate must hold on its own, and an undocumented
// public item here is an emitter defect worth failing the gate over.
#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod actor;
pub mod behaviour;
pub mod config;
pub mod decision;
pub mod direction;
pub mod dispatch;
pub mod host;
pub mod obligation;
pub mod observation;
pub mod primitives;
pub mod session;

pub mod ports;
pub mod system;
