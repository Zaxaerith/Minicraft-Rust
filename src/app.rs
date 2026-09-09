use std::path::Path;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use minicraft_client::core::{display_stack::DisplayStack, game_loop::GameLoopClock};

mod display_navigation;
use display_navigation::navigate as navigate_display;
use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};

use crate::{
    assets::{Assets, skin_options},
    audio::{Audio, SoundEffect, validate_embedded_assets, validate_pcm_stream},
    config::{Config, KeyBindings, Settings},
    content::{Achievement, Book, achievements},
    gfx::{HEIGHT, Screen, WIDTH},
    input::{Gamepad, Input, key_name},
    localization::{LOCALES, Localization},
    presenter::Presenter,
    resource_pack::{ResourcePack, discover, display::render as render_resource_packs},
    world::{GameMode, PlayOptions, World, WorldAction, WorldSpec},
    worlds::{
        WorldRecord, copy_record, create_named as create_world_record, delete_record,
        load as load_world_records, load_state, rename_record, save_state,
    },
};

pub fn self_check(arguments: &[String]) -> Result<(), String> {
    let config = Config::load(arguments)?;
    let assets = Assets::load(&[])?;
    if !assets.warnings.is_empty() {
        return Err(format!(
            "embedded asset validation produced warnings: {}",
            assets.warnings.join("; ")
        ));
    }
    for (locale, _) in LOCALES {
        let localization = Localization::load(locale, &[]);
        if localization.text("minicraft.displays.title.play") == "minicraft.displays.title.play" {
            return Err(format!("bundled locale {locale} has no title fallback"));
        }
    }
    for book in Book::ALL.into_iter().chain([Book::Antidious]) {
        if book.pages().is_empty() {
            return Err(format!("embedded book {} has no pages", book.title()));
        }
    }
    if crate::content::tutorials()?.len() != 5
        || crate::content::quests()?
            .iter()
            .map(|group| group.quests.len())
            .sum::<usize>()
            != 14
        || achievements()?.len() != 17
    {
        return Err("embedded progression counts do not match 2.2.4".to_owned());
    }
    validate_embedded_assets()?;
    validate_pcm_stream()?;
    println!(
        "Minicraft Rust self-check passed (savedir {})",
        config.game_dir.display()
    );
    Ok(())
}

const TITLE_KEYS: [&str; 6] = [
    "minicraft.displays.title.play",
    "minicraft.display.options_display",
    "minicraft.displays.skin",
    "minicraft.displays.achievements",
    "minicraft.displays.title.help",
    "minicraft.displays.title.quit",
];

enum State {
    Title {
        selection: usize,
        ticks: u64,
    },
    Options {
        selection: usize,
    },
    PlayMenu {
        selection: usize,
    },
    NewWorld {
        selection: usize,
        name: String,
        seed: String,
        from_play_menu: bool,
    },
    Worlds {
        selection: usize,
        popup: Option<WorldPopup>,
    },
    Help {
        selection: usize,
    },
    Book {
        book: Book,
        page: usize,
    },
    Achievements {
        selection: usize,
    },
    Controls {
        selection: usize,
        capture: Option<(usize, bool)>,
    },
    ControlGuide {
        selection: usize,
        controller: bool,
    },
    Languages {
        selection: usize,
    },
    Skins {
        selection: usize,
        ticks: u64,
    },
    ResourcePacks {
        selection: usize,
    },
    ResourcePackInfo {
        selection: usize,
    },
    Playing {
        world: Box<World>,
        record: WorldRecord,
    },
}

#[derive(Clone, Copy)]
enum WorldPopupKind {
    Copy,
    Rename,
    Delete,
}

struct WorldPopup {
    kind: WorldPopupKind,
    text: String,
}

enum Transition {
    None,
    Title,
    PlayMenu,
    NewWorld(bool),
    CreateWorld(String, Option<i64>),
    Worlds,
    LoadWorld(usize),
    CopyWorld(usize, String),
    RenameWorld(usize, String),
    DeleteWorld(usize),
    Options,
    Controls,
    ControlGuide,
    CaptureBinding(usize, bool),
    SetBinding(usize, String, bool),
    ResetBindings,
    Help,
    Book(Book),
    Achievements,
    Languages,
    Skins,
    PreviewSkin(usize),
    CancelSkins,
    ChangeSkin(usize),
    ResourcePacks,
    TogglePack(usize),
    MovePack(usize),
    PackInfo(usize),
    ClosePackInfo(usize),
    ChangeLocale(usize),
    Quit,
}

