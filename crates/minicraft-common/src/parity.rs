//! Stable, platform-independent records exchanged by parity tools.

use serde::{Deserialize, Serialize};

pub const PARITY_SCHEMA: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParityTrace {
    pub schema: u32,
    pub system: String,
    pub seed: i64,
    pub commands: Vec<ParityCommand>,
}

impl ParityTrace {
    pub fn validate_for(&self, expected_system: &str) -> Result<(), String> {
        if self.schema != PARITY_SCHEMA {
            return Err(format!(
                "unsupported parity schema {}; expected {PARITY_SCHEMA}",
                self.schema
            ));
        }
        if self.system != expected_system {
            return Err(format!(
                "trace system {:?} cannot be executed by {expected_system}",
                self.system
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ParityCommand {
    GameWorld {
        time: i32,
        calendar: u32,
        paused: bool,
    },
    GameTitle,
    GameMenu {
        focused: bool,
    },
    RenderLayers {
        ready: bool,
        gui: bool,
    },
    NotificationAdd {
        value: u32,
        reset: Option<i32>,
    },
    NotificationTick {
        ticks: u32,
    },
    NotificationRender {
        gui: bool,
        permanent: bool,
    },
    NotificationClear,
    CoreSetup {
        paused: bool,
        autosave_ticks: u32,
        score_ticks: i32,
        game_over: bool,
    },
    CoreTick {
        ticks: u32,
        focused: bool,
        has_display: bool,
        autosave: bool,
        alive: bool,
        score_mode: bool,
    },
    CoreSave,
    NextInt {
        bound: i32,
    },
    NextBool,
    NextFloat,
    AdvanceNanos {
        elapsed_nanos: u64,
    },
    FocusFrame {
        elapsed_nanos: u64,
        focused: bool,
        has_display: bool,
    },
    MenuInput {
        action: MenuAction,
    },
    DisplayInput {
        action: DisplayAction,
    },
    AudioSetup {
        scene: AudioScene,
    },
    AudioPlay {
        sound: AudioSound,
    },
    AudioMix {
        samples: u32,
    },
    PlayerInput {
        up: bool,
        down: bool,
        left: bool,
        right: bool,
    },
    SetPlayerStats {
        stamina: u8,
        stamina_recharge: u8,
        stamina_recharge_delay: u8,
    },
    SetObstacle {
        dx: i32,
        dy: i32,
        solid: bool,
    },
    CombatSetup {
        tool: CombatTool,
    },
    CombatAttack,
    CraftingSetup {
        station: CraftingStation,
        wood: u16,
        coal: u16,
        stone: u16,
        string: u16,
    },
    CraftingSelect {
        product: CraftingProduct,
    },
    CraftingCraft,
    RenderScene {
        scene: RenderScene,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatTool {
    None,
    WoodAxe,
    WoodSword,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DisplayAction {
    World,
    Title,
    Pause,
    Child,
    Update,
    Clear,
    ExitOne,
    ExitTwo,
    ExitZero,
    ExitMany,
    SelectNext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftingStation {
    Personal,
    Workbench,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CraftingProduct {
    Workbench,
    Plank,
    WoodAxe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RenderScene {
    NotificationsMultiple,
    NotificationsWrapped,
    NotificationsExpired,
    StatusSleeping,
    StatusSaving,
    Title,
    Pause,
    Inventory,
    Crafting,
    Hud,
    Focus,
    ExitConfirm,
    Quests,
    QuestsCompleted,
    QuestSeriesInfo,
    QuestSeriesViewer,
    QuestDetail,
    QuestTreeLinked,
    QuestTreeScrolled,
    QuestDetailCompleted,
    ResourcePacks,
    ResourcePacksReordered,
    ResourcePacksToggled,
    ResourcePacksDefault,
    ResourcePacksLogo,
    ResourcePacksInfo,
    ResourcePacksScrolled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioScene {
    Title,
    NewWorld,
    WorldSelect,
    WorldPopupValid,
    WorldPopupInvalid,
    Options,
    Languages,
    Skins,
    Achievements,
    Controls,
    ControlGuide,
    Book,
    ResourcePacks,
    Pause,
    Quests,
    Inventory,
    Crafting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioSound {
    BossDeath,
    Confirm,
    Craft,
    Death,
    Explode,
    Fuse,
    MonsterHurt,
    Pickup,
    PlayerHurt,
    Select,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MenuAction {
    None,
    Up,
    Down,
    Left,
    Right,
    Select,
    Exit,
    Attack,
    Menu,
    Craft,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParitySnapshot {
    pub schema: u32,
    pub system: String,
    pub tick: u64,
    pub observation: Observation,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Observation {
    GameClock {
        time: i32,
        calendar: u32,
        paused: bool,
        ready: bool,
    },
    RenderLayers {
        layers: Vec<String>,
    },
    Notifications {
        messages: Vec<String>,
        ticks: i32,
        ticking: bool,
        visible: Option<Vec<String>>,
    },
    CorePrelude {
        autosave_ticks: u32,
        calendar_ticks: u64,
        score_ticks: i32,
        game_over: bool,
        paused: bool,
        saves: Vec<String>,
    },
    FocusUpdate {
        updates: u64,
        calendar_ticks: u64,
        input_ticks: u64,
        paused: bool,
    },
    Display {
        #[serde(default)]
        ready: bool,
        current: Option<String>,
        requested: Option<String>,
        current_selection: u32,
        parents: Vec<String>,
        events: Vec<String>,
        sounds: Vec<String>,
    },
    Int {
        value: i32,
    },
    Bool {
        value: bool,
    },
    FloatBits {
        value: u32,
    },
    Ticks {
        updates: u32,
        total: u64,
    },
    Menu {
        screen: String,
        selection: u32,
        sounds: Vec<String>,
        outcome: String,
    },
    AudioEvent {
        scene: String,
        selection: u32,
        frame_sounds: Vec<String>,
        tick_sounds: Vec<String>,
        outcome: String,
    },
    AudioPcm {
        active_pointers: u32,
        offsets: Vec<u32>,
        sample_count: u32,
        pcm_hash: String,
        first_samples: Vec<i16>,
        minimum: i16,
        maximum: i16,
    },
    Player {
        x: i32,
        y: i32,
        direction: String,
        walk_distance: u32,
        stamina: u8,
        stamina_recharge: u8,
        stamina_recharge_delay: u8,
        step_count: u32,
    },
    Combat {
        tile: String,
        tile_damage: u16,
        stamina: u8,
        attack_time: u8,
        walk_distance: u32,
        tool_durability: i32,
        smash_count: u32,
        text_values: Vec<String>,
        sounds: Vec<String>,
    },
    Crafting {
        capacity: u32,
        used_slots: u32,
        selection: u32,
        items: Vec<CraftingCount>,
        tools: Vec<CraftingCount>,
        recipe_order: Vec<String>,
        craftable: Vec<bool>,
        selected_have: u32,
        selected_costs: Vec<CraftingCost>,
        sounds: Vec<String>,
    },
    RenderFrame {
        scene: String,
        width: u32,
        height: u32,
        rgb_hash: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftingCount {
    pub item: String,
    pub count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftingCost {
    pub item: String,
    pub have: u32,
    pub need: u32,
}

#[cfg(test)]
mod tests {
    use super::{PARITY_SCHEMA, ParityCommand, ParityTrace};

    #[test]
    fn trace_rejects_the_wrong_schema_or_system() {
        let mut trace = ParityTrace {
            schema: PARITY_SCHEMA,
            system: "java_random".to_owned(),
            seed: 0,
            commands: vec![ParityCommand::NextBool],
        };
        assert!(trace.validate_for("java_random").is_ok());
        trace.schema += 1;
        assert!(trace.validate_for("java_random").is_err());
        trace.schema = PARITY_SCHEMA;
        assert!(trace.validate_for("menu").is_err());
    }
}
