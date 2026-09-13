//! Persistent game settings and key bindings (Java Config/Settings).
use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

use crate::saveload::storage::{atomic_write, read_candidates_limited};
use crate::io::key_names::key_name_exists;

#[derive(Debug, Clone)]
pub struct KeyBindings {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub select: String,
    pub exit: String,
    pub attack: String,
    pub menu: String,
    pub craft: String,
    pub pickup: String,
    pub searcher_bar: String,
    pub page_up: String,
    pub page_down: String,
    pub toggle_hud: String,
    pub screenshot: String,
    pub fullscreen: String,
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            // Java InputHandler.initKeyMap keymap defaults (order and aliases).
            up: "UP|W".to_owned(),
            down: "DOWN|S".to_owned(),
            left: "LEFT|A".to_owned(),
            right: "RIGHT|D".to_owned(),
            select: "ENTER".to_owned(),
            exit: "ESCAPE".to_owned(),
            attack: "C|SPACE|ENTER".to_owned(),
            menu: "X|E".to_owned(),
            craft: "Z|SHIFT-E".to_owned(),
            pickup: "V|P".to_owned(),
            searcher_bar: "SHIFT-F".to_owned(),
            page_up: "PAGE_UP".to_owned(),
            page_down: "PAGE_DOWN".to_owned(),
            toggle_hud: "F1".to_owned(),
            screenshot: "F2".to_owned(),
            fullscreen: "F11".to_owned(),
        }
    }
}

impl KeyBindings {
    /// Action → physical binding, matching Java `getMappedKey` substitution.
    /// Cursor actions stay arrow-only so menus never share WASD with movement.
    pub fn mapping(&self, action: &str) -> Option<String> {
        if let Some(index) = Self::LABELS.iter().position(|label| *label == action) {
            return Some(self.value(index).to_owned());
        }
        match action {
            "CURSOR-UP" => Some("UP".into()),
            "CURSOR-DOWN" => Some("DOWN".into()),
            "CURSOR-LEFT" => Some("LEFT".into()),
            "CURSOR-RIGHT" => Some("RIGHT".into()),
            _ => None,
        }
    }
    pub const LABELS: [&'static str; 16] = [
        "MOVE-UP",
        "MOVE-DOWN",
        "MOVE-LEFT",
        "MOVE-RIGHT",
        "SELECT",
        "EXIT",
        "ATTACK",
        "MENU",
        "CRAFT",
        "PICKUP",
        "SEARCHER-BAR",
        "PAGE-UP",
        "PAGE-DOWN",
        "TOGGLEHUD",
        "SCREENSHOT",
        "FULLSCREEN",
    ];

    /// Java `InputHandler.getMapping` display form (`|` alternatives → `/`).
    pub fn display_mapping(&self, action: &str) -> String {
        self.mapping(action)
            .map(|value| value.replace('|', "/"))
            .unwrap_or_else(|| "NO_KEY".to_owned())
    }

    pub fn value(&self, index: usize) -> &str {
        match index {
            0 => &self.up,
            1 => &self.down,
            2 => &self.left,
            3 => &self.right,
            4 => &self.select,
            5 => &self.exit,
            6 => &self.attack,
            7 => &self.menu,
            8 => &self.craft,
            9 => &self.pickup,
            10 => &self.searcher_bar,
            11 => &self.page_up,
            12 => &self.page_down,
            13 => &self.toggle_hud,
            14 => &self.screenshot,
            _ => &self.fullscreen,
        }
    }

    pub fn set(&mut self, index: usize, value: String) {
        match index {
            0 => self.up = value,
            1 => self.down = value,
            2 => self.left = value,
            3 => self.right = value,
            4 => self.select = value,
            5 => self.exit = value,
            6 => self.attack = value,
            7 => self.menu = value,
            8 => self.craft = value,
            9 => self.pickup = value,
            10 => self.searcher_bar = value,
            11 => self.page_up = value,
            12 => self.page_down = value,
            13 => self.toggle_hud = value,
            14 => self.screenshot = value,
            _ => self.fullscreen = value,
        }
    }

