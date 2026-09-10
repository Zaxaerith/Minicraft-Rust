//! Java Menu: vertical list, scroll, search bar, frame/title, selection arrows.
use super::{
    entry::{COL_SLCT, Entry, EntryInput, EntryTick},
    rel_pos::RelPos,
};
use crate::gfx::{Image, Screen, font, font_style::FontStyle, gui};

/// Java Menu.LIMIT_TYPING_SEARCHER.
pub const LIMIT_TYPING_SEARCHER: usize = 22;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MenuEvent {
    None,
    Moved,
    Activated,
    Closed,
}

/// Host-facing input edges for Menu.tick (Java InputHandler subset).
#[derive(Clone, Default)]
pub struct MenuInput {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub exit: bool,
    pub shift_up: bool,
    pub shift_down: bool,
    pub page_up: bool,
    pub page_down: bool,
    pub search: bool,
    pub alt: bool,
    pub backspace: bool,
    pub typed_char: Option<char>,
    pub typed_text: String,
}

/// Shared vertical-list updater used by compatibility hosts during migration.
pub fn tick_selectable(
    selection: &mut usize,
    count: usize,
    up: bool,
    down: bool,
    select: bool,
    exit: bool,
) -> MenuEvent {
    if exit {
        return MenuEvent::Closed;
    }
    if count == 0 {
        return MenuEvent::None;
    }
    let delta = i32::from(down) - i32::from(up);
    if delta != 0 {
        *selection = (*selection as i32 + delta).rem_euclid(count as i32) as usize;
        return MenuEvent::Moved;
    }
    if select {
        MenuEvent::Activated
    } else {
        MenuEvent::None
    }
}

pub struct Menu {
    pub entries: Vec<Entry>,
    pub spacing: i32,
    pub bounds: (i32, i32, i32, i32),
    pub entry_bounds: (i32, i32, i32, i32),
    pub entry_pos: RelPos,
    pub title: String,
    pub title_color: u32,
    pub title_loc: (i32, i32),
    pub draw_vertically: bool,
    pub has_frame: bool,
    pub selectable: bool,
    pub should_render: bool,
    pub display_length: i32,
    pub padding: i32,
    pub wrap: bool,
    pub selection: usize,
    pub disp_selection: i32,
    pub offset: i32,
    pub use_searcher_bar: bool,
    pub searcher_bar_active: bool,
    typing_searcher: String,
    list_searcher: Vec<usize>,
    list_position_searcher: usize,
    selection_searcher: i32,
}

impl Default for Menu {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            spacing: 0,
            bounds: (0, 0, 0, 0),
            entry_bounds: (0, 0, 0, 0),
            entry_pos: RelPos::CENTER,
            title: String::new(),
            title_color: COL_SLCT,
            title_loc: (0, 0),
            draw_vertically: false,
            has_frame: false,
            selectable: false,
            should_render: true,
            display_length: 0,
            padding: 0,
            wrap: false,
            selection: 0,
            disp_selection: 0,
            offset: 0,
            use_searcher_bar: false,
            searcher_bar_active: false,
            typing_searcher: String::new(),
            list_searcher: Vec::new(),
            list_position_searcher: 0,
            selection_searcher: 0,
        }
    }
}

impl Menu {
    pub fn init(&mut self) {
        if self.entries.is_empty() {
            self.selection = 0;
            self.disp_selection = 0;
            self.offset = 0;
            return;
        }
        self.selection = self.selection.min(self.entries.len() - 1);
        if !self.entries[self.selection].is_selectable() {
            let prev = self.selection;
            loop {
                self.selection = (self.selection + 1) % self.entries.len();
                if self.entries[self.selection].is_selectable() || self.selection == prev {
                    break;
                }
            }
        }
        self.disp_selection = self.selection as i32;
        self.disp_selection = self
            .disp_selection
            .clamp(0, (self.display_length - 1).max(0));
        self.do_scroll();
    }

    pub fn set_selection(&mut self, idx: i32) {
        if self.entries.is_empty() {
            self.selection = 0;
            return;
        }
        self.selection = idx.clamp(0, self.entries.len() as i32 - 1) as usize;
        self.do_scroll();
    }

    pub fn cur_entry(&self) -> Option<&Entry> {
        self.entries.get(self.selection)
    }

