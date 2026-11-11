// generated from conductor v1
// contract digest 2b03183365aedd50e23f3659c527d6ee72cb8b1ac517c9e046bb013284b8a454
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
pub mod agent;
pub mod behaviour;
pub mod config;
pub mod decision;
pub mod direction;
pub mod dispatch;
pub mod host;
pub mod improvement;
pub mod instance;
pub mod obligation;
pub mod observation;
pub mod operator;
pub mod placement;
pub mod primitives;
pub mod secret;
pub mod session;
pub mod store;
pub mod token;
pub mod tool;

pub mod ports;
pub mod system;
#[cfg(feature = "server")]
pub mod server;
