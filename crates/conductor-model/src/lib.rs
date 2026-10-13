// generated from conductor v1
// model digest 12036b1b18860e485d49ea562998626ce5687d688d1ebc44c2155ce8c5d47ceb
// contract digest a0e58f786885660221ebe1d489f0dd17c50bad33467c15be211a67dad661d7d7
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
