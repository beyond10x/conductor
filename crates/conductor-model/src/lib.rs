// generated from conductor v1
// model digest e6c77bcc7ee0e79df63499d4607914bc01087e8a17250dc26ed3b68caa1041e1
// contract digest b02ae21d8bc0215dc253eb959a34254ff287e39dbd164717556e86b8af4a99fd
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
