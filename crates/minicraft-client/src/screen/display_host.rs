//! Java Display host that owns one or more Menus (title, canExit, clearScreen).
use super::menu::{Menu, MenuEvent, MenuInput};
use super::rel_pos::RelPos;

/// Java Display: menus[], selection, canExit, clearScreen, tick/render loop.
pub struct MenuDisplay {
    pub menus: Vec<Menu>,
    pub selection: usize,
    pub can_exit: bool,
    pub clear_screen: bool,
    pub should_render: bool,
}

impl MenuDisplay {
    pub fn new(menus: Vec<Menu>) -> Self {
        Self {
            menus,
            selection: 0,
            can_exit: true,
            clear_screen: true,
            should_render: true,
        }
    }

    pub fn with_flags(mut self, clear_screen: bool, can_exit: bool) -> Self {
        self.clear_screen = clear_screen;
        self.can_exit = can_exit;
        self
    }

    pub fn current_menu(&self) -> Option<&Menu> {
        self.menus.get(self.selection)
    }

    pub fn menu_mut(&mut self, index: usize) -> Option<&mut Menu> {
        self.menus.get_mut(index)
    }

    pub fn init(&mut self) {
        for menu in &mut self.menus {
            menu.init();
        }
        if self.selection >= self.menus.len() {
            self.selection = 0;
        }
    }

    /// Java Display.tick: optional exit, multi-menu left/right, then current menu tick.
    /// Returns sound/menu events and a requested exit.
    pub fn tick(&mut self, input: &MenuInput) -> DisplayTick {
        let mut events = Vec::new();
        if self.can_exit && input.exit {
            return DisplayTick {
                events: vec![MenuEvent::Closed],
                exited: true,
                activated: false,
            };
        }
        if self.menus.is_empty() {
            return DisplayTick {
                events,
                exited: false,
                activated: false,
            };
        }
        if self.menus.len() > 1 && self.menus[self.selection].selectable {
            let prev = self.selection;
            if input.left {
                self.selection = self.selection.saturating_sub(1);
            }
            if input.right {
                self.selection += 1;
            }
            if self.selection != prev {
                events.push(MenuEvent::Moved);
                let delta = self.selection as i32 - prev as i32;
                let count = self.menus.len() as i32;
                let mut next = prev as i32;
                loop {
                    next = (next + delta).rem_euclid(count);
                    if self.menus[next as usize].selectable || next as i32 == prev as i32 {
                        break;
                    }
                }
                self.selection = next as usize;
            }
        }
        let menu_events = self.menus[self.selection].tick(input);
        let activated = menu_events.contains(&MenuEvent::Activated);
        events.extend(menu_events);
        DisplayTick {
            events,
            exited: false,
            activated,
        }
    }

    pub fn render_menus(
        &self,
        screen: &mut crate::gfx::Screen,
        hud: &crate::gfx::Image,
        font: &crate::gfx::Image,
    ) {
        if !self.should_render || self.menus.is_empty() {
            return;
        }
        // Render non-selected first so the current menu is on top.
        let n = self.menus.len();
        let mut idx = self.selection;
        for _ in 0..n {
            idx = (idx + 1) % n;
            if self.menus[idx].should_render {
                self.menus[idx].render(screen, hud, font);
            }
        }
        if self.menus[self.selection].should_render {
            self.menus[self.selection].render(screen, hud, font);
        }
    }

    /// Simple single-menu helper matching TitleDisplay's one Menu.Builder.
    pub fn single(menu: Menu, clear_screen: bool, can_exit: bool) -> Self {
        Self::new(vec![menu]).with_flags(clear_screen, can_exit)
    }
}

pub struct DisplayTick {
    pub events: Vec<MenuEvent>,
    pub exited: bool,
    pub activated: bool,
}

impl Default for DisplayTick {
    fn default() -> Self {
        Self {
            events: Vec::new(),
            exited: false,
            activated: false,
        }
    }
}

/// Convenience: TitleDisplay's default centered spacing=2 no-frame list.
pub fn centered_title_menu(entries: Vec<super::entry::Entry>) -> Menu {
    super::menu::MenuBuilder::new(false, 2, RelPos::CENTER, entries)
        .set_positioning(
            (
                crate::gfx::WIDTH as i32 / 2,
                crate::gfx::HEIGHT as i32 * 3 / 5,
            ),
            RelPos::CENTER,
        )
        .create_menu()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::screen::entry::Entry;

    #[test]
    fn single_menu_display_ticks_entries() {
        let menu = super::super::menu::MenuBuilder::new(
            false,
            2,
            RelPos::CENTER,
            vec![
                Entry::string("Header"),
                Entry::blank(),
                Entry::select("Play", false),
            ],
        )
        .create_menu();
        let mut display = MenuDisplay::single(menu, true, false);
        display.init();
        assert_eq!(display.selection, 0);
        let tick = display.tick(&MenuInput {
            down: true,
            ..Default::default()
        });
        assert!(tick.events.contains(&MenuEvent::Moved));
    }

    #[test]
    fn multi_menu_left_right_moves_selection() {
        let a = super::super::menu::MenuBuilder::new(false, 0, RelPos::CENTER, vec![])
            .set_selectable(true)
            .create_menu();
        let b = super::super::menu::MenuBuilder::new(false, 0, RelPos::CENTER, vec![])
            .set_selectable(true)
            .create_menu();
        let mut display = MenuDisplay::new(vec![a, b]);
        let tick = display.tick(&MenuInput {
            right: true,
            ..Default::default()
        });
        assert!(tick.events.contains(&MenuEvent::Moved));
        assert_eq!(display.selection, 1);
    }
}
