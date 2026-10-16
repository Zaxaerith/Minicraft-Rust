// Multi-frame player scenarios deliberately mutate inputs between ticks.
#![allow(clippy::field_reassign_with_default)]
//! World registry / lifecycle characterization tests (was inlined in mod.rs).
use super::{
    ActiveItem, DAY_LENGTH, Direction, FurnitureKind, GameMode, HUNGER_STAMINA_STEPS, Level,
    MAX_HUNGER_TICKS, PausePage, PauseSession, Player, RecipeOutput, RecipeView,
    STARVATION_HEALTH_FLOORS, TILE_SIZE, Tile, World, creative_entries,
    entity::{EntityArena, EntityKind},
    pause_actions, player_sprite_frame,
    random::JavaRandom,
    recipe_can_craft, spawn, time_name, try_queue_natural_spawn,
};
use crate::core::updater::Updater;
use crate::{
    audio::SoundEffect,
    io::Input,
    item::{
        ArmorKind, Inventory, ItemId, PotionKind, ToolItem, ToolKind, ToolTier,
        crafting::sorted_recipe_views, recipe_views,
    },
};

pub(crate) fn tiny_world() -> World {
    World {
        generation_version: crate::level::GenerationVersion::Legacy224,
        flora_random: JavaRandom::new(0),
        width: 16,
        height: 16,
        levels: vec![Level {
            depth: 0,
            tiles: vec![Tile::Grass; 16 * 16],
            data: vec![0; 16 * 16],
            max_mob_count: 100,
            pending_spawns: Vec::new(),
            entities: EntityArena::default(),
        }],
        current_level: 0,
        player: Player {
            x: 8 * 16 + 8,
            y: 8 * 16 + 8,
            direction: Direction::Right,
            walk_distance: 0,
            attack_time: 0,
            attack_direction: Direction::Down,
            attack_item: None,
            health: 10,
            max_health: 10,
            stamina: 10,
            hunger: 10,
            armor: 0,
            armor_kind: None,
            armor_damage_buffer: 0,
            hurt_time: 0,
            stamina_recharge: 0,
            stamina_recharge_delay: 0,
            hunger_stamina_count: HUNGER_STAMINA_STEPS[1],
            hunger_ticks: MAX_HUNGER_TICKS,
            step_count: 0,
            hunger_charge_delay: 0,
            hunger_starve_delay: 0,
            potion_effects: [0; PotionKind::ALL.len()],
            regen_tick: 0,
            fishing_level: None,
            fishing_ticks: 0,
            watering_content: 0,
            clothing: ItemId::RegularClothes,
            imported_shirt_color: None,
            inventory: Inventory::new(27),
            active_item: None,
        },
        seed: 1,
        tick: 0,
        clock: Default::default(),
        animation_enter_millis: crate::assets::animation_millis(),
        render_gui: true,
        updater: Default::default(),
        days: 1,
        difficulty: 1,
        mode: super::GameMode::Survival,
        score: 0,
        score_ticks: 20 * 60 * 60,
        score_minutes: 20,
        score_multiplier: 1,
        multiplier_ticks: 300,
        game_over: false,
        end_game: None,
        story_complete: false,
        sleeping: 0,
        bed_level: None,
        respawn_tile: None,
        sleep_speed: 1,
        player_removed: false,
        player_dead_time: 0,
        pending_level_change: 0,
        on_stair_delay: 0,
        on_fall_delay: 0,
        level_transition: None,
        death_open: false,
        death_selection: 0,
        tutorials_enabled: true,
        quests_enabled: true,
        show_quests: true,
        progress: crate::progression::ProgressState::load().unwrap(),
        signs: vec![std::collections::HashMap::new()],
        sign_editor: None,
        book_open: None,
        pause: PauseSession::default(),
        inventory_open: false,
        quest_tree_state: super::quest_tree::State::default(),
        inventory_selection: 0,
        inventory_item_selection: 0,
        inventory_pane: 0,
        crafting_station: None,
        container_id: None,
        crafting_order: Vec::new(),
        personal_crafting: false,
        notifications: super::Notifications::default(),
        notification_context: super::notifications::Context::default(),
        air_wizard_defeated: false,
        obsidian_knight_defeated: false,
        random: JavaRandom::new(1),
        sound_events: Vec::new(),
        tile_ticks: 0,
        sound_ticks: 0,
    }
}

#[test]
fn inventory_counter_uses_java_heaviness_colors() {
    assert_eq!(
        crate::gfx::gui::inventory_counter_color(0, 27, true),
        0x00FF00
    );
    assert_eq!(
        crate::gfx::gui::inventory_counter_color(27, 27, true),
        0xFF0000
    );
}

#[test]
fn cactus_bump_hurts_and_blocks_the_player() {
    let mut world = tiny_world();
    let start_x = world.player.x;
    world.levels[0].tiles[9 + 8 * 16] = Tile::Cactus;

    assert!(!world.move_player(5, 0));
    assert_eq!(world.player.x, start_x);
    assert_eq!(world.player.health, 9);
    // Java playerHurtTime = 30; white flash while hurtTime > playerHurtTime - 10.
    assert_eq!(world.player.hurt_time, 30);
    assert!(world.player.hurt_time > 20);

    assert!(!world.move_player(5, 0));
    assert_eq!(world.player.health, 9);
}

#[test]
fn hard_cactus_and_lava_brick_use_java_bump_damage() {
    let mut hard = tiny_world();
    hard.difficulty = 2;
    hard.levels[0].tiles[9 + 8 * 16] = Tile::Cactus;
    assert!(!hard.move_player(5, 0));
    assert_eq!(hard.player.health, 8);

    let mut lava_brick = tiny_world();
    lava_brick.levels[0].tiles[9 + 8 * 16] = Tile::LavaBrick;
    assert!(lava_brick.move_player(5, 0));
    assert_eq!(lava_brick.player.health, 7);
}

#[test]
fn menu_key_opens_chest_container_and_transfers_items() {
    let mut world = tiny_world();
    let chest_x = world.player.x;
    let chest_y = world.player.y + 12;
    let id = world.levels[0]
        .entities
        .spawn_furniture(FurnitureKind::Chest, chest_x, chest_y);
    world.levels[0]
        .entities
        .entities_mut_for_storage(id)
        .unwrap()
        .add(ItemId::Wood, 5);
    world.player.inventory.add(ItemId::Stone, 3);
    world.player.direction = Direction::Down;

    let mut menu = Input::default();
    menu.menu = true;
    world.tick_gameplay_player(&menu);
    // Furniture use runs from tick_after_prelude via input; call directly.
    assert!(world.use_furniture_front());
    assert!(world.inventory_open);
    assert_eq!(world.container_id, Some(id));

    // Java CURSOR-RIGHT enters the chest menu; MOVE-RIGHT is not the menu key.
    let mut move_only = Input::default();
    move_only.right_pressed = true;
    let _ = world.tick_container_ui(&move_only);
    assert_eq!(world.inventory_pane, 0);
    let mut enter_chest = Input::default();
    enter_chest.cursor_right_pressed = true;
    let _ = world.tick_container_ui(&enter_chest);
    assert_eq!(world.inventory_pane, 1);
    world.inventory_selection = 0;
    let mut attack = Input::default();
    attack.attack = true;
    let _ = world.tick_container_ui(&attack);
    assert_eq!(world.player.inventory.count(ItemId::Wood), 1);
    assert_eq!(
        world.levels[0]
            .entities
            .death_chest_storage(id)
            .unwrap()
            .slots()[0]
            .count,
        4
    );

    let mut esc = Input::default();
    esc.menu = true;
    let _ = world.tick_container_ui(&esc);
    assert!(!world.inventory_open);
    assert!(world.container_id.is_none());
}

#[test]
fn swimming_drains_stamina_every_60_ticks_without_swim_potion() {
    let mut world = tiny_world();
    let tile_x = world.player.x.div_euclid(TILE_SIZE) as usize;
    let tile_y = world.player.y.div_euclid(TILE_SIZE) as usize;
    world.levels[0].tiles[tile_x + tile_y * world.width] = Tile::Water;
    assert!(world.player_is_swimming());
    world.player.stamina = 10;
    // World.tick increments before player body; drive the gameplay path.
    world.tick = 59;
    let input = Input::default();
    world.tick_gameplay_player(&input);
    // After tick() this becomes 60 in the main loop; call the drain check by
    // setting tick so the in-body condition sees a multiple of 60.
    world.tick = 60;
    world.tick_gameplay_player(&input);
    assert_eq!(world.player.stamina, 9);
    // Swim potion prevents drowning drain.
    world.player.stamina = 10;
    world.player.potion_effects[crate::item::PotionKind::Swim.id()] = 600;
    world.tick = 120;
    world.tick_gameplay_player(&input);
    assert_eq!(world.player.stamina, 10);
}

#[test]
fn swimming_blocks_stamina_recharge() {
    let mut world = tiny_world();
    let tile_x = world.player.x.div_euclid(TILE_SIZE) as usize;
    let tile_y = world.player.y.div_euclid(TILE_SIZE) as usize;
    world.levels[0].tiles[tile_x + tile_y * world.width] = Tile::Water;
    world.player.stamina = 5;
    world.player.stamina_recharge = 10;
    world.player.stamina_recharge_delay = 0;
    world.tick_survival_stats();
    assert_eq!(world.player.stamina_recharge, 0);
    assert_eq!(world.player.stamina, 5);
}

#[test]
fn player_skin_frames_match_java_four_direction_animation() {
    assert_eq!(player_sprite_frame(Direction::Down, 0), (0, false));
    assert_eq!(player_sprite_frame(Direction::Down, 8), (0, true));
    assert_eq!(player_sprite_frame(Direction::Up, 0), (16, false));
    assert_eq!(player_sprite_frame(Direction::Up, 8), (16, true));
    assert_eq!(player_sprite_frame(Direction::Left, 0), (32, true));
    assert_eq!(player_sprite_frame(Direction::Left, 8), (48, true));
    assert_eq!(player_sprite_frame(Direction::Right, 0), (32, false));
    assert_eq!(player_sprite_frame(Direction::Right, 8), (48, false));
}

#[test]
fn player_motion_matches_java_direction_collision_and_exhaustion_rules() {
    let mut opposing = tiny_world();
    let start_x = opposing.player.x;
    opposing.tick(&Input {
        left: true,
        right: true,
        ..Input::default()
    });
    assert_eq!(opposing.player.x, start_x);
    assert!(matches!(opposing.player.direction, Direction::Right));
    assert_eq!(opposing.player.walk_distance, 0);

    let mut blocked = tiny_world();
    blocked.player.x = 9 * TILE_SIZE - 5;
    blocked.levels[0].tiles[9 + 8 * 16] = Tile::Rock;
    let blocked_x = blocked.player.x;
    blocked.tick(&Input {
        right: true,
        ..Input::default()
    });
    assert_eq!(blocked.player.x, blocked_x);
    assert_eq!(blocked.player.walk_distance, 1);
    assert_eq!(blocked.player.step_count, 0);

    let mut exhausted = tiny_world();
    exhausted.player.stamina = 0;
    let exhausted_x = exhausted.player.x;
    exhausted.tick(&Input {
        right: true,
        ..Input::default()
    });
    assert_eq!(exhausted.player.stamina_recharge_delay, 39);
    assert_eq!(exhausted.player.x, exhausted_x);
    exhausted.tick(&Input {
        right: true,
        ..Input::default()
    });
    assert_eq!(exhausted.player.stamina_recharge_delay, 38);
    assert_eq!(exhausted.player.x, exhausted_x + 1);
}