    pub fn cur_entry_mut(&mut self) -> Option<&mut Entry> {
        self.entries.get_mut(self.selection)
    }

    fn do_scroll(&mut self) {
        // Java: (disp < pad || (!wrap && offset+dispLen > size)) && (wrap || offset > 0)
        self.disp_selection = self.selection as i32 - self.offset;
        let mut offset = self.offset;
        while (self.disp_selection < self.padding
            || (!self.wrap && offset + self.display_length > self.entries.len() as i32))
            && (self.wrap || offset > 0)
        {
            offset -= 1;
            self.disp_selection += 1;
        }
        while (self.display_length - self.disp_selection <= self.padding
            || (!self.wrap && offset < 0))
            && (self.wrap || offset + self.display_length < self.entries.len() as i32)
        {
            offset += 1;
            self.disp_selection -= 1;
        }
        let len = self.entries.len() as i32;
        if len > 0 {
            if offset < 0 {
                offset += len;
            }
            if offset > 0 {
                offset = offset % len;
            }
        }
        self.offset = offset;
    }

    /// Java Menu.tick. Returns sound events produced by real selection/action
    /// changes, not by a parallel frame-level predictor.
    pub fn tick(&mut self, input: &MenuInput) -> Vec<MenuEvent> {
        if !self.selectable || self.entries.is_empty() {
            return Vec::new();
        }
        let mut events = Vec::new();
        let prev_sel = self.selection;
        let mut selection = self.selection as i32;
        // Java: cursor-up/down; shift+cursor when searcher is idle moves by ±2.
        if input.up {
            selection -= 1;
        }
        if input.down {
            selection += 1;
        }
        let mut selection_searcher = self.selection_searcher;
        if input.shift_up && selection_searcher == 0 {
            selection_searcher -= 2;
        }
        if input.shift_down && selection_searcher == 0 {
            selection_searcher += 2;
        }
        if selection as usize != prev_sel && selection_searcher != 0 {
            selection = prev_sel as i32;
        }

        if self.use_searcher_bar {
            if input.search {
                self.searcher_bar_active = !self.searcher_bar_active;
            }
            if !self.list_searcher.is_empty() && selection_searcher == 0 {
                let speed = if input.page_up {
                    -1
                } else if input.page_down {
                    1
                } else {
                    0
                };
                if speed != 0 {
                    let mut list_position = self.list_position_searcher as i32 + speed;
                    if list_position < 0 {
                        list_position = self.list_searcher.len() as i32 - 1;
                    }
                    self.list_position_searcher =
                        list_position.rem_euclid(self.list_searcher.len().max(1) as i32) as usize;
                    if let Some(&position) = self.list_searcher.get(self.list_position_searcher) {
                        let difference = position as i32 - self.selection as i32;
                        selection_searcher = if difference > position as i32 {
                            -difference
                        } else {
                            difference
                        };
                    }
                }
            }
            if self.searcher_bar_active {
                if input.backspace {
                    self.typing_searcher.pop();
                } else if !input.typed_text.is_empty() {
                    for ch in input.typed_text.chars() {
                        self.typing_searcher.push(ch);
                    }
                } else if let Some(ch) = input.typed_char {
                    self.typing_searcher.push(ch);
                }
                if self.typing_searcher.chars().count() <= LIMIT_TYPING_SEARCHER {
                    self.list_searcher.clear();
                    self.list_position_searcher = 0;
                    let needle = self.typing_searcher.to_lowercase();
                    let mut should_select = true;
                    for (i, entry) in self.entries.iter().enumerate() {
                        if entry.text().to_lowercase().contains(&needle) {
                            if should_select && !needle.is_empty() {
                                let difference = i as i32 - self.selection as i32;
                                selection_searcher = if difference > i as i32 {
                                    -difference
                                } else {
                                    difference
                                };
                                should_select = false;
                            }
                            self.list_searcher.push(i);
                        }
                    }
                }
            }
            if selection_searcher != 0 {
                let down = selection_searcher > 0;
                selection_searcher += if down { -1 } else { 1 };
                selection += if down { 1 } else { -1 };
            }
        }
        self.selection_searcher = selection_searcher;

        let delta = selection - prev_sel as i32;
        if delta == 0 {
            let entry_input = EntryInput {
                left: input.left,
                right: input.right,
                select: input.select,
                backspace: input.backspace,
                typed: input.typed_char,
                typed_text: input.typed_text.clone(),
            };
            if let Some(entry) = self.entries.get_mut(self.selection) {
                match entry.tick(&entry_input) {
                    EntryTick::Select => events.push(MenuEvent::Moved),
                    EntryTick::Confirm => events.push(MenuEvent::Activated),
                    EntryTick::None => {}
                }
            }
            return events;
        }
        events.push(MenuEvent::Moved);

        let count = self.entries.len() as i32;
        let mut next = prev_sel as i32;
        loop {
            next = (next + delta).rem_euclid(count);
            if self.entries[next as usize].is_selectable() || next as i32 == prev_sel as i32 {
                break;
            }
        }
        self.selection = next as usize;
        self.disp_selection += self.selection as i32 - prev_sel as i32;
        self.disp_selection = self
            .disp_selection
            .clamp(0, (self.display_length - 1).max(0));
        self.do_scroll();
        events
    }

