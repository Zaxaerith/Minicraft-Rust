//! Compatibility adapter: world actions/settings and quest data for PauseSession.
//! Ownership and parent restoration live in minicraft-client, not in World flags.

use super::{
    Input, PauseAction, PausePage, World, WorldAction, pause_actions, quest_tree, wrap_index,
};

impl World {
    pub(super) fn tick_pause(&mut self, input: &Input) -> WorldAction {
        if self.pause.view().confirm {
            if input.up_pressed || input.down_pressed {
                self.pause.view_mut().selection = usize::from(self.pause.view().selection == 0);
                self.sound(super::SoundEffect::Select);
            }
            if input.select {
                if self.pause.view().selection == 0 {
                    self.pause.back();
                    self.sound(super::SoundEffect::Confirm);
                } else {
                    self.pause.close();
                    self.sound(super::SoundEffect::Confirm);
                    return WorldAction::QuitWithoutSaving;
                }
            }
            return WorldAction::None;
        }
        if matches!(
            self.pause.view().page,
            PausePage::Quests
                | PausePage::QuestSeriesInfo
                | PausePage::QuestSeriesViewer
                | PausePage::QuestDetail
        ) {
            self.tick_pause_quests(input);
            return WorldAction::None;
        }
        let count = match self.pause.view().page {
            PausePage::Main => pause_actions(self.quests_enabled).len(),
            PausePage::Options => 5,
            PausePage::Achievements => 1,
            _ => unreachable!(),
        };
        // A saved cursor or a changed option list must never index out of bounds.
        self.pause.view_mut().selection = self.pause.view().selection.min(count - 1);
        if input.up_pressed {
            self.pause.view_mut().selection = self
                .pause
                .view()
                .selection
                .checked_sub(1)
                .unwrap_or(count - 1);
            self.sound(super::SoundEffect::Select);
        }
        if input.down_pressed {
            self.pause.view_mut().selection = (self.pause.view().selection + 1) % count;
            self.sound(super::SoundEffect::Select);
        }
        if self.pause.view().page == PausePage::Options {
            let direction = i32::from(input.right_pressed) - i32::from(input.left_pressed);
            match self.pause.view().selection {
                0 if direction != 0 => {
                    self.difficulty = wrap_index(self.difficulty, direction, 3);
                    self.sound(super::SoundEffect::Select);
                }
                1 if direction != 0 => {
                    self.show_quests = !self.show_quests;
                    self.sound(super::SoundEffect::Select);
                }
                2 if direction != 0 => {
                    self.tutorials_enabled = !self.tutorials_enabled;
                    self.sound(super::SoundEffect::Select);
                }
                3 if direction != 0 => {
                    self.quests_enabled = !self.quests_enabled;
                    self.sound(super::SoundEffect::Select);
                }
                _ => {}
            }
        }
        if input.select {
            match self.pause.view().page {
                PausePage::Main => {
                    match pause_actions(self.quests_enabled)[self.pause.view().selection] {
                        PauseAction::Return => {
                            self.pause.close();
                            self.sound(super::SoundEffect::Confirm);
                        }
                        PauseAction::Options => {
                            self.pause.push(PausePage::Options, 0);
                            self.sound(super::SoundEffect::Confirm);
                        }
                        PauseAction::Achievements => {
                            self.pause.push(PausePage::Achievements, 0);
                            self.sound(super::SoundEffect::Confirm);
                        }
                        PauseAction::Quests => {
                            self.pause.push(PausePage::Quests, 0);
                            self.pause.view_mut().quest_tab = 0;
                            self.sound(super::SoundEffect::Confirm);
                        }
                        PauseAction::Save => {
                            self.pause.close();
                            self.sound(super::SoundEffect::Confirm);
                            return WorldAction::SaveGame;
                        }
                        PauseAction::MainMenu => {
                            self.pause.confirm_exit();
                            self.sound(super::SoundEffect::Confirm);
                            // Java exit-to-title also fires the display exit sound.
                            self.sound(super::SoundEffect::Craft);
                        }
                    }
                }
                PausePage::Options => match self.pause.view().selection {
                    1 => {
                        self.show_quests = !self.show_quests;
                        self.sound(super::SoundEffect::Select);
                    }
                    2 => {
                        self.tutorials_enabled = !self.tutorials_enabled;
                        self.sound(super::SoundEffect::Select);
                    }
                    3 => {
                        self.quests_enabled = !self.quests_enabled;
                        self.sound(super::SoundEffect::Select);
                    }
                    4 => {
                        self.pause.back();
                        self.sound(super::SoundEffect::Confirm);
                    }
                    _ => {}
                },
                PausePage::Achievements => {}
                _ => unreachable!(),
            }
        }
        WorldAction::None
    }

    fn tick_pause_quests(&mut self, input: &Input) {
        match self.pause.view().page {
            PausePage::Quests => {
                if input.left_pressed && self.pause.view().quest_tab > 0 {
                    self.pause.view_mut().quest_tab -= 1;
                    self.pause.view_mut().selection = 0;
                    self.sound(super::SoundEffect::Select);
                }
                if input.right_pressed && self.pause.view().quest_tab < 1 {
                    self.pause.view_mut().quest_tab += 1;
                    self.pause.view_mut().selection = 0;
                    self.sound(super::SoundEffect::Select);
                }
                let groups = self.quest_group_views_for_current_tab();
                if groups.is_empty() {
                    self.pause.view_mut().selection = 0;
                    return;
                }
                self.pause.view_mut().selection = self.pause.view().selection.min(groups.len() - 1);
                if input.up_pressed {
                    self.pause.view_mut().selection = self
                        .pause
                        .view()
                        .selection
                        .checked_sub(1)
                        .unwrap_or(groups.len() - 1);
                    self.sound(super::SoundEffect::Select);
                }
                if input.down_pressed {
                    self.pause.view_mut().selection =
                        (self.pause.view().selection + 1) % groups.len();
                    self.sound(super::SoundEffect::Select);
                }
                if input.select {
                    self.pause.view_mut().quest_series_selection = self.pause.view().selection;
                    self.pause.view_mut().quest_group = groups[self.pause.view().selection].index;
                    self.pause.push(PausePage::QuestSeriesInfo, 0);
                    self.sound(super::SoundEffect::Confirm);
                }
            }
            PausePage::QuestSeriesInfo => {
                if input.select {
                    let first = self.quest_tree().first();
                    self.pause.push(PausePage::QuestSeriesViewer, first);
                    self.quest_tree_state.x_scroll = 0;
                    self.quest_tree_state.y_scroll = 0;
                    self.quest_tree_state.browsing = false;
                    self.sound(super::SoundEffect::Confirm);
                }
            }
            PausePage::QuestSeriesViewer => {
                if self.quest_tree().tick(
                    &mut self.pause.view_mut().selection,
                    &mut self.quest_tree_state,
                    input,
                ) == quest_tree::Action::Open
                {
                    self.pause.view_mut().quest_item = self.pause.view().selection;
                    self.pause
                        .push(PausePage::QuestDetail, self.pause.view().selection);
                }
            }
            PausePage::QuestDetail
            | PausePage::Main
            | PausePage::Options
            | PausePage::Achievements => {}
        }
    }
}