pub fn run(arguments: &[String]) -> Result<(), String> {
    let mut config = Config::load(arguments)?;
    let (mut packs, discovery_warnings) = discover(&config.game_dir);
    order_packs_for_menu(&mut packs, &config.settings.resource_packs);
    let active = active_packs(&packs, &config.settings.resource_packs);
    let mut pack_warnings = discovery_warnings.clone();
    let mut localization =
        std::sync::Arc::new(Localization::load(&config.settings.locale, &active));
    let mut assets = Assets::load(&active)?;
    pack_warnings.extend(assets.warnings.iter().cloned());
    let skins = skin_options(&config.game_dir);
    let achievements = achievements()?;
    let mut world_records = load_world_records(&config.game_dir)?;
    let mut unlocked_achievements = HashSet::new();
    for record in &world_records {
        if let Ok(Some(world)) = load_state(record) {
            unlocked_achievements.extend(
                achievements
                    .iter()
                    .filter(|achievement| world.achievement_unlocked(&achievement.id))
                    .map(|achievement| achievement.id.clone()),
            );
        }
    }
    if let Err(error) = assets.select_skin(&config.settings.selected_skin, &config.game_dir) {
        pack_warnings.push(error);
        config.settings.selected_skin = "minicraft.skin.paul".to_owned();
        assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
        config.save()?;
    }
    let mut screen = Screen::new();
    let mut window = Window::new(
        "Minicraft Rust",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: true,
            scale: Scale::X4,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .map_err(|error| error.to_string())?;
    window.set_target_fps(config.settings.fps);
    let mut presenter = match Presenter::new(&window, config.settings.hwa) {
        Ok(presenter) => presenter,
        Err(error) => {
            config.settings.hwa = false;
            config.save()?;
            pack_warnings.push(format!(
                "OpenGL hardware acceleration was disabled: {error}"
            ));
            Presenter::new(&window, false)?
        }
    };
    let mut displays = DisplayStack::default();
    let mut game = minicraft_client::core::game::Game::default();
    displays.open(State::Title {
        selection: 0,
        ticks: 0,
    });
    displays.update();
    let audio = Audio::default();
    let mut gamepad = Gamepad::default();
    let mut fixed_step = GameLoopClock::new();
    let mut input_buffer = crate::input::InputBuffer::default();

    'game: while window.is_open() {
        let focused = window.is_active();
        let raw_keys = if focused {
            window.get_keys_pressed(KeyRepeat::No)
        } else {
            Vec::new()
        };
        let frame_input = if focused {
            Input::poll(
                &window,
                &config.settings.key_bindings,
                &raw_keys,
                &mut gamepad,
            )
        } else {
            Input::default()
        };
        if let Some(State::Playing { world, .. }) = displays.requested_mut() {
            world
                .set_notification_context(localization.clone(), &config.settings.key_bindings.exit);
            world.prepare_quest_layout(&localization);
        }
        let menu_sounds = match displays.requested().expect("application screen") {
            State::Playing { world, .. } => world.immediate_menu_sounds(&frame_input),
            State::Title { .. }
            | State::PlayMenu { .. }
            | State::Options { .. }
            | State::Languages { .. }
            | State::Skins { .. } => Vec::new(),
            State::Achievements { .. } => immediate_achievement_sounds(&frame_input),
            State::Controls { capture, .. } => {
                immediate_key_binding_sounds(capture.is_some(), &frame_input)
            }
            State::ControlGuide { .. } => immediate_control_guide_sounds(&frame_input),
            State::Book { .. } => immediate_book_sounds(&frame_input),
            State::NewWorld { selection, .. } => {
                immediate_new_world_sounds(*selection, &frame_input)
            }
            State::Worlds { popup, .. } => {
                immediate_world_select_sounds(popup.as_ref(), &frame_input)
            }
            State::ResourcePacks { selection } => {
                immediate_resource_pack_sounds(*selection, packs.len(), &frame_input)
            }
            State::ResourcePackInfo { .. } => {
                if frame_input.exit {
                    vec![SoundEffect::Craft]
                } else {
                    Vec::new()
                }
            }
            _ => immediate_display_sounds(&frame_input, true),
        };
        for effect in menu_sounds {
            audio.play(effect, config.settings.sound);
        }
        input_buffer.frame(focused, frame_input, raw_keys);
        if focused {
            input_buffer.sample_keyboard(&window, &config.settings.key_bindings);
        }
        // Java keeps consuming fixed ticks while unfocused. Only input and
        // player/level dispatch are gated, not Display lifecycle or calendar.
        let speed = match displays.current() {
            Some(State::Playing { world, .. }) => world.simulation_speed(),
            _ => 1.0,
        };
        let updates = fixed_step.updates_scaled(speed);
        for _ in 0..updates {
            display_navigation::begin_game_tick(&mut displays, &mut game, focused);
            if !focused {
                if let Some(State::Playing { world, record }) = displays.current_mut() {
                    world.tick_with_autosave(
                        false,
                        &Input::default(),
                        config.settings.autosave,
                        |world| save_state(record, world),
                    )?;
                    world.write_end_game_preferences(&mut config, record.score_minutes)?;
                }
                continue;
            }
            let (input, raw_keys) = input_buffer.tick();
            let mut settings_changed = false;
            let mut menu_events = Vec::new();
            let transition = match displays.current_mut().expect("application screen") {
                State::Title { selection, ticks } => {
                    let (transition, event) =
                        tick_title_event(selection, ticks, &input, world_records.len());
                    menu_events.push(event);
                    transition
                }
                State::Options { selection } => {
                    use minicraft_client::screen::menu::MenuEvent;
                    let previous = *selection;
                    let result = tick_options(selection, &input, &mut config.settings);
                    menu_events.push(if input.exit {
                        MenuEvent::Closed
                    } else if previous != *selection || result.1 {
                        MenuEvent::Moved
                    } else if !matches!(result.0, Transition::None) {
                        MenuEvent::Activated
                    } else {
                        MenuEvent::None
                    });
                    settings_changed = result.1;
                    result.0
                }
                State::PlayMenu { selection } => {
                    let (transition, event) = tick_play_menu_event(selection, &input);
                    menu_events.push(event);
                    transition
                }
                State::NewWorld {
                    selection,
                    name,
                    seed,
                    from_play_menu,
                } => {
                    let result = tick_new_world(
                        selection,
                        name,
                        seed,
                        *from_play_menu,
                        &input,
                        &mut config.settings,
                    );
                    settings_changed = result.1;
                    result.0
                }
                State::Worlds { selection, popup } => {
                    tick_worlds(selection, popup, &input, &world_records)
                }
                State::Help { selection } => tick_help(selection, &input),
                State::Book { page, book } => tick_book(*book, page, &input),
                State::Achievements { selection } => {
                    tick_achievements(selection, &input, achievements.len())
                }
                State::Controls { selection, capture } => {
                    tick_controls(selection, capture, &input, &raw_keys)
                }
                State::ControlGuide {
                    selection,
                    controller,
                } => tick_control_guide(selection, controller, &input),
                State::Languages { selection } => {
                    use minicraft_client::screen::menu::MenuEvent;
                    let previous = *selection;
                    let result = tick_languages(selection, &input);
                    if input.exit {
                        menu_events.push(MenuEvent::Closed);
                    } else if previous != *selection {
                        menu_events.push(MenuEvent::Moved);
                    } else if matches!(result, Transition::ChangeLocale(_)) {
                        menu_events.extend([MenuEvent::Activated, MenuEvent::Closed]);
                    }
                    result
                }
                State::Skins { selection, ticks } => {
                    use minicraft_client::screen::menu::MenuEvent;
                    let result = tick_skins(selection, ticks, &input, skins.len());
                    match result {
                        Transition::CancelSkins => menu_events.push(MenuEvent::Closed),
                        Transition::PreviewSkin(_) => menu_events.push(MenuEvent::Moved),
                        Transition::ChangeSkin(_) => {
                            menu_events.extend([MenuEvent::Activated, MenuEvent::Closed])
                        }
                        _ => {}
                    }
                    result
                }
                State::ResourcePacks { selection } => {
                    tick_resource_packs(selection, &input, &mut packs)
                }
                State::ResourcePackInfo { selection } => {
                    if input.exit {
                        Transition::ClosePackInfo(*selection)
                    } else {
                        Transition::None
                    }
                }
                State::Playing { world, record } => {
                    let action = world.tick_with_autosave(
                        true,
                        &input,
                        config.settings.autosave,
                        |world| save_state(record, world),
                    )?;
                    world.write_end_game_preferences(&mut config, record.score_minutes)?;
                    unlocked_achievements.extend(
                        achievements
                            .iter()
                            .filter(|achievement| world.achievement_unlocked(&achievement.id))
                            .map(|achievement| achievement.id.clone()),
                    );
                    for effect in world.take_sound_events() {
                        audio.play(effect, config.settings.sound);
                    }
                    match action {
                        WorldAction::None => Transition::None,
                        WorldAction::SaveGame => {
                            save_state(record, world)?;
                            world.record_save_success();
                            Transition::None
                        }
                        WorldAction::ReturnToTitle => {
                            save_state(record, world)?;
                            Transition::Title
                        }
                        WorldAction::QuitWithoutSaving => Transition::Title,
                    }
                }
            };

            for event in menu_events {
                use minicraft_client::screen::menu::MenuEvent;
                let effect = match event {
                    MenuEvent::None => continue,
                    MenuEvent::Moved => SoundEffect::Select,
                    MenuEvent::Activated => SoundEffect::Confirm,
                    MenuEvent::Closed => SoundEffect::Craft,
                };
                audio.play(effect, config.settings.sound);
            }
            if settings_changed {
                config.save()?;
                window.set_target_fps(config.settings.fps);
            }
            match transition {
                Transition::None => {}
                Transition::Title => {
                    navigate_display(
                        &mut displays,
                        State::Title {
                            selection: 0,
                            ticks: 0,
                        },
                    );
                }
                Transition::PlayMenu => {
                    navigate_display(&mut displays, State::PlayMenu { selection: 0 })
                }
                Transition::NewWorld(from_play_menu) => navigate_display(
                    &mut displays,
                    State::NewWorld {
                        selection: 0,
                        name: String::new(),
                        seed: String::new(),
                        from_play_menu,
                    },
                ),
                Transition::CreateWorld(name, requested_seed) => {
                    let seed = requested_seed.unwrap_or_else(random_seed);
                    let spec = WorldSpec::new(
                        config.settings.world_size,
                        config.settings.theme,
                        config.settings.terrain_type,
                    );
                    let mode = GameMode::from_index(config.settings.game_mode);
                    let record = create_world_record(
                        &config.game_dir,
                        &name,
                        seed,
                        spec,
                        mode,
                        config.settings.score_minutes,
                    )?;
                    world_records = load_world_records(&config.game_dir)?;
                    let mut world = World::new_with_play_options(
                        seed,
                        spec,
                        play_options(&config.settings, mode, config.settings.score_minutes),
                    );
                    world.set_notification_context(
                        localization.clone(),
                        &config.settings.key_bindings.exit,
                    );
                    save_state(&record, &world)?;
                    world.record_save_success();
                    navigate_display(
                        &mut displays,
                        State::Playing {
                            world: Box::new(world),
                            record,
                        },
                    );
                }
                Transition::Worlds => {
                    world_records = load_world_records(&config.game_dir)?;
                    navigate_display(
                        &mut displays,
                        State::Worlds {
                            selection: 0,
                            popup: None,
                        },
                    );
                }
                Transition::LoadWorld(selection) => {
                    let record = world_records[selection].clone();
                    let mut world = load_state(&record)?.unwrap_or_else(|| {
                        World::new_with_play_options(
                            record.seed,
                            record.spec,
                            play_options(&config.settings, record.mode, record.score_minutes),
                        )
                    });
                    // Java World.initWorld resets asTick when entering a world.
                    world.reset_autosave();
                    world.clear_notifications();
                    world.set_notification_context(
                        localization.clone(),
                        &config.settings.key_bindings.exit,
                    );
                    navigate_display(
                        &mut displays,
                        State::Playing {
                            world: Box::new(world),
                            record,
                        },
                    );
                }
                Transition::CopyWorld(selection, name) => {
                    copy_record(&config.game_dir, &world_records[selection], &name)?;
                    world_records = load_world_records(&config.game_dir)?;
                    let selection = world_records
                        .iter()
                        .position(|record| record.name.eq_ignore_ascii_case(name.trim()))
                        .unwrap_or(0);
                    navigate_display(
                        &mut displays,
                        State::Worlds {
                            selection,
                            popup: None,
                        },
                    );
                }
                Transition::RenameWorld(selection, name) => {
                    rename_record(&config.game_dir, &world_records[selection], &name)?;
                    world_records = load_world_records(&config.game_dir)?;
                    let selection = world_records
                        .iter()
                        .position(|record| record.name.eq_ignore_ascii_case(name.trim()))
                        .unwrap_or(0);
                    navigate_display(
                        &mut displays,
                        State::Worlds {
                            selection,
                            popup: None,
                        },
                    );
                }
                Transition::DeleteWorld(selection) => {
                    delete_record(&config.game_dir, &world_records[selection])?;
                    world_records = load_world_records(&config.game_dir)?;
                    navigate_display(
                        &mut displays,
                        if world_records.is_empty() {
                            State::NewWorld {
                                selection: 0,
                                name: String::new(),
                                seed: String::new(),
                                from_play_menu: true,
                            }
                        } else {
                            State::Worlds {
                                selection: selection.min(world_records.len() - 1),
                                popup: None,
                            }
                        },
                    );
                }
                Transition::Options => {
                    navigate_display(&mut displays, State::Options { selection: 0 })
                }
                Transition::Controls => navigate_display(
                    &mut displays,
                    State::Controls {
                        selection: 0,
                        capture: None,
                    },
                ),
                Transition::ControlGuide => navigate_display(
                    &mut displays,
                    State::ControlGuide {
                        selection: 0,
                        controller: false,
                    },
                ),
                Transition::CaptureBinding(selection, append) => navigate_display(
                    &mut displays,
                    State::Controls {
                        selection,
                        capture: Some((selection, append)),
                    },
                ),
                Transition::SetBinding(selection, value, append) => {
                    if append {
                        config.settings.key_bindings.add(selection, value);
                    } else {
                        config.settings.key_bindings.set(selection, value);
                    }
                    config.save()?;
                    navigate_display(
                        &mut displays,
                        State::Controls {
                            selection,
                            capture: None,
                        },
                    );
                }
                Transition::ResetBindings => {
                    config.settings.key_bindings = KeyBindings::default();
                    config.save()?;
                    navigate_display(
                        &mut displays,
                        State::Controls {
                            selection: 0,
                            capture: None,
                        },
                    );
                }
                Transition::Help => navigate_display(&mut displays, State::Help { selection: 0 }),
                Transition::Book(book) => {
                    navigate_display(&mut displays, State::Book { book, page: 0 })
                }
                Transition::Achievements => {
                    navigate_display(&mut displays, State::Achievements { selection: 0 })
                }
                Transition::Languages => {
                    let current_index = LOCALES
                        .iter()
                        .position(|(code, _)| *code == config.settings.locale)
                        .unwrap_or(0);
                    let selection = sorted_locale_indices()
                        .iter()
                        .position(|index| *index == current_index)
                        .unwrap_or(0);
                    navigate_display(&mut displays, State::Languages { selection });
                }
                Transition::Skins => {
                    let selection = skins
                        .iter()
                        .position(|skin| *skin == config.settings.selected_skin)
                        .unwrap_or(0);
                    navigate_display(
                        &mut displays,
                        State::Skins {
                            selection,
                            ticks: 0,
                        },
                    );
                }
                Transition::PreviewSkin(selection) => {
                    assets.select_skin(&skins[selection], &config.game_dir)?;
                }
                Transition::CancelSkins => {
                    assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
                    navigate_display(
                        &mut displays,
                        State::Title {
                            selection: 2,
                            ticks: 0,
                        },
                    );
                }
                Transition::ChangeSkin(selection) => {
                    assets.select_skin(&skins[selection], &config.game_dir)?;
                    config.settings.selected_skin = skins[selection].clone();
                    config.save()?;
                    navigate_display(
                        &mut displays,
                        State::Title {
                            selection: 2,
                            ticks: 0,
                        },
                    );
                }
                Transition::ResourcePacks => {
                    navigate_display(&mut displays, State::ResourcePacks { selection: 0 })
                }
                Transition::PackInfo(selection) => {
                    navigate_display(&mut displays, State::ResourcePackInfo { selection })
                }
                Transition::ClosePackInfo(selection) => {
                    navigate_display(&mut displays, State::ResourcePacks { selection })
                }
                Transition::TogglePack(selection) => {
                    let id = &packs[selection].id;
                    if let Some(index) = config
                        .settings
                        .resource_packs
                        .iter()
                        .position(|enabled| enabled == id)
                    {
                        config.settings.resource_packs.remove(index);
                    } else {
                        config.settings.resource_packs.push(id.clone());
                    }
                    sync_enabled_pack_order(&mut config.settings.resource_packs, &packs);
                    reload_resources(
                        &mut assets,
                        &mut localization,
                        &mut pack_warnings,
                        &discovery_warnings,
                        &packs,
                        &config,
                    )?;
                    config.save()?;
                    navigate_display(&mut displays, State::ResourcePacks { selection });
                }
                Transition::MovePack(selection) => {
                    sync_enabled_pack_order(&mut config.settings.resource_packs, &packs);
                    reload_resources(
                        &mut assets,
                        &mut localization,
                        &mut pack_warnings,
                        &discovery_warnings,
                        &packs,
                        &config,
                    )?;
                    config.save()?;
                    navigate_display(&mut displays, State::ResourcePacks { selection });
                }
                Transition::ChangeLocale(selection) => {
                    let index = sorted_locale_indices()[selection];
                    config.settings.locale = LOCALES[index].0.to_owned();
                    config.save()?;
                    let active = active_packs(&packs, &config.settings.resource_packs);
                    localization =
                        std::sync::Arc::new(Localization::load(&config.settings.locale, &active));
                    navigate_display(&mut displays, State::Options { selection: 0 });
                }
                Transition::Quit => break 'game,
            }
        }

        screen.clear(0);
        match displays.current_mut().expect("application screen") {
            State::Title { selection, ticks } => {
                render_title(&mut screen, &assets, &localization, *selection, *ticks)
            }
            State::Options { selection } => render_options(
                &mut screen,
                &assets,
                &localization,
                &config.settings,
                *selection,
            ),
            State::PlayMenu { selection } => render_play_menu(
                &mut screen,
                &assets,
                &localization,
                world_records.len(),
                *selection,
            ),
            State::NewWorld {
                selection,
                name,
                seed,
                ..
            } => render_new_world(
                &mut screen,
                &assets,
                &localization,
                &config.settings,
                *selection,
                name,
                seed,
            ),
            State::Worlds { selection, popup } => render_worlds(
                &mut screen,
                &assets,
                &localization,
                &config.settings,
                &world_records,
                *selection,
                popup.as_ref(),
            ),
            State::Help { selection } => render_help(&mut screen, &assets, *selection),
            State::Book { book, page } => render_book(&mut screen, &assets, *book, *page),
            State::Achievements { selection } => render_achievements(
                &mut screen,
                &assets,
                &localization,
                &achievements,
                &unlocked_achievements,
                &config.settings.key_bindings,
                *selection,
            ),
            State::Controls { selection, capture } => render_controls(
                &mut screen,
                &assets,
                &localization,
                &config.settings.key_bindings,
                *selection,
                *capture,
            ),
            State::ControlGuide {
                selection,
                controller,
            } => render_control_guide(&mut screen, &assets, &localization, *selection, *controller),
            State::Languages { selection } => render_languages(
                &mut screen,
                &assets,
                &localization,
                *selection,
                &config.settings.locale,
            ),
            State::Skins { selection, ticks } => render_skins(
                &mut screen,
                &assets,
                &localization,
                &skins,
                *selection,
                *ticks,
            ),
            State::ResourcePacks { selection } => render_resource_packs(
                &mut screen,
                &assets,
                &localization,
                &packs,
                &config.settings.resource_packs,
                *selection,
                &config.settings.key_bindings,
            ),
            State::Playing { world, .. } => {
                world.render_with_ready(&mut screen, &assets, &localization, game.ready())
            }
            State::ResourcePackInfo { selection } => {
                render_resource_packs(
                    &mut screen,
                    &assets,
                    &localization,
                    &packs,
                    &config.settings.resource_packs,
                    *selection,
                    &config.settings.key_bindings,
                );
                crate::resource_pack::display::render_info(
                    &mut screen,
                    &assets,
                    &localization,
                    packs.get(*selection),
                );
            }
        }
        if !focused {
            let focus_ticks = match displays.current() {
                Some(State::Playing { world, .. }) => world.calendar_tick(),
                _ => u64::from(game.calendar_tick()),
            };
            render_focus_nagger(&mut screen, &assets, focus_ticks);
        }
        presenter.present(&mut window, screen.pixels())?;
    }
    if let Some(State::Playing { world, record, .. }) = displays.requested() {
        save_state(record, world)?;
    }
    Ok(())
}

