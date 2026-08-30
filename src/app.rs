use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use minifb::{Key, KeyRepeat, Scale, ScaleMode, Window, WindowOptions};

use crate::{
    assets::{Assets, skin_options},
    audio::{Audio, SoundEffect, validate_embedded_assets},
    config::{Config, KeyBindings, Settings},
    content::{Achievement, Book, achievements},
    gfx::{HEIGHT, Screen, WIDTH},
    input::{Gamepad, Input, key_name},
    localization::{LOCALES, Localization},
    presenter::Presenter,
    resource_pack::{ResourcePack, discover},
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

const NORMAL_TPS: u32 = 60;
const TICK_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / NORMAL_TPS as u64);
const MAX_CATCH_UP_TICKS: usize = 15;

struct FixedStep {
    last: Instant,
    backlog: Duration,
}

impl FixedStep {
    fn new() -> Self {
        Self {
            last: Instant::now(),
            backlog: TICK_INTERVAL,
        }
    }

    fn updates(&mut self) -> usize {
        let now = Instant::now();
        let elapsed = now.saturating_duration_since(self.last);
        self.last = now;
        consume_fixed_updates(&mut self.backlog, elapsed)
    }
}

fn consume_fixed_updates(backlog: &mut Duration, elapsed: Duration) -> usize {
    let maximum = TICK_INTERVAL * MAX_CATCH_UP_TICKS as u32;
    *backlog = (*backlog + elapsed.min(maximum)).min(maximum);
    let count = (backlog.as_nanos() / TICK_INTERVAL.as_nanos()) as usize;
    *backlog -= TICK_INTERVAL * count as u32;
    count
}

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
        capture: Option<usize>,
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
    Playing {
        world: Box<World>,
        record: WorldRecord,
        last_autosave_tick: u64,
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
    CaptureBinding(usize),
    SetBinding(usize, String),
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
    let mut state = State::Title {
        selection: 0,
        ticks: 0,
    };
    let audio = Audio::default();
    let mut gamepad = Gamepad::default();
    let mut fixed_step = FixedStep::new();
    let mut input_latch = Input::default();
    let mut pending_raw_keys = Vec::new();

    'game: while window.is_open() {
        let raw_keys = window.get_keys_pressed(KeyRepeat::No);
        let frame_input = Input::poll(
            &window,
            &config.settings.key_bindings,
            &raw_keys,
            &mut gamepad,
        );
        let menu_sound = match &state {
            State::Playing { world, .. } => world.immediate_menu_sound(&frame_input),
            _ if frame_input.select => Some(SoundEffect::Confirm),
            _ if frame_input.up_pressed
                || frame_input.down_pressed
                || frame_input.left_pressed
                || frame_input.right_pressed =>
            {
                Some(SoundEffect::Select)
            }
            _ => None,
        };
        if let Some(effect) = menu_sound {
            audio.play(effect, config.settings.sound);
        }
        input_latch.latch(frame_input);
        pending_raw_keys.extend(raw_keys);
        for _ in 0..fixed_step.updates() {
            let input = input_latch.take_tick();
            let raw_keys = std::mem::take(&mut pending_raw_keys);
            let mut settings_changed = false;
            let transition = match &mut state {
                State::Title { selection, ticks } => {
                    tick_title(selection, ticks, &input, world_records.len())
                }
                State::Options { selection } => {
                    let result = tick_options(selection, &input, &mut config.settings);
                    settings_changed = result.1;
                    result.0
                }
                State::PlayMenu { selection } => tick_play_menu(selection, &input),
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
                State::Languages { selection } => tick_languages(selection, &input),
                State::Skins { selection, ticks } => {
                    tick_skins(selection, ticks, &input, skins.len())
                }
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
                        WorldAction::SaveGame => {
                            save_state(record, world)?;
                            *last_autosave_tick = world.save_tick();
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
                Transition::NewWorld(from_play_menu) => {
                    state = State::NewWorld {
                        selection: 0,
                        name: String::new(),
                        seed: String::new(),
                        from_play_menu,
                    }
                }
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
                    state = State::Worlds {
                        selection: 0,
                        popup: None,
                    };
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
                Transition::CopyWorld(selection, name) => {
                    copy_record(&config.game_dir, &world_records[selection], &name)?;
                    world_records = load_world_records(&config.game_dir)?;
                    let selection = world_records
                        .iter()
                        .position(|record| record.name.eq_ignore_ascii_case(name.trim()))
                        .unwrap_or(0);
                    state = State::Worlds {
                        selection,
                        popup: None,
                    };
                }
                Transition::RenameWorld(selection, name) => {
                    rename_record(&config.game_dir, &world_records[selection], &name)?;
                    world_records = load_world_records(&config.game_dir)?;
                    let selection = world_records
                        .iter()
                        .position(|record| record.name.eq_ignore_ascii_case(name.trim()))
                        .unwrap_or(0);
                    state = State::Worlds {
                        selection,
                        popup: None,
                    };
                }
                Transition::DeleteWorld(selection) => {
                    delete_record(&config.game_dir, &world_records[selection])?;
                    world_records = load_world_records(&config.game_dir)?;
                    state = if world_records.is_empty() {
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
                    };
                }
                Transition::Options => state = State::Options { selection: 0 },
                Transition::Controls => {
                    state = State::Controls {
                        selection: 0,
                        capture: None,
                    }
                }
                Transition::ControlGuide => {
                    state = State::ControlGuide {
                        selection: 0,
                        controller: false,
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
                    state = State::Skins {
                        selection,
                        ticks: 0,
                    };
                }
                Transition::PreviewSkin(selection) => {
                    assets.select_skin(&skins[selection], &config.game_dir)?;
                }
                Transition::CancelSkins => {
                    assets.select_skin(&config.settings.selected_skin, &config.game_dir)?;
                    state = State::Title {
                        selection: 2,
                        ticks: 0,
                    };
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
                Transition::Quit => break 'game,
            }
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
                *selection,
            ),
            State::Controls { selection, capture } => render_controls(
                &mut screen,
                &assets,
                &config.settings.key_bindings,
                *selection,
                *capture,
            ),
            State::ControlGuide {
                selection,
                controller,
            } => render_control_guide(&mut screen, &assets, &localization, *selection, *controller),
            State::Languages { selection } => render_languages(&mut screen, &assets, *selection),
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
                &packs,
                &config.settings.resource_packs,
                &pack_warnings,
                *selection,
            ),
            State::Playing { world, .. } => world.render(&mut screen, &assets),
        }
        presenter.present(&mut window, screen.pixels())?;
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
    let skins = skin_options(&config.game_dir);
    let mut screen = Screen::new();
    screen.clear(0x08080C);
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
        "achievements" => {
            render_achievements(&mut screen, &assets, &localization, &achievements, 0)
        }
        "controls" => render_controls(&mut screen, &assets, &config.settings.key_bindings, 0, None),
        "skins" => render_skins(&mut screen, &assets, &localization, &skins, 0, 160),
        _ => {
            return Err(format!(
                "unknown UI preview {name}; use options, play, new-world, worlds, help, book, achievements, controls, or skins"
            ));
        }
    }
    screen.save_png(output)
}