    pub fn add(&mut self, index: usize, value: String) {
        let current = self.value(index);
        if current
            .split('|')
            .any(|key| key.eq_ignore_ascii_case(&value))
        {
            return;
        }
        self.set(index, format!("{current}|{value}"));
    }

    /// Controls-screen string: the stored keymap value, as Java `KeyInputEntry`
    /// renders `getKeyPrefs()` (pipe-joined, no synthesized aliases).
    pub fn display_value(&self, index: usize) -> String {
        self.value(index).to_owned()
    }

    /// One-time materialization of pre-format-2 alias-imposed bindings.
    /// Older Rust clients displayed MOVE-UP as `W|UP` while storing only `W`.
    fn legacy_display_value(&self, index: usize) -> String {
        let configured = self.value(index);
        let aliases: &[&str] = match index {
            0 => &["UP"],
            1 => &["DOWN"],
            2 => &["LEFT"],
            3 => &["RIGHT"],
            6 => &["SPACE", "ENTER"],
            7 => &["E"],
            8 => &["SHIFT-E"],
            9 => &["P"],
            _ => &[],
        };
        let mut values: Vec<&str> = configured.split('|').collect();
        for alias in aliases {
            if !values
                .iter()
                .any(|value| value.eq_ignore_ascii_case(alias))
            {
                values.push(alias);
            }
        }
        values.join("|")
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub locale: String,
    pub fps: usize,
    pub hwa: bool,
    pub difficulty: usize,
    pub sound: bool,
    pub autosave: bool,
    pub world_size: usize,
    pub theme: usize,
    pub terrain_type: usize,
    pub game_mode: usize,
    pub score_minutes: usize,
    pub score_time_10_unlocked: bool,
    pub score_time_120_unlocked: bool,
    pub tutorials: bool,
    pub quests: bool,
    pub show_quests: bool,
    pub selected_skin: String,
    pub resource_packs: Vec<String>,
    pub key_bindings: KeyBindings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: "en-us".to_owned(),
            fps: 60,
            hwa: false,
            difficulty: 1,
            sound: true,
            autosave: true,
            world_size: 128,
            theme: 0,
            terrain_type: 0,
            game_mode: 0,
            score_minutes: 20,
            score_time_10_unlocked: false,
            score_time_120_unlocked: false,
            tutorials: false,
            quests: false,
            show_quests: true,
            selected_skin: "minicraft.skin.paul".to_owned(),
            resource_packs: Vec::new(),
            key_bindings: KeyBindings::default(),
        }
    }
}

pub struct Config {
    pub game_dir: PathBuf,
    pub settings: Settings,
}

impl Config {
    pub fn load(arguments: &[String]) -> Result<Self, String> {
        let game_dir = determine_game_dir(arguments)?;
        fs::create_dir_all(&game_dir).map_err(|error| {
            format!(
                "cannot create game directory {}: {error}",
                game_dir.display()
            )
        })?;
        fs::create_dir_all(game_dir.join("resourcepacks"))
            .map_err(|error| format!("cannot create resource pack directory: {error}"))?;
        fs::create_dir_all(game_dir.join("skins"))
            .map_err(|error| format!("cannot create skin directory: {error}"))?;

        let path = game_dir.join("settings.json");
        let settings = read_candidates_limited(&path, 1024 * 1024)?
            .into_iter()
            .find_map(|(_, text)| parse_settings(&text))
            .unwrap_or_default();
        Ok(Self { game_dir, settings })
    }

