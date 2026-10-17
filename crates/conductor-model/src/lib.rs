// generated from conductor v1
// model digest e500103a100d7d78986d63e0a374f290e95f0a58dd6379fa8d320a440f8460a7
// contract digest 0896ff66ec00e66191cd7bd2f022a5f43bb846f69adf89b29693726772c8dda3
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
pub mod improvement;
pub mod obligation;
pub mod observation;
pub mod primitives;
pub mod session;
pub mod tool;

pub mod ports;
pub mod system;