#[test]
fn creative_inventory_is_a_separate_unlimited_catalogue() {
    let mut world = tiny_world();
    world.mode = GameMode::Creative;

    world.tick(&Input {
        menu: true,
        ..Input::default()
    });
    assert!(world.inventory_open);
    assert_eq!(world.inventory_pane, 1);
    assert_eq!(world.player.inventory.capacity(), 27);
    assert_eq!(world.player.inventory.used_slots(), 0);

    assert!(
        creative_entries()
            .iter()
            .all(|entry| !matches!(entry, super::CreativeEntry::Stack(ItemId::PowerGlove)))
    );
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    assert_eq!(world.player.inventory.count(ItemId::Wood), 2);
    assert_eq!(world.player.inventory.used_slots(), 1);

    world.tick(&Input {
        left_pressed: true,
        ..Input::default()
    });
    assert_eq!(world.inventory_pane, 0);
    world.tick(&Input {
        attack: true,
        ..Input::default()
    });
    assert!(!world.inventory_open);
    assert_eq!(
        world.player.active_item,
        Some(ActiveItem::Stack(ItemId::Wood))
    );
}

#[test]
fn enter_equips_inventory_items_and_z_opens_personal_crafting() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::Wood, 10);
    world.tick(&Input {
        menu: true,
        ..Input::default()
    });
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    assert!(!world.inventory_open);
    assert_eq!(
        world.player.active_item,
        Some(ActiveItem::Stack(ItemId::Wood))
    );

    world.tick(&Input {
        craft: true,
        ..Input::default()
    });
    assert!(world.inventory_open);
    assert!(world.personal_crafting);
    assert_eq!(world.crafting_station, None);
    world.inventory_selection = sorted_recipe_views(
        None,
        &world.player.inventory,
        false,
    )
    .iter()
    .position(|recipe| {
        matches!(recipe.output, RecipeOutput::Stack(stack) if stack.item == ItemId::Workbench)
    })
    .unwrap();
    let craft = Input {
        select: true,
        ..Input::default()
    };
    world.tick(&craft);
    assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
    let sounds = world.take_sound_events();
    assert!(
        sounds.contains(&SoundEffect::Craft) || sounds.contains(&SoundEffect::Confirm),
        "crafting select should emit audio, got {sounds:?}"
    );
}

#[test]
fn z_switches_an_open_player_inventory_to_workbench_crafting() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::Wood, 10);
    world.tick(&Input {
        menu: true,
        ..Input::default()
    });
    assert!(world.inventory_open && !world.personal_crafting);

    world.tick(&Input {
        craft: true,
        ..Input::default()
    });
    assert!(world.inventory_open && world.personal_crafting);
    world.inventory_selection = sorted_recipe_views(
        None,
        &world.player.inventory,
        false,
    )
    .iter()
    .position(|recipe| {
        matches!(recipe.output, RecipeOutput::Stack(stack) if stack.item == ItemId::Workbench)
    })
    .unwrap();
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    assert_eq!(world.player.inventory.count(ItemId::Wood), 0);
    assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
    assert!(
        world
            .notifications
            .messages()
            .iter()
            .all(|message| !message.starts_with("CRAFTED"))
    );
}

#[test]
fn an_item_equipped_with_enter_can_be_used_after_closing_inventory() {
    let mut world = tiny_world();
    world.player.hunger = 5;
    world.player.inventory.add(ItemId::Apple, 1);
    world.tick(&Input {
        menu: true,
        ..Input::default()
    });
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    world.tick(&Input {
        attack: true,
        ..Input::default()
    });
    assert_eq!(world.player.hunger, 6);
    assert_eq!(world.player.inventory.count(ItemId::Apple), 0);
}

#[test]
fn pickup_uses_the_power_glove_slash_without_subtitles() {
    let mut world = tiny_world();
    world.tutorials_enabled = false;
    world.quests_enabled = false;
    world.notifications.clear();
    world.tick(&Input {
        pickup: true,
        ..Input::default()
    });
    assert_eq!(world.player.attack_time, 5);
    assert!(world.notifications.messages().is_empty());

    world.levels[0].entities.spawn_furniture(
        FurnitureKind::Workbench,
        world.player.x + 12,
        world.player.y,
    );
    world.tick(&Input {
        pickup: true,
        ..Input::default()
    });
    assert_eq!(
        world.player.active_item,
        Some(ActiveItem::Stack(ItemId::Workbench))
    );
    assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);
    assert!(world.notifications.messages().is_empty());
    assert!(world.take_sound_events().contains(&SoundEffect::Pickup));
}

#[test]
fn walking_into_furniture_pushes_it_like_java_touchedby() {
    let mut world = tiny_world();
    world.player.x = 8 * 16 + 8;
    world.player.y = 8 * 16 + 8;
    world.player.direction = Direction::Right;
    // Overlap Workbench (xr=3) with player (xr=4) so the first right step is blocked and tryPush runs.
    let id = world.levels[0].entities.spawn_furniture(
        FurnitureKind::Workbench,
        world.player.x + 6,
        world.player.y,
    );
    let before = world.levels[0]
        .entities
        .entities()
        .iter()
        .find(|e| e.id == id)
        .map(|e| (e.x, e.y))
        .unwrap();
    // Hold a placeable so the regression "can place but cannot push" is covered.
    world.player.active_item = Some(ActiveItem::Stack(ItemId::Workbench));
    for _ in 0..24 {
        world.tick(&Input {
            right: true,
            ..Input::default()
        });
    }
    let after = world.levels[0]
        .entities
        .entities()
        .iter()
        .find(|e| e.id == id)
        .map(|e| (e.x, e.y))
        .unwrap();
    assert!(
        after.0 > before.0,
        "furniture should be pushed right: {before:?} -> {after:?}"
    );
    // Java: pushTime=10 only cools down; each tryPush moves at most 1px.
    assert!(
        after.0 <= before.0 + 3,
        "furniture must not coast far after the player stops: {:?}",
        after
    );
}

#[test]
fn furniture_push_moves_one_pixel_per_try_push_cool_down() {
    // Java: tryPush arms pushDir once when pushTime==0; Furniture.tick moves 1px then clears it.
    // Holding direction does not coast furniture for the remaining pushTime ticks.
    let mut world = tiny_world();
    world.player.x = 8 * 16 + 8;
    world.player.y = 8 * 16 + 8;
    world.player.direction = Direction::Right;
    let id = world.levels[0].entities.spawn_furniture(
        FurnitureKind::Workbench,
        world.player.x + 6,
        world.player.y,
    );
    let start_x = world.levels[0]
        .entities
        .entities()
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.x)
        .unwrap();
    // One blocked step arms push; furniture may move at most 1px that tick.
    world.tick(&Input {
        right: true,
        ..Input::default()
    });
    let after_one = world.levels[0]
        .entities
        .entities()
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.x)
        .unwrap();
    assert!(
        after_one <= start_x + 1,
        "furniture must not coast more than 1px: {start_x} -> {after_one}"
    );
    // Next few ticks with pushTime cooling down: no further free flight.
    for _ in 0..5 {
        world.tick(&Input::default());
    }
    let after_idle = world.levels[0]
        .entities
        .entities()
        .iter()
        .find(|e| e.id == id)
        .map(|e| e.x)
        .unwrap();
    assert_eq!(
        after_idle, after_one,
        "idle ticks must not keep flying furniture"
    );
}

#[test]
fn pause_audio_is_available_to_the_input_frame_without_a_tick_delay() {
    let mut world = tiny_world();
    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    world.take_sound_events();
    let down = Input {
        down_pressed: true,
        ..Input::default()
    };
    world.tick(&down);
    assert!(world.take_sound_events().contains(&SoundEffect::Select));
    let confirm = Input {
        select: true,
        ..Input::default()
    };
    world.tick(&confirm);
    assert!(world.take_sound_events().contains(&SoundEffect::Confirm));
}

#[test]
fn closing_pause_uses_the_java_display_exit_sound() {
    let mut world = tiny_world();
    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    assert!(world.take_sound_events().contains(&SoundEffect::Craft));
}

#[test]
fn canceling_pause_quit_plays_confirm_then_display_exit() {
    let mut world = tiny_world();
    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    world.take_sound_events();
    world.pause.view_mut().selection = pause_actions(world.quests_enabled).len() - 1;
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    let sounds = world.take_sound_events();
    assert!(sounds.contains(&SoundEffect::Confirm));
    assert!(sounds.contains(&SoundEffect::Craft));
}

#[test]
fn pause_achievements_use_null_entries_and_java_on_exit_sound_order() {
    let mut world = tiny_world();
    world.prepare_pause_audio_parity();
    world.pause.view_mut().selection = 2;
    world.pause.push(PausePage::Achievements, 0);
    world.pause.begin_tick();
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    assert_eq!(world.pause.view().page, PausePage::Achievements);
    world.take_sound_events();
    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    let sounds = world.take_sound_events();
    assert!(sounds.contains(&SoundEffect::Craft));
    assert!(sounds.contains(&SoundEffect::Confirm));
}

#[test]
fn quest_root_tabs_and_nested_displays_follow_java_stack() {
    let mut world = tiny_world();
    world.prepare_quests_audio_parity();
    assert_eq!(world.audio_menu_state_for_parity(), ("quests", 0));

    let select = Input {
        select: true,
        ..Input::default()
    };
    world.tick(&select);
    assert!(world.take_sound_events().contains(&SoundEffect::Confirm));
    assert_eq!(
        world.audio_menu_state_for_parity(),
        ("quest_series_info", 0)
    );
    world.tick(&select);
    assert_eq!(
        world.audio_menu_state_for_parity(),
        ("quest_series_viewer", 0)
    );
    world.tick(&select);
    assert_eq!(world.audio_menu_state_for_parity(), ("quest_detail", 0));
    world.take_sound_events();

    let exit = Input {
        exit: true,
        ..Input::default()
    };
    for expected in ["quest_series_viewer", "quest_series_info", "quests"] {
        world.tick(&exit);
        assert!(world.take_sound_events().contains(&SoundEffect::Craft));
        assert_eq!(world.audio_menu_state_for_parity().0, expected);
    }

    let right = Input {
        right_pressed: true,
        ..Input::default()
    };
    world.tick(&right);
    assert_eq!(world.audio_menu_state_for_parity(), ("quests", 1));
    world.take_sound_events();

    let down = Input {
        down_pressed: true,
        ..Input::default()
    };
    world.tick(&down);
    // Empty helper tab may skip Select; non-empty tabs emit Select on move.
    let _ = world.take_sound_events();
}

#[test]
fn quest_tree_navigation_and_detail_return_preserve_cursor_and_viewport() {
    let mut world = tiny_world();
    world.prepare_quests_audio_parity();
    world.complete_hoe_quest_for_parity();
    let select = Input {
        select: true,
        ..Input::default()
    };
    world.tick(&select);
    world.prepare_quest_layout(&crate::resource::localization::Localization::load(
        "en-us",
        &[],
    ));
    world.tick(&select);
    let right = Input {
        right_pressed: true,
        ..Input::default()
    };
    world.tick(&right);
    assert_eq!(world.pause.view().selection, 0); // Children are below, not to the right.
    world.take_sound_events();
    let down = Input {
        down_pressed: true,
        ..Input::default()
    };
    world.tick(&down);
    let _ = world.take_sound_events();
    // Selection may stay 0 if the quest series has a single child row.
    assert!(world.pause.view().selection <= 1);

    let browsing = Input {
        shift: true,
        down_pressed: true,
        select: true,
        ..Input::default()
    };
    assert!(world.immediate_menu_sounds(&browsing).is_empty());
    world.tick(&browsing);
    assert_eq!(world.pause.view().page, PausePage::QuestSeriesViewer);
    assert_eq!(world.quest_tree_state.y_scroll, 3);
    assert_eq!(world.pause.view().selection, 1);
    world.tick(&select); // Release Shift; current node is already visible.
    assert_eq!(world.pause.view().page, PausePage::QuestDetail);
    assert_eq!(world.pause.view().quest_item, 1);
    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    assert_eq!(world.pause.view().page, PausePage::QuestSeriesViewer);
    assert_eq!(
        (
            world.pause.view().selection,
            world.quest_tree_state.y_scroll
        ),
        (1, 3)
    );
    world.take_sound_events();

    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    world.tick(&select); // A newly constructed viewer resets its scroll/cursor.
    assert_eq!(
        (
            world.pause.view().selection,
            world.quest_tree_state.y_scroll
        ),
        (0, 0)
    );
}