fn tick_title(
    selection: &mut usize,
    ticks: &mut u64,
    input: &Input,
    world_count: usize,
) -> Transition {
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
        0 if world_count == 0 => Transition::NewWorld(false),
        0 => Transition::PlayMenu,
        1 => Transition::Options,
        2 => Transition::Skins,
        3 => Transition::Achievements,
        4 => Transition::Help,
        _ => Transition::Quit,
    }
}

fn tick_play_menu(selection: &mut usize, input: &Input) -> Transition {
    const COUNT: usize = 2;
    if input.exit {
        return Transition::Title;
    }
    navigate(selection, input, COUNT);
    if input.select {
        match *selection {
            0 => Transition::Worlds,
            _ => Transition::NewWorld(true),
        }
    } else {
        Transition::None
    }
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
                let times = [10, 20, 40, 60, 120];
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
            7 => {
                settings.quests = !settings.quests;
                (Transition::None, true)
            }
            8 => {
                settings.tutorials = !settings.tutorials;
                (Transition::None, true)
            }
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
    const COUNT: usize = 12;
    if input.exit {
        return Transition::Options;
    }
    navigate(selection, input, COUNT);
    if input.select {
        match *selection {
            0..=9 => Transition::CaptureBinding(*selection),
            10 => Transition::ResetBindings,
            _ => Transition::Options,
        }
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
            1 => {
                settings.sound = !settings.sound;
                (Transition::None, true)
            }
            2 => {
                settings.show_quests = !settings.show_quests;
                (Transition::None, true)
            }
            3 => {
                settings.hwa = !settings.hwa;
                (Transition::None, true)
            }
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
    _ticks: u64,
) {
    screen.blit(
        &assets.title,
        (WIDTH as i32 - assets.title.width as i32) / 2,
        26 - assets.title.height as i32 / 2,
    );
    screen.centered_text(&assets.font, "Now with skins!", HEIGHT as i32 / 2 - 44);
    let version = localization.format("minicraft.displays.title.display.version", &["2.2.4"]);
    screen.text_colored(&assets.font, &version, 1, 1, 0x333333);
    let latest = localization.text("minicraft.displays.title.display.latest_already");
    screen.text_colored(
        &assets.font,
        latest,
        (WIDTH as i32 - latest.chars().count() as i32 * 8) / 2,
        76,
        0x333333,
    );
    for (index, key) in TITLE_KEYS.iter().enumerate() {
        let item = localization.text(key);
        let y = 96 + index as i32 * 10;
        if index == selection {
            let width = item.chars().count() as i32 * 8;
            let x = (WIDTH as i32 - width) / 2;
            screen.text(&assets.font, ">", x - 8, y);
            screen.text(&assets.font, "<", x + width, y);
        }
        let x = (WIDTH as i32 - item.chars().count() as i32 * 8) / 2;
        screen.text(&assets.font, item, x, y);
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
            &assets.font,
            &message,
            (WIDTH as i32 - message.chars().count() as i32 * 8) / 2,
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
        &assets.font,
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
            screen.text(&assets.font, ">", x - 8, y);
            screen.text(&assets.font, "<", x + label.chars().count() as i32 * 8, y);
        }
        screen.text(&assets.font, label, x, y);
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
        &assets.font,
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
            screen.text(&assets.font, ">", x - 8, y);
            screen.text(&assets.font, "<", x + label.chars().count() as i32 * 8, y);
        }
        screen.text(&assets.font, label, x, y);
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
        &assets.font,
        localization.text("minicraft.displays.world_select.select_world"),
        0,
    );
    if worlds.is_empty() {
        screen.centered_text(&assets.font, "NO WORLDS FOUND", 92);
    } else {
        let first = selection
            .saturating_sub(2)
            .min(worlds.len().saturating_sub(5));
        for (row, world) in worlds.iter().skip(first).take(5).enumerate() {
            let index = first + row;
            let label = &world.name;
            let y = 76 + row as i32 * 10;
            if index == selection {
                let width = label.chars().count() as i32 * 8;
                let x = (WIDTH as i32 - width) / 2;
                screen.text(&assets.font, ">", x - 8, y);
                screen.text(&assets.font, "<", x + width, y);
            }
            screen.centered_text(&assets.font, label, y);
        }
        screen.centered_text(
            &assets.font,
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
        screen.centered_text(&assets.font, text, y);
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
                &assets.font,
                localization.text("minicraft.displays.world_select.popups.display.change"),
                y + 12,
            );
            screen.centered_text(&assets.font, &format!("{}{}", popup.text, "_"), y + 30);
        }
        WorldPopupKind::Delete => {
            screen.centered_text(&assets.font, "ARE YOU SURE YOU WANT TO DELETE", y + 12);
            screen.centered_text(
                &assets.font,
                worlds
                    .get(selection)
                    .map(|world| world.name.as_str())
                    .unwrap_or(""),
                y + 26,
            );
            screen.centered_text(&assets.font, "THIS CAN NOT BE UNDONE!", y + 40);
        }
    }
    screen.centered_text(&assets.font, "ENTER TO CONFIRM", y + height - 22);
    screen.centered_text(&assets.font, "ESC TO CANCEL", y + height - 12);
}

