// generated from conductor v1
// model digest 6c4af053c24bc9c6a181dbaebd71d00cc9c508fd7c3c9865cd09b9c508d08303
// contract digest 670f64163e3ed1c1ffe26024d3e070d306960ad6f0e75b1b599e0a52a6562f23
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
