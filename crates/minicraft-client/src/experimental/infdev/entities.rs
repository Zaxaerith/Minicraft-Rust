//! Chunk-local persistent entities. Identity is (depth/chunk X/chunk Y, id),
//! never an i32 pixel coordinate. Inventory is the original shared item model.
use super::{
    paged::Key,
    survival::{self, DeathRule, Panel, State},
};
use crate::{entity::furniture::chest::death_chest_lifetime, item::Inventory};
use serde::{Deserialize, Serialize};

pub mod dynamic;

pub(crate) const PAGE_LIMIT: usize = 256 * 1024;
const MAX_ENTITIES: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Position {
    pub depth: i8,
    pub x: i64,
    pub y: i64,
}
impl Position {
    pub fn key(self) -> Key {
        Key::at_depth(self.depth, self.x, self.y)
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeathChest {
    pub inventory: Inventory,
    pub remaining: u32,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) enum Kind {
    DeathChest(DeathChest),
    Pig(dynamic::Pig),
    Animal(dynamic::Animal),
    Drop(dynamic::ItemDrop),
}

impl Kind {
    pub fn animal(&self) -> Option<&dynamic::Animal> {
        match self {
            Self::Pig(a) | Self::Animal(a) => Some(a),
            _ => None,
        }
    }
    pub fn animal_mut(&mut self) -> Option<&mut dynamic::Animal> {
        match self {
            Self::Pig(a) | Self::Animal(a) => Some(a),
            _ => None,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entity {
    pub id: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub birth: Option<dynamic::Birth>,
    pub x: u8,
    pub y: u8,
    pub kind: Kind,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Page {
    pub key: Key,
    next_id: u64,
    pub entities: Vec<Entity>,
}
impl Page {
    pub fn empty(key: Key) -> Self {
        Self {
            key,
            next_id: 1,
            entities: Vec::new(),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self.key.valid(true) || self.next_id == 0 || self.entities.len() > MAX_ENTITIES {
            return Err("invalid entity page bounds".into());
        }
        let mut ids = std::collections::BTreeSet::new();
        for e in &self.entities {
            if e.id == 0 || e.id >= self.next_id || !ids.insert(e.id) || e.x >= 64 || e.y >= 64 {
                return Err("invalid/duplicate chunk entity identity".into());
            }
            if let Some(b) = &e.birth {
                b.validate()?;
            }
            if !matches!(e.kind, Kind::DeathChest(_)) && e.birth.is_none() {
                return Err("dynamic entity missing birth identity".into());
            }
            match &e.kind {
                Kind::Pig(p) => {
                    p.validate()?;
                    if p.mob.species != crate::entity::mob::NaturalMob::Pig
                        || p.knock_x != 0
                        || p.knock_y != 0
                        || p.idle != 0
                    {
                        return Err("PigV1 state changed meaning".into());
                    }
                    continue;
                }
                Kind::Animal(a) => {
                    a.validate()?;
                    continue;
                }
                Kind::Drop(d) => {
                    survival::validate_loot(d.loot)?;
                    if d.delay > 60 {
                        return Err("invalid drop delay".into());
                    }
                    continue;
                }
                Kind::DeathChest(_) => (),
            }
            let Kind::DeathChest(chest) = &e.kind else {
                unreachable!()
            };
            if chest.inventory.capacity() != 27
                || chest.inventory.used_slots() > 27
                || chest.inventory.used_slots() == 0
                || chest.remaining > death_chest_lifetime(1)
            {
                return Err("invalid death chest inventory/time".into());
            }
            survival::validate_inventory(&chest.inventory)?;
        }
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > PAGE_LIMIT {
            return Err("entity page exceeds size bound".into());
        }
        Ok(())
    }
    pub fn position(&self, e: &Entity) -> Position {
        // validate() guarantees the signed tile domain, including both ends.
        Position {
            depth: self.key.depth,
            x: self.key.x * 64 + i64::from(e.x),
            y: self.key.y * 64 + i64::from(e.y),
        }
    }
    pub fn reserve(&self, count: usize) -> Result<(), String> {
        if self.entities.len() + count > MAX_ENTITIES
            || self.next_id.checked_add(count as u64).is_none()
            || serde_json::to_vec(self).map_err(|e| e.to_string())?.len() + count * 2048
                > PAGE_LIMIT
        {
            return Err("entity page full; action retained unchanged".into());
        }
        Ok(())
    }
    pub fn insert(
        &mut self,
        at: Position,
        kind: Kind,
        birth: Option<dynamic::Birth>,
    ) -> Result<(), String> {
        self.reserve(1)?;
        if at.key() != self.key {
            return Err("entity destination page mismatch".into());
        }
        let id = self.next_id;
        self.next_id += 1;
        self.entities.push(Entity {
            id,
            birth: Some(birth.unwrap_or(dynamic::Birth { key: self.key, id })),
            x: at.x.rem_euclid(64) as u8,
            y: at.y.rem_euclid(64) as u8,
            kind,
        });
        Ok(())
    }
    pub fn record_death(&mut self, state: &mut State, at: Position, can_place: bool) {
        if state.death_rule != DeathRule::ChestNormal || state.death_recorded {
            return;
        }
        state.death_recorded = true;
        state.death_site = Some(at);
        if state.player.inventory.used_slots() == 0 {
            return;
        }
        // Java uses an unlimited chest for inventory + independently held item.
        // Here active is an alias into the same 27-slot Inventory: move it ONCE.
        let Some(next) = self.next_id.checked_add(1) else {
            state.death_retained = true;
            return;
        };
        if !can_place || self.entities.len() == MAX_ENTITIES {
            state.death_retained = true;
            return;
        }
        let candidate = Entity {
            id: self.next_id,
            birth: None,
            x: at.x.rem_euclid(64) as u8,
            y: at.y.rem_euclid(64) as u8,
            kind: Kind::DeathChest(DeathChest {
                inventory: state.player.inventory.clone(),
                remaining: death_chest_lifetime(1),
            }),
        };
        self.entities.push(candidate);
        let old = self.next_id;
        self.next_id = next;
        if self.validate().is_err() {
            self.entities.pop();
            self.next_id = old;
            state.death_retained = true;
            return;
        }
        self.entities.last_mut().expect("inserted").kind = Kind::DeathChest(DeathChest {
            inventory: std::mem::replace(&mut state.player.inventory, Inventory::new(27)),
            remaining: death_chest_lifetime(1),
        });
        state.player.active_item = None;
        state.panel = Panel::Closed;
    }
    pub fn tick(&mut self, at: Position, player: &mut Inventory) {
        for e in &mut self.entities {
            let Kind::DeathChest(chest) = &mut e.kind else {
                continue;
            };
            // Unloaded/offline chunks do not age. At zero retain a recovery
            // chest instead of losing items to unimplemented drop expiry.
            chest.remaining = chest.remaining.saturating_sub(1);
            if e.x == at.x.rem_euclid(64) as u8 && e.y == at.y.rem_euclid(64) as u8 {
                take(&mut chest.inventory, player);
            }
        }
        self.remove_empty();
    }
    pub fn interact(&mut self, at: Position, player: &mut Inventory) -> bool {
        let mut found = false;
        for e in &mut self.entities {
            if e.x == at.x.rem_euclid(64) as u8 && e.y == at.y.rem_euclid(64) as u8 {
                found = true;
                let Kind::DeathChest(chest) = &mut e.kind else {
                    continue;
                };
                take(&mut chest.inventory, player);
            }
        }
        self.remove_empty();
        found
    }
    fn remove_empty(&mut self) {
        self.entities.retain(|e| {
            let Kind::DeathChest(c) = &e.kind else {
                return true;
            };
            c.inventory.used_slots() != 0
        });
    }
}

fn take(source: &mut Inventory, target: &mut Inventory) {
    let mut i = 0;
    while let Some(stack) = source.slots().get(i).copied() {
        let remaining = target.add_stack(stack);
        if remaining < stack.count {
            source.remove_slot(i, stack.count - remaining);
        }
        if remaining > 0 {
            i += 1;
        }
    }
    while let Some(tool) = source.tools().first().copied() {
        if target.add_tool(tool).is_none() {
            break;
        }
        source.take_tool(0);
    }
}
