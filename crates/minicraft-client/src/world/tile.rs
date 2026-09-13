//! Level-bound tile helpers (damage against a Level); identities live in client.
#![allow(unused_imports)]
use super::Level;
pub use crate::level::tile::{
    Tile, connection_mask, connects, find_stair_site, fluid_animation_phase, increment_tile_tick,
    is_tile_item, tile_placement, tile_tick, underlay_layers,
};

pub fn damage_tile(
    level: &mut Level,
    index: usize,
    damage: u16,
    health: u16,
    replacement: Tile,
) -> bool {
    let total = level.data[index].saturating_add(damage);
    if total >= health {
        level.tiles[index] = replacement;
        level.data[index] = 0;
        true
    } else {
        level.data[index] = total;
        false
    }
}
