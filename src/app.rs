use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use minifb::{Key, KeyRepeat, Scale, Window, WindowOptions};

use crate::{
    assets::{Assets, skin_options},
    audio::{Audio, SoundEffect, validate_embedded_assets},
    config::{Config, KeyBindings, Settings},
    content::{Achievement, Book, achievements},
    gfx::{HEIGHT, Screen, WIDTH},
    input::{Gamepad, Input, key_name},
    localization::{LOCALES, Localization},
    resource_pack::{ResourcePack, discover},
    world::{GameMode, PlayOptions, World, WorldAction, WorldSpec},
    worlds::{
        WorldRecord, create as create_world_record, load as load_world_records, load_state,
        save_state,
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
    println!(
        "Minicraft+ Rust self-check passed (savedir {})",
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
    },
    Worlds {
        selection: usize,
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
        capture: Option<usize>,
    },
    Languages {
        selection: usize,
    },
    Skins {
        selection: usize,
    },
    ResourcePacks {
        selection: usize,
    },
    Playing {
        world: Box<World>,
        record: WorldRecord,
        last_autosave_tick: u64,
    },
}

enum Transition {
    None,
    Title,
    PlayMenu,
    NewWorld,
    CreateWorld,
    Worlds,
    LoadWorld(usize),
    Options,
    Controls,
    CaptureBinding(usize),
    SetBinding(usize, String),
    ResetBindings,
    Help,
    Book(Book),
    Achievements,
    Languages,
    Skins,
    ChangeSkin(usize),
    ResourcePacks,
    TogglePack(usize),
    MovePack(usize, i32),
    ChangeLocale(usize),
    Quit,
}

pub fn run(arguments: &[String]) -> Result<(), String> {
    let mut config = Config::load(arguments)?;
    let (packs, discovery_warnings) = discover(&config.game_dir);
    let active = active_packs(&packs, &config.settings.resource_packs);
    let mut pack_warnings = discovery_warnings.clone();
    let mut localization = Localization::load(&config.settings.locale, &active);
    let mut assets = Assets::load(&active)?;
    pack_warnings.extend(assets.warnings.iter().cloned());
    let skins = skin_options(&config.game_dir);
    let achievements = achievements()?;
    let mut world_records = load_world_records(&config.game_dir)?;
    if let Err(error) = assets.select_skin(&config.settings.selected_skin, &config.game_dir) {
        pack_warnings.push(error);
        config.settings.selected_skin = "minicraft.skin.paul".to_owned();
        assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
        config.save()?;
    }
    let mut screen = Screen::new();
    let mut window = Window::new(
        "Minicraft Plus 2.2.4 — Rust",
        WIDTH,
        HEIGHT,
        WindowOptions {
            resize: true,
            scale: Scale::X4,
            ..WindowOptions::default()
        },
    )
    .map_err(|error| error.to_string())?;
    window.set_target_fps(config.settings.fps);
    let mut state = State::Title {
        selection: 0,
        ticks: 0,
    };
    let audio = Audio::default();
    let mut gamepad = Gamepad::default();

    while window.is_open() {
        let raw_keys = window.get_keys_pressed(KeyRepeat::No);
        let input = Input::poll(
            &window,
            &config.settings.key_bindings,
            &raw_keys,
            &mut gamepad,
        );
        let was_playing = matches!(state, State::Playing { .. });
        let mut settings_changed = false;
        let transition = match &mut state {
            State::Title { selection, ticks } => tick_title(selection, ticks, &input),
            State::Options { selection } => {
                let result = tick_options(selection, &input, &mut config.settings);
                settings_changed = result.1;
                result.0
            }
            State::PlayMenu { selection } => tick_play_menu(selection, &input),
            State::NewWorld { selection } => {
                let result = tick_new_world(selection, &input, &mut config.settings);
                settings_changed = result.1;
                result.0
            }
            State::Worlds { selection } => tick_worlds(selection, &input, world_records.len()),
            State::Help { selection } => tick_help(selection, &input),
            State::Book { page, book } => tick_book(*book, page, &input),
            State::Achievements { selection } => {
                tick_achievements(selection, &input, achievements.len())
            }
            State::Controls { selection, capture } => {
                tick_controls(selection, capture, &input, &raw_keys)
            }
            State::Languages { selection } => tick_languages(selection, &input),
            State::Skins { selection } => tick_skins(selection, &input, skins.len()),
            State::ResourcePacks { selection } => {
                tick_resource_packs(selection, &input, &packs, &config.settings.resource_packs)
            }
            State::Playing {
                world,
                record,
                last_autosave_tick,
            } => {
                let action = world.tick(&input);
                for effect in world.take_sound_events() {
                    audio.play(effect, config.settings.sound);
                }
                if config.settings.autosave
                    && world.autosave_due()
                    && *last_autosave_tick != world.save_tick()
                {
                    save_state(record, world)?;
                    *last_autosave_tick = world.save_tick();
                }
                match action {
                    WorldAction::None => Transition::None,
                    WorldAction::ReturnToTitle => {
                        save_state(record, world)?;
                        Transition::Title
                    }
                }
            }
        };

        if !was_playing {
            if input.select {
                audio.play(SoundEffect::Confirm, config.settings.sound);
            } else if input.up_pressed
                || input.down_pressed
                || input.left_pressed
                || input.right_pressed
            {
                audio.play(SoundEffect::Select, config.settings.sound);
            }
        }

        if settings_changed {
            config.save()?;
            window.set_target_fps(config.settings.fps);
        }
        match transition {
            Transition::None => {}
            Transition::Title => {
                state = State::Title {
                    selection: 0,
                    ticks: 0,
                };
            }
            Transition::PlayMenu => state = State::PlayMenu { selection: 0 },
            Transition::NewWorld => state = State::NewWorld { selection: 0 },
            Transition::CreateWorld => {
                let seed = random_seed();
                let spec = WorldSpec::new(
                    config.settings.world_size,
                    config.settings.theme,
                    config.settings.terrain_type,
                );
                let mode = GameMode::from_index(config.settings.game_mode);
                let record = create_world_record(
                    &config.game_dir,
                    seed,
                    spec,
                    mode,
                    config.settings.score_minutes,
                )?;
                world_records = load_world_records(&config.game_dir)?;
                let world = World::new_with_play_options(
                    seed,
                    spec,
                    play_options(&config.settings, mode, config.settings.score_minutes),
                );
                save_state(&record, &world)?;
                state = State::Playing {
                    world: Box::new(world),
                    record,
                    last_autosave_tick: 0,
                };
            }
            Transition::Worlds => {
                world_records = load_world_records(&config.game_dir)?;
                state = State::Worlds { selection: 0 };
            }
            Transition::LoadWorld(selection) => {
                let record = world_records[selection].clone();
                let world = load_state(&record)?.unwrap_or_else(|| {
                    World::new_with_play_options(
                        record.seed,
                        record.spec,
                        play_options(&config.settings, record.mode, record.score_minutes),
                    )
                });
                let last_autosave_tick = world.save_tick();
                state = State::Playing {
                    world: Box::new(world),
                    record,
                    last_autosave_tick,
                };
            }
            Transition::Options => state = State::Options { selection: 0 },
            Transition::Controls => {
                state = State::Controls {
                    selection: 0,
                    capture: None,
                }
            }
            Transition::CaptureBinding(selection) => {
                state = State::Controls {
                    selection,
                    capture: Some(selection),
                }
            }
            Transition::SetBinding(selection, value) => {
                config.settings.key_bindings.set(selection, value);
                config.save()?;
                state = State::Controls {
                    selection,
                    capture: None,
                };
            }
            Transition::ResetBindings => {
                config.settings.key_bindings = KeyBindings::default();
                config.save()?;
                state = State::Controls {
                    selection: 0,
                    capture: None,
                };
            }
            Transition::Help => state = State::Help { selection: 0 },
            Transition::Book(book) => state = State::Book { book, page: 0 },
            Transition::Achievements => state = State::Achievements { selection: 0 },
            Transition::Languages => {
                let selection = LOCALES
                    .iter()
                    .position(|(code, _)| *code == config.settings.locale)
                    .unwrap_or(0);
                state = State::Languages { selection };
            }
            Transition::Skins => {
                let selection = skins
                    .iter()
                    .position(|skin| *skin == config.settings.selected_skin)
                    .unwrap_or(0);
                state = State::Skins { selection };
            }
            Transition::ChangeSkin(selection) => {
                assets.select_skin(&skins[selection], &config.game_dir)?;
                config.settings.selected_skin = skins[selection].clone();
                config.save()?;
                state = State::Title {
                    selection: 2,
                    ticks: 0,
                };
            }
            Transition::ResourcePacks => state = State::ResourcePacks { selection: 0 },
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
                reload_resources(
                    &mut assets,
                    &mut localization,
                    &mut pack_warnings,
                    &discovery_warnings,
                    &packs,
                    &config,
                )?;
                config.save()?;
                state = State::ResourcePacks { selection };
            }
            Transition::MovePack(selection, direction) => {
                move_enabled_pack(
                    &mut config.settings.resource_packs,
                    &packs[selection].id,
                    direction,
                );
                reload_resources(
                    &mut assets,
                    &mut localization,
                    &mut pack_warnings,
                    &discovery_warnings,
                    &packs,
                    &config,
                )?;
                config.save()?;
                state = State::ResourcePacks { selection };
            }
            Transition::ChangeLocale(selection) => {
                config.settings.locale = LOCALES[selection].0.to_owned();
                config.save()?;
                let active = active_packs(&packs, &config.settings.resource_packs);
                localization = Localization::load(&config.settings.locale, &active);
                state = State::Options { selection: 0 };
            }
            Transition::Quit => break,
        }

        screen.clear(0x08080C);
        match &state {
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
            State::PlayMenu { selection } => {
                render_play_menu(&mut screen, &assets, world_records.len(), *selection)
            }
            State::NewWorld { selection } => render_new_world(
                &mut screen,
                &assets,
                &localization,
                &config.settings,
                *selection,
            ),
            State::Worlds { selection } => {
                render_worlds(&mut screen, &assets, &world_records, *selection)
            }
            State::Help { selection } => render_help(&mut screen, &assets, *selection),
            State::Book { book, page } => render_book(&mut screen, &assets, *book, *page),
            State::Achievements { selection } => render_achievements(
                &mut screen,
                &assets,
                &localization,
                &achievements,
                *selection,
            ),
            State::Controls { selection, capture } => render_controls(
                &mut screen,
                &assets,
                &config.settings.key_bindings,
                *selection,
                *capture,
            ),
            State::Languages { selection } => render_languages(&mut screen, &assets, *selection),
            State::Skins { selection } => {
                render_skins(&mut screen, &assets, &localization, &skins, *selection)
            }
            State::ResourcePacks { selection } => render_resource_packs(
                &mut screen,
                &assets,
                &packs,
                &config.settings.resource_packs,
                &pack_warnings,
                *selection,
            ),
            State::Playing { world, .. } => world.render(&mut screen, &assets),
        }
        window
            .update_with_buffer(screen.pixels(), WIDTH, HEIGHT)
            .map_err(|error| error.to_string())?;
    }
    if let State::Playing { world, record, .. } = &state {
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
    screen.clear(0x08080C);
    render_title(&mut screen, &assets, &localization, 0, 30);
    screen.save_png(output)
}

pub fn render_world_preview(arguments: &[String], output: &Path) -> Result<(), String> {
    let config = Config::load(arguments)?;
    let (packs, _) = discover(&config.game_dir);
    let active = active_packs(&packs, &config.settings.resource_packs);
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
    let mut screen = Screen::new();
    screen.clear(0);
    world.render(&mut screen, &assets);
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
    let mut screen = Screen::new();
    screen.clear(0x08080C);
    match name {
        "options" => render_options(&mut screen, &assets, &localization, &config.settings, 5),
        "play" => render_play_menu(&mut screen, &assets, worlds.len(), 0),
        "new-world" => render_new_world(&mut screen, &assets, &localization, &config.settings, 0),
        "worlds" => render_worlds(&mut screen, &assets, &worlds, 0),
        "help" => render_help(&mut screen, &assets, 0),
        "book" => render_book(&mut screen, &assets, Book::Instructions, 0),
        "achievements" => {
            render_achievements(&mut screen, &assets, &localization, &achievements, 0)
        }
        "controls" => render_controls(&mut screen, &assets, &config.settings.key_bindings, 0, None),
        _ => {
            return Err(format!(
                "unknown UI preview {name}; use options, play, new-world, worlds, help, book, achievements, or controls"
            ));
        }
    }
    screen.save_png(output)
}

fn tick_title(selection: &mut usize, ticks: &mut u64, input: &Input) -> Transition {
    *ticks = ticks.wrapping_add(1);
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(TITLE_KEYS.len() - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % TITLE_KEYS.len();
    }
    if input.exit {
        return Transition::Quit;
    }
    if !input.select {
        return Transition::None;
    }
    match *selection {
        0 => Transition::PlayMenu,
        1 => Transition::Options,
        2 => Transition::Skins,
        3 => Transition::Achievements,
        4 => Transition::Help,
        _ => Transition::Quit,
    }
}

fn tick_play_menu(selection: &mut usize, input: &Input) -> Transition {
    const COUNT: usize = 3;
    if input.exit {
        return Transition::Title;
    }
    navigate(selection, input, COUNT);
    if input.select {
        match *selection {
            0 => Transition::NewWorld,
            1 => Transition::Worlds,
            _ => Transition::Title,
        }
    } else {
        Transition::None
    }
}

fn tick_new_world(
    selection: &mut usize,
    input: &Input,
    settings: &mut Settings,
) -> (Transition, bool) {
    const COUNT: usize = 7;
    if input.exit {
        return (Transition::PlayMenu, false);
    }
    navigate(selection, input, COUNT);
    let direction = i32::from(input.right_pressed) - i32::from(input.left_pressed);
    let mut changed = false;
    if direction != 0 {
        match *selection {
            0 => {
                let sizes = [128, 256, 512];
                let current = sizes
                    .iter()
                    .position(|size| *size == settings.world_size)
                    .unwrap_or(0);
                settings.world_size = sizes[wrap(current, direction, sizes.len())];
                changed = true;
            }
            1 => {
                settings.theme = wrap(settings.theme, direction, 5);
                changed = true;
            }
            2 => {
                settings.terrain_type = wrap(settings.terrain_type, direction, 4);
                changed = true;
            }
            3 => {
                settings.game_mode = wrap(settings.game_mode, direction, GameMode::ALL.len());
                changed = true;
            }
            4 => {
                let times = [10, 20, 40, 60, 120];
                let current = times
                    .iter()
                    .position(|time| *time == settings.score_minutes)
                    .unwrap_or(1);
                settings.score_minutes = times[wrap(current, direction, times.len())];
                changed = true;
            }
            _ => {}
        }
    }
    if input.select {
        match *selection {
            5 => (Transition::CreateWorld, changed),
            6 => (Transition::PlayMenu, changed),
            _ => (Transition::None, changed),
        }
    } else {
        (Transition::None, changed)
    }
}

fn tick_worlds(selection: &mut usize, input: &Input, count: usize) -> Transition {
    if input.exit {
        return Transition::PlayMenu;
    }
    if count == 0 {
        return if input.select {
            Transition::NewWorld
        } else {
            Transition::None
        };
    }
    navigate(selection, input, count);
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
    capture: &mut Option<usize>,
    input: &Input,
    raw_keys: &[Key],
) -> Transition {
    if let Some(index) = *capture {
        if raw_keys.contains(&Key::Escape) {
            return Transition::Controls;
        }
        if let Some(name) = raw_keys.iter().find_map(|key| key_name(*key)) {
            return Transition::SetBinding(index, name.to_owned());
        }
        return Transition::None;
    }
    const COUNT: usize = 11;
    if input.exit {
        return Transition::Options;
    }
    navigate(selection, input, COUNT);
    if input.select {
        match *selection {
            0..=8 => Transition::CaptureBinding(*selection),
            9 => Transition::ResetBindings,
            _ => Transition::Options,
        }
    } else {
        Transition::None
    }
}

fn tick_options(
    selection: &mut usize,
    input: &Input,
    settings: &mut Settings,
) -> (Transition, bool) {
    const COUNT: usize = 11;
    if input.exit {
        return (Transition::Title, false);
    }
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(COUNT - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % COUNT;
    }
    let direction = i32::from(input.right_pressed) - i32::from(input.left_pressed);
    let mut changed = false;
    if direction != 0 {
        match *selection {
            1 => {
                settings.fps = (settings.fps as i32 + direction * 10).clamp(10, 300) as usize;
                changed = true;
            }
            2 => {
                settings.difficulty = wrap(settings.difficulty, direction, 3);
                changed = true;
            }
            3 => {
                settings.sound = !settings.sound;
                changed = true;
            }
            4 => {
                settings.autosave = !settings.autosave;
                changed = true;
            }
            5 => {
                settings.tutorials = !settings.tutorials;
                changed = true;
            }
            6 => {
                settings.quests = !settings.quests;
                changed = true;
            }
            7 => {
                settings.show_quests = !settings.show_quests;
                changed = true;
            }
            _ => {}
        }
    }
    if input.select {
        return match *selection {
            0 => (Transition::Languages, changed),
            3 => {
                settings.sound = !settings.sound;
                (Transition::None, true)
            }
            4 => {
                settings.autosave = !settings.autosave;
                (Transition::None, true)
            }
            5 => {
                settings.tutorials = !settings.tutorials;
                (Transition::None, true)
            }
            6 => {
                settings.quests = !settings.quests;
                (Transition::None, true)
            }
            7 => {
                settings.show_quests = !settings.show_quests;
                (Transition::None, true)
            }
            8 => (Transition::Controls, changed),
            9 => (Transition::ResourcePacks, changed),
            10 => (Transition::Title, changed),
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
    if input.select {
        Transition::ChangeLocale(*selection)
    } else {
        Transition::None
    }
}

fn tick_skins(selection: &mut usize, input: &Input, count: usize) -> Transition {
    if input.exit {
        return Transition::Title;
    }
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(count - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % count;
    }
    if input.select {
        Transition::ChangeSkin(*selection)
    } else {
        Transition::None
    }
}

fn tick_resource_packs(
    selection: &mut usize,
    input: &Input,
    packs: &[ResourcePack],
    enabled: &[String],
) -> Transition {
    if input.exit {
        return Transition::Options;
    }
    if packs.is_empty() {
        return if input.select {
            Transition::Options
        } else {
            Transition::None
        };
    }
    if input.up_pressed {
        *selection = selection.checked_sub(1).unwrap_or(packs.len() - 1);
    }
    if input.down_pressed {
        *selection = (*selection + 1) % packs.len();
    }
    if input.select {
        return Transition::TogglePack(*selection);
    }
    let direction = i32::from(input.right_pressed) - i32::from(input.left_pressed);
    if direction != 0 && enabled.contains(&packs[*selection].id) {
        Transition::MovePack(*selection, direction)
    } else {
        Transition::None
    }
}

fn render_title(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    selection: usize,
    ticks: u64,
) {
    screen.blit(
        &assets.title,
        (WIDTH as i32 - assets.title.width as i32) / 2,
        18,
    );
    screen.centered_text(
        &assets.font,
        "NOW POWERED BY RUST!",
        48 + ((ticks / 30) % 2) as i32,
    );
    let version = localization.format("minicraft.displays.title.display.version", &["2.2.4"]);
    screen.text(&assets.font, &version, 2, 2);
    for (index, key) in TITLE_KEYS.iter().enumerate() {
        let item = localization.text(key);
        let y = 76 + index as i32 * 14;
        if index == selection {
            screen.rect(72, y - 2, 144, 11, 0x27334D);
            screen.text(&assets.font, ">", 76, y);
        }
        let x = (WIDTH as i32 - item.chars().count() as i32 * 8) / 2;
        screen.text(&assets.font, item, x, y);
    }
    screen.centered_text(&assets.font, "UP DOWN TO MOVE", HEIGHT as i32 - 19);
    screen.centered_text(
        &assets.font,
        "ENTER TO SELECT  ESC TO EXIT",
        HEIGHT as i32 - 10,
    );
}

fn render_options(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    settings: &Settings,
    selection: usize,
) {
    screen.centered_text(
        &assets.font,
        localization.text("minicraft.display.options_display"),
        18,
    );
    let difficulty_keys = [
        "minicraft.settings.difficulty.easy",
        "minicraft.settings.difficulty.normal",
        "minicraft.settings.difficulty.hard",
    ];
    let on = localization.text("minicraft.display.entries.boolean.true");
    let off = localization.text("minicraft.display.entries.boolean.false");
    let values = [
        LOCALES
            .iter()
            .find(|(code, _)| *code == settings.locale)
            .map(|(_, name)| *name)
            .unwrap_or("English (US)")
            .to_owned(),
        settings.fps.to_string(),
        localization
            .text(difficulty_keys[settings.difficulty])
            .to_owned(),
        if settings.sound { on } else { off }.to_owned(),
        if settings.autosave { on } else { off }.to_owned(),
        if settings.tutorials { on } else { off }.to_owned(),
        if settings.quests { on } else { off }.to_owned(),
        if settings.show_quests { on } else { off }.to_owned(),
        String::new(),
        String::new(),
        String::new(),
    ];
    let labels = [
        localization.text("minicraft.display.options_display.language"),
        localization.text("minicraft.settings.fps"),
        localization.text("minicraft.settings.difficulty"),
        localization.text("minicraft.settings.sound"),
        localization.text("minicraft.settings.autosave"),
        "TUTORIALS",
        "QUESTS",
        "SHOW QUEST HUD",
        "KEY BINDINGS",
        localization.text("minicraft.display.options_display.resource_packs"),
        "BACK",
    ];
    for index in 0..labels.len() {
        let y = 29 + index as i32 * 14;
        if index == selection {
            screen.rect(14, y - 2, WIDTH as i32 - 28, 11, 0x27334D);
            screen.text(&assets.font, ">", 17, y);
        }
        screen.text(&assets.font, labels[index], 29, y);
        if !values[index].is_empty() {
            let x = WIDTH as i32 - 21 - values[index].chars().count() as i32 * 8;
            screen.text(&assets.font, &values[index], x, y);
        }
    }
    screen.centered_text(&assets.font, "LEFT RIGHT TO CHANGE", HEIGHT as i32 - 13);
}

fn render_play_menu(screen: &mut Screen, assets: &Assets, world_count: usize, selection: usize) {
    screen.centered_text(&assets.font, "PLAY", 22);
    let labels = [
        "CREATE NEW WORLD".to_owned(),
        format!("WORLDS ({world_count})"),
        "BACK".to_owned(),
    ];
    render_centered_menu(screen, assets, &labels, selection, 65, 22);
    screen.centered_text(&assets.font, "VERSIONED SAVES + BACKUPS", 155);
}

fn render_new_world(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    settings: &Settings,
    selection: usize,
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
    screen.centered_text(&assets.font, "NEW WORLD", 18);
    let labels = [
        "SIZE",
        "THEME",
        "TERRAIN",
        "MODE",
        "SCORE TIME",
        "CREATE",
        "BACK",
    ];
    let values = [
        settings.world_size.to_string(),
        localization.text(THEME_KEYS[settings.theme]).to_owned(),
        localization
            .text(TYPE_KEYS[settings.terrain_type])
            .to_owned(),
        GameMode::from_index(settings.game_mode)
            .display_name()
            .to_owned(),
        format!("{} MIN", settings.score_minutes),
        String::new(),
        String::new(),
    ];
    for index in 0..labels.len() {
        let y = 34 + index as i32 * 20;
        if index == selection {
            screen.rect(18, y - 2, WIDTH as i32 - 36, 11, 0x27334D);
            screen.text(&assets.font, ">", 22, y);
        }
        screen.text(&assets.font, labels[index], 38, y);
        if !values[index].is_empty() {
            let x = WIDTH as i32 - 24 - values[index].chars().count() as i32 * 8;
            screen.text(&assets.font, &values[index], x, y);
        }
    }
    screen.centered_text(&assets.font, "LEFT RIGHT TO CHANGE", HEIGHT as i32 - 12);
}

fn render_worlds(screen: &mut Screen, assets: &Assets, worlds: &[WorldRecord], selection: usize) {
    screen.centered_text(&assets.font, "SELECT WORLD", 13);
    if worlds.is_empty() {
        screen.centered_text(&assets.font, "NO WORLDS FOUND", 74);
        screen.centered_text(&assets.font, "ENTER TO CREATE ONE", 94);
    } else {
        let first = selection
            .saturating_sub(6)
            .min(worlds.len().saturating_sub(13));
        for (row, world) in worlds.iter().skip(first).take(13).enumerate() {
            let index = first + row;
            let y = 32 + row as i32 * 11;
            if index == selection {
                screen.rect(25, y - 1, 238, 9, 0x27334D);
                screen.text(&assets.font, ">", 29, y);
            }
            screen.text(&assets.font, &world.name, 41, y);
            let summary = format!("{} {}", world.spec.size, world.mode.display_name());
            let x = WIDTH as i32 - 29 - summary.chars().count() as i32 * 8;
            screen.text(&assets.font, &summary, x, y);
        }
    }
    screen.centered_text(&assets.font, "SEED-BASED WORLD RECORDS", HEIGHT as i32 - 12);
}

fn render_help(screen: &mut Screen, assets: &Assets, selection: usize) {
    screen.centered_text(&assets.font, "HELP", 22);
    let labels: Vec<String> = Book::ALL
        .iter()
        .map(|book| book.title().to_owned())
        .collect();
    render_centered_menu(screen, assets, &labels, selection, 58, 21);
    screen.centered_text(&assets.font, "ORIGINAL 2.2.4 BOOK CONTENT", 157);
}

fn render_book(screen: &mut Screen, assets: &Assets, book: Book, page: usize) {
    let pages = book.pages();
    let page = page.min(pages.len() - 1);
    screen.frame(6, 6, WIDTH as i32 - 12, HEIGHT as i32 - 12, 0xC8C8C8);
    screen.centered_text(&assets.font, book.title(), 13);
    for (row, line) in pages[page].iter().enumerate() {
        screen.text(&assets.font, line, 16, 31 + row as i32 * 10);
    }
    screen.centered_text(
        &assets.font,
        &format!("<  PAGE {} / {}  >", page + 1, pages.len()),
        HEIGHT as i32 - 18,
    );
}

fn render_achievements(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    achievements: &[Achievement],
    selection: usize,
) {
    screen.centered_text(&assets.font, "ACHIEVEMENTS", 10);
    let first = selection
        .saturating_sub(5)
        .min(achievements.len().saturating_sub(11));
    for (row, achievement) in achievements.iter().skip(first).take(11).enumerate() {
        let index = first + row;
        let y = 27 + row as i32 * 10;
        if index == selection {
            screen.rect(16, y - 1, WIDTH as i32 - 32, 9, 0x27334D);
            screen.text(&assets.font, ">", 19, y);
        }
        let name = localization.text(&achievement.id);
        screen.text(
            &assets.font,
            &format!("[ ] {}  {}P", clipped(name, 23), achievement.score),
            31,
            y,
        );
    }
    if let Some(achievement) = achievements.get(selection) {
        screen.frame(12, 143, WIDTH as i32 - 24, 34, 0x686878);
        let description = localization.text(&achievement.description);
        for (row, line) in crate::content::paginate(description, 31, 2)[0]
            .iter()
            .enumerate()
        {
            screen.text(&assets.font, line, 20, 150 + row as i32 * 10);
        }
    }
}

fn render_controls(
    screen: &mut Screen,
    assets: &Assets,
    bindings: &KeyBindings,
    selection: usize,
    capture: Option<usize>,
) {
    screen.centered_text(&assets.font, "KEY BINDINGS", 8);
    for (index, label) in KeyBindings::LABELS.iter().enumerate() {
        let y = 21 + index as i32 * 13;
        if index == selection {
            screen.rect(11, y - 1, WIDTH as i32 - 22, 10, 0x27334D);
            screen.text(&assets.font, ">", 14, y);
        }
        screen.text(&assets.font, label, 27, y);
        let value = if capture == Some(index) {
            "PRESS A KEY..."
        } else {
            bindings.value(index)
        };
        let x = WIDTH as i32 - 17 - value.chars().count() as i32 * 8;
        screen.text(&assets.font, value, x, y);
    }
    for (offset, label) in ["RESET DEFAULTS", "BACK"].iter().enumerate() {
        let index = 9 + offset;
        let y = 143 + offset as i32 * 14;
        if index == selection {
            screen.rect(56, y - 1, 176, 10, 0x27334D);
            screen.text(&assets.font, ">", 60, y);
        }
        screen.text(&assets.font, label, 74, y);
    }
    if capture.is_some() {
        screen.centered_text(&assets.font, "ESC CANCELS", HEIGHT as i32 - 12);
    }
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
            screen.rect(48, y - 2, WIDTH as i32 - 96, 11, 0x27334D);
            screen.text(&assets.font, ">", 54, y);
        }
        screen.centered_text(&assets.font, label, y);
    }
}

fn clipped(text: &str, columns: usize) -> String {
    let mut value: String = text.chars().take(columns).collect();
    if text.chars().count() > columns {
        value.pop();
        value.push('>');
    }
    value
}

fn render_languages(screen: &mut Screen, assets: &Assets, selection: usize) {
    screen.centered_text(&assets.font, "LANGUAGE", 12);
    let first = selection
        .saturating_sub(7)
        .min(LOCALES.len().saturating_sub(15));
    for (row, (_, name)) in LOCALES.iter().skip(first).take(15).enumerate() {
        let index = first + row;
        let y = 30 + row as i32 * 10;
        if index == selection {
            screen.rect(38, y - 1, 212, 9, 0x27334D);
            screen.text(&assets.font, ">", 41, y);
        }
        screen.text(&assets.font, name, 53, y);
    }
    screen.centered_text(&assets.font, "ENTER TO APPLY", HEIGHT as i32 - 11);
}

fn render_skins(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    skins: &[String],
    selection: usize,
) {
    screen.centered_text(
        &assets.font,
        localization.text("minicraft.displays.skin"),
        12,
    );
    screen.blit_region(
        &assets.skin,
        WIDTH as i32 / 2 - 8,
        31,
        0,
        assets.skin_row,
        16,
        16,
        false,
    );
    let first = selection
        .saturating_sub(5)
        .min(skins.len().saturating_sub(11));
    for (row, skin) in skins.iter().skip(first).take(11).enumerate() {
        let index = first + row;
        let y = 59 + row as i32 * 10;
        if index == selection {
            screen.rect(38, y - 1, 212, 9, 0x27334D);
            screen.text(&assets.font, ">", 41, y);
        }
        let name = if skin.starts_with("minicraft.skin.") {
            localization.text(skin)
        } else {
            skin
        };
        screen.text(&assets.font, name, 53, y);
    }
    screen.centered_text(
        &assets.font,
        "ENTER TO APPLY  ESC TO CANCEL",
        HEIGHT as i32 - 11,
    );
}

fn render_resource_packs(
    screen: &mut Screen,
    assets: &Assets,
    packs: &[ResourcePack],
    enabled: &[String],
    warnings: &[String],
    selection: usize,
) {
    screen.centered_text(&assets.font, "RESOURCE PACKS", 14);
    screen.text(&assets.font, "[X] DEFAULT", 35, 31);
    let first = selection
        .saturating_sub(5)
        .min(packs.len().saturating_sub(11));
    for (row, pack) in packs.iter().skip(first).take(11).enumerate() {
        let index = first + row;
        let y = 45 + row as i32 * 10;
        if index == selection {
            screen.rect(24, y - 1, 240, 9, 0x27334D);
            screen.text(&assets.font, ">", 27, y);
        }
        let mark = if enabled.contains(&pack.id) { "X" } else { " " };
        screen.text(&assets.font, &format!("[{mark}] {}", pack.name), 39, y);
    }
    if packs.is_empty() {
        screen.centered_text(&assets.font, "NO CUSTOM PACKS FOUND", 78);
    }
    if !warnings.is_empty() {
        screen.centered_text(
            &assets.font,
            &format!("{} INVALID ENTRIES SKIPPED", warnings.len()),
            159,
        );
    }
    screen.centered_text(
        &assets.font,
        "ENTER TO TOGGLE  LEFT RIGHT ORDER",
        HEIGHT as i32 - 13,
    );
}

fn random_seed() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos() as i64)
        .unwrap_or(0x100)
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

fn move_enabled_pack(enabled: &mut [String], id: &str, direction: i32) {
    let Some(index) = enabled.iter().position(|candidate| candidate == id) else {
        return;
    };
    let target = (index as i32 + direction).rem_euclid(enabled.len() as i32) as usize;
    enabled.swap(index, target);
}

fn reload_resources(
    assets: &mut Assets,
    localization: &mut Localization,
    warnings: &mut Vec<String>,
    discovery_warnings: &[String],
    packs: &[ResourcePack],
    config: &Config,
) -> Result<(), String> {
    let active = active_packs(packs, &config.settings.resource_packs);
    *assets = Assets::load(&active)?;
    assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
    *localization = Localization::load(&config.settings.locale, &active);
    warnings.clear();
    warnings.extend(discovery_warnings.iter().cloned());
    warnings.extend(assets.warnings.iter().cloned());
    Ok(())
}
