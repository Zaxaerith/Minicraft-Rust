//! Client multiplayer surface: Java `network/*` 1:1 plus the Rust TCP transport.
//!
//! Layout mirrors Java `network/`:
//! - `minicraft_protocol` ↔ `MinicraftProtocol.java` (PORT, InputType registry)
//! - `network` ↔ `Network.java` (entity ids, release version lookup)
//! - `transport` — line-delimited TCP client used by `--multiplayer-probe`

mod minicraft_protocol;
// Preserve the established Java class/module naming.
#[allow(clippy::module_inception)]
mod network;
mod transport;

pub use minicraft_protocol::{
    DEFAULT_PORT, GAME_VERSION, InputType, MAX_MESSAGE_BYTES, Message, PORT, PROTOCOL_VERSION,
    PlayerState, decode_line, encode_line, read_bounded_line, sanitize_username,
};
pub use network::{
    LOCAL_RELEASE_VERSION, VersionInfo, collect_entity_ids, generate_unique_entity_id,
    generate_unique_entity_id_with, get_entity, get_latest_version, id_is_available,
    select_latest_release, set_latest_version, store_local_version_fallback,
};
pub use transport::{MultiplayerClient, run_client_probe};
