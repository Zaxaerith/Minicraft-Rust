//! Java `util/Logging.java` — tagged logger names and tinylog format flags.
//!
//! Rust has no tinylog; these tags are the stable string identities used by
//! parity hooks and future log sinks so call sites can stay 1:1 with Java.

/// Java `Logging` format flags (from `assets/tinylog.properties`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LoggingFlags {
    pub log_time: bool,
    pub log_thread: bool,
    /// Applied only when debug mode is enabled.
    pub log_trace: bool,
    pub log_level: bool,
    pub file_log_full: bool,
}

/// Java `Logging` tag constants.
pub mod tags {
    pub const UNTAGGED: &str = "";
    pub const GAMEHANDLER: &str = "Game Handler";
    pub const CRASHHANDLER: &str = "Crash Handler";
    pub const WORLD: &str = "World";
    pub const RESOURCEHANDLER: &str = "Resource Handler";
    pub const RESOURCEHANDLER_LOCALIZATION: &str = "Resource Handler/Localization";
    pub const RESOURCEHANDLER_SOUND: &str = "Resource Handler/Sound";
    pub const RESOURCEHANDLER_SKIN: &str = "Resource Handler/Skin";
    pub const RESOURCEHANDLER_RESOURCEPACK: &str = "Resource Handler/Resource Pack";
    pub const SAVELOAD: &str = "SaveLoad";
    pub const ACHIEVEMENT: &str = "Achievement";
    pub const ENTITY: &str = "Entity";
    pub const INVENTORY: &str = "Inventory";
    pub const ITEMS: &str = "Items";
    pub const TILES: &str = "Tiles";
    pub const NETWORK: &str = "Network";
    pub const QUEST: &str = "Quest";
    pub const TUTORIAL: &str = "Tutorial";
    pub const SPRITE: &str = "Sprite";
    pub const CONTROLLER: &str = "Controller";
    pub const PLAYER: &str = "Player";
}

#[cfg(test)]
mod tests {
    use super::tags;

    #[test]
    fn java_tag_names_match() {
        assert_eq!(tags::GAMEHANDLER, "Game Handler");
        assert_eq!(tags::SAVELOAD, "SaveLoad");
        assert_eq!(tags::QUEST, "Quest");
        assert_eq!(tags::TUTORIAL, "Tutorial");
    }
}
