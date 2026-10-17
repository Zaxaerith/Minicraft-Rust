//! Java AirWizard / ObsidianKnight defeat handling and boss health queries.
use super::{SoundEffect, World, spawn::NaturalMob};

impl World {
    /// Java Mob die side-effects for bosses and scoring.
    #[cfg(test)]
    pub(super) fn handle_defeated_mob(&mut self, species: NaturalMob) {
        self.handle_defeated_mob_level(species, 1);
    }

    pub(super) fn handle_defeated_mob_level(&mut self, species: NaturalMob, level: u8) {
        let (score, multiplier) = match species {
            NaturalMob::AirWizard => (100_000, 0),
            NaturalMob::ObsidianKnight => (300_000, 0),
            NaturalMob::Cow | NaturalMob::Pig | NaturalMob::Sheep => (15, 0),
            _ => (50 * u32::from(level), 1),
        };
        self.add_score(score, multiplier);
        match species {
            NaturalMob::AirWizard => {
                self.sound(SoundEffect::BossDeath);
                self.notify("minicraft.notification.air_wizard_defeated", 0);
                if !self.air_wizard_defeated {
                    self.unlock_achievement("minicraft.achievement.airwizard");
                    self.notify("minicraft.notification.dungeon_opened", -400);
                }
                self.air_wizard_defeated = true;
            }
            NaturalMob::ObsidianKnight => {
                self.sound(SoundEffect::BossDeath);
                self.obsidian_knight_defeated = true;
                self.story_complete = true;
                self.notify("minicraft.notification.obsidian_knight_defeated", 0);
                self.unlock_achievement("minicraft.achievement.obsidianknight");
            }
            _ => {}
        }
    }

    pub(super) fn current_boss_health(&self) -> Option<(u16, u16)> {
        match self.levels[self.current_level].depth {
            1 => self.levels[self.current_level]
                .entities
                .active_boss(NaturalMob::AirWizard),
            -4 => self.levels[self.current_level]
                .entities
                .active_boss(NaturalMob::ObsidianKnight),
            _ => None,
        }
    }
}
