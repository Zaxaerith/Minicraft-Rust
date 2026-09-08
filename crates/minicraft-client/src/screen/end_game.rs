//! Java 2.2.4 EndGameDisplay timing and score settlement.

/// Constructor-time result, independent of the player's score and inventory.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScoreSettlement {
    pub player_score: i32,
    pub final_score: i32,
}

impl ScoreSettlement {
    /// Counts are ordered Cloth, Slime, Bone, Arrow, Gunpowder, Antidious.
    /// Java draws once per item, including items with zero inventory count.
    pub fn calculate(
        player_score: i32,
        counts: [i32; 6],
        mut next_int: impl FnMut(i32) -> i32,
    ) -> Self {
        let final_score = counts.into_iter().fold(player_score, |score, count| {
            score.wrapping_add(count.wrapping_mul(next_int(2) + 1).wrapping_mul(10))
        });
        Self {
            player_score,
            final_score,
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EndGameDisplay {
    pub active: bool,
    pub display_timer: u8,
    pub input_delay: u8,
    pub settlement: Option<ScoreSettlement>,
    pub unlocked_minutes: Option<u8>,
    pub preferences_written: bool,
}

impl Default for EndGameDisplay {
    fn default() -> Self {
        Self {
            active: false,
            display_timer: 60,
            input_delay: 30,
            settlement: None,
            unlocked_minutes: None,
            preferences_written: false,
        }
    }
}

impl EndGameDisplay {
    /// Java compares the selected value, not its visibility; repeat unlock
    /// notices on subsequent qualifying runs are intentional.
    pub fn unlock_for(&self, score_minutes: usize) -> Option<u8> {
        let score = self.settlement?.final_score;
        match score_minutes {
            20 if score > 1000 => Some(10),
            60 if score > 100000 => Some(120),
            _ => None,
        }
    }
    pub fn begin_display_tick(&mut self) {
        self.active = true;
    }
    pub fn visible(&self) -> bool {
        self.active && self.display_timer == 0
    }

    /// super.tick runs BEFORE timer updates. Java's inputDelay field is never
    /// used as an input gate; canExit=false ignores the Escape action.
    pub fn tick(&mut self, select: bool) -> bool {
        if !self.active {
            return false;
        }
        if self.display_timer > 0 {
            self.display_timer -= 1;
        } else if self.input_delay > 0 {
            self.input_delay -= 1;
        }
        select
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn queued_display_waits_for_boundary_then_sixty_ticks_to_render() {
        let mut display = EndGameDisplay::default();
        assert!(!display.tick(true));
        assert_eq!(display.display_timer, 60);
        display.begin_display_tick();
        for _ in 0..59 {
            assert!(!display.tick(false));
        }
        assert!(!display.visible());
        display.tick(false);
        assert!(display.visible());
        assert_eq!(display.input_delay, 30);
        display.tick(false);
        assert_eq!(display.input_delay, 29);
    }
    #[test]
    fn hidden_menu_still_accepts_select_like_java() {
        let mut display = EndGameDisplay::default();
        display.begin_display_tick();
        assert!(display.tick(true));
        assert_eq!((display.display_timer, display.input_delay), (59, 30));
    }
}