    pub fn save(&self) -> Result<(), String> {
        let value = json!({
            "version": "2.2.4-rust",
            "locale": self.settings.locale,
            "fps": self.settings.fps,
            "hwa": self.settings.hwa,
            "difficulty": self.settings.difficulty,
            "sound": self.settings.sound,
            "autosave": self.settings.autosave,
            "world_size": self.settings.world_size,
            "theme": self.settings.theme,
            "terrain_type": self.settings.terrain_type,
            "game_mode": self.settings.game_mode,
            "score_minutes": self.settings.score_minutes,
            "score_time_10_unlocked": self.settings.score_time_10_unlocked,
            "score_time_120_unlocked": self.settings.score_time_120_unlocked,
            "tutorials": self.settings.tutorials,
            "quests": self.settings.quests,
            "show_quests": self.settings.show_quests,
            "selected_skin": self.settings.selected_skin,
            "resource_packs": self.settings.resource_packs,
            "key_bindings": {
                "format": 2,
                "up": self.settings.key_bindings.up,
                "down": self.settings.key_bindings.down,
                "left": self.settings.key_bindings.left,
                "right": self.settings.key_bindings.right,
                "select": self.settings.key_bindings.select,
                "exit": self.settings.key_bindings.exit,
                "attack": self.settings.key_bindings.attack,
                "menu": self.settings.key_bindings.menu,
                "craft": self.settings.key_bindings.craft,
                "pickup": self.settings.key_bindings.pickup,
                "searcher_bar": self.settings.key_bindings.searcher_bar,
                "page_up": self.settings.key_bindings.page_up,
                "page_down": self.settings.key_bindings.page_down,
                "toggle_hud": self.settings.key_bindings.toggle_hud,
                "screenshot": self.settings.key_bindings.screenshot,
                "fullscreen": self.settings.key_bindings.fullscreen,
            },
        });
        let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
        let path = self.game_dir.join("settings.json");
        atomic_write(&path, text.as_bytes())
    }
}

fn determine_game_dir(arguments: &[String]) -> Result<PathBuf, String> {
    if let Some(index) = arguments
        .iter()
        .position(|argument| argument.eq_ignore_ascii_case("--savedir"))
    {
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| "--savedir requires a directory path".to_owned())?;
        return absolute(Path::new(value));
    }

    if cfg!(target_os = "windows")
        && let Some(app_data) = env::var_os("APPDATA")
    {
        return Ok(PathBuf::from(app_data)
            .join("playminicraft")
            .join("mods")
            .join("Minicraft_Rust"));
    }
    let base = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(|error| error.to_string())?);
    Ok(base
        .join(".playminicraft")
        .join("mods")
        .join("Minicraft_Rust"))
}

fn absolute(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(path.to_owned())
    } else {
        Ok(env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path))
    }
}

