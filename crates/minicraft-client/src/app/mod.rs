use std::path::Path;
use std::{
    collections::HashSet,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::core::{display_stack::DisplayStack, game_loop::GameLoopClock};

mod display_navigation;
mod menu_build;
mod menu_render;
use display_navigation::navigate as navigate_display;
use menu_build::*;
use menu_render::*;
pub(crate) use menu_render::{render_focus_nagger, render_resource_pack_parity, render_title};
use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};

use crate::{
    assets::{Assets, skin_options},
    audio::{Audio, SoundEffect, validate_embedded_assets, validate_pcm_stream},
    gfx::{HEIGHT, Screen, WIDTH},
    input_full::{Gamepad, key_name},
    io::{Config, Input, KeyBindings, Settings},
    presenter::Presenter,
    progression::{Book, achievements},
    resource::localization::{LOCALES, Localization},
    resource_pack::{ResourcePack, discover, display::render as render_resource_packs},
    world::{GameMode, PlayOptions, World, WorldAction, WorldSpec},
    worlds::{
        WorldRecord, copy_record, create_named as create_world_record, delete_record,
        load as load_world_records, load_state, rename_record, save_state,
    },
};

#[cfg(windows)]
fn set_embedded_window_icon(window: &Window) {
    use std::ffi::c_void;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetModuleHandleW(name: *const u16) -> *mut c_void;
    }
    #[link(name = "user32")]
    unsafe extern "system" {
        fn LoadImageW(
            instance: *mut c_void,
            name: *const u16,
            image_type: u32,
            width: i32,
            height: i32,
            flags: u32,
        ) -> *mut c_void;
        fn SendMessageW(window: *mut c_void, message: u32, wparam: usize, lparam: isize) -> isize;
    }

    // Resource ID 1 comes from build.rs. Windows chooses the appropriate ICO
    // layer for the title bar and taskbar; no external file is needed at run time.
    unsafe {
        let module = GetModuleHandleW(std::ptr::null());
        // MAKEINTRESOURCEW(1): the low pointer bits encode an ID, not alignment.
        let resource = std::ptr::without_provenance::<u16>(1);
        let hwnd = window.get_window_handle();
        let small = LoadImageW(module, resource, 1, 16, 16, 0);
        let large = LoadImageW(module, resource, 1, 32, 32, 0);
        if !small.is_null() {
            SendMessageW(hwnd, 0x0080, 0, small as isize);
        }
        if !large.is_null() {
            SendMessageW(hwnd, 0x0080, 1, large as isize);
        }
    }
}

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
    if crate::progression::tutorials()?.len() != 5
        || crate::progression::quests()?
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

