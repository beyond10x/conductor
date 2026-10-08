// generated from conductor v1
// model digest da7f223adaed415012785783afd834b78fee21f26d65fe9977c5813f8c3f9b33
// contract digest 2cad3a53f65176fd47da0cb4583a9f915aca1519194cf0b0d6ce4fcde6f0b33c
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

pub mod ports;
pub mod system;