fn render_help(screen: &mut Screen, assets: &Assets, selection: usize) {
    screen.centered_text(&assets.font, "HELP", 66);
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
        let y = 17 + index as i32 * 12;
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
        let index = 10 + offset;
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

fn render_control_guide(
    screen: &mut Screen,
    assets: &Assets,
    localization: &Localization,
    selection: usize,
    controller: bool,
) {
    screen.centered_text(
        &assets.font,
        localization.text("minicraft.displays.controls"),
        0,
    );
    screen.centered_text(
        &assets.font,
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
            let width = label.chars().count() as i32 * 8;
            let x = (WIDTH as i32 - width) / 2;
            screen.text(&assets.font, ">", x - 8, y);
            screen.text(&assets.font, "<", x + width, y);
        }
        screen.centered_text(&assets.font, label, y);
    }
    screen.centered_text(
        &assets.font,
        if controller {
            "CONTROLLER INPUTS"
        } else {
            "KEYBOARD INPUTS"
        },
        HEIGHT as i32 - 16,
    );
    screen.centered_text(&assets.font, "LEFT RIGHT TO SWITCH", HEIGHT as i32 - 8);
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
            let width = label.chars().count() as i32 * 8;
            let x = (WIDTH as i32 - width) / 2;
            screen.text(&assets.font, ">", x - 8, y);
            screen.text(&assets.font, "<", x + width, y);
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
    ticks: u64,
) {
    screen.centered_text(
        &assets.font,
        localization.text("minicraft.displays.skin"),
        16,
    );
    let (source_x, flip_x) = skin_preview_frame(ticks);
    screen.blit_region(
        &assets.skin,
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
                skin.chars().count() as i32 * 8
            };
            let x = (WIDTH as i32 - name_width) / 2;
            screen.text(&assets.font, ">", x - 8, y);
            screen.text(&assets.font, "<", x + name_width, y);
        }
        let name = if skin.starts_with("minicraft.skin.") {
            localization.text(skin)
        } else {
            skin
        };
        screen.centered_text(&assets.font, name, y);
    }
    screen.centered_text(&assets.font, "UP DOWN TO MOVE", HEIGHT as i32 - 17);
    screen.centered_text(&assets.font, "ENTER SELECT  ESC EXIT", HEIGHT as i32 - 9);
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

#[cfg(test)]
mod tests {
    use super::{
        TICK_INTERVAL, Transition, consume_fixed_updates, parse_world_seed, skin_preview_frame,
        tick_worlds,
    };
    use crate::{
        input::Input,
        world::{GameMode, WorldSpec},
        worlds::WorldRecord,
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
    fn render_fps_does_not_change_the_fixed_sixty_tps_clock() {
        let mut backlog = std::time::Duration::ZERO;
        let half_tick = TICK_INTERVAL / 2;
        let updates = (0..120)
            .map(|_| consume_fixed_updates(&mut backlog, half_tick))
            .sum::<usize>();
        assert_eq!(updates, 60);
        assert_eq!(backlog, std::time::Duration::ZERO);

        let mut low_fps_backlog = std::time::Duration::ZERO;
        assert_eq!(
            consume_fixed_updates(&mut low_fps_backlog, TICK_INTERVAL * 6),
            6
        );
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