fn parse_settings(text: &str) -> Option<Settings> {
    let value: Value = serde_json::from_str(text).ok()?;
    let mut settings = Settings::default();
    settings.locale = string(&value, "locale", &settings.locale);
    settings.fps = integer(&value, "fps", settings.fps).clamp(10, 300);
    settings.hwa = boolean(&value, "hwa", settings.hwa);
    settings.difficulty = integer(&value, "difficulty", settings.difficulty).min(2);
    settings.sound = boolean(&value, "sound", settings.sound);
    settings.autosave = boolean(&value, "autosave", settings.autosave);
    settings.world_size = match integer(&value, "world_size", settings.world_size) {
        128 | 256 | 512 => integer(&value, "world_size", settings.world_size),
        _ => 128,
    };
    settings.theme = integer(&value, "theme", settings.theme).min(4);
    settings.terrain_type = integer(&value, "terrain_type", settings.terrain_type).min(3);
    settings.game_mode = integer(&value, "game_mode", settings.game_mode).min(3);
    settings.score_minutes = match integer(&value, "score_minutes", settings.score_minutes) {
        10 | 20 | 40 | 60 | 120 => integer(&value, "score_minutes", settings.score_minutes),
        _ => 20,
    };
    settings.tutorials = boolean(&value, "tutorials", settings.tutorials);
    // Preserve a previously selected legacy duration without unlocking every
    // duration that the old Rust menu incorrectly exposed.
    settings.score_time_10_unlocked = boolean(
        &value,
        "score_time_10_unlocked",
        settings.score_minutes == 10,
    );
    settings.score_time_120_unlocked = boolean(
        &value,
        "score_time_120_unlocked",
        settings.score_minutes == 120,
    );
    settings.quests = boolean(&value, "quests", settings.quests);
    settings.show_quests = boolean(&value, "show_quests", settings.show_quests);
    settings.selected_skin = string(&value, "selected_skin", &settings.selected_skin);
    settings.resource_packs = value["resource_packs"]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let bindings = &value["key_bindings"];
    settings.key_bindings.up = binding(bindings, "up", &settings.key_bindings.up);
    settings.key_bindings.down = binding(bindings, "down", &settings.key_bindings.down);
    settings.key_bindings.left = binding(bindings, "left", &settings.key_bindings.left);
    settings.key_bindings.right = binding(bindings, "right", &settings.key_bindings.right);
    settings.key_bindings.select = binding(bindings, "select", &settings.key_bindings.select);
    settings.key_bindings.exit = binding(bindings, "exit", &settings.key_bindings.exit);
    settings.key_bindings.attack = binding(bindings, "attack", &settings.key_bindings.attack);
    settings.key_bindings.menu = binding(bindings, "menu", &settings.key_bindings.menu);
    settings.key_bindings.craft = binding(bindings, "craft", &settings.key_bindings.craft);
    settings.key_bindings.pickup = binding(bindings, "pickup", &settings.key_bindings.pickup);
    settings.key_bindings.searcher_bar =
        binding(bindings, "searcher_bar", &settings.key_bindings.searcher_bar);
    settings.key_bindings.page_up = binding(bindings, "page_up", &settings.key_bindings.page_up);
    settings.key_bindings.page_down =
        binding(bindings, "page_down", &settings.key_bindings.page_down);
    settings.key_bindings.toggle_hud =
        binding(bindings, "toggle_hud", &settings.key_bindings.toggle_hud);
    settings.key_bindings.screenshot =
        binding(bindings, "screenshot", &settings.key_bindings.screenshot);
    settings.key_bindings.fullscreen =
        binding(bindings, "fullscreen", &settings.key_bindings.fullscreen);
    if bindings.is_object() && bindings["format"].as_u64().unwrap_or(1) < 2 {
        // Old clients imposed aliases outside the stored binding. Preserve
        // that effective configuration once, then allow real replacement.
        for index in 0..10 {
            let effective = settings.key_bindings.legacy_display_value(index);
            settings.key_bindings.set(index, effective);
        }
    }
    Some(settings)
}

fn binding(value: &Value, key: &str, default: &str) -> String {
    value[key]
        .as_str()
        .filter(|name| {
            !name.is_empty()
                && name.to_ascii_uppercase().split('|').all(|alternative| {
                    !alternative.is_empty()
                        && alternative.split(['-', '+']).all(|key| {
                            matches!(key, "SHIFT" | "CTRL" | "ALT")
                                || key_name_exists(key)
                        })
                })
        })
        .unwrap_or(default)
        .to_owned()
}

fn string(value: &Value, key: &str, default: &str) -> String {
    value[key].as_str().unwrap_or(default).to_owned()
}

fn integer(value: &Value, key: &str, default: usize) -> usize {
    value[key]
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .unwrap_or(default)
}

fn boolean(value: &Value, key: &str, default: bool) -> bool {
    value[key].as_bool().unwrap_or(default)
}

#[cfg(test)]
mod tests {
    use super::{KeyBindings, parse_settings};

