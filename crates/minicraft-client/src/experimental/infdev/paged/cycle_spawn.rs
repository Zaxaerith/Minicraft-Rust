//! Original Level.trySpawn's single candidate stream: hostile first, then
//! surface passive fallback. Unsupported species consume a valid attempt.
use super::super::{
    entities::{Kind, Page, Position, dynamic::Animal},
    survival::State,
};
use super::*;
use crate::{entity::mob::NaturalMob, world::spawn};
impl World {
    pub(super) fn spawn_environment(
        &mut self,
        s: &mut State,
        pages: &mut BTreeMap<Key, Page>,
        at: Position,
    ) -> Result<(), String> {
        if !(-3..=0).contains(&at.depth) {
            return Ok(());
        }
        let env = s.environment.as_ref().ok_or("missing spawn calendar")?;
        let (night, past_day_one) = (env.night(), env.past_day_one);
        let passive_radius = if night { 15 } else { 20 };
        let count = pages
            .values()
            .flat_map(|p| &p.entities)
            .filter(|e| e.kind.mover().is_some())
            .count();
        let max = spawn::max_mob_count(at.depth, 1);
        let skip = spawn::spawn_skip_chance(count, max);
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
            let Some(page) = pages.get(&pos.key()) else {
                continue;
            };
            if page.reserve(1).is_err() {
                continue;
            }
            let Some(cell) = self.loaded_cell(pos) else {
                continue;
            };
            let torch_here = page.entities.iter().any(|e| {
                matches!(e.kind, Kind::Torch)
                    && e.x == x.rem_euclid(64) as u8
                    && e.y == y.rem_euclid(64) as u8
            });
            if torch_here {
                continue;
            }
            let occupied = |radius: i64| {
                pages.values().any(|p| {
                    p.entities.iter().any(|e| {
                        if matches!(e.kind, Kind::Torch) {
                            return false;
                        }
                        let q = p.position(e);
                        let (px, py) = e
                            .kind
                            .mover()
                            .map_or((8, 8), |a| (i32::from(a.px), i32::from(a.py)));
                        super::passive::delta(q.x, px, x, s.player.x).abs() <= radius * 16 + 4
                            && super::passive::delta(q.y, py, y, s.player.y).abs()
                                <= radius * 16 + 3
                    })
                })
            };
            let hostile = (at.depth != 0 || night && past_day_one)
                && (dx * dx + dy * dy) * 256 >= 60 * 60
                && !(dx.abs() * 16 < 212 && dy.abs() * 16 < 211)
                && spawn::hostile_allowed(
                    at.depth,
                    if night { 48600 } else { 0 },
                    if past_day_one { 2 } else { 1 },
                    super::super::survival::tile(cell),
                    false,
                )
                && !occupied(13);
            let species = if hostile && !self.torch_lit(pos, pages)? {
                let species = spawn::choose_hostile(at.depth, roll);
                if !matches!(species, NaturalMob::Slime | NaturalMob::Zombie) {
                    break;
                }
                species
            } else if at.depth == 0
                && spawn::passive_allowed(0, super::super::survival::tile(cell))
                && dx * dx + dy * dy >= 25
                && !(dx.abs() <= passive_radius && dy.abs() <= passive_radius)
                && !occupied(i64::from(passive_radius))
            {
                spawn::choose_passive(night, roll)
            } else {
                continue;
            };
            let mut body = Animal::of_level(
                species,
                if species.hostile() {
                    (-at.depth).max(1) as u8
                } else {
                    1
                },
            );
            if species == NaturalMob::Sheep {
                body.mob.sheep_color = spawn::sheep_color(s.random.next_double());
            }
            body.px = s.player.x as u8;
            body.py = s.player.y as u8;
            pages.get_mut(&pos.key()).unwrap().insert(
                pos,
                if species.hostile() {
                    Kind::Enemy(body)
                } else {
                    Kind::Animal(body)
                },
                None,
            )?;
            break;
        }
        Ok(())
    }
}