    pub fn render(&self, screen: &mut Screen, hud: &Image, font: &Image) {
        if !self.should_render {
            return;
        }
        self.render_frame(screen, hud);
        if !self.title.is_empty() {
            if self.draw_vertically {
                for (i, ch) in self.title.chars().enumerate() {
                    let text = ch.to_string();
                    if self.has_frame {
                        screen.blit_region(
                            hud,
                            self.title_loc.0,
                            self.title_loc.1 + i as i32 * 8,
                            24,
                            48,
                            8,
                            8,
                            false,
                        );
                    }
                    FontStyle::new(self.title_color).draw_at(
                        screen,
                        font,
                        &text,
                        self.title_loc.0,
                        self.title_loc.1 + i as i32 * 8,
                    );
                }
            } else {
                for (i, ch) in self.title.chars().enumerate() {
                    let text = ch.to_string();
                    let x = self.title_loc.0 + i as i32 * 8;
                    if self.has_frame {
                        screen.blit_region(hud, x, self.title_loc.1, 24, 48, 8, 8, false);
                    }
                    FontStyle::new(self.title_color).draw_at(
                        screen,
                        font,
                        &text,
                        x,
                        self.title_loc.1,
                    );
                }
            }
        }
        if self.searcher_bar_active && self.use_searcher_bar {
            let leading = self.typing_searcher.chars().count() as i32 * 8 / 2;
            let x_searcher = self.title_loc.0 + self.title.chars().count() as i32 * 8 / 2 - 16;
            let mut lead = leading;
            if x_searcher - lead < 0 {
                lead += x_searcher - lead;
            }
            let label = format!("> {} <", self.typing_searcher);
            let color = if self.typing_searcher.chars().count() < LIMIT_TYPING_SEARCHER {
                0xFF_FF_00
            } else {
                0xFF_00_00
            };
            FontStyle::new(color).draw_at(
                screen,
                font,
                &label,
                x_searcher - lead,
                self.title_loc.1 - 8,
            );
        }

        let mut y = self.entry_bounds.1;
        let special = self.wrap && (self.entries.len() as i32) < self.display_length;
        if special {
            let diff = self.display_length - self.entries.len() as i32;
            let extra = diff * (Entry::height() + self.spacing) / 2;
            y += extra;
        }
        let end = if self.wrap {
            self.offset + self.display_length
        } else {
            (self.offset + self.display_length).min(self.entries.len() as i32)
        };
        for i in self.offset..end {
            if special && i - self.offset >= self.entries.len() as i32 {
                break;
            }
            let idx = i.rem_euclid(self.entries.len() as i32) as usize;
            let entry = &self.entries[idx];
            if !entry.is_blank() {
                let size = (entry.width(), Entry::height());
                let row = (self.entry_bounds.0, y, self.entry_bounds.2, Entry::height());
                let pos = self.entry_pos.in_rect(size, row);
                let selected = idx == self.selection;
                if self.searcher_bar_active && self.use_searcher_bar {
                    entry.render(
                        screen,
                        font,
                        pos.0,
                        pos.1,
                        selected,
                        Some(self.typing_searcher.as_str()),
                        0xFF_FF_00,
                    );
                } else {
                    entry.render(screen, font, pos.0, pos.1, selected, None, 0);
                }
                if selected && entry.is_selectable() {
                    FontStyle::new(COL_SLCT).draw_at(screen, font, "> ", pos.0 - 16, y);
                    FontStyle::new(COL_SLCT).draw_at(screen, font, " <", pos.0 + entry.width(), y);
                }
            }
            y += Entry::height() + self.spacing;
        }
    }

