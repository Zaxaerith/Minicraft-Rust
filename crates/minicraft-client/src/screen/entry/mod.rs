//! Java ListEntry hierarchy for Menu items.
use crate::gfx::{Screen, font, font_style::FontStyle};

/// Java ListEntry.COL_UNSLCT / COL_SLCT (Color.GRAY / Color.WHITE).
pub const COL_UNSLCT: u32 = 0x99_99_99;
pub const COL_SLCT: u32 = 0xFF_FF_FF;
/// Java Color.GREEN / Color.RED / Color.CYAN (minicraft get(1,…) RGB bytes).
pub const COL_GREEN: u32 = 0x4D_D4_4D;
pub const COL_RED: u32 = 0xC6_2C_2C;
pub const COL_CYAN: u32 = 0x5A_CC_CC;

/// Menu-space input slice used by entries (Java InputHandler subset).
#[derive(Clone, Default)]
pub struct EntryInput {
    pub left: bool,
    pub right: bool,
    pub select: bool,
    pub backspace: bool,
    pub typed: Option<char>,
    pub typed_text: String,
}

/// Result of entry.tick so Menu can play sounds at the real event site.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryTick {
    None,
    /// Java ArrayEntry plays select when the option index changes.
    Select,
    /// SelectEntry plays confirm when select is pressed on an actionable entry.
    Confirm,
}

#[derive(Clone, Debug)]
pub enum Entry {
    Blank,
    String {
        text: String,
        color: u32,
    },
    Select {
        text: String,
        localize: bool,
        /// Java SelectEntry.getColor override (e.g. Create World is always CYAN).
        color: Option<u32>,
    },
    Boolean {
        label: String,
        value: bool,
        localize: bool,
    },
    Array {
        label: String,
        options: Vec<String>,
        selection: usize,
        visible: Vec<bool>,
        wrap: bool,
        localize: bool,
        /// ALT steps by 10 (RangeEntry).
        range_step: bool,
        /// Java ListEntry.setVisible(false): still occupies a menu row, draws nothing.
        hidden: bool,
    },
    Input {
        prompt: String,
        user_input: String,
        max_length: usize,
        /// Optional regex-like validation is applied by the host; empty = always valid.
        pattern: Option<String>,
    },
    /// Java KeyInputEntry: action label + mapping string, always full screen width.
    KeyInput {
        action: String,
        mapping: String,
        /// Red-highlighted alternate bindings (Java duplicated set).
        duplicated: Vec<String>,
    },
    /// Java ItemEntry / ItemListing: optional icon key + display text.
    Item {
        text: String,
        /// Sprite key under Item category; None skips the icon.
        asset_key: Option<String>,
        selectable: bool,
    },
    /// Java RecipeEntry: craftable gate changes selected/unselected color.
    Recipe {
        text: String,
        asset_key: Option<String>,
        can_craft: bool,
    },
    /// Java LinkEntry: SelectEntry with a fixed color (browser open is host-side).
    Link {
        text: String,
        url: String,
        color: u32,
        localize: bool,
    },
}

impl Entry {
    pub fn blank() -> Self {
        Self::Blank
    }

    pub fn string(text: impl Into<String>) -> Self {
        Self::String {
            text: text.into(),
            color: COL_UNSLCT,
        }
    }

    pub fn string_colored(text: impl Into<String>, color: u32) -> Self {
        Self::String {
            text: text.into(),
            color,
        }
    }

    /// Localizable string line; host passes already-resolved text when localize is false.
    pub fn string_localized(text: impl Into<String>, localize: bool) -> Self {
        Self::String {
            text: text.into(),
            color: COL_UNSLCT,
        }
        .with_localize_flag(localize)
    }

    fn with_localize_flag(self, _localize: bool) -> Self {
        // Localization is applied by the production host before construction.
        self
    }

    pub fn select(text: impl Into<String>, localize: bool) -> Self {
        Self::Select {
            text: text.into(),
            localize,
            color: None,
        }
    }