pub fn render_preview(arguments: &[String], output: &Path) -> Result<(), String> {
    let config = Config::load(arguments)?;
    let (packs, _) = discover(&config.game_dir);
    let active = active_packs(&packs, &config.settings.resource_packs);
    let localization = Localization::load(&config.settings.locale, &active);
    let assets = Assets::load(&active)?;
    let mut screen = Screen::new();
    screen.clear(0);
    render_title(&mut screen, &assets, &localization, 0, 30);
    screen.save_png(output)
}

pub fn render_world_preview(arguments: &[String], output: &Path) -> Result<(), String> {
    let config = Config::load(arguments)?;
    let (packs, _) = discover(&config.game_dir);
    let active = active_packs(&packs, &config.settings.resource_packs);
    let localization = Localization::load(&config.settings.locale, &active);
    let mut assets = Assets::load(&active)?;
    assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
    let depth = arguments
        .iter()
        .position(|value| value == "--depth")
        .and_then(|index| arguments.get(index + 1))
        .map(|value| {
            value
                .parse::<i8>()
                .map_err(|error| format!("invalid --depth value {value}: {error}"))
        })
        .transpose()?
        .unwrap_or(0);
    let mut world = World::new_at_depth_with_options(
        0x100,
        depth,
        WorldSpec::new(
            config.settings.world_size,
            config.settings.theme,
            config.settings.terrain_type,
        ),
        play_options(
            &config.settings,
            GameMode::from_index(config.settings.game_mode),
            config.settings.score_minutes,
        ),
    )?;
    if arguments.iter().any(|argument| argument == "--entities") {
        world.populate_entity_preview();
    }
    if arguments
        .iter()
        .any(|argument| argument == "--workbench-ui")
    {
        world.populate_workbench_preview();
    }
    if arguments.iter().any(|argument| argument == "--food-ui") {
        world.populate_food_preview();
    }
    if arguments
        .iter()
        .any(|argument| argument == "--inventory-ui")
    {
        world.populate_inventory_preview();
    }
    if arguments
        .iter()
        .any(|argument| argument == "--personal-crafting-ui")
    {
        world.populate_personal_crafting_preview();
    }
    if arguments
        .iter()
        .any(|argument| argument == "--creative-inventory-ui")
    {
        world.populate_creative_inventory_preview();
    }
    if arguments.iter().any(|argument| argument == "--stations") {
        world.populate_stations_preview();
    }
    if arguments.iter().any(|argument| argument == "--score-ui") {
        world.populate_score_preview();
    }
    if arguments.iter().any(|argument| argument == "--book-ui") {
        world.populate_book_preview();
    }
    if arguments.iter().any(|argument| argument == "--sign-ui") {
        world.populate_sign_preview();
    }
    if arguments.iter().any(|argument| argument == "--progress-ui") {
        world.populate_progress_preview();
    }
    if arguments.iter().any(|argument| argument == "--boss-ui") {
        world.populate_boss_preview();
    }
    if arguments.iter().any(|argument| argument == "--pause-ui") {
        world.populate_pause_preview();
    }
    if arguments.iter().any(|argument| argument == "--quest-ui") {
        world.prepare_quests_audio_parity();
    }
    let mut screen = Screen::new();
    screen.clear(0);
    world.render(&mut screen, &assets, &localization);
    screen.save_png(output)
}

pub fn render_ui_preview(arguments: &[String], name: &str, output: &Path) -> Result<(), String> {
    let config = Config::load(arguments)?;
    let (packs, _) = discover(&config.game_dir);
    let active = active_packs(&packs, &config.settings.resource_packs);
    let localization = Localization::load(&config.settings.locale, &active);
    let assets = Assets::load(&active)?;
    let achievements = achievements()?;
    let worlds = load_world_records(&config.game_dir)?;
    let skins = skin_options(&config.game_dir);
    let mut screen = Screen::new();
    screen.clear(0);
    match name {
        "options" => render_options(&mut screen, &assets, &localization, &config.settings, 5),
        "play" => render_play_menu(&mut screen, &assets, &localization, worlds.len(), 0),
        "new-world" => render_new_world(
            &mut screen,
            &assets,
            &localization,
            &config.settings,
            0,
            "",
            "",
        ),
        "worlds" => render_worlds(
            &mut screen,
            &assets,
            &localization,
            &config.settings,
            &worlds,
            0,
            None,
        ),
        "help" => render_help(&mut screen, &assets, 0),
        "book" => render_book(&mut screen, &assets, Book::Instructions, 0),
        "achievements" => render_achievements(
            &mut screen,
            &assets,
            &localization,
            &achievements,
            &HashSet::new(),
            &config.settings.key_bindings,
            0,
        ),
        "controls" => render_controls(
            &mut screen,
            &assets,
            &localization,
            &config.settings.key_bindings,
            0,
            None,
        ),
        "skins" => render_skins(&mut screen, &assets, &localization, &skins, 0, 160),
        _ => {
            return Err(format!(
                "unknown UI preview {name}; use options, play, new-world, worlds, help, book, achievements, controls, or skins"
            ));
        }
    }
    screen.save_png(output)
}

