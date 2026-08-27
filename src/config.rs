use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::{Value, json};

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
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self {
            up: "W".to_owned(),
            down: "S".to_owned(),
            left: "A".to_owned(),
            right: "D".to_owned(),
            select: "ENTER".to_owned(),
            exit: "ESCAPE".to_owned(),
            attack: "C".to_owned(),
            menu: "X".to_owned(),
        }
    }
}

impl KeyBindings {
    pub const LABELS: [&'static str; 8] = [
        "MOVE UP",
        "MOVE DOWN",
        "MOVE LEFT",
        "MOVE RIGHT",
        "SELECT",
        "EXIT / PAUSE",
        "ATTACK",
        "INVENTORY",
    ];

    pub fn value(&self, index: usize) -> &str {
        match index {
            0 => &self.up,
            1 => &self.down,
            2 => &self.left,
            3 => &self.right,
            4 => &self.select,
            5 => &self.exit,
            6 => &self.attack,
            _ => &self.menu,
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
            _ => self.menu = value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub locale: String,
    pub fps: usize,
    pub difficulty: usize,
    pub sound: bool,
    pub autosave: bool,
    pub world_size: usize,
    pub theme: usize,
    pub terrain_type: usize,
    pub tutorials: bool,
    pub quests: bool,
    pub selected_skin: String,
    pub resource_packs: Vec<String>,
    pub key_bindings: KeyBindings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            locale: "en-us".to_owned(),
            fps: 60,
            difficulty: 1,
            sound: true,
            autosave: true,
            world_size: 128,
            theme: 0,
            terrain_type: 0,
            tutorials: false,
            quests: false,
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
        let settings = match fs::read_to_string(&path) {
            Ok(text) => parse_settings(&text).unwrap_or_default(),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        };
        Ok(Self { game_dir, settings })
    }

    pub fn save(&self) -> Result<(), String> {
        let value = json!({
            "version": "2.2.4-rust",
            "locale": self.settings.locale,
            "fps": self.settings.fps,
            "difficulty": self.settings.difficulty,
            "sound": self.settings.sound,
            "autosave": self.settings.autosave,
            "world_size": self.settings.world_size,
            "theme": self.settings.theme,
            "terrain_type": self.settings.terrain_type,
            "tutorials": self.settings.tutorials,
            "quests": self.settings.quests,
            "selected_skin": self.settings.selected_skin,
            "resource_packs": self.settings.resource_packs,
            "key_bindings": {
                "up": self.settings.key_bindings.up,
                "down": self.settings.key_bindings.down,
                "left": self.settings.key_bindings.left,
                "right": self.settings.key_bindings.right,
                "select": self.settings.key_bindings.select,
                "exit": self.settings.key_bindings.exit,
                "attack": self.settings.key_bindings.attack,
                "menu": self.settings.key_bindings.menu,
            },
        });
        let text = serde_json::to_string_pretty(&value).map_err(|error| error.to_string())?;
        let path = self.game_dir.join("settings.json");
        fs::write(&path, text).map_err(|error| format!("cannot write {}: {error}", path.display()))
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
            .join("Minicraft_Plus_Rust"));
    }
    let base = env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or(env::current_dir().map_err(|error| error.to_string())?);
    Ok(base
        .join(".playminicraft")
        .join("mods")
        .join("Minicraft_Plus_Rust"))
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
    settings.difficulty = integer(&value, "difficulty", settings.difficulty).min(2);
    settings.sound = boolean(&value, "sound", settings.sound);
    settings.autosave = boolean(&value, "autosave", settings.autosave);
    settings.world_size = match integer(&value, "world_size", settings.world_size) {
        128 | 256 | 512 => integer(&value, "world_size", settings.world_size),
        _ => 128,
    };
    settings.theme = integer(&value, "theme", settings.theme).min(4);
    settings.terrain_type = integer(&value, "terrain_type", settings.terrain_type).min(3);
    settings.tutorials = boolean(&value, "tutorials", settings.tutorials);
    settings.quests = boolean(&value, "quests", settings.quests);
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
    Some(settings)
}

fn binding(value: &Value, key: &str, default: &str) -> String {
    value[key]
        .as_str()
        .filter(|name| crate::input::key_from_name(name).is_some())
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
    use super::parse_settings;

    #[test]
    fn invalid_settings_are_clamped() {
        let settings =
            parse_settings(r#"{"locale":"fr-fr","fps":999,"difficulty":9,"world_size":17}"#)
                .unwrap();
        assert_eq!(settings.locale, "fr-fr");
        assert_eq!(settings.fps, 300);
        assert_eq!(settings.difficulty, 2);
        assert_eq!(settings.world_size, 128);
    }
}
