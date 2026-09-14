//! Java `EntitySummonItem` — summons a vehicle/entity in front of the player.
//!
//! Java registers `Boat` via `EntitySummonItem.getAllInstances` and places it
//! at `player + 12 * attackDir`. The Rust `ItemId` registry has no Boat yet,
//! so `all()` stays empty until the vehicle entity lands.
use super::ItemId;

/// Java places the entity at `player + 12 * attackDir`.
pub const SPAWN_OFFSET: i32 = 12;

/// Java `EntitySummonItem.getAllInstances` once Boat exists as an ItemId.
pub fn all() -> &'static [ItemId] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spawn_offset_matches_java() {
        assert_eq!(SPAWN_OFFSET, 12);
        assert!(all().is_empty());
    }
}
