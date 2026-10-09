//! Headless adapters that expose live compatibility code to differential tools.

mod display_stack;
pub use display_stack::display_stack_snapshots;
mod focus_update;
pub use focus_update::focus_update_snapshots;
mod core_prelude;
pub use core_prelude::core_prelude_snapshots;
mod notifications;
pub use notifications::notification_snapshots;
mod game;
mod renderer;
pub use game::game_snapshots;
pub use renderer::renderer_snapshots;

use minicraft_common::parity::{
    AudioScene, AudioSound, CombatTool, CraftingCost, CraftingCount, CraftingProduct,
    CraftingStation, MenuAction, Observation, PARITY_SCHEMA, ParityCommand, ParitySnapshot,
    ParityTrace, RenderScene,
};

use crate::core::game_loop::FixedTickAccumulator;
use crate::world::random::JavaRandom;
use crate::{
    app::{
        immediate_achievement_sounds, immediate_book_sounds, immediate_control_guide_sounds,
        immediate_display_sounds, immediate_key_binding_sounds, immediate_new_world_sounds,
        immediate_options_sounds, immediate_select_and_close_sounds, immediate_world_popup_sounds,
        title_menu_parity_tick,
    },
    io::Input,
    world::{GameMode, PlayOptions, World, WorldAction, WorldSpec},
};

/// Executes a trace against the Rust Java-compatible random implementation.
pub fn java_random_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("java_random")?;
    let mut random = JavaRandom::new(trace.seed);
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        let observation = match *command {
            ParityCommand::GameWorld { .. }
            | ParityCommand::GameTitle
            | ParityCommand::GameMenu { .. } => {
                return Err("game command in random trace".into());
            }
            ParityCommand::RenderLayers { .. } => {
                return Err("renderer command in random trace".into());
            }
            ParityCommand::NotificationAdd { .. }
            | ParityCommand::NotificationTick { .. }
            | ParityCommand::NotificationRender { .. }
            | ParityCommand::NotificationClear => {
                return Err("notification command in java_random trace".into());
            }
            ParityCommand::CoreSetup { .. }
            | ParityCommand::CoreTick { .. }
            | ParityCommand::CoreSave => {
                return Err("core prelude command in java_random trace".to_owned());
            }
            ParityCommand::NextInt { bound } => {
                if bound <= 0 {
                    return Err(format!("next_int bound must be positive, got {bound}"));
                }
                Observation::Int {
                    value: random.next_int(bound),
                }
            }
            ParityCommand::NextBool => Observation::Bool {
                value: random.next_bool(),
            },
            ParityCommand::NextFloat => Observation::FloatBits {
                value: (random.next_float() as f32).to_bits(),
            },
            ParityCommand::AdvanceNanos { .. } | ParityCommand::FocusFrame { .. } => {
                return Err("fixed-step command in java_random trace".to_owned());
            }
            ParityCommand::MenuInput { .. } | ParityCommand::DisplayInput { .. } => {
                return Err("menu command in java_random trace".to_owned());
            }
            ParityCommand::AudioSetup { .. }
            | ParityCommand::AudioPlay { .. }
            | ParityCommand::AudioMix { .. } => {
                return Err("audio command in java_random trace".to_owned());
            }
            ParityCommand::PlayerInput { .. }
            | ParityCommand::SetPlayerStats { .. }
            | ParityCommand::SetObstacle { .. }
            | ParityCommand::CombatSetup { .. }
            | ParityCommand::CombatAttack
            | ParityCommand::CraftingSetup { .. }
            | ParityCommand::CraftingSelect { .. }
            | ParityCommand::CraftingCraft
            | ParityCommand::RenderScene { .. } => {
                return Err("player command in java_random trace".to_owned());
            }
        };
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation,
        });
    }
    Ok(snapshots)
}

/// Runs the copied PCM clips through the Java 2.2.4 pointer/compression mixer.
pub fn audio_pcm_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("audio_pcm")?;
    let mut mixer = crate::audio::JavaPcmMixer::new()?;
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        let samples = match *command {
            ParityCommand::AudioPlay { sound } => {
                mixer.play(sound_effect(sound));
                Vec::new()
            }
            ParityCommand::AudioMix { samples } => mixer.mix(
                usize::try_from(samples)
                    .map_err(|_| format!("audio sample count does not fit usize: {samples}"))?,
            ),
            _ => return Err("non-audio command in audio_pcm trace".to_owned()),
        };
        let offsets = mixer
            .offsets()
            .into_iter()
            .map(|offset| {
                u32::try_from(offset)
                    .map_err(|_| format!("audio pointer offset does not fit u32: {offset}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        let minimum = samples.iter().copied().min().unwrap_or(0);
        let maximum = samples.iter().copied().max().unwrap_or(0);
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation: Observation::AudioPcm {
                active_pointers: u32::try_from(offsets.len())
                    .map_err(|_| "audio pointer count does not fit u32".to_owned())?,
                offsets,
                sample_count: u32::try_from(samples.len())
                    .map_err(|_| "mixed sample count does not fit u32".to_owned())?,
                pcm_hash: pcm_hash(&samples),
                first_samples: samples.iter().take(16).copied().collect(),
                minimum,
                maximum,
            },
        });
    }
    Ok(snapshots)
}

fn sound_effect(sound: AudioSound) -> crate::audio::SoundEffect {
    match sound {
        AudioSound::BossDeath => crate::audio::SoundEffect::BossDeath,
        AudioSound::Confirm => crate::audio::SoundEffect::Confirm,
        AudioSound::Craft => crate::audio::SoundEffect::Craft,
        AudioSound::Death => crate::audio::SoundEffect::Death,
        AudioSound::Explode => crate::audio::SoundEffect::Explode,
        AudioSound::Fuse => crate::audio::SoundEffect::Fuse,
        AudioSound::MonsterHurt => crate::audio::SoundEffect::MonsterHurt,
        AudioSound::Pickup => crate::audio::SoundEffect::Pickup,
        AudioSound::PlayerHurt => crate::audio::SoundEffect::PlayerHurt,
        AudioSound::Select => crate::audio::SoundEffect::Select,
    }
}

