//! i64-safe body queries and the bounded passive lifetime policy.
use super::*;
use crate::experimental::infdev::entities::{
    Page, Position,
    dynamic::{Animal, Birth},
};
use crate::{level::noise::JavaRandom, world::Player};
pub(super) fn delta(a: i64, ap: i32, b: i64, bp: i32) -> i64 {
    ((i128::from(a) - i128::from(b)) * 16 + i128::from(ap - bp)).clamp(-1281, 1281) as i64
}
pub(super) fn blocked(
    pages: &BTreeMap<Key, Page>,
    birth: Option<Birth>,
    at: Position,
    px: i32,
    py: i32,
    player: Option<(Position, i32, i32)>,
) -> bool {
    let overlaps = |q: Position, x: i32, y: i32| {
        q.depth == at.depth
            && delta(at.x, px, q.x, x).abs() < 8
            && delta(at.y, py, q.y, y).abs() < 6
    };
    if player.is_some_and(|(q, x, y)| overlaps(q, x, y)) {
        return true;
    }
    pages.values().any(|p| {
        p.entities.iter().any(|e| {
            if e.birth == birth && birth.is_some() {
                return false;
            }
            if matches!(e.kind, super::super::entities::Kind::Station(_)) {
                return overlaps(p.position(e), 8, 8);
            }
            e.kind
                .mover()
                .is_some_and(|a| overlaps(p.position(e), i32::from(a.px), i32::from(a.py)))
        })
    })
}
pub(super) fn despawn(
    animal: &mut Animal,
    pos: Position,
    at: Position,
    player: &Player,
    random: &mut JavaRandom,
) -> bool {
    let dx = delta(pos.x, i32::from(animal.px), at.x, player.x);
    let dy = delta(pos.y, i32::from(animal.py), at.y, player.y);
    let d = dx * dx + dy * dy;
    animal.idle = animal.idle.saturating_add(1);
    if d < 640 * 640 {
        animal.idle = 0;
    }
    // Original Level.distanceOfClosestPlayer incorrectly caps distance by w/h.
    // Use the true infinite-coordinate distance, preserving Mob thresholds.
    d > 1280 * 1280
        || animal.age > 18000 && !(dx.abs() < 128 && dy.abs() < 76)
        || animal.idle > 1800 && random.next_int(800) == 0 && d > 640 * 640
}
impl World {
    pub(crate) fn animal_blocks_player(
        &mut self,
        depth: i8,
        x: i64,
        y: i64,
        px: i32,
        py: i32,
    ) -> Result<bool, String> {
        let at = Position { depth, x, y };
        let mut pages = BTreeMap::new();
        for dx in [-1, 0, 1] {
            for dy in [-1, 0, 1] {
                if let Some((x, y)) = x.checked_add(dx).zip(y.checked_add(dy)) {
                    let key = Key::at_depth(depth, x, y);
                    if let std::collections::btree_map::Entry::Vacant(e) = pages.entry(key) {
                        e.insert(self.entity_page(key)?);
                    }
                }
            }
        }
        if let Some(state) = self
            .survival
            .as_ref()
            .filter(|s| s.enemy_rules == super::super::survival::EnemyRules::UndergroundZombieV1)
        {
            let (ox, oy) = self.position();
            let old = Position {
                depth,
                x: ox,
                y: oy,
            };
            // Entity.java's wasInside policy: entering a populated layer must
            // permit escaping an existing overlap, never delete the occupant.
            // Preserve historical PassiveV1 collision thresholds and behavior.
            let overlaps = |q: Position, bx: i32, by: i32, t: Position, tx: i32, ty: i32| {
                q.depth == t.depth
                    && delta(q.x, bx, t.x, tx).abs() < 8
                    && delta(q.y, by, t.y, ty).abs() < 6
            };
            return Ok(pages.values().any(|page| {
                page.entities.iter().any(|e| {
                    let Some(body) = e.kind.mover() else {
                        return false;
                    };
                    let q = page.position(e);
                    let (bx, by) = (i32::from(body.px), i32::from(body.py));
                    overlaps(q, bx, by, at, px, py)
                        && !overlaps(q, bx, by, old, state.player.x, state.player.y)
                })
            }));
        }
        Ok(blocked(&pages, None, at, px, py, None))
    }
}