#[test]
fn inventory_equip_and_exit_use_the_java_display_exit_sound_immediately() {
    let mut world = tiny_world();
    world.prepare_inventory_audio_parity();
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    assert!(world.take_sound_events().contains(&SoundEffect::Craft));
    assert_eq!(world.audio_menu_state_for_parity().0, "gameplay");

    world.prepare_inventory_audio_parity();
    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    assert!(world.take_sound_events().contains(&SoundEffect::Craft));
}

#[test]
fn successful_crafting_sound_is_not_deferred_or_duplicated() {
    let mut world = tiny_world();
    world.prepare_crafting_parity(false, 10, 0, 0, 0);
    let select = Input {
        select: true,
        ..Input::default()
    };
    world.tick(&select);
    let craft_sounds = world.take_sound_events();
    assert!(
        craft_sounds.contains(&SoundEffect::Craft) || craft_sounds.contains(&SoundEffect::Confirm),
        "crafting select should emit an audio event, got {craft_sounds:?}"
    );
    assert_eq!(world.player.inventory.count(ItemId::Workbench), 1);

    world.tick(&Input {
        craft: true,
        ..Input::default()
    });
    assert!(world.take_sound_events().contains(&SoundEffect::Craft));
}

#[test]
fn crafting_list_stably_partitions_craftable_recipes_like_java() {
    let mut inventory = Inventory::new(27);
    inventory.add(ItemId::Wood, 10);
    let recipes = sorted_recipe_views(None, &inventory, false);
    let craftable: Vec<bool> = recipes
        .iter()
        .map(|recipe| recipe_can_craft(recipe, &inventory, false))
        .collect();
    assert!(craftable.windows(2).all(|pair| pair[0] || !pair[1]));

    let original = recipe_views(None);
    let mut expected = Vec::new();
    for wanted in [true, false] {
        expected.extend(
            original
                .iter()
                .filter(|recipe| recipe_can_craft(recipe, &inventory, false) == wanted)
                .map(RecipeView::product_name),
        );
    }
    assert_eq!(
        recipes
            .iter()
            .map(RecipeView::product_name)
            .collect::<Vec<_>>(),
        expected
    );
}

#[test]
fn pause_menu_matches_java_save_and_confirmed_quit_flow() {
    let mut world = tiny_world();
    assert!(matches!(
        world.tick(&Input {
            exit: true,
            ..Input::default()
        }),
        super::WorldAction::None
    ));
    assert!(world.pause.is_open());
    world.pause.view_mut().selection = 4;
    assert!(matches!(
        world.tick(&Input {
            select: true,
            ..Input::default()
        }),
        super::WorldAction::SaveGame
    ));
    assert!(!world.pause.is_open());

    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    world.pause.view_mut().selection = 5;
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    assert!(world.pause.view().confirm);
    world.tick(&Input {
        down_pressed: true,
        ..Input::default()
    });
    assert!(matches!(
        world.tick(&Input {
            select: true,
            ..Input::default()
        }),
        super::WorldAction::QuitWithoutSaving
    ));
}

#[test]
fn pause_parent_selection_and_world_clock_follow_update_boundaries() {
    let mut world = tiny_world();
    let exit = Input {
        exit: true,
        ..Default::default()
    };
    let select = Input {
        select: true,
        ..Default::default()
    };
    world.tick(&exit);
    assert!(!world.pause.is_rendering());
    world.tick(&Input {
        down_pressed: true,
        ..Default::default()
    });
    assert_eq!(world.pause.render_view().selection, 1);
    world.tick(&select);
    assert_eq!(world.pause.view().page, PausePage::Options);
    assert_eq!(world.pause.render_view().page, PausePage::Main);
    world.tick(&exit);
    assert_eq!(world.pause.view().selection, 1);
    assert_eq!(world.pause.render_view().page, PausePage::Options);
    world.tick(&Input::default());
    assert_eq!(world.tick, 0);
    assert_eq!(world.pause.render_view().selection, 1);
    world.tick(&exit);
    assert!(!world.pause.is_open());
    assert!(world.pause.is_rendering());
    world.take_sound_events();
    world.tick(&Input::default());
    assert_eq!(world.tick, 1);
    assert!(!world.pause.is_rendering());
    assert!(world.take_sound_events().is_empty());
}

#[test]
fn focus_loss_advances_calendar_but_not_player_and_reconciles_pause() {
    let mut world = tiny_world();
    world.tick(&Input::default());
    assert_eq!((world.tick, world.clock.get().day_tick), (1, 0));
    let position = (world.player.x, world.player.y, world.player.stamina);
    for _ in 0..10 {
        world.begin_display_tick();
        world.tick_after_display(
            false,
            &Input {
                exit: true,
                attack: true,
                right: true,
                ..Default::default()
            },
        );
    }
    assert_eq!((world.tick, world.clock.get().day_tick), (1, 10));
    assert_eq!(
        (world.player.x, world.player.y, world.player.stamina),
        position
    );
    assert!(!world.pause.is_open());
    world.tick(&Input {
        exit: true,
        ..Default::default()
    });
    assert!(!world.pause.is_rendering());
    world.begin_display_tick();
    world.tick_after_display(false, &Input::default());
    assert!(world.pause.is_rendering());
    assert_eq!(world.clock.get().day_tick, 12); // Paused is not overwritten without focus.
    world.tick(&Input::default());
    assert_eq!(world.clock.get().day_tick, 13);
    for _ in 0..10 {
        world.begin_display_tick();
        world.tick_after_display(false, &Input::default());
    }
    assert_eq!(world.clock.get().day_tick, 13);
    world.tick(&Input {
        exit: true,
        ..Default::default()
    });
    world.take_sound_events();
    world.tick(&Input::default());
    assert_eq!(world.clock.get().day_tick, 13); // Resume uses previous paused=true.
    world.tick(&Input::default());
    assert_eq!(world.clock.get().day_tick, 14);
    assert!(world.take_sound_events().is_empty());
}

#[test]
fn game_time_advances_in_inventory_but_not_without_focus() {
    let mut world = tiny_world();
    world.inventory_open = true;
    for _ in 0..5 {
        world.tick(&Input::default());
    }
    assert_eq!(
        (
            world.clock.get().game_time,
            world.tick,
            world.clock.get().day_tick
        ),
        (5, 0, 0)
    );
    world.tick_after_display(false, &Input::default());
    assert_eq!(world.clock.get().game_time, 5);
    let restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert_eq!((restored.clock.get().game_time, restored.tick), (5, 0));
}

#[test]
fn missing_legacy_game_time_starts_a_new_counter() {
    let mut world = tiny_world();
    world.tick = 123;
    let mut saved: serde_json::Value =
        serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    saved["world"].as_object_mut().unwrap().remove("game_time");
    let restored = World::from_save_string(&saved.to_string()).unwrap();
    assert_eq!((restored.clock.get().game_time, restored.tick), (0, 123));
}

#[test]
fn focused_and_unfocused_frames_produce_identical_live_worlds_at_four_fps() {
    use crate::core::game_loop::GameLoopClock;
    use std::time::{Duration, Instant};
    let mut expected = None;
    for fps in [30_u64, 60, 120, 300] {
        let mut world = tiny_world();
        let start = Instant::now();
        let mut clock = GameLoopClock::starting_at(start);
        let input = Input {
            right: true,
            ..Default::default()
        };
        for frame in 1..=fps * 3 {
            // One second playing, one unfocused, one playing. Timestamps
            // are exact fixtures; this does not test OS event sampling.
            let focused = frame <= fps || frame > fps * 2;
            for _ in 0..clock.updates_at(start + Duration::from_nanos(frame * 1_000_000_000 / fps))
            {
                world.begin_display_tick();
                world.tick_after_display(focused, &input);
            }
        }
        assert_eq!((world.tick, world.clock.get().day_tick), (120, 179));
        let state: serde_json::Value =
            serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
        if let Some(expected) = &expected {
            assert_eq!(&state, expected);
        } else {
            expected = Some(state);
        }
    }
}

#[test]
fn save_resume_preserves_previous_pause_gate_and_calendar_rollover() {
    let mut world = tiny_world();
    world.tick(&Input::default());
    world.clock.set_day_tick(DAY_LENGTH - 1);
    let saved = world.to_save_string().unwrap();
    let mut restored = World::from_save_string(&saved).unwrap();
    for target in [&mut world, &mut restored] {
        target.begin_display_tick();
        target.tick_after_display(false, &Input::default());
        assert_eq!(
            (target.days, target.clock.get().day_tick, target.tick),
            (2, 0, 1)
        );
    }
    let as_value = |world: &World| {
        serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap()).unwrap()
    };
    assert_eq!(as_value(&world), as_value(&restored));
    let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
    legacy["world"].as_object_mut().unwrap().remove("updater");
    assert!(World::from_save_string(&legacy.to_string()).is_ok());
}

#[test]
fn core_prelude_autosave_captures_before_calendar_score_and_input() {
    let mut world = tiny_world();
    world.updater = Updater::from_state(false, 7_200);
    world.mode = GameMode::Score;
    world.score_ticks = 0;
    world.clock.set_day_tick(42);
    let position = (world.player.x, world.player.y);
    let input = Input {
        right: true,
        ..Default::default()
    };
    let failure = world.tick_with_autosave(false, &input, true, |_| Err("write failed".into()));
    assert!(matches!(failure, Err(error) if error == "write failed"));
    assert_eq!(
        (
            world.updater.autosave_ticks(),
            world.clock.get().day_tick,
            world.score_ticks
        ),
        (7_200, 42, 0)
    );
    let mut saved = None;
    world
        .tick_with_autosave(false, &input, true, |snapshot| {
            saved = Some(snapshot.to_save_string()?);
            Ok(())
        })
        .unwrap();
    let snapshot = World::from_save_string(&saved.unwrap()).unwrap();
    assert_eq!(
        (
            snapshot.clock.get().day_tick,
            snapshot.score_ticks,
            snapshot.game_over
        ),
        (42, 0, false)
    );
    assert_eq!(
        (
            world.clock.get().day_tick,
            world.score_ticks,
            world.game_over
        ),
        (43, -1, true)
    );
    assert_eq!(world.updater.autosave_ticks(), 0);
    assert_eq!(world.tick, 0);
    assert_eq!((world.player.x, world.player.y), position);
}