    /// Java WorldGenDisplay Create World: Font.draw(..., Color.CYAN).
    pub fn select_colored(text: impl Into<String>, color: u32) -> Self {
        Self::Select {
            text: text.into(),
            localize: false,
            color: Some(color),
        }
    }

    pub fn boolean(label: impl Into<String>, value: bool) -> Self {
        Self::Boolean {
            label: label.into(),
            value,
            localize: true,
        }
    }

    pub fn array(
        label: impl Into<String>,
        options: Vec<String>,
        selection: usize,
        wrap: bool,
    ) -> Self {
        let visible = vec![true; options.len()];
        Self::Array {
            label: label.into(),
            options,
            selection,
            visible,
            wrap,
            localize: true,
            range_step: false,
            hidden: false,
        }
    }

    pub fn range(label: impl Into<String>, min: i32, max: i32, initial: i32) -> Self {
        let options: Vec<String> = (min..=max).map(|v| v.to_string()).collect();
        let selection = (initial - min).max(0) as usize;
        Self::Array {
            label: label.into(),
            options,
            selection,
            visible: vec![true; (max - min + 1).max(0) as usize],
            wrap: false,
            localize: false,
            range_step: true,
            hidden: false,
        }
    }

    /// Java ListEntry.setVisible(false).
    pub fn set_hidden(&mut self, hidden: bool) {
        if let Self::Array { hidden: slot, .. } = self {
            *slot = hidden;
        }
    }

    pub fn input(prompt: impl Into<String>, max_length: usize) -> Self {
        Self::Input {
            prompt: prompt.into(),
            user_input: String::new(),
            max_length,
            pattern: None,
        }
    }

    pub fn key_input(action: impl Into<String>, mapping: impl Into<String>) -> Self {
        Self::KeyInput {
            action: action.into(),
            mapping: mapping.into(),
            duplicated: Vec::new(),
        }
    }

    pub fn item(text: impl Into<String>, asset_key: Option<String>) -> Self {
        Self::Item {
            text: text.into(),
            asset_key,
            selectable: true,
        }
    }

    pub fn item_listing(text: impl Into<String>, asset_key: Option<String>) -> Self {
        Self::Item {
            text: text.into(),
            asset_key,
            selectable: false,
        }
    }

    pub fn recipe(text: impl Into<String>, asset_key: Option<String>, can_craft: bool) -> Self {
        Self::Recipe {
            text: text.into(),
            asset_key,
            can_craft,
        }
    }

    pub fn link(text: impl Into<String>, url: impl Into<String>, color: u32) -> Self {
        Self::Link {
            text: text.into(),
            url: url.into(),
            color,
            localize: true,
        }
    }

    pub fn is_selectable(&self) -> bool {
        match self {
            Self::Blank | Self::String { .. } => false,
            Self::Item { selectable, .. } => *selectable,
            Self::Array { hidden, .. } => !*hidden,
            _ => true,
        }
    }

    pub fn is_blank(&self) -> bool {
        matches!(self, Self::Blank)
    }

    pub fn text(&self) -> String {
        match self {
            Self::Blank => " ".to_owned(),
            Self::String { text, .. } | Self::Select { text, .. } => text.clone(),
            Self::Boolean { label, value, .. } => {
                // Java BooleanEntry: localization keys boolean.true/false → On/Off.
                format!("{label}: {}", if *value { "On" } else { "Off" })
            }
            Self::Array {
                label,
                options,
                selection,
                ..
            } => {
                let option = options.get(*selection).cloned().unwrap_or_default();
                format!("{label}: {option}")
            }
            Self::Input {
                prompt, user_input, ..
            } => {
                if prompt.is_empty() {
                    user_input.clone()
                } else {
                    format!("{prompt}: {user_input}")
                }
            }
            Self::KeyInput {
                action,
                mapping,
                duplicated,
            } => {
                // Java pads with spaces so mapping sits on the right edge (Screen.w).
                let space = font::text_width(" ");
                let action_w = font::text_width(action);
                let map_plain = mapping
                    .split('|')
                    .map(|part| {
                        part.trim_start_matches(['§', 'a', 'b', 'c', 'd', 'e', 'f', 'r', 'g'])
                    })
                    .collect::<Vec<_>>()
                    .join("|");
                let map_w = font::text_width(&map_plain);
                let pad_count =
                    ((crate::gfx::WIDTH as i32) / space - action_w / space - map_w / space).max(0);
                let pad = " ".repeat(pad_count as usize);
                // Red duplicates use §-codes only in render; text() stays plain for width.
                let _ = duplicated;
                format!("{action}{pad}{map_plain}")
            }
            Self::Item { text, .. } | Self::Recipe { text, .. } => text.clone(),
            Self::Link { text, .. } => text.clone(),
        }
    }

