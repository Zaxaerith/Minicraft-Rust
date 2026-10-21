//! Bounded active-page transactions. All fallible reads precede installing any
//! player/page owner; moving entities publish source and destination together.
use super::*;
use crate::experimental::infdev::{
    entities::{
        Kind, Page, Position,
        dynamic::{ItemDrop, Pig, animal_drops, pig_drop, take_drop},
    },
    survival::{self, Action, Loot, Panel, State},
};
use crate::{item::ToolKind, world::ActiveItem};

type Pages = BTreeMap<Key, Page>;
impl World {
    pub(super) fn loaded_cell(&self, at: Position) -> Option<Cell> {
        self.resident
            .get(&at.key())
            .map(|c| c.cells[Key::local(at.x, at.y)])
    }
    fn dynamic_pages(&mut self, at: Position, target: Position) -> Result<Pages, String> {
        let center = at.key();
        let mut keys: Vec<_> = self
            .resident
            .keys()
            .copied()
            .filter(|k| {
                k.depth == at.depth && k.x.abs_diff(center.x) <= 1 && k.y.abs_diff(center.y) <= 1
            })
            .collect();
        keys.extend([at.key(), target.key()]);
        keys.sort();
        keys.dedup();
        let mut pages = Pages::new();
        for key in keys {
            pages.insert(key, self.entity_page(key)?);
        }
        Ok(pages)
    }
    pub(super) fn act_dynamic(&mut self, action: Action) -> Result<bool, String> {
        let mut state = self.survival.clone().ok_or("not Survival")?;
        let (x, y) = self.position();
        let at = Position {
            depth: self.depth(),
            x,
            y,
        };
        let eating = matches!(action, Action::Attack)
            && matches!(state.player.active_item, Some(ActiveItem::Stack(id)) if id.food_value().is_some());
        let target = if !eating
            && matches!(action, Action::Attack | Action::Interact | Action::Pickup)
            && state.player.health > 0
        {
            let (x, y) = state.target(self)?;
            Position {
                depth: at.depth,
                x,
                y,
            }
        } else {
            at
        };
        // No entity changes for ordinary movement/menu actions. Original map
        // reads may publish only the old complete transaction during eviction.
        if !matches!(
            action,
            Action::Tick
                | Action::Attack
                | Action::Interact
                | Action::Pickup
                | Action::Drop(_)
                | Action::DiagnosticDeath
        ) && state.player.health > 0
        {
            state.act(self, action)?;
            self.survival = Some(state);
            return Ok(false);
        }
        let mut pages = self.dynamic_pages(at, target)?;
        let before = pages.clone();
        super::stations::project(&mut state, &pages, target)?;
        let was_dead = state.player.health == 0;
        let health_before = state.player.health;
        let hurt_before = state.player.hurt_time;
        let receipt = state.death_recorded;
        let cell = self.cell_at(at.depth, at.x, at.y)?;
        let can_place = !survival::tile(cell).solid(cell.data)
            && !state.station_at(at.depth, at.x, at.y)
            && !super::stations::contains(&pages, at);
        if state.player.health == 0 {
            pages
                .get_mut(&at.key())
                .unwrap()
                .record_death(&mut state, at, can_place);
        }
        // Reserve the maximum original mining outbox before any map/inventory
        // mutation. A full destination refuses the action, never deletes loot.
        if !was_dead
            && !eating
            && state.panel == Panel::Closed
            && matches!(action, Action::Attack | Action::Drop(_) | Action::Pickup)
        {
            pages.get(&target.key()).unwrap().reserve(3)?;
            pages.get(&at.key()).unwrap().reserve(1)?;
        }
        let mut grazes = Vec::new();
        if !was_dead && state.panel == Panel::Closed && matches!(action, Action::Tick) {
            grazes = self.tick_animals(&mut state, &mut pages, at)?;
            self.spawn_animal(&mut state, &mut pages, at)?;
            if state.enemy_rules == survival::EnemyRules::UndergroundZombieV1 {
                self.spawn_enemy(&mut state, &mut pages, at)?;
            }
        }
        let hit = !was_dead
            && state.panel == Panel::Closed
            && matches!(action, Action::Attack)
            && hit_animal(&mut state, &mut pages, target)?;
        let undo = if matches!(action, Action::Attack) && !hit {
            Some(self.cell_at(target.depth, target.x, target.y)?)
        } else {
            None
        };
        let result = (|| {
            if !hit {
                state.act(self, action)?;
            }
            // Original mining/drop/station code remains the producer of all items.
            // This outbox never persists in new-rule sessions and is moved, not copied.
            let produced = !state.drops.is_empty();
            for drop in std::mem::take(&mut state.drops) {
                let pos = Position {
                    depth: drop.depth,
                    x: drop.x,
                    y: drop.y,
                };
                pages
                    .get_mut(&pos.key())
                    .ok_or("unreserved drop destination")?
                    .insert(
                        pos,
                        Kind::Drop(ItemDrop {
                            loot: drop.loot,
                            delay: drop.delay,
                        }),
                        None,
                    )?;
            }
            let mut taken = false;
            if state.player.health == 0 {
                pages
                    .get_mut(&at.key())
                    .unwrap()
                    .record_death(&mut state, at, can_place);
            } else if matches!(action, Action::Tick) && state.panel == Panel::Closed {
                for page in pages.values_mut() {
                    for e in &mut page.entities {
                        let Kind::Drop(d) = &mut e.kind else { continue };
                        d.delay = d.delay.saturating_sub(1);
                    }
                    let mut keep = Vec::new();
                    for mut e in std::mem::take(&mut page.entities) {
                        let pos = page.position(&e);
                        let mut empty = false;
                        if let Kind::Drop(d) = &mut e.kind
                            && d.delay == 0
                            && pos.x.abs_diff(at.x) <= 1
                            && pos.y.abs_diff(at.y) <= 1
                        {
                            let old = d.clone();
                            empty = take_drop(d, &mut state.player.inventory);
                            taken |= empty || *d != old;
                        }
                        if !empty {
                            keep.push(e);
                        }
                    }
                    page.entities = keep;
                }
                pages
                    .get_mut(&at.key())
                    .unwrap()
                    .tick(at, &mut state.player.inventory);
            } else if !was_dead && matches!(action, Action::Interact) {
                pages
                    .get_mut(&target.key())
                    .unwrap()
                    .interact(target, &mut state.player.inventory);
            }
            super::stations::collect(&mut state, &mut pages, target)?;
            state.validate()?;
            // Validation cannot perform IO; no checkpoint between page installs and
            // the corresponding session. Candidate pages remain local until here.
            for page in pages.values() {
                page.validate()?;
            }
            let death_change = pages
                .iter()
                .any(|(k, p)| death_contents(p) != death_contents(&before[k]));
            let migrated = pages
                .iter()
                .any(|(k, p)| p.entities.len() != before[k].entities.len());
            let critical = (state.enemy_rules == survival::EnemyRules::UndergroundZombieV1
                && (state.player.health != health_before || state.player.hurt_time > hurt_before))
                || !grazes.is_empty()
                || migrated
                || produced
                || taken
                || hit
                || death_change
                || !receipt && state.death_recorded;
            // Grazing and regrowth join the SAME publication as the animal.
            for pos in grazes {
                let chunk = self
                    .resident
                    .get_mut(&pos.key())
                    .expect("loaded grazing tile; no later terrain IO on tick");
                chunk.cells[Key::local(pos.x, pos.y)] = Cell {
                    ground: Ground::Dirt,
                    data: 0,
                };
                chunk.dirty = true;
            }
            for (key, page) in pages {
                if page != before[&key] {
                    state.entity_pages_present = true;
                    self.clock = self.clock.wrapping_add(1);
                    self.entity_cache.insert(
                        key,
                        entities::CachedEntities {
                            page,
                            dirty: true,
                            touched: self.clock,
                        },
                    );
                }
            }
            self.survival = Some(state);
            Ok(critical)
        })();
        if result.is_err() {
            // No IO follows the game's final tile write. Preserve the old map
            // too if a future producer violates the validated outbox contract.
            if let Some(old) = undo
                && let Some(chunk) = self.resident.get_mut(&target.key())
            {
                chunk.cells[Key::local(target.x, target.y)] = old;
                chunk.dirty = true;
            }
            self.saved.position = (at.x, at.y);
            self.saved.depth = at.depth;
        }
        result
    }
    pub(super) fn trim_entities(&mut self) -> Result<(), String> {
        if self.entity_cache.len() <= self.capacity {
            return Ok(());
        }
        self.checkpoint()?;
        while self.entity_cache.len() > self.capacity {
            let key = *self
                .entity_cache
                .iter()
                .min_by_key(|(_, c)| c.touched)
                .unwrap()
                .0;
            self.entity_cache.remove(&key);
        }
        Ok(())
    }
    pub(super) fn tick_animals(
        &self,
        state: &mut State,
        pages: &mut Pages,
        at: Position,
    ) -> Result<Vec<Position>, String> {
        let full = state.entity_rules == survival::EntityRules::PassiveV1;
        let mut grazes = Vec::new();
        // Iterate a snapshot once: a migrant is never updated again by its
        // destination page in this tick. At most9 nearby resident terrain pages.
        let todo: Vec<_> = pages
            .iter()
            .flat_map(|(&key, p)| {
                p.entities
                    .iter()
                    .filter(|e| e.kind.mover().is_some())
                    .map(move |e| (key, e.id))
            })
            .collect();
        for (key, id) in todo {
            let index = pages[&key]
                .entities
                .iter()
                .position(|e| e.id == id)
                .unwrap();
            let mut entity = pages[&key].entities[index].clone();
            let mut pos = pages[&key].position(&entity);
            let pig = entity.kind.mover_mut().unwrap();
            pig.age = if full {
                pig.age.saturating_add(1)
            } else {
                pig.age.wrapping_add(1)
            };
            let hostile = pig.mob.species.hostile();
            if if hostile {
                super::enemy::expired(pig, pos, at, &state.player)
            } else {
                full && super::passive::despawn(pig, pos, at, &state.player, &mut state.random)
            } {
                pages.get_mut(&key).unwrap().entities.remove(index);
                continue;
            }
            pig.mob.hurt_time = pig.mob.hurt_time.saturating_sub(1);
            let knocked = full && (pig.knock_x != 0 || pig.knock_y != 0);
            let motion = if knocked {
                (i32::from(pig.knock_x / 2), i32::from(pig.knock_y / 2))
            } else {
                (pig.mob.x_move, pig.mob.y_move)
            };
            if full {
                pig.knock_x -= pig.knock_x.signum();
                pig.knock_y -= pig.knock_y.signum();
            }
            if knocked || (!pig.age.is_multiple_of(2) && pig.mob.hurt_time == 0) {
                if pig.mob.x_move != 0 || pig.mob.y_move != 0 {
                    pig.mob.walk_distance = pig.mob.walk_distance.wrapping_add(1);
                }
                for axis in [true, false] {
                    let (dx, dy) = if axis { (motion.0, 0) } else { (0, motion.1) };
                    let (px, py) = (i32::from(pig.px) + dx, i32::from(pig.py) + dy);
                    let Some((x, y)) = pos
                        .x
                        .checked_add(i64::from(px.div_euclid(16)))
                        .zip(pos.y.checked_add(i64::from(py.div_euclid(16))))
                    else {
                        continue;
                    };
                    let next = Position {
                        depth: pos.depth,
                        x,
                        y,
                    };
                    let terrain_legal = pages.contains_key(&next.key())
                        && [(-4, -3), (4, -3), (-4, 3), (4, 3)]
                            .into_iter()
                            .all(|(bx, by)| {
                                let Some((x, y)) = next
                                    .x
                                    .checked_add(i64::from((px.rem_euclid(16) + bx).div_euclid(16)))
                                    .zip(next.y.checked_add(i64::from(
                                        (py.rem_euclid(16) + by).div_euclid(16),
                                    )))
                                else {
                                    return false;
                                };
                                let q = Position {
                                    depth: next.depth,
                                    x,
                                    y,
                                };
                                pages.contains_key(&q.key())
                                    && !state.station_at(q.depth, x, y)
                                    && !super::stations::contains(pages, q)
                                    && self.loaded_cell(q).is_some_and(|c| {
                                        !survival::tile(c).solid(c.data)
                                            && !matches!(
                                                c.ground,
                                                Ground::Hole | Ground::Water | Ground::Lava
                                            )
                                    })
                            })
                        && self.loaded_cell(next).is_some_and(|c| {
                            !survival::tile(c).solid(c.data)
                                && !matches!(c.ground, Ground::Hole | Ground::Water | Ground::Lava)
                        })
                        && !state.station_at(next.depth, x, y)
                        && !super::stations::contains(pages, next);
                    if terrain_legal
                        && (dx != 0 || dy != 0)
                        && hostile
                        && super::enemy::touches(
                            next,
                            px.rem_euclid(16),
                            py.rem_euclid(16),
                            at,
                            &state.player,
                        )
                    {
                        super::enemy::hurt_player(state, pig.mob.level);
                    }
                    let legal = terrain_legal
                        && if full {
                            !super::passive::blocked(
                                pages,
                                entity.birth,
                                next,
                                px.rem_euclid(16),
                                py.rem_euclid(16),
                                Some((at, state.player.x, state.player.y)),
                            )
                        } else {
                            !(x == at.x && y == at.y)
                        };
                    if legal {
                        pos = next;
                        pig.px = px.rem_euclid(16) as u8;
                        pig.py = py.rem_euclid(16) as u8;
                    } else if axis {
                        pig.mob.x_move = 0;
                    } else {
                        pig.mob.y_move = 0;
                    }
                }
            }
            if hostile {
                super::enemy::think(pig, pos, at, &state.player, &mut state.random);
            } else {
                pig.reroll(&mut state.random);
            }
            let grazed =
                full && matches!(
                    pig.mob.species,
                    crate::entity::mob::NaturalMob::Cow | crate::entity::mob::NaturalMob::Sheep
                ) && !grazes.contains(&pos)
                    && self
                        .loaded_cell(pos)
                        .is_some_and(|c| c.ground == Ground::Grass)
                    && state.random.next_int(1000) == 0;
            if grazed {
                pig.mob.sheared = false;
            }
            entity.x = pos.x.rem_euclid(64) as u8;
            entity.y = pos.y.rem_euclid(64) as u8;
            let accepted = pos.key() == key || pages[&pos.key()].reserve(1).is_ok();
            if grazed && accepted {
                grazes.push(pos);
            }
            if pos.key() != key {
                if accepted {
                    pages.get_mut(&pos.key()).unwrap().insert(
                        pos,
                        entity.kind.clone(),
                        entity.birth,
                    )?;
                    pages.get_mut(&key).unwrap().entities.remove(index);
                } // Full destination retains the original owner and position.
            } else {
                pages.get_mut(&key).unwrap().entities[index] = entity;
            }
        }
        Ok(grazes)
    }
    pub(super) fn spawn_animal(
        &self,
        state: &mut State,
        pages: &mut Pages,
        at: Position,
    ) -> Result<(), String> {
        if at.depth != 0 {
            return Ok(());
        }
        let count = pages
            .values()
            .flat_map(|p| &p.entities)
            .filter(|e| e.kind.animal().is_some())
            .count();
        let max = crate::world::spawn::max_mob_count(0, 1);
        let skip = crate::world::spawn::spawn_skip_chance(count, max);
        if count >= max || skip > 0 && state.random.next_int(skip as i32) != 0 {
            return Ok(());
        }
        for _ in 0..30 {
            let roll = state.random.next_int(100);
            let (dx, dy) = (
                state.random.next_int(128) - 64,
                state.random.next_int(128) - 64,
            );
            let Some((x, y)) =
                at.x.checked_add(i64::from(dx))
                    .zip(at.y.checked_add(i64::from(dy)))
            else {
                continue;
            };
            let pos = Position { depth: 0, x, y };
            if dx * dx + dy * dy < 25 || dx.abs() < 20 && dy.abs() < 20 {
                continue;
            }
            let Some(page) = pages.get(&pos.key()) else {
                continue;
            };
            let Some(c) = self.loaded_cell(pos) else {
                continue;
            };
            if !crate::world::spawn::passive_allowed(0, survival::tile(c))
                || state.station_at(0, x, y)
            {
                continue;
            }
            let occupied = pages.values().any(|p| {
                p.entities.iter().any(|e| {
                    let q = p.position(e);
                    q.x.abs_diff(x) < 20 && q.y.abs_diff(y) < 20
                })
            }) || state
                .stations
                .iter()
                .any(|s| s.depth == 0 && s.x.abs_diff(x) < 20 && s.y.abs_diff(y) < 20);
            if occupied || page.reserve(1).is_err() {
                continue;
            }
            let species = crate::world::spawn::choose_passive(false, roll);
            let full = state.entity_rules == survival::EntityRules::PassiveV1;
            if !full && species != crate::entity::mob::NaturalMob::Pig {
                break;
            }
            let mut pig = if full { Pig::of(species) } else { Pig::new() };
            if species == crate::entity::mob::NaturalMob::Sheep {
                pig.mob.sheep_color = crate::world::spawn::sheep_color(state.random.next_double());
            }
            pig.px = state.player.x as u8;
            pig.py = state.player.y as u8;
            pages.get_mut(&pos.key()).unwrap().insert(
                pos,
                if full {
                    Kind::Animal(pig)
                } else {
                    Kind::Pig(pig)
                },
                None,
            )?;
            break;
        }
        Ok(())
    }
}
fn death_contents(page: &Page) -> Vec<(u64, crate::item::Inventory)> {
    page.entities
        .iter()
        .filter_map(|e| {
            if let Kind::DeathChest(c) = &e.kind {
                Some((e.id, c.inventory.clone()))
            } else {
                None
            }
        })
        .collect()
}
pub(super) fn hit_animal(
    state: &mut State,
    pages: &mut Pages,
    target: Position,
) -> Result<bool, String> {
    use crate::{
        entity::mob::NaturalMob,
        item::{ItemStack, dye::DyeColor},
    };
    if state.player.attack_time > 0 || state.player.stamina == 0 {
        return Ok(false);
    }
    let full = state.entity_rules == survival::EntityRules::PassiveV1;
    let page = pages.get_mut(&target.key()).unwrap();
    let Some(index) = page.entities.iter().position(|e| {
        e.kind.mover().is_some()
            && e.x == target.x.rem_euclid(64) as u8
            && e.y == target.y.rem_euclid(64) as u8
    }) else {
        return Ok(false);
    };
    let mut entity = page.entities[index].clone();
    let pig = entity.kind.mover_mut().unwrap();
    if full && pig.mob.species == NaturalMob::Sheep {
        if let Some(ActiveItem::Stack(id)) = state.player.active_item
            && let Some(color) = DyeColor::from_dye_item(id)
        {
            if !state.player.inventory.remove(id, 1) {
                return Err("selected dye missing".into());
            }
            pig.mob.sheep_color = color as u8;
            if state.player.inventory.count(id) == 0 {
                state.player.active_item = None;
            }
            state.player.attack_time = 10;
            page.entities[index] = entity;
            return Ok(true);
        }
        if !pig.mob.sheared
            && state
                .tool()
                .is_some_and(|t| t.kind == ToolKind::Shears && t.durability > 0)
        {
            let color = DyeColor::from_index(u16::from(pig.mob.sheep_color)).unwrap();
            state.charge_tool(0).ok_or("depleted shears retained")?;
            let count = (state.random.next_int(3) + 1) as u16;
            pig.mob.sheared = true;
            state.player.attack_time = 10;
            page.entities[index] = entity;
            page.insert(
                target,
                Kind::Drop(ItemDrop {
                    loot: Loot::Stack(ItemStack::new(color.wool_item(), count)),
                    delay: 0,
                }),
                None,
            )?;
            return Ok(true);
        }
    }
    if matches!(state.player.active_item, Some(ActiveItem::Stack(_))) {
        return Ok(false);
    }
    if pig.mob.hurt_time > 0 {
        return Ok(true);
    }
    let mut damage = (1 + state.random.next_int(2)) as u16;
    if let Some(t) = state.charge_tool(0) {
        let bound = match t.kind {
            ToolKind::Axe => 4,
            ToolKind::Sword => 2 + i32::from(t.tier.level()).pow(2),
            ToolKind::Claymore => 4 + 3 * i32::from(t.tier.level()).pow(2),
            ToolKind::Pickaxe => 2,
            _ => 1,
        };
        damage += u16::from(t.melee_bonus(state.random.next_int(bound) as u8));
    }
    state.player.stamina -= 1;
    state.player.stamina_recharge = 0;
    state.player.attack_time = 10;
    pig.mob.health = pig.mob.health.saturating_sub(damage);
    pig.mob.hurt_time = 10;
    if full {
        let (dx, dy) = state.facing();
        pig.knock_x = (dx * 6) as i8;
        pig.knock_y = (dy * 6) as i8;
    }
    if pig.mob.health == 0 {
        let drops = if pig.mob.species.hostile() {
            crate::entity::stats::mob_drops_at_level(
                pig.mob.species,
                pig.mob.level,
                1,
                false,
                &mut state.random,
            )
        } else if full {
            animal_drops(pig, &mut state.random)
        } else {
            vec![pig_drop(&mut state.random)]
        };
        let mut drops = drops.into_iter();
        entity.kind = Kind::Drop(ItemDrop {
            loot: Loot::Stack(drops.next().expect("Normal passive drop")),
            delay: 0,
        });
        page.entities[index] = entity;
        for stack in drops {
            page.insert(
                target,
                Kind::Drop(ItemDrop {
                    loot: Loot::Stack(stack),
                    delay: 0,
                }),
                None,
            )?;
        }
    } else {
        page.entities[index] = entity;
    }
    Ok(true)
}