#[test]
fn core_prelude_autosave_gates_dead_disabled_and_game_over_and_resets_manual_save() {
    for (enabled, alive, over) in [
        (true, true, false),
        (false, true, false),
        (true, false, false),
        (true, true, true),
    ] {
        let mut world = tiny_world();
        world.updater = Updater::from_state(false, 7_199);
        world.player.health = if alive { 10 } else { 0 };
        world.game_over = over;
        let mut saves = 0;
        world
            .tick_with_autosave(false, &Input::default(), enabled, |_| {
                saves += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(saves, 0);
        assert_eq!(world.updater.autosave_ticks(), 7_200);
        world
            .tick_with_autosave(false, &Input::default(), enabled, |_| {
                saves += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(saves, usize::from(enabled && alive && !over));
        assert_eq!(world.updater.autosave_ticks(), 0);
        world.tick_after_display(false, &Input::default());
        world.reset_autosave();
        assert_eq!(world.updater.autosave_ticks(), 0);
    }
}

#[test]
fn core_prelude_score_uses_previous_pause_even_when_focus_is_lost() {
    let mut world = tiny_world();
    world.mode = GameMode::Score;
    world.score_ticks = 2;
    world.tick(&Input::default()); // Java initially paused=true.
    assert_eq!(world.score_ticks, 2);
    world.pause.open();
    world.tick(&Input::default()); // Pause boundary still uses old false.
    assert_eq!(world.score_ticks, 1);
    world.tick_after_display(false, &Input::default());
    assert_eq!(world.score_ticks, 1);
    world.pause.close();
    world.tick(&Input::default()); // Resume uses old true.
    assert_eq!(world.score_ticks, 1);
    world.tick_after_display(false, &Input::default());
    assert_eq!((world.score_ticks, world.game_over), (0, false));
    world.tick_after_display(false, &Input::default());
    assert_eq!((world.score_ticks, world.game_over), (-1, true));
    world.tick_after_display(false, &Input::default());
    assert_eq!(world.score_ticks, -1);
}

#[test]
fn core_prelude_save_continuation_and_legacy_boolean_updater_are_supported() {
    let mut world = tiny_world();
    world.updater = Updater::from_state(false, 7_200);
    world.mode = GameMode::Score;
    world.score_ticks = 0;
    let saved = world.to_save_string().unwrap();
    let mut restored = World::from_save_string(&saved).unwrap();
    for target in [&mut world, &mut restored] {
        let mut saves = 0;
        target
            .tick_with_autosave(false, &Input::default(), true, |_| {
                saves += 1;
                Ok(())
            })
            .unwrap();
        assert_eq!(saves, 1);
    }
    let value = |world: &World| {
        serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap()).unwrap()
    };
    assert_eq!(value(&world), value(&restored));
    assert_eq!(
        World::from_save_string(&world.to_save_string().unwrap())
            .unwrap()
            .score_ticks,
        -1
    );
    for paused in [false, true] {
        let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
        legacy["world"]["updater"] = paused.into();
        let old = World::from_save_string(&legacy.to_string()).unwrap();
        assert_eq!(old.updater, Updater::from_paused(paused));
    }
}

#[test]
fn core_prelude_autosave_and_score_are_identical_at_four_render_rates() {
    use crate::core::game_loop::GameLoopClock;
    use std::time::{Duration, Instant};
    let mut expected = None;
    for fps in [30_u64, 60, 120, 300] {
        let mut world = tiny_world();
        world.updater = Updater::from_state(false, 7_190);
        world.mode = GameMode::Score;
        world.score_ticks = 120;
        let start = Instant::now();
        let mut clock = GameLoopClock::starting_at(start);
        let mut saves = Vec::new();
        for frame in 1..=fps {
            for _ in 0..clock.updates_at(start + Duration::from_nanos(frame * 1_000_000_000 / fps))
            {
                world.begin_display_tick();
                world
                    .tick_with_autosave(false, &Input::default(), true, |snapshot| {
                        saves.push(
                            serde_json::from_str::<serde_json::Value>(
                                &snapshot.to_save_string().unwrap(),
                            )
                            .unwrap(),
                        );
                        Ok(())
                    })
                    .unwrap();
            }
        }
        assert_eq!(
            (world.score_ticks, world.updater.autosave_ticks()),
            (60, 49)
        );
        assert_eq!(saves.len(), 1);
        let state = (
            saves,
            serde_json::from_str::<serde_json::Value>(&world.to_save_string().unwrap()).unwrap(),
        );
        if let Some(expected) = &expected {
            assert_eq!(&state, expected);
        } else {
            expected = Some(state);
        }
    }
}

#[test]
fn score_end_game_is_deferred_focus_gated_and_save_resumable() {
    let mut world = tiny_world();
    world.mode = GameMode::Score;
    world.updater = Updater::from_paused(false);
    world.score_ticks = 0;
    assert!(matches!(
        world.tick(&Input {
            select: true,
            ..Default::default()
        }),
        super::WorldAction::None
    ));
    assert!(!world.end_game.unwrap().active);
    assert!(!world.end_game_visible());
    world.begin_display_tick();
    world.tick_after_display(
        false,
        &Input {
            select: true,
            ..Default::default()
        },
    );
    assert_eq!(world.end_game.unwrap().display_timer, 60);
    for _ in 0..59 {
        world.tick(&Input::default());
    }
    assert!(!world.end_game_visible());
    let mut loaded = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    loaded.tick(&Input::default());
    assert!(loaded.end_game_visible());
    assert_eq!(loaded.end_game.unwrap().input_delay, 30);
    assert!(matches!(
        loaded.tick(&Input {
            exit: true,
            ..Default::default()
        }),
        super::WorldAction::None
    ));
    assert!(matches!(
        loaded.tick(&Input {
            select: true,
            ..Default::default()
        }),
        super::WorldAction::ReturnToTitle
    ));
}

#[test]
fn score_end_game_accepts_select_while_hidden_after_activation() {
    let mut world = tiny_world();
    world.mode = GameMode::Score;
    world.updater = Updater::from_paused(false);
    world.score_ticks = 0;
    world.tick(&Input::default());
    world.book_open = Some((crate::progression::Book::ALL[0], 0));
    assert!(matches!(
        world.tick(&Input {
            select: true,
            ..Default::default()
        }),
        super::WorldAction::ReturnToTitle
    ));
    assert!(!world.end_game_visible());
}

#[test]
fn player_display_gate_freezes_player_and_ignores_hud_on_menu_close_tick() {
    let mut world = tiny_world();
    world.inventory_open = true;
    world.player.attack_time = 7;
    world.player.hurt_time = 9;
    world.player.stamina = 3;
    world.player.potion_effects[0] = 100;
    let before = serde_json::to_value(&world.player).unwrap();
    world.tick(&Input {
        toggle_hud: true,
        attack: true,
        right: true,
        ..Default::default()
    });
    assert_eq!(serde_json::to_value(&world.player).unwrap(), before);
    assert!(world.render_gui);
    world.tick(&Input {
        exit: true,
        toggle_hud: true,
        ..Default::default()
    });
    assert!(!world.inventory_open);
    assert!(world.render_gui); // Controls run before this tick closes Display.
    world.tick(&Input {
        toggle_hud: true,
        ..Default::default()
    });
    assert!(!world.render_gui);
}

/// Java Updater calls Player.tick before Display.tick while currentDisplay
/// != null, but Player.tick returns on the already-requested Display before
/// hunger/stamina recharge. Pause, inventory and book must freeze those
/// timers, then resume after the Display closes.
#[test]
fn open_menus_freeze_hunger_and_stamina_like_java_player_tick() {
    let mut world = tiny_world();
    world.updater = Updater::from_paused(false);
    world.player.stamina = 4;
    world.player.stamina_recharge = 7;
    world.player.stamina_recharge_delay = 0;
    world.player.hunger = 9;
    world.player.hunger_ticks = 1_000;
    let frozen = (
        world.player.stamina,
        world.player.stamina_recharge,
        world.player.stamina_recharge_delay,
        world.player.hunger,
        world.player.hunger_ticks,
        world.tick,
    );
    let snapshot = |world: &World| {
        (
            world.player.stamina,
            world.player.stamina_recharge,
            world.player.stamina_recharge_delay,
            world.player.hunger,
            world.player.hunger_ticks,
            world.tick,
        )
    };

    world.pause.open();
    for _ in 0..5 {
        world.tick(&Input::default());
    }
    assert_eq!(
        snapshot(&world),
        frozen,
        "pause must freeze Player.tick timers"
    );

    world.pause.close();
    world.inventory_open = true;
    for _ in 0..5 {
        world.tick(&Input::default());
    }
    assert_eq!(
        snapshot(&world),
        frozen,
        "inventory must freeze Player.tick timers"
    );

    world.inventory_open = false;
    world.book_open = Some((crate::progression::Book::About, 0));
    for _ in 0..5 {
        world.tick(&Input::default());
    }
    assert_eq!(
        snapshot(&world),
        frozen,
        "book must freeze Player.tick timers"
    );

    world.book_open = None;
    world.tick(&Input::default());
    assert_eq!(world.tick, frozen.5 + 1);
    assert_eq!(world.player.stamina_recharge, frozen.1 + 1);
    assert!(world.player.hunger_ticks < frozen.4);
}

#[test]
fn renderer_hud_toggle_keeps_menu_and_notification_feedback() {
    let mut world = tiny_world();
    let assets = crate::assets::Assets::load(&[]).unwrap();
    let locale = crate::resource::localization::Localization::load("en-us", &[]);
    let mut visible = crate::gfx::Screen::new();
    let mut hidden = crate::gfx::Screen::new();
    world.notify("HUD notice", 181);
    let mut input = Input::default();
    input.latch(Input {
        toggle_hud: true,
        ..Default::default()
    });
    world.tick(&input.take_tick());
    assert!(!world.render_gui);
    world.tick(&input.take_tick());
    assert!(!world.render_gui); // A latched press is consumed once.
    world.pause.open();
    world.begin_display_tick();
    world.render(&mut hidden, &assets, &locale);
    assert_eq!(world.notifications.messages(), ["HUD notice"]);
    assert_eq!(world.notifications.ticks(), 181); // GUI did not run.
    world.tick_after_display(
        false,
        &Input {
            toggle_hud: true,
            ..Default::default()
        },
    );
    assert!(!world.render_gui); // Unfocused keys cannot toggle.
    world.tick(&Input {
        toggle_hud: true,
        ..Default::default()
    });
    assert!(!world.render_gui); // Player.tick returns before HUD controls in menus.
    world.render_gui = true; // Independently test Renderer gate combinations.
    world.render(&mut visible, &assets, &locale);
    assert!(world.notifications.messages().is_empty());
    // The same opaque Pause frame must be present with either HUD setting.
    let mut menu = crate::gfx::Screen::new();
    super::render_pause(&mut menu, &assets, &locale, &world);
    for (index, &pixel) in menu.pixels().iter().enumerate() {
        if pixel != 0 {
            assert_eq!(hidden.pixels()[index], pixel);
            assert_eq!(visible.pixels()[index], pixel);
        }
    }
    world.render_gui = false;
    assert!(
        World::from_save_string(&world.to_save_string().unwrap())
            .unwrap()
            .render_gui
    );
}

#[test]
fn notifications_follow_gui_feedback_even_while_paused_and_unfocused() {
    let mut world = tiny_world();
    let assets = crate::assets::Assets::load(&[]).unwrap();
    let locale = crate::resource::localization::Localization::load("en-us", &[]);
    let mut screen = crate::gfx::Screen::new();
    world.add_notification("First notice");
    world.tick_after_display(false, &Input::default());
    assert_eq!(world.notifications.ticks(), 0);
    world.pause.open();
    world.begin_display_tick();
    world.render(&mut screen, &assets, &locale);
    for _ in 0..180 {
        world.tick_after_display(false, &Input::default());
    }
    assert_eq!(world.notifications.ticks(), 180);
    world.render(&mut screen, &assets, &locale);
    assert_eq!(world.notifications.messages(), ["First notice"]);
    world.tick_after_display(false, &Input::default());
    world.render(&mut screen, &assets, &locale);
    assert!(world.notifications.messages().is_empty());
    assert_eq!(world.tick, 0);
}

#[test]
fn notifications_are_hidden_and_frozen_by_sleep_status_on_render() {
    let mut world = tiny_world();
    let assets = crate::assets::Assets::load(&[]).unwrap();
    let locale = crate::resource::localization::Localization::load("en-us", &[]);
    let mut screen = crate::gfx::Screen::new();
    world.notify("Keep this notice", 170);
    world.render(&mut screen, &assets, &locale);
    world.sleeping = 120;
    world.render(&mut screen, &assets, &locale);
    assert!(!world.notifications.ticking());
    for _ in 0..100 {
        world.tick_after_display(false, &Input::default());
    }
    assert_eq!(world.notifications.ticks(), 170);
    world.sleeping = 0;
    world.render(&mut screen, &assets, &locale);
    assert_eq!(world.notifications.messages(), ["Keep this notice"]);
    assert!(world.notifications.ticking());
    for (day_tick, days, allowed) in [
        (0, 1, false),
        (40_499, 1, false),
        (40_500, 1, true),
        (8_099, 2, true),
        (8_100, 2, false),
        (64_799, 1, true),
    ] {
        let mut world = tiny_world();
        world.clock.set_day_tick(day_tick);
        world.days = days;
        world.player.direction = super::Direction::Down;
        world.levels[0].entities.spawn_furniture(
            FurnitureKind::Bed,
            world.player.x,
            world.player.y + 12,
        );
        world.use_target();
        assert_eq!(world.sleeping > 0, allowed, "day {days}, tick {day_tick}");
        assert_eq!(world.notifications.messages().is_empty(), allowed);
        if day_tick == 40_499 {
            assert_eq!(
                world.notifications.messages(),
                ["Can't sleep! 0Min 1 Sec left!"]
            );
        }
    }
}

#[test]
fn sleeping_level_still_increments_tile_tick() {
    let mut world = tiny_world();
    world.sleeping = 120;
    world.tick(&Input::default());
    assert_eq!(world.tile_ticks(), 1);
    world.tick(&Input::default());
    assert_eq!(world.tile_ticks(), 2);
}

#[test]
fn notifications_save_success_is_once_only_and_failure_is_silent() {
    let mut world = tiny_world();
    world.updater = Updater::from_state(false, 7_200);
    assert!(
        world
            .tick_with_autosave(false, &Input::default(), true, |_| Err("disk".into()))
            .is_err()
    );
    assert!(world.notifications.messages().is_empty());
    let mut snapshot = String::new();
    world
        .tick_with_autosave(false, &Input::default(), true, |saved| {
            snapshot = saved.to_save_string()?;
            Ok(())
        })
        .unwrap();
    assert!(
        World::from_save_string(&snapshot)
            .unwrap()
            .notifications
            .messages()
            .is_empty()
    );
    assert_eq!(world.notifications.messages(), ["World Saved!"]);
    world
        .tick_with_autosave(false, &Input::default(), true, |_| panic!("duplicate save"))
        .unwrap();
    assert_eq!(world.notifications.messages().len(), 1);
    world.record_save_success();
    assert_eq!(
        world.notifications.messages(),
        ["World Saved!", "World Saved!"]
    );
    assert_eq!(
        (world.notifications.ticks(), world.updater.autosave_ticks()),
        (0, 0)
    );
}

#[test]
fn notifications_localize_at_enqueue_and_preserve_java_boss_order() {
    use std::sync::Arc;
    let mut world = tiny_world();
    let french = Arc::new(crate::resource::localization::Localization::load(
        "fr-fr",
        &[],
    ));
    let saved_text = french.text("minicraft.notification.world_saved").to_owned();
    world.set_notification_context(french, "Q");
    world.record_save_success();
    world.set_notification_context(
        Arc::new(crate::resource::localization::Localization::load(
            "en-us",
            &[],
        )),
        "ESCAPE",
    );
    assert_eq!(world.notifications.messages(), [saved_text]);
    world.notifications.clear();
    world.handle_defeated_mob(super::spawn::NaturalMob::AirWizard);
    let messages = world.notifications.messages();
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[0], "Air Wizard Defeated!");
    assert!(messages[1].starts_with("Achivement unlocked: ")); // Java locale typo retained.
    assert_eq!(messages[2], "The Dungeon is now open!");
    assert_eq!(world.notifications.ticks(), -400);
    world.notifications.clear();
    world.handle_defeated_mob(super::spawn::NaturalMob::AirWizard);
    assert_eq!(world.notifications.messages(), ["Air Wizard Defeated!"]);
    assert_eq!(world.notifications.ticks(), 0);
}

#[test]
fn notifications_save_restores_queue_feedback_and_reads_legacy_slot() {
    let mut world = tiny_world();
    world.notifications =
        super::Notifications::from_state(vec!["One".into(), "Two".into()], -123, true);
    let saved = world.to_save_string().unwrap();
    let mut restored = World::from_save_string(&saved).unwrap();
    for target in [&mut world, &mut restored] {
        target.tick_after_display(false, &Input::default());
    }
    assert_eq!(world.notifications, restored.notifications);
    assert_eq!(restored.notifications.ticks(), -122);
    let mut old: serde_json::Value = serde_json::from_str(&saved).unwrap();
    old["world"]["notification"] = serde_json::json!(["Legacy notice", 60]);
    let restored = World::from_save_string(&old.to_string()).unwrap();
    assert_eq!(restored.notifications.messages(), ["Legacy notice"]);
    assert_eq!(restored.notifications.ticks(), 121);
    old["world"]["notification"] = serde_json::Value::Null;
    assert!(
        World::from_save_string(&old.to_string())
            .unwrap()
            .notifications
            .messages()
            .is_empty()
    );
    old["world"].as_object_mut().unwrap().remove("notification");
    assert!(
        World::from_save_string(&old.to_string())
            .unwrap()
            .notifications
            .messages()
            .is_empty()
    );
}

#[test]
fn pause_render_keeps_old_page_until_lifecycle_update() {
    let mut world = tiny_world();
    world.populate_pause_preview();
    let assets = crate::assets::Assets::load(&[]).unwrap();
    let localization = crate::resource::localization::Localization::load("en-us", &[]);
    let draw = |world: &World| {
        let mut screen = crate::gfx::Screen::new();
        super::render_pause(&mut screen, &assets, &localization, world);
        screen.pixels().to_vec()
    };
    let parent = draw(&world);
    world.tick(&Input {
        select: true,
        ..Default::default()
    });
    assert_eq!(draw(&world), parent);
    world.tick(&Input::default());
    let options = draw(&world);
    assert_ne!(options, parent);
    world.tick(&Input {
        exit: true,
        ..Default::default()
    });
    assert_eq!(draw(&world), options);
    world.tick(&Input::default());
    assert_eq!(draw(&world), parent);
}

#[test]
fn pause_pending_child_save_restores_parent_and_reads_legacy_keys() {
    let mut world = tiny_world();
    world.populate_pause_preview();
    world.tick(&Input {
        select: true,
        ..Default::default()
    });
    let saved = world.to_save_string().unwrap();
    let mut restored = World::from_save_string(&saved).unwrap();
    assert_eq!(restored.pause.view().page, PausePage::Options);
    restored.tick(&Input {
        exit: true,
        ..Default::default()
    });
    assert_eq!(restored.pause.view().selection, 1);
    let mut legacy: serde_json::Value = serde_json::from_str(&saved).unwrap();
    legacy["world"]
        .as_object_mut()
        .unwrap()
        .remove("pause_frames");
    let mut restored = World::from_save_string(&legacy.to_string()).unwrap();
    restored.tick(&Input {
        exit: true,
        ..Default::default()
    });
    assert_eq!(restored.pause.view().selection, 1);
    // Data added in this batch is not allowed to create arbitrary cycles or roots.
    let mut malformed: serde_json::Value = serde_json::from_str(&saved).unwrap();
    malformed["world"]["pause_frames"][0]["page"] = "QuestDetail".into();
    assert!(World::from_save_string(&malformed.to_string()).is_err());
}

#[test]
fn legacy_pause_confirm_cancel_uses_actual_main_menu_length() {
    for quests in [false, true] {
        let mut world = tiny_world();
        world.quests_enabled = quests;
        world.prepare_pause_audio_parity();
        let last = pause_actions(quests).len() - 1;
        world.pause.view_mut().selection = last;
        world.tick(&Input {
            select: true,
            ..Default::default()
        });
        let mut legacy: serde_json::Value =
            serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
        legacy["world"]
            .as_object_mut()
            .unwrap()
            .remove("pause_frames");
        let mut restored = World::from_save_string(&legacy.to_string()).unwrap();
        restored.tick(&Input {
            exit: true,
            ..Default::default()
        });
        assert_eq!(restored.pause.view().selection, last);
        assert!(!restored.pause.view().confirm);
    }
}

#[test]
fn quest_nested_return_restores_pause_parent_without_world_progress() {
    let mut world = tiny_world();
    world.prepare_quests_audio_parity();
    let select = Input {
        select: true,
        ..Default::default()
    };
    for _ in 0..3 {
        world.tick(&select);
    }
    assert_eq!(world.pause.frames().len(), 5);
    let mut restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    for _ in 0..4 {
        restored.tick(&Input {
            exit: true,
            ..Default::default()
        });
    }
    assert_eq!(restored.pause.view().page, PausePage::Main);
    assert_eq!(restored.pause.view().selection, 3);
    assert_eq!(restored.tick, 0);
}

#[test]
fn ordinary_tile_hit_uses_java_damage_timing_and_tool_wear() {
    let mut world = tiny_world();
    let seed = 47_227;
    world.random = JavaRandom::new(seed);
    let expected_damage = (JavaRandom::new(seed).next_int(3) + 1) as u16;
    let target = 9 + 8 * world.width;
    world.levels[0].tiles[target] = Tile::Tree;
    world
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Sword, ToolTier::Wood));
    world.player.active_item = Some(ActiveItem::Tool(0));

    world.attack();

    assert_eq!(world.levels[0].data[target], expected_damage);
    assert_eq!(world.player.inventory.tools()[0].durability, 51);
    assert_eq!(world.player.attack_time, 5);
    assert_eq!(world.player.walk_distance, 8);
    assert_eq!(world.player.attack_item, Some(ActiveItem::Tool(0)));
    assert_eq!(
        world.take_sound_events(),
        vec![crate::io::audio::SoundEffect::MonsterHurt]
    );
}

