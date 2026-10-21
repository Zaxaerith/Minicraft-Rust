//! Explicit opt-in calendar contract. Values advance via the shared core Clock,
//! never wall time or GUI frames; old sessions have no Environment at all.
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Version {
    JavaDayTorchSlimeV1,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Environment {
    pub version: Version,
    pub day_tick: u32,
    pub game_time: i32,
    pub past_day_one: bool,
}
impl Environment {
    pub fn new() -> Self {
        Self {
            version: Version::JavaDayTorchSlimeV1,
            day_tick: 0,
            game_time: 0,
            past_day_one: false,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.day_tick >= crate::core::updater::DAY_LENGTH {
            return Err("invalid environment calendar".into());
        }
        Ok(())
    }
    pub fn advance(&mut self) {
        let mut clock = crate::core::clock::Clock {
            day_tick: self.day_tick,
            game_time: self.game_time,
        };
        self.past_day_one |= clock.advance_day(true);
        clock.advance_focused(true);
        self.day_tick = clock.day_tick;
        self.game_time = clock.game_time;
    }
    pub fn night(&self) -> bool {
        self.day_tick >= 48_600
    }
    pub fn label(&self) -> &'static str {
        ["MORNING", "DAY", "EVENING", "NIGHT"][(self.day_tick / 16_200) as usize]
    }
}
