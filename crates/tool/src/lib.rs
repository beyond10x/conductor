//! The `tool` binary (`story:tool-broker-read`, design docs/design/roles-and-surfaces.md § 6.1):
//! a thin client of the instance's broker. Each verb is one call over the session's socket
//! ([`conductor_broker::call`]); `tool mcp` offers the same calls as MCP tools over stdio
//! ([`mcp`]). It opens no store, holds no grant and has no admin command.

#![forbid(unsafe_code)]

pub mod cli;
pub mod mcp;