    #[test]
    fn invalid_settings_are_clamped() {
        let settings = parse_settings(
            r#"{"locale":"fr-fr","fps":999,"hwa":true,"difficulty":9,"world_size":17}"#,
        )
        .unwrap();
        assert_eq!(settings.locale, "fr-fr");
        assert_eq!(settings.fps, 300);
        assert!(settings.hwa);
        assert_eq!(settings.difficulty, 2);
        assert_eq!(settings.world_size, 128);

        let phase_six = parse_settings(
            r#"{"game_mode":3,"score_minutes":60,"tutorials":true,"quests":true,"show_quests":false}"#,
        )
        .unwrap();
        assert_eq!(phase_six.game_mode, 3);
        assert_eq!(phase_six.score_minutes, 60);
        assert!(phase_six.tutorials);
        assert!(phase_six.quests);
        assert!(!phase_six.show_quests);
    }

    #[test]
    fn defaults_match_java_input_handler_keymap() {
        let bindings = KeyBindings::default();
        assert_eq!(bindings.up, "UP|W");
        assert_eq!(bindings.down, "DOWN|S");
        assert_eq!(bindings.left, "LEFT|A");
        assert_eq!(bindings.right, "RIGHT|D");
        assert_eq!(bindings.select, "ENTER");
        assert_eq!(bindings.exit, "ESCAPE");
        assert_eq!(bindings.attack, "C|SPACE|ENTER");
        assert_eq!(bindings.menu, "X|E");
        assert_eq!(bindings.craft, "Z|SHIFT-E");
        assert_eq!(bindings.pickup, "V|P");
        assert_eq!(bindings.searcher_bar, "SHIFT-F");
        assert_eq!(bindings.page_up, "PAGE_UP");
        assert_eq!(bindings.page_down, "PAGE_DOWN");
        assert_eq!(bindings.toggle_hud, "F1");
        assert_eq!(bindings.screenshot, "F2");
        assert_eq!(bindings.fullscreen, "F11");
        assert_eq!(bindings.display_value(0), "UP|W");
        assert_eq!(bindings.display_mapping("ATTACK"), "C/SPACE/ENTER");
        assert_eq!(bindings.display_mapping("CURSOR-UP"), "UP");
        assert_eq!(bindings.display_mapping("missing"), "NO_KEY");
    }

    #[test]
    fn key_bindings_support_java_style_alternatives() {
        let mut bindings = KeyBindings::default();
        bindings.add(0, "I".to_owned());
        bindings.add(0, "I".to_owned());
        assert_eq!(bindings.up, "UP|W|I");
        // display_value is the stored keymap, not a synthesized alias list.
        assert_eq!(bindings.display_value(0), "UP|W|I");
        assert_eq!(bindings.display_value(6), "C|SPACE|ENTER");
        // Overwrite drops the previous alternatives, like Java changeKeyBinding.
        bindings.set(0, "I".to_owned());
        assert_eq!(bindings.up, "I");
        assert_eq!(bindings.display_value(0), "I");
    }

    #[test]
    fn format_one_saves_materialize_legacy_aliases_once() {
        let settings = parse_settings(
            r#"{"key_bindings":{"format":1,"up":"W","down":"S","left":"A","right":"D",
                "select":"ENTER","exit":"ESCAPE","attack":"C","menu":"X","craft":"Z","pickup":"V"}}"#,
        )
        .unwrap();
        assert_eq!(settings.key_bindings.up, "W|UP");
        assert_eq!(settings.key_bindings.down, "S|DOWN");
        assert_eq!(settings.key_bindings.attack, "C|SPACE|ENTER");
        assert_eq!(settings.key_bindings.menu, "X|E");
        assert_eq!(settings.key_bindings.craft, "Z|SHIFT-E");
        assert_eq!(settings.key_bindings.pickup, "V|P");
        assert_eq!(settings.key_bindings.toggle_hud, "F1");

        let format_two = parse_settings(
            r#"{"key_bindings":{"format":2,"up":"I","toggle_hud":"F5","page_up":"W"}}"#,
        )
        .unwrap();
        assert_eq!(format_two.key_bindings.up, "I");
        assert_eq!(format_two.key_bindings.toggle_hud, "F5");
        assert_eq!(format_two.key_bindings.page_up, "W");
    }
}
