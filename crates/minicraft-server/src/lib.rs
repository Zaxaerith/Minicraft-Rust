//! Standalone multiplayer server for the Minicraft+ 2.2.4 parity port.
//!
//! Depends only on `minicraft-common` for protocol types. No client assets,
//! world generation, or rendering code are linked into this crate.

pub use minicraft_common as common;

pub mod server;
pub mod session;

pub use server::{default_server_address, run_server, serve};