    pub fn width(&self) -> i32 {
        match self {
            Self::Blank => 8,
            // Java KeyInputEntry.getWidth() is always Screen.w.
            Self::KeyInput { .. } => crate::gfx::WIDTH as i32,
            _ => font::text_width(&self.text()),
        }
    }

    pub fn height() -> i32 {
        crate::gfx::font_style::TEXT_HEIGHT
    }

    pub fn color(&self, selected: bool) -> u32 {
        match self {
            Self::String { color, .. } => *color,
            Self::Input { .. } => {
                if self.is_valid_input() {
                    if selected { COL_GREEN } else { COL_UNSLCT }
                } else {
                    COL_RED
                }
            }
            Self::Select { color: Some(color), .. } => *color,
            // Java RecipeEntry: craftable → white, else gray (ignores selection).
            Self::Recipe { can_craft, .. } => {
                if *can_craft {
                    COL_SLCT
                } else {
                    COL_UNSLCT
                }
            }
            Self::Link { color, .. } => *color,
            _ => {
                if selected {
                    COL_SLCT
                } else {
                    COL_UNSLCT
                }
            }
        }
    }

    fn is_valid_input(&self) -> bool {
        match self {
            Self::Input {
                user_input,
                pattern,
                ..
            } => match pattern {
                None => true,
                Some(p) if p.is_empty() => true,
                // Minimal host pattern: only reject empty when a non-empty pattern is set.
                // Full regex is applied by the production layer that owns InputEntry.
                Some(_) => !user_input.is_empty(),
            },
            _ => true,
        }
    }

    pub fn user_input(&self) -> &str {
        match self {
            Self::Input { user_input, .. } => user_input,
            _ => "",
        }
    }

    pub fn asset_key(&self) -> Option<&str> {
        match self {
            Self::Item { asset_key, .. } | Self::Recipe { asset_key, .. } => asset_key.as_deref(),
            _ => None,
        }
    }

    pub fn url(&self) -> &str {
        match self {
            Self::Link { url, .. } => url,
            _ => "",
        }
    }

    pub fn set_user_input(&mut self, value: impl Into<String>) {
        if let Self::Input {
            user_input,
            max_length,
            ..
        } = self
        {
            let mut next = value.into();
            if *max_length > 0 && next.chars().count() > *max_length {
                next = next.chars().take(*max_length).collect();
            }
            *user_input = next;
        }
    }

