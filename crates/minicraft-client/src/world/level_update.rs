//! Shared Level dispatch boundary for awake and sleeping compatibility hosts.
//! Species AI remains in entity; this adapter owns pending-add ordering only.
use super::{FurnitureKind, World, entity::TickOutcome};

impl World {
    pub(super) fn tick_level_entities(
        &mut self,
        player: Option<(i32, i32)>,
        time_slowed: bool,
        player_passive: bool,
    ) -> TickOutcome {
        let level = &mut self.levels[self.current_level];
        for intent in std::mem::take(&mut level.pending_spawns) {
            level.entities.spawn_mob(intent.kind, intent.x, intent.y);
        }
        let depth = level.depth;
        let outcome = level.entities.tick_with_player_difficulty(
            &mut level.tiles,
            &level.data,
            self.width,
            self.height,
            player,
            time_slowed,
            player_passive,
            &mut self.random,
            self.difficulty.min(2) as u8,
            depth,
        );
        if player.is_some() {
            self.retrieve_death_chests();
        }
        outcome
    }

    /// Java DeathChest.touchedBy: walk into the chest to take everything.
    pub(super) fn retrieve_death_chests(&mut self) {
        let px = self.player.x;
        let py = self.player.y;
        let mut chest_id = None;
        for entity in self.levels[self.current_level].entities.entities() {
            if !matches!(
                entity.kind,
                super::entity::EntityKind::Furniture(
                    FurnitureKind::Chest | FurnitureKind::DungeonChest
                )
            ) || entity.state != crate::entity::furniture::chest::DEATH_CHEST_STATE
            {
                continue;
            }
            if (i64::from(entity.x) - i64::from(px)).abs() < 10
                && (i64::from(entity.y) - i64::from(py)).abs() < 10
            {
                chest_id = Some(entity.id);
                break;
            }
        }
        let Some(id) = chest_id else {
            return;
        };
        let Some(storage) = self.levels[self.current_level]
            .entities
            .death_chest_storage(id)
        else {
            return;
        };
        let stacks = storage.slots().to_vec();
        let tools = storage.tools().to_vec();
        for stack in stacks {
            let remaining = self.player.inventory.add(stack.item, stack.count);
            let transferred = stack.count - remaining;
            if transferred > 0 {
                self.levels[self.current_level]
                    .entities
                    .entities_mut_for_storage(id)
                    .expect("chest exists")
                    .remove(stack.item, transferred);
                self.sound(super::SoundEffect::Pickup);
            }
            if remaining > 0 {
                return;
            }
        }
        for tool in tools {
            if self.player.inventory.add_tool(tool).is_none() {
                return;
            }
            // Remove this exact tool, preserving durability even for equal kinds.
            let storage = self.levels[self.current_level]
                .entities
                .entities_mut_for_storage(id)
                .expect("chest exists");
            let index = storage
                .tools
                .iter()
                .position(|candidate| *candidate == tool)
                .expect("copied tool exists");
            storage.tools.remove(index);
        }
        self.levels[self.current_level]
            .entities
            .mark_removed_by_id(id);
        self.add_notification("minicraft.notification.death_chest_retrieved");
    }
}
