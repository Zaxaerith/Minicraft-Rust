//! Compatibility bridge for notification serialization, localization and GUI.
//! The shared queue/timer policy belongs to client/core, not this adapter.
use super::World;
use crate::{
    assets::Assets,
    gfx::{HEIGHT, Screen, WIDTH},
    resource::localization::Localization,
};
use crate::core::notifications::Notifications;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::sync::{Arc, OnceLock};

pub(super) struct Context {
    pub(super) locale: Arc<Localization>,
    exit: String,
}
impl Default for Context {
    fn default() -> Self {
        static ENGLISH: OnceLock<Arc<Localization>> = OnceLock::new();
        Self {
            locale: ENGLISH
                .get_or_init(|| Arc::new(Localization::load("en-us", &[])))
                .clone(),
            exit: "ESCAPE".into(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Saved {
    messages: Vec<String>,
    ticks: i32,
    ticking: bool,
}
pub(super) fn serialize<S: Serializer>(
    notes: &Notifications,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    Saved {
        messages: notes.messages().to_vec(),
        ticks: notes.ticks(),
        ticking: notes.ticking(),
    }
    .serialize(serializer)
}
pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Notifications, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Format {
        Current(Saved),
        Legacy(Option<(String, u16)>),
    }
    Ok(match Format::deserialize(deserializer)? {
        Format::Current(saved) => {
            Notifications::from_state(saved.messages, saved.ticks, saved.ticking)
        }
        // Preserve old text and approximately its remaining lifetime. The old
        // format never recorded render feedback, so use its active-slot state.
        Format::Legacy(Some((text, remaining))) => {
            Notifications::from_state(vec![text], 181 - i32::from(remaining), true)
        }
        Format::Legacy(None) => Notifications::default(),
    })
}

impl World {
    pub(crate) fn set_notification_context(&mut self, locale: Arc<Localization>, exit: &str) {
        self.notification_context = Context {
            locale,
            exit: exit.to_owned(),
        };
    }
    pub(super) fn add_notification(&mut self, message: &str) {
        self.notifications
            .add(self.notification_context.locale.text(message));
    }
    pub(super) fn notify(&mut self, message: &str, ticks: i32) {
        self.notifications
            .notify(self.notification_context.locale.text(message), ticks);
    }
    pub(super) fn add_notification_args(&mut self, key: &str, args: &[&str]) {
        self.notifications
            .add(self.notification_context.locale.format(key, args));
    }
    pub(crate) fn record_save_success(&mut self) {
        self.reset_autosave();
        self.notify("minicraft.notification.world_saved", 0);
    }
    pub(crate) fn clear_notifications(&mut self) {
        self.notifications.clear();
    }

    pub(super) fn render_status(
        &mut self,
        screen: &mut Screen,
        assets: &Assets,
        locale: &Localization,
    ) {
        render_status(
            screen,
            assets,
            locale,
            &mut self.notifications,
            None,
            self.sleeping > 0,
            self.sleeping > 0,
            &self.notification_context.exit,
        );
    }
}

/// Saving is synchronous today, so production has no intermediate progress frame.
/// Keep the Java permanent-status composition available for the future async host.
#[allow(clippy::too_many_arguments)]
pub(crate) fn render_status(
    screen: &mut Screen,
    assets: &Assets,
    locale: &Localization,
    notes: &mut Notifications,
    saving: Option<u8>,
    sleeping: bool,
    in_bed: bool,
    exit: &str,
) {
    let mut permanent = Vec::new();
    if let Some(percent) = saving {
        permanent.push(
            locale
                .format(
                    "minicraft.display.gui.perm_status.saving",
                    &[&percent.to_string()],
                )
                .replace("%%", "%"),
        );
    }
    if sleeping {
        permanent.push(
            locale
                .text("minicraft.display.gui.perm_status.sleeping")
                .to_owned(),
        );
    }
    if in_bed {
        permanent.push(locale.format("minicraft.display.gui.perm_status.sleep_cancel", &[exit]));
    }
    let has_permanent = !permanent.is_empty();
    let visible = notes.prepare_gui(has_permanent);
    // Java Renderer: permanent status uses TOP relTextPos + spacing 1;
    // notifications use TOP with spacing 0 and keep the same bottom-right shadow.
    let style = crate::gfx::font_style::FontStyle::new(0xFFFFFF)
        .set_shadow_type(0x33_33_33, false);
    let style = if has_permanent {
        style
            .set_y_pos(HEIGHT as i32 / 2 - 25)
            .set_rel_text_pos(crate::screen::rel_pos::RelPos::TOP, true)
    } else {
        style
            .set_y_pos_aligned(HEIGHT as i32 * 2 / 5, false)
            .set_rel_text_pos(crate::screen::rel_pos::RelPos::TOP, false)
    };
    let lines = if has_permanent {
        permanent
    } else {
        visible
            .iter()
            .flat_map(|text| notification_lines(text))
            .collect::<Vec<_>>()
    };
    let borrowed: Vec<&str> = lines.iter().map(String::as_str).collect();
    let spacing = if has_permanent { 1 } else { 0 };
    style.draw_paragraph(screen, assets.font(), &borrowed, spacing);
}

/// Font.getLines at 288x192, 8px fixed glyphs. No trimming: repeated spaces matter.
fn notification_lines(text: &str) -> Vec<String> {
    let mut remainder = text.encode_utf16().collect::<Vec<_>>();
    let mut result = Vec::new();
    while !remainder.is_empty() {
        let split = java_line_length(&remainder).min(remainder.len());
        result.push(String::from_utf16_lossy(&remainder[..split]));
        let skip = split + usize::from(matches!(remainder.get(split), Some(32 | 10)));
        remainder.drain(..skip);
        if result.len() >= HEIGHT / 8 {
            if !remainder.is_empty() {
                result.push(String::from_utf16_lossy(&remainder));
            }
            break;
        }
    }
    result
}

fn java_line_length(text: &[u16]) -> usize {
    // Java getLine first applies replaceAll(" ?\\n ?", " \\n ").
    let mut normalized = Vec::new();
    let mut index = 0;
    while index < text.len() {
        let newline = if text[index] == 10 {
            Some(index)
        } else if text[index] == 32 && text.get(index + 1) == Some(&10) {
            Some(index + 1)
        } else {
            None
        };
        if let Some(newline) = newline {
            normalized.extend([32, 10, 32]);
            index = newline + 1;
            if text.get(index) == Some(&32) {
                index += 1;
            }
        } else {
            normalized.push(text[index]);
            index += 1;
        }
    }
    let words = normalized.split(|&ch| ch == 32).collect::<Vec<_>>();
    let width = |word: &[u16]| {
        word.len()
            .saturating_sub(word.iter().filter(|&&ch| ch == 0xA7).count() * 5)
            * 8
    };
    if width(words[0]) > WIDTH {
        return (1..words[0].len())
            .find(|&end| width(&words[0][..=end]) > WIDTH)
            .unwrap_or(words[0].len());
    }
    let mut used_width = width(words[0]);
    let mut length = words[0].len();
    for word in &words[1..] {
        if *word == [10] {
            break;
        }
        used_width += 8 + width(word);
        if used_width > WIDTH {
            break;
        }
        length += 1 + word.len();
    }
    length
}

#[cfg(test)]
mod tests {
    #[test]
    fn notification_wrapping_keeps_java_spaces_newlines_and_long_word_split() {
        assert_eq!(
            super::notification_lines("Line one\nLine two"),
            ["Line one", "Line two"]
        );
        assert_eq!(super::notification_lines(" A  B "), [" A  B "]);
        assert_eq!(super::notification_lines("\nA\n"), ["", "A"]);
        assert_eq!(
            super::notification_lines("ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-long"),
            ["ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789", "-long"]
        );
        let long = (0..30).map(|_| "Line").collect::<Vec<_>>().join("\n");
        let lines = super::notification_lines(&long);
        assert_eq!(lines.len(), 25); // 24 drawable lines, then Java's remainder.
        assert!(lines[24].contains('\n'));
    }
}