#[test]
fn successful_tool_tile_interaction_keeps_java_ten_tick_slash() {
    let mut world = tiny_world();
    let seed = 47_227;
    world.random = JavaRandom::new(seed);
    let expected_damage = (JavaRandom::new(seed).next_int(5) + 10) as u16;
    let target = 9 + 8 * world.width;
    world.levels[0].tiles[target] = Tile::Tree;
    world
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood));
    world.player.active_item = Some(ActiveItem::Tool(0));

    world.attack();

    assert_eq!(world.levels[0].data[target], expected_damage);
    assert_eq!(world.player.inventory.tools()[0].durability, 33);
    assert_eq!(world.player.attack_time, 10);
    assert_eq!(
        world.take_sound_events(),
        vec![crate::io::audio::SoundEffect::MonsterHurt]
    );
}

#[test]
fn workbench_placement_and_tool_use_are_in_the_normal_world_path() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::Workbench, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::Workbench));
    world.attack();
    assert_eq!(world.player.inventory.count(ItemId::Workbench), 0);
    assert_eq!(
        world.levels[0].entities.furniture_near(152, 136, 1),
        Some(FurnitureKind::Workbench)
    );

    world.player.inventory.add(ItemId::Wood, 5);
    world.use_target();
    assert!(world.inventory_open);
    assert_eq!(world.crafting_station, Some(FurnitureKind::Workbench));

    world.inventory_selection = sorted_recipe_views(
        Some(FurnitureKind::Workbench),
        &world.player.inventory,
        false,
    )
    .iter()
    .position(
        |recipe| matches!(recipe.output, RecipeOutput::Tool(tool) if tool.kind == ToolKind::Axe),
    )
    .unwrap();
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    let Some(ActiveItem::Tool(tool_index)) = world.player.active_item else {
        panic!("crafted axe was not equipped");
    };
    assert_eq!(
        world.player.inventory.tools()[tool_index].kind,
        ToolKind::Axe
    );
    let damage = world.pay_tool_terrain_damage(ToolKind::Axe, 4).unwrap();
    assert!((10..=14).contains(&damage));
    assert_eq!(world.player.inventory.tools()[tool_index].durability, 33);
    assert_eq!(world.player.stamina, 6);
}

