//! World/save adapter for terminal timing and constructor-time score settlement.
use super::{GameMode, World, WorldAction};
use crate::io::Input;
use crate::screen::end_game::{EndGameDisplay, ScoreSettlement};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Serialize, Deserialize)]
struct Saved {
    active: bool,
    display_timer: u8,
    input_delay: u8,
    #[serde(default)]
    settlement: Option<SavedSettlement>,
    #[serde(default)]
    unlocked_minutes: Option<u8>,
    #[serde(default)]
    preferences_written: bool,
}

#[derive(Serialize, Deserialize)]
struct SavedSettlement {
    player_score: i32,
    final_score: i32,
}

thread_local! {
    // Java EndGameDisplay owns a static Random, not the world's terrain/combat
    // stream. Keep its lifetime at process scope and never advance World.random.
    static BONUS_RANDOM: std::cell::RefCell<super::random::JavaRandom> =
        std::cell::RefCell::new(super::random::JavaRandom::new(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_nanos() as i64
        ));
}

pub(super) fn serialize<S: Serializer>(
    value: &Option<EndGameDisplay>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    value
        .map(|v| Saved {
            active: v.active,
            display_timer: v.display_timer,
            input_delay: v.input_delay,
            unlocked_minutes: v.unlocked_minutes,
            preferences_written: v.preferences_written,
            settlement: v.settlement.map(|score| SavedSettlement {
                player_score: score.player_score,
                final_score: score.final_score,
            }),
        })
        .serialize(serializer)
}
pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<EndGameDisplay>, D::Error> {
    Option::<Saved>::deserialize(deserializer)?
        .map(|v| {
            if v.display_timer > 60
                || v.input_delay > 30
                || v.unlocked_minutes
                    .is_some_and(|value| !matches!(value, 10 | 120))
            {
                return Err(serde::de::Error::custom("invalid end-game timer"));
            }
            Ok(EndGameDisplay {
                active: v.active,
                display_timer: v.display_timer,
                input_delay: v.input_delay,
                unlocked_minutes: v.unlocked_minutes,
                preferences_written: v.preferences_written,
                settlement: v.settlement.map(|score| ScoreSettlement {
                    player_score: score.player_score,
                    final_score: score.final_score,
                }),
            })
        })
        .transpose()
}

impl World {
    /// Host-side persistence adapter. Run even when unfocused: Java constructs
    /// EndGameDisplay and writes preferences in the score prelude.
    pub(crate) fn write_end_game_preferences(
        &mut self,
        config: &mut crate::io::Config,
        score_minutes: usize,
    ) -> Result<(), String> {
        let Some(display) = self.end_game.as_mut() else {
            return Ok(());
        };
        if display.preferences_written {
            return Ok(());
        }
        display.unlocked_minutes = display.unlock_for(score_minutes);
        match display.unlocked_minutes {
            Some(10) => config.settings.score_time_10_unlocked = true,
            Some(120) => config.settings.score_time_120_unlocked = true,
            _ => {}
        }
        config.save()?;
        display.preferences_written = true;
        Ok(())
    }

    pub(super) fn render_end_game(
        &self,
        screen: &mut super::Screen,
        assets: &super::Assets,
        localization: &super::Localization,
    ) {
        let Some(display) = self.end_game.as_ref() else {
            return;
        };
        let Some(score) = display.settlement else {
            return;
        };
        let mut rows = vec![
            (
                localization.format(
                    "minicraft.displays.end_game.display.player_score",
                    &[&score.player_score.to_string()],
                ),
                0xffffff,
            ),
            (
                localization
                    .text("minicraft.displays.end_game.display.bonuses")
                    .to_owned(),
                0xffff00,
            ),
            (
                localization.format(
                    "minicraft.displays.end_game.display.final_score",
                    &[&score.final_score.to_string()],
                ),
                0xffffff,
            ),
        ];
        if let Some(minutes) = display.unlocked_minutes {
            rows.push((
                localization.format(
                    "minicraft.displays.end_game.display.unlocked",
                    &[&minutes.to_string()],
                ),
                0xffffff,
            ));
        }
        let exit = localization.text("minicraft.displays.end_game.exit");
        // Menu.Builder(true, 0, LEFT): 8px frame, 16px cursor insets,
        // non-selectable entries contribute width minus 32, no title.
        let content_width = rows
            .iter()
            .map(|(text, _)| (crate::gfx::font::text_width(&text) - 32).max(0))
            .chain([crate::gfx::font::text_width(&exit)])
            .max()
            .unwrap_or(0);
        let width = content_width + 48;
        let height = (rows.len() as i32 + 1) * 8 + 16;
        let x = (super::WIDTH as i32 - width) / 2;
        let y = (super::HEIGHT as i32 - height) / 2;
        super::render_menu_frame(screen, assets.hud(), x, y, width, height);
        for (index, (text, color)) in rows.iter().enumerate() {
            screen.text_colored(
                assets.font(),
                text,
                x + 24,
                y + 8 + index as i32 * 8,
                *color,
            );
        }
        screen.text_colored(
            assets.font(),
            &format!("> {exit} <"),
            x + 8,
            y + 8 + rows.len() as i32 * 8,
            0xffffff,
        );
    }

    pub(super) fn prepare_end_game(&mut self) {
        use crate::item::ItemId;
        let display = self.end_game.get_or_insert_with(EndGameDisplay::default);
        if display.settlement.is_some() {
            return;
        }
        let counts = [
            ItemId::Cloth,
            ItemId::Slime,
            ItemId::Bone,
            ItemId::Arrow,
            ItemId::Gunpowder,
            ItemId::AntidiousBook,
        ]
        .map(|item| i32::from(self.player.inventory.count(item)));
        display.settlement = Some(BONUS_RANDOM.with(|random| {
            ScoreSettlement::calculate(self.score as i32, counts, |bound| {
                random.borrow_mut().next_int(bound)
            })
        }));
    }

    pub(super) fn begin_end_game_tick(&mut self) {
        if self.game_over && self.mode == GameMode::Score {
            // Legacy terminal saves did not retain a settlement. Calculate once
            // on restoration; all new saves retain the original result.
            self.prepare_end_game();
            self.end_game
                .get_or_insert_with(EndGameDisplay::default)
                .begin_display_tick();
        }
    }
    pub(super) fn tick_end_game(&mut self, input: &Input) -> WorldAction {
        let exit = if self.mode == GameMode::Score {
            self.end_game
                .as_mut()
                .is_some_and(|display| display.tick(input.select))
        } else {
            input.select || input.exit
        };
        if exit {
            WorldAction::ReturnToTitle
        } else {
            WorldAction::None
        }
    }
    pub(super) fn end_game_visible(&self) -> bool {
        self.mode != GameMode::Score || self.end_game.as_ref().is_some_and(EndGameDisplay::visible)
    }
}
