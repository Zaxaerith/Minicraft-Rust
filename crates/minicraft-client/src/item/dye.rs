//! Java `DyeItem` — dye colors used by wool / clothing / loom recipes.
use super::ItemId;

/// Java `DyeItem.DyeColor` packed colors (`0x1_RRGGBB`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DyeColor {
    Black = 0,
    Red = 1,
    Green = 2,
    Brown = 3,
    Blue = 4,
    Purple = 5,
    Cyan = 6,
    LightGray = 7,
    Gray = 8,
    Pink = 9,
    Lime = 10,
    Yellow = 11,
    LightBlue = 12,
    Magenta = 13,
    Orange = 14,
    White = 15,
}

impl DyeColor {
    pub const ALL: [Self; 16] = [
        Self::Black,
        Self::Red,
        Self::Green,
        Self::Brown,
        Self::Blue,
        Self::Purple,
        Self::Cyan,
        Self::LightGray,
        Self::Gray,
        Self::Pink,
        Self::Lime,
        Self::Yellow,
        Self::LightBlue,
        Self::Magenta,
        Self::Orange,
        Self::White,
    ];

    /// Java `DyeColor.color` field.
    pub const fn packed_color(self) -> u32 {
        match self {
            Self::Black => 0x1_1D_1D_21,
            Self::Red => 0x1_B0_2E_26,
            Self::Green => 0x1_5E_7C_16,
            Self::Brown => 0x1_83_54_32,
            Self::Blue => 0x1_3C_44_AA,
            Self::Purple => 0x1_89_32_B8,
            Self::Cyan => 0x1_16_9C_9C,
            Self::LightGray => 0x1_9D_9D_97,
            Self::Gray => 0x1_47_4F_52,
            Self::Pink => 0x1_F3_8B_AA,
            Self::Lime => 0x1_80_C7_1F,
            Self::Yellow => 0x1_FE_D8_3D,
            Self::LightBlue => 0x1_3A_B3_DA,
            Self::Magenta => 0x1_C7_4E_BD,
            Self::Orange => 0x1_F9_80_1D,
            Self::White => 0x1_F9_FF_FE,
        }
    }

    /// Java `MyUtils.capitalizeFully(color.toString().replace('_', ' '))`.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Black => "Black",
            Self::Red => "Red",
            Self::Green => "Green",
            Self::Brown => "Brown",
            Self::Blue => "Blue",
            Self::Purple => "Purple",
            Self::Cyan => "Cyan",
            Self::LightGray => "Light Gray",
            Self::Gray => "Gray",
            Self::Pink => "Pink",
            Self::Lime => "Lime",
            Self::Yellow => "Yellow",
            Self::LightBlue => "Light Blue",
            Self::Magenta => "Magenta",
            Self::Orange => "Orange",
            Self::White => "White",
        }
    }

    /// Lowercase underscore form used for asset keys (`light_gray_dye`).
    pub const fn asset_stem(self) -> &'static str {
        match self {
            Self::Black => "black",
            Self::Red => "red",
            Self::Green => "green",
            Self::Brown => "brown",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Cyan => "cyan",
            Self::LightGray => "light_gray",
            Self::Gray => "gray",
            Self::Pink => "pink",
            Self::Lime => "lime",
            Self::Yellow => "yellow",
            Self::LightBlue => "light_blue",
            Self::Magenta => "magenta",
            Self::Orange => "orange",
            Self::White => "white",
        }
    }
}

/// Dye items are not yet separate `ItemId` variants in the Rust registry;
/// wool/clothing recipes reference the color sources that exist today
/// (Rose / Lapis / Cactus / Flower / Coal). Kept for Java parity tables.
pub const fn dye_source_item(color: DyeColor) -> Option<ItemId> {
    match color {
        DyeColor::Red => Some(ItemId::Rose),
        DyeColor::Blue => Some(ItemId::Lapis),
        DyeColor::Green => Some(ItemId::Cactus),
        DyeColor::Yellow => Some(ItemId::Flower),
        DyeColor::Black => Some(ItemId::Coal),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dye_colors_match_java_palette() {
        assert_eq!(DyeColor::ALL.len(), 16);
        assert_eq!(DyeColor::Black.packed_color(), 0x1_1D_1D_21);
        assert_eq!(DyeColor::White.packed_color(), 0x1_F9_FF_FE);
        assert_eq!(DyeColor::LightGray.display_name(), "Light Gray");
        assert_eq!(DyeColor::LightGray.asset_stem(), "light_gray");
    }
}
