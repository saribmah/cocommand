//! Extension host for third-party integrations (Core-10).
//!
//! Provides the manifest types, JSON-RPC protocol, and lifecycle management
//! for loading and invoking extension tools via the Deno extension host.

pub mod manifest;
pub mod rpc;
pub mod lifecycle;
