//! Game.notifications and Updater.notetick/updateNoteTick, without UI or I/O.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Notifications {
    messages: Vec<String>,
    ticks: i32,
    ticking: bool,
}

impl Notifications {
    pub fn from_state(messages: Vec<String>, ticks: i32, ticking: bool) -> Self {
        Self {
            messages,
            ticks,
            ticking,
        }
    }
    pub fn messages(&self) -> &[String] {
        &self.messages
    }
    pub fn ticks(&self) -> i32 {
        self.ticks
    }
    pub fn ticking(&self) -> bool {
        self.ticking
    }
    /// Game.notifications.add does not reset the shared timer.
    pub fn add(&mut self, message: impl Into<String>) {
        self.messages.push(message.into());
    }
    /// Updater.notifyAll resets the shared timer, including negative delays.
    pub fn notify(&mut self, message: impl Into<String>, ticks: i32) {
        self.add(message);
        self.ticks = ticks;
    }
    /// World.initWorld clears the list, not the Updater timer/feedback flag.
    pub fn clear(&mut self) {
        self.messages.clear();
    }
    pub fn tick(&mut self) {
        if self.ticking {
            self.ticks = self.ticks.wrapping_add(1);
        }
    }
    /// Called only when renderGui actually runs, before drawing a Display.
    pub fn prepare_gui(&mut self, permanent_status: bool) -> &[String] {
        self.ticking = false;
        if !permanent_status && !self.messages.is_empty() {
            self.ticking = true;
            if self.messages.len() > 3 {
                self.messages.drain(..self.messages.len() - 3);
            }
            if self.ticks > 180 {
                self.messages.remove(0);
                self.ticks = 0;
            }
            &self.messages
        } else {
            &[]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rendering_enables_ticks_and_only_expires_strictly_after_180() {
        let mut notes = Notifications::default();
        notes.add("First");
        notes.tick();
        assert_eq!(notes.ticks(), 0);
        notes.prepare_gui(false);
        for _ in 0..180 {
            notes.tick();
        }
        assert_eq!(notes.prepare_gui(false), ["First"]);
        notes.tick();
        assert!(notes.prepare_gui(false).is_empty());
        assert!(notes.ticking()); // Java leaves true until the NEXT empty render.
        notes.tick();
        notes.prepare_gui(false);
        assert_eq!(notes.ticks(), 1);
        assert!(!notes.ticking());
    }
    #[test]
    fn truncation_happens_only_in_unblocked_gui_and_before_expiry() {
        let mut notes = Notifications::from_state(vec![], 181, true);
        for label in ["A", "B", "C", "D", "E"] {
            notes.add(label);
        }
        assert!(notes.prepare_gui(true).is_empty());
        assert_eq!(notes.messages().len(), 5);
        notes.tick();
        assert_eq!(notes.ticks(), 181);
        assert_eq!(notes.prepare_gui(false), ["D", "E"]);
        assert_eq!(notes.ticks(), 0);
    }
    #[test]
    fn append_preserves_timer_notify_resets_it_and_clear_keeps_feedback() {
        let mut notes = Notifications::from_state(vec![], 123, true);
        notes.add("A");
        assert_eq!(notes.ticks(), 123);
        notes.notify("B", -400);
        for _ in 0..580 {
            notes.tick();
        }
        assert_eq!(notes.prepare_gui(false), ["A", "B"]);
        notes.tick();
        assert_eq!(notes.prepare_gui(false), ["B"]);
        notes.clear();
        notes.tick();
        assert_eq!(notes.ticks(), 1);
    }
}
