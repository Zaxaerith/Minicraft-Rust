//! Java Bed: sleep skip handled by World; state preserves the bed color.
use super::FurnitureKind;
use crate::item::dye::DyeColor;

/// Native state zero predates colored beds and retains the legacy red bed.
/// New colors use ordinal + 1, leaving every existing Bed entity valid.
pub const fn state_for_color(color: DyeColor) -> u16 {
    color as u16 + 1
}

pub fn color_from_state(state: u16) -> DyeColor {
    state
        .checked_sub(1)
        .and_then(DyeColor::from_index)
        .unwrap_or(DyeColor::Red)
}

/// Java Bed.use: triggers sleep transition (World handles day skip).
pub fn is_bed(kind: FurnitureKind) -> bool {
    kind == FurnitureKind::Bed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_beds_are_red_and_new_states_are_unique() {
        assert_eq!(color_from_state(0), DyeColor::Red);
        for color in DyeColor::ALL {
            assert_eq!(color_from_state(state_for_color(color)), color);
        }
    }
}