/// CLI dispatch for the compatibility client binary.
pub fn run_cli(arguments: &[String]) -> Result<(), String> {
    if arguments.iter().any(|value| value == "--audio-probe") {
        crate::audio::device_probe()
    } else if arguments.iter().any(|value| value == "--self-check") {
        self_check(arguments)
    } else if let Some(index) = arguments.iter().position(|value| value == "--server") {
        let default_address = minicraft_server::default_server_address();
        let address = arguments
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .map(String::as_str)
            .unwrap_or(&default_address);
        minicraft_server::run_server(address)
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--multiplayer-probe")
    {
        arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| "--multiplayer-probe requires an address and username".to_owned())
            .and_then(|(address, username)| crate::network::run_client_probe(address, username))
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--render-preview")
    {
        arguments
            .get(index + 1)
            .ok_or_else(|| "--render-preview requires an output PNG path".to_owned())
            .and_then(|path| render_preview(arguments, std::path::Path::new(path)))
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--render-world-preview")
    {
        arguments
            .get(index + 1)
            .ok_or_else(|| "--render-world-preview requires an output PNG path".to_owned())
            .and_then(|path| render_world_preview(arguments, std::path::Path::new(path)))
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--render-ui-preview")
    {
        arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| {
                "--render-ui-preview requires a screen name and output PNG path".to_owned()
            })
            .and_then(|(name, path)| render_ui_preview(arguments, name, std::path::Path::new(path)))
    } else {
        run(arguments)
    }
}

impl State {
    #[cfg(test)]
    pub(super) fn title(selection: usize, ticks: u64) -> Self {
        // Compatibility index is TITLE_KEYS order (0=Play); menu has 2 leading rows.
        let menu = build_title_menu(&Localization::load("en-us", &[]), 1, selection + 2);
        State::Title {
            menu,
            ticks,
            splash: "Now with skins!",
        }
    }
}

enum State {
    Title {
        menu: crate::screen::menu::Menu,
        ticks: u64,
        splash: &'static str,
    },
    Options {
        menu: crate::screen::menu::Menu,
    },
    PlayMenu {
        menu: crate::screen::menu::Menu,
    },
    Languages {
        menu: crate::screen::menu::Menu,
    },
    Controls {
        menu: crate::screen::menu::Menu,
        capture: Option<(usize, bool)>,
        keyboard: bool,
    },
    NewWorld {
        menu: crate::screen::menu::Menu,
        name: String,
        seed: String,
        from_play_menu: bool,
    },
    Worlds {
        menu: crate::screen::menu::Menu,
        popup: Option<WorldPopup>,
    },
    Help {
        menu: crate::screen::menu::Menu,
    },
    Book {
        menu: crate::screen::menu::Menu,
        book: Book,
        page: usize,
    },
    Achievements {
        menu: crate::screen::menu::Menu,
        selection: usize,
    },
    ControlGuide {
        menu: crate::screen::menu::Menu,
        controller: bool,
    },
    Skins {
        menu: crate::screen::menu::Menu,
        ticks: u64,
    },
    ResourcePacks {
        menu: crate::screen::menu::Menu,
    },
    ResourcePackInfo {
        menu: crate::screen::menu::Menu,
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
    if prune_missing_enabled_packs(&mut config.settings.resource_packs, &packs) {
        let _ = config.save();
    }
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
    #[cfg(windows)]
    set_embedded_window_icon(&window);
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
    let mut game = crate::core::game::Game::default();
    displays.open(State::Title {
        menu: build_title_menu(&localization, world_records.len(), 0),
        ticks: 0,
        splash: crate::screen::title_display::random_splash(),
    });
    displays.update();
    let audio = Audio::default();
    let mut gamepad = Gamepad::default();
    let mut fixed_step = GameLoopClock::new();
    let mut input_buffer = crate::input_full::InputBuffer::default();

    'game: while window.is_open() {
        let focused = window.is_active();
        let raw_keys = if focused {
            window.get_keys_pressed(KeyRepeat::No)
        } else {
            Vec::new()
        };
        let frame_input = if focused {
            crate::input_full::poll_input(
                &window,
                &config.settings.key_bindings,
                &raw_keys,
                &mut gamepad,
            )
        } else {
            Input::default()
        };
        let want_screenshot = frame_input.screenshot;
        let want_fullscreen = frame_input.fullscreen;
        if let Some(State::Playing { world, .. }) = displays.requested_mut() {
            world
                .set_notification_context(localization.clone(), &config.settings.key_bindings.exit);
            world.prepare_quest_layout(&localization);
        }
        // Frame-level menu sound prediction is retired; tick handlers emit MenuEvent.
        let menu_sounds = match displays.requested().expect("application screen") {
            State::Playing { world, .. } => world.immediate_menu_sounds(&frame_input),
            _ => Vec::new(),
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
                State::Title { menu, ticks, .. } => {
                    let (transition, event) = tick_title_menu_event(
                        menu,
                        ticks,
                        &input,
                        world_records.len(),
                        &localization,
                    );
                    menu_events.push(event);
                    transition
                }
                State::Options { menu } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let result = tick_options_menu(menu, &input, &mut config.settings);
                    menu_events.push(if input.exit {
                        MenuEvent::Closed
                    } else if previous != menu.selection || result.1 {
                        MenuEvent::Moved
                    } else if !matches!(result.0, Transition::None) {
                        MenuEvent::Activated
                    } else {
                        MenuEvent::None
                    });
                    settings_changed = result.1;
                    result.0
                }
                State::PlayMenu { menu } => {
                    let (transition, event) = tick_play_menu_event_menu(menu, &input);
                    menu_events.push(event);
                    transition
                }
                State::NewWorld {
                    menu,
                    name,
                    seed,
                    from_play_menu,
                } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let result = tick_new_world_menu(
                        menu,
                        name,
                        seed,
                        *from_play_menu,
                        &input,
                        &mut config.settings,
                        world_records.len(),
                    );
                    menu_events.push(if input.exit {
                        MenuEvent::Closed
                    } else if previous != menu.selection || result.1 {
                        MenuEvent::Moved
                    } else if matches!(result.0, Transition::CreateWorld(..)) {
                        MenuEvent::Activated
                    } else {
                        MenuEvent::None
                    });
                    settings_changed = result.1;
                    result.0
                }
                State::Worlds { menu, popup } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let had_popup = popup.is_some();
                    let result = tick_worlds_menu(menu, popup, &input, &world_records);
                    if had_popup {
                        if input.exit {
                            menu_events.push(MenuEvent::Closed);
                        } else if matches!(
                            result,
                            Transition::CopyWorld(..)
                                | Transition::RenameWorld(..)
                                | Transition::DeleteWorld(..)
                        ) {
                            menu_events.extend([MenuEvent::Activated, MenuEvent::Closed]);
                        }
                    } else if input.exit {
                        menu_events.push(MenuEvent::Closed);
                    } else if previous != menu.selection {
                        menu_events.push(MenuEvent::Moved);
                    } else if !matches!(result, Transition::None) {
                        menu_events.push(MenuEvent::Activated);
                    }
                    result
                }
                State::Help { menu } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let result = tick_help_menu(menu, &input);
                    menu_events.push(if input.exit {
                        MenuEvent::Closed
                    } else if previous != menu.selection {
                        MenuEvent::Moved
                    } else if !matches!(result, Transition::None) {
                        MenuEvent::Activated
                    } else {
                        MenuEvent::None
                    });
                    result
                }
                State::Book { menu, book, page } => {
                    use crate::screen::menu::MenuEvent;
                    let result = tick_book_menu(menu, book, page, &input);
                    if input.exit || input.menu || matches!(result, Transition::Help) {
                        menu_events.push(MenuEvent::Closed);
                    }
                    result
                }
                State::Achievements { menu, selection } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let result = tick_achievements_menu(menu, &input);
                    *selection = menu.selection;
                    if input.exit {
                        menu_events.extend([MenuEvent::Closed, MenuEvent::Activated]);
                    } else if previous != menu.selection {
                        menu_events.push(MenuEvent::Moved);
                    }
                    result
                }
                State::Controls {
                    menu,
                    capture,
                    keyboard,
                } => {
                    use crate::screen::menu::MenuEvent;
                    let capturing = capture.is_some();
                    let previous = menu.selection;
                    let previous_kb = *keyboard;
                    let result = tick_controls_menu(menu, capture, keyboard, &input, &raw_keys);
                    if previous_kb != *keyboard {
                        *menu = build_controls_menu(&localization, *keyboard, 0);
                    }
                    if capturing {
                        // no confirm sound while capturing
                    } else if input.exit
                        || matches!(
                            result,
                            Transition::SetBinding(..) | Transition::ResetBindings
                        )
                    {
                        menu_events.push(MenuEvent::Closed);
                    } else if previous != menu.selection || previous_kb != *keyboard {
                        menu_events.push(MenuEvent::Moved);
                    }
                    result
                }
                State::ControlGuide { menu, controller } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = (menu.selection, *controller);
                    let result = tick_control_guide_menu(menu, controller, &input);
                    if previous.1 != *controller {
                        *menu = build_control_guide_menu(&localization, *controller, 0);
                    }
                    menu_events.push(if input.exit {
                        MenuEvent::Closed
                    } else if previous != (menu.selection, *controller) {
                        MenuEvent::Moved
                    } else {
                        MenuEvent::None
                    });
                    result
                }
                State::Languages { menu } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let result = tick_languages_menu(menu, &input);
                    if input.exit {
                        menu_events.push(MenuEvent::Closed);
                    } else if previous != menu.selection {
                        menu_events.push(MenuEvent::Moved);
                    } else if matches!(result, Transition::ChangeLocale(_)) {
                        menu_events.extend([MenuEvent::Activated, MenuEvent::Closed]);
                    }
                    result
                }
                State::Skins { menu, ticks } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let result = tick_skins_menu(menu, ticks, &input);
                    match result {
                        Transition::CancelSkins => menu_events.push(MenuEvent::Closed),
                        Transition::ChangeSkin(_) => {
                            menu_events.extend([MenuEvent::Activated, MenuEvent::Closed])
                        }
                        _ => {
                            if menu.selection != previous {
                                menu_events.push(MenuEvent::Moved);
                            }
                        }
                    }
                    result
                }
                State::ResourcePacks { menu } => {
                    use crate::screen::menu::MenuEvent;
                    let previous = menu.selection;
                    let result = tick_resource_packs_menu(menu, &input, &mut packs);
                    if input.exit {
                        menu_events.push(MenuEvent::Closed);
                    } else if matches!(result, Transition::PackInfo(_)) {
                        // silent
                    } else if matches!(result, Transition::TogglePack(_)) {
                        menu_events.push(MenuEvent::Activated);
                    } else if previous != menu.selection
                        || matches!(result, Transition::MovePack(_))
                    {
                        menu_events.push(MenuEvent::Moved);
                    }
                    result
                }
                State::ResourcePackInfo { menu } => {
                    use crate::screen::menu::MenuEvent;
                    if input.exit {
                        menu_events.push(MenuEvent::Closed);
                        Transition::ClosePackInfo(menu.selection)
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
                use crate::screen::menu::MenuEvent;
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
                            menu: build_title_menu(&localization, world_records.len(), 0),
                            ticks: 0,
                            splash: crate::screen::title_display::random_splash(),
                        },
                    );
                }
                Transition::PlayMenu => navigate_display(
                    &mut displays,
                    State::PlayMenu {
                        menu: build_play_menu(&localization, !world_records.is_empty(), 0),
                    },
                ),
                Transition::NewWorld(from_play_menu) => navigate_display(
                    &mut displays,
                    State::NewWorld {
                        menu: build_new_world_menu(
                            &localization,
                            &config.settings,
                            "",
                            "",
                            0,
                            world_records.len(),
                        ),
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
                    let mut world = World::new_with_generation(
                        seed,
                        spec,
                        play_options(&config.settings, mode, config.settings.score_minutes),
                        record.generation_version,
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
                            menu: build_worlds_menu(&world_records, &localization, 0),
                            popup: None,
                        },
                    );
                }
                Transition::LoadWorld(selection) => {
                    let record = world_records[selection].clone();
                    let mut world = load_state(&record)?.unwrap_or_else(|| {
                        World::new_with_generation(
                            record.seed,
                            record.spec,
                            play_options(&config.settings, record.mode, record.score_minutes),
                            record.generation_version,
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
                            menu: build_worlds_menu(&world_records, &localization, selection),
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
                            menu: build_worlds_menu(&world_records, &localization, selection),
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
                                menu: build_new_world_menu(
                                    &localization,
                                    &config.settings,
                                    "",
                                    "",
                                    0,
                                    0,
                                ),
                                name: String::new(),
                                seed: String::new(),
                                from_play_menu: true,
                            }
                        } else {
                            State::Worlds {
                                menu: build_worlds_menu(
                                    &world_records,
                                    &localization,
                                    selection.min(world_records.len().saturating_sub(1)),
                                ),
                                popup: None,
                            }
                        },
                    );
                }
                Transition::Options => navigate_display(
                    &mut displays,
                    State::Options {
                        menu: build_options_menu(&localization, &config.settings, 0),
                    },
                ),
                Transition::Controls => navigate_display(
                    &mut displays,
                    State::Controls {
                        menu: build_controls_menu(&localization, true, 0),
                        capture: None,
                        keyboard: true,
                    },
                ),
                Transition::ControlGuide => navigate_display(
                    &mut displays,
                    State::ControlGuide {
                        menu: build_control_guide_menu(&localization, false, 0),
                        controller: false,
                    },
                ),
                Transition::CaptureBinding(selection, append) => navigate_display(
                    &mut displays,
                    State::Controls {
                        menu: build_controls_menu(&localization, true, selection),
                        capture: Some((selection, append)),
                        keyboard: true,
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
                            menu: build_controls_menu(&localization, true, selection),
                            capture: None,
                            keyboard: true,
                        },
                    );
                }
                Transition::ResetBindings => {
                    config.settings.key_bindings = KeyBindings::default();
                    config.save()?;
                    navigate_display(
                        &mut displays,
                        State::Controls {
                            menu: build_controls_menu(&localization, true, 0),
                            capture: None,
                            keyboard: true,
                        },
                    );
                }
                Transition::Help => navigate_display(
                    &mut displays,
                    State::Help {
                        menu: build_help_menu(&localization, 0),
                    },
                ),
                Transition::Book(book) => navigate_display(
                    &mut displays,
                    State::Book {
                        menu: build_book_menu(book, 0),
                        book,
                        page: 0,
                    },
                ),
                Transition::Achievements => navigate_display(
                    &mut displays,
                    State::Achievements {
                        menu: build_achievements_menu(
                            &localization,
                            &achievements,
                            &unlocked_achievements,
                            0,
                        ),
                        selection: 0,
                    },
                ),
                Transition::Languages => {
                    let current_index = LOCALES
                        .iter()
                        .position(|(code, _)| *code == config.settings.locale)
                        .unwrap_or(0);
                    let selection = sorted_locale_indices()
                        .iter()
                        .position(|index| *index == current_index)
                        .unwrap_or(0);
                    navigate_display(
                        &mut displays,
                        State::Languages {
                            menu: build_languages_menu(&localization, selection),
                        },
                    );
                }
                Transition::Skins => {
                    let selection = skins
                        .iter()
                        .position(|skin| *skin == config.settings.selected_skin)
                        .unwrap_or(0);
                    navigate_display(
                        &mut displays,
                        State::Skins {
                            menu: build_skins_menu(&skins, selection),
                            ticks: 0,
                        },
                    );
                }
                Transition::CancelSkins => {
                    assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
                    navigate_display(
                        &mut displays,
                        State::Title {
                            menu: build_title_menu(&localization, world_records.len(), 4),
                            ticks: 0,
                            splash: crate::screen::title_display::random_splash(),
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
                            menu: build_title_menu(&localization, world_records.len(), 4),
                            ticks: 0,
                            splash: crate::screen::title_display::random_splash(),
                        },
                    );
                }
                Transition::ResourcePacks => navigate_display(
                    &mut displays,
                    State::ResourcePacks {
                        menu: build_resource_packs_menu(&packs, &config.settings.resource_packs, 0),
                    },
                ),
                Transition::PackInfo(selection) => navigate_display(
                    &mut displays,
                    State::ResourcePackInfo {
                        menu: build_resource_packs_menu(
                            &packs,
                            &config.settings.resource_packs,
                            selection,
                        ),
                    },
                ),
                Transition::ClosePackInfo(selection) => navigate_display(
                    &mut displays,
                    State::ResourcePacks {
                        menu: build_resource_packs_menu(
                            &packs,
                            &config.settings.resource_packs,
                            selection,
                        ),
                    },
                ),
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
                    navigate_display(
                        &mut displays,
                        State::ResourcePacks {
                            menu: build_resource_packs_menu(
                                &packs,
                                &config.settings.resource_packs,
                                selection,
                            ),
                        },
                    );
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
                    navigate_display(
                        &mut displays,
                        State::ResourcePacks {
                            menu: build_resource_packs_menu(
                                &packs,
                                &config.settings.resource_packs,
                                selection,
                            ),
                        },
                    );
                }
                Transition::ChangeLocale(selection) => {
                    let index = sorted_locale_indices()[selection];
                    config.settings.locale = LOCALES[index].0.to_owned();
                    config.save()?;
                    let active = active_packs(&packs, &config.settings.resource_packs);
                    localization =
                        std::sync::Arc::new(Localization::load(&config.settings.locale, &active));
                    navigate_display(
                        &mut displays,
                        State::Options {
                            menu: build_options_menu(&localization, &config.settings, 0),
                        },
                    );
                }
                Transition::Quit => break 'game,
            }
        }

        screen.clear(0);
        match displays.current_mut().expect("application screen") {
            State::Title {
                menu,
                ticks,
                splash,
            } => render_title_menu(&mut screen, &assets, &localization, menu, *ticks, splash),
            State::Options { menu } => {
                // Rebuild if settings-driven labels may be stale after entry edits.
                menu.render(&mut screen, assets.hud(), assets.font());
            }
            State::PlayMenu { menu } => {
                // Keep world-count gated menu contents; rebuild if count changed.
                if menu.entries.len() != if world_records.is_empty() { 1 } else { 2 } {
                    *menu =
                        build_play_menu(&localization, !world_records.is_empty(), menu.selection);
                }
                menu.render(&mut screen, assets.hud(), assets.font());
            }
            State::NewWorld { menu, .. } => {
                render_new_world_menu(&mut screen, &assets, menu);
            }
            State::Worlds { menu, popup } => render_worlds_menu(
                &mut screen,
                &assets,
                &localization,
                &config.settings,
                &world_records,
                menu,
                popup.as_ref(),
            ),
            State::Help { menu } => {
                menu.render(&mut screen, assets.hud(), assets.font());
            }
            State::Book { menu, book, page } => {
                render_book_menu(&mut screen, &assets, *book, *page, menu);
            }
            State::Achievements { menu, selection } => {
                *selection = menu.selection;
                render_achievements(
                    &mut screen,
                    &assets,
                    &localization,
                    &achievements,
                    &unlocked_achievements,
                    &config.settings.key_bindings,
                    menu.selection,
                );
            }
            State::Controls {
                menu,
                capture,
                keyboard,
            } => {
                render_controls_menu(
                    &mut screen,
                    &assets,
                    &localization,
                    menu,
                    *keyboard,
                    *capture,
                );
            }
            State::Languages { menu } => {
                render_languages_menu(
                    &mut screen,
                    &assets,
                    &localization,
                    &config.settings.locale,
                    menu.selection,
                );
            }
            State::ControlGuide { menu, controller } => {
                render_controls_menu(
                    &mut screen,
                    &assets,
                    &localization,
                    menu,
                    !*controller,
                    None,
                );
            }
            State::Skins { menu, ticks } => {
                // Preview the selection without requiring a preview transition (Java tick+render).
                assets.select_skin(&skins[menu.selection], &config.game_dir)?;
                render_skins_menu(
                    &mut screen,
                    &assets,
                    &localization,
                    &skins,
                    menu.selection,
                    *ticks,
                );
            }
            State::ResourcePacks { menu } => {
                render_resource_packs(
                    &mut screen,
                    &assets,
                    &localization,
                    &packs,
                    &config.settings.resource_packs,
                    menu.selection,
                    &config.settings.key_bindings,
                );
            }
            State::Playing { world, .. } => {
                world.render_with_ready(&mut screen, &assets, &localization, game.ready())
            }
            State::ResourcePackInfo { menu } => {
                menu.render(&mut screen, assets.hud(), assets.font());
            }
        }
        if !focused {
            // Java renderFocusNagger sets Updater.paused after the unfocused update.
            if let Some(State::Playing { world, .. }) = displays.current_mut() {
                world.mark_unfocused_paused();
            }
            let focus_ticks = match displays.current() {
                Some(State::Playing { world, .. }) => world.calendar_tick(),
                _ => u64::from(game.calendar_tick()),
            };
            render_focus_nagger(&mut screen, &assets, focus_ticks);
        }
        presenter.present(&mut window, screen.pixels())?;
        // Java SCREENSHOT (F2) and FULLSCREEN (F11) after render.
        if want_screenshot {
            let path = config.game_dir.join("screenshot.png");
            screen
                .save_png(&path)
                .map_err(|error| format!("screenshot failed: {error}"))?;
        }
        // F11 FULLSCREEN is bound; minifb 0.27 has no portable fullscreen API.
        let _ = want_fullscreen;
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
        "options" => {
            let menu = build_options_menu(&localization, &config.settings, 5);
            menu.render(&mut screen, assets.hud(), assets.font());
        }
        "play" => render_play_menu(&mut screen, &assets, &localization, worlds.len(), 0),
        "new-world" => {
            let menu =
                build_new_world_menu(&localization, &config.settings, "", "", 0, worlds.len());
            render_new_world_menu(&mut screen, &assets, &menu);
        }
        "worlds" => {
            let menu = build_worlds_menu(&worlds, &localization, 0);
            render_worlds_menu(
                &mut screen,
                &assets,
                &localization,
                &config.settings,
                &worlds,
                &menu,
                None,
            );
        }
        "help" => {
            let menu = build_help_menu(&localization, 0);
            menu.render(&mut screen, assets.hud(), assets.font());
        }
        "book" => {
            let menu = build_book_menu(Book::Instructions, 0);
            render_book_menu(&mut screen, &assets, Book::Instructions, 0, &menu);
        }
        "achievements" => render_achievements(
            &mut screen,
            &assets,
            &localization,
            &achievements,
            &HashSet::new(),
            &config.settings.key_bindings,
            0,
        ),
        "controls" => {
            let menu = build_controls_menu(&localization, true, 0);
            render_controls_menu(&mut screen, &assets, &localization, &menu, true, None);
        }
        "skins" => render_skins_menu(&mut screen, &assets, &localization, &skins, 0, 160),
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
    } else if input.cursor_up_pressed
        || input.cursor_down_pressed
        || input.cursor_left_pressed
        || input.cursor_right_pressed
    {
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
    } else if input.cursor_up_pressed
        || input.cursor_down_pressed
        || (selection < 4 && (input.cursor_left_pressed || input.cursor_right_pressed))
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
    } else if input.cursor_up_pressed || input.cursor_down_pressed {
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
    } else if input.cursor_up_pressed
        || input.cursor_down_pressed
        || (matches!(selection, 1 | 2 | 4 | 5 | 6 | 7 | 8)
            && (input.cursor_left_pressed || input.cursor_right_pressed))
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
    if input.cursor_right_pressed {
        return if selection + 1 < pack_count {
            vec![SoundEffect::Select]
        } else {
            Vec::new()
        };
    }
    if input.cursor_left_pressed {
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
        if input.cursor_up_pressed != input.cursor_down_pressed {
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

fn tick_title(
    selection: &mut usize,
    ticks: &mut u64,
    input: &Input,
    world_count: usize,
) -> Transition {
    tick_title_event(selection, ticks, input, world_count).0
}

fn tick_title_menu_event(
    menu: &mut crate::screen::menu::Menu,
    ticks: &mut u64,
    input: &Input,
    world_count: usize,
    _localization: &Localization,
) -> (Transition, crate::screen::menu::MenuEvent) {
    use crate::screen::menu::MenuEvent;
    *ticks = ticks.wrapping_add(1);
    let menu_input = menu_input_from(input);
    let events = menu.tick(&menu_input);
    let event = events
        .into_iter()
        .find(|e| *e != MenuEvent::None)
        .unwrap_or(MenuEvent::None);
    if event != MenuEvent::Activated {
        return (Transition::None, event);
    }
    // Entries 0–1 are non-selectable; first selectable is Play at index 2.
    let transition = match crate::screen::title_display::action_for_menu_selection(menu.selection) {
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

fn tick_title_event(
    selection: &mut usize,
    ticks: &mut u64,
    input: &Input,
    world_count: usize,
) -> (Transition, crate::screen::menu::MenuEvent) {
    // Compatibility entry used by parity probes that pass a bare selection index.
    use crate::screen::menu::{MenuEvent, tick_selectable};
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

fn tick_play_menu_event_menu(
    menu: &mut crate::screen::menu::Menu,
    input: &Input,
) -> (Transition, crate::screen::menu::MenuEvent) {
    use crate::screen::menu::MenuEvent;
    let menu_input = menu_input_from(input);
    let events = menu.tick(&menu_input);
    let exited = menu_input.exit;
    let event = events
        .into_iter()
        .find(|e| *e != MenuEvent::None)
        .unwrap_or(MenuEvent::None);
    let transition = if exited || event == MenuEvent::Closed {
        Transition::Title
    } else if event == MenuEvent::Activated {
        match menu.selection {
            0 if menu.entries.len() > 1 => Transition::Worlds,
            0 => Transition::NewWorld(true),
            _ => Transition::NewWorld(true),
        }
    } else {
        Transition::None
    };
    (transition, event)
}

#[allow(dead_code)]
fn tick_play_menu_event(
    selection: &mut usize,
    input: &Input,
) -> (Transition, crate::screen::menu::MenuEvent) {
    use crate::screen::menu::{MenuEvent, tick_selectable};
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

fn tick_new_world_menu(
    menu: &mut crate::screen::menu::Menu,
    name: &mut String,
    seed: &mut String,
    from_play_menu: bool,
    input: &Input,
    settings: &mut Settings,
    world_count: usize,
) -> (Transition, bool) {
    use crate::screen::menu::MenuEvent;
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
    let events_before = menu.selection;
    let menu_input = menu_input_from(input);
    let events = menu.tick(&menu_input);
    let mut changed = events.iter().any(|e| matches!(e, MenuEvent::Moved));

    // Sync name/seed/arrays/booleans and scoretime visibility (Java setVisible).
    changed |= crate::screen::world_gen_display::apply_menu_to_settings(menu, settings, name, seed);
    let _ = events_before;
    let _ = world_count;

    if events.contains(&MenuEvent::Activated)
        && let Some(world_name) = crate::screen::world_gen_display::create_requested(menu, name)
    {
        return (
            Transition::CreateWorld(world_name, parse_world_seed(seed)),
            changed,
        );
    }
    (Transition::None, changed)
}

fn tick_worlds_menu(
    menu: &mut crate::screen::menu::Menu,
    popup: &mut Option<WorldPopup>,
    input: &Input,
    worlds: &[WorldRecord],
) -> Transition {
    use crate::screen::menu::MenuEvent;
    if let Some(dialog) = popup {
        if input.exit {
            *popup = None;
            return Transition::None;
        }
        if !matches!(dialog.kind, WorldPopupKind::Delete) {
            edit_text(&mut dialog.text, input, 36);
        }
        if input.select {
            let selection = menu.selection;
            return match dialog.kind {
                WorldPopupKind::Copy if !dialog.text.trim().is_empty() => {
                    Transition::CopyWorld(selection, dialog.text.clone())
                }
                WorldPopupKind::Rename if !dialog.text.trim().is_empty() => {
                    Transition::RenameWorld(selection, dialog.text.clone())
                }
                WorldPopupKind::Delete => Transition::DeleteWorld(selection),
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
    let selection = menu.selection;
    let menu_input = menu_input_from(input);
    let events = menu.tick(&menu_input);
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
            text: worlds[selection.min(worlds.len() - 1)].name.clone(),
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
    if events.contains(&MenuEvent::Activated) {
        Transition::LoadWorld(menu.selection)
    } else {
        Transition::None
    }
}

fn tick_help_menu(menu: &mut crate::screen::menu::Menu, input: &Input) -> Transition {
    use crate::screen::menu::MenuEvent;
    if input.exit {
        return Transition::Title;
    }
    let events = menu.tick(&menu_input_from(input));
    if events.contains(&MenuEvent::Activated) {
        // Blank row is non-selectable; selection 1..=4 maps to Book::ALL[0..=3].
        match crate::screen::help_display::book_for_selection(menu.selection) {
            Some(book) => Transition::Book(book),
            None => Transition::None,
        }
    } else {
        Transition::None
    }
}

fn tick_book_menu(
    menu: &mut crate::screen::menu::Menu,
    book: &mut Book,
    page: &mut usize,
    input: &Input,
) -> Transition {
    // Java BookDisplay closes on menu OR exit; page turns are silent.
    if input.exit || input.menu {
        return Transition::Help;
    }
    let page_count = book.pages().len();
    let mut turned = false;
    if (input.left_pressed || input.up_pressed)
        && let Some(next) = crate::screen::book::turn_page(*page, -1, page_count)
    {
        *page = next;
        turned = true;
    }
    if (input.right_pressed || input.down_pressed)
        && let Some(next) = crate::screen::book::turn_page(*page, 1, page_count)
    {
        *page = next;
        turned = true;
    }
    if turned {
        *menu = build_book_menu(*book, *page);
        return Transition::None;
    }
    // Still tick menu for scroll if multi-line page; select closes on last page via select+right already handled.
    menu.tick(&menu_input_from(input));
    Transition::None
}

fn tick_achievements_menu(menu: &mut crate::screen::menu::Menu, input: &Input) -> Transition {
    if input.exit {
        return Transition::Title;
    }
    menu.tick(&menu_input_from(input));
    Transition::None
}

#[allow(dead_code)]
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

fn tick_control_guide_menu(
    menu: &mut crate::screen::menu::Menu,
    controller: &mut bool,
    input: &Input,
) -> Transition {
    if input.exit {
        return Transition::Options;
    }
    if input.left_pressed || input.right_pressed {
        *controller = !*controller;
        return Transition::None; // Host rebuilds menu.
    }
    menu.tick(&menu_input_from(input));
    Transition::None
}

fn tick_options_menu(
    menu: &mut crate::screen::menu::Menu,
    input: &Input,
    settings: &mut Settings,
) -> (Transition, bool) {
    use crate::screen::menu::MenuEvent;
    use crate::screen::options_main_menu_display as options;
    if input.exit {
        return (Transition::Title, false);
    }
    let menu_input = menu_input_from(input);
    let events = menu.tick(&menu_input);
    let mut changed = events.iter().any(|e| matches!(e, MenuEvent::Moved));
    // Java FPS RangeEntry steps by 10 on left/right of the FPS row.
    if (menu_input.left || menu_input.right) && menu.selection == options::indices::FPS {
        changed |= options::step_fps(menu, menu_input.left);
    }
    changed |= options::apply_menu_to_settings(menu, settings);
    if events.contains(&MenuEvent::Activated) {
        let transition = match options::action_for_selection(menu.selection) {
            Some(options::OptionsAction::KeyBindings) => Transition::Controls,
            Some(options::OptionsAction::Controls) => Transition::ControlGuide,
            Some(options::OptionsAction::Language) => Transition::Languages,
            Some(options::OptionsAction::ResourcePacks) => Transition::ResourcePacks,
            None => Transition::None,
        };
        return (transition, changed);
    }
    (Transition::None, changed)
}

fn tick_languages_menu(menu: &mut crate::screen::menu::Menu, input: &Input) -> Transition {
    use crate::screen::menu::MenuEvent;
    if input.exit {
        return Transition::Options;
    }
    let menu_input = menu_input_from(input);
    let events = menu.tick(&menu_input);
    if events.contains(&MenuEvent::Activated) {
        Transition::ChangeLocale(menu.selection)
    } else {
        Transition::None
    }
}

fn tick_controls_menu(
    menu: &mut crate::screen::menu::Menu,
    capture: &mut Option<(usize, bool)>,
    keyboard: &mut bool,
    input: &Input,
    raw_keys: &[Key],
) -> Transition {
    use crate::screen::menu::MenuEvent;
    if let Some((index, append)) = *capture {
        if raw_keys.contains(&Key::Escape) {
            *capture = None;
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
            *capture = None;
            return Transition::SetBinding(index, value, append);
        }
        return Transition::None;
    }
    if input.exit {
        return Transition::Options;
    }
    if input.cursor_left_pressed || input.cursor_right_pressed {
        *keyboard = !*keyboard;
        return Transition::None; // Host rebuilds menu after transition tick.
    }
    let menu_input = menu_input_from(input);
    let events = menu.tick(&menu_input);
    if events.contains(&MenuEvent::Activated) {
        // Key binding capture is still Compatibility Controls (stage 7 continues).
        Transition::CaptureBinding(menu.selection, false)
    } else {
        Transition::None
    }
}

fn navigate(selection: &mut usize, input: &Input, count: usize) {
    if input.cursor_up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(count - 1);
    }
    if input.cursor_down_pressed {
        *selection = (*selection + 1) % count;
    }
}

#[allow(dead_code)]
fn tick_languages(selection: &mut usize, input: &Input) -> Transition {
    if input.exit {
        return Transition::Options;
    }
    if input.cursor_up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(LOCALES.len() - 1);
    }
    if input.cursor_down_pressed {
        *selection = (*selection + 1) % LOCALES.len();
    }
    if input.cursor_up_pressed != input.cursor_down_pressed {
        return Transition::None;
    }
    if input.select {
        Transition::ChangeLocale(*selection)
    } else {
        Transition::None
    }
}

fn tick_skins_menu(
    menu: &mut crate::screen::menu::Menu,
    ticks: &mut u64,
    input: &Input,
) -> Transition {
    use crate::screen::menu::MenuEvent;
    *ticks = ticks.wrapping_add(1);
    if input.exit {
        return Transition::CancelSkins;
    }
    let events = menu.tick(&menu_input_from(input));
    if events.contains(&MenuEvent::Activated) {
        Transition::ChangeSkin(menu.selection)
    } else {
        Transition::None
    }
}

/// Compatibility wrapper for parity probes that still pass a bare selection index.
fn tick_resource_packs(
    selection: &mut usize,
    input: &Input,
    packs: &mut [ResourcePack],
) -> Transition {
    use crate::screen::entry::Entry;
    use crate::screen::menu::MenuBuilder;
    use crate::screen::rel_pos::RelPos;
    let mut menu = MenuBuilder::new(
        false,
        2,
        RelPos::CENTER,
        packs
            .iter()
            .map(|pack| Entry::select(pack.name.clone(), false))
            .collect(),
    )
    .set_selection(*selection as i32)
    .create_menu();
    let result = tick_resource_packs_menu(&mut menu, input, packs);
    *selection = menu.selection;
    result
}

fn tick_resource_packs_menu(
    menu: &mut crate::screen::menu::Menu,
    input: &Input,
    packs: &mut [ResourcePack],
) -> Transition {
    use crate::screen::menu::MenuEvent;
    use crate::screen::resource_pack_display as packs_ui;
    if input.cursor_right_pressed {
        return match packs_ui::try_reorder(packs, menu.selection, true) {
            packs_ui::Reorder::Moved(next) => {
                menu.set_selection(next as i32);
                Transition::MovePack(next)
            }
            packs_ui::Reorder::Blocked => Transition::None,
        };
    }
    if input.cursor_left_pressed {
        return match packs_ui::try_reorder(packs, menu.selection, false) {
            packs_ui::Reorder::Moved(next) => {
                menu.set_selection(next as i32);
                Transition::MovePack(next)
            }
            packs_ui::Reorder::Blocked => Transition::None,
        };
    }
    if input.exit {
        return Transition::Options;
    }
    if input.menu {
        return Transition::PackInfo(menu.selection);
    }
    if packs.is_empty() {
        return Transition::None;
    }
    let events = menu.tick(&menu_input_from(input));
    if events.contains(&MenuEvent::Activated) {
        Transition::TogglePack(menu.selection)
    } else {
        Transition::None
    }
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
    crate::screen::world_gen_display::parse_world_seed(value)
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

/// Stage 13: drop enabled ids that no longer resolve, so settings stay consistent
/// with the discovered pack set. Returns true when the list changed.
fn prune_missing_enabled_packs(enabled: &mut Vec<String>, packs: &[ResourcePack]) -> bool {
    crate::screen::resource_pack_display::prune_missing_enabled_packs(enabled, packs)
}

#[cfg(test)]
mod stage13_tests {
    use super::{ResourcePack, prune_missing_enabled_packs};

    #[test]
    fn missing_enabled_pack_ids_are_pruned() {
        let packs = vec![
            ResourcePack::parity_fixture("keep", "Keep", None),
            ResourcePack::parity_fixture("other", "Other", None),
        ];
        let mut enabled = vec!["keep".to_owned(), "ghost".to_owned()];
        assert!(prune_missing_enabled_packs(&mut enabled, &packs));
        assert_eq!(enabled, vec!["keep".to_owned()]);
        assert!(!prune_missing_enabled_packs(&mut enabled, &packs));
    }
}

fn active_packs(all: &[ResourcePack], enabled: &[String]) -> Vec<ResourcePack> {
    enabled
        .iter()
        .filter_map(|id| all.iter().find(|pack| &pack.id == id).cloned())
        .collect()
}

fn order_packs_for_menu(packs: &mut [ResourcePack], enabled: &[String]) {
    crate::screen::resource_pack_display::order_packs_for_menu(packs, enabled);
}

fn sync_enabled_pack_order(enabled: &mut Vec<String>, packs: &[ResourcePack]) {
    crate::screen::resource_pack_display::sync_enabled_pack_order(enabled, packs);
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
        sync_enabled_pack_order, tick_resource_packs,
    };
    use crate::{
        audio::SoundEffect,
        io::Input,
        io::Settings,
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
                &crate::io::Input {
                    exit: true,
                    ..crate::io::Input::default()
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
            cursor_right_pressed: true,
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
            let mut menu = super::build_options_menu(
                &crate::resource::localization::Localization::load("en-us", &[]),
                &settings,
                selection,
            );
            let (transition, _) = super::tick_options_menu(
                &mut menu,
                &Input {
                    select: true,
                    ..Input::default()
                },
                &mut settings,
            );
            assert!(matches!(transition, Transition::None));
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
            let mut menu = super::build_new_world_menu(
                &crate::resource::localization::Localization::load("en-us", &[]),
                &settings,
                "World",
                "",
                selection,
                1,
            );
            let result = super::tick_new_world_menu(
                &mut menu,
                &mut "World".to_owned(),
                &mut String::new(),
                false,
                &Input {
                    select: true,
                    ..Input::default()
                },
                &mut settings,
                1,
            );
            assert!(matches!(result.0, Transition::None));
        }
        assert_eq!(settings.quests, original_quests);
        assert_eq!(settings.tutorials, original_tutorials);
    }

    #[test]
    fn new_world_menu_stays_on_screen_like_java_center_anchor() {
        let menu = super::build_new_world_menu(
            &crate::resource::localization::Localization::load("en-us", &[]),
            &Settings::default(),
            "",
            "",
            0,
            0,
        );
        let (x, y, w, h) = menu.bounds;
        assert!(w > 0 && h > 0);
        assert!(
            x >= 0
                && y >= 0
                && x + w <= crate::gfx::WIDTH as i32
                && y + h <= crate::gfx::HEIGHT as i32,
            "menu bounds {x},{y} {w}x{h} must fit {}x{}",
            crate::gfx::WIDTH,
            crate::gfx::HEIGHT
        );
        assert_eq!(menu.display_length, 5);
        assert_eq!(menu.title, "World Gen Options");
        let (ex, ey, ew, _) = menu.entry_bounds;
        assert!(ex >= 0 && ey >= 0 && ex + ew <= crate::gfx::WIDTH as i32);
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
            cursor_left_pressed: true,
            cursor_down_pressed: true,
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
            cursor_down_pressed: true,
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
            cursor_right_pressed: true,
            cursor_left_pressed: true,
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
                cursor_left_pressed: true,
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
                    cursor_left_pressed: true,
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
            generation_version: crate::level::GenerationVersion::Legacy224,
            name: "ORIGINAL".to_owned(),
            seed: 1,
            spec: WorldSpec::default(),
            mode: GameMode::Survival,
            score_minutes: 20,
            directory: std::path::PathBuf::new(),
        }];
        let mut menu = super::build_worlds_menu(
            &worlds,
            &crate::resource::localization::Localization::load("en-us", &[]),
            0,
        );
        let mut popup = None;
        assert!(matches!(
            super::tick_worlds_menu(
                &mut menu,
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
            super::tick_worlds_menu(
                &mut menu,
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
