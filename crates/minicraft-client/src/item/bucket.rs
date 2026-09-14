//! Java `BucketItem` — fill states and tile interaction table.
use super::ItemId;
use crate::level::tile::Tile;

/// Java `BucketItem.Fill`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BucketFill {
    Empty,
    Water,
    Lava,
}

impl BucketFill {
    pub const ALL: [Self; 3] = [Self::Empty, Self::Water, Self::Lava];

    /// Java `Fill` display prefix used in the item name (`"Water Bucket"`).
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Empty => "Empty",
            Self::Water => "Water",
            Self::Lava => "Lava",
        }
    }

    /// Java `Fill.offset` sprite index.
    pub const fn offset(self) -> u8 {
        match self {
            Self::Empty => 2,
            Self::Water => 0,
            Self::Lava => 1,
        }
    }

    pub const fn item(self) -> ItemId {
        match self {
            Self::Empty => ItemId::EmptyBucket,
            Self::Water => ItemId::WaterBucket,
            Self::Lava => ItemId::LavaBucket,
        }
    }

    /// Java `Fill.contained` tile id.
    pub const fn contained_tile(self) -> Tile {
        match self {
            Self::Empty => Tile::Hole,
            Self::Water => Tile::Water,
            Self::Lava => Tile::Lava,
        }
    }

    pub const fn for_item(item: ItemId) -> Option<Self> {
        match item {
            ItemId::EmptyBucket => Some(Self::Empty),
            ItemId::WaterBucket => Some(Self::Water),
            ItemId::LavaBucket => Some(Self::Lava),
            _ => None,
        }
    }
}

/// Java `BucketItem.getFilling`.
pub fn fill_for_tile(tile: Tile) -> Option<BucketFill> {
    BucketFill::ALL
        .into_iter()
        .find(|fill| fill.contained_tile() == tile)
}

/// Java `BucketItem.interactOn` pure decision table.
/// Returns `(tile_after, bucket_after)` when the bucket action applies.
pub fn bucket_action(item: ItemId, target: Tile) -> Option<(Tile, ItemId)> {
    let filling = BucketFill::for_item(item)?;
    let target_fill = fill_for_tile(target)?;
    match (filling, target_fill) {
        // Pour water/lava into a hole.
        (BucketFill::Water | BucketFill::Lava, BucketFill::Empty) => {
            Some((filling.contained_tile(), ItemId::EmptyBucket))
        }
        // Water on lava makes obsidian.
        (BucketFill::Water, BucketFill::Lava) => Some((Tile::ObsidianFloor, ItemId::EmptyBucket)),
        // Empty bucket picks up water/lava.
        (BucketFill::Empty, BucketFill::Water | BucketFill::Lava) => {
            Some((Tile::Hole, target_fill.item()))
        }
        _ => None,
    }
}

/// Java `BucketItem` instances.
pub const ALL: [ItemId; 3] = [
    ItemId::EmptyBucket,
    ItemId::WaterBucket,
    ItemId::LavaBucket,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_bucket_picks_up_fluids() {
        assert_eq!(
            bucket_action(ItemId::EmptyBucket, Tile::Water),
            Some((Tile::Hole, ItemId::WaterBucket))
        );
        assert_eq!(
            bucket_action(ItemId::EmptyBucket, Tile::Lava),
            Some((Tile::Hole, ItemId::LavaBucket))
        );
    }

    #[test]
    fn water_bucket_makes_obsidian_on_lava() {
        assert_eq!(
            bucket_action(ItemId::WaterBucket, Tile::Lava),
            Some((Tile::ObsidianFloor, ItemId::EmptyBucket))
        );
    }

    #[test]
    fn filled_bucket_pours_into_hole() {
        assert_eq!(
            bucket_action(ItemId::WaterBucket, Tile::Hole),
            Some((Tile::Water, ItemId::EmptyBucket))
        );
        assert_eq!(
            bucket_action(ItemId::LavaBucket, Tile::Hole),
            Some((Tile::Lava, ItemId::EmptyBucket))
        );
        assert_eq!(bucket_action(ItemId::EmptyBucket, Tile::Hole), None);
    }
}