pub(crate) fn immediate_display_sounds(input: &Input, can_exit: bool) -> Vec<SoundEffect> {
    if can_exit && input.exit {
        vec![SoundEffect::Craft]
    } else if input.up_pressed || input.down_pressed || input.left_pressed || input.right_pressed {
        vec![SoundEffect::Select]
    } else if input.select {
        vec![SoundEffect::Confirm]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_options_sounds(selection: usize, input: &Input) -> Vec<SoundEffect> {
    if input.exit {
        vec![SoundEffect::Craft]
    } else if input.up_pressed
        || input.down_pressed
        || (selection < 4 && (input.left_pressed || input.right_pressed))
    {
        vec![SoundEffect::Select]
    } else if selection >= 4 && input.select {
        vec![SoundEffect::Confirm]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_select_and_close_sounds(input: &Input) -> Vec<SoundEffect> {
    if input.exit {
        vec![SoundEffect::Craft]
    } else if input.up_pressed || input.down_pressed {
        vec![SoundEffect::Select]
    } else if input.select {
        vec![SoundEffect::Confirm, SoundEffect::Craft]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_achievement_sounds(input: &Input) -> Vec<SoundEffect> {
    if input.exit {
        vec![SoundEffect::Craft, SoundEffect::Confirm]
    } else if input.up_pressed || input.down_pressed {
        vec![SoundEffect::Select]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_key_binding_sounds(capturing: bool, input: &Input) -> Vec<SoundEffect> {
    if capturing {
        Vec::new()
    } else if input.exit {
        vec![SoundEffect::Craft]
    } else if input.up_pressed || input.down_pressed {
        vec![SoundEffect::Select]
    } else if input.select {
        vec![SoundEffect::Confirm]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_control_guide_sounds(input: &Input) -> Vec<SoundEffect> {
    if input.exit {
        vec![SoundEffect::Craft]
    } else if input.up_pressed || input.down_pressed {
        vec![SoundEffect::Select]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_book_sounds(input: &Input) -> Vec<SoundEffect> {
    if input.exit {
        vec![SoundEffect::Craft]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_new_world_sounds(selection: usize, input: &Input) -> Vec<SoundEffect> {
    if input.exit {
        vec![SoundEffect::Craft]
    } else if input.up_pressed
        || input.down_pressed
        || (matches!(selection, 1 | 2 | 4 | 5 | 6 | 7 | 8)
            && (input.left_pressed || input.right_pressed))
    {
        vec![SoundEffect::Select]
    } else if selection == 3 && input.select {
        vec![SoundEffect::Confirm]
    } else {
        Vec::new()
    }
}

pub(crate) fn immediate_resource_pack_sounds(
    selection: usize,
    pack_count: usize,
    input: &Input,
) -> Vec<SoundEffect> {
    // ResourcePackDisplay consumes horizontal inputs before Display.tick, even at a boundary.
    if input.right_pressed {
        return if selection + 1 < pack_count {
            vec![SoundEffect::Select]
        } else {
            Vec::new()
        };
    }
    if input.left_pressed {
        return if selection > 0 && selection < pack_count {
            vec![SoundEffect::Select]
        } else {
            Vec::new()
        };
    }
    if input.exit {
        return vec![SoundEffect::Craft];
    }
    let mut sounds = Vec::new();
    if pack_count > 0 {
        if input.up_pressed != input.down_pressed {
            sounds.push(SoundEffect::Select);
        } else if input.select {
            sounds.push(SoundEffect::Confirm);
        }
    }
    sounds
}

pub(crate) fn immediate_world_popup_sounds(
    popup_accepts_select: bool,
    input: &Input,
) -> Vec<SoundEffect> {
    if input.exit {
        vec![SoundEffect::Craft]
    } else if input.select && popup_accepts_select {
        vec![SoundEffect::Confirm, SoundEffect::Craft]
    } else {
        Vec::new()
    }
}

fn immediate_world_select_sounds(popup: Option<&WorldPopup>, input: &Input) -> Vec<SoundEffect> {
    if let Some(popup) = popup {
        return immediate_world_popup_sounds(
            matches!(popup.kind, WorldPopupKind::Delete) || !popup.text.trim().is_empty(),
            input,
        );
    }
    immediate_display_sounds(input, true)
}

fn tick_title(
    selection: &mut usize,
    ticks: &mut u64,
    input: &Input,
    world_count: usize,
) -> Transition {
    tick_title_event(selection, ticks, input, world_count).0
}

fn tick_title_event(
    selection: &mut usize,
    ticks: &mut u64,
    input: &Input,
    world_count: usize,
) -> (Transition, minicraft_client::screen::menu::MenuEvent) {
    use minicraft_client::screen::menu::{MenuEvent, tick_selectable};
    *ticks = ticks.wrapping_add(1);
    let event = tick_selectable(
        selection,
        TITLE_KEYS.len(),
        input.up_pressed,
        input.down_pressed,
        input.select,
        false,
    );
    if event != MenuEvent::Activated {
        return (Transition::None, event);
    }
    let transition = match *selection {
        0 if world_count == 0 => Transition::NewWorld(false),
        0 => Transition::PlayMenu,
        1 => Transition::Options,
        2 => Transition::Skins,
        3 => Transition::Achievements,
        4 => Transition::Help,
        _ => Transition::Quit,
    };
    (transition, event)
}

pub(crate) fn title_menu_parity_tick(
    selection: &mut usize,
    ticks: &mut u64,
    input: &Input,
) -> (&'static str, &'static str) {
    match tick_title(selection, ticks, input, 0) {
        Transition::None => ("title", "none"),
        Transition::PlayMenu => ("play_menu", "open"),
        Transition::NewWorld(_) => ("new_world", "open"),
        Transition::Options => ("options", "open"),
        Transition::Skins => ("skins", "open"),
        Transition::Achievements => ("achievements", "open"),
        Transition::Help => ("help", "open"),
        Transition::Quit => ("terminated", "quit"),
        _ => ("unsupported", "open"),
    }
}

fn tick_play_menu_event(
    selection: &mut usize,
    input: &Input,
) -> (Transition, minicraft_client::screen::menu::MenuEvent) {
    use minicraft_client::screen::menu::{MenuEvent, tick_selectable};
    const COUNT: usize = 2;
    let event = tick_selectable(
        selection,
        COUNT,
        input.up_pressed,
        input.down_pressed,
        input.select,
        input.exit,
    );
    let transition = match event {
        MenuEvent::Closed => Transition::Title,
        MenuEvent::Activated => match *selection {
            0 => Transition::Worlds,
            _ => Transition::NewWorld(true),
        },
        _ => Transition::None,
    };
    (transition, event)
}

fn tick_new_world(
    selection: &mut usize,
    name: &mut String,
    seed: &mut String,
    from_play_menu: bool,
    input: &Input,
    settings: &mut Settings,
) -> (Transition, bool) {
    if input.exit {
        return (
            if from_play_menu {
                Transition::PlayMenu
            } else {
                Transition::Title
            },
            false,
        );
    }
    let score_mode = GameMode::from_index(settings.game_mode) == GameMode::Score;
    if input.up_pressed {
        loop {
            *selection = selection.checked_sub(1).unwrap_or(9);
            if score_mode || *selection != 2 {
                break;
            }
        }
    }
    if input.down_pressed {
        loop {
            *selection = (*selection + 1) % 10;
            if score_mode || *selection != 2 {
                break;
            }
        }
    }
    if *selection == 0 {
        edit_text(name, input, 36);
    } else if *selection == 9 {
        edit_text(seed, input, 20);
    }
    let direction = i32::from(input.right_pressed) - i32::from(input.left_pressed);
    let mut changed = false;
    if direction != 0 {
        match *selection {
            1 => {
                settings.game_mode = wrap(settings.game_mode, direction, GameMode::ALL.len());
                if GameMode::from_index(settings.game_mode) != GameMode::Score && *selection == 2 {
                    *selection = 3;
                }
                changed = true;
            }
            2 => {
                let times: Vec<usize> = [10, 20, 40, 60, 120]
                    .into_iter()
                    .filter(|time| match time {
                        10 => settings.score_time_10_unlocked,
                        120 => settings.score_time_120_unlocked,
                        _ => true,
                    })
                    .collect();
                let current = times
                    .iter()
                    .position(|time| *time == settings.score_minutes)
                    .unwrap_or(1);
                settings.score_minutes = times[wrap(current, direction, times.len())];
                changed = true;
            }
            4 => {
                let sizes = [128, 256, 512];
                let current = sizes
                    .iter()
                    .position(|size| *size == settings.world_size)
                    .unwrap_or(0);
                settings.world_size = sizes[wrap(current, direction, sizes.len())];
                changed = true;
            }
            5 => {
                settings.theme = wrap(settings.theme, direction, 5);
                changed = true;
            }
            6 => {
                settings.terrain_type = wrap(settings.terrain_type, direction, 4);
                changed = true;
            }
            7 => {
                settings.quests = !settings.quests;
                changed = true;
            }
            8 => {
                settings.tutorials = !settings.tutorials;
                changed = true;
            }
            _ => {}
        }
    }
    if input.select {
        match *selection {
            3 if !name.trim().is_empty() => (
                Transition::CreateWorld(name.trim().to_owned(), parse_world_seed(seed)),
                changed,
            ),
            _ => (Transition::None, changed),
        }
    } else {
        (Transition::None, changed)
    }
}

fn tick_worlds(
    selection: &mut usize,
    popup: &mut Option<WorldPopup>,
    input: &Input,
    worlds: &[WorldRecord],
) -> Transition {
    if let Some(dialog) = popup {
        if input.exit {
            *popup = None;
            return Transition::None;
        }
        if !matches!(dialog.kind, WorldPopupKind::Delete) {
            edit_text(&mut dialog.text, input, 36);
        }
        if input.select {
            return match dialog.kind {
                WorldPopupKind::Copy if !dialog.text.trim().is_empty() => {
                    Transition::CopyWorld(*selection, dialog.text.clone())
                }
                WorldPopupKind::Rename if !dialog.text.trim().is_empty() => {
                    Transition::RenameWorld(*selection, dialog.text.clone())
                }
                WorldPopupKind::Delete => Transition::DeleteWorld(*selection),
                _ => Transition::None,
            };
        }
        return Transition::None;
    }
    if input.exit {
        return Transition::PlayMenu;
    }
    if worlds.is_empty() {
        return if input.select {
            Transition::NewWorld(true)
        } else {
            Transition::None
        };
    }
    navigate(selection, input, worlds.len());
    if input.world_copy {
        *popup = Some(WorldPopup {
            kind: WorldPopupKind::Copy,
            text: String::new(),
        });
        return Transition::None;
    }
    if input.world_rename {
        *popup = Some(WorldPopup {
            kind: WorldPopupKind::Rename,
            text: worlds[*selection].name.clone(),
        });
        return Transition::None;
    }
    if input.world_delete {
        *popup = Some(WorldPopup {
            kind: WorldPopupKind::Delete,
            text: String::new(),
        });
        return Transition::None;
    }
    if input.select {
        Transition::LoadWorld(*selection)
    } else {
        Transition::None
    }
}

fn tick_help(selection: &mut usize, input: &Input) -> Transition {
    if input.exit {
        return Transition::Title;
    }
    navigate(selection, input, Book::ALL.len());
    if input.select {
        Transition::Book(Book::ALL[*selection])
    } else {
        Transition::None
    }
}

fn tick_book(book: Book, page: &mut usize, input: &Input) -> Transition {
    if input.exit {
        return Transition::Help;
    }
    let page_count = book.pages().len();
    if input.left_pressed || input.up_pressed {
        *page = page.saturating_sub(1);
    }
    if input.right_pressed || input.down_pressed || input.select {
        if *page + 1 < page_count {
            *page += 1;
        } else if input.select {
            return Transition::Help;
        }
    }
    Transition::None
}

fn tick_achievements(selection: &mut usize, input: &Input, count: usize) -> Transition {
    if input.exit {
        return Transition::Title;
    }
    navigate(selection, input, count);
    Transition::None
}

fn tick_controls(
    selection: &mut usize,
    capture: &mut Option<(usize, bool)>,
    input: &Input,
    raw_keys: &[Key],
) -> Transition {
    if let Some((index, append)) = *capture {
        if raw_keys.contains(&Key::Escape) {
            return Transition::Controls;
        }
        if let Some(name) = raw_keys
            .iter()
            .filter(|key| {
                !matches!(
                    key,
                    Key::LeftShift
                        | Key::RightShift
                        | Key::LeftCtrl
                        | Key::RightCtrl
                        | Key::LeftAlt
                        | Key::RightAlt
                )
            })
            .find_map(|key| key_name(*key))
        {
            let value = format!(
                "{}{}{}{name}",
                if input.ctrl { "CTRL-" } else { "" },
                if input.alt { "ALT-" } else { "" },
                if input.shift { "SHIFT-" } else { "" }
            );
            return Transition::SetBinding(index, value, append);
        }
        return Transition::None;
    }
    const COUNT: usize = 10;
    if input.exit {
        return Transition::Options;
    }
    navigate(selection, input, COUNT);
    if input.world_delete {
        Transition::ResetBindings
    } else if raw_keys.contains(&Key::A) {
        Transition::CaptureBinding(*selection, true)
    } else if input.select || raw_keys.contains(&Key::C) {
        Transition::CaptureBinding(*selection, false)
    } else {
        Transition::None
    }
}

fn tick_control_guide(selection: &mut usize, controller: &mut bool, input: &Input) -> Transition {
    if input.exit {
        return Transition::Options;
    }
    if input.left_pressed || input.right_pressed {
        *controller = !*controller;
        *selection = 0;
    }
    navigate(selection, input, if *controller { 16 } else { 23 });
    Transition::None
}

fn tick_options(
    selection: &mut usize,
    input: &Input,
    settings: &mut Settings,
) -> (Transition, bool) {
    const COUNT: usize = 8;
    if input.exit {
        return (Transition::Title, false);
    }
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(COUNT - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % COUNT;
    }
    if input.up_pressed != input.down_pressed {
        return (Transition::None, false);
    }
    let direction = i32::from(input.right_pressed) - i32::from(input.left_pressed);
    let mut changed = false;
    if direction != 0 {
        match *selection {
            0 => {
                settings.fps = (settings.fps as i32 + direction * 10).clamp(10, 300) as usize;
                changed = true;
            }
            1 => {
                settings.sound = !settings.sound;
                changed = true;
            }
            2 => {
                settings.show_quests = !settings.show_quests;
                changed = true;
            }
            3 => {
                settings.hwa = !settings.hwa;
                changed = true;
            }
            _ => {}
        }
    }
    if input.select {
        return match *selection {
            4 => (Transition::Controls, changed),
            5 => (Transition::ControlGuide, changed),
            6 => (Transition::Languages, changed),
            7 => (Transition::ResourcePacks, changed),
            _ => (Transition::None, changed),
        };
    }
    (Transition::None, changed)
}

fn navigate(selection: &mut usize, input: &Input, count: usize) {
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(count - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % count;
    }
}

fn tick_languages(selection: &mut usize, input: &Input) -> Transition {
    if input.exit {
        return Transition::Options;
    }
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(LOCALES.len() - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % LOCALES.len();
    }
    if input.up_pressed != input.down_pressed {
        return Transition::None;
    }
    if input.select {
        Transition::ChangeLocale(*selection)
    } else {
        Transition::None
    }
}

fn tick_skins(selection: &mut usize, ticks: &mut u64, input: &Input, count: usize) -> Transition {
    *ticks = ticks.wrapping_add(1);
    if input.exit {
        return Transition::CancelSkins;
    }
    if count == 0 {
        return Transition::None;
    }
    let previous = *selection;
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(count - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % count;
    }
    if *selection != previous {
        Transition::PreviewSkin(*selection)
    } else if input.select {
        Transition::ChangeSkin(*selection)
    } else {
        Transition::None
    }
}

fn tick_resource_packs(
    selection: &mut usize,
    input: &Input,
    packs: &mut [ResourcePack],
) -> Transition {
    if input.right_pressed {
        if *selection + 1 < packs.len() {
            packs.swap(*selection, *selection + 1);
            *selection += 1;
            return Transition::MovePack(*selection);
        }
        return Transition::None;
    }
    if input.left_pressed {
        if *selection > 0 && *selection < packs.len() {
            packs.swap(*selection, *selection - 1);
            *selection -= 1;
            return Transition::MovePack(*selection);
        }
        return Transition::None;
    }
    if input.exit {
        return Transition::Options;
    }
    if input.menu {
        return Transition::PackInfo(*selection);
    }
    if packs.is_empty() {
        return Transition::None;
    }
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(packs.len() - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % packs.len();
    }
    if input.up_pressed != input.down_pressed {
        return Transition::None;
    }
    if input.select {
        return Transition::TogglePack(*selection);
    }
    Transition::None
}

pub(crate) fn resource_pack_menu_parity_step(
    selection: &mut usize,
    input: &Input,
    packs: &mut [ResourcePack],
    info: &mut bool,
) -> (&'static str, &'static str, Vec<SoundEffect>) {
    if *info {
        if input.exit {
            *info = false;
            return ("resource_packs", "close", vec![SoundEffect::Craft]);
        }
        return ("resource_pack_info", "none", Vec::new());
    }
    let sounds = immediate_resource_pack_sounds(*selection, packs.len(), input);
    match tick_resource_packs(selection, input, packs) {
        Transition::PackInfo(_) => {
            *info = true;
            ("resource_pack_info", "open", sounds)
        }
        Transition::Options => ("options", "close", sounds),
        _ => ("resource_packs", "none", sounds),
    }
}

pub(crate) fn render_resource_pack_parity(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    scene: minicraft_common::parity::RenderScene,
) {
    use minicraft_common::parity::RenderScene;
    let mut packs = vec![
        ResourcePack::parity_fixture("alpha", "Alpha", Some(assets.default_pack_logo.clone())),
        ResourcePack::parity_fixture("beta", "Beta", None),
        ResourcePack::parity_fixture("gamma", "Gamma", None),
    ];
    let mut enabled = vec!["alpha".to_owned(), "gamma".to_owned()];
    order_packs_for_menu(&mut packs, &enabled);
    let mut selection = 2;
    if matches!(
        scene,
        RenderScene::ResourcePacksReordered | RenderScene::ResourcePacksToggled
    ) {
        tick_resource_packs(
            &mut selection,
            &Input {
                left_pressed: true,
                ..Input::default()
            },
            &mut packs,
        );
        sync_enabled_pack_order(&mut enabled, &packs);
    }
    if scene == RenderScene::ResourcePacksToggled
        && let Transition::TogglePack(index) = tick_resource_packs(
            &mut selection,
            &Input {
                select: true,
                ..Input::default()
            },
            &mut packs,
        )
    {
        enabled.push(packs[index].id.clone());
        sync_enabled_pack_order(&mut enabled, &packs);
    }
    if scene == RenderScene::ResourcePacksDefault {
        packs.clear();
        enabled.clear();
        selection = 0;
    }
    if scene == RenderScene::ResourcePacksLogo {
        selection = 1;
    }
    if scene == RenderScene::ResourcePacksScrolled {
        packs = (0..15)
            .map(|index| {
                ResourcePack::parity_fixture(
                    &format!("pack{index:02}"),
                    &format!("Pack {index:02}"),
                    None,
                )
            })
            .collect();
        enabled = vec!["pack00".into(), "pack14".into()];
        selection = 0;
        for _ in 0..14 {
            tick_resource_packs(
                &mut selection,
                &Input {
                    down_pressed: true,
                    ..Input::default()
                },
                &mut packs,
            );
        }
    }
    render_resource_packs(
        screen,
        assets,
        localization,
        &packs,
        &enabled,
        selection,
        &KeyBindings::default(),
    );
    if scene == RenderScene::ResourcePacksInfo {
        crate::resource_pack::display::render_info(
            screen,
            assets,
            localization,
            packs.get(selection),
        );
    }
}

pub(crate) fn render_title(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    selection: usize,
    ticks: u64,
) {
    screen.blit(
        assets.title(),
        (WIDTH as i32 - assets.title().width as i32) / 2,
        26 - assets.title().height as i32 / 2,
    );
    let phase = ticks % 50;
    let count = if phase < 25 { phase + 1 } else { 49 - phase };
    let brightness = 5_u64.saturating_sub(count / 5);
    let splash_color = ((brightness * 51) << 16) | ((brightness * 51) << 8) | (brightness * 25);
    screen.text_colored(
        assets.font(),
        "Now with skins!",
        (WIDTH as i32 - minicraft_client::gfx::font::text_width(&"Now with skins!")) / 2,
        HEIGHT as i32 / 2 - 44,
        splash_color as u32,
    );
    let version = localization.format("minicraft.displays.title.display.version", &["2.2.4"]);
    screen.text_colored(assets.font(), &version, 1, 1, 0x333333);
    let latest = localization.text("minicraft.displays.title.display.latest_already");
    screen.text_colored(
        assets.font(),
        latest,
        (WIDTH as i32 - minicraft_client::gfx::font::text_width(&latest)) / 2,
        76,
        0x333333,
    );
    for (index, key) in TITLE_KEYS.iter().enumerate() {
        let item = localization.text(key);
        let y = 96 + index as i32 * 10;
        if index == selection {
            let width = minicraft_client::gfx::font::text_width(&item);
            let x = (WIDTH as i32 - width) / 2;
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(assets.font(), "<", x + width, y);
        }
        let x = (WIDTH as i32 - minicraft_client::gfx::font::text_width(&item)) / 2;
        screen.text_colored(
            assets.font(),
            item,
            x,
            y,
            if index == selection {
                0xFFFFFF
            } else {
                0x999999
            },
        );
    }
    for (message, y) in [
        (
            localization.format("minicraft.displays.title.display.help.0", &["UP", "DOWN"]),
            HEIGHT as i32 - 30,
        ),
        (
            localization.format("minicraft.displays.title.display.help.1", &["ENTER"]),
            HEIGHT as i32 - 20,
        ),
        (
            localization.format("minicraft.displays.title.display.help.2", &["ESCAPE"]),
            HEIGHT as i32 - 10,
        ),
    ] {
        screen.text_colored(
            assets.font(),
            &message,
            (WIDTH as i32 - minicraft_client::gfx::font::text_width(&message)) / 2,
            y,
            0x333333,
        );
    }
}

fn render_options(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    settings: &Settings,
    selection: usize,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.options_main_menu"),
        35,
    );
    let on = localization.text("minicraft.display.entries.boolean.true");
    let off = localization.text("minicraft.display.entries.boolean.false");
    let labels = [
        format!(
            "{}: {}",
            localization.text("minicraft.settings.fps"),
            settings.fps
        ),
        format!(
            "{}: {}",
            localization.text("minicraft.settings.sound"),
            if settings.sound { on } else { off }
        ),
        format!(
            "{}: {}",
            localization.text("minicraft.settings.show_quests"),
            if settings.show_quests { on } else { off }
        ),
        format!(
            "{}: {}",
            localization.text("minicraft.settings.opengl_hwa"),
            if settings.hwa { on } else { off }
        ),
        localization
            .text("minicraft.display.options_display.change_key_bindings")
            .to_owned(),
        localization.text("minicraft.displays.controls").to_owned(),
        localization
            .text("minicraft.display.options_display.language")
            .to_owned(),
        localization
            .text("minicraft.display.options_display.resource_packs")
            .to_owned(),
    ];
    let width = labels
        .iter()
        .map(|label| label.chars().count())
        .max()
        .unwrap_or(0) as i32
        * 8;
    let x = (WIDTH as i32 - width) / 2;
    for (index, label) in labels.iter().enumerate() {
        let y = 51 + index as i32 * 14;
        if index == selection {
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(
                assets.font(),
                "<",
                x + minicraft_client::gfx::font::text_width(&label),
                y,
            );
        }
        screen.text_colored(
            assets.font(),
            label,
            x,
            y,
            if index == selection {
                0xFFFFFF
            } else {
                0x999999
            },
        );
    }
}

fn render_play_menu(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    world_count: usize,
    selection: usize,
) {
    let _ = world_count;
    let labels = [
        localization
            .text("minicraft.displays.title.play.load_world")
            .to_owned(),
        localization
            .text("minicraft.displays.title.play.new_world")
            .to_owned(),
    ];
    render_centered_menu(screen, assets, &labels, selection, 87, 10);
}

fn render_new_world(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    settings: &Settings,
    selection: usize,
    name: &str,
    seed: &str,
) {
    const THEME_KEYS: [&str; 5] = [
        "minicraft.settings.theme.normal",
        "minicraft.settings.theme.forest",
        "minicraft.settings.theme.desert",
        "minicraft.settings.theme.plain",
        "minicraft.settings.theme.hell",
    ];
    const TYPE_KEYS: [&str; 4] = [
        "minicraft.settings.type.island",
        "minicraft.settings.type.box",
        "minicraft.settings.type.mountain",
        "minicraft.settings.type.irregular",
    ];
    const MODE_KEYS: [&str; 4] = [
        "minicraft.settings.mode.survival",
        "minicraft.settings.mode.creative",
        "minicraft.settings.mode.hardcore",
        "minicraft.settings.mode.score",
    ];
    let on = localization.text("minicraft.display.entries.boolean.true");
    let off = localization.text("minicraft.display.entries.boolean.false");
    let mut entries = vec![
        (
            0,
            format!(
                "{}: {}{}",
                localization.text("minicraft.displays.world_gen.enter_world"),
                name,
                if selection == 0 { "_" } else { "" }
            ),
        ),
        (
            1,
            format!(
                "{}: {}",
                localization.text("minicraft.settings.mode"),
                localization.text(MODE_KEYS[settings.game_mode])
            ),
        ),
    ];
    if GameMode::from_index(settings.game_mode) == GameMode::Score {
        entries.push((
            2,
            format!(
                "{}: {}",
                localization.text("minicraft.settings.scoretime"),
                settings.score_minutes
            ),
        ));
    }
    entries.extend([
        (
            3,
            localization
                .text("minicraft.displays.world_gen.create_world")
                .to_owned(),
        ),
        (
            4,
            format!(
                "{}: {}",
                localization.text("minicraft.settings.size"),
                settings.world_size
            ),
        ),
        (
            5,
            format!(
                "{}: {}",
                localization.text("minicraft.settings.theme"),
                localization.text(THEME_KEYS[settings.theme])
            ),
        ),
        (
            6,
            format!(
                "{}: {}",
                localization.text("minicraft.settings.type"),
                localization.text(TYPE_KEYS[settings.terrain_type])
            ),
        ),
        (
            7,
            format!(
                "{}: {}",
                localization.text("minicraft.settings.quests"),
                if settings.quests { on } else { off }
            ),
        ),
        (
            8,
            format!(
                "{}: {}",
                localization.text("minicraft.settings.tutorials"),
                if settings.tutorials { on } else { off }
            ),
        ),
        (
            9,
            format!(
                "{}: {}{}",
                localization.text("minicraft.displays.world_gen.world_seed"),
                seed,
                if selection == 9 { "_" } else { "" }
            ),
        ),
    ]);
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.world_gen.title"),
        52,
    );
    let selected_row = entries
        .iter()
        .position(|(index, _)| *index == selection)
        .unwrap_or(0);
    let first = selected_row
        .saturating_sub(2)
        .min(entries.len().saturating_sub(5));
    let width = entries
        .iter()
        .skip(first)
        .take(5)
        .map(|(_, label)| label.chars().count())
        .max()
        .unwrap_or(0) as i32
        * 8;
    let x = (WIDTH as i32 - width) / 2;
    for (row, (index, label)) in entries.iter().skip(first).take(5).enumerate() {
        let y = 68 + row as i32 * 18;
        if *index == selection {
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(
                assets.font(),
                "<",
                x + minicraft_client::gfx::font::text_width(&label),
                y,
            );
        }
        screen.text_colored(
            assets.font(),
            label,
            x,
            y,
            if *index == selection {
                0xFFFFFF
            } else {
                0x999999
            },
        );
    }
}

fn render_worlds(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    settings: &Settings,
    worlds: &[WorldRecord],
    selection: usize,
    popup: Option<&WorldPopup>,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.world_select.select_world"),
        0,
    );
    if worlds.is_empty() {
        screen.centered_text(assets.font(), "NO WORLDS FOUND", 92);
    } else {
        let first = selection
            .saturating_sub(2)
            .min(worlds.len().saturating_sub(5));
        for (row, world) in worlds.iter().skip(first).take(5).enumerate() {
            let index = first + row;
            let label = &world.name;
            let y = 76 + row as i32 * 10;
            if index == selection {
                let width = minicraft_client::gfx::font::text_width(&label);
                let x = (WIDTH as i32 - width) / 2;
                screen.text(assets.font(), ">", x - 8, y);
                screen.text(assets.font(), "<", x + width, y);
            }
            screen.centered_text(assets.font(), label, y);
        }
        screen.centered_text(
            assets.font(),
            &localization.format(
                "minicraft.displays.world_select.display.world_version",
                &["2.2.4"],
            ),
            28,
        );
    }
    for (row, text) in [
        localization.format(
            "minicraft.displays.world_select.display.help.0",
            &[&settings.key_bindings.select],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.1",
            &[&settings.key_bindings.exit],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.2",
            &["SHIFT-C"],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.3",
            &["SHIFT-R"],
        ),
        localization.format(
            "minicraft.displays.world_select.display.help.4",
            &["SHIFT-D"],
        ),
    ]
    .iter()
    .enumerate()
    {
        let y = [
            HEIGHT as i32 - 60,
            HEIGHT as i32 - 40,
            HEIGHT as i32 - 24,
            HEIGHT as i32 - 16,
            HEIGHT as i32 - 8,
        ][row];
        screen.centered_text(assets.font(), text, y);
    }
    if let Some(dialog) = popup {
        render_world_popup(screen, assets, localization, worlds, selection, dialog);
    }
}

fn render_world_popup(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    worlds: &[WorldRecord],
    selection: usize,
    popup: &WorldPopup,
) {
    let (x, y, width, height) = (28, 56, WIDTH as i32 - 56, 80);
    screen.rect(x, y, width, height, 0x08080C);
    screen.frame(x, y, width, height, 0xC8C8C8);
    match popup.kind {
        WorldPopupKind::Copy | WorldPopupKind::Rename => {
            screen.centered_text(
                assets.font(),
                localization.text("minicraft.displays.world_select.popups.display.change"),
                y + 12,
            );
            screen.centered_text(assets.font(), &format!("{}{}", popup.text, "_"), y + 30);
        }
        WorldPopupKind::Delete => {
            screen.centered_text(assets.font(), "ARE YOU SURE YOU WANT TO DELETE", y + 12);
            screen.centered_text(
                assets.font(),
                worlds
                    .get(selection)
                    .map(|world| world.name.as_str())
                    .unwrap_or(""),
                y + 26,
            );
            screen.centered_text(assets.font(), "THIS CAN NOT BE UNDONE!", y + 40);
        }
    }
    screen.centered_text(assets.font(), "ENTER TO CONFIRM", y + height - 22);
    screen.centered_text(assets.font(), "ESC TO CANCEL", y + height - 12);
}

fn render_help(screen: &mut Screen, assets: &Assets, selection: usize) {
    screen.centered_text(assets.font(), "HELP", 66);
    let labels: Vec<String> = Book::ALL
        .iter()
        .map(|book| book.title().to_owned())
        .collect();
    render_centered_menu(screen, assets, &labels, selection, 91, 9);
}

fn render_book(screen: &mut Screen, assets: &Assets, book: Book, page: usize) {
    let pages = book.pages();
    let page = page.min(pages.len() - 1);
    screen.frame(6, 6, WIDTH as i32 - 12, HEIGHT as i32 - 12, 0xC8C8C8);
    screen.centered_text(assets.font(), book.title(), 13);
    for (row, line) in pages[page].iter().enumerate() {
        screen.text(assets.font(), line, 16, 31 + row as i32 * 10);
    }
    screen.centered_text(
        assets.font(),
        &format!("<  PAGE {} / {}  >", page + 1, pages.len()),
        HEIGHT as i32 - 18,
    );
}

fn render_achievements(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    achievements: &[Achievement],
    unlocked: &HashSet<String>,
    bindings: &KeyBindings,
    selection: usize,
) {
    screen.centered_text(assets.font(), "Achievements", 8);
    let score: u32 = achievements
        .iter()
        .filter(|achievement| unlocked.contains(&achievement.id))
        .map(|achievement| achievement.score)
        .sum();
    let score_label = localization.format(
        "minicraft.displays.achievements.display.score",
        &[&score.to_string()],
    );
    screen.text_colored(
        assets.font(),
        &score_label,
        (WIDTH as i32 - minicraft_client::gfx::font::text_width(&score_label)) / 2,
        32,
        0x999999,
    );
    if let Some(achievement) = achievements.get(selection) {
        let achieved = unlocked.contains(&achievement.id);
        let status = localization.text(if achieved {
            "minicraft.displays.achievements.display.achieved"
        } else {
            "minicraft.displays.achievements.display.not_achieved"
        });
        screen.text_colored(
            assets.font(),
            status,
            (WIDTH as i32 - minicraft_client::gfx::font::text_width(&status)) / 2,
            48,
            if achieved { 0x00FF00 } else { 0xFF3030 },
        );
    }
    let first = selection
        .saturating_sub(2)
        .min(achievements.len().saturating_sub(5));
    for (row, achievement) in achievements.iter().skip(first).take(5).enumerate() {
        let index = first + row;
        let y = 72 + row as i32 * 10;
        let name = localization.text(&achievement.id);
        let x = (WIDTH as i32 - minicraft_client::gfx::font::text_width(&name)) / 2;
        if index == selection {
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(
                assets.font(),
                "<",
                x + minicraft_client::gfx::font::text_width(&name),
                y,
            );
        }
        screen.text_colored(
            assets.font(),
            name,
            x,
            y,
            if unlocked.contains(&achievement.id) {
                0x00FF00
            } else {
                0xFFFFFF
            },
        );
    }
    if let Some(achievement) = achievements.get(selection) {
        render_app_menu_frame(screen, assets.hud(), 44, 128, 200, 32);
        let description = localization.text(&achievement.description);
        for (row, line) in crate::content::paginate(description, 23, 2)[0]
            .iter()
            .enumerate()
        {
            screen.text(assets.font(), line, 52, 136 + row as i32 * 10);
        }
    }
    let help = localization.format(
        "minicraft.displays.achievements.display.help",
        &[&bindings.down, &bindings.up],
    );
    screen.text_colored(
        assets.font(),
        &help,
        (WIDTH as i32 - minicraft_client::gfx::font::text_width(&help)) / 2,
        184,
        0x333333,
    );
}

fn render_controls(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    bindings: &KeyBindings,
    selection: usize,
    capture: Option<(usize, bool)>,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.key_input.title"),
        8,
    );
    for (index, label) in KeyBindings::LABELS.iter().enumerate() {
        let y = 24 + index as i32 * 10;
        if index == selection {
            screen.text(assets.font(), ">", 6, y);
            screen.text(assets.font(), "<", WIDTH as i32 - 14, y);
        }
        screen.text_colored(
            assets.font(),
            label,
            16,
            y,
            if index == selection {
                0xFFFFFF
            } else {
                0x999999
            },
        );
        let value = bindings.display_value(index);
        let x = WIDTH as i32 - 17 - minicraft_client::gfx::font::text_width(&value);
        screen.text_colored(assets.font(), &value, x, y, 0x999999);
    }
    let help = [
        localization
            .text("minicraft.displays.key_input.display.help.0")
            .to_owned(),
        localization
            .text("minicraft.displays.key_input.display.help.1")
            .to_owned(),
        localization
            .text("minicraft.displays.key_input.display.help.2")
            .to_owned(),
        localization.format(
            "minicraft.displays.key_input.display.help.3",
            &[&bindings.exit],
        ),
    ];
    for (line, y) in help.iter().zip([146, 156, 166, 176]) {
        screen.text_colored(assets.font(), line, 8, y, 0x999999);
    }
    if let Some((_, _append)) = capture {
        let message =
            localization.text("minicraft.displays.key_input.popup_display.press_key_sequence");
        let width = minicraft_client::gfx::font::text_width(&message) + 16;
        let x = (WIDTH as i32 - width) / 2;
        render_app_menu_frame(screen, assets.hud(), x, 82, width, 24);
        screen.text_colored(assets.font(), message, x + 8, 90, 0xFFFF00);
    }
}

fn render_control_guide(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    selection: usize,
    controller: bool,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.controls"),
        0,
    );
    screen.centered_text(
        assets.font(),
        localization.text(if controller {
            "minicraft.displays.controls.display.controller"
        } else {
            "minicraft.displays.controls.display.keyboard"
        }),
        10,
    );
    let count: usize = if controller { 16 } else { 23 };
    let first = selection.saturating_sub(8).min(count.saturating_sub(17));
    for (row, index) in (first..count).take(17).enumerate() {
        let key = format!(
            "minicraft.displays.controls.display.{}.{index:02}",
            if controller { "controller" } else { "keyboard" }
        );
        let label = localization.text(&key);
        let y = 20 + row as i32 * 8;
        if index == selection {
            let width = minicraft_client::gfx::font::text_width(&label);
            let x = (WIDTH as i32 - width) / 2;
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(assets.font(), "<", x + width, y);
        }
        screen.centered_text(assets.font(), label, y);
    }
    screen.centered_text(
        assets.font(),
        if controller {
            "CONTROLLER INPUTS"
        } else {
            "KEYBOARD INPUTS"
        },
        HEIGHT as i32 - 16,
    );
    screen.centered_text(assets.font(), "LEFT RIGHT TO SWITCH", HEIGHT as i32 - 8);
}

fn render_centered_menu(
    screen: &mut Screen,
    assets: &Assets,
    labels: &[String],
    selection: usize,
    start_y: i32,
    spacing: i32,
) {
    for (index, label) in labels.iter().enumerate() {
        let y = start_y + index as i32 * spacing;
        if index == selection {
            let width = minicraft_client::gfx::font::text_width(&label);
            let x = (WIDTH as i32 - width) / 2;
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(assets.font(), "<", x + width, y);
        }
        let x = (WIDTH as i32 - minicraft_client::gfx::font::text_width(&label)) / 2;
        screen.text_colored(
            assets.font(),
            label,
            x,
            y,
            if index == selection {
                0xFFFFFF
            } else {
                0x999999
            },
        );
    }
}

pub(crate) fn render_focus_nagger(screen: &mut Screen, assets: &Assets, ticks: u64) {
    let message = "Click to focus!";
    let width = minicraft_client::gfx::font::text_width(&message) + 16;
    let height = 24;
    let x = (WIDTH as i32 - width) / 2;
    let y = (HEIGHT as i32 - height) / 2;
    render_app_menu_frame(screen, assets.hud(), x, y, width, height);
    screen.text_colored(
        assets.font(),
        message,
        x + 8,
        y + 8,
        if (ticks / 20) & 1 == 0 {
            0x999999
        } else {
            0xFFFFFF
        },
    );
}

fn render_app_menu_frame(
    screen: &mut Screen,
    hud: &crate::gfx::Image,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
) {
    let right = x + width - 8;
    let bottom = y + height - 8;
    for draw_y in (y..=bottom).step_by(8) {
        for draw_x in (x..=right).step_by(8) {
            let edge_x = draw_x == x || draw_x == right;
            let edge_y = draw_y == y || draw_y == bottom;
            let source_x = if edge_x && edge_y {
                0
            } else if edge_y {
                8
            } else if edge_x {
                16
            } else {
                24
            };
            screen.blit_region_transformed(
                hud,
                source_x,
                48,
                8,
                8,
                draw_x,
                draw_y,
                draw_x == right,
                draw_y == bottom,
            );
        }
    }
}

fn render_languages(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    selection: usize,
    current_locale: &str,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.language_settings.title"),
        10,
    );
    let order = sorted_locale_indices();
    let first = selection
        .saturating_sub(7)
        .min(order.len().saturating_sub(15));
    for (row, locale_index) in order.iter().skip(first).take(15).enumerate() {
        let index = first + row;
        let (code, name) = LOCALES[*locale_index];
        let y = 30 + row as i32 * 10;
        if index == selection {
            screen.text(assets.font(), ">", 41, y);
            screen.text(assets.font(), "<", 239, y);
        }
        let color = if code.eq_ignore_ascii_case(current_locale) {
            if index == selection {
                0x00FF00
            } else {
                0x669966
            }
        } else if index == selection {
            0xFFFFFF
        } else {
            0x999999
        };
        screen.text_colored(assets.font(), name, 53, y, color);
    }
}

fn sorted_locale_indices() -> Vec<usize> {
    let mut indices: Vec<usize> = (0..LOCALES.len()).collect();
    indices.sort_by(|left, right| {
        LOCALES[*left]
            .1
            .to_lowercase()
            .cmp(&LOCALES[*right].1.to_lowercase())
    });
    indices
}

fn render_skins(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    skins: &[String],
    selection: usize,
    ticks: u64,
) {
    screen.centered_text(
        assets.font(),
        localization.text("minicraft.displays.skin"),
        16,
    );
    let (source_x, flip_x) = skin_preview_frame(ticks);
    screen.blit_region(
        assets.skin(),
        WIDTH as i32 / 2 - 8,
        40,
        source_x,
        assets.skin_row,
        16,
        16,
        flip_x,
    );
    let first = selection
        .saturating_sub(3)
        .min(skins.len().saturating_sub(8));
    for (row, skin) in skins.iter().skip(first).take(8).enumerate() {
        let index = first + row;
        let y = 82 + row as i32 * 10;
        if index == selection {
            let name_width = if skin.starts_with("minicraft.skin.") {
                localization.text(skin).chars().count() as i32 * 8
            } else {
                minicraft_client::gfx::font::text_width(&skin)
            };
            let x = (WIDTH as i32 - name_width) / 2;
            screen.text(assets.font(), ">", x - 8, y);
            screen.text(assets.font(), "<", x + name_width, y);
        }
        let name = if skin.starts_with("minicraft.skin.") {
            localization.text(skin)
        } else {
            skin
        };
        screen.centered_text(assets.font(), name, y);
    }
    screen.centered_text(assets.font(), "UP DOWN TO MOVE", HEIGHT as i32 - 17);
    screen.centered_text(assets.font(), "ENTER SELECT  ESC EXIT", HEIGHT as i32 - 9);
}

fn skin_preview_frame(ticks: u64) -> (usize, bool) {
    let sprite_index = (ticks / 40 % 8) as usize;
    let direction = sprite_index / 2;
    let frame = sprite_index % 2;
    match direction {
        0 => (0, frame == 1),
        1 => (16, frame == 1),
        2 => (32 + frame * 16, true),
        _ => (32 + frame * 16, false),
    }
}

fn random_seed() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as i64)
        .unwrap_or(0x100)
}

fn edit_text(value: &mut String, input: &Input, max_chars: usize) {
    if input.backspace {
        value.pop();
    }
    for character in &input.text {
        if value.chars().count() >= max_chars || character.is_control() {
            break;
        }
        value.push(*character);
    }
}

fn parse_world_seed(value: &str) -> Option<i64> {
    if value.is_empty() {
        return None;
    }
    if value.len() < 20
        && value
            .strip_prefix('-')
            .unwrap_or(value)
            .bytes()
            .all(|byte| byte.is_ascii_digit())
    {
        return value.parse().ok();
    }
    let mut seed = 1_125_899_906_842_597_i64;
    for character in value.encode_utf16() {
        seed = seed.wrapping_mul(31).wrapping_add(i64::from(character));
    }
    Some(seed)
}

fn play_options(settings: &Settings, mode: GameMode, score_minutes: usize) -> PlayOptions {
    PlayOptions {
        difficulty: settings.difficulty,
        mode,
        score_minutes,
        tutorials: settings.tutorials,
        quests: settings.quests,
        show_quests: settings.show_quests,
        custom_skin: settings.selected_skin != "minicraft.skin.paul",
    }
}

fn wrap(value: usize, direction: i32, count: usize) -> usize {
    (value as i32 + direction).rem_euclid(count as i32) as usize
}

fn active_packs(all: &[ResourcePack], enabled: &[String]) -> Vec<ResourcePack> {
    enabled
        .iter()
        .filter_map(|id| all.iter().find(|pack| &pack.id == id).cloned())
        .collect()
}

fn order_packs_for_menu(packs: &mut [ResourcePack], enabled: &[String]) {
    packs.sort_by_key(|pack| {
        enabled
            .iter()
            .rev()
            .position(|id| id == &pack.id)
            .map_or((1, usize::MAX), |priority| (0, priority))
    });
}

fn sync_enabled_pack_order(enabled: &mut Vec<String>, packs: &[ResourcePack]) {
    let was_enabled = enabled.iter().cloned().collect::<HashSet<_>>();
    enabled.clear();
    enabled.extend(
        packs
            .iter()
            .rev()
            .filter(|pack| was_enabled.contains(&pack.id))
            .map(|pack| pack.id.clone()),
    );
}

fn reload_resources(
    assets: &mut Assets,
    localization: &mut std::sync::Arc<Localization>,
    warnings: &mut Vec<String>,
    discovery_warnings: &[String],
    packs: &[ResourcePack],
    config: &Config,
) -> Result<(), String> {
    let active = active_packs(packs, &config.settings.resource_packs);
    *assets = Assets::load(&active)?;
    assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
    *localization = std::sync::Arc::new(Localization::load(&config.settings.locale, &active));
    warnings.clear();
    warnings.extend(discovery_warnings.iter().cloned());
    warnings.extend(assets.warnings.iter().cloned());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        Transition, order_packs_for_menu, parse_world_seed, skin_preview_frame,
        sync_enabled_pack_order, tick_options, tick_resource_packs, tick_worlds,
    };
    use crate::{
        audio::SoundEffect,
        config::Settings,
        input::Input,
        resource_pack::discover,
        world::{GameMode, WorldSpec},
        worlds::WorldRecord,
    };
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    #[test]
    fn skin_preview_cycles_java_direction_and_walk_frames() {
        assert_eq!(skin_preview_frame(0), (0, false));
        assert_eq!(skin_preview_frame(40), (0, true));
        assert_eq!(skin_preview_frame(80), (16, false));
        assert_eq!(skin_preview_frame(120), (16, true));
        assert_eq!(skin_preview_frame(160), (32, true));
        assert_eq!(skin_preview_frame(200), (48, true));
        assert_eq!(skin_preview_frame(240), (32, false));
        assert_eq!(skin_preview_frame(280), (48, false));
        assert_eq!(skin_preview_frame(320), (0, false));
    }

    #[test]
    fn world_seed_input_matches_java_numeric_and_string_rules() {
        assert_eq!(parse_world_seed(""), None);
        assert_eq!(parse_world_seed("-42"), Some(-42));
        assert_eq!(parse_world_seed("abc"), Some(-3_351_804_022_671_199_651));
    }

    #[test]
    fn title_exit_key_is_ignored_like_java_non_exit_display() {
        let mut selection = 0;
        let mut ticks = 0;
        assert!(matches!(
            super::tick_title(
                &mut selection,
                &mut ticks,
                &crate::input::Input {
                    exit: true,
                    ..crate::input::Input::default()
                },
                0,
            ),
            Transition::None
        ));
        assert_eq!(selection, 0);
    }

    #[test]
    fn display_sound_profiles_follow_java_entry_types_and_exit_order() {
        let select = Input {
            select: true,
            ..Input::default()
        };
        let right = Input {
            right_pressed: true,
            ..Input::default()
        };
        let exit = Input {
            exit: true,
            ..Input::default()
        };
        assert!(super::immediate_options_sounds(1, &select).is_empty());
        assert_eq!(
            super::immediate_options_sounds(1, &right),
            vec![SoundEffect::Select]
        );
        assert_eq!(
            super::immediate_options_sounds(4, &select),
            vec![SoundEffect::Confirm]
        );
        assert_eq!(
            super::immediate_options_sounds(0, &exit),
            vec![SoundEffect::Craft]
        );
        assert_eq!(
            super::immediate_select_and_close_sounds(&select),
            vec![SoundEffect::Confirm, SoundEffect::Craft]
        );
        assert_eq!(
            super::immediate_achievement_sounds(&exit),
            vec![SoundEffect::Craft, SoundEffect::Confirm]
        );
        assert!(super::immediate_control_guide_sounds(&right).is_empty());
        assert!(super::immediate_book_sounds(&right).is_empty());
        assert!(super::immediate_new_world_sounds(0, &select).is_empty());
        assert!(super::immediate_new_world_sounds(7, &select).is_empty());
        assert_eq!(
            super::immediate_new_world_sounds(7, &right),
            vec![SoundEffect::Select]
        );
        assert_eq!(
            super::immediate_new_world_sounds(3, &select),
            vec![SoundEffect::Confirm]
        );
        assert!(
            super::immediate_resource_pack_sounds(
                0,
                3,
                &Input {
                    left_pressed: true,
                    ..Input::default()
                }
            )
            .is_empty()
        );
        assert_eq!(
            super::immediate_resource_pack_sounds(0, 3, &right),
            vec![SoundEffect::Select]
        );
        assert_eq!(
            super::immediate_world_popup_sounds(true, &select),
            vec![SoundEffect::Confirm, SoundEffect::Craft]
        );
    }

    #[test]
    fn option_array_entries_ignore_select_like_java() {
        let mut settings = Settings::default();
        let sound = settings.sound;
        let show_quests = settings.show_quests;
        let hwa = settings.hwa;
        for selection in 1..=3 {
            let mut selection = selection;
            let (transition, changed) = tick_options(
                &mut selection,
                &Input {
                    select: true,
                    ..Input::default()
                },
                &mut settings,
            );
            assert!(matches!(transition, Transition::None));
            assert!(!changed);
        }
        assert_eq!(
            (settings.sound, settings.show_quests, settings.hwa),
            (sound, show_quests, hwa)
        );
    }

    #[test]
    fn world_gen_array_entries_ignore_select_like_java() {
        let mut settings = Settings::default();
        let original_quests = settings.quests;
        let original_tutorials = settings.tutorials;
        for selection in [7, 8] {
            let mut selection = selection;
            let result = super::tick_new_world(
                &mut selection,
                &mut "World".to_owned(),
                &mut String::new(),
                false,
                &Input {
                    select: true,
                    ..Input::default()
                },
                &mut settings,
            );
            assert!(matches!(result.0, Transition::None));
            assert!(!result.1);
        }
        assert_eq!(settings.quests, original_quests);
        assert_eq!(settings.tutorials, original_tutorials);
    }

    #[test]
    fn default_resource_pack_cannot_toggle_move_or_close_on_select() {
        let mut packs = Vec::new();
        let mut selection = 0;
        for input in [
            Input {
                select: true,
                ..Input::default()
            },
            Input {
                up_pressed: true,
                ..Input::default()
            },
            Input {
                right_pressed: true,
                ..Input::default()
            },
        ] {
            assert!(matches!(
                tick_resource_packs(&mut selection, &input, &mut packs),
                Transition::None
            ));
            assert!(super::immediate_resource_pack_sounds(selection, 0, &input).is_empty());
        }
        assert!(matches!(
            tick_resource_packs(
                &mut selection,
                &Input {
                    menu: true,
                    ..Input::default()
                },
                &mut packs
            ),
            Transition::PackInfo(0)
        ));
    }

    #[test]
    fn resource_pack_horizontal_priority_and_vertical_select_consumption_match_java() {
        let mut packs: Vec<_> = ["a", "b", "c"]
            .into_iter()
            .map(|id| crate::resource_pack::ResourcePack::parity_fixture(id, id, None))
            .collect();
        let mut selection = 0;
        let blocked = Input {
            left_pressed: true,
            down_pressed: true,
            select: true,
            exit: true,
            ..Input::default()
        };
        assert!(super::immediate_resource_pack_sounds(0, 3, &blocked).is_empty());
        assert!(matches!(
            tick_resource_packs(&mut selection, &blocked, &mut packs),
            Transition::None
        ));
        assert_eq!(selection, 0);
        let moving = Input {
            down_pressed: true,
            select: true,
            ..Input::default()
        };
        assert_eq!(
            super::immediate_resource_pack_sounds(0, 3, &moving),
            vec![SoundEffect::Select]
        );
        assert!(matches!(
            tick_resource_packs(&mut selection, &moving, &mut packs),
            Transition::None
        ));
        assert_eq!(selection, 1);
        let right = Input {
            right_pressed: true,
            left_pressed: true,
            select: true,
            ..Input::default()
        };
        assert!(matches!(
            tick_resource_packs(&mut selection, &right, &mut packs),
            Transition::MovePack(2)
        ));
        assert_eq!(packs[2].id, "b");
    }

    #[test]
    fn resource_pack_info_is_silent_until_exit_and_preserves_selection() {
        let mut packs = vec![crate::resource_pack::ResourcePack::parity_fixture(
            "a", "Alpha", None,
        )];
        let mut info = false;
        let mut selection = 0;
        assert_eq!(
            super::resource_pack_menu_parity_step(
                &mut selection,
                &Input {
                    menu: true,
                    ..Input::default()
                },
                &mut packs,
                &mut info
            ),
            ("resource_pack_info", "open", Vec::new())
        );
        assert_eq!(
            super::resource_pack_menu_parity_step(
                &mut selection,
                &Input {
                    select: true,
                    down_pressed: true,
                    ..Input::default()
                },
                &mut packs,
                &mut info
            ),
            ("resource_pack_info", "none", Vec::new())
        );
        assert_eq!(
            super::resource_pack_menu_parity_step(
                &mut selection,
                &Input {
                    exit: true,
                    ..Input::default()
                },
                &mut packs,
                &mut info
            ),
            ("resource_packs", "close", vec![SoundEffect::Craft])
        );
        assert!(!info);
        assert_eq!(selection, 0);
    }

    #[test]
    fn disabled_resource_packs_reorder_without_wrapping_like_java() {
        let root = std::env::temp_dir().join(format!(
            "minicraft-rust-pack-order-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        for id in ["a", "b", "c"] {
            let directory = root.join("resourcepacks").join(id);
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                directory.join("pack.json"),
                format!(r#"{{"pack_format":1,"name":"{id}"}}"#),
            )
            .unwrap();
        }
        let (mut packs, warnings) = discover(&root);
        assert!(warnings.is_empty());
        let mut enabled = vec!["a".to_owned(), "c".to_owned()];
        order_packs_for_menu(&mut packs, &enabled);
        assert_eq!(
            packs
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            ["c", "a", "b"]
        );

        let mut selection = 2;
        let transition = tick_resource_packs(
            &mut selection,
            &Input {
                left_pressed: true,
                ..Input::default()
            },
            &mut packs,
        );
        assert!(matches!(transition, Transition::MovePack(1)));
        assert_eq!(selection, 1);
        assert_eq!(
            packs
                .iter()
                .map(|pack| pack.id.as_str())
                .collect::<Vec<_>>(),
            ["c", "b", "a"]
        );
        sync_enabled_pack_order(&mut enabled, &packs);
        assert_eq!(enabled, ["a", "c"]);

        selection = 0;
        assert!(matches!(
            tick_resource_packs(
                &mut selection,
                &Input {
                    left_pressed: true,
                    ..Input::default()
                },
                &mut packs,
            ),
            Transition::None
        ));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn world_select_shift_actions_open_java_style_confirmation_popups() {
        let worlds = vec![WorldRecord {
            name: "ORIGINAL".to_owned(),
            seed: 1,
            spec: WorldSpec::default(),
            mode: GameMode::Survival,
            score_minutes: 20,
            directory: std::path::PathBuf::new(),
        }];
        let mut selection = 0;
        let mut popup = None;
        assert!(matches!(
            tick_worlds(
                &mut selection,
                &mut popup,
                &Input {
                    world_rename: true,
                    ..Input::default()
                },
                &worlds,
            ),
            Transition::None
        ));
        assert_eq!(
            popup.as_ref().map(|popup| popup.text.as_str()),
            Some("ORIGINAL")
        );
        assert!(matches!(
            tick_worlds(
                &mut selection,
                &mut popup,
                &Input {
                    select: true,
                    ..Input::default()
                },
                &worlds,
            ),
            Transition::RenameWorld(0, name) if name == "ORIGINAL"
        ));
    }
}
