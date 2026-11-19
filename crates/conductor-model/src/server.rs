// generated from conductor v1
// contract digest f83017a2b1547ffe433fb36ace7fc15a0642089bbb79d7bd80c1038f423d74e1
// do not edit: regenerate with `ess synthesize --layout crate`

//! The HTTP surface of `conductor` v1, synthesised.
//!
//! One module per component the specification declares is reached over a network, each holding
//! that component's route table, its listener and the two documents it publishes about itself.
//! The routes are the ones the committed `OpenAPI` document declares, from the same mapping, so a
//! path served here and a path published there cannot be two different answers.
//!
//! Generated, not written: the specification is the source of truth, and the door to changing
//! anything here is `ess synthesize`. What is deliberately absent is absent by
//! decision — no framework, no runtime, no second protocol, no concurrency, no authentication —
//! and each absence is argued in the `TARGET.md` beside this workspace.

#![forbid(unsafe_code)]
#![deny(missing_docs)]

pub mod entry;
pub mod http;
pub mod json;
pub mod wire;
pub mod conductor_instance;
pub mod conductord;
pub mod memory;
mod static_assets;
