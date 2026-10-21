//! Paged non-solid Torch overlays preserve the complete underlying Tile/data.
//! This storage adaptation avoids erasing an underlying data field on pickup.
use super::super::{
    entities::{Kind, Page, Position, dynamic::ItemDrop},
    survival::{Action, Loot, State},
};
use super::*;
use crate::{
    item::{ItemId, ItemStack},
    world::ActiveItem,
};
pub(super) fn torch_at(pages: &BTreeMap<Key, Page>, at: Position) -> bool {
    pages.get(&at.key()).is_some_and(|p| {
        p.entities.iter().any(|e| {
            matches!(e.kind, Kind::Torch)
                && e.x == at.x.rem_euclid(64) as u8
                && e.y == at.y.rem_euclid(64) as u8
        })
    })
}
impl World {
    pub(super) fn torch_lit(
        &mut self,
        at: Position,
        active: &BTreeMap<Key, Page>,
    ) -> Result<bool, String> {
        let mut keys = std::collections::BTreeSet::new();
        for dx in [-3, 0, 3] {
            for dy in [-3, 0, 3] {
                if let Some((x, y)) = at.x.checked_add(dx).zip(at.y.checked_add(dy)) {
                    keys.insert(Key::at_depth(at.depth, x, y));
                }
            }
        }
        for key in keys {
            let page = if let Some(p) = active.get(&key) {
                p.clone()
            } else {
                self.entity_page(key)?
            };
            if page.entities.iter().any(|e| {
                matches!(e.kind, Kind::Torch) && {
                    let p = page.position(e);
                    p.x.abs_diff(at.x) <= 3 && p.y.abs_diff(at.y) <= 3
                }
            }) {
                return Ok(true);
            }
        }
        Ok(false)
    }
    pub(super) fn torch_action(
        &mut self,
        s: &mut State,
        pages: &mut BTreeMap<Key, Page>,
        action: Action,
        target: Position,
        standing: Ground,
    ) -> Result<bool, String> {
        if s.environment.is_none()
            || s.player.health == 0
            || s.panel != super::super::survival::Panel::Closed
        {
            return Ok(false);
        }
        if !matches!(action, Action::Attack | Action::Pickup) {
            return Ok(false);
        }
        if matches!(action, Action::Pickup)
            && matches!(standing, Ground::StairsDown | Ground::StairsUp)
        {
            return Ok(false);
        }
        if matches!(action, Action::Attack)
            && matches!(s.player.active_item, Some(ActiveItem::Stack(i)) if i.food_value().is_some())
        {
            return Ok(false);
        }
        let page = pages
            .get_mut(&target.key())
            .ok_or("unreserved torch page")?;
        let index = page.entities.iter().position(|e| {
            matches!(e.kind, Kind::Torch)
                && e.x == target.x.rem_euclid(64) as u8
                && e.y == target.y.rem_euclid(64) as u8
        });
        if let Some(index) = index {
            if matches!(action, Action::Pickup) {
                page.entities.remove(index);
                page.insert(
                    target,
                    Kind::Drop(ItemDrop {
                        loot: Loot::Stack(ItemStack::new(ItemId::Torch, 1)),
                        delay: 0,
                    }),
                    None,
                )?;
            }
            return Ok(true);
        }
        if !matches!(action, Action::Attack)
            || s.player.active_item != Some(ActiveItem::Stack(ItemId::Torch))
        {
            return Ok(false);
        }
        if s.player.attack_time > 0 || s.player.stamina == 0 {
            return Ok(true);
        }
        let cell = self.cell_at(target.depth, target.x, target.y)?;
        if !matches!(
            cell.ground,
            Ground::Dirt
                | Ground::Grass
                | Ground::Sand
                | Ground::WoodFloor
                | Ground::StoneFloor
                | Ground::RawStone
                | Ground::Obsidian
                | Ground::RawObsidian
                | Ground::OrnateObsidian
        ) || super::stations::contains(pages, target)
        {
            return Ok(true);
        }
        if !s.player.inventory.remove(ItemId::Torch, 1) {
            return Err("selected Torch missing".into());
        }
        pages
            .get_mut(&target.key())
            .unwrap()
            .insert(target, Kind::Torch, None)?;
        s.player.attack_time = 10;
        s.cleanup_active();
        Ok(true)
    }
}
