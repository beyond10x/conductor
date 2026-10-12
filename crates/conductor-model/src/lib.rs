// generated from conductor v1
// model digest 75ae36bcd64ac6ed7ea7350fa76e1813347a0d4390844f42e75e83fd78193d30
// contract digest 218b3db30bad06f16859184bd6831e8cd6288a02160b6f0fec5f8e61a9fbd7ee
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
pub mod obligation;
pub mod observation;
pub mod primitives;
pub mod session;

pub mod ports;
pub mod system;