    /// Java ArrayEntry.tick: left/right change the option; play Select on change.
    pub fn tick(&mut self, input: &EntryInput) -> EntryTick {
        match self {
            Self::Blank | Self::String { .. } | Self::Item { .. } | Self::Recipe { .. }
            | Self::KeyInput { .. } => EntryTick::None,
            Self::Input {
                user_input,
                max_length,
                pattern,
                ..
            } => {
                // Java InputEntry.tick → InputHandler.addKeyTyped (printable + pattern).
                if input.backspace {
                    user_input.pop();
                }
                let typed: String = if !input.typed_text.is_empty() {
                    input.typed_text.clone()
                } else if let Some(ch) = input.typed {
                    ch.to_string()
                } else {
                    String::new()
                };
                for ch in typed.chars() {
                    if ch == '\x08' {
                        user_input.pop();
                        continue;
                    }
                    if ch.is_control() {
                        continue;
                    }
                    if let Some(pattern) = pattern.as_deref()
                        && !pattern.is_empty()
                        && !char_matches_pattern(ch, pattern)
                    {
                        continue;
                    }
                    if *max_length > 0 && user_input.chars().count() >= *max_length {
                        break;
                    }
                    user_input.push(ch);
                }
                EntryTick::None
            }
            Self::Select { .. } | Self::Link { .. } => {
                if input.select {
                    return EntryTick::Confirm;
                }
                EntryTick::None
            }
            Self::Boolean { value, .. } => {
                if input.left || input.right {
                    *value = !*value;
                    return EntryTick::Select;
                }
                if input.select {
                    return EntryTick::Confirm;
                }
                EntryTick::None
            }
            Self::Array {
                options,
                selection,
                visible,
                wrap,
                range_step,
                ..
            } => {
                let step = if *range_step && input.select {
                    10
                } else if *range_step {
                    1
                } else {
                    1
                };
                // RangeEntry uses ALT for *10; host folds that into `select` for step 10.
                let _ = step;
                let mut next = *selection as i32;
                let step = if *range_step {
                    // Production host passes select=true for ALT step; EntryInput has no ALT.
                    // Mirror ArrayEntry: left/right ±1 unless host multiplies via range_step+select.
                    if input.select { 10 } else { 1 }
                } else {
                    1
                };
                if input.left {
                    next -= step;
                }
                if input.right {
                    next += step;
                }
                if next as usize == *selection {
                    return EntryTick::None;
                }
                let len = options.len() as i32;
                if len == 0 {
                    return EntryTick::None;
                }
                loop {
                    if *wrap {
                        next = next.rem_euclid(len);
                    } else {
                        next = next.clamp(0, len - 1);
                    }
                    if visible.get(next as usize).copied().unwrap_or(true)
                        || next as usize == *selection
                    {
                        break;
                    }
                    if input.left {
                        next -= 1;
                    } else {
                        next += 1;
                    }
                    if next as usize == *selection {
                        break;
                    }
                }
                let next = next as usize;
                if next != *selection && next < options.len() {
                    *selection = next;
                    EntryTick::Select
                } else {
                    EntryTick::None
                }
            }
        }
    }

    pub fn render(
        &self,
        screen: &mut Screen,
        font: &crate::gfx::Image,
        x: i32,
        y: i32,
        selected: bool,
        highlight: Option<&str>,
        highlight_color: u32,
    ) {
        if matches!(self, Self::Blank) || matches!(self, Self::Array { hidden: true, .. }) {
            return;
        }
        let text = self.text();
        let color = self.color(selected);
        // Item/Recipe: Java draws text then overlays the 8×8 item icon at (x, y).
        // Icon blit stays host-side via asset_key(); text uses ListEntry.render.
        if let Some(needle) = highlight
            && !needle.is_empty()
            && text.to_lowercase().contains(&needle.to_lowercase())
        {
            // Java ListEntry search overlay uses Font.drawColor with containColor.
            let replaced = text.replace(
                needle,
                &format!(
                    "\u{a7}{}\u{a7}{}",
                    encode_argb(highlight_color),
                    encode_argb(COL_SLCT)
                ),
            );
            FontStyle::new(color).draw_at(screen, font, &replaced, x, y);
            return;
        }
        FontStyle::new(color).draw_at(screen, font, &text, x, y);
    }
}