fn pcm_hash(samples: &[i16]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for sample in samples {
        for byte in sample.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Renders deterministic frames through the live renderer and records a raw-RGB hash.
pub fn render_frame_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("render_frame")?;
    let output_directory = std::env::var_os("MINICRAFT_PARITY_RENDER_OUTPUT")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("target/parity/render-frame/rust"));
    std::fs::create_dir_all(&output_directory).map_err(|error| {
        format!(
            "cannot create render parity directory {}: {error}",
            output_directory.display()
        )
    })?;
    let assets = crate::assets::Assets::load(&[])?;
    let localization = crate::resource::localization::Localization::load("en-us", &[]);
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    let mut render_world: Option<World> = None;
    for (tick, command) in trace.commands.iter().enumerate() {
        let ParityCommand::RenderScene { scene } = *command else {
            return Err("non-render command in render_frame trace".to_owned());
        };
        let mut screen = crate::gfx::Screen::new();
        match scene {
            RenderScene::NotificationsMultiple
            | RenderScene::NotificationsWrapped
            | RenderScene::NotificationsExpired
            | RenderScene::StatusSleeping
            | RenderScene::StatusSaving => {
                notifications::render_fixture(&mut screen, &assets, &localization, scene);
            }
            RenderScene::ResourcePacks
            | RenderScene::ResourcePacksReordered
            | RenderScene::ResourcePacksToggled
            | RenderScene::ResourcePacksDefault
            | RenderScene::ResourcePacksLogo
            | RenderScene::ResourcePacksInfo
            | RenderScene::ResourcePacksScrolled => {
                crate::app::render_resource_pack_parity(&mut screen, &assets, &localization, scene);
            }
            RenderScene::Title => {
                screen.clear(0);
                crate::app::render_title(&mut screen, &assets, &localization, 0, 0);
            }
            RenderScene::Pause => {
                screen.clear(0);
                let world = parity_render_world(&mut render_world, trace.seed);
                world.prepare_pause_audio_parity();
                crate::world::render_pause(&mut screen, &assets, &localization, world);
            }
            RenderScene::Inventory => {
                screen.clear(0);
                let world = parity_render_world(&mut render_world, trace.seed);
                world.populate_inventory_preview();
                crate::world::render_inventory_parity(&mut screen, &assets, world);
            }
            RenderScene::Crafting => {
                screen.clear(0);
                let world = parity_render_world(&mut render_world, trace.seed);
                world.populate_personal_crafting_preview();
                crate::world::render_inventory_parity(&mut screen, &assets, world);
            }
            RenderScene::Hud => {
                screen.clear(0);
                let world = parity_render_world(&mut render_world, trace.seed);
                world.populate_hud_parity_preview();
                crate::world::render_hud_parity(&mut screen, &assets, world);
            }
            RenderScene::Focus => {
                screen.clear(0);
                crate::app::render_focus_nagger(&mut screen, &assets, 0);
            }
            RenderScene::ExitConfirm => {
                screen.clear(0);
                let world = parity_render_world(&mut render_world, trace.seed);
                world.populate_exit_confirm_parity_preview();
                crate::world::render_pause(&mut screen, &assets, &localization, world);
            }
            RenderScene::Quests
            | RenderScene::QuestsCompleted
            | RenderScene::QuestSeriesInfo
            | RenderScene::QuestSeriesViewer
            | RenderScene::QuestDetail
            | RenderScene::QuestTreeLinked
            | RenderScene::QuestTreeScrolled
            | RenderScene::QuestDetailCompleted => {
                let world = parity_render_world(&mut render_world, trace.seed);
                world.prepare_quests_audio_parity();
                if matches!(
                    scene,
                    RenderScene::QuestTreeLinked
                        | RenderScene::QuestTreeScrolled
                        | RenderScene::QuestDetailCompleted
                ) {
                    world.complete_hoe_quest_for_parity();
                }
                if scene == RenderScene::QuestsCompleted {
                    world.tick(&Input {
                        right_pressed: true,
                        ..Input::default()
                    });
                } else if scene != RenderScene::Quests {
                    world.tick(&Input {
                        select: true,
                        ..Input::default()
                    });
                    world.prepare_quest_layout(&localization);
                    if scene != RenderScene::QuestSeriesInfo {
                        world.tick(&Input {
                            select: true,
                            ..Input::default()
                        });
                        if matches!(
                            scene,
                            RenderScene::QuestTreeLinked | RenderScene::QuestTreeScrolled
                        ) {
                            world.tick(&Input {
                                down_pressed: true,
                                ..Input::default()
                            });
                        }
                        if scene == RenderScene::QuestTreeScrolled {
                            for _ in 0..4 {
                                world.tick(&Input {
                                    shift: true,
                                    right_pressed: true,
                                    ..Input::default()
                                });
                                world.tick(&Input {
                                    shift: true,
                                    down_pressed: true,
                                    ..Input::default()
                                });
                            }
                        }
                        if matches!(
                            scene,
                            RenderScene::QuestDetail | RenderScene::QuestDetailCompleted
                        ) {
                            world.tick(&Input {
                                select: true,
                                ..Input::default()
                            });
                        }
                    }
                }
                world.settle_pause_preview();
                crate::world::render_pause(&mut screen, &assets, &localization, world);
            }
        }
        let scene_name = match scene {
            RenderScene::NotificationsMultiple => "notifications_multiple",
            RenderScene::NotificationsWrapped => "notifications_wrapped",
            RenderScene::NotificationsExpired => "notifications_expired",
            RenderScene::StatusSleeping => "status_sleeping",
            RenderScene::StatusSaving => "status_saving",
            RenderScene::Title => "title",
            RenderScene::Pause => "pause",
            RenderScene::Inventory => "inventory",
            RenderScene::Crafting => "crafting",
            RenderScene::Hud => "hud",
            RenderScene::Focus => "focus",
            RenderScene::ExitConfirm => "exit_confirm",
            RenderScene::Quests => "quests",
            RenderScene::QuestsCompleted => "quests_completed",
            RenderScene::QuestSeriesInfo => "quest_series_info",
            RenderScene::QuestSeriesViewer => "quest_series_viewer",
            RenderScene::QuestDetail => "quest_detail",
            RenderScene::QuestTreeLinked => "quest_tree_linked",
            RenderScene::QuestTreeScrolled => "quest_tree_scrolled",
            RenderScene::QuestDetailCompleted => "quest_detail_completed",
            RenderScene::ResourcePacks => "resource_packs",
            RenderScene::ResourcePacksReordered => "resource_packs_reordered",
            RenderScene::ResourcePacksToggled => "resource_packs_toggled",
            RenderScene::ResourcePacksDefault => "resource_packs_default",
            RenderScene::ResourcePacksLogo => "resource_packs_logo",
            RenderScene::ResourcePacksInfo => "resource_packs_info",
            RenderScene::ResourcePacksScrolled => "resource_packs_scrolled",
        };
        screen.save_png(&output_directory.join(format!("{scene_name}.png")))?;
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation: Observation::RenderFrame {
                scene: scene_name.to_owned(),
                width: crate::gfx::WIDTH as u32,
                height: crate::gfx::HEIGHT as u32,
                rgb_hash: rgb_hash(screen.pixels()),
            },
        });
    }
    Ok(snapshots)
}

