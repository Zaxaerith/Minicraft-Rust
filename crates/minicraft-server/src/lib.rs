//! Staged standalone server boundary.
//!
//! The compatibility server remains in the root package until the protocol
//! registry and session behavior have differential coverage.

pub use minicraft_common as common;

pub mod server;
pub mod session;