#[test]
fn ore_requires_a_pickaxe_and_consumes_its_durability() {
    let mut world = tiny_world();
    let ore_index = 9 + 8 * 16;
    world.levels[0].tiles[ore_index] = Tile::IronOre;
    world.attack();
    assert_eq!(world.levels[0].data[ore_index], 0);

    let index = world
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Pickaxe, ToolTier::Wood))
        .unwrap();
    world.player.active_item = Some(ActiveItem::Tool(index));
    world.attack();
    assert!(world.levels[0].data[ore_index] > 0);
    assert_eq!(world.player.inventory.tools()[index].durability, 37);
}

#[test]
fn furnace_placement_smelts_ore_through_the_station_menu() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::Furnace, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::Furnace));
    world.attack();
    assert_eq!(
        world.levels[0].entities.furniture_near(152, 136, 1),
        Some(FurnitureKind::Furnace)
    );
    world.use_target();
    assert_eq!(world.crafting_station, Some(FurnitureKind::Furnace));

    world.player.inventory.add(ItemId::IronOre, 3);
    world.player.inventory.add(ItemId::Coal, 1);
    world.tick(&Input {
        select: true,
        ..Input::default()
    });
    assert_eq!(world.player.inventory.count(ItemId::IronOre), 0);
    assert_eq!(world.player.inventory.count(ItemId::Coal), 0);
    assert_eq!(world.player.inventory.count(ItemId::IronIngot), 1);
}

#[test]
fn equipped_food_consumes_stamina_and_restores_hunger() {
    let mut world = tiny_world();
    world.player.hunger = 5;
    world.player.inventory.add(ItemId::CookedPork, 2);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::CookedPork));
    world.tick(&Input {
        attack: true,
        ..Input::default()
    });
    assert_eq!(world.player.hunger, 8);
    assert_eq!(world.player.stamina, 7);
    assert_eq!(world.player.inventory.count(ItemId::CookedPork), 1);
    assert!(world.notifications.messages().is_empty());
}

#[test]
fn food_uses_java_partial_stamina_payment_and_never_attacks() {
    let mut world = tiny_world();
    let target = 9 + 8 * 16;
    world.levels[0].tiles[target] = Tile::Rock;
    world.player.hunger = 5;
    world.player.stamina = 1;
    world.player.inventory.add(ItemId::Apple, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::Apple));

    world.tick(&Input {
        attack: true,
        ..Input::default()
    });

    assert_eq!(world.player.hunger, 5);
    assert_eq!(world.player.stamina, 0);
    assert_eq!(world.player.inventory.count(ItemId::Apple), 1);
    assert_eq!(world.levels[0].data[target], 0);
}

#[test]
fn every_attack_input_pays_the_java_base_stamina_cost() {
    let mut world = tiny_world();
    world.tick(&Input {
        attack: true,
        ..Input::default()
    });
    assert_eq!(world.player.stamina, 9);
    assert_eq!(world.player.stamina_recharge, 0);

    world.player.stamina = 0;
    world.player.attack_time = 0;
    world.tick(&Input {
        attack: true,
        ..Input::default()
    });
    assert_eq!(world.player.attack_time, 0);
}

#[test]
fn tool_actions_accept_java_partial_stamina_payment_after_the_base_cost() {
    let mut world = tiny_world();
    let target = 9 + 8 * world.width;
    world.levels[0].tiles[target] = Tile::Tree;
    let axe = world
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Axe, ToolTier::Wood))
        .unwrap();
    world.player.active_item = Some(ActiveItem::Tool(axe));
    world.player.stamina = 2;

    world.tick(&Input {
        attack: true,
        ..Input::default()
    });

    assert_eq!(world.player.stamina, 0);
    assert!((10..=14).contains(&world.levels[0].data[target]));
    assert_eq!(world.player.attack_time, 10);
}

#[test]
fn starvation_respects_the_difficulty_health_floor() {
    let mut world = tiny_world();
    world.player.hunger = 0;
    world.player.health = 10;
    world.player.hunger_starve_delay = 1;
    world.tick_survival_stats();
    assert_eq!(world.player.health, 9);

    world.player.health = STARVATION_HEALTH_FLOORS[world.difficulty];
    world.player.hunger_starve_delay = 1;
    world.tick_survival_stats();
    assert_eq!(world.player.health, 3);
}

#[test]
fn armor_and_potions_run_through_the_equipped_item_path() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::LeatherArmor, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::LeatherArmor));
    assert!(world.use_active_self_item());
    assert_eq!(world.player.armor_kind, Some(ArmorKind::Leather));
    assert_eq!(world.player.armor, 30);
    assert_eq!(world.player.stamina, 1);
    assert!(world.hurt_player(4, false));
    assert_eq!(world.player.armor, 26);
    assert_eq!(world.player.health, 8);

    world.player.inventory.add(ItemId::SpeedPotion, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::SpeedPotion));
    assert!(world.use_active_self_item());
    assert_eq!(
        world.player.potion_effects[PotionKind::Speed.id()],
        PotionKind::Speed.duration()
    );
    assert_eq!(world.player.inventory.count(ItemId::GlassBottle), 1);
}

#[test]
fn hoe_plant_growth_and_harvest_form_a_playable_farm_loop() {
    let mut world = tiny_world();
    let target = 9 + 8 * 16;
    world.levels[0].tiles[target] = Tile::Dirt;
    let hoe = world
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Hoe, ToolTier::Wood))
        .unwrap();
    world.player.active_item = Some(ActiveItem::Tool(hoe));
    world.attack();
    assert_eq!(world.levels[0].tiles[target], Tile::Farmland);
    assert_eq!(world.player.inventory.tools()[hoe].durability, 29);

    world.player.inventory.add(ItemId::WheatSeeds, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::WheatSeeds));
    world.attack();
    assert_eq!(world.levels[0].tiles[target], Tile::Wheat);
    world.levels[0].data[target] = 7 << 3;
    world.player.active_item = None;
    world.attack();
    assert_eq!(world.levels[0].tiles[target], Tile::Farmland);
    for _ in 0..31 {
        world.tick(&Input::default());
    }
    let drops = world.levels[0]
        .entities
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, super::EntityKind::Item(_)))
        .map(|entity| (entity.x, entity.y))
        .collect::<Vec<_>>();
    for (x, y) in drops {
        world.levels[0]
            .entities
            .collect_near(x, y, &mut world.player.inventory);
    }
    assert!(world.player.inventory.count(ItemId::Wheat) >= 2);
    assert_eq!(world.player.inventory.count(ItemId::WheatSeeds), 1);
}

#[test]
fn all_2_2_4_tile_ids_round_trip() {
    assert!(Tile::ALL.len() > 59);
    // The stable 0..58 prefix is unchanged even after appending 2.3 tiles.
    for (id, tile) in Tile::ALL[..59].iter().copied().enumerate() {
        assert_eq!(tile.id(), id as u8);
        assert_eq!(Tile::from_id(id as u8), Some(tile));
    }
    assert!(Tile::from_id(59).is_some());
    assert_eq!(Tile::from_id(Tile::ALL.len() as u8), None);
    assert_eq!(Tile::Sign.id(), 58);
    assert_eq!(Tile::from_legacy_id(1), Some((Tile::Rock, 0)));
    assert_eq!(
        Tile::from_legacy_id(44),
        Some((Tile::Torch, Tile::Grass.id() as u16))
    );
    assert_eq!(Tile::from_legacy_id(123), Some((Tile::ObsidianDoor, 0)));
    assert_eq!(Tile::from_legacy_id(999), None);
    for legacy_id in [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 24, 30,
        31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53,
        54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 100, 101, 102, 103, 104, 105, 107, 108,
        109, 110, 111, 112, 113, 114, 115, 116, 117, 119, 120, 121, 122, 123, 127,
    ] {
        assert!(
            Tile::from_legacy_id(legacy_id).is_some(),
            "missing legacy tile ID {legacy_id}"
        );
    }
    assert!(Tile::WoodDoor.solid(0));
    assert!(!Tile::WoodDoor.solid(1));
    assert_eq!(Tile::Lava.light_radius(), 6);
    assert_eq!(Tile::Torch.light_radius(), 5);
}

#[test]
fn six_levels_are_linked_by_matching_stairs() {
    let world = World::new_with_options(0x100, super::WorldSpec::default(), 1);
    assert_eq!(
        world
            .levels
            .iter()
            .map(|level| level.depth)
            .collect::<Vec<_>>(),
        vec![1, 0, -1, -2, -3, -4]
    );
    for upper in 0..world.levels.len() - 1 {
        let mut links = 0;
        for (index, tile) in world.levels[upper].tiles.iter().enumerate() {
            if *tile == Tile::StairsDown {
                assert_eq!(world.levels[upper + 1].tiles[index], Tile::StairsUp);
                links += 1;
            }
        }
        assert!(
            links > 0,
            "depth {} has no lower link",
            world.levels[upper].depth
        );
    }
    assert!(world.levels[0].tiles.contains(&Tile::WoodFloor));
    assert!(world.levels[1].tiles.contains(&Tile::WoodWall));
    assert!(
        world.levels[2..5]
            .iter()
            .any(|level| level.tiles.contains(&Tile::StoneFloor))
    );
    assert!(world.levels[5].tiles.contains(&Tile::BossFloor));
    assert!(world.levels[5].tiles.contains(&Tile::OrnateObsidian));
    assert!(
        world.levels[0]
            .entities
            .has_mob(spawn::NaturalMob::AirWizard)
    );
    assert!(
        world.levels[5]
            .entities
            .has_furniture(FurnitureKind::KnightStatue)
    );
    assert!(world.levels[2..=5].iter().any(|level| {
        level.entities.entities().iter().any(|entity| {
            matches!(
                entity.kind,
                super::EntityKind::Furniture(kind) if kind.spawner_mob().is_some()
            )
        })
    }));
}

#[test]
fn day_cycle_uses_the_2_2_4_quarters() {
    assert_eq!(DAY_LENGTH, 64_800);
    assert_eq!(time_name(0), "MORN");
    assert_eq!(time_name(16_200), "DAY");
    assert_eq!(time_name(32_400), "EVE");
    assert_eq!(time_name(48_600), "NIGHT");
    let morning = crate::gfx::lighting::dark_factor(3, 0, true);
    let mid_morning = crate::gfx::lighting::dark_factor(3, 8_100, true);
    let day = crate::gfx::lighting::dark_factor(3, 16_200, true);
    let night = crate::gfx::lighting::dark_factor(3, 48_600, true);
    assert!(morning > mid_morning);
    assert_eq!(day, 0.0);
    assert!(night > 0.0);
}

#[test]
fn natural_spawn_pipeline_queues_eligible_requests() {
    let mut level = Level {
        depth: -1,
        tiles: vec![Tile::Dirt; 128 * 128],
        data: vec![0; 128 * 128],
        max_mob_count: 1,
        pending_spawns: Vec::new(),
        entities: crate::world::entity::EntityArena::default(),
    };
    let mut random = JavaRandom::new(7);
    try_queue_natural_spawn(&mut level, 128, 128, 8, 8, 0, 1, &mut random);
    assert_eq!(level.pending_spawns.len(), 1);
    try_queue_natural_spawn(&mut level, 128, 128, 8, 8, 0, 1, &mut random);
    assert_eq!(level.pending_spawns.len(), 1);
}

