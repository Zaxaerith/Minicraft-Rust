//! Java MobAi.tick: walk AI, despawn, sheep eat, species attacks, contact damage.
use crate::entity::helpers::{can_stand, squared_distance};
use crate::entity::projectile::ProjectileKind;
use crate::entity::types::{DESPAWN_AGE, Entity, EntityKind, Mob, TickOutcome};
use crate::level::noise::JavaRandom;
use crate::level::tile::Tile;

use super::NaturalMob;
use super::air_wizard;
use super::creeper;
use super::obsidian_knight;
use super::skeleton;

/// Advances one Mob entity for one game tick (Java MobAi.tick + move + contact).
#[allow(clippy::too_many_arguments)]
pub fn tick_mob(
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

    // Java MobAi.handleDespawn: lifetime exceeded and player far away.
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

    // Java Sheep.eat: random 1/1000 chance to eat grass into dirt and regrow wool.
    let (Some((player_x, player_y)), false) = (player, player_passive) else {
        tick_sheep_eat(entity.x, entity.y, mob, tiles, width, height, random);
        return;
    };

    let distance = squared_distance(entity.x, entity.y, player_x, player_y);
    tick_sheep_eat(entity.x, entity.y, mob, tiles, width, height, random);

    match mob.species {
        NaturalMob::Skeleton => {
            skeleton::tick_attack(
                mob,
                entity.x,
                entity.y,
                player_x,
                player_y,
                distance,
                queued_projectiles,
            );
        }
        NaturalMob::Creeper => {
            if creeper::tick_fuse(
                mob,
                entity.x,
                entity.y,
                distance,
                player_difficulty,
                outcome,
            ) {
                entity.removed = true;
                return;
            }
        }
        NaturalMob::AirWizard | NaturalMob::ObsidianKnight => {
            air_wizard::tick_boss_attack(mob, entity.x, entity.y, distance, queued_projectiles);
            let _ = obsidian_knight::phase_speed(mob);
        }
        _ => {}
    }

    tick_walk_ai(
        mob, entity.x, entity.y, distance, player_x, player_y, random,
    );

    // Java Mob.move: walkTime (2) skips movement on even ticks; slime jump window.
    let slime_jumping = mob.species != NaturalMob::Slime || entity.age % 20 < 10;
    let boss_speed = obsidian_knight::phase_speed(mob);
    // walkTime defaults to 2 for MobAi: move only on odd tickTime.
    let can_act = !entity.age.is_multiple_of(2);
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
    // Java EnemyMob contact damage via touchedBy.
    if mob.species.hostile() && squared_distance(entity.x, entity.y, player_x, player_y) <= 10 * 10
    {
        outcome.player_damage =
            outcome
                .player_damage
                .max(crate::entity::stats::mob_contact_damage(
                    mob.species,
                    mob.level,
                    player_difficulty,
                ));
    }
}

fn tick_sheep_eat(
    entity_x: i32,
    entity_y: i32,
    mob: &mut Mob,
    tiles: &mut [Tile],
    width: usize,
    height: usize,
    random: &mut JavaRandom,
) {
    if mob.species != NaturalMob::Sheep || random.next_int(1000) != 0 {
        return;
    }
    let tx = entity_x.div_euclid(16);
    let ty = entity_y.div_euclid(16);
    if tx >= 0 && ty >= 0 && tx < width as i32 && ty < height as i32 {
        let tile_index = tx as usize + ty as usize * width;
        if tiles[tile_index] == Tile::Grass {
            tiles[tile_index] = Tile::Dirt;
            mob.sheared = false;
        }
    }
}

/// Java PassiveMob.randomizeWalkDir / EnemyMob chase-or-wander.
fn tick_walk_ai(
    mob: &mut Mob,
    entity_x: i32,
    entity_y: i32,
    distance: i32,
    player_x: i32,
    player_y: i32,
    random: &mut JavaRandom,
) {
    // Java MobAi: walkTime starts at 2 → skip real movement on even ticks.
    // Java PassiveMob.randomizeWalkDir often picks 0 via extra nextInt(2).
    if !mob.species.hostile() {
        if mob.walk_time == 0 && random.next_int(40) == 0 {
            mob.x_move = (random.next_int(3) - 1) * random.next_int(2);
            mob.y_move = (random.next_int(3) - 1) * random.next_int(2);
            mob.walk_time = 45;
        }
    } else if mob.species.hostile()
        && distance
            < if matches!(mob.species, NaturalMob::Slime | NaturalMob::Creeper) {
                50 * 50
            } else {
                100 * 100
            }
        && mob.attack_time == 0
    {
        mob.x_move = (player_x - entity_x).signum();
        mob.y_move = (player_y - entity_y).signum();
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
}
