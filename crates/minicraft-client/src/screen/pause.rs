//! Pause and child Display ownership, independent of World, devices and rendering.
//! Menu contents/settings remain in the compatibility adapter during migration.

use crate::{
    core::display_stack::DisplayStack,
    screen::display::{Display, ParentPolicy},
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PausePage {
    #[default]
    Main,
    Options,
    Achievements,
    Quests,
    QuestSeriesInfo,
    QuestSeriesViewer,
    QuestDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseAction {
    Return,
    Options,
    Achievements,
    Quests,
    Save,
    MainMenu,
}

pub fn pause_actions(quests_enabled: bool) -> Vec<PauseAction> {
    let mut actions = vec![
        PauseAction::Return,
        PauseAction::Options,
        PauseAction::Achievements,
    ];
    if quests_enabled {
        actions.push(PauseAction::Quests);
    }
    actions.extend([PauseAction::Save, PauseAction::MainMenu]);
    actions
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PauseView {
    pub page: PausePage,
    pub selection: usize,
    pub confirm: bool,
    pub quest_tab: usize,
    pub quest_group: usize,
    pub quest_item: usize,
    pub quest_series_selection: usize,
}

impl Display for PauseView {
    fn parent_policy(&self) -> ParentPolicy {
        if self.page == PausePage::Main && !self.confirm {
            ParentPolicy::Root
        } else {
            ParentPolicy::Inherit
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct PauseSession {
    stack: DisplayStack<PauseView>,
    inactive: PauseView,
}

impl PauseSession {
    pub fn begin_tick(&mut self) {
        self.stack.update();
    }
    pub fn is_open(&self) -> bool {
        self.stack.requested().is_some()
    }
    pub fn is_rendering(&self) -> bool {
        self.stack.current().is_some()
    }
    pub fn view(&self) -> &PauseView {
        self.stack.requested().unwrap_or(&self.inactive)
    }
    pub fn view_mut(&mut self) -> &mut PauseView {
        self.stack.requested_mut().unwrap_or(&mut self.inactive)
    }
    pub fn render_view(&self) -> &PauseView {
        self.stack.current().unwrap_or(&self.inactive)
    }
    pub fn open(&mut self) {
        self.stack.clear();
        self.stack.open(PauseView::default());
    }
    pub fn push(&mut self, page: PausePage, selection: usize) {
        self.stack.open(PauseView {
            page,
            selection,
            confirm: false,
            ..*self.view()
        });
    }
    pub fn confirm_exit(&mut self) {
        self.stack.open(PauseView {
            selection: 0,
            confirm: true,
            ..*self.view()
        });
    }
    /// Sound is intentionally NOT delivered here; the existing input-frame
    /// audio path already sends Java's exit sound once.
    pub fn back(&mut self) {
        self.stack.exit(1);
    }
    pub fn close(&mut self) {
        self.stack.clear();
    }

    /// Stable save projection of the requested path, including pending opens.
    /// PauseView hooks are pure, so reconciliation of the clone has no effects.
    /// Pending render state is transient and is not persisted.
    pub fn frames(&self) -> Vec<PauseView> {
        let mut stack = self.stack.clone();
        stack.update();
        let mut frames = Vec::new();
        let mut id = stack.requested_id();
        while let Some(current) = id {
            frames.push(*stack.get(current).unwrap());
            id = stack.parent_id(current);
        }
        frames.reverse();
        frames
    }

    pub fn from_frames(frames: &[PauseView]) -> Self {
        let mut session = Self::default();
        for frame in frames {
            session.stack.open(*frame);
            session.begin_tick();
        }
        session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_page_does_not_replace_rendered_parent_before_next_tick() {
        let mut pause = PauseSession::default();
        pause.open();
        assert!(pause.is_open());
        assert!(!pause.is_rendering());
        pause.begin_tick();
        pause.view_mut().selection = 1;
        pause.push(PausePage::Options, 0);
        assert_eq!(pause.view().page, PausePage::Options);
        assert_eq!(pause.render_view().page, PausePage::Main);
        pause.begin_tick();
        pause.back();
        assert_eq!(pause.view().selection, 1);
        assert_eq!(pause.render_view().page, PausePage::Options);
        pause.begin_tick();
        assert_eq!(pause.render_view().selection, 1);
    }

    #[test]
    fn popup_cancel_restores_parent_and_close_keeps_it_only_until_update() {
        let mut pause = PauseSession::default();
        pause.open();
        pause.begin_tick();
        pause.view_mut().selection = 5;
        pause.confirm_exit();
        pause.begin_tick();
        pause.back();
        assert_eq!(pause.view().selection, 5);
        assert!(!pause.view().confirm);
        pause.begin_tick();
        pause.close();
        assert!(!pause.is_open());
        assert!(pause.is_rendering());
        pause.begin_tick();
        assert!(!pause.is_rendering());
    }

    #[test]
    fn pending_save_projection_keeps_ancestors_without_advancing_live_stack() {
        let mut pause = PauseSession::default();
        pause.open();
        pause.begin_tick();
        pause.view_mut().selection = 3;
        pause.push(PausePage::Quests, 2);
        let frames = pause.frames();
        assert_eq!(frames.len(), 2);
        assert_eq!(pause.render_view().page, PausePage::Main);
        let mut restored = PauseSession::from_frames(&frames);
        restored.back();
        restored.begin_tick();
        assert_eq!(restored.view().selection, 3);
    }
}
