//! Original EnemyMob/Zombie rules, restricted to the currently unlit -1..-3 domain.
use super::passive::delta;
use super::*;
use crate::{
    entity::mob::NaturalMob,
    experimental::infdev::{
        entities::{Kind, Page, Position, dynamic::Animal},
        survival::{EnemyRules, State},
    },
    level::noise::JavaRandom,
    world::Player,
};
pub(super) fn touches(pos: Position, px: i32, py: i32, at: Position, p: &Player) -> bool {
    pos.depth == at.depth
        && delta(pos.x, px, at.x, p.x).abs() <= 8
        && delta(pos.y, py, at.y, p.y).abs() <= 6
}
pub(super) fn hurt_player(s: &mut State, level: u8) {
    if s.player.health > 0 && s.player.hurt_time == 0 {
        let damage = crate::entity::stats::mob_contact_damage(NaturalMob::Zombie, level, 1);
        s.player.health = s.player.health.saturating_sub(damage);
        s.player.hurt_time = 30;
    }
}
pub(super) fn expired(e: &Animal, pos: Position, at: Position, p: &Player) -> bool {
    // EnemyMob.handleDespawn does not call MobAi.handleDespawn underground.
    e.age > 3600
        && !(delta(pos.x, i32::from(e.px), at.x, p.x).abs() < 128
            && delta(pos.y, i32::from(e.py), at.y, p.y).abs() < 76)
}
fn randomize(e: &mut Animal, r: &mut JavaRandom, by_chance: bool) {
    if !by_chance && r.next_int(200) != 0 {
        return;
    }
    e.mob.walk_time = 60;
    e.mob.x_move = r.next_int(3) - 1;
    e.mob.y_move = r.next_int(3) - 1;
}
pub(super) fn think(e: &mut Animal, pos: Position, at: Position, p: &Player, r: &mut JavaRandom) {
    if r.next_int(200) == 0 {
        randomize(e, r, true);
    }
    e.mob.walk_time = e.mob.walk_time.saturating_sub(1);
    if e.mob.walk_time == 0 {
        let dx = delta(at.x, p.x, pos.x, i32::from(e.px));
        let dy = delta(at.y, p.y, pos.y, i32::from(e.py));
        if dx * dx + dy * dy < 100 * 100 {
            // Preserve the original sig0=1 asymmetry, including aligned axes.
            e.mob.x_move = if dx < 1 { -1 } else { i32::from(dx > 1) };
            e.mob.y_move = if dy < 1 { -1 } else { i32::from(dy > 1) };
        } else {
            randomize(e, r, false);
        }
    }
}
impl World {
    #[cfg(test)]
    pub(in crate::experimental::infdev) fn passive_fixture(&mut self) {
        self.survival.as_mut().unwrap().enemy_rules = EnemyRules::None;
    }
    pub(super) fn spawn_enemy(
        &self,
        s: &mut State,
        pages: &mut BTreeMap<Key, Page>,
        at: Position,
    ) -> Result<(), String> {
        if s.enemy_rules != EnemyRules::UndergroundZombieV1 || !(-3..=-1).contains(&at.depth) {
            return Ok(());
        }
        let count = pages
            .values()
            .flat_map(|p| &p.entities)
            .filter(|e| e.kind.mover().is_some())
            .count();
        let max = crate::world::spawn::max_mob_count(at.depth, 1);
        let skip = crate::world::spawn::spawn_skip_chance(count, max);
        if count >= max || skip > 0 && s.random.next_int(skip as i32) != 0 {
            return Ok(());
        }
        for _ in 0..30 {
            let roll = s.random.next_int(100);
            let dx = s.random.next_int(128) - 64;
            let dy = s.random.next_int(128) - 64;
            let Some((x, y)) =
                at.x.checked_add(i64::from(dx))
                    .zip(at.y.checked_add(i64::from(dy)))
            else {
                continue;
            };
            let pos = Position {
                depth: at.depth,
                x,
                y,
            };
            if (dx * dx + dy * dy) * 256 < 60 * 60 || dx.abs() * 16 < 212 && dy.abs() * 16 < 211 {
                continue;
            }
            let Some(page) = pages.get(&pos.key()) else {
                continue;
            };
            let Some(c) = self.loaded_cell(pos) else {
                continue;
            };
            // Torch/Lantern are not representable by this experimental terrain/station
            // schema. Do not claim full lighting or surface night spawning support.
            if !crate::world::spawn::hostile_allowed(
                at.depth,
                0,
                0,
                super::super::survival::tile(c),
                false,
            ) || s.station_at(at.depth, x, y)
            {
                continue;
            }
            let occupied = pages.values().any(|p| {
                p.entities.iter().any(|e| {
                    let q = p.position(e);
                    let (px, py) = e
                        .kind
                        .mover()
                        .map_or((8, 8), |a| (i32::from(a.px), i32::from(a.py)));
                    delta(q.x, px, x, s.player.x).abs() <= 212
                        && delta(q.y, py, y, s.player.y).abs() <= 211
                })
            }) || s
                .stations
                .iter()
                .any(|q| q.depth == at.depth && q.x.abs_diff(x) <= 13 && q.y.abs_diff(y) <= 13);
            if occupied || page.reserve(1).is_err() {
                continue;
            }
            // The first valid unsupported species still consumes the attempt.
            if crate::world::spawn::choose_hostile(at.depth, roll) == NaturalMob::Zombie {
                let mut mob = Animal::of_level(NaturalMob::Zombie, (-at.depth) as u8);
                mob.px = s.player.x as u8;
                mob.py = s.player.y as u8;
                pages
                    .get_mut(&pos.key())
                    .unwrap()
                    .insert(pos, Kind::Enemy(mob), None)?;
            }
            break;
        }
        Ok(())
    }
}