    fn render_frame(&self, screen: &mut Screen, hud: &Image) {
        if !self.has_frame {
            return;
        }
        gui::render_menu_frame(
            screen,
            hud,
            self.bounds.0,
            self.bounds.1,
            self.bounds.2,
            self.bounds.3,
        );
    }
}

/// Java Menu.Builder: compute bounds/title/entry area, then init().
pub struct MenuBuilder {
    menu: Menu,
    set_selectable: bool,
    padding: f32,
    title_pos: RelPos,
    full_title_color: bool,
    set_title_color: bool,
    title_col: u32,
    anchor: (i32, i32),
    menu_pos: RelPos,
    menu_size: Option<(i32, i32)>,
    searcher_bar: bool,
}

impl MenuBuilder {
    pub fn new(
        has_frame: bool,
        entry_spacing: i32,
        entry_pos: RelPos,
        entries: Vec<Entry>,
    ) -> Self {
        let mut menu = Menu::default();
        menu.entries = entries;
        menu.has_frame = has_frame;
        menu.spacing = entry_spacing;
        menu.entry_pos = entry_pos;
        Self {
            menu,
            set_selectable: false,
            padding: 1.0,
            title_pos: RelPos::TOP,
            full_title_color: false,
            set_title_color: false,
            title_col: 0xFF_FF_00,
            anchor: (crate::gfx::WIDTH as i32 / 2, crate::gfx::HEIGHT as i32 / 2),
            menu_pos: RelPos::CENTER,
            menu_size: None,
            searcher_bar: false,
        }
    }

    pub fn set_entries(mut self, entries: Vec<Entry>) -> Self {
        self.menu.entries = entries;
        self
    }

    pub fn set_positioning(mut self, anchor: (i32, i32), menu_pos: RelPos) -> Self {
        self.anchor = anchor;
        self.menu_pos = menu_pos;
        self
    }

    pub fn set_size(mut self, width: i32, height: i32) -> Self {
        self.menu_size = Some((width, height));
        self
    }

    pub fn set_display_length(mut self, num: i32) -> Self {
        self.menu.display_length = num;
        self
    }

