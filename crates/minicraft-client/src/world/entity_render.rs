//! Per-kind entity drawing (Java Entity.render / MobAi / Furniture / particles).
use super::{Assets, EntityKind, FurnitureKind, Screen, TILE_SIZE, Tile, entity, spawn};

// Keep the existing coordinate/render API without adding wrapper state.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_entity(
    screen: &mut Screen,
    assets: &Assets,
    entity: &entity::Entity,
    camera_x: i32,
    camera_y: i32,
    tiles: &[Tile],
    width: usize,
    height: usize,
    entity_tick: u64,
) {
    let tile_under = |px: i32, py: i32| -> Tile {
        let tx = px.div_euclid(TILE_SIZE);
        let ty = py.div_euclid(TILE_SIZE);
        if tx < 0 || ty < 0 || tx >= width as i32 || ty >= height as i32 {
            return Tile::Grass;
        }
        tiles[tx as usize + ty as usize * width]
    };
    match &entity.kind {
        EntityKind::Item(stack) => {
            // Java ItemEntity: LinkedSprite at (x-4, y-4); shadow is color override.
            let linked = assets.item_stack_linked(*stack);
            let Some(motion) = &entity.item_motion else {
                linked.render(
                    assets.sheets(),
                    screen,
                    entity.x - 4 - camera_x,
                    entity.y - 4 - camera_y,
                    0,
                    false,
                    0,
                );
                return;
            };
            if entity.age >= motion.lifetime.saturating_sub(120)
                && (entity.age / 6).is_multiple_of(2)
            {
                return;
            }
            linked.render(
                assets.sheets(),
                screen,
                entity.x - 4 - camera_x,
                entity.y - 4 - camera_y,
                0,
                false,
                0x1F_1F_1F,
            );
            linked.render(
                assets.sheets(),
                screen,
                entity.x - 4 - camera_x,
                entity.y - 4 - camera_y - motion.height as i32,
                0,
                false,
                0,
            );
        }
        EntityKind::Tool(tool) => {
            let image = assets.tool(*tool);
            let height = entity
                .item_motion
                .as_ref()
                .map_or(0, |motion| motion.height as i32);
            screen.blit(
                image,
                entity.x - 4 - camera_x,
                entity.y - 4 - camera_y - height,
            );
        }
        EntityKind::Mob(mob) => {
            let under = tile_under(entity.x, entity.y);
            let swimming = matches!(under, Tile::Water | Tile::Lava);
            let on_water = under == Tile::Water;
            // Java MobAi.render base position, plus Player-style liquid overlay when swimming.
            let mut yo = entity.y - 11;
            if swimming {
                yo += 4;
                let frame = ((entity_tick / 8) % 2) as usize;
                let (sheet_x, left_flip, right_flip) = if on_water {
                    (5usize, false, true)
                } else {
                    (6usize, true, false)
                };
                let source_x = sheet_x * 8;
                let source_y = frame * 8;
                screen.blit_region(
                    assets.hud(),
                    entity.x - 8 - camera_x,
                    yo + 3 - camera_y,
                    source_x,
                    source_y,
                    8,
                    8,
                    left_flip,
                );
                screen.blit_region(
                    assets.hud(),
                    entity.x - camera_x,
                    yo + 3 - camera_y,
                    source_x,
                    source_y,
                    8,
                    8,
                    right_flip,
                );
            }
            let image = assets.mob(mob.species);
            let frame = mob.walk_distance as usize / 8 % 2;
            let jump = if mob.species == spawn::NaturalMob::Slime && entity.age % 20 < 10 {
                -4
            } else {
                0
            };
            if image.width < 64 {
                crate::gfx::sprite_linker::LinkedSprite::new(
                    crate::gfx::sprite_linker::SpriteType::Entity,
                    mob.species.asset_name(),
                )
                .set_sprite_dim(frame * 2, 0, 2, 2)
                .render(
                    assets.sheets(),
                    screen,
                    entity.x - 8 - camera_x,
                    yo - camera_y + jump,
                    0,
                    mob.hurt_time > 0,
                    0,
                );
            } else {
                let direction = if mob.y_move < 0 {
                    1
                } else if mob.x_move < 0 {
                    2
                } else if mob.x_move > 0 {
                    3
                } else {
                    0
                };
                let linked = assets.mob_linked_level(
                    mob.species,
                    mob.level,
                    direction,
                    frame,
                    mob.species == spawn::NaturalMob::Sheep && mob.sheared,
                );
                if swimming {
                    // Java Player swimming: only the top 16×8 half.
                    linked.render_half_top(
                        assets.sheets(),
                        screen,
                        entity.x - 8 - camera_x,
                        yo - camera_y + jump,
                        mob.hurt_time > 0,
                    );
                } else {
                    linked.render(
                        assets.sheets(),
                        screen,
                        entity.x - 8 - camera_x,
                        yo - camera_y + jump,
                        0,
                        mob.hurt_time > 0,
                        0,
                    );
                }
            }
            if mob.health < mob.max_health {
                screen.rect(
                    entity.x - 7 - camera_x,
                    entity.y - 14 - camera_y,
                    14,
                    2,
                    0x3A1010,
                );
                screen.rect(
                    entity.x - 7 - camera_x,
                    entity.y - 14 - camera_y,
                    14 * i32::from(mob.health) / i32::from(mob.max_health),
                    2,
                    0xD84848,
                );
            }
            if matches!(
                mob.species,
                spawn::NaturalMob::AirWizard | spawn::NaturalMob::ObsidianKnight
            ) {
                let percent = (u32::from(mob.health) * 100 / u32::from(mob.max_health)).max(1);
                screen.text(
                    assets.font(),
                    &format!("{percent}%"),
                    entity.x - 12 - camera_x,
                    entity.y - 23 - camera_y,
                );
            }
        }
        EntityKind::Furniture(kind) => {
            // Java Furniture.render: LinkedSprite at (x-8, y-8).
            let sprite = if *kind == FurnitureKind::Bed && entity.state != 0 {
                assets.colored_bed_linked(crate::entity::furniture::bed::color_from_state(
                    entity.state,
                ))
            } else {
                assets.furniture_linked(*kind)
            };
            sprite.render(
                assets.sheets(),
                screen,
                entity.x - 8 - camera_x,
                entity.y - 8 - camera_y,
                0,
                false,
                0,
            );
        }
        EntityKind::Projectile(projectile) => {
            let color = match projectile.kind {
                entity::ProjectileKind::Arrow => 0xD8C48A,
                entity::ProjectileKind::Spark => 0xC8E8FF,
                entity::ProjectileKind::FireSpark => 0xFF6A24,
            };
            screen.rect(
                entity.x - 2 - camera_x,
                entity.y - 2 - camera_y,
                4,
                4,
                color,
            );
        }
        EntityKind::Particle(kind) => match kind {
            entity::ParticleKind::Smash => {
                screen.blit(assets.smash(), entity.x - camera_x, entity.y - camera_y);
            }
            entity::ParticleKind::Fire => {
                screen.rect(entity.x - camera_x, entity.y - camera_y, 2, 2, 0xFF8A30);
            }
        },
        EntityKind::TextParticle(text) => {
            // Java TextParticle: FontStyle(color).setShadowType(BLACK,false).draw.
            let (offset_x, offset_y, z) = entity::text_particle_motion(entity.id, entity.age);
            let x = entity.x - camera_x + offset_x;
            let y = entity.y - camera_y + offset_y - z;
            // Color.RED = Color.get(1, 198, 44, 44) → 0xC62C2C after upgrade.
            crate::gfx::font_style::FontStyle::new(0xC6_2C_2C)
                .set_shadow_type(0x00_00_00, false)
                .set_x_pos(x - text.encode_utf16().count() as i32 * 4)
                .set_y_pos(y)
                .draw(screen, assets.font(), text);
        }
        EntityKind::ExplosionTicker => {
            // Java ExplosionTileTicker.render is empty.
        }
    }
}