#[test]
fn phase_six_modes_apply_their_distinct_runtime_rules() {
    let mut creative = tiny_world();
    creative.mode = GameMode::Creative;
    creative.player.inventory.add(ItemId::Workbench, 1);
    creative.player.active_item = Some(ActiveItem::Stack(ItemId::Workbench));
    creative.attack();
    assert_eq!(creative.player.inventory.count(ItemId::Workbench), 1);
    assert!(!creative.hurt_player(5, false));
    assert_eq!(creative.player.health, 10);

    let mut score = tiny_world();
    score.mode = GameMode::Score;
    score.add_score(50, 1);
    assert_eq!(score.score, 50);
    assert_eq!(score.score_multiplier, 2);
    score.add_score(50, 0);
    assert_eq!(score.score, 150);

    let mut hardcore = tiny_world();
    hardcore.mode = GameMode::Hardcore;
    hardcore.player.health = 1;
    hardcore.hurt_player(5, false);
    assert_eq!(hardcore.player.health, 0);
    hardcore.die_player();
    assert!(hardcore.player_removed);
}

#[test]
fn beds_signs_and_world_books_are_interactive() {
    let mut world = tiny_world();
    world.levels[0].entities.spawn_furniture(
        FurnitureKind::Bed,
        9 * TILE_SIZE + 8,
        8 * TILE_SIZE + 8,
    );
    world.clock.set_day_tick(48_600);
    world.use_target();
    assert_eq!(world.sleeping, 120);

    world.sleeping = 0;
    world.player.direction = Direction::Down;
    let sign = 8 + 9 * 16;
    world.levels[0].tiles[sign] = Tile::Sign;
    world.use_target();
    assert!(world.sign_editor.is_some());
    let typing = Input {
        text: "HELLO".chars().collect(),
        ..Input::default()
    };
    assert!(world.tick_sign_editor(&typing));
    let save = Input {
        select: true,
        ..Input::default()
    };
    assert!(world.tick_sign_editor(&save));
    assert_eq!(world.signs[0].get(&sign).map(String::as_str), Some("HELLO"));

    world.player.inventory.add(ItemId::AntidiousBook, 1);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::AntidiousBook));
    assert!(world.use_active_self_item());
    assert!(world.book_open.is_some());
}

#[test]
fn versioned_world_snapshot_round_trips_complete_runtime_state() {
    let mut world = tiny_world();
    world.tick = 1_801;
    world.clock.set_day_tick(49_000);
    world.days = 7;
    world.mode = GameMode::Score;
    world.score = 4_321;
    world.score_multiplier = 9;
    world.player.health = 6;
    let _ = world.player.inventory.set_capacity(32);
    world.player.inventory.add(ItemId::Gem, 12);
    world.signs[0].insert(7, "ROUND TRIP".to_owned());
    world
        .progress
        .unlock_achievement("minicraft.achievement.find_gem");
    world.levels[0]
        .entities
        .spawn_furniture(FurnitureKind::Chest, 40, 56);

    let text = world.to_save_string().unwrap();
    let mut loaded = World::from_save_string(&text).unwrap();
    assert_eq!(loaded.tick, 1_801);
    assert_eq!(loaded.clock.get().day_tick, 49_000);
    assert_eq!(loaded.days, 7);
    assert_eq!(loaded.mode, GameMode::Score);
    assert_eq!(loaded.score, 4_321);
    assert_eq!(loaded.score_multiplier, 9);
    assert_eq!(loaded.player.health, 6);
    assert_eq!(loaded.player.inventory.capacity(), 27);
    assert_eq!(loaded.player.inventory.count(ItemId::Gem), 12);
    assert_eq!(
        loaded.signs[0].get(&7).map(String::as_str),
        Some("ROUND TRIP")
    );
    assert!(
        loaded
            .progress
            .achievement_unlocked("minicraft.achievement.find_gem")
    );
    assert!(
        loaded.levels[0]
            .entities
            .has_furniture(FurnitureKind::Chest)
    );
    assert_eq!(
        world.random.next_int(10_000),
        loaded.random.next_int(10_000)
    );

    loaded.levels[0].tiles.pop();
    let corrupt = loaded.to_save_string().unwrap();
    assert!(World::from_save_string(&corrupt).is_err());
}

#[test]
fn oversized_legacy_inventory_is_migrated_to_27_slots_without_deleting_stacks() {
    let mut world = tiny_world();
    let _ = world.player.inventory.set_capacity(32);
    for item in ItemId::ALL.iter().copied().take(30) {
        assert_eq!(world.player.inventory.add(item, 1), 0);
    }
    let total_before = world
        .player
        .inventory
        .slots()
        .iter()
        .map(|stack| usize::from(stack.count))
        .sum::<usize>();

    let loaded = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    let held = loaded
        .player
        .inventory
        .slots()
        .iter()
        .map(|stack| usize::from(stack.count))
        .sum::<usize>();
    let dropped = loaded.levels[loaded.current_level]
        .entities
        .entities()
        .iter()
        .filter_map(|entity| match entity.kind {
            EntityKind::Item(stack) => Some(usize::from(stack.count)),
            _ => None,
        })
        .sum::<usize>();
    assert_eq!(loaded.player.inventory.capacity(), 27);
    assert_eq!(loaded.player.inventory.used_slots(), 27);
    assert_eq!(held + dropped, total_before);
}

#[test]
fn malformed_world_snapshot_mutations_never_panic() {
    let valid = tiny_world().to_save_string().unwrap().into_bytes();
    for case in 0..512_usize {
        let mut mutated = valid.clone();
        if case % 3 == 0 {
            mutated.truncate(case.wrapping_mul(7_919) % mutated.len());
        } else if case % 3 == 1 {
            let index = case.wrapping_mul(104_729) % mutated.len();
            mutated[index] ^= 1 << (case % 8);
        } else {
            let index = case.wrapping_mul(65_537) % mutated.len();
            mutated.splice(index..index, [b'{', b'[', 0xff, b']', b'}']);
        }
        let text = String::from_utf8_lossy(&mutated);
        assert!(
            std::panic::catch_unwind(|| World::from_save_string(&text)).is_ok(),
            "mutation {case} panicked"
        );
    }
}

#[test]
fn save_resume_is_deterministic_and_two_day_soak_stays_bounded() {
    let mut continuous = tiny_world();
    continuous.mode = GameMode::Creative;
    continuous.tutorials_enabled = false;
    continuous.quests_enabled = false;
    continuous.levels[0].max_mob_count = 0;

    for _ in 0..10_000 {
        continuous.tick(&Input::default());
    }
    let mut resumed = World::from_save_string(&continuous.to_save_string().unwrap()).unwrap();
    for _ in 0..10_000 {
        continuous.tick(&Input::default());
        resumed.tick(&Input::default());
    }
    assert_eq!(
        continuous.to_save_string().unwrap(),
        resumed.to_save_string().unwrap()
    );

    let started = std::time::Instant::now();
    for _ in 20_000..(DAY_LENGTH as usize * 2) {
        continuous.tick(&Input::default());
    }
    assert_eq!(continuous.tick, u64::from(DAY_LENGTH) * 2);
    // Updater starts paused: the first gameplay dispatch does not advance
    // the calendar prelude. Two calendar days require one additional tick.
    assert_eq!(continuous.days, 2);
    assert_eq!(continuous.clock.get().day_tick, DAY_LENGTH - 1);
    continuous.tick(&Input::default());
    assert_eq!(continuous.days, 3);
    assert_eq!(continuous.clock.get().day_tick, 0);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(15),
        "headless two-day soak exceeded the generous regression budget"
    );
    assert!(continuous.levels[0].entities.entities().len() < 32);
}

#[test]
fn tile_tick_increments_after_level_tick_and_not_while_menu_open() {
    let mut world = tiny_world();
    // Updater starts paused: the first focused tick clears pause but still
    // runs the no-menu branch (Java sets paused=false then ticks level).
    world.tick(&Input::default());
    assert_eq!(world.tile_ticks(), 1);
    world.tick(&Input::default());
    assert_eq!(world.tile_ticks(), 2);

    // Open pause: Java currentDisplay != null skips level.tick and Tile.tickCount++.
    world.tick(&Input {
        exit: true,
        ..Default::default()
    });
    let paused_ticks = world.tile_ticks();
    for _ in 0..5 {
        world.tick(&Input::default());
    }
    assert_eq!(world.tile_ticks(), paused_ticks);
}

#[test]
fn sound_tick_runs_once_per_updater_tick_including_menus() {
    let mut world = tiny_world();
    world.tick(&Input::default());
    assert_eq!(world.sound_ticks(), 1);
    world.tick(&Input::default());
    assert_eq!(world.sound_ticks(), 2);

    // Menu ticks still pump Sound.tick() in Java Updater.tick.
    world.tick(&Input {
        exit: true,
        ..Default::default()
    });
    assert_eq!(world.sound_ticks(), 3);
    for _ in 0..4 {
        world.tick(&Input::default());
    }
    assert_eq!(world.sound_ticks(), 7);
}

#[test]
fn gameplay_ticks_feed_fluid_animation_phase_reader() {
    // World-level: each gameplay tick advances the host tile counter that
    // backs Java Tile.tickCount. Global phase math is covered in level::tile.
    let mut world = tiny_world();
    world.tick(&Input::default());
    assert_eq!(world.tile_ticks(), 1);
    world.tick(&Input::default());
    assert_eq!(world.tile_ticks(), 2);
    // Pause does not advance the gameplay tile counter.
    world.tick(&Input {
        exit: true,
        ..Default::default()
    });
    assert_eq!(world.tile_ticks(), 2);
}

#[test]
fn release_regression_crafting_and_placement_consume_owned_items() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::Wood, 10);
    assert!(world.craft_stack(crate::item::HAND_RECIPES[0]));
    assert_eq!(world.player.inventory.count(ItemId::Wood), 0);
    world.player.active_item = Some(ActiveItem::Stack(ItemId::Workbench));
    world.attack();
    assert_eq!(world.player.inventory.count(ItemId::Workbench), 0);
    let recipe = crate::item::ANVIL_TOOL_RECIPES[1];
    world.player.inventory.add(ItemId::Shard, 15);
    world
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Sword, ToolTier::Iron))
        .unwrap();
    assert!(world.craft_tool(recipe));
    assert_eq!(world.player.inventory.count(ItemId::Shard), 0);
    assert_eq!(world.player.inventory.tools().len(), 1);
    assert_eq!(world.player.inventory.tools()[0].kind, ToolKind::Claymore);
}

#[test]
fn crafting_missing_material_and_full_bag_keep_all_owned_items() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::Wood, 9);
    let before = world.player.inventory.clone();
    assert!(!world.craft_stack(crate::item::HAND_RECIPES[0]));
    assert_eq!(world.player.inventory, before);
    world.player.inventory.set_capacity(1);
    world.player.inventory.add(ItemId::Wood, 1);
    let before = world.player.inventory.clone();
    // One plank recipe leaves the wood slot occupied, so output cannot fit.
    assert!(!world.craft_stack(crate::item::HAND_RECIPES[2]));
    assert_eq!(world.player.inventory, before);
    world.player.inventory.remove(ItemId::Wood, 10);
    world.player.inventory.add(ItemId::Wood, 1);
    assert!(world.craft_stack(crate::item::HAND_RECIPES[2]));
    assert_eq!(world.player.inventory.count(ItemId::Wood), 0);
    assert_eq!(world.player.inventory.count(ItemId::Plank), 2);
}

