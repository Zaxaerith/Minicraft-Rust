//! Entity IDs, arena lifecycle, collision, mobs, furniture, and particles.
//!
//! Layout mirrors Java `entity/*`: one module per class family. This file is
//! the public surface (`Entity`, `EntityKind`, `EntityArena`, helpers) plus
//! re-exports so the rest of the crate keeps compiling.
pub mod arena;
pub mod furniture;
pub mod helpers;
pub mod mob;
pub mod particle;
pub mod projectile;
pub mod stats;
pub mod tick;
pub mod types;

pub use arena::EntityArena;
pub use furniture::FurnitureKind;
pub use helpers::{can_stand, furniture_can_stand, squared_distance};
pub use particle::{ParticleKind, text_particle_motion};
pub use projectile::{Projectile, ProjectileKind};
pub use stats::{mob_drops, mob_health};
pub use types::{DESPAWN_AGE, Entity, EntityKind, HitResult, ItemMotion, Mob, TickOutcome};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::item::{Inventory, ItemId, ItemStack};
    use crate::level::noise::JavaRandom;
    use crate::level::tile::Tile;

    #[test]
    fn mob_damage_creates_collectible_drop() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(4);
        arena.spawn_mob(mob::NaturalMob::Slime, 40, 40);
        let hit = arena.damage_nearest(40, 40, 2, &mut random).unwrap();
        assert!(hit.defeated);
        assert_eq!(arena.mob_count(), 0);
        for entity in &mut arena.entities {
            if matches!(entity.kind, EntityKind::Item(_)) {
                entity.age = 31;
            }
        }
        let mut inventory = Inventory::new(8);
        let collected = arena.collect_near(40, 40, &mut inventory);
        assert_eq!(collected[0].item, ItemId::Slime);
        assert!(inventory.count(ItemId::Slime) > 0);
    }

    #[test]
    fn explosion_ticker_converts_exploded_tiles_to_holes_after_18_ticks() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(1);
        let mut tiles = vec![Tile::Grass; 16 * 16];
        // Pre-place Exploded tiles as apply_explosion does before the ticker settles them.
        for y in 7..=9 {
            for x in 7..=9 {
                tiles[x + y * 16] = Tile::Exploded;
            }
        }
        let data = vec![0u16; 16 * 16];
        arena.spawn_explosion_ticker(8 * 16 + 8, 8 * 16 + 8, 1, 0);
        for _ in 0..18 {
            arena.tick(&mut tiles, &data, 16, 16, 0, 0, false, false, &mut random);
        }
        assert_eq!(tiles[8 + 8 * 16], Tile::Hole);
        assert_eq!(tiles[7 + 7 * 16], Tile::Hole);
        assert_eq!(tiles[9 + 9 * 16], Tile::Hole);
        assert!(
            arena
                .entities()
                .iter()
                .all(|entity| !matches!(entity.kind, EntityKind::ExplosionTicker))
        );
    }

    #[test]
    fn explosion_ticker_on_sky_depth_becomes_infinite_fall() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(1);
        let mut tiles = vec![Tile::Exploded; 16 * 16];
        let data = vec![0u16; 16 * 16];
        arena.spawn_explosion_ticker(8 * 16 + 8, 8 * 16 + 8, 0, 1);
        for _ in 0..18 {
            arena.tick(&mut tiles, &data, 16, 16, 0, 0, false, false, &mut random);
        }
        assert_eq!(tiles[8 + 8 * 16], Tile::InfiniteFall);
    }

    #[test]
    fn skeleton_projectile_and_creeper_fuse_are_stateful() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(9);
        let mut tiles = vec![Tile::Grass; 16 * 16];
        let data = vec![0; 16 * 16];
        arena.spawn_mob(mob::NaturalMob::Skeleton, 80, 80);
        arena.spawn_mob(mob::NaturalMob::Creeper, 145, 80);
        arena.tick(
            &mut tiles,
            &data,
            16,
            16,
            150,
            80,
            false,
            false,
            &mut random,
        );
        assert!(
            arena
                .entities()
                .iter()
                .any(|entity| matches!(entity.kind, EntityKind::Projectile(_)))
        );
        assert!(arena.entities().iter().any(|entity| {
            matches!(
                &entity.kind,
                EntityKind::Mob(Mob {
                    species: mob::NaturalMob::Creeper,
                    attack_time: 1..,
                    ..
                })
            )
        }));
    }

    #[test]
    fn dungeon_chest_unlocks_and_transfers_its_local_loot() {
        let mut arena = EntityArena::default();
        arena.spawn_furniture(furniture::FurnitureKind::DungeonChest, 40, 40);
        let mut inventory = Inventory::new(8);
        inventory.add(ItemId::Key, 1);
        assert_eq!(
            arena.use_container_near(40, 40, &mut inventory, None),
            Some("DUNGEON CHEST UNLOCKED".to_owned())
        );
        assert_eq!(inventory.count(ItemId::Key), 0);
        assert!(
            arena
                .use_container_near(40, 40, &mut inventory, None)
                .unwrap()
                .starts_with("TOOK")
        );
        assert_eq!(inventory.count(ItemId::GoldIngot), 3);
    }

    #[test]
    fn both_bosses_drop_their_progression_items() {
        let mut arena = EntityArena::default();
        let mut random = JavaRandom::new(12);
        arena.spawn_mob(mob::NaturalMob::AirWizard, 40, 40);
        assert_eq!(
            arena.active_boss(mob::NaturalMob::AirWizard),
            Some((2_000, 2_000))
        );
        assert!(
            arena
                .damage_nearest(40, 40, 2_000, &mut random)
                .unwrap()
                .defeated
        );
        assert_eq!(arena.active_boss(mob::NaturalMob::AirWizard), None);
        arena.spawn_mob(mob::NaturalMob::ObsidianKnight, 60, 40);
        assert!(
            arena
                .damage_nearest(60, 40, 5_000, &mut random)
                .unwrap()
                .defeated
        );
        for entity in &mut arena.entities {
            if matches!(entity.kind, EntityKind::Item(_)) {
                entity.age = 31;
            }
        }
        let mut inventory = Inventory::new(8);
        arena.collect_near(40, 40, &mut inventory);
        arena.collect_near(60, 40, &mut inventory);
        assert!(inventory.count(ItemId::CloudOre) >= 5);
        assert!(inventory.count(ItemId::Shard) >= 15);
        assert_eq!(inventory.count(ItemId::ObsidianHeart), 1);
    }

    #[test]
    fn dropped_items_use_java_ballistics_and_the_thirty_tick_pickup_delay() {
        let mut arena = EntityArena::default();
        arena.spawn_item(ItemStack::new(ItemId::Acorn, 1), 40, 40);
        let item = &arena.entities[0];
        let motion = item.item_motion.as_ref().unwrap();
        assert_eq!(motion.height, 2.0);
        assert!((1.0..1.7).contains(&motion.velocity_z));

        let mut inventory = Inventory::new(8);
        assert!(arena.collect_near(40, 40, &mut inventory).is_empty());
        arena.entities[0].age = 31;
        assert_eq!(arena.collect_near(40, 40, &mut inventory).len(), 1);
        assert_eq!(inventory.count(ItemId::Acorn), 1);
    }
}
