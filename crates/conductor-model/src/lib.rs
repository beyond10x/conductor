// generated from conductor v1
// model digest 109eac9bc6b0c75695a11d17996366f04df7819f95538fdd47e80a6ddf6d1fa6
// contract digest 11801c6b74d900cf330031ec8373c8af2996240dbfd292a2c2e2e8207e6bf6fd
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