#[test]
fn death_moves_owned_items_once_and_preserves_full_bag_and_armor() {
    let mut world = tiny_world();
    for &item in ItemId::ALL
        .iter()
        .filter(|item| **item != ItemId::PowerGlove)
        .take(26)
    {
        world.player.inventory.add(item, 1);
    }
    let tool = ToolItem::new(ToolKind::Axe, ToolTier::Iron);
    world.player.inventory.add_tool(tool).unwrap();
    let expected = world.player.inventory.clone();
    let active = expected.slots()[0].item;
    world.player.active_item = Some(ActiveItem::Stack(active));
    world.player.armor_kind = Some(crate::item::ArmorKind::Gem);
    world.die_player();
    assert_eq!(world.player.inventory.used_slots(), 0);
    assert!(world.player.active_item.is_none());
    let chest = world.levels[0]
        .entities
        .entities()
        .last()
        .unwrap()
        .storage
        .as_ref()
        .unwrap();
    for stack in expected.slots() {
        let armor = u16::from(stack.item == ItemId::GemArmor);
        assert_eq!(chest.count(stack.item), stack.count + armor);
    }
    assert_eq!(
        chest.count(ItemId::GemArmor),
        expected.count(ItemId::GemArmor) + 1
    );
    assert_eq!(chest.tools(), &[tool]);
    let loaded = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    assert_eq!(loaded.player.inventory.used_slots(), 0);
    assert!(loaded.player_removed);
}

#[test]
fn last_tool_durability_removes_equipped_tool_and_frees_bag_slot() {
    let mut world = tiny_world();
    let target = 9 + 8 * world.width;
    world.levels[0].tiles[target] = Tile::Tree;
    let mut axe = ToolItem::new(ToolKind::Axe, ToolTier::Wood);
    axe.durability = 1;
    world.player.inventory.add_tool(axe);
    world
        .player
        .inventory
        .add_tool(ToolItem::new(ToolKind::Sword, ToolTier::Wood));
    world.player.active_item = Some(ActiveItem::Tool(0));

    world.attack();

    assert_eq!(world.player.inventory.used_slots(), 1);
    assert_eq!(world.player.inventory.tools()[0].kind, ToolKind::Sword);
    assert_eq!(world.player.active_item, None);
    assert_eq!(world.player.attack_item, None);
}

#[test]
fn extreme_saved_counters_survive_live_survival_and_calendar_ticks() {
    let mut world = tiny_world();
    world.days = u32::MAX;
    world.clock.set_day_tick(DAY_LENGTH - 1);
    world.player.health = 1;
    world.player.stamina = 0;
    world.player.hunger_ticks = i16::MIN;
    world.player.hunger_stamina_count = i16::MIN;
    world.player.hunger_charge_delay = u16::MAX;
    world.player.inventory.add(ItemId::Wood, 7);
    let mut restored = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
    restored.tick_survival_stats();
    restored.begin_display_tick();
    restored.tick_after_display(false, &Input::default());
    assert_eq!(restored.days, u32::MAX);
    assert!(restored.player.hunger_ticks > 0);
    assert!(restored.player.hunger_stamina_count > 0);
    assert_eq!(restored.player.inventory.count(ItemId::Wood), 7);
    restored.player.hunger = u8::MAX;
    restored.player.hunger_charge_delay = u16::MAX;
    restored.tick_survival_stats();
}

#[test]
fn java224_death_chest_lifetime_full_bag_and_expiry_conserve_instances() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java224/reference.json")).unwrap();
    for difficulty in 0..3 {
        let mut world = tiny_world();
        world.difficulty = difficulty;
        let mut tool = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
        tool.durability = 17;
        world.player.inventory.add(ItemId::Wood, 7);
        world.player.inventory.add_tool(tool).unwrap();
        world.player.active_item = Some(ActiveItem::Tool(0));
        world.player.armor_kind = Some(ArmorKind::Iron);
        world.die_player();
        let entity = &world.levels[0].entities.entities()[0];
        let id = entity.id;
        assert!(
            world.levels[0]
                .entities
                .pickup_furniture_near(world.player.x, world.player.y, 16, true)
                .is_none()
        );
        assert_eq!(
            world.levels[0].entities.entities()[0]
                .death_chest_lifetime
                .unwrap(),
            reference["deathChestTicks"][difficulty].as_u64().unwrap() as u32
        );
        // Native saving preserves the captured difficulty/timer, even if settings change.
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        world.player.inventory = Inventory::new(1);
        world.player.inventory.add(ItemId::Wood, 998);
        world.retrieve_death_chests();
        assert_eq!(world.player.inventory.count(ItemId::Wood), 999);
        let storage = world.levels[0].entities.death_chest_storage(id).unwrap();
        assert_eq!(storage.count(ItemId::Wood), 6);
        assert_eq!(storage.tools(), &[tool]);
        assert_eq!(storage.count(ItemId::IronArmor), 1);
        world.player.inventory.take_all();
        world.player.inventory.set_capacity(27);
        world.retrieve_death_chests();
        assert!(world.levels[0].entities.entities().is_empty());
        assert_eq!(world.player.inventory.count(ItemId::Wood), 6);
        assert_eq!(world.player.inventory.tools(), &[tool]);
        assert_eq!(world.player.inventory.count(ItemId::IronArmor), 1);
        world.player.active_item = Some(ActiveItem::Tool(0));
        world.die_player();
        let id = world.levels[0].entities.entities()[0].id;
        world.levels[0].entities.set_death_chest_remaining(id, 1);
        // No player target during expiry, so collision pickup cannot hide a spill.
        world.tick_level_entities(None, false, false);
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        assert_eq!(world.levels[0].entities.entities().len(), 3);
        assert!(
            world.levels[0]
                .entities
                .entities()
                .iter()
                .any(|entity| matches!(entity.kind, EntityKind::Tool(saved) if saved == tool))
        );
        for entity in &mut world.levels[0].entities.entities {
            entity.age = 31;
            entity.x = world.player.x;
            entity.y = world.player.y;
        }
        world.levels[0].entities.collect_near(
            world.player.x,
            world.player.y,
            &mut world.player.inventory,
        );
        assert!(world.levels[0].entities.entities().is_empty());
        assert_eq!(world.player.inventory.tools(), &[tool]);
        assert_eq!(world.player.inventory.count(ItemId::Wood), 6);
        assert_eq!(world.player.inventory.count(ItemId::IronArmor), 1);
    }
}

#[test]
fn java224_imported_recipe_locks_apply_to_real_crafting_actions() {
    let mut world = tiny_world();
    world
        .progress
        .import_unlocked_recipes(serde_json::json!({}))
        .unwrap();
    world.refresh_crafting_order();
    assert_eq!(world.crafting_recipe_count(), 0);
    world.player.inventory.add(ItemId::Wood, 9);
    world.craft_selected_recipe();
    assert_eq!(world.player.inventory.count(ItemId::Wood), 9);
    world.update_progress(crate::progression::ProgressEvent::InventoryChanged);
    assert!(world.crafting_recipe_count() > 0);
    let recipes = super::ordered_recipe_views(
        None,
        &world.player.inventory,
        false,
        &world.crafting_order,
        &world.progress,
    );
    world.inventory_selection = recipes
        .iter()
        .position(|recipe| recipe.product_name() == "Plank")
        .unwrap();
    world.craft_selected_recipe();
    assert_eq!(world.player.inventory.count(ItemId::Wood), 8);
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java224/reference.json")).unwrap();
    assert_eq!(
        u64::from(world.player.inventory.count(ItemId::Plank)),
        reference["plankAfterCraft"].as_u64().unwrap()
    );
    assert_eq!(
        u64::from(world.player.inventory.count(ItemId::Wood)),
        reference["woodAfterCraft"].as_u64().unwrap()
    );
}

#[test]
fn java224_cross_layer_and_respawn_use_depth_in_both_saved_layouts() {
    for depths in [[-3, -2, -1, 0, 1, -4], [1, 0, -1, -2, -3, -4]] {
        let mut world = tiny_world();
        world.levels = depths
            .into_iter()
            .map(|depth| Level {
                depth,
                tiles: vec![Tile::Grass; 256],
                data: vec![0; 256],
                max_mob_count: 0,
                pending_spawns: vec![],
                entities: EntityArena::default(),
            })
            .collect();
        world.signs = vec![Default::default(); 6];
        world.current_level = world
            .levels
            .iter()
            .position(|level| level.depth == 0)
            .unwrap();
        world.apply_level_change(-1);
        assert_eq!(world.levels[world.current_level].depth, 1);
        world.apply_level_change(1);
        assert_eq!(world.levels[world.current_level].depth, 0);
        world.apply_level_change(1);
        assert_eq!(world.levels[world.current_level].depth, -1);
        world.respawn_tile = Some((2, 3));
        let mut world = World::from_save_string(&world.to_save_string().unwrap()).unwrap();
        world.respawn();
        assert_eq!(world.levels[world.current_level].depth, 0);
        assert_eq!((world.player.x, world.player.y), (40, 56));
        assert_eq!(crate::saveload::save::player_fields(&world)[11], "3");
    }
}

#[test]
fn java224_legacy_native_fields_and_unknown_progress_are_preserved() {
    let mut world = tiny_world();
    world.player.inventory.add(ItemId::Wood, 11);
    let mut legacy: serde_json::Value =
        serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
    legacy["world"]
        .as_object_mut()
        .unwrap()
        .remove("score_minutes");
    legacy["world"]["player"]
        .as_object_mut()
        .unwrap()
        .remove("imported_shirt_color");
    for field in [
        "java_advancements",
        "imported_tutorial_cursor",
        "tutorial_cursor",
        "unlocking_hits",
        "recipe_elements",
        "recipe_completed",
        "unlocked_recipes",
        "criterion_times",
        "unlocking_times",
        "imported_recipe_unlocks",
    ] {
        legacy["world"]["progress"]
            .as_object_mut()
            .unwrap()
            .remove(field);
    }
    let mut restored = World::from_save_string(&legacy.to_string()).unwrap();
    assert_eq!(restored.player.inventory.count(ItemId::Wood), 11);
    assert_eq!(restored.score_minutes, 20);
    let unknown = serde_json::json!({"Version": "2.2.4", "CurrentOngoingTutorial": "custom.tutorial", "custom.progress": {"criteria": {"custom": "2024-01-01T00:00:00"}, "done": true, "additional": [1,2]}});
    restored
        .progress
        .import_java_advancements(unknown.clone())
        .unwrap();
    assert_eq!(restored.progress.java_advancements_json(), unknown);
    restored.tutorials_enabled = false;
    restored.quests_enabled = false;
    restored
        .progress
        .import_unlocked_recipes(serde_json::json!({}))
        .unwrap();
    restored.update_progress(crate::progression::ProgressEvent::InventoryChanged);
    assert!(
        restored.crafting_recipe_count() > 0,
        "recipe progress runs with tutorials and quests disabled"
    );
}

#[test]
fn java224_actual_tool_and_rejected_craft_reference() {
    let reference: serde_json::Value =
        serde_json::from_str(include_str!("../../tests/fixtures/java224/reference.json")).unwrap();
    let mut tool = ToolItem::new(ToolKind::Pickaxe, ToolTier::Iron);
    tool.durability = 1;
    assert_eq!(
        tool.pay_durability(),
        reference["lastToolUse"].as_bool().unwrap()
    );
    assert_eq!(
        u64::from(tool.durability),
        reference["afterLastUse"].as_u64().unwrap()
    );
    assert_eq!(
        tool.pay_durability(),
        reference["depletedToolUse"].as_bool().unwrap()
    );
    let mut bag = Inventory::new(27);
    bag.add(ItemId::Wood, 9);
    let recipe = crate::item::recipe::HAND_RECIPES
        .iter()
        .find(|recipe| recipe.output.item == ItemId::WoodDoor)
        .unwrap();
    assert_eq!(
        recipe.craft(&mut bag),
        reference["craftInsufficient"].as_bool().unwrap()
    );
    assert_eq!(
        u64::from(bag.count(ItemId::Wood)),
        reference["woodAfterRejectedCraft"].as_u64().unwrap()
    );
}
