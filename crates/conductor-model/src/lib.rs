// generated from conductor v1
// model digest cef836f319668d2492af74e7071a0a12fe762ad06c7635432c0f51a06be91469
// contract digest 1cb1dde36edec20c87f5fbc7455900f69fed5276ffdef4838c372cc21d625df7
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
