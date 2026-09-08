//! Java Updater's pre-autosave bed phase, separate from normal calendar ticks.
pub const END: u32 = 8_100;
pub const START: u32 = 40_500;

pub fn can_sleep(day_tick: u32, past_day_one: bool) -> bool {
    day_tick >= START || day_tick < END && past_day_one
}

pub struct SleepStep {
    pub day_tick: u32,
    pub passed_midnight: bool,
    pub wake: bool,
}

pub fn advance(day_tick: u32) -> SleepStep {
    // Preserve the order of the two Java conditions, including the exact END
    // boundary: values greater than END reset before the wake check.
    let passed_midnight = day_tick > END;
    let day_tick = if passed_midnight { 0 } else { day_tick };
    SleepStep {
        day_tick,
        passed_midnight,
        wake: (END..=START).contains(&day_tick),
    }
}