/// Minimal host pattern check for single characters (Java InputEntry regex subset).
fn char_matches_pattern(ch: char, pattern: &str) -> bool {
    // World name: reject path separators and control chars (Java worldNameRegex).
    if pattern.starts_with("[^<>") || pattern.contains("\\\\|?*") {
        return !matches!(ch, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*')
            && !ch.is_control();
    }
    // Seed pattern: [-!"#%/()=+,a-zA-Z0-9]+
    if pattern.starts_with("[-!") {
        return ch.is_ascii_alphanumeric()
            || matches!(ch, '-' | '!' | '"' | '#' | '%' | '/' | '(' | ')' | '=' | '+' | ',');
    }
    !ch.is_control()
}

fn encode_argb(color: u32) -> String {
    // Java Color.toStringCode uses four §-escaped chars of raw component values.
    // For display hosts that already resolved colors, use the § + 4-char form.
    let a = ((color >> 24) & 0xff) as u8;
    let r = ((color >> 16) & 0xff) as u8;
    let g = ((color >> 8) & 0xff) as u8;
    let b = (color & 0xff) as u8;
    let mut s = String::new();
    for c in [a, r, g, b] {
        s.push(char::from_u32(u32::from(c)).unwrap_or('\u{a7}'));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_and_string_are_unselectable() {
        assert!(!Entry::blank().is_selectable());
        assert!(!Entry::string("hi").is_selectable());
        assert!(Entry::select("ok", true).is_selectable());
    }

    #[test]
    fn input_entry_accepts_typed_chars_and_backspace() {
        let mut entry = Entry::input("World", 36);
        if let Entry::Input { pattern, .. } = &mut entry {
            *pattern = Some("[^<>:\"/\\\\|?*\\x00-\\x1F]+".to_owned());
        }
        assert_eq!(
            entry.tick(&EntryInput {
                typed_text: "Ab/c".into(),
                ..Default::default()
            }),
            EntryTick::None
        );
        assert_eq!(entry.user_input(), "Abc");
        assert_eq!(
            entry.tick(&EntryInput {
                backspace: true,
                ..Default::default()
            }),
            EntryTick::None
        );
        assert_eq!(entry.user_input(), "Ab");
        assert_eq!(entry.color(true), COL_GREEN);
    }

    #[test]
    fn create_world_select_uses_java_cyan() {
        let entry = Entry::select_colored("Create World", COL_CYAN);
        assert_eq!(entry.color(false), COL_CYAN);
        assert_eq!(entry.color(true), COL_CYAN);
    }

    #[test]
    fn array_entry_moves_and_wraps() {
        let mut entry = Entry::array(
            "Diff",
            vec!["Easy".into(), "Normal".into(), "Hard".into()],
            0,
            true,
        );
        let left = EntryInput {
            left: true,
            ..Default::default()
        };
        assert_eq!(entry.tick(&left), EntryTick::Select);
        let text = entry.text();
        assert!(text.contains("Hard"));
    }

    #[test]
    fn recipe_color_follows_can_craft_not_selection() {
        let ready = Entry::recipe("Wood Wall", Some("plank_wall".into()), true);
        let blocked = Entry::recipe("Stone Wall", Some("stone_wall".into()), false);
        assert_eq!(ready.color(false), COL_SLCT);
        assert_eq!(blocked.color(true), COL_UNSLCT);
    }

    #[test]
    fn key_input_is_full_screen_width_and_selectable() {
        let entry = Entry::key_input("ATTACK", "C|X");
        assert_eq!(entry.width(), crate::gfx::WIDTH as i32);
        assert!(entry.is_selectable());
    }

    #[test]
    fn link_confirm_on_select() {
        let mut entry = Entry::link("Download", "https://example.com", 0x00_FF_FF);
        let select = EntryInput {
            select: true,
            ..Default::default()
        };
        assert_eq!(entry.tick(&select), EntryTick::Confirm);
        assert_eq!(entry.url(), "https://example.com");
    }

    #[test]
    fn item_listing_is_unselectable() {
        assert!(!Entry::item_listing(" Leather", Some("leather".into())).is_selectable());
        assert!(Entry::item("Leather", Some("leather".into())).is_selectable());
    }

    #[test]
    fn input_truncates_to_max_length() {
        let mut entry = Entry::input("Name", 3);
        entry.set_user_input("abcdef");
        assert_eq!(entry.user_input(), "abc");
    }
}