fn parity_render_world(slot: &mut Option<World>, seed: i64) -> &mut World {
    slot.get_or_insert_with(|| {
        World::new_with_play_options(
            seed,
            WorldSpec::default(),
            PlayOptions {
                difficulty: 1,
                mode: GameMode::Survival,
                score_minutes: 20,
                tutorials: false,
                quests: false,
                show_quests: true,
                custom_skin: false,
            },
        )
    })
}

fn rgb_hash(pixels: &[u32]) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for color in pixels {
        for value in [(color >> 16) as u8, (color >> 8) as u8, *color as u8] {
            hash ^= u64::from(value);
            hash = hash.wrapping_mul(0x100_0000_01b3);
        }
    }
    format!("{hash:016x}")
}

/// Drives the live inventory, recipe ordering, Have/Cost, and crafting paths.
pub fn inventory_crafting_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("inventory_crafting")?;
    let mut world = World::new_with_play_options(
        trace.seed,
        WorldSpec::default(),
        PlayOptions {
            difficulty: 1,
            mode: GameMode::Survival,
            score_minutes: 20,
            tutorials: false,
            quests: false,
            show_quests: true,
            custom_skin: false,
        },
    );
    let mut ready = false;
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        let mut frame_sounds = Vec::new();
        match *command {
            ParityCommand::CraftingSetup {
                station,
                wood,
                coal,
                stone,
                string,
            } => {
                world.prepare_crafting_parity(
                    station == CraftingStation::Workbench,
                    wood,
                    coal,
                    stone,
                    string,
                );
                ready = true;
            }
            ParityCommand::CraftingSelect { product } if ready => {
                let product = match product {
                    CraftingProduct::Workbench => "Workbench",
                    CraftingProduct::Plank => "Plank",
                    CraftingProduct::WoodAxe => "Wood Axe",
                };
                world.select_crafting_product_for_parity(product)?;
            }
            ParityCommand::CraftingCraft if ready => {
                let input = Input {
                    select: true,
                    ..Input::default()
                };
                frame_sounds = world.immediate_menu_sounds(&input);
                world.tick(&input);
            }
            ParityCommand::CraftingSelect { .. } | ParityCommand::CraftingCraft => {
                return Err("crafting command requires a preceding crafting_setup".to_owned());
            }
            _ => {
                return Err("non-crafting command in inventory_crafting trace".to_owned());
            }
        }
        let state = world.crafting_state_for_parity();
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation: Observation::Crafting {
                capacity: u32::try_from(state.capacity)
                    .map_err(|_| "inventory capacity does not fit u32".to_owned())?,
                used_slots: u32::try_from(state.used_slots)
                    .map_err(|_| "inventory slot count does not fit u32".to_owned())?,
                selection: u32::try_from(state.selection)
                    .map_err(|_| "crafting selection does not fit u32".to_owned())?,
                items: state
                    .items
                    .into_iter()
                    .map(|(item, count)| CraftingCount { item, count })
                    .collect(),
                tools: state
                    .tools
                    .into_iter()
                    .map(|(item, count)| CraftingCount { item, count })
                    .collect(),
                recipe_order: state.recipe_order,
                craftable: state.craftable,
                selected_have: u32::try_from(state.selected_have)
                    .map_err(|_| "selected recipe count does not fit u32".to_owned())?,
                selected_costs: state
                    .selected_costs
                    .into_iter()
                    .map(|(item, have, need)| CraftingCost {
                        item,
                        have: u32::from(have),
                        need: u32::from(need),
                    })
                    .collect(),
                sounds: sound_names(
                    frame_sounds
                        .into_iter()
                        .chain(world.take_sound_events())
                        .collect(),
                ),
            },
        });
    }
    Ok(snapshots)
}

