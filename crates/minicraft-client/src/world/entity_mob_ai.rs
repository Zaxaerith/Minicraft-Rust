//! Java MobAi / PassiveMob / EnemyMob per-tick AI, movement, and contact damage.
use super::entity::{
    DESPAWN_AGE, Entity, EntityKind, ProjectileKind, TickOutcome, can_stand, squared_distance,
};
use super::{Tile, random::JavaRandom, spawn::NaturalMob};

/// Advances one Mob entity for one game tick (Java MobAi.tick + move + contact).
#[allow(clippy::too_many_arguments)]
pub(super) fn tick_mob(
    entity: &mut Entity,
    tiles: &mut [Tile],
    data: &[u16],
    width: usize,
    height: usize,
    player: Option<(i32, i32)>,
    player_passive: bool,
    time_slowed: bool,
    random: &mut JavaRandom,
    outcome: &mut TickOutcome,
    queued_projectiles: &mut Vec<(ProjectileKind, i32, i32, i32, i32, u8)>,
    player_difficulty: u8,
) {
    let EntityKind::Mob(mob) = &mut entity.kind else {
        return;
    };
    if time_slowed && entity.age.is_multiple_of(2) {
        return;
    }
    mob.hurt_time = mob.hurt_time.saturating_sub(1);

    if entity.age > DESPAWN_AGE
        && player.is_some()
        && !matches!(
            mob.species,
            NaturalMob::AirWizard | NaturalMob::ObsidianKnight
        )
        && player.is_some_and(|(px, py)| squared_distance(entity.x, entity.y, px, py) > 160 * 160)
    {
        entity.removed = true;
        return;
    }

    let (Some((player_x, player_y)), false) = (player, player_passive) else {
        if mob.species == NaturalMob::Sheep && random.next_int(1000) == 0 {
            let tx = entity.x.div_euclid(16);
            let ty = entity.y.div_euclid(16);
            if tx >= 0 && ty >= 0 && tx < width as i32 && ty < height as i32 {
                let tile_index = tx as usize + ty as usize * width;
                if tiles[tile_index] == Tile::Grass {
                    tiles[tile_index] = Tile::Dirt;
                    mob.sheared = false;
                }
            }
        }
        return;
    };

    let distance = squared_distance(entity.x, entity.y, player_x, player_y);
    if mob.species == NaturalMob::Sheep && random.next_int(1000) == 0 {
        let tx = entity.x.div_euclid(16);
        let ty = entity.y.div_euclid(16);
        if tx >= 0 && ty >= 0 && tx < width as i32 && ty < height as i32 {
            let tile_index = tx as usize + ty as usize * width;
            if tiles[tile_index] == Tile::Grass {
                tiles[tile_index] = Tile::Dirt;
                mob.sheared = false;
            }
        }
    }

    match mob.species {
        NaturalMob::Skeleton => {
            mob.attack_delay = mob.attack_delay.saturating_sub(1);
            if distance < 100 * 100 && mob.attack_delay == 0 {
                queued_projectiles.push((
                    ProjectileKind::Arrow,
                    entity.x,
                    entity.y,
                    player_x,
                    player_y,
                    1,
                ));
                mob.attack_delay = 83;
            }
        }
        NaturalMob::Creeper => {
            // Java Creeper: fuse starts on player touch (touchedBy), 60-tick fuse,
            // explode only if a player is still within TRIGGER_RADIUS (64px).
            if distance <= 8 * 8 && mob.attack_time == 0 {
                mob.attack_time = 60;
            }
            if mob.attack_time > 0 {
                mob.attack_time -= 1;
                mob.x_move = 0;
                mob.y_move = 0;
                if mob.attack_time == 0 {
                    if distance < 64 * 64 {
                        // Java: BLAST_DAMAGE * lvl * (1/(dist+1)) + difficulty index.
                        let blast = 50_i32;
                        let dist_diag = (distance as f64).sqrt();
                        let damage = (f64::from(blast) * (1.0 / (dist_diag + 1.0))
                            + f64::from(player_difficulty))
                            as u8;
                        outcome.player_damage = outcome.player_damage.max(damage.max(1));
                        outcome.explosions.push((entity.x, entity.y, 1, false));
                        entity.removed = true;
                        return;
                    }
                    // No player in blast range: defuse (fuseTime=0, fuseLit=false).
                }
            }
        }
        NaturalMob::AirWizard | NaturalMob::ObsidianKnight => {
            mob.phase = u8::from(mob.health <= mob.max_health / 2);
            mob.attack_delay = mob.attack_delay.saturating_sub(1);
            if mob.attack_time > 0 {
                mob.attack_time -= 1;
                mob.x_move = 0;
                mob.y_move = 0;
                if mob.attack_time.is_multiple_of(5) {
                    let directions = [
                        (-2, 0),
                        (-1, -1),
                        (0, -2),
                        (1, -1),
                        (2, 0),
                        (1, 1),
                        (0, 2),
                        (-1, 1),
                    ];
                    let direction =
                        directions[(mob.attack_time as usize / 5 + mob.phase as usize) % 8];
                    queued_projectiles.push((
                        if mob.species == NaturalMob::ObsidianKnight {
                            ProjectileKind::FireSpark
                        } else {
                            ProjectileKind::Spark
                        },
                        entity.x,
                        entity.y,
                        entity.x + direction.0,
                        entity.y + direction.1,
                        1 + mob.phase,
                    ));
                }
            } else if mob.attack_delay == 0 && distance < 50 * 50 {
                mob.attack_delay = 120;
                mob.attack_time = 120;
            }
        }
        _ => {}
    }

    // Java MobAi: walkTime starts at 2 → skip real movement on even ticks.
    // Java PassiveMob.randomizeWalkDir often picks 0 via extra nextInt(2).
    if !mob.species.hostile() {
        if mob.walk_time == 0 && random.next_int(40) == 0 {
            mob.x_move = (random.next_int(3) - 1) * random.next_int(2);
            mob.y_move = (random.next_int(3) - 1) * random.next_int(2);
            mob.walk_time = 45;
        }
    } else if mob.species.hostile() && distance < 80 * 80 && mob.attack_time == 0 {
        mob.x_move = (player_x - entity.x).signum();
        mob.y_move = (player_y - entity.y).signum();
        mob.walk_time = 20;
    } else if mob.walk_time == 0 && random.next_int(40) == 0 {
        mob.x_move = random.next_int(3) - 1;
        mob.y_move = random.next_int(3) - 1;
        mob.walk_time = 45;
    }
    if mob.walk_time > 0 {
        mob.walk_time -= 1;
    } else {
        mob.x_move = 0;
        mob.y_move = 0;
    }

    let slime_jumping = mob.species != NaturalMob::Slime || entity.age % 20 < 10;
    let boss_speed = if mob.species == NaturalMob::ObsidianKnight && mob.phase == 1 {
        2
    } else {
        1
    };
    // Java Mob.move: walkTime (2) skips movement on even ticks; swimming also skips even ticks.
    let swimming = {
        let tx = entity.x.div_euclid(16);
        let ty = entity.y.div_euclid(16);
        if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
            false
        } else {
            matches!(
                tiles[tx as usize + ty as usize * width],
                Tile::Water | Tile::Lava
            )
        }
    };
    // walkTime defaults to 2 for MobAi: move only on odd tickTime.
    let can_act = !entity.age.is_multiple_of(2);
    let _ = swimming;
    if slime_jumping && can_act {
        for _ in 0..boss_speed {
            let next_x = entity.x + mob.x_move;
            if can_stand(tiles, data, width, height, next_x, entity.y) {
                entity.x = next_x;
            } else {
                mob.x_move = 0;
            }
            let next_y = entity.y + mob.y_move;
            if can_stand(tiles, data, width, height, entity.x, next_y) {
                entity.y = next_y;
            } else {
                mob.y_move = 0;
            }
        }
        if mob.x_move != 0 || mob.y_move != 0 {
            mob.walk_distance = mob.walk_distance.wrapping_add(1);
        }
    }
    if mob.species.hostile() && squared_distance(entity.x, entity.y, player_x, player_y) <= 10 * 10
    {
        outcome.player_damage =
            outcome
                .player_damage
                .max(if mob.species == NaturalMob::ObsidianKnight {
                    2
                } else {
                    1
                });
    }
}