    pub fn set_title_pos(mut self, pos: RelPos) -> Self {
        self.title_pos = pos;
        self
    }

    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.menu.title = title.into();
        self
    }

    pub fn set_title_colored(mut self, title: impl Into<String>, color: u32) -> Self {
        self.menu.title = title.into();
        self.set_title_color = true;
        self.title_col = color;
        self
    }

    pub fn set_frame(mut self, has_frame: bool) -> Self {
        self.menu.has_frame = has_frame;
        self
    }

    pub fn set_scroll_policies(mut self, padding: f32, wrap: bool) -> Self {
        self.padding = padding;
        self.menu.wrap = wrap;
        self
    }

    pub fn set_should_render(mut self, render: bool) -> Self {
        self.menu.should_render = render;
        self
    }

    pub fn set_selectable(mut self, selectable: bool) -> Self {
        self.set_selectable = true;
        self.menu.selectable = selectable;
        self
    }

    pub fn set_selection(mut self, sel: i32) -> Self {
        self.menu.selection = sel.max(0) as usize;
        self
    }

    pub fn set_searcher_bar(mut self, enabled: bool) -> Self {
        self.searcher_bar = enabled;
        self
    }

    pub fn create_menu(mut self) -> Menu {
        if !self.set_selectable {
            self.menu.selectable = self.menu.entries.iter().any(Entry::is_selectable);
        }
        self.menu.draw_vertically =
            self.title_pos == RelPos::LEFT || self.title_pos == RelPos::RIGHT;
        let title_dim = if self.menu.draw_vertically {
            (16, font::text_width(&self.menu.title))
        } else {
            (font::text_width(&self.menu.title), 16)
        };

        // Insets: frame → 8; else title side 16; selectable → +16 left/right.
        let mut border = (0, 0, 0, 0);
        if self.menu.has_frame {
            border = (8, 8, 8, 8);
        } else if !self.menu.title.is_empty() && self.title_pos != RelPos::CENTER {
            let space = 16;
            if self.title_pos.y_index() == 0 {
                border.0 = space;
            } else if self.title_pos.y_index() == 2 {
                border.1 = space;
            } else if self.title_pos.x_index() == 0 {
                border.2 = space;
            } else if self.title_pos.x_index() == 2 {
                border.3 = space;
            }
        }
        if self.menu.selectable {
            border.2 += 16;
            border.3 += 16;
        }
        if self.menu.wrap && self.menu.display_length > 0 {
            self.menu.display_length = self.menu.display_length.min(self.menu.entries.len() as i32);
        }

        let entry_size = if let Some((w, h)) = self.menu_size {
            (w - border.2 - border.3, h - border.0 - border.1)
        } else {
            let mut width = title_dim.0;
            for entry in &self.menu.entries {
                let mut entry_width = entry.width();
                if self.menu.selectable && !entry.is_selectable() {
                    entry_width = (entry_width - 32).max(0);
                }
                width = width.max(entry_width);
            }
            if self.menu.display_length > 0 {
                let height = (Entry::height() + self.menu.spacing) * self.menu.display_length
                    - self.menu.spacing;
                (width, height)
            } else {
                let max_height = match self.menu_pos.y_index() {
                    0 => self.anchor.1,
                    2 => crate::gfx::HEIGHT as i32 - self.anchor.1,
                    _ => (self.anchor.1.min(crate::gfx::HEIGHT as i32 - self.anchor.1)) * 2,
                };
                let max_height = max_height - border.0 - border.1;
                let entry_height = self.menu.spacing + Entry::height();
                let total = entry_height * self.menu.entries.len() as i32 - self.menu.spacing;
                let max_height = ((max_height + self.menu.spacing) / entry_height) * entry_height
                    - self.menu.spacing;
                (width, max_height.min(total))
            }
        };
        let menu_size = (
            entry_size.0 + border.2 + border.3,
            entry_size.1 + border.0 + border.1,
        );
        if self.menu.display_length <= 0 && !self.menu.entries.is_empty() {
            self.menu.display_length =
                (entry_size.1 + self.menu.spacing) / (Entry::height() + self.menu.spacing);
        }
        let top_left = self.menu_pos.around(menu_size, self.anchor);
        self.menu.bounds = (top_left.0, top_left.1, menu_size.0, menu_size.1);
        self.menu.entry_bounds = (
            top_left.0 + border.2,
            top_left.1 + border.0,
            entry_size.0,
            entry_size.1,
        );
        // Java: titlePos.positionRect(titleDim, menu.bounds) — relative to the
        // full menu box (including borders), not the top-left anchor point.
        self.menu.title_loc =
            self.title_pos
                .in_rect(title_dim, (top_left.0, top_left.1, menu_size.0, menu_size.1));
        // Java nudges corner titles one box inward.
        if self.title_pos.x_index() == 0 && self.title_pos.y_index() != 1 {
            self.menu.title_loc = (self.menu.title_loc.0 + 8, self.menu.title_loc.1);
        }
        if self.title_pos.x_index() == 2 && self.title_pos.y_index() != 1 {
            self.menu.title_loc = (self.menu.title_loc.0 - 8, self.menu.title_loc.1);
        }
        if !self.menu.title.is_empty() {
            if self.full_title_color {
                self.menu.title_color = self.title_col;
            } else {
                if !self.set_title_color {
                    self.title_col = if self.menu.has_frame {
                        0xFF_FF_00
                    } else {
                        COL_SLCT
                    };
                }
                self.menu.title_color = self.title_col;
            }
        }
        let padding = self.padding.clamp(0.0, 1.0);
        self.menu.padding = (padding * self.menu.display_length as f32 / 2.0).floor() as i32;
        self.menu.use_searcher_bar = self.searcher_bar;
        self.menu.init();
        self.menu
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_centers_selectable_list_with_frame() {
        let entries = vec![
            Entry::string("Header"),
            Entry::select("Play", true),
            Entry::select("Options", true),
        ];
        let menu = MenuBuilder::new(true, 0, RelPos::CENTER, entries)
            .set_title("Test")
            .create_menu();
        assert!(menu.selectable);
        assert!(menu.has_frame);
        assert_eq!(menu.entries.len(), 3);
        assert_eq!(menu.selection, 1); // first selectable
        assert!(menu.display_length > 0);
    }

    /// Java Menu.Builder titleLoc = titlePos.positionRect(titleDim, menu.bounds).
    /// TOP titles must be centered on the full menu box, not the left edge.
    #[test]
    fn top_title_is_centered_on_menu_bounds() {
        let entries = vec![
            Entry::input("Enter World Name", 36),
            Entry::select("Create World", false),
        ];
        let menu = MenuBuilder::new(false, 10, RelPos::LEFT, entries)
            .set_title("World Gen Options")
            .create_menu();
        let (mx, my, mw, _) = menu.bounds;
        let title_w = font::text_width(&menu.title);
        let expected_x = mx + mw / 2 - title_w / 2;
        assert_eq!(menu.title_loc.0, expected_x);
        assert_eq!(menu.title_loc.1, my); // TOP sits on the menu top edge
    }

    #[test]
    fn scroll_keeps_selection_visible() {
        let entries: Vec<Entry> = (0..20)
            .map(|i| Entry::select(format!("E{i}"), false))
            .collect();
        let menu = MenuBuilder::new(false, 0, RelPos::CENTER, entries)
            .set_display_length(5)
            .set_scroll_policies(0.0, false)
            .create_menu();
        let mut menu = menu;
        menu.set_selection(18);
        assert!(menu.selection >= 18);
        assert!(menu.offset + menu.display_length > menu.selection as i32 - 1);
    }

    /// Mirrors Java TitleDisplay: Menu.Builder(false, 2, CENTER, …)
    /// positioned at (w/2, h*3/5).
    #[test]
    fn title_like_menu_matches_java_center_anchor() {
        let entries = vec![
            Entry::string_colored("Checking…", 0x00_00_FF),
            Entry::blank(),
            Entry::select("Play", false),
            Entry::select("Options", false),
            Entry::select("Skins", false),
            Entry::select("Achievements", false),
            Entry::select("Help", false),
            Entry::select("Quit", false),
        ];
        let menu = MenuBuilder::new(false, 2, RelPos::CENTER, entries)
            .set_positioning(
                (
                    crate::gfx::WIDTH as i32 / 2,
                    crate::gfx::HEIGHT as i32 * 3 / 5,
                ),
                RelPos::CENTER,
            )
            .create_menu();
        assert_eq!(menu.selection, 2);
        assert_eq!(menu.spacing, 2);
        assert!(!menu.has_frame);
        assert!(menu.selectable);
        assert_eq!(
            menu.bounds.0 + menu.bounds.2 / 2,
            crate::gfx::WIDTH as i32 / 2
        );
        assert!(menu.display_length >= 5);
        assert!(menu.entry_bounds.2 < menu.bounds.2);
    }

    #[test]
    fn menu_tick_move_emits_moved_and_skips_blank() {
        let entries = vec![
            Entry::string("Header"),
            Entry::blank(),
            Entry::select("A", false),
            Entry::select("B", false),
        ];
        let mut menu = MenuBuilder::new(true, 0, RelPos::CENTER, entries)
            .set_title("T")
            .create_menu();
        assert_eq!(menu.selection, 2);
        let input = MenuInput {
            down: true,
            ..Default::default()
        };
        let events = menu.tick(&input);
        assert!(events.contains(&MenuEvent::Moved));
        assert_eq!(menu.selection, 3);
        let events = menu.tick(&input);
        assert!(events.contains(&MenuEvent::Moved));
        assert_eq!(menu.selection, 2);
    }

    #[test]
    fn array_entry_left_right_in_menu_emits_select() {
        let entries = vec![Entry::array(
            "Diff",
            vec!["Easy".into(), "Normal".into(), "Hard".into()],
            0,
            true,
        )];
        let mut menu = MenuBuilder::new(false, 0, RelPos::CENTER, entries).create_menu();
        let input = MenuInput {
            right: true,
            ..Default::default()
        };
        let events = menu.tick(&input);
        assert!(events.contains(&MenuEvent::Moved));
    }
}