/// Records Java display sounds at the input-frame boundary separately from
/// fixed-world-tick events so menu feedback cannot silently regress by one tick.
pub fn audio_event_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("audio_event")?;
    let mut world = parity_world(trace.seed);
    let mut scene = None;
    let mut title_selection = 0_usize;
    let mut title_ticks = 0_u64;
    let mut title_screen = "title";
    let mut pack_info = false;
    let mut packs: Vec<_> = ["Alpha", "Beta", "Gamma"]
        .into_iter()
        .map(|name| crate::resource::pack::ResourcePack::parity_fixture(name, name, None))
        .collect();
    let mut snapshots = Vec::with_capacity(trace.commands.len());

    for (tick, command) in trace.commands.iter().enumerate() {
        let mut frame_sounds = Vec::new();
        let mut tick_sounds = Vec::new();
        let mut outcome = "ready";

        match *command {
            ParityCommand::AudioSetup { scene: next } => {
                pack_info = false;
                scene = Some(next);
                match next {
                    AudioScene::Title
                    | AudioScene::NewWorld
                    | AudioScene::WorldSelect
                    | AudioScene::WorldPopupValid
                    | AudioScene::WorldPopupInvalid
                    | AudioScene::Options
                    | AudioScene::Languages
                    | AudioScene::Skins
                    | AudioScene::Achievements
                    | AudioScene::Controls
                    | AudioScene::ControlGuide
                    | AudioScene::Book
                    | AudioScene::ResourcePacks => {
                        title_selection = 0;
                        title_ticks = 0;
                        title_screen = audio_scene_name(next);
                    }
                    AudioScene::Pause => {
                        world.prepare_pause_audio_parity();
                    }
                    AudioScene::Quests => {
                        world.prepare_quests_audio_parity();
                    }
                    AudioScene::Inventory => {
                        world.prepare_inventory_audio_parity();
                    }
                    AudioScene::Crafting => {
                        world.prepare_crafting_parity(false, 10, 0, 0, 0);
                    }
                }
            }
            ParityCommand::MenuInput { action } => {
                let active = scene.ok_or_else(|| {
                    "audio_event menu_input requires a preceding audio_setup".to_owned()
                })?;
                let input = menu_input(action);
                match active {
                    AudioScene::Title => {
                        frame_sounds = immediate_display_sounds(&input, false);
                        let (next_screen, next_outcome) =
                            title_menu_parity_tick(&mut title_selection, &mut title_ticks, &input);
                        title_screen = next_screen;
                        outcome = next_outcome;
                    }
                    AudioScene::NewWorld => {
                        frame_sounds = immediate_new_world_sounds(title_selection, &input);
                        navigate_audio_selection(&mut title_selection, &input, 10);
                        if input.exit {
                            title_screen = "title";
                            outcome = "close";
                        } else if input.select && title_selection == 3 {
                            title_screen = "loading";
                            outcome = "create";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::WorldSelect => {
                        frame_sounds = immediate_display_sounds(&input, true);
                        navigate_audio_selection(&mut title_selection, &input, 3);
                        if input.exit {
                            title_screen = "play";
                            outcome = "close";
                        } else if input.select {
                            title_screen = "loading";
                            outcome = "load";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::WorldPopupValid | AudioScene::WorldPopupInvalid => {
                        let valid = active == AudioScene::WorldPopupValid;
                        frame_sounds = immediate_world_popup_sounds(valid, &input);
                        if input.exit || (valid && input.select) {
                            title_screen = "world_select";
                            outcome = "close";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::Options => {
                        frame_sounds = immediate_options_sounds(title_selection, &input);
                        navigate_audio_selection(&mut title_selection, &input, 8);
                        if input.exit {
                            title_screen = "title";
                            outcome = "close";
                        } else if input.select && title_selection >= 4 {
                            title_screen = match title_selection {
                                4 => "controls",
                                5 => "control_guide",
                                6 => "languages",
                                _ => "resource_packs",
                            };
                            outcome = "open";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::Languages | AudioScene::Skins => {
                        frame_sounds = immediate_select_and_close_sounds(&input);
                        navigate_audio_selection(
                            &mut title_selection,
                            &input,
                            if active == AudioScene::Skins { 4 } else { 3 },
                        );
                        if input.exit || input.select {
                            title_screen = if active == AudioScene::Languages {
                                "options"
                            } else {
                                "title"
                            };
                            outcome = "close";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::Achievements => {
                        frame_sounds = immediate_achievement_sounds(&input);
                        navigate_audio_selection(&mut title_selection, &input, 17);
                        if input.exit {
                            title_screen = "title";
                            outcome = "close";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::Controls => {
                        frame_sounds = immediate_key_binding_sounds(false, &input);
                        navigate_audio_selection(&mut title_selection, &input, 10);
                        if input.exit {
                            title_screen = "options";
                            outcome = "close";
                        } else if input.select {
                            title_screen = "key_capture";
                            outcome = "open";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::ControlGuide => {
                        frame_sounds = immediate_control_guide_sounds(&input);
                        navigate_audio_selection(&mut title_selection, &input, 23);
                        if input.left_pressed || input.right_pressed {
                            title_selection = 0;
                        }
                        if input.exit {
                            title_screen = "options";
                            outcome = "close";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::Book => {
                        frame_sounds = immediate_book_sounds(&input);
                        if input.left_pressed || input.up_pressed {
                            title_selection = title_selection.saturating_sub(1);
                        }
                        if input.right_pressed || input.down_pressed || input.select {
                            title_selection += 1;
                        }
                        if input.exit {
                            title_screen = "help";
                            outcome = "close";
                        } else {
                            outcome = "none";
                        }
                    }
                    AudioScene::ResourcePacks => {
                        (title_screen, outcome, frame_sounds) =
                            crate::app::resource_pack_menu_parity_step(
                                &mut title_selection,
                                &input,
                                &mut packs,
                                &mut pack_info,
                            );
                    }
                    AudioScene::Pause
                    | AudioScene::Quests
                    | AudioScene::Inventory
                    | AudioScene::Crafting => {
                        frame_sounds = world.immediate_menu_sounds(&input);
                        let before = world.audio_menu_state_for_parity().0;
                        let action = world.tick(&input);
                        tick_sounds = world.take_sound_events();
                        let after = world.audio_menu_state_for_parity().0;
                        outcome = match action {
                            WorldAction::SaveGame => "save",
                            WorldAction::ReturnToTitle => "title",
                            WorldAction::QuitWithoutSaving => "quit",
                            WorldAction::None if before != "gameplay" && after == "gameplay" => {
                                "close"
                            }
                            WorldAction::None if input.exit && before != after => "close",
                            WorldAction::None
                                if input.select
                                    && before != after
                                    && before.starts_with("quest")
                                    && after.starts_with("quest") =>
                            {
                                "open"
                            }
                            WorldAction::None => "none",
                        };
                    }
                }
            }
            _ => return Err("non-audio command in audio_event trace".to_owned()),
        }

        let active =
            scene.ok_or_else(|| "audio_event trace must begin with audio_setup".to_owned())?;
        let (screen, selection) = match active {
            AudioScene::Title
            | AudioScene::NewWorld
            | AudioScene::WorldSelect
            | AudioScene::WorldPopupValid
            | AudioScene::WorldPopupInvalid
            | AudioScene::Options
            | AudioScene::Languages
            | AudioScene::Skins
            | AudioScene::Achievements
            | AudioScene::Controls
            | AudioScene::ControlGuide
            | AudioScene::Book
            | AudioScene::ResourcePacks => (title_screen, title_selection),
            AudioScene::Pause
            | AudioScene::Quests
            | AudioScene::Inventory
            | AudioScene::Crafting => world.audio_menu_state_for_parity(),
        };
        snapshots.push(audio_snapshot(
            trace,
            tick,
            screen,
            selection,
            sound_names(frame_sounds),
            sound_names(tick_sounds),
            outcome,
        )?);
    }
    Ok(snapshots)
}

fn audio_scene_name(scene: AudioScene) -> &'static str {
    match scene {
        AudioScene::Title => "title",
        AudioScene::NewWorld => "new_world",
        AudioScene::WorldSelect => "world_select",
        AudioScene::WorldPopupValid => "world_popup_valid",
        AudioScene::WorldPopupInvalid => "world_popup_invalid",
        AudioScene::Options => "options",
        AudioScene::Languages => "languages",
        AudioScene::Skins => "skins",
        AudioScene::Achievements => "achievements",
        AudioScene::Controls => "controls",
        AudioScene::ControlGuide => "control_guide",
        AudioScene::Book => "book",
        AudioScene::ResourcePacks => "resource_packs",
        AudioScene::Pause => "pause",
        AudioScene::Quests => "quests",
        AudioScene::Inventory => "inventory",
        AudioScene::Crafting => "crafting",
    }
}

fn navigate_audio_selection(selection: &mut usize, input: &Input, count: usize) {
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(count - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % count;
    }
}

fn parity_world(seed: i64) -> World {
    World::new_with_play_options(
        seed,
        WorldSpec::default(),
        PlayOptions {
            difficulty: 1,
            mode: GameMode::Survival,
            score_minutes: 20,
            tutorials: false,
            quests: false,
            show_quests: true,
            custom_skin: false,
        },
    )
}

fn audio_snapshot(
    trace: &ParityTrace,
    tick: usize,
    scene: &str,
    selection: usize,
    frame_sounds: Vec<String>,
    tick_sounds: Vec<String>,
    outcome: &str,
) -> Result<ParitySnapshot, String> {
    Ok(ParitySnapshot {
        schema: PARITY_SCHEMA,
        system: trace.system.clone(),
        tick: tick as u64,
        observation: Observation::AudioEvent {
            scene: scene.to_owned(),
            selection: u32::try_from(selection)
                .map_err(|_| format!("audio selection does not fit u32: {selection}"))?,
            frame_sounds,
            tick_sounds,
            outcome: outcome.to_owned(),
        },
    })
}

/// Drives the live attack implementation in a controlled tree fixture.
pub fn combat_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("combat")?;
    let mut world = World::new_with_play_options(
        trace.seed,
        WorldSpec::default(),
        PlayOptions {
            difficulty: 1,
            mode: GameMode::Survival,
            score_minutes: 20,
            tutorials: false,
            quests: false,
            show_quests: true,
            custom_skin: false,
        },
    );
    let mut ready = false;
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        match *command {
            ParityCommand::CombatSetup { tool } => {
                let tool = match tool {
                    CombatTool::None => None,
                    CombatTool::WoodAxe => Some(crate::item::ToolKind::Axe),
                    CombatTool::WoodSword => Some(crate::item::ToolKind::Sword),
                };
                world.prepare_combat_parity(trace.seed, tool);
                ready = true;
            }
            ParityCommand::CombatAttack if ready => world.combat_attack_for_parity(),
            ParityCommand::CombatAttack => {
                return Err("combat_attack requires a preceding combat_setup".to_owned());
            }
            _ => return Err("non-combat command in combat trace".to_owned()),
        }
        let (
            tile,
            tile_damage,
            stamina,
            attack_time,
            walk_distance,
            tool_durability,
            smash_count,
            text_values,
        ) = world.combat_state_for_parity();
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation: Observation::Combat {
                tile,
                tile_damage,
                stamina,
                attack_time,
                walk_distance,
                tool_durability,
                smash_count,
                text_values,
                sounds: sound_names(world.take_sound_events()),
            },
        });
    }
    Ok(snapshots)
}

/// Drives held movement inputs through the live world/player update path.
pub fn player_motion_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("player_motion")?;
    let mut world = World::new_with_play_options(
        trace.seed,
        WorldSpec::default(),
        PlayOptions {
            difficulty: 1,
            mode: GameMode::Survival,
            score_minutes: 20,
            tutorials: false,
            quests: false,
            show_quests: true,
            custom_skin: false,
        },
    );
    world.prepare_player_motion_parity();
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        match *command {
            ParityCommand::PlayerInput {
                up,
                down,
                left,
                right,
            } => {
                world.tick(&Input {
                    up,
                    down,
                    left,
                    right,
                    ..Input::default()
                });
            }
            ParityCommand::SetPlayerStats {
                stamina,
                stamina_recharge,
                stamina_recharge_delay,
            } => {
                world.set_player_stats_for_parity(stamina, stamina_recharge, stamina_recharge_delay)
            }
            ParityCommand::SetObstacle { dx, dy, solid } => {
                world.set_obstacle_for_parity(dx, dy, solid);
            }
            _ => return Err("non-player command in player_motion trace".to_owned()),
        }
        let (x, y, direction, walk_distance, stamina, recharge, delay, step_count) =
            world.player_motion_state_for_parity();
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation: Observation::Player {
                x,
                y,
                direction: direction.to_owned(),
                walk_distance,
                stamina,
                stamina_recharge: recharge,
                stamina_recharge_delay: delay,
                step_count: u32::from(step_count),
            },
        });
    }
    Ok(snapshots)
}

/// Drives the live title-menu tick function and its frame-time sound decision.
pub fn title_menu_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("title_menu")?;
    let mut selection = 0_usize;
    let mut ticks = 0_u64;
    let mut terminal = false;
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        let ParityCommand::MenuInput { action } = *command else {
            return Err("non-menu command in title_menu trace".to_owned());
        };
        if terminal {
            return Err("title_menu trace continues after a terminal transition".to_owned());
        }
        let input = menu_input(action);
        let sounds = sound_names(immediate_display_sounds(&input, false));
        let (screen, outcome) = title_menu_parity_tick(&mut selection, &mut ticks, &input);
        terminal = screen == "terminated";
        snapshots.push(menu_snapshot(
            trace, tick, screen, selection, sounds, outcome,
        )?);
    }
    Ok(snapshots)
}

/// Drives pause navigation through the live `World::tick` implementation.
pub fn pause_menu_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("pause_menu")?;
    let mut world = World::new_with_play_options(
        trace.seed,
        WorldSpec::default(),
        PlayOptions {
            difficulty: 1,
            mode: GameMode::Survival,
            score_minutes: 20,
            tutorials: false,
            quests: false,
            show_quests: true,
            custom_skin: false,
        },
    );
    world.tick(&Input {
        exit: true,
        ..Input::default()
    });
    let mut terminal = false;
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        let ParityCommand::MenuInput { action } = *command else {
            return Err("non-menu command in pause_menu trace".to_owned());
        };
        if terminal {
            return Err("pause_menu trace continues after leaving the menu".to_owned());
        }
        let input = menu_input(action);
        let sounds = sound_names(world.immediate_menu_sounds(&input));
        let outcome = match world.tick(&input) {
            WorldAction::None => "none",
            WorldAction::SaveGame => "save",
            WorldAction::ReturnToTitle => "title",
            WorldAction::QuitWithoutSaving => "quit",
        };
        let (paused, screen, selection) = world.pause_state_for_parity();
        terminal = !paused;
        snapshots.push(menu_snapshot(
            trace, tick, screen, selection, sounds, outcome,
        )?);
    }
    Ok(snapshots)
}

fn menu_input(action: MenuAction) -> Input {
    let mut input = Input::default();
    match action {
        MenuAction::None => {}
        MenuAction::Up => input.up_pressed = true,
        MenuAction::Down => input.down_pressed = true,
        MenuAction::Left => input.left_pressed = true,
        MenuAction::Right => input.right_pressed = true,
        MenuAction::Select => input.select = true,
        MenuAction::Exit => input.exit = true,
        MenuAction::Attack => input.attack = true,
        MenuAction::Menu => input.menu = true,
        MenuAction::Craft => input.craft = true,
    }
    input
}

fn sound_names(sounds: Vec<crate::audio::SoundEffect>) -> Vec<String> {
    sounds
        .into_iter()
        .map(|sound| sound.name().to_owned())
        .collect()
}

fn menu_snapshot(
    trace: &ParityTrace,
    tick: usize,
    screen: &str,
    selection: usize,
    sounds: Vec<String>,
    outcome: &str,
) -> Result<ParitySnapshot, String> {
    let selection = u32::try_from(selection)
        .map_err(|_| format!("menu selection does not fit u32: {selection}"))?;
    Ok(ParitySnapshot {
        schema: PARITY_SCHEMA,
        system: trace.system.clone(),
        tick: tick as u64,
        observation: Observation::Menu {
            screen: screen.to_owned(),
            selection,
            sounds,
            outcome: outcome.to_owned(),
        },
    })
}

/// Executes elapsed-time samples against the same accumulator used by the game loop.
pub fn fixed_step_snapshots(trace: &ParityTrace) -> Result<Vec<ParitySnapshot>, String> {
    trace.validate_for("fixed_step")?;
    let mut accumulator = FixedTickAccumulator::new();
    let mut total = 0_u64;
    let mut snapshots = Vec::with_capacity(trace.commands.len());
    for (tick, command) in trace.commands.iter().enumerate() {
        let ParityCommand::AdvanceNanos { elapsed_nanos } = *command else {
            return Err("random command in fixed_step trace".to_owned());
        };
        let updates = accumulator.advance(std::time::Duration::from_nanos(elapsed_nanos));
        let updates = u32::try_from(updates)
            .map_err(|_| format!("fixed-step update count does not fit u32: {updates}"))?;
        total += u64::from(updates);
        snapshots.push(ParitySnapshot {
            schema: PARITY_SCHEMA,
            system: trace.system.clone(),
            tick: tick as u64,
            observation: Observation::Ticks { updates, total },
        });
    }
    Ok(snapshots)
}

#[cfg(test)]
mod tests {
    use minicraft_common::parity::{Observation, ParityCommand, ParityTrace};

    use super::{fixed_step_snapshots, java_random_snapshots};

    #[test]
    fn parity_adapter_uses_the_live_java_random_implementation() {
        let trace = ParityTrace {
            schema: 1,
            system: "java_random".to_owned(),
            seed: 0,
            commands: vec![
                ParityCommand::NextInt { bound: 100 },
                ParityCommand::NextBool,
                ParityCommand::NextFloat,
            ],
        };
        let observations: Vec<Observation> = java_random_snapshots(&trace)
            .unwrap()
            .into_iter()
            .map(|snapshot| snapshot.observation)
            .collect();
        assert_eq!(observations[0], Observation::Int { value: 60 });
        assert!(matches!(observations[1], Observation::Bool { .. }));
        assert!(matches!(observations[2], Observation::FloatBits { .. }));
    }

    #[test]
    fn migrated_clock_keeps_live_world_tick_states_identical_across_render_rates() {
        use crate::core::game_loop::GameLoopClock;
        use std::time::{Duration, Instant};

        let mut fixture = super::parity_world(42);
        fixture.prepare_player_motion_parity();
        let saved = fixture.to_save_string().unwrap();
        let mut expected = None;
        let mut expected_save = None;
        for fps in [30_u64, 60, 120, 300] {
            let mut world = crate::world::World::from_save_string(&saved).unwrap();
            let start = Instant::now();
            let mut clock = GameLoopClock::starting_at(start);
            let mut states = Vec::new();
            for frame in 1..=fps * 2 {
                let now = start + Duration::from_nanos(frame * 1_000_000_000 / fps);
                for _ in 0..clock.updates_at(now) {
                    let tick = states.len();
                    if tick == 30 {
                        world.set_player_stats_for_parity(0, 0, 8);
                    }
                    if tick == 75 {
                        world.set_obstacle_for_parity(1, 0, true);
                    }
                    let input = crate::io::Input {
                        right: tick < 30 || (75..90).contains(&tick),
                        down: (30..60).contains(&tick),
                        left: (60..75).contains(&tick),
                        up: tick >= 90,
                        attack: tick == 10,
                        ..crate::io::Input::default()
                    };
                    world.tick(&input);
                    states.push((
                        world.player_motion_state_for_parity(),
                        world.take_sound_events(),
                    ));
                }
            }
            assert_eq!(states.len(), 120, "fps={fps}");
            let final_save: serde_json::Value =
                serde_json::from_str(&world.to_save_string().unwrap()).unwrap();
            if let Some(expected) = &expected {
                assert_eq!(
                    &states, expected,
                    "per-tick state/sound mismatch at {fps} FPS"
                );
                assert_eq!(
                    Some(&final_save),
                    expected_save.as_ref(),
                    "final world mismatch at {fps} FPS"
                );
            } else {
                expected = Some(states);
                expected_save = Some(final_save);
            }
        }
    }

    #[test]
    fn parity_adapter_uses_the_live_fixed_step_accumulator() {
        let trace = ParityTrace {
            schema: 1,
            system: "fixed_step".to_owned(),
            seed: 0,
            commands: vec![
                ParityCommand::AdvanceNanos {
                    elapsed_nanos: 8_333_333,
                },
                ParityCommand::AdvanceNanos {
                    elapsed_nanos: 8_333_334,
                },
                ParityCommand::AdvanceNanos {
                    elapsed_nanos: 100_000_000,
                },
            ],
        };
        let observations: Vec<Observation> = fixed_step_snapshots(&trace)
            .unwrap()
            .into_iter()
            .map(|snapshot| snapshot.observation)
            .collect();
        assert_eq!(
            observations,
            vec![
                Observation::Ticks {
                    updates: 0,
                    total: 0,
                },
                Observation::Ticks {
                    updates: 1,
                    total: 1,
                },
                Observation::Ticks {
                    updates: 6,
                    total: 7,
                },
            ]
        );
    }

    #[test]
    fn render_frame_hashes_are_stable_for_key_menu_and_world_scenes() {
        use minicraft_common::parity::{ParityCommand, ParityTrace, RenderScene};
        for scene in [
            RenderScene::Title,
            RenderScene::Pause,
            RenderScene::Inventory,
            RenderScene::Crafting,
            RenderScene::Hud,
            RenderScene::Quests,
            RenderScene::Focus,
            RenderScene::ExitConfirm,
            RenderScene::ResourcePacks,
        ] {
            let trace = ParityTrace {
                schema: 1,
                system: "render_frame".to_owned(),
                seed: 0,
                commands: vec![ParityCommand::RenderScene { scene }],
            };
            let first = super::render_frame_snapshots(&trace).unwrap();
            let second = super::render_frame_snapshots(&trace).unwrap();
            assert_eq!(first.len(), 1);
            assert_eq!(first, second, "unstable render hash for {scene:?}");
            // Hash fields are non-empty hex from the live RGB buffer.
            if let Observation::RenderFrame {
                rgb_hash,
                width,
                height,
                ..
            } = &first[0].observation
            {
                assert!(!rgb_hash.is_empty());
                assert_eq!(*width, crate::gfx::WIDTH as u32);
                assert_eq!(*height, crate::gfx::HEIGHT as u32);
            } else {
                panic!("expected render frame observation for {scene:?}");
            }
        }
    }

    #[test]
    fn render_frame_rejects_non_render_traces() {
        let trace = ParityTrace {
            schema: 1,
            system: "java_random".to_owned(),
            seed: 0,
            commands: vec![ParityCommand::NextInt { bound: 10 }],
        };
        assert!(super::render_frame_snapshots(&trace).is_err());
    }

    #[test]
    fn combat_snapshots_are_deterministic_for_wood_axe_chop() {
        use minicraft_common::parity::CombatTool;
        let commands = vec![
            ParityCommand::CombatSetup {
                tool: CombatTool::WoodAxe,
            },
            ParityCommand::CombatAttack,
            ParityCommand::CombatAttack,
            ParityCommand::CombatAttack,
        ];
        let trace = |commands: Vec<ParityCommand>| ParityTrace {
            schema: 1,
            system: "combat".to_owned(),
            seed: 7,
            commands,
        };
        let first = super::combat_snapshots(&trace(commands.clone())).unwrap();
        let second = super::combat_snapshots(&trace(commands)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 4);
        // Setup alone leaves the tree intact; attacks must eventually damage it.
        if let Observation::Combat {
            tile,
            tile_damage,
            smash_count,
            text_values,
            ..
        } = &first[0].observation
        {
            assert_eq!(tile, "tree");
            assert_eq!(*tile_damage, 0);
            let _ = smash_count;
            let _ = text_values;
        } else {
            panic!("expected combat observation after setup");
        }
        let after_hits = matches!(&first[3].observation, Observation::Combat { .. });
        assert!(after_hits);
        assert!(
            first
                .iter()
                .skip(1)
                .any(|snapshot| matches!(&snapshot.observation, Observation::Combat { tile_damage, smash_count, .. } if *tile_damage > 0 || *smash_count > 0)),
            "expected wood axe attacks to damage the tree or spawn smash particles"
        );
    }

    #[test]
    fn audio_event_snapshots_are_deterministic_for_pause_menu() {
        use minicraft_common::parity::{AudioScene, MenuAction};
        let commands = vec![
            ParityCommand::AudioSetup {
                scene: AudioScene::Pause,
            },
            ParityCommand::MenuInput {
                action: MenuAction::Down,
            },
            ParityCommand::MenuInput {
                action: MenuAction::Select,
            },
        ];
        let trace = |commands: Vec<ParityCommand>| ParityTrace {
            schema: 1,
            system: "audio_event".to_owned(),
            seed: 3,
            commands,
        };
        let first = super::audio_event_snapshots(&trace(commands.clone())).unwrap();
        let second = super::audio_event_snapshots(&trace(commands)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 3);
    }

    #[test]
    fn inventory_crafting_snapshots_are_deterministic_for_workbench() {
        use minicraft_common::parity::{CraftingProduct, CraftingStation};
        let commands = vec![
            ParityCommand::CraftingSetup {
                station: CraftingStation::Workbench,
                wood: 20,
                coal: 4,
                stone: 8,
                string: 2,
            },
            ParityCommand::CraftingSelect {
                product: CraftingProduct::Plank,
            },
            ParityCommand::CraftingSelect {
                product: CraftingProduct::WoodAxe,
            },
        ];
        let trace = |commands: Vec<ParityCommand>| ParityTrace {
            schema: 1,
            system: "inventory_crafting".to_owned(),
            seed: 11,
            commands,
        };
        let first = super::inventory_crafting_snapshots(&trace(commands.clone())).unwrap();
        let second = super::inventory_crafting_snapshots(&trace(commands)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 3);
    }

    fn motion_trace(commands: Vec<ParityCommand>) -> ParityTrace {
        ParityTrace {
            schema: 1,
            system: "player_motion".to_owned(),
            seed: 0,
            commands,
        }
    }

    #[test]
    fn player_motion_snapshots_are_deterministic_for_replayed_commands() {
        let commands = vec![
            ParityCommand::PlayerInput {
                up: false,
                down: false,
                left: false,
                right: true,
            },
            ParityCommand::PlayerInput {
                up: false,
                down: false,
                left: false,
                right: true,
            },
            ParityCommand::PlayerInput {
                up: true,
                down: false,
                left: false,
                right: false,
            },
            ParityCommand::SetPlayerStats {
                stamina: 10,
                stamina_recharge: 2,
                stamina_recharge_delay: 0,
            },
            ParityCommand::PlayerInput {
                up: false,
                down: false,
                left: true,
                right: false,
            },
        ];
        let first = super::player_motion_snapshots(&motion_trace(commands.clone())).unwrap();
        let second = super::player_motion_snapshots(&motion_trace(commands)).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.len(), 5);
        assert!(
            first.iter().any(|snapshot| matches!(
                &snapshot.observation,
                Observation::Player { walk_distance, .. } if *walk_distance > 0
            )),
            "expected the held right input to advance walk_distance"
        );
    }

    #[test]
    fn fixed_step_logic_is_identical_for_equal_total_time_across_frame_groups() {
        let grouped = ParityTrace {
            schema: 1,
            system: "fixed_step".to_owned(),
            seed: 0,
            commands: vec![
                ParityCommand::AdvanceNanos {
                    elapsed_nanos: 33_333_333,
                },
                ParityCommand::AdvanceNanos {
                    elapsed_nanos: 33_333_333,
                },
                ParityCommand::AdvanceNanos {
                    elapsed_nanos: 33_333_334,
                },
            ],
        };
        let even = ParityTrace {
            schema: 1,
            system: "fixed_step".to_owned(),
            seed: 0,
            commands: vec![ParityCommand::AdvanceNanos {
                elapsed_nanos: 100_000_000,
            }],
        };
        let grouped_total = fixed_step_snapshots(&grouped)
            .unwrap()
            .into_iter()
            .next_back()
            .map(|snapshot| snapshot.observation)
            .unwrap();
        let even_total = fixed_step_snapshots(&even)
            .unwrap()
            .into_iter()
            .next_back()
            .map(|snapshot| snapshot.observation)
            .unwrap();
        let Observation::Ticks {
            updates: grouped_updates,
            total: grouped_ticks,
        } = grouped_total
        else {
            panic!("expected ticks observation");
        };
        let Observation::Ticks {
            updates: even_updates,
            total: even_ticks,
        } = even_total
        else {
            panic!("expected ticks observation");
        };
        assert_eq!(grouped_ticks, even_ticks);
        assert_eq!(grouped_ticks, 6);
        assert_eq!(grouped_updates, 3);
        assert_eq!(even_updates, 6);
    }
}
